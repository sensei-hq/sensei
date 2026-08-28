//! What the daemon does when the `dojo_sync` schedule fires.
//!
//! The first task with no bespoke worker: no ticker, no interval constant, no
//! `dojo_sync_scheduler` module. `main` hands this `tick` to
//! [`crate::tasks::ticker::run_scheduled`] and the cadence lives in
//! `sensei.schedules` like every other worker's. That is the whole point of
//! docs/spec/daemon/schedules.md step 5 — adding a scheduled task is now a row, a
//! registry entry and a `tick()`.
//!
//! # What one pass does
//!
//! ```text
//! for each persona that has signed in:            PgStore::signed_in_personas
//!     token = live access token                   skip this persona on failure
//!     shared = repositories marked 'shared'       gate 1, local, already exists
//!     POST /v1/you/repositories                   identity: which tenant?
//!         store tenant_id per repo                D2
//!         log unmapped[]                          D6
//!     GET  /v1/you/sync/plan                      entitlement: what may sync?
//!         on failure: record and skip the persona D7
//!     POST /v1/you/metrics                        push the rows the plan allows
//!         mark shared_at ONLY if the whole batch landed  the re-push watermark
//! ```
//!
//! # What it deliberately does NOT do yet
//!
//! **User-scoped rows are held back**, counted, and logged with the reason.
//! `dojo.repository_metrics.principal_id` is a principal, never a git email —
//! commit trailers are unverified — and `personas.principal_id` is unset until a
//! persona is linked, so there is nothing honest to attribute a per-person row
//! to. The ingest endpoint refuses them, so sending them would earn a rejection
//! every cycle forever.
//!
//! Per-repo governance pull (D3) is not here.

use std::sync::Arc;

use crate::api::handlers::auth::{AuthError, live_access_token};
use crate::db::pg_store::PgStore;
use crate::db::pg_store::sync::SyncMark;
use crate::dojo_client::settings::dojo_url;
use crate::dojo_client::user_plane::{self, HttpUserPlane, RepoInput, UserPlane};

/// How many shared repositories one pass will register at a time.
const REGISTER_LIMIT: i64 = 500;

/// How many metric rows one pass will push. The dōjō caps a batch at 1000; the
/// daemon pages rather than sending an unbounded body, because a memory profile
/// set by the client is not a decision a client gets to make.
const PUSH_LIMIT: i64 = 500;

/// The `sensei.sync_entity` value a whole-cycle plan fetch is recorded against.
/// Keyed on the persona's KEYCHAIN SLOT (`personas.session_slot`), so two personas'
/// failures stay distinguishable — never the label, which a sign-in rewrites.
const PLAN_ENTITY: &str = "dojo_sync_plan";

/// One pass of the dōjō sync cycle, for every signed-in persona.
///
/// Returns `Err` only when the pass could not be attempted at all (the persona
/// list could not be read). A persona that individually fails is logged and
/// skipped: one expired session must not stall the others, which is the failure
/// D1 exists to prevent.
pub async fn tick(pg: Arc<PgStore>) -> Result<(), String> {
    let personas = pg.signed_in_personas().await?;
    if personas.is_empty() {
        tracing::debug!("dojo_sync: no signed-in personas");
        return Ok(());
    }

    let plane = HttpUserPlane { dojo_url: dojo_url() };
    let total = personas.len();
    let mut failures = Vec::new();
    for persona in personas {
        if let Err(e) = sync_one(&pg, &persona, &plane).await {
            tracing::warn!(persona, error = %e, "dojo_sync: persona skipped");
            failures.push(format!("{persona}: {e}"));
        }
    }
    // A pass where EVERY persona failed is not a success. `run_scheduled` records
    // this return value as the schedule's `last_ok`, so swallowing it printed a
    // green worker over a cycle that moved nothing — the exact false report
    // observed live (plan rows 0, shared 0, last_ok = true). A PARTIAL failure
    // still returns Ok: D1's whole point is that one expired session must not
    // stall the others, and the survivors did work.
    if failures.len() == total {
        return Err(format!("all {total} personas failed: {}", failures.join("; ")));
    }
    Ok(())
}

