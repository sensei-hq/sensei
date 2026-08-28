//! Dōjō upstream contribute-cadence scheduler (R1).
//!
//! A long-lived tokio task (mirroring [`crate::tasks::analyzer_scheduler`]) that
//! wakes on an interval and, on the user's configured cadence, PREPARES an
//! approved memory-share batch into the daemon's durable outbox — the upstream
//! twin of the downstream pull loop in [`crate::federation::run_pull_loop`]
//! (spawned beside it at the same boot site).
//!
//! ## STAGE-ONLY — this NEVER publishes / egresses
//! The scheduler calls the stage-only path
//! ([`crate::dojo::contribute::stage_contribution`]): it runs the SAME strict
//! anonymise + confidentiality gate as the publish path and writes `pending`
//! rows into `sensei.dojo_outbox`. It does NOT push anything to a Dōjō. The
//! outbox→dojo publish stays the explicit manual C6 step
//! (`POST /api/share-review/{batch}/publish`). Auto-publish is deliberately not
//! implemented (a Jerry decision — open question #3 in the build plan).
//!
//! ## Respects the paused default
//! `collective_preferences` defaults to `destination = none` / `cadence = manual`
//! ("batching is paused until the user triggers it"). While either holds, the
//! scheduler is a NO-OP. Only a user-set `daily` / `weekly` cadence (with a
//! destination other than `none`) prepares a batch — the first (immediate) tick
//! included.
//!
//! ## Incrementality / idempotency
//! - **Persisted watermark** — [`LAST_PREPARED_KEY`] in `sensei.config` records
//!   the ms-epoch of the last cadence prepare, so an hourly tick / daemon restart
//!   can't re-prepare within a cadence window (mirrors
//!   `analyzer.last_full_refresh`).
//! - **Outbox dedup** — the outbox's `unique(membership_id, signature)` +
//!   `state <> 'sent'` write guard make re-staging the same artifact a no-op, so
//!   even a watermark miss can't double-stage or clobber a manual publish.

use std::sync::Arc;

use crate::collective::anonymize::{
    ContributorIdentity, GatewayGeneralizer, Generalizer, current_rotation_bucket,
};
use crate::collective::preferences;
use crate::db::pg_store::PgStore;
use crate::dojo::contribute::{PgOutbox, StageOutcome, load_batch, stage_contribution};
use crate::tasks::ticker;

/// `sensei.config` key holding the ms-epoch of the last cadence batch we PREPARED
/// into the outbox. Gates re-preparing within a cadence window (mirrors the
/// analyzer scheduler's `analyzer.last_full_refresh`).
pub const LAST_PREPARED_KEY: &str = "collective.last_prepared";

const DAY_MS: i64 = 86_400_000;
const WEEK_MS: i64 = 7 * DAY_MS;

/// Pure cadence gate. `manual` (the paused default) is NEVER due — the scheduler
/// stays a no-op until the user sets a `daily` / `weekly` cadence. A scheduled
/// cadence is due when never-prepared (`None`) or the interval has elapsed. Any
/// unexpected cadence value is treated as paused (the safe default — never share).
pub fn contribute_due(cadence: &str, last_prepared_ms: Option<i64>, now_ms: i64) -> bool {
    let interval = match cadence {
        "daily" => DAY_MS,
        "weekly" => WEEK_MS,
        // "manual" / unset / anything unexpected → paused, never due.
        _ => return false,
    };
    match last_prepared_ms {
        None => true,
        Some(prev) => now_ms - prev >= interval,
    }
}

/// What one scheduler tick did — surfaced for logging and unit-testing the gates.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PrepareOutcome {
    /// Sharing is paused: `destination = none` or `cadence = manual`. No-op.
    Paused,
    /// A scheduled cadence, but the interval hasn't elapsed since the last prepare.
    NotDue,
    /// Due, but no approved-and-unsent batch to prepare.
    NothingEligible,
    /// Due — one batch was prepared (staged) into the outbox.
    Prepared(StageOutcome),
}

