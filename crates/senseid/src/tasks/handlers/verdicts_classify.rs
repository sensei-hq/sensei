//! `ClassifyPendingVerdicts` task handler — scheduled gap-fill for per-tool-call
//! verdicts.
//!
//! `verdict_classifier::classify_session` (#90) writes
//! `sensei.tool_call_verdicts` only when a session's Replay tab is opened
//! (`api/handlers/sessions.rs`) or the manual endpoint is hit
//! (`api/handlers/verdicts.rs`). So the Health-tab aggregate
//! (`tool_insights::aggregate_tool_insights`, which reads
//! `get_verdict_split_per_tool`) only ever reflected OPENED sessions.
//!
//! This global task finds every session with a recent `PostToolUse` that has
//! no verdict rows yet and classifies it, so the same scheduler tick's
//! `AggregateToolInsights` sees the full window. It REUSES `classify_session`
//! unchanged — the upsert is idempotent, so re-running a session is safe — and
//! the same `HEALTH_VERDICT_WINDOW_DAYS` window the aggregate keys off, so the
//! classifier window and the aggregate window never drift.
//!
//! Scoped as a "global" task (folder/name blank) so a single tick gap-fills
//! the whole window in one shot; ignores `task.path`.

use super::super::Task;
use super::super::executor::TaskContext;
use super::super::verdict_classifier::classify_session;
use super::tool_insights::HEALTH_VERDICT_WINDOW_DAYS;
use crate::db::pg_store::PgStore;

/// Classify every unclassified in-window session, returning
/// `(sessions_classified, verdicts_written)`. Split from the task handler so
/// the batch loop is unit-testable against a bare `PgStore` (no `TaskContext`).
///
/// No silent errors: if one session's classification fails, it is logged
/// (`tracing::warn`) and skipped so a single bad session can't abort the whole
/// pass — only the successful count is returned.
pub(crate) async fn classify_pending(
    pg: &PgStore,
    window_days: i32,
) -> Result<(usize, usize), String> {
    let sessions = pg.unclassified_verdict_sessions(window_days).await?;
    let mut sessions_classified = 0usize;
    let mut verdicts_written = 0usize;
    for sid in &sessions {
        match classify_session(pg, sid).await {
            Ok(n) => {
                sessions_classified += 1;
                verdicts_written += n;
            }
            Err(e) => {
                tracing::warn!(
                    session = %sid, error = %e,
                    "classify_pending_verdicts: session classification failed — skipping",
                );
            }
        }
    }
    Ok((sessions_classified, verdicts_written))
}

/// Global handler: gap-fill verdicts for sessions never opened in Replay.
/// Returns the number of verdict rows written across the batch.
pub async fn classify_pending_verdicts(ctx: &TaskContext, _task: &Task) -> Result<u32, String> {
    let (sessions_classified, verdicts_written) =
        classify_pending(ctx.pg(), HEALTH_VERDICT_WINDOW_DAYS).await?;

    tracing::info!(
        sessions_classified,
        verdicts_written,
        window_days = HEALTH_VERDICT_WINDOW_DAYS,
        "classify_pending_verdicts: gap-filled unopened sessions",
    );
    Ok(verdicts_written as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A classifiable pair (`PostToolUse` answer citing a path → `PreToolUse`
    /// reaction touching that path) on a fresh session with no verdicts:
    /// `classify_pending` writes rows for it, then a second pass excludes it
    /// (unclassified-only filter + upsert idempotency).
    #[tokio::test]
    async fn classify_pending_fills_unopened_then_excludes_on_rerun() {
        let Ok(pg) = PgStore::connect_test().await else {
            return;
        };
        let sid = format!("_test-classify-pending-{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now().timestamp_millis();

        // Answer: a PostToolUse whose response text cites a distinctive path.
        pg.insert_hook_event(
            &sid,
            "claude",
            "PostToolUse",
            Some("Read"),
            None,
            now,
            Some(true),
            &serde_json::json!({
                "tool_input": {"file_path": "crates/senseid/src/db/pg_store.rs"},
                "tool_response": "the fix is in crates/senseid/src/db/pg_store.rs:3421",
            }),
        )
        .await
        .unwrap();
        // Reaction: a PreToolUse whose tool_input cites the same path fragment
        // → the classifier scores this pair `used`.
        pg.insert_hook_event(
            &sid,
            "claude",
            "PreToolUse",
            Some("Edit"),
            None,
            now + 1,
            None,
            &serde_json::json!({
                "tool_input": {"file_path": "crates/senseid/src/db/pg_store.rs", "new_string": "x"},
            }),
        )
        .await
        .unwrap();

        // Before: the fixture session is pending.
        let before = pg.unclassified_verdict_sessions(HEALTH_VERDICT_WINDOW_DAYS).await.unwrap();
        assert!(before.contains(&sid), "fixture session pending before the pass");

        // First pass writes at least the fixture's verdict.
        let (classified, written) =
            classify_pending(&pg, HEALTH_VERDICT_WINDOW_DAYS).await.unwrap();
        assert!(classified >= 1, "at least the fixture session classified");
        assert!(written >= 1, "at least one verdict written");

        let summary = pg.get_verdict_summary_for_session(&sid).await.unwrap();
        assert!(summary["total"].as_i64().unwrap() >= 1, "fixture session now has verdicts");

        // Second pass: fixture is no longer pending (unclassified-only + idempotent),
        // so it is not re-counted.
        let after = pg.unclassified_verdict_sessions(HEALTH_VERDICT_WINDOW_DAYS).await.unwrap();
        assert!(!after.contains(&sid), "classified session excluded on the rerun");
    }
}