/// Resolve the persona's credential, then run the cycle.
///
/// Split from [`sync_persona`] so the cycle itself is reachable in a test: token
/// resolution needs the real Keychain, and everything after it does not.
async fn sync_one(pg: &PgStore, persona: &str, plane: &dyn UserPlane) -> Result<(), String> {
    let token = match live_access_token(persona).await {
        Ok(t) => t,
        Err(e) => {
            // Distinguished rather than collapsed: "sign in again" is worth
            // saying out loud, a network blip is not worth saying at all.
            let needs_sign_in = e.needs_sign_in();
            return Err(match e {
                AuthError::SignedOut => "no stored session".to_string(),
                AuthError::Rejected(d) => format!("session rejected, sign in again: {d}"),
                AuthError::Unreachable(d) if !needs_sign_in => {
                    format!("dōjō unreachable, session kept: {d}")
                }
                AuthError::Unreachable(d) => d,
            });
        }
    };
    sync_persona(pg, persona, &token, plane).await
}

/// One persona's cycle, with the credential and the transport handed in.
async fn sync_persona(
    pg: &PgStore,
    persona: &str,
    token: &str,
    plane: &dyn UserPlane,
) -> Result<(), String> {
    // Gate 1 (intent), local: only repositories the user marked 'shared'.
    let shared = pg.shared_repositories(REGISTER_LIMIT).await?;
    if shared.is_empty() {
        tracing::debug!(persona, "dojo_sync: nothing shared");
        return Ok(());
    }

    let inputs: Vec<RepoInput<'_>> = shared
        .iter()
        .map(|r| RepoInput {
            repo_key: &r.repo_key,
            remote_url: r.remote_url.as_deref(),
            name: &r.name,
        })
        .collect();

    let registered = plane.register_repositories(token, &inputs).await?;
    for m in &registered.mapped {
        match uuid::Uuid::parse_str(&m.tenant_id) {
            // Store what the dōjō said, never a guess: an unparseable tenant id
            // is a bug on one side or the other, and writing a placeholder would
            // bury it under a plausible-looking row.
            Ok(id) => {
                if let Err(e) = pg.set_repository_tenant(&m.repo_key, id).await {
                    tracing::warn!(persona, repo = m.repo_key, error = %e,
                                   "dojo_sync: could not store the tenant mapping");
                }
            }
            Err(e) => tracing::warn!(persona, repo = m.repo_key, tenant_id = m.tenant_id,
                                     error = %e, "dojo_sync: dōjō sent an unparseable tenant id"),
        }
    }
    for u in &registered.unmapped {
        // Each reason is a different problem with different advice (D6), so they
        // are logged as themselves rather than counted.
        tracing::info!(
            persona,
            repo = u.repo_key,
            reason = u.reason,
            "dojo_sync: repository not mapped to a tenant"
        );
    }

    let mark = SyncMark { entity: PLAN_ENTITY, key: persona, direction: "pull" };
    let plan = match plane.sync_plan(token).await {
        Ok(p) => p,
        Err(e) => {
            // Recorded, not just logged: without a row here a failed cycle is
            // indistinguishable from a cycle with nothing to do. Not `?` — see the
            // push path below; the plan error is what matters, not the write.
            if let Err(be) = pg.mark_sync_error(&mark, &e).await {
                tracing::warn!(persona, error = %be, "could not record the plan failure");
            }
            return Err(e);
        }
    };
    for d in &plan.denied {
        tracing::info!(
            persona,
            repo = d.repo_key,
            tenant = d.tenant,
            reason = d.reason,
            "dojo_sync: repository not permitted to sync"
        );
    }
    pg.mark_synced(&mark, None).await?;

    let pushed = push_allowed(pg, persona, token, plane, &plan).await?;
    tracing::info!(
        persona,
        allowed = plan.allowed.len(),
        denied = plan.denied.len(),
        mapped = registered.mapped.len(),
        unmapped = registered.unmapped.len(),
        pushed,
        "dojo_sync: cycle complete"
    );
    Ok(())
}

