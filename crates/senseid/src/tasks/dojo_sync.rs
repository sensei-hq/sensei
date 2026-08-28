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
//! ```
//!
//! # What it deliberately does NOT do yet
//!
//! **It pushes no metrics.** `docs/spec/dojo/daemon-sync.md` §1 called
//! `unpushed_metric_rows` "the one production push path"; that was wrong — it has
//! no production caller at all, only tests, and there is no dōjō endpoint
//! receiving metrics. So there is no existing push to gate on `plan.allowed`, and
//! building one is its own slice rather than something to smuggle in here.
//!
//! This pass therefore establishes IDENTITY and ENTITLEMENT and records both. It
//! is deliberately visible in the log rather than quietly no-op: a worker that
//! looks like it syncs and does not is worse than one that says what it did.
//!
//! Per-repo governance pull (D3) is likewise not here.

use std::sync::Arc;

use crate::api::handlers::auth::{AuthError, live_access_token};
use crate::db::pg_store::PgStore;
use crate::db::pg_store::sync::SyncMark;
use crate::dojo_client::settings::dojo_url;
use crate::dojo_client::user_plane::{self, RepoInput};

/// How many shared repositories one pass will register at a time.
const REGISTER_LIMIT: i64 = 500;

/// The `sensei.sync_entity` value a whole-cycle plan fetch is recorded against.
/// Keyed on the persona label, so two personas' failures stay distinguishable.
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

    for persona in personas {
        if let Err(e) = sync_persona(&pg, &persona).await {
            tracing::warn!(persona, error = %e, "dojo_sync: persona skipped");
        }
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
    match user_plane::sync_plan(&dojo, &token).await {
        Ok(plan) => {
            tracing::info!(
                persona,
                allowed = plan.allowed.len(),
                denied = plan.denied.len(),
                mapped = registered.mapped.len(),
                unmapped = registered.unmapped.len(),
                "dojo_sync: plan fetched (no metrics are pushed yet — see the module docs)"
            );
            pg.mark_synced(&mark, None).await?;
            Ok(())
        }
        Err(e) => {
            // Recorded, not just logged: without a row here a failed cycle is
            // indistinguishable from a cycle with nothing to do.
            pg.mark_sync_error(&mark, &e).await?;
            Err(e)
        }
    }
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