/// Read config as an optional ms-epoch, tolerating a missing / unparseable value
/// as "never prepared" (safe: it just means the cadence gate treats it as due).
async fn read_last_prepared_ms(pg: &PgStore) -> Option<i64> {
    match pg.get_config(LAST_PREPARED_KEY).await {
        Ok(raw) => raw.and_then(|v| v.trim().parse::<i64>().ok()),
        Err(e) => {
            tracing::warn!(error = %e, "contribute_scheduler: reading {LAST_PREPARED_KEY} failed — treating as never-prepared");
            None
        }
    }
}

/// The cadence-gated prepare hook (mirrors `version_rescan::maybe_rescan…`):
/// if sharing is enabled and a batch is due, run the STAGE-ONLY contribute path
/// and advance the watermark. Non-fatal — every DB failure is logged and
/// swallowed; a bad tick never crashes the loop. Generic over the generalizer so
/// it is DB-testable with the deterministic `NoLlm` (no gateway required); the
/// spawned loop injects the live [`GatewayGeneralizer`].
pub async fn maybe_prepare_contribution_batch<G: Generalizer>(
    pg: &PgStore,
    generalizer: &G,
    now_ms: i64,
) -> PrepareOutcome {
    // 1. Honor the paused default: nothing leaves the machine until the user opts
    //    in with BOTH a destination and a scheduled cadence.
    let prefs = match preferences::get(pg).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "contribute_scheduler: reading collective preferences failed — skipping tick");
            return PrepareOutcome::Paused;
        }
    };
    if prefs.destination == "none" || prefs.cadence == "manual" {
        return PrepareOutcome::Paused;
    }

    // 2. Cadence gate against the persisted watermark.
    if !contribute_due(&prefs.cadence, read_last_prepared_ms(pg).await, now_ms) {
        return PrepareOutcome::NotDue;
    }

    // 3. Eligible-insight selection: the oldest approved batch still owing a Dōjō.
    let batch_id = match pg.next_unsent_approved_batch().await {
        Ok(Some((id, _project, _decided))) => id,
        Ok(None) => return PrepareOutcome::NothingEligible,
        Err(e) => {
            tracing::warn!(error = %e, "contribute_scheduler: next_unsent_approved_batch failed — skipping tick");
            return PrepareOutcome::NothingEligible;
        }
    };

    // 4. STAGE-ONLY: load → anonymise (the REAL gate) → write outbox `pending`.
    //    Never publishes; the outbox→dojo send stays the manual C6 step.
    let loaded = match load_batch(pg, batch_id).await {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!(error = %e, batch = %batch_id, "contribute_scheduler: load_batch failed — skipping tick");
            return PrepareOutcome::NothingEligible;
        }
    };
    let contributor = match pg.get_or_create_contributor_key().await {
        Ok(k) => ContributorIdentity { user_key: k },
        Err(e) => {
            tracing::warn!(error = %e, "contribute_scheduler: contributor key unavailable — skipping tick");
            return PrepareOutcome::NothingEligible;
        }
    };
    let outbox = PgOutbox(pg);
    let outcome =
        stage_contribution(&loaded, &outbox, generalizer, &contributor, current_rotation_bucket())
            .await;

    // 5. Advance the watermark only when the prepare actually touched the outbox
    //    (staged or held), so an unroutable batch (NoDestination) re-checks next
    //    tick instead of parking the cadence for a full interval.
    if outcome.staged > 0 || outcome.held > 0 {
        if let Err(e) = pg.set_config(LAST_PREPARED_KEY, &now_ms.to_string()).await {
            tracing::warn!(error = %e, "contribute_scheduler: persisting {LAST_PREPARED_KEY} failed — next tick may re-prepare (idempotent)");
        }
        tracing::info!(
            batch = %batch_id, staged = outcome.staged, held = outcome.held,
            "contribute_scheduler: prepared contribution batch into outbox (stage-only)",
        );
    }

    PrepareOutcome::Prepared(outcome)
}

