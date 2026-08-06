//! Relay segment-publish (A2) — project a run's `TodoWrite` list into the relay
//! outline and push it to every enrolled Dōjō.
//!
//! When an assistant emits a `TodoWrite`, `ingest_hook_event` enqueues a
//! `PublishRelaySegments` task carrying the assistant `session_id` in
//! `task.path`. This handler reads that session's latest `TodoWrite`, projects
//! it via the pure [`crate::dojo::relay_project`] core (todos → segments →
//! session update), and publishes to each enabled membership over the outbound
//! [`DojoClient`] relay seam. P2 uses the assistant `session_id` (a UUID for
//! Claude) directly as the relay `run_id` — there is no `activity.runs` table
//! yet (that's P3).
//!
//! Zero-knowledge (D10): only todo `content` (a short human phrase) becomes a
//! segment title — never tool args, file contents, or diffs.

use super::super::executor::TaskContext;
use super::super::Task;
use crate::dojo::client::DojoClient;
use crate::dojo::relay_project::{parse_todos, session_update, title_from_cwd, todos_to_segments};

/// Handler for `TaskKind::PublishRelaySegments`: project the session's latest
/// `TodoWrite` into the relay outline and publish it to every enabled Dōjō
/// membership. Returns the number of memberships successfully published to.
///
/// Short-circuits to `Ok(0)` (empty work, not an error) when there is nothing
/// to publish: no session id, a non-UUID session, no enrolled dojo, no
/// `TodoWrite` yet, or an empty todo list.
pub async fn publish_relay_segments(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let session_id = task.path.as_str();
    if session_id.is_empty() {
        return Ok(0);
    }

    // UUID guard: `relay_sessions.run_id` is `uuid NOT NULL` on the Worker, and
    // P2 uses the assistant `session_id` as the `run_id`. A non-UUID session
    // (non-Claude harness) would make the Worker 500 on insert, so skip it.
    if uuid::Uuid::parse_str(session_id).is_err() {
        return Ok(0);
    }

    // Enrolled dojos — the "is a dojo enrolled" check (same source as the
    // contribute path). Only enabled memberships receive the outline.
    let memberships: Vec<crate::db::pg_store::DojoMembership> = ctx
        .pg()
        .list_dojo_memberships()
        .await
        .map_err(|e| format!("list_dojo_memberships failed: {e}"))?
        .into_iter()
        .filter(|m| m.enabled)
        .collect();
    if memberships.is_empty() {
        return Ok(0);
    }

    // Latest TodoWrite for the session → todos → segments → session update.
    let Some((payload, cwd)) = ctx
        .pg()
        .latest_todowrite(session_id)
        .await
        .map_err(|e| format!("latest_todowrite failed: {e}"))?
    else {
        return Ok(0);
    };
    let todos = parse_todos(&payload);
    if todos.is_empty() {
        return Ok(0);
    }
    let segments = todos_to_segments(&todos);
    let title = title_from_cwd(cwd.as_deref());
    let update = session_update(session_id, &title, &segments);

    // Publish to each enabled membership. A per-membership failure is logged
    // (never swallowed — house rule) and skipped so one down dojo can't wedge
    // the others.
    let mut published = 0u32;
    for m in &memberships {
        let client = DojoClient::for_membership(m);
        if let Err(e) = client.publish_session_update(&update).await {
            tracing::warn!(session_id, membership = %m.id, error = %e, "publish_relay_segments: session update failed");
            continue;
        }
        if let Err(e) = client.upsert_segments(session_id, &segments).await {
            tracing::warn!(session_id, membership = %m.id, error = %e, "publish_relay_segments: segment upsert failed");
            continue;
        }
        published += 1;
    }
    Ok(published)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    
    use crate::tasks::TaskKind;
    

    use crate::tasks::test_support::make_ctx;

    #[tokio::test]
    async fn empty_session_id_is_empty_work() {
        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::PublishRelaySegments, "", "");
        assert_eq!(publish_relay_segments(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn non_uuid_session_is_skipped() {
        // A non-Claude / non-uuid session would 500 the Worker (run_id is uuid
        // NOT NULL), so the handler skips it before touching the DB.
        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::PublishRelaySegments, "", "zed-session-123");
        assert_eq!(publish_relay_segments(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn uuid_session_with_no_dojo_or_todos_is_zero() {
        // A valid uuid with no TodoWrite (and typically no enrolled dojo) is
        // empty work, not an error.
        let ctx = make_ctx().await;
        let sid = uuid::Uuid::new_v4().to_string();
        let task = Task::new(TaskKind::PublishRelaySegments, "", &sid);
        assert_eq!(publish_relay_segments(&ctx, &task).await.unwrap(), 0);
    }
}