/// Push the metric rows the plan allows, and mark exactly those as shared.
///
/// Returns how many rows the dōjō accepted.
async fn push_allowed(
    pg: &PgStore,
    persona: &str,
    token: &str,
    plane: &dyn UserPlane,
    plan: &user_plane::SyncPlan,
) -> Result<u32, String> {
    // The allow-list, as a set. The daemon syncs the set it was HANDED — it never
    // asks "may I sync X?", so it cannot include a repository it never offered,
    // and offline degrades to no-sync by construction.
    let allowed: std::collections::HashSet<&str> =
        plan.allowed.iter().map(|a| a.repo_key.as_str()).collect();
    if allowed.is_empty() {
        return Ok(0);
    }

    // `["repo"]`, filtered in SQL so the LIMIT applies to rows we can actually
    // push. Scope is a CAPABILITY question, not an entitlement one:
    // `dojo.repository_metrics.principal_id` is a principal, never a git email,
    // and `personas.principal_id` is unset until a persona is linked, so a
    // per-person row has nothing honest to attribute to. The ingest refuses them.
    //
    // Filtering here rather than after the fetch is load-bearing — see
    // `unpushed_metric_rows`. Held-back rows once crowded the window and a pass
    // pushed 66 of 132.
    let keys: Vec<&str> = allowed.iter().copied().collect();
    let pushable = pg.unpushed_metric_rows(&["repo"], &keys, PUSH_LIMIT).await?;
    // A failure here must not print "0 held back" — that is the exact mislead the
    // counter exists to prevent.
    let held = match pg.unpushed_metric_count(&["user"]).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(persona, error = %e, "could not count held-back rows");
            0
        }
    };
    if held > 0 {
        tracing::info!(
            persona,
            held,
            "dojo_sync: user-scoped rows held back — no principal to attribute them to yet"
        );
    }
    if pushable.is_empty() {
        return Ok(0);
    }

    let batch: Vec<user_plane::MetricPush<'_>> = pushable
        .iter()
        .map(|m| user_plane::MetricPush {
            repo_key: &m.repo_key,
            metric: &m.metric,
            scope: &m.scope,
            grain: &m.grain,
            computed_on: m.computed_on.to_string(),
            value: m.value,
            commit_sha: m.commit_sha.as_deref(),
            props: &m.props,
            source: &m.source,
        })
        .collect();

    // Which repositories this batch covers — the keys the outcome is recorded
    // against, so a failure is durable rather than a log line nobody tailed.
    let repos: std::collections::BTreeSet<&str> =
        pushable.iter().map(|m| m.repo_key.as_str()).collect();
    fn push_mark<'k>(key: &'k str) -> SyncMark<'k> {
        SyncMark { entity: "repository_metric", key, direction: "push" }
    }

    let result = match plane.push_metrics(token, &batch).await {
        Ok(r) => r,
        Err(e) => {
            // RECORDED, not just logged. Without this a push that fails every
            // cycle is invisible: the plan row still reads `synced`, the schedule
            // still reads ok, and the only symptom is that nothing ever arrives.
            // Observed exactly that way before this existed.
            for key in &repos {
                // NOT `?`: a failed bookkeeping write must not replace the push
                // error with itself. The operator needs to know why the dōjō
                // refused, not that sync_state was briefly unwritable.
                if let Err(be) = pg.mark_sync_error(&push_mark(key), &e).await {
                    tracing::warn!(repo = key, error = %be, "could not record the push failure");
                }
            }
            return Err(e);
        }
    };
    for r in &result.rejected {
        tracing::warn!(
            persona,
            repo = r.repo_key,
            metric = r.metric,
            reason = r.reason,
            "dojo_sync: metric row refused"
        );
    }

    // Mark every row the dōjō did NOT refuse — the COMPLEMENT of `rejected`.
    //
    // This used to require an all-or-nothing batch, on the reasoning that "the
    // response reports a count, not which rows". That was wrong: `rejected[]`
    // carries `(repo_key, metric)`, which identifies exactly what to exclude. The
    // consequence was a livelock — one permanently-refused row (an `unknown_metric`
    // from version skew, say) meant NO row in the 500-row window was ever marked,
    // so the identical batch was re-sent every cadence forever and the queue never
    // drained.
    let refused: std::collections::HashSet<(&str, &str)> =
        result.rejected.iter().map(|r| (r.repo_key.as_str(), r.metric.as_str())).collect();
    let ids: Vec<uuid::Uuid> = pushable
        .iter()
        .filter(|m| !refused.contains(&(m.repo_key.as_str(), m.metric.as_str())))
        .map(|m| m.id)
        .collect();
    pg.mark_metric_rows_shared(&ids).await?;

    if result.rejected.is_empty() {
        for key in &repos {
            pg.mark_synced(&push_mark(key), None).await?;
        }
    } else {
        // `skipped`, not `error`: the dōjō answered, and a refusal is a decision
        // rather than a fault. Both mean "not synced", but only one is a problem,
        // and a dashboard that cannot tell them apart cries wolf or stays silent.
        let why = result
            .rejected
            .iter()
            .map(|r| format!("{}: {}", r.metric, r.reason))
            .take(5)
            .collect::<Vec<_>>()
            .join("; ");
        for key in &repos {
            pg.mark_sync_skipped(&push_mark(key), &why).await?;
        }
        tracing::warn!(
            persona,
            accepted = result.accepted,
            refused = result.rejected.len(),
            "dojo_sync: partial acceptance — nothing marked shared, the batch retries next cycle"
        );
    }
    Ok(result.accepted)
}