/// Spawn the contribute-cadence scheduler for the daemon's lifetime. Spawned
/// beside [`crate::federation::run_pull_loop`] (its downstream twin). The first
/// tick fires immediately but is a no-op under the paused default. STAGE-ONLY —
/// it never publishes / egresses.
pub fn spawn(pg: Arc<PgStore>, gateway: Arc<gateway::Gateway>) {
    tokio::spawn(async move {
        // Cadence lives in `sensei.schedules` (name `contribute`). The
        // generalizer is built once and shared across ticks — it is stateless
        // per pass, so it does not need rebuilding each time.
        let generalizer = Arc::new(GatewayGeneralizer::new(gateway));
        let store = pg.clone();
        ticker::run_scheduled(pg, "contribute", move || {
            let (pg, generalizer) = (store.clone(), generalizer.clone());
            async move {
                let now_ms = chrono::Utc::now().timestamp_millis();
                // Non-fatal: maybe_prepare logs its own failures and returns an
                // outcome; a staged-nothing pass is a success, not an error.
                let _ = maybe_prepare_contribution_batch(&pg, generalizer.as_ref(), now_ms).await;
                Ok(())
            }
        })
        .await;
    });
}

#[cfg(test)]
// Test gates are blocking `std::sync::Mutex` held across awaits ON PURPOSE —
// see `crate::tasks::test_support::TestGate` for why an async mutex loses
// wakeups across per-test runtimes. One allow per test module, not per site.
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::*;

    #[test]
    fn contribute_due_truth_table() {
        let day = DAY_MS;
        let week = WEEK_MS;

        // Paused: manual / unset / unknown are NEVER due, regardless of watermark.
        assert!(!contribute_due("manual", None, 10 * day));
        assert!(!contribute_due("manual", Some(0), 10 * day));
        assert!(!contribute_due("", None, 10 * day));
        assert!(!contribute_due("hourly", None, 10 * day));

        // Daily: never-prepared → due; a full day elapsed → due; less → not due.
        assert!(contribute_due("daily", None, 123));
        assert!(contribute_due("daily", Some(0), day));
        assert!(contribute_due("daily", Some(0), day + 1));
        assert!(!contribute_due("daily", Some(0), day - 1));

        // Weekly: 7 days is the interval.
        assert!(contribute_due("weekly", None, 123));
        assert!(contribute_due("weekly", Some(0), week));
        assert!(!contribute_due("weekly", Some(0), week - 1));
        assert!(!contribute_due("weekly", Some(0), day), "a day is not a week");
    }

    /// DB-gated: the prepare hook honors the paused default, stages exactly one
    /// outbox row via the REAL anonymise path when a scheduled cadence is due,
    /// no-ops when not due, and is idempotent (never double-stages). Uses the
    /// deterministic `NoLlm` generalizer (no gateway needed) against an employer
    /// (named) destination. Serialized on the collective-preferences singleton.
    #[tokio::test]
    async fn maybe_prepare_stages_one_row_and_honors_pause_and_idempotency() {
        use crate::db::pg_store::{InsertMemory, NewDojoMembership};
        use crate::dojo::contribute::NoLlm;

        let Ok(pg) = PgStore::connect_test().await else {
            return;
        };
        let _guard = preferences::test_lock().enter();

        // Clean slate for the FIXTURE, not just for the watermark below. Step 3
        // of the prepare picks the OLDEST approved batch IN THE DATABASE, so
        // this run's own leftovers from a previous run are what a later run
        // stages: `staged` is 1, the row lands on the earlier run's membership,
        // and the count against THIS one reads 0. Deleting first makes the test
        // independent of how many times it has run before.
        let (project, tenant) = ("_test:dojo:contribute_sched", "github/acme-corp");
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE name = $1")
            .bind(project)
            .execute(pg.pool())
            .await
            .unwrap();
        // Cascades to the outbox rows those memberships own.
        sqlx_core::query::query("DELETE FROM sensei.dojo_memberships WHERE tenant_key = $1")
            .bind(tenant)
            .execute(pg.pool())
            .await
            .unwrap();

        // A project + memory + APPROVED batch, a destination membership, and the
        // project bound to it so routing yields exactly one target.
        let proj = pg.create_project(project, None, None).await.unwrap();
        let mem = pg
            .insert_memory(&InsertMemory {
                project_id: Some(proj),
                scope: "project".into(),
                scope_filter: None,
                mtype: "convention".into(),
                title: "gate on green ci".into(),
                content: "Gate merges on a green pipeline before deploy.".into(),
                impact: None,
                tags: vec![],
                triage_signal: None,
                status: "active".into(),
                namespace_id: None,
                enforcement: None,
                origin: Some("learned".into()),
                source_id: None,
                spine_slot: None,
                feature: None,
            })
            .await
            .unwrap();
        let batch = pg.create_memory_share_batch(&proj, &[mem], None).await.unwrap();
        pg.set_memory_share_batch_status(&batch, "approved", None).await.unwrap();

        let mid = uuid::Uuid::new_v4();
        pg.create_dojo_membership(&NewDojoMembership {
            id: mid,
            registry_url: "http://localhost:7755".into(),
            tenant_key: tenant.into(),
            dojo_url: format!("http://localhost:7755/{tenant}"),
            kind: "employer".into(),
            org_slugs: vec![],
            role: "contributor".into(),
            authenticated_via: "device_code".into(),
            attribution_default: "named".into(),
            credential_ref: format!("dojo-{}", uuid::Uuid::new_v4()),
            sync_status: "healthy".into(),
        })
        .await
        .unwrap();
        pg.bind_project_to_dojo(&proj, Some(&mid)).await.unwrap();

        // Clean slate for the watermark this test drives.
        pg.delete_config(LAST_PREPARED_KEY).await.unwrap();

        // 1. PAUSED (default manual cadence) → no-op, nothing staged.
        set_prefs(&pg, "both", "manual").await;
        let paused = maybe_prepare_contribution_batch(&pg, &NoLlm, 1_000_000).await;
        assert!(matches!(paused, PrepareOutcome::Paused));
        assert_eq!(count_pending(&pg, mid).await, 0, "paused cadence stages nothing");

        // 2. DUE (daily, never prepared) → exactly one pending row via anonymise.
        set_prefs(&pg, "both", "daily").await;
        let prepared = maybe_prepare_contribution_batch(&pg, &NoLlm, 2_000_000).await;
        match prepared {
            PrepareOutcome::Prepared(o) => {
                assert_eq!(o.staged, 1, "one item staged");
                assert_eq!(o.held, 0);
            }
            other => panic!("expected Prepared, got {other:?}"),
        }
        assert_eq!(count_pending(&pg, mid).await, 1, "exactly one pending outbox row");
        assert!(pg.get_config(LAST_PREPARED_KEY).await.unwrap().is_some(), "watermark advanced");

        // 3. NOT DUE (same day) → no-op, still exactly one row.
        let not_due = maybe_prepare_contribution_batch(&pg, &NoLlm, 2_000_500).await;
        assert!(matches!(not_due, PrepareOutcome::NotDue), "within the cadence window → not due");
        assert_eq!(count_pending(&pg, mid).await, 1, "not-due tick stages nothing new");

        // 4. IDEMPOTENT: force due again (clear watermark) → re-stages the SAME
        //    signature → still exactly one row (outbox dedup), never a duplicate.
        pg.delete_config(LAST_PREPARED_KEY).await.unwrap();
        let again = maybe_prepare_contribution_batch(&pg, &NoLlm, 3_000_000).await;
        assert!(matches!(again, PrepareOutcome::Prepared(_)));
        assert_eq!(count_pending(&pg, mid).await, 1, "re-staging is idempotent — no duplicate row");

        // Cleanup: membership delete cascades its outbox rows; then batch/memory/project.
        assert!(pg.delete_dojo_membership(&mid).await.unwrap());
        pg.delete_project(&proj).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.collective_preferences")
            .execute(pg.pool())
            .await
            .unwrap();
        pg.delete_config(LAST_PREPARED_KEY).await.unwrap();
    }

    /// Count `pending` outbox rows for a membership — the staged-batch signal.
    async fn count_pending(pg: &PgStore, mid: uuid::Uuid) -> i64 {
        let (n,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.dojo_outbox WHERE membership_id = $1 AND state = 'pending'")
            .bind(mid).fetch_one(pg.pool()).await.unwrap();
        n
    }

    async fn set_prefs(pg: &PgStore, destination: &str, cadence: &str) {
        let prefs = preferences::CollectivePreferences::from_request(
            &serde_json::json!({ "destination": destination, "cadence": cadence }),
        )
        .unwrap();
        preferences::set(pg, prefs).await.unwrap();
    }
}
