//! Relay run→relay publish bridge (P1) — federate a daemon-owned run
//! (`activity.runs`) to the Dōjō relay so Jerry can watch the build.
//!
//! The gap this closes: nothing federated `activity.runs` → `dojo.relay_sessions`.
//! The only relay writers were TodoWrite-session-keyed
//! ([`super::publish_relay_segments`]), so a `start_run` run never appeared in
//! relay. This handler is the missing bridge: per active run (each tick / on a
//! status change) it maps the run (+ its newest cadence event + — in P1's
//! plan-as-run chunk — its plan phases) onto the relay wire types via the pure
//! [`crate::dojo::relay_run_project`] core and publishes over the outbound
//! [`DojoClient`] relay seam, then persists the returned cloud session id into
//! `activity.runs.dojo_session_id`.
//!
//! **STATUS, not drive.** This is the safe half of relay: the daemon PUBLISHES
//! the run's real status + heartbeat + stall so the phone can watch. It never
//! reads a reply to advance the run — `SENSEI_RUN_DRIVE` stays OFF and is not
//! touched here.
//!
//! **Membership routing.** A run publishes to its owning membership: the one its
//! project is bound to (`sensei.projects.dojo_id`), if any; otherwise it falls
//! back to every enabled membership (the personal-beta case = one). A
//! per-membership failure is logged (never swallowed — house rule) and skipped so
//! one down dojo can't wedge the others.
//!
//! **Zero-knowledge (D10):** only logical status crosses (status, progress,
//! phase/feature labels, timestamps) — never code, diffs, or tool output. The
//! mapping is the pure, unit-tested [`crate::dojo::relay_run_project`].

use super::super::executor::TaskContext;
use super::super::Task;
use crate::db::pg_store::DojoMembership;
use crate::dojo::client::DojoClient;
use crate::dojo::relay_run_project::run_to_session_update;
use crate::runs::Run;

/// Handler for `TaskKind::PublishRun`: federate one run's status to every owning
/// Dōjō membership. Returns the number of memberships successfully published to.
///
/// Short-circuits to `Ok(0)` (empty work, not an error) when there is nothing to
/// publish: no run id, a non-UUID run, a run that no longer exists, or no
/// membership to publish to.
pub async fn publish_run(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let run_id_str = task.path.as_str();
    if run_id_str.is_empty() {
        return Ok(0);
    }
    let Ok(run_id) = uuid::Uuid::parse_str(run_id_str) else {
        return Ok(0);
    };

    // A run completed/deleted between enqueue and dispatch is empty work.
    let Some(run) = ctx
        .pg()
        .get_run(&run_id)
        .await
        .map_err(|e| format!("get_run failed: {e}"))?
    else {
        return Ok(0);
    };

    let memberships = resolve_run_memberships(ctx, &run).await?;
    if memberships.is_empty() {
        return Ok(0);
    }

    // Newest cadence event timestamp → "last progress N min ago". `list_run_events`
    // returns newest-first, so the first row (if any) is the latest.
    let last_event_at = ctx
        .pg()
        .list_run_events(&run_id, 1)
        .await
        .map_err(|e| format!("list_run_events failed: {e}"))?
        .into_iter()
        .next()
        .map(|e| e.created_at);

    // P1 chunk 2 publishes the status snapshot (real status + heartbeat + stall).
    // Plan→segments (progress rollup) is wired in the plan-as-run chunk; until
    // then progress is an honest 0/0 (a start_run run has no TodoWrite outline).
    let update = run_to_session_update(&run, last_event_at.as_deref(), 0, 0);

    let mut published = 0u32;
    // Persist the cloud session id once — from the first membership that acks it.
    let mut persisted = run.dojo_session_id.is_some();
    for m in &memberships {
        let client = DojoClient::for_membership(m);
        match client.publish_session_update_returning_id(&update).await {
            Ok(session_id) => {
                if !persisted {
                    match uuid::Uuid::parse_str(&session_id) {
                        Ok(sid) => {
                            if let Err(e) = ctx.pg().set_run_dojo_session_id(&run_id, &sid).await {
                                // Non-fatal: the publish succeeded; only the local
                                // join-id write failed. Log and keep going — the
                                // next tick retries the persist.
                                tracing::warn!(run_id = %run_id, error = %e, "publish_run: persisting dojo_session_id failed");
                            } else {
                                persisted = true;
                            }
                        }
                        Err(e) => {
                            tracing::warn!(run_id = %run_id, session_id, error = %e, "publish_run: Worker returned a non-uuid session id");
                        }
                    }
                }
                published += 1;
            }
            Err(e) => {
                tracing::warn!(run_id = %run_id, membership = %m.id, error = %e, "publish_run: session publish failed");
            }
        }
    }
    Ok(published)
}

