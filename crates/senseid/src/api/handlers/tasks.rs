//! Following queued work — the read side of the task queue.
//!
//! Endpoints that hand back a task id (the backfills, any future long job) are
//! only half a contract without this: the caller gets an id it has no way to
//! resolve. These are the endpoints that make an id answerable.
//!
//! ## Why the durable log and not only the live stream
//!
//! The queue broadcasts [`TaskEvent`](crate::tasks::progress::TaskEvent)s, and
//! `/api/tasks/progress` streams them. That is genuinely live, but it is also
//! ephemeral and unaddressed: a subscriber who connects one second after its
//! task finished sees nothing at all, forever, because the events already fired.
//! That is the worst failure mode for a follower — indistinguishable from "still
//! running".
//!
//! So the durable `activity.task_executions` log is the source of truth and the
//! stream is an accelerator. [`get_task`] answers from the log and therefore
//! works before, during, and long after the run. [`task_events`] opens with a
//! snapshot from that same log and only then attaches the live stream, so a late
//! subscriber is caught up rather than stranded.
//!
//! ## Dispatchers
//!
//! A dispatcher (`BackfillTranscripts` queues one task per transcript) completes
//! in milliseconds having done none of the work. Reporting only its own row
//! would tell a follower "completed" while thousands of children had barely
//! started, so both endpoints carry the children and aggregate them.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        Json,
    },
};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::api::state::AppState;
use crate::tasks::progress::TaskEvent;

/// Roll the children of a dispatcher into the counts a follower actually wants.
///
/// Pure so the aggregation is testable without a queue or a database.
pub(crate) fn summarize_children(children: &[serde_json::Value]) -> serde_json::Value {
    let count = |s: &str| {
        children.iter().filter(|c| c["status"].as_str() == Some(s)).count()
    };
    let (completed, failed, running) = (count("completed"), count("failed"), count("running"));
    serde_json::json!({
        "total": children.len(),
        "completed": completed,
        "failed": failed,
        "running": running,
        // The question a follower is really asking. A dispatcher's own row says
        // `completed` the moment it finishes queueing, so "is the WORK done" can
        // only be answered by the children.
        "settled": completed + failed == children.len(),
    })
}

/// The live-queue view of a task, for one that has been queued but has not yet
/// started — it has no execution row, so the log alone would 404 a task that
/// genuinely exists and is about to run.
async fn queued_state(state: &AppState, task_id: u64) -> Option<serde_json::Value> {
    state.task_queue.find_task(task_id).await.map(|t| {
        serde_json::json!({
            "taskId": task_id,
            "kind": t.kind.to_string(),
            "pipeline": t.kind.pipeline(),
            "stage": t.kind.stage(),
            "folderPath": t.folder_path,
            "path": t.path,
            "status": "queued",
        })
    })
}

/// `GET /api/tasks/{id}` — what happened to one queued task.
///
/// Answers from the durable log, so it stays valid long after the run finishes —
/// which is the point, since the live stream tells a late subscriber nothing.
///
/// Scoped to the CURRENT daemon session, because a task id is only meaningful
/// there: ids restart at 1 on every boot, so an unscoped lookup for id 1 returns
/// a heap of unrelated rows from previous sessions. That is not a limitation to
/// work around — after a restart the in-memory queue is gone and the task no
/// longer exists in any followable sense (boot reconcile fails its row).
///
/// A task still waiting in the queue has no execution row yet (the executor
/// writes one on start) and is reported as `queued` from the live queue.
///
/// 404 means genuinely unknown: not in the queue and never executed in this
/// session. Fail-closed on a read error (500) — never an empty "not found",
/// which a caller would read as "my task vanished".
pub(crate) async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let since = state.task_queue.session_start();
    let attempts = state
        .pg
        .task_execution_attempts(id as i64, since)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, task_id = id, "get_task: attempts read failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let children = state.pg.child_task_executions(id as i64, since).await.map_err(|e| {
        tracing::warn!(error = %e, task_id = id, "get_task: children read failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if attempts.is_empty() {
        return match queued_state(&state, id).await {
            Some(q) => Ok(Json(q)),
            None => Err(StatusCode::NOT_FOUND),
        };
    }

    // The newest attempt is the task's current state; the rest are history a
    // follower needs to see a retry for what it was (see
    // `task_execution_attempts`).
    Ok(Json(serde_json::json!({
        "taskId": id,
        "current": attempts[0],
        "attempts": attempts,
        "children": children,
        "childSummary": summarize_children(&children),
    })))
}

/// True when a broadcast event concerns the task being followed, or one of its
/// children. Pure — the filtering rule is the part worth testing.
pub(crate) fn event_task_id(event: &TaskEvent) -> Option<u64> {
    match event {
        TaskEvent::Queued { task_id }
        | TaskEvent::Started { task_id, .. }
        | TaskEvent::Completed { task_id, .. }
        | TaskEvent::Failed { task_id, .. } => Some(*task_id),
        // Folder-level rollups carry no task id, so they cannot be attributed to
        // one follower and are dropped rather than broadcast to everybody.
        TaskEvent::FolderQueued { .. } => None,
    }
}

