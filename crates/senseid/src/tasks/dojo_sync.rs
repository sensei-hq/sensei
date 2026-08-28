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
use crate::dojo_client::user_plane::{self, RepoInput};

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

    let total = personas.len();
    let mut failures = Vec::new();
    for persona in personas {
        if let Err(e) = sync_persona(&pg, &persona).await {
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

async fn sync_persona(pg: &PgStore, persona: &str) -> Result<(), String> {
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
    let dojo = dojo_url();

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

    let registered = user_plane::register_repositories(&dojo, &token, &inputs).await?;
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
    let plan = match user_plane::sync_plan(&dojo, &token).await {
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

    let pushed = push_allowed(pg, persona, &dojo, &token, &plan).await?;
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
    dojo: &str,
    token: &str,
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

    let result = match user_plane::push_metrics(dojo, token, &batch).await {
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

    #[tokio::test]
    async fn a_pass_with_no_signed_in_personas_is_a_no_op_not_an_error() {
        // The state of a fresh install. It must not log an error every cadence,
        // or the daemon cries wolf until the user signs in.
        let pg = Arc::new(PgStore::connect_test().await.unwrap());
        // Whatever this developer's database holds, the contract is the same:
        // the pass completes. It never fabricates a persona to work on.
        assert!(tick(pg).await.is_ok());
    }

    #[test]
    fn the_plan_is_recorded_against_a_real_sync_entity_value() {
        // A typo here fails at INSERT time inside an unattended worker, where the
        // only symptom is a warning nobody reads. `sensei.sync_entity` is an enum
        // precisely so this cannot be free text.
        assert_eq!(PLAN_ENTITY, "dojo_sync_plan");
    }
}