/// The memberships a run federates to: its project-bound membership if the
/// project is bound (`sensei.projects.dojo_id`), otherwise every enabled
/// membership. Only enabled memberships are ever returned. DB errors bubble up;
/// an unbound / unknown project cleanly falls back.
async fn resolve_run_memberships(
    ctx: &TaskContext,
    run: &Run,
) -> Result<Vec<DojoMembership>, String> {
    let all_enabled = || async {
        Ok::<_, String>(
            ctx.pg()
                .list_dojo_memberships()
                .await
                .map_err(|e| format!("list_dojo_memberships failed: {e}"))?
                .into_iter()
                .filter(|m| m.enabled)
                .collect::<Vec<_>>(),
        )
    };

    let Some(project_id) = run.project_id else {
        return all_enabled().await;
    };
    let bound = ctx
        .pg()
        .project_bound_membership(&project_id)
        .await
        .map_err(|e| format!("project_bound_membership failed: {e}"))?;
    let Some(membership_id) = bound else {
        return all_enabled().await;
    };
    // The project is bound — publish only to that membership, and only if it is
    // still enabled (a disabled binding falls back to nothing, not to broadcast:
    // an explicit binding is a deliberate scoping the owner shouldn't have
    // silently widened).
    match ctx
        .pg()
        .get_dojo_membership(&membership_id)
        .await
        .map_err(|e| format!("get_dojo_membership failed: {e}"))?
    {
        Some(m) if m.enabled => Ok(vec![m]),
        _ => Ok(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::state::SharedState;
    use crate::runs::NewRun;
    use crate::tasks::queue::TaskQueue;
    use crate::tasks::TaskKind;
    use std::sync::Arc;

    async fn make_ctx() -> Option<Arc<TaskContext>> {
        let queue = Arc::new(TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let pg = crate::db::pg_store::PgStore::connect_test().await.ok()?;
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg,
            gateway,
            event_tx: { let (tx, _) = tokio::sync::broadcast::channel(16); tx },
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            provisioning: None,
        });
        Some(Arc::new(TaskContext { queue, app_state, _graph_path: None, logger: sensei_logger::Logger::noop() }))
    }

    async fn del_run(pg: &crate::db::pg_store::PgStore, id: &uuid::Uuid) {
        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(id).execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn empty_run_id_is_empty_work() {
        let Some(ctx) = make_ctx().await else { return; };
        let task = Task::new(TaskKind::PublishRun, "", "");
        assert_eq!(publish_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn non_uuid_run_id_is_empty_work() {
        let Some(ctx) = make_ctx().await else { return; };
        let task = Task::new(TaskKind::PublishRun, "", "not-a-uuid");
        assert_eq!(publish_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn unknown_run_is_empty_work() {
        let Some(ctx) = make_ctx().await else { return; };
        let id = uuid::Uuid::new_v4().to_string();
        let task = Task::new(TaskKind::PublishRun, "", &id);
        assert_eq!(publish_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn known_run_with_no_dojo_is_zero() {
        // A real run but (typically) no enrolled dojo in the test DB → empty work,
        // not an error. Exercises the get_run + membership-resolve path end to end.
        let Some(ctx) = make_ctx().await else { return; };
        let pg = ctx.pg();
        let id = pg.create_run(&NewRun::default()).await.unwrap();
        let task = Task::new(TaskKind::PublishRun, "", &id.to_string());
        // With no enabled membership, resolve returns empty → Ok(0). (If a dev DB
        // happens to have an enrolled dojo, the publish would attempt a network
        // call to it; the test DB has none, so this is a clean 0.)
        let n = publish_run(&ctx, &task).await.unwrap();
        assert_eq!(n, 0, "no enrolled dojo ⇒ nothing published");
        del_run(pg, &id).await;
    }
}