/// `GET /api/tasks/{id}/events` — SSE for ONE task and its children.
///
/// Opens with a `snapshot` event carrying the same payload as [`get_task`], so a
/// subscriber that attaches late (or after completion) is immediately correct
/// instead of waiting on events that already fired. Live `TaskEvent`s follow,
/// filtered to this task and the children known at subscribe time.
///
/// Subscribing BEFORE the snapshot read is deliberate: the reverse order has a
/// window where an event fires after the read and before the subscription, and
/// is lost.
pub(crate) async fn task_events(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.task_queue.sender().subscribe();
    let since = state.task_queue.session_start();

    // One error path for the whole snapshot. Reading children separately and
    // defaulting them to empty would emit a snapshot claiming a dispatcher has no
    // children — indistinguishable from one that genuinely has none, and the
    // follower would conclude the work was done.
    let snapshot_read = async {
        let children = state.pg.child_task_executions(id as i64, since).await?;
        let attempts = state.pg.task_execution_attempts(id as i64, since).await?;
        Ok::<_, String>((attempts, children))
    }
    .await;

    let (snapshot, children) = match snapshot_read {
        Ok((attempts, children)) => (
            serde_json::json!({
                "type": "snapshot",
                "taskId": id,
                "current": attempts.first(),
                "attempts": attempts,
                "children": children,
                "childSummary": summarize_children(&children),
            }),
            children,
        ),
        Err(e) => {
            // Say so explicitly rather than sending an empty-looking snapshot: a
            // read failure and "nothing has happened yet" must not look alike.
            tracing::warn!(error = %e, task_id = id, "task_events: snapshot read failed");
            (serde_json::json!({ "type": "error", "taskId": id, "error": e }), Vec::new())
        }
    };

    let mut followed: std::collections::HashSet<u64> = std::collections::HashSet::new();
    followed.insert(id);
    for c in &children {
        if let Some(t) = c["taskId"].as_i64() {
            followed.insert(t as u64);
        }
    }

    let head = tokio_stream::once(Ok(Event::default().data(snapshot.to_string())));
    let live = BroadcastStream::new(rx).filter_map(move |result| {
        let event = result.ok()?;
        let tid = event_task_id(&event)?;
        if !followed.contains(&tid) {
            return None;
        }
        Some(Ok(Event::default().data(serde_json::to_string(&event).ok()?)))
    });

    Sse::new(head.chain(live))
}

/// `GET /api/tasks/kinds` — the task catalogue, grouped by pipeline.
///
/// Exists because the pipelines were previously convention only: which stage a
/// kind belonged to lived in reviewers' heads and in the order of a `match`.
/// Publishing the grouping means the app, the docs and a human debugging a
/// stalled queue all read the same answer from one place.
///
/// Pure: derived entirely from the kind descriptors, so it cannot drift from the
/// behaviour it describes.
pub(crate) async fn list_kinds() -> Json<serde_json::Value> {
    use std::collections::BTreeMap;
    let mut by_pipeline: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for k in crate::tasks::TaskKind::ALL {
        let i = k.info();
        by_pipeline
            .entry(format!("{:?}", i.pipeline).to_lowercase())
            .or_default()
            .push(serde_json::json!({
                "kind": i.name,
                "stage": format!("{:?}", i.stage).to_lowercase(),
                "budgetSecs": i.budget_secs,
                "highPriority": i.high_priority,
                "retryable": i.retryable,
            }));
    }
    Json(serde_json::json!({ "pipelines": by_pipeline }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn child(status: &str) -> serde_json::Value {
        serde_json::json!({ "taskId": 1, "status": status })
    }

    #[test]
    fn a_dispatcher_is_not_settled_while_a_child_still_runs() {
        // The whole reason children are aggregated: the dispatcher's OWN row says
        // `completed` the instant it finishes queueing, so only the children can
        // answer "is the work done".
        let s = summarize_children(&[child("completed"), child("running")]);
        assert_eq!(s["settled"], false);
        assert_eq!(s["total"], 2);
        assert_eq!(s["running"], 1);
    }

    #[test]
    fn a_failed_child_still_counts_as_settled() {
        // Settled means "no longer moving", not "succeeded" — a follower that
        // waited for completed == total would hang forever on a failure.
        let s = summarize_children(&[child("completed"), child("failed")]);
        assert_eq!(s["settled"], true);
        assert_eq!(s["failed"], 1);
    }

    #[test]
    fn no_children_is_settled_not_stuck() {
        // A leaf task has no children; reporting it unsettled would strand any
        // follower that waits on the summary.
        let s = summarize_children(&[]);
        assert_eq!(s["settled"], true);
        assert_eq!(s["total"], 0);
    }

    #[test]
    fn every_addressable_event_yields_its_task_id() {
        assert_eq!(event_task_id(&TaskEvent::Queued { task_id: 7 }), Some(7));
        assert_eq!(
            event_task_id(&TaskEvent::Started {
                task_id: 8,
                folder_path: String::new(),
                kind: String::new(),
                path: String::new(),
            }),
            Some(8)
        );
        assert_eq!(
            event_task_id(&TaskEvent::Failed {
                task_id: 9,
                folder_path: String::new(),
                kind: String::new(),
                error: String::new(),
            }),
            Some(9)
        );
    }

    #[test]
    fn a_folder_rollup_is_not_attributed_to_a_follower() {
        // It carries no task id, so forwarding it would send every follower
        // another follower's noise.
        assert_eq!(
            event_task_id(&TaskEvent::FolderQueued {
                folder_path: "/x".into(),
                files_total: 3,
            }),
            None
        );
    }
}