/// Run the cycle on its schedule, forever.
///
/// No interval argument and no config key: `run_scheduled` reads the cadence
/// from `sensei.schedules` (name `dojo_sync`) on every poll, so changing it is a
/// PATCH rather than a restart.
pub fn spawn(pg: Arc<PgStore>) {
    tokio::spawn(async move {
        let store = pg.clone();
        crate::tasks::ticker::run_scheduled(pg, "dojo_sync", move || {
            let pg = store.clone();
            async move { tick(pg).await }
        })
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dojo_client::user_plane::{
        DeniedRepo, IngestResult, MappedRepo, MetricPush, RegisterResult, RejectedMetric, SyncPlan,
        UnmappedRepo,
    };
    use std::sync::Mutex;

    /// A scripted dōjō. Records what it was asked and answers what the test says.
    ///
    /// This is what makes the cycle testable at all: before the `UserPlane` trait,
    /// `tick`'s whole body could be replaced with `Ok(())` and both of its tests
    /// still passed.
    #[derive(Default)]
    struct StubPlane {
        register: Option<RegisterResult>,
        plan: Option<SyncPlan>,
        push: Option<Result<IngestResult, String>>,
        pushed_rows: Mutex<Vec<(String, String)>>,
        push_calls: Mutex<usize>,
    }

    #[async_trait::async_trait]
    impl UserPlane for StubPlane {
        async fn register_repositories(
            &self,
            _t: &str,
            _r: &[RepoInput<'_>],
        ) -> Result<RegisterResult, String> {
            Ok(self.register.clone().unwrap_or(RegisterResult { mapped: vec![], unmapped: vec![] }))
        }
        async fn sync_plan(&self, _t: &str) -> Result<SyncPlan, String> {
            Ok(self.plan.clone().unwrap_or(SyncPlan { allowed: vec![], denied: vec![] }))
        }
        async fn push_metrics(
            &self,
            _t: &str,
            m: &[MetricPush<'_>],
        ) -> Result<IngestResult, String> {
            *self.push_calls.lock().unwrap() += 1;
            self.pushed_rows
                .lock()
                .unwrap()
                .extend(m.iter().map(|x| (x.repo_key.to_string(), x.metric.to_string())));
            self.push
                .clone()
                .unwrap_or(Ok(IngestResult { accepted: m.len() as u32, rejected: vec![] }))
        }
    }

    fn mapped(repo_key: &str, tenant_id: &str) -> MappedRepo {
        MappedRepo {
            repo_key: repo_key.to_string(),
            tenant: "organization/ztest".to_string(),
            tenant_id: tenant_id.to_string(),
            repo_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// A shared repository with one pushable repo-scoped metric row.
    async fn seed(pg: &PgStore) -> (uuid::Uuid, String, String) {
        let uniq = uuid::Uuid::new_v4();
        let pid = pg.create_project(&format!("_test:cycle:{uniq}"), None, None).await.unwrap();
        let rid = crate::tasks::test_support::seed_bare_repository(pg, &pid, &uniq).await;
        sqlx_core::query::query(
            "UPDATE sensei.repositories SET visibility = 'shared' WHERE id = $1",
        )
        .bind(rid)
        .execute(pg.pool())
        .await
        .unwrap();
        let metric_key = format!("_test:cycle:{uniq}:m");
        // Seeded here rather than reaching into another module's private test
        // helpers — the cycle's tests own their own fixture.
        let mid: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.metrics \
                 (key, name, description, family, type, direction, purpose, how_to_read, \
                  formula, task_name, effective_from) \
             VALUES ($1, $1, 'cycle test', 'quality'::sensei.metric_family, \
                     'ratio'::sensei.metric_type, 'higher_better'::sensei.metric_direction, \
                     'p', 'h', 'f', 'ComputeFtr', current_date) \
             RETURNING id",
        )
        .bind(&metric_key)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let mid = mid.0;
        pg.upsert_project_metric_repo(
            &mid,
            &rid,
            "repo",
            None,
            None,
            chrono::NaiveDate::from_ymd_opt(2026, 8, 27).unwrap(),
            "daily",
            1.0,
            &serde_json::json!({}),
            "measured",
        )
        .await
        .unwrap();
        (pid, format!("test/bare-{uniq}"), metric_key)
    }

    async fn shared_count(pg: &PgStore, repo_key: &str) -> i64 {
        let n: (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.repository_metrics rm \
               JOIN sensei.repositories r ON r.id = rm.repository_id \
              WHERE r.repo_key = $1 AND rm.shared_at IS NOT NULL",
        )
        .bind(repo_key)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        n.0
    }

    async fn push_state(pg: &PgStore, repo_key: &str) -> Option<(String, Option<String>)> {
        sqlx_core::query_as::query_as(
            "SELECT state, last_error FROM sensei.sync_state \
              WHERE entity = 'repository_metric' AND entity_key = $1 AND direction = 'push'",
        )
        .bind(repo_key)
        .fetch_optional(pg.pool())
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_failed_push_is_recorded_and_marks_nothing_shared() {
        // Live bug #2. Deleting the `mark_sync_error` loop leaves the plan row
        // `synced` and the schedule `ok` while nothing ever arrives — the failure
        // is invisible.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed(&pg).await;
        let plane = StubPlane {
            plan: Some(SyncPlan {
                allowed: vec![mapped(&key, &uuid::Uuid::new_v4().to_string())],
                denied: vec![],
            }),
            push: Some(Err("dōjō returned 500 for the metric push: boom".into())),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let state = push_state(&pg, &key).await;
        let shared = shared_count(&pg, &key).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_err(), "the persona's cycle reports the failure");
        let (st, err) = state.expect("the push failure is RECORDED, not just logged");
        assert_eq!(st, "error");
        assert!(err.unwrap_or_default().contains("boom"), "the dōjō's reason is preserved");
        assert_eq!(shared, 0, "a failed push must not mark anything shared");
    }

    #[tokio::test]
    async fn a_partial_acceptance_marks_only_what_was_not_refused() {
        // The livelock: gating the watermark on an EMPTY `rejected` meant one
        // permanently-refused row kept the whole window queued forever. `rejected[]`
        // names the rows, so the complement is markable.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, metric_key) = seed(&pg).await;
        let plane = StubPlane {
            plan: Some(SyncPlan {
                allowed: vec![mapped(&key, &uuid::Uuid::new_v4().to_string())],
                denied: vec![],
            }),
            push: Some(Ok(IngestResult {
                accepted: 0,
                rejected: vec![RejectedMetric {
                    repo_key: key.clone(),
                    metric: metric_key.clone(),
                    reason: "unknown_metric".into(),
                }],
            })),
            ..Default::default()
        };

        let _ = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let shared = shared_count(&pg, &key).await;
        let state = push_state(&pg, &key).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert_eq!(shared, 0, "the REFUSED row stays queued so it can be retried");
        let (st, _) = state.expect("a refusal is recorded");
        assert_eq!(st, "skipped", "the dōjō answered — a refusal is a decision, not a fault");
    }

    #[tokio::test]
    async fn an_empty_allow_list_pushes_nothing_and_does_not_error() {
        // The plan is an allow-list: an empty one means "sync nothing", which is a
        // legitimate answer and must not look like a failure.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed(&pg).await;
        let plane = StubPlane {
            plan: Some(SyncPlan { allowed: vec![], denied: vec![] }),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let calls = *plane.push_calls.lock().unwrap();
        let shared = shared_count(&pg, &key).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_ok(), "nothing allowed is not an error");
        assert_eq!(calls, 0, "and nothing is sent");
        assert_eq!(shared, 0);
    }

    #[tokio::test]
    async fn an_unparseable_tenant_id_is_skipped_without_failing_the_cycle() {
        // The dōjō sending a non-uuid tenant id is a bug on one side or the other.
        // It must not be written as a placeholder, and must not take the cycle down.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed(&pg).await;
        let mut bad = mapped(&key, "not-a-uuid");
        bad.tenant_id = "not-a-uuid".into();
        let plane = StubPlane {
            register: Some(RegisterResult { mapped: vec![bad], unmapped: vec![] }),
            plan: Some(SyncPlan {
                allowed: vec![mapped(&key, &uuid::Uuid::new_v4().to_string())],
                denied: vec![],
            }),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let stored: (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT tenant_id FROM sensei.repositories WHERE repo_key = $1",
        )
        .bind(&key)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_ok(), "one bad tenant id does not fail the cycle");
        assert_eq!(stored.0, None, "and nothing plausible-looking is written in its place");
    }

    #[tokio::test]
    async fn a_successful_push_marks_the_rows_and_records_synced() {
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed(&pg).await;
        let plane = StubPlane {
            plan: Some(SyncPlan {
                allowed: vec![mapped(&key, &uuid::Uuid::new_v4().to_string())],
                denied: vec![],
            }),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let sent = plane.pushed_rows.lock().unwrap().clone();
        let shared = shared_count(&pg, &key).await;
        let state = push_state(&pg, &key).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_ok());
        assert_eq!(sent.len(), 1, "the allowed repo's row is sent");
        assert_eq!(shared, 1, "and marked shared so the next cycle does not re-send it");
        assert_eq!(state.expect("recorded").0, "synced");
    }

    #[tokio::test]
    async fn a_denied_repository_is_never_pushed() {
        // Entitlement: the daemon syncs the set it was HANDED. A repo in `denied`
        // must not reach the wire even though it passed gate 1 locally.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed(&pg).await;
        let plane = StubPlane {
            plan: Some(SyncPlan {
                allowed: vec![],
                denied: vec![DeniedRepo {
                    repo_key: key.clone(),
                    tenant: "organization/ztest".into(),
                    reason: "no_seat".into(),
                }],
            }),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let sent = plane.pushed_rows.lock().unwrap().clone();
        let shared = shared_count(&pg, &key).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_ok());
        assert!(sent.is_empty(), "a denied repository's rows never reach the wire");
        assert_eq!(shared, 0);
    }

    #[tokio::test]
    async fn an_unmapped_repository_is_reported_and_not_pushed() {
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed(&pg).await;
        let plane = StubPlane {
            register: Some(RegisterResult {
                mapped: vec![],
                unmapped: vec![UnmappedRepo {
                    repo_key: key.clone(),
                    reason: "unknown_host".into(),
                }],
            }),
            plan: Some(SyncPlan { allowed: vec![], denied: vec![] }),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let sent = plane.pushed_rows.lock().unwrap().clone();
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_ok(), "an unmapped repository is a reportable outcome, not a failure");
        assert!(sent.is_empty());
    }

    #[tokio::test]
    async fn a_pass_with_no_signed_in_personas_is_a_no_op_not_an_error() {
        // The state of a fresh install. It must not log an error every cadence,
        // or the daemon cries wolf until the user signs in.
        let pg = Arc::new(PgStore::connect_test().await.unwrap());
        assert!(tick(pg).await.is_ok());
    }

    #[tokio::test]
    async fn the_plan_entity_exists_in_the_database_enum() {
        // Was `assert_eq!(PLAN_ENTITY, "dojo_sync_plan")` — a const compared to its
        // own literal, which passed even with the enum value deleted. A typo or a
        // dropped label fails `mark_sync_error`'s cast at INSERT time inside an
        // unattended worker, where the only symptom is a warning nobody reads.
        let pg = PgStore::connect_test().await.unwrap();
        let labels: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT e.enumlabel::text FROM pg_enum e \
               JOIN pg_type t ON t.oid = e.enumtypid WHERE t.typname = 'sync_entity'",
        )
        .fetch_all(pg.pool())
        .await
        .unwrap();
        let labels: Vec<String> = labels.into_iter().map(|(l,)| l).collect();
        assert!(
            labels.iter().any(|l| l == PLAN_ENTITY),
            "{PLAN_ENTITY} is not a sensei.sync_entity value; the enum holds {labels:?}"
        );
    }
}
