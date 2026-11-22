use super::*;

/// One `activity.task_executions` row as selected by [`TASK_EXECUTION_COLUMNS`].
/// The alias and the column list travel together — changing one without the
/// other is a decode error at runtime, so neither is written out twice.
type TaskExecutionRow = (
    i64,                                   // task_id
    Option<i64>,                           // parent_task_id
    String,                                // task_kind
    String,                                // folder_path
    String,                                // path
    String,                                // status
    Option<i32>,                           // items_processed
    Option<i32>,                           // duration_ms
    i32,                                   // retry_number
    Option<String>,                        // error_message
    chrono::DateTime<chrono::Utc>,         // started_at
    Option<chrono::DateTime<chrono::Utc>>, // completed_at
    uuid::Uuid,                            // id
    Option<uuid::Uuid>,                    // retried_from_execution_id
    Option<i64>,                           // retried_from_task_id (via self-join)
);

/// Watchdog abandonments over a window — see [`PgStore::watchdog_abandonments`].
#[derive(Debug, serde::Serialize)]
pub struct Abandonments {
    pub days: i32,
    pub by_day: Vec<AbandonedDay>,
    pub worst: Vec<AbandonedTask>,
    /// Per-task rows, newest first — which task, where, when, and why.
    /// Populated separately (the aggregates answer "how much", these
    /// answer "which").
    #[serde(default)]
    pub tasks: Vec<serde_json::Value>,
}

/// Abandonments of one kind on one day.
#[derive(Debug, serde::Serialize)]
pub struct AbandonedDay {
    pub day: chrono::NaiveDate,
    pub task_kind: String,
    pub count: i64,
}

/// One task (kind and path) the watchdog abandoned, and how often.
#[derive(Debug, serde::Serialize)]
pub struct AbandonedTask {
    pub task_kind: String,
    pub path: String,
    pub count: i64,
    pub last_at: chrono::DateTime<chrono::Utc>,
}

/// Select list matching [`TaskExecutionRow`], field for field and in order.
/// The lineage tail resolves the retried row's task id inline, so every
/// reader (attempts, children, feeds) carries it without its own join.
const TASK_EXECUTION_COLUMNS: &str = "task_id, parent_task_id, task_kind::text, folder_path, path, \
     status, items_processed, duration_ms, retry_number, error_message, started_at, completed_at, \
     id, retried_from_execution_id, \
     (SELECT task_id FROM activity.task_executions r2 WHERE r2.id = activity.task_executions.retried_from_execution_id)";

/// One execution row as the wire shape a follower reads.
///
/// `items_processed` is the handler's own return value (rows written, files
/// queued …) — the only place a caller can learn what the work actually DID, so
/// it is carried through rather than reduced to a status.
fn task_execution_json(r: TaskExecutionRow) -> serde_json::Value {
    serde_json::json!({
        "taskId": r.0,
        "parentTaskId": r.1,
        "kind": r.2,
        "folderPath": r.3,
        "path": r.4,
        "status": r.5,
        "itemsProcessed": r.6,
        "durationMs": r.7,
        "retryNumber": r.8,
        "error": r.9,
        "startedAt": r.10,
        "completedAt": r.11,
        "id": r.12,
        "retriedFrom": r.13,
        "retriedFromTaskId": r.14,
    })
}

#[allow(dead_code, clippy::type_complexity)]
impl PgStore {
    pub async fn log_index_error(&self, index_error: &IndexErrorRow<'_>) -> Result<(), String> {
        let &IndexErrorRow { folder_id, file_path, error, adapter, phase } = index_error;
        sqlx_core::query::query(
            "INSERT INTO sensei.index_errors(folder_id, file_path, error, adapter, phase) VALUES($1, $2, $3, $4, $5)"
        )
            .bind(folder_id).bind(file_path).bind(error).bind(adapter).bind(phase)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_index_errors(
        &self,
        folder_id: Option<&uuid::Uuid>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<String>, chrono::DateTime<chrono::Utc>)> = match folder_id {
            Some(fid) => sqlx_core::query_as::query_as(
                "SELECT folder_id, file_path, error, adapter, phase, created_at FROM sensei.index_errors WHERE folder_id = $1 ORDER BY created_at DESC"
            ).bind(fid).fetch_all(&self.pool).await,
            None => sqlx_core::query_as::query_as(
                "SELECT folder_id, file_path, error, adapter, phase, created_at FROM sensei.index_errors ORDER BY created_at DESC LIMIT 200"
            ).fetch_all(&self.pool).await,
        }.map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(|(fid, fp, err, adapter, phase, ts)| {
                serde_json::json!({
                    "folder_id": fid, "file_path": fp, "error": err,
                    "adapter": adapter, "phase": phase, "created_at": ts.to_rfc3339(),
                })
            })
            .collect())
    }

    pub async fn clear_index_errors(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.index_errors WHERE folder_id = $1")
            .bind(folder_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete `public.logs` rows older than `days` days. The task logger writes
    /// two rows per task, so large scans add hundreds of thousands of rows;
    /// this enforces a retention window. Returns the number of rows removed.
    pub async fn prune_logs(&self, days: i32) -> Result<u64, String> {
        let r = sqlx_core::query::query(
            "DELETE FROM public.logs WHERE logged_at < now() - (interval '1 day' * $1)",
        )
        .bind(days)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(r.rows_affected())
    }

    /// Prune raw activity older than `days` days, respecting the analyzer's
    /// value-extraction guard (#74) AND the capture-before-reclaim guard
    /// (2026-08-12 retention decision):
    ///
    /// - Sessions are eligible only when `analyzed_at IS NOT NULL` AND
    ///   `started_at < now() - days` — a session whose insights the analyzer
    ///   never derived is kept even if it is old (would lose signal).
    /// - AND (capture-before-reclaim, invariant I20) the session's day must
    ///   EITHER already be captured in the durable metric store — an `EXISTS`
    ///   daily `scope = 'user'` `sensei.workspace_metrics` row for the session's
    ///   repository (via `folders.repository_id` on the session's
    ///   `repo_folder_id`) with `computed_on = date_trunc('day',
    ///   s.started_at)::date` AND whose metric has `capture_source = 'session'`
    ///   (a session-derived delivery metric — git/snapshot metrics never
    ///   authorize reclaim) — OR the session must be older than a hard backstop
    ///   (`backstop_days`) so nothing lingers forever if metrics never compute.
    ///   Backfilled history is thus durable regardless of prune/compute
    ///   ordering: a day's sessions are only reclaimed once that day's
    ///   session-derived snapshot exists (or the backstop forces it).
    /// - The eligible sessions' \`activity.turns\` cascade (FK ON DELETE
    ///   CASCADE) so `turns` deletes are counted via a preflight
    ///   `COUNT(*) WHERE session_id IN (…)` for observability.
    /// - `activity.transcript_turns` and `activity.assistant_events` key
    ///   session-scoped rows off `client_session_id` (text), NOT the
    ///   session uuid — no FK, so we DELETE by matching that column.
    /// - Session-less assistant_events (never attached to a session; still
    ///   valuable for global tool-usage stats via ts) are pruned by ts alone
    ///   when they're older than the cutoff — same window, but they don't
    ///   need the analyzed-only guard.
    ///
    /// Derived signals (\`inference.detected_patterns\` /
    /// \`inference.recommendations\` / \`inference.reasoning_traces\` /
    /// \`sensei.memories\`) are NEVER touched — they are the distilled value
    /// that survives raw-event pruning.
    ///
    /// Ordering respects FKs: children first (transcript_turns / assistant_events
    /// keyed by client_session_id), then sessions (which cascades turns).
    pub async fn prune_activity(
        &self,
        days: i32,
        backstop_days: i32,
    ) -> Result<ActivityPruneCounts, String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        // (1) Snapshot eligible sessions once — used for every child delete
        //     so we don't re-scan the guard SQL four times. Capture-before-
        //     reclaim (invariant I20): a session is eligible only when its day is
        //     already captured by a CAPTURE-AUTHORIZING metric in
        //     sensei.workspace_metrics (daily grain, scope = 'user', on the
        //     session's OWN repository via the `repo_folder_id` -> folders
        //     -> repository_id join, and `metrics.capture_source = 'session'`)
        //     OR it is older than
        //     the hard backstop. Scoping to `capture_source = 'session'` is
        //     load-bearing: it is the registry's own record of which metrics are
        //     session-derived delivery signals — git/snapshot metrics stamp a
        //     grain='daily' row on their own day too, so an unscoped EXISTS would
        //     let a session be reclaimed before its delivery metric ever computed
        //     (the 164GB data-loss class). Keying the EXISTS on the session's OWN
        //     repository (via `repo_folder_id` -> `folders.repository_id`), not
        //     just its workspace, keeps the guard honest at repo grain: a session
        //     is reclaimed only once a session-derived snapshot exists FOR THAT
        //     REPOSITORY. The authorization now lives in the
        //     `metrics.capture_source` column (retiring the planner's
        //     day_keyed_task_names feed), so producer and guard can't drift.
        let eligible: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT s.id, s.client_session_id
               FROM activity.sessions s
               JOIN sensei.folders f ON f.id = s.folder_id
              WHERE s.analyzed_at IS NOT NULL
                AND s.started_at < now() - (interval '1 day' * $1)
                AND (EXISTS (SELECT 1
                               FROM sensei.workspace_metrics pm
                               JOIN sensei.metrics m ON m.id = pm.metric_id
                               JOIN sensei.folders rf ON rf.id = s.repo_folder_id
                              WHERE pm.repository_id = rf.repository_id
                                AND pm.scope = 'user'
                                AND pm.grain = 'daily'
                                AND pm.computed_on = date_trunc('day', s.started_at)::date
                                AND m.capture_source = 'session')
                     OR s.started_at < now() - (interval '1 day' * $2))",
        )
        .bind(days)
        .bind(backstop_days)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        if eligible.is_empty() {
            // Even with no eligible sessions, orphan assistant_events by ts
            // are still a valid target.
            let cutoff_ms = self.cutoff_millis(days);
            let ae = sqlx_core::query::query(
                // NOT EXISTS instead of NOT IN because sessions.client_session_id
                // is nullable — a NULL in the NOT IN subquery poisons the whole
                // predicate under ANSI three-valued logic.
                "DELETE FROM activity.assistant_events ae
              WHERE ae.ts < $1
                AND (ae.session_id = ''
                     OR NOT EXISTS (
                        SELECT 1 FROM activity.sessions s
                         WHERE s.client_session_id = ae.session_id))",
            )
            .bind(cutoff_ms)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            tx.commit().await.map_err(|e| e.to_string())?;
            return Ok(ActivityPruneCounts {
                assistant_events: ae.rows_affected(),
                ..Default::default()
            });
        }
        let session_uuids: Vec<uuid::Uuid> = eligible.iter().map(|(u, _)| *u).collect();
        let client_ids: Vec<String> = eligible.iter().map(|(_, c)| c.clone()).collect();

        // (2) Count turns that will cascade on the session delete — for the
        //     log line; the DELETE itself happens via CASCADE below.
        let turns_count: (i64,) = sqlx_core::query_as::query_as(
            "SELECT COUNT(*) FROM activity.turns WHERE session_id = ANY($1::uuid[])",
        )
        .bind(&session_uuids)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        // (3) transcript_turns keyed by client_session_id (text, no FK).
        let tt = sqlx_core::query::query(
            "DELETE FROM activity.transcript_turns WHERE session_id = ANY($1::text[])",
        )
        .bind(&client_ids)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        // (3b) The capture watermark for those sessions. Without this the prune
        //      is self-sealing: `ingest_one` gates the turn re-import on
        //      `prose_fresh` (watermark >= source mtime), so a pruned session
        //      never re-imports its turns — while `synthesize_session` happily
        //      re-creates the session row from the still-present source file.
        //      The result is a session that exists, has no transcript, and
        //      reports as fully ingested. It cannot heal (#125).
        let wm = sqlx_core::query::query(
            "DELETE FROM activity.capture_watermarks WHERE session_id = ANY($1::text[])",
        )
        .bind(&client_ids)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        let _ = wm;

        // (4) assistant_events for the same client_session_ids.
        let ae_session = sqlx_core::query::query(
            "DELETE FROM activity.assistant_events WHERE session_id = ANY($1::text[])",
        )
        .bind(&client_ids)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        // (5) sessions — cascades turns.
        let sess =
            sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = ANY($1::uuid[])")
                .bind(&session_uuids)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

        // (6) Session-less orphan assistant_events by ts. Runs after the
        //     session-scoped prune so we don't double-count.
        //
        //     NOT EXISTS, never NOT IN — the same reason the eligible-empty path
        //     above spells it out: `sessions.client_session_id` is NULLABLE, and
        //     under ANSI three-valued logic a single NULL in a `NOT IN` subquery
        //     makes the predicate NULL for EVERY row, so the DELETE silently
        //     matches nothing. This path had the `NOT IN` form, so orphan events
        //     were never reclaimed once any session carried a NULL client id —
        //     which is the normal case for a session anchored without one (e.g.
        //     an AI-start row). A retention leak that only showed up as a flaky
        //     test, because it passed exactly when no such session happened to
        //     exist.
        let cutoff_ms = self.cutoff_millis(days);
        let ae_orphan = sqlx_core::query::query(
            "DELETE FROM activity.assistant_events ae
              WHERE ae.ts < $1
                AND (ae.session_id = ''
                     OR NOT EXISTS (
                        SELECT 1 FROM activity.sessions s
                         WHERE s.client_session_id = ae.session_id))",
        )
        .bind(cutoff_ms)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;

        Ok(ActivityPruneCounts {
            sessions: sess.rows_affected(),
            turns: turns_count.0.max(0) as u64,
            transcript_turns: tt.rows_affected(),
            assistant_events: ae_session.rows_affected() + ae_orphan.rows_affected(),
        })
    }

    /// Reclaim dead tuples + refresh planner stats on the activity tables after the
    /// daily prune (`VACUUM (ANALYZE)`, once per prune tick — "once a day after
    /// processing"). Uses the SIMPLE query protocol via `raw_sql` because VACUUM
    /// cannot run through the prepared/extended protocol nor inside a transaction.
    /// Plain `VACUUM (ANALYZE)` ONLY — never `VACUUM FULL`, which takes an ACCESS
    /// EXCLUSIVE lock and rewrites the table (it would block the daemon). Autovacuum
    /// still runs continuously; this is an explicit post-bulk-delete reclaim.
    pub async fn vacuum_activity(&self) -> Result<(), String> {
        sqlx_core::raw_sql::raw_sql(
            "VACUUM (ANALYZE) activity.sessions, activity.turns, \
             activity.assistant_events, activity.transcript_turns, activity.transcript_raw",
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    /// Execute a parameterized query returning unresolved edges.
    pub async fn execute_raw_query(
        &self,
        sql: &str,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, Option<String>, String)> =
            sqlx_core::query_as::query_as(sql)
                .bind(folder_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, src, tgt_name, kind)| {
            serde_json::json!({ "id": id, "source_id": src, "target_name": tgt_name, "kind": kind })
        }).collect())
    }

    /// Execute a raw SQL statement.
    pub async fn execute_raw(&self, sql: &str) -> Result<(), String> {
        sqlx_core::query::query(sql)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("PgStore execute_raw: {}", e))?;
        Ok(())
    }

    // ── Logging (public.logs) ───────────────────────────────────────

    /// Insert a structured log entry into public.logs (kavach pattern).
    pub async fn insert_log(&self, row: &LogRow<'_>) -> Result<(), String> {
        // EXHAUSTIVE (#161): a field added to `LogRow` stops this
        // compiling until someone binds it.
        let LogRow { level, running_on, module, logged_at, message, context, data, error } = row;
        sqlx_core::query::query(
            "INSERT INTO public.logs(level, running_on, module, logged_at, message, context, data, error)
             VALUES($1, $2, $3, $4::timestamptz, $5, $6, $7, $8)"
        )
        .bind(level)
        .bind(running_on)
        .bind(module)
        .bind(logged_at)
        .bind(message)
        .bind(context)
        .bind(data)
        .bind(error)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("insert_log: {}", e))?;
        Ok(())
    }

    /// Read structured log rows from `public.logs` for the Observatory · Logs
    /// screen. All filters are optional (`None` = no constraint) and fully
    /// parameterized — never string-interpolated. Rows come back newest-first
    /// (`logged_at DESC`), capped at `limit`.
    ///
    /// - `level`   → exact match on the indexed `level` column.
    /// - `source`  → exact match on the indexed `running_on` column (which
    ///   component wrote the log: daemon / cli / mcp / app).
    /// - `module`  → exact match on the indexed `module` column
    ///   (finer source: scanner / watcher / analyzer / scheduler / …).
    /// - `since`   → lower bound on the indexed `logged_at` timestamp.
    pub async fn query_logs(&self, q: &LogQuery<'_>) -> Result<Vec<serde_json::Value>, String> {
        let &LogQuery { level, source, module, since, limit } = q;
        type LogRow = (
            uuid::Uuid,
            String,
            String,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
            Option<String>,
            serde_json::Value,
            Option<serde_json::Value>,
            Option<serde_json::Value>,
        );
        let rows: Vec<LogRow> = sqlx_core::query_as::query_as(
            "SELECT id, level, running_on, module, logged_at, message, context, data, error
             FROM public.logs
             WHERE ($1::text IS NULL OR level = $1)
               AND ($2::text IS NULL OR running_on = $2)
               AND ($3::text IS NULL OR module = $3)
               AND ($4::timestamptz IS NULL OR logged_at >= $4)
             ORDER BY logged_at DESC
             LIMIT $5",
        )
        .bind(level)
        .bind(source)
        .bind(module)
        .bind(since)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("query_logs: {}", e))?;

        Ok(rows
            .into_iter()
            .map(|(id, level, running_on, module, logged_at, message, context, data, error)| {
                serde_json::json!({
                    "id": id,
                    "level": level,
                    "source": running_on,
                    "module": module,
                    "logged_at": logged_at.to_rfc3339(),
                    "message": message,
                    "context": context,
                    "data": data,
                    "error": error,
                })
            })
            .collect())
    }

    // ── Task Executions (activity.task_executions) ──────────────────

    /// Insert a running task execution record. Returns the row UUID.
    /// `retry_number` is the task's attempt count (0 = first attempt), persisted
    /// so bounded retries (D6c) are observable on the logs/health screen.
    pub async fn start_task_execution(
        &self,
        start: &TaskExecutionStart<'_>,
    ) -> Result<uuid::Uuid, String> {
        let &TaskExecutionStart {
            task_id,
            parent_task_id,
            task_kind,
            folder_path,
            path,
            retry_number,
            retried_from,
        } = start;
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.task_executions(task_id, parent_task_id, task_kind, folder_path, path, status, retry_number, retried_from_execution_id)
             VALUES($1, $2, $3::sensei.task_execution_kind, $4, $5, 'running', $6, $7) RETURNING id"
        )
        .bind(task_id)
        .bind(parent_task_id)
        .bind(task_kind)
        .bind(folder_path)
        .bind(path)
        .bind(retry_number)
        .bind(retried_from)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("start_task_execution: {}", e))?;
        Ok(row.0)
    }

    /// Mark a task execution as completed.
    pub async fn complete_task_execution(
        &self,
        id: &uuid::Uuid,
        items_processed: i32,
        duration_ms: i32,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.task_executions
                SET status = 'completed', items_processed = $2, duration_ms = $3, completed_at = now()
              WHERE id = $1"
        )
        .bind(id)
        .bind(items_processed)
        .bind(duration_ms)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("complete_task_execution: {}", e))?;
        Ok(())
    }

    /// Mark a task execution as failed.
    pub async fn fail_task_execution(
        &self,
        id: &uuid::Uuid,
        duration_ms: i32,
        error_message: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.task_executions
                SET status = 'failed', duration_ms = $2, error_message = $3, completed_at = now()
              WHERE id = $1",
        )
        .bind(id)
        .bind(duration_ms)
        .bind(error_message)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("fail_task_execution: {}", e))?;
        Ok(())
    }

    /// Every execution row for one queue task id, newest attempt first.
    ///
    /// A task id can have MORE than one row: a retry inserts a fresh row with an
    /// incremented `retry_number`. Returning all of them (rather than the latest)
    /// is what makes a retried task honest to a follower — "succeeded on attempt
    /// 3 after two failures" and "succeeded first time" are different stories and
    /// collapsing them would hide the failures.
    ///
    /// `since` MUST be the issuing queue's session start. Task ids restart at 1
    /// with every daemon, so this table holds many unrelated rows per id — an
    /// unscoped lookup for id 1 returns a pile of other sessions' tasks, of other
    /// KINDS, which is worse than returning nothing.
    pub async fn task_execution_attempts(
        &self,
        task_id: i64,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let sql = format!(
            "SELECT {TASK_EXECUTION_COLUMNS} FROM activity.task_executions \
              WHERE task_id = $1 AND started_at >= $2 \
              ORDER BY retry_number DESC, started_at DESC"
        );
        let rows: Vec<TaskExecutionRow> = sqlx_core::query_as::query_as(&sql)
            .bind(task_id)
            .bind(since)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("task_execution_attempts: {e}"))?;
        Ok(rows.into_iter().map(task_execution_json).collect())
    }

    /// Execution rows for the CHILDREN of one queue task id.
    ///
    /// A dispatcher (`IngestCaptures` enqueues one task per transcript) has
    /// no meaningful progress of its own — it finishes in milliseconds having
    /// queued thousands of children. Following the parent alone would report
    /// "completed" while the actual ingestion had barely started, so a follower
    /// needs the children to see real progress.
    /// `since` scopes to the issuing queue's session, for the same reason as
    /// [`task_execution_attempts`] — parent ids restart at 1 too.
    pub async fn child_task_executions(
        &self,
        parent_task_id: i64,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let sql = format!(
            "SELECT {TASK_EXECUTION_COLUMNS} FROM activity.task_executions \
              WHERE parent_task_id = $1 AND started_at >= $2 ORDER BY started_at"
        );
        let rows: Vec<TaskExecutionRow> = sqlx_core::query_as::query_as(&sql)
            .bind(parent_task_id)
            .bind(since)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("child_task_executions: {e}"))?;
        Ok(rows.into_iter().map(task_execution_json).collect())
    }

    /// Watchdog abandonments in the last `days` (#188): a count per day and
    /// kind, and the `worst` offenders by kind and path — the tasks that timed
    /// out and stopped existing, which no other surface reports as a class.
    ///
    /// Read from the raw failed rows, which outlive the daily rollup (failures
    /// are kept `tasks.execution_failed_retention_days`); the rollup's status
    /// alone cannot tell an abandonment from any other failure.
    pub async fn watchdog_abandonments(
        &self,
        days: i32,
        worst: i64,
    ) -> Result<Abandonments, String> {
        let prefix = format!("{}%", crate::tasks::executor::WATCHDOG_ABANDONED);
        let by_day: Vec<(chrono::NaiveDate, String, i64)> = sqlx_core::query_as::query_as(
            "SELECT started_at::date, task_kind::text, count(*)
               FROM activity.task_executions
              WHERE status = 'failed' AND error_message LIKE $1
                AND started_at >= now() - (interval '1 day' * $2)
              GROUP BY 1, 2
              ORDER BY 1 DESC, 3 DESC, 2",
        )
        .bind(&prefix)
        .bind(days)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("watchdog_abandonments by day: {e}"))?;
        let offenders: Vec<(String, String, i64, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT task_kind::text, path, count(*), max(started_at)
                   FROM activity.task_executions
                  WHERE status = 'failed' AND error_message LIKE $1
                    AND started_at >= now() - (interval '1 day' * $2)
                  GROUP BY 1, 2
                  ORDER BY 3 DESC, 4 DESC
                  LIMIT $3",
            )
            .bind(&prefix)
            .bind(days)
            .bind(worst)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("watchdog_abandonments worst: {e}"))?;
        Ok(Abandonments {
            days,
            by_day: by_day
                .into_iter()
                .map(|(day, task_kind, count)| AbandonedDay { day, task_kind, count })
                .collect(),
            worst: offenders
                .into_iter()
                .map(|(task_kind, path, count, last_at)| AbandonedTask {
                    task_kind,
                    path,
                    count,
                    last_at,
                })
                .collect(),
            tasks: Vec::new(),
        })
    }

    /// Terminal failures in the last `days`, newest first, with kind, path,
    /// and reason — the failures feed. Completed and still-running rows are
    /// not failures.
    pub async fn failed_tasks(
        &self,
        days: i32,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        let sql = format!(
            "SELECT {TASK_EXECUTION_COLUMNS} FROM activity.task_executions
              WHERE status = 'failed' AND started_at >= now() - (interval '1 day' * $1)
              ORDER BY started_at DESC LIMIT $2"
        );
        let rows: Vec<TaskExecutionRow> = sqlx_core::query_as::query_as(&sql)
            .bind(days)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("failed_tasks: {e}"))?;
        Ok(rows.into_iter().map(task_execution_json).collect())
    }

    /// Watchdog abandonments as per-task rows, newest first — the aggregate
    /// queries answer "how much", these answer "which task, where, when".
    /// Same watchdog-prefix rule as [`Self::watchdog_abandonments`], so the
    /// two cannot disagree about what an abandonment is.
    /// Open a wave run row: `WaveTotals` landed. Returns the row id. Keys
    /// restart per session (dispatcher task ids), so opens never conflict —
    /// each open is its own row, and closes match the latest open pair.
    pub async fn open_wave_run(
        &self,
        wave: &str,
        key: &str,
        parent_task_id: Option<i64>,
        total: i64,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.wave_runs(wave, key, parent_task_id, total)
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(wave)
        .bind(key)
        .bind(parent_task_id)
        .bind(total)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("open_wave_run: {e}"))?;
        Ok(row.0)
    }

    /// Close the latest open row for a (wave, key) pair with settled counts.
    /// Returns false when no open row exists (a terminal without totals —
    /// recorded as a miss, never an error, so a missed open cannot fail a
    /// wave that otherwise completed).
    pub async fn close_wave_run(
        &self,
        wave: &str,
        key: &str,
        settled: i64,
        total: i64,
        outcome: &str,
    ) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE activity.wave_runs SET settled = $3, total = $4, outcome = $5, closed_at = now()
              WHERE id = (SELECT id FROM activity.wave_runs
                           WHERE wave = $1 AND key = $2 AND outcome = 'open'
                           ORDER BY opened_at DESC LIMIT 1)",
        )
        .bind(wave)
        .bind(key)
        .bind(settled)
        .bind(total)
        .bind(outcome)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("close_wave_run: {e}"))?;
        Ok(res.rows_affected() == 1)
    }

    /// Boot abandonment: close rows left open by dead sessions. A wave whose
    /// terminal never emitted can never complete — its dispatcher died with
    /// the process — so it reads `abandoned` with its last known totals
    /// instead of lingering open. Idempotent. Returns rows abandoned.
    pub async fn abandon_open_wave_runs(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE activity.wave_runs SET outcome = 'abandoned', closed_at = now()
              WHERE outcome = 'open' AND opened_at < $1",
        )
        .bind(before)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("abandon_open_wave_runs: {e}"))?;
        Ok(res.rows_affected())
    }

    /// Recent wave runs, newest first, optionally narrowed to one wave.
    pub async fn list_wave_runs(
        &self,
        wave: Option<&str>,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(
            String,
            String,
            Option<i64>,
            i32,
            i32,
            String,
            chrono::DateTime<chrono::Utc>,
            Option<chrono::DateTime<chrono::Utc>>,
        )> = sqlx_core::query_as::query_as(
            "SELECT wave, key, parent_task_id, total, settled, outcome, opened_at, closed_at
                   FROM activity.wave_runs
                  WHERE ($1::text IS NULL OR wave = $1)
                  ORDER BY opened_at DESC LIMIT $2",
        )
        .bind(wave)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("list_wave_runs: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(wave, key, parent_task_id, total, settled, outcome, opened_at, closed_at)| {
                serde_json::json!({
                    "wave": wave, "key": key, "parent_task_id": parent_task_id,
                    "total": total, "settled": settled, "outcome": outcome,
                    "opened_at": opened_at.to_rfc3339(),
                    "closed_at": closed_at.map(|t| t.to_rfc3339()),
                })
            })
            .collect())
    }

    /// Every alert disposition: what the reader acked, and what cleared
    /// before (recurrence memory for escalation). Alerts themselves
    /// re-derive from the feeds; this table holds only dispositions.
    pub async fn list_alert_acks(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, bool, i32, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT class, alert_key, acked, seen_clears, updated_at
                   FROM activity.alert_acks ORDER BY updated_at DESC",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("list_alert_acks: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(class, key, acked, seen_clears, updated_at)| {
                serde_json::json!({
                    "class": class, "key": key, "acked": acked,
                    "seen_clears": seen_clears,
                    "updated_at": updated_at.to_rfc3339(),
                })
            })
            .collect())
    }

    /// Write one disposition. `acked=true` acks; `acked=false` records a
    /// client-observed clear: un-acks AND bumps `seen_clears` — but only on
    /// the acked→cleared transition (or a first-seen clear), so a duplicate
    /// clear call never double-counts a recurrence. Piggybacks the stale
    /// prune: cleared rows untouched for 90+ days go, so recurrence memory
    /// fades instead of growing forever. No new worker.
    pub async fn set_alert_ack(
        &self,
        class: &str,
        key: &str,
        acked: bool,
    ) -> Result<serde_json::Value, String> {
        let row: (String, String, bool, i32, chrono::DateTime<chrono::Utc>) =
            sqlx_core::query_as::query_as(
                "INSERT INTO activity.alert_acks(class, alert_key, acked, seen_clears, updated_at)
                 VALUES ($1, $2, $3, CASE WHEN $3 THEN 0 ELSE 1 END, now())
                 ON CONFLICT(class, alert_key) DO UPDATE SET
                   acked = EXCLUDED.acked,
                   seen_clears = activity.alert_acks.seen_clears
                     + CASE WHEN NOT EXCLUDED.acked AND activity.alert_acks.acked THEN 1 ELSE 0 END,
                   updated_at = now()
                 RETURNING class, alert_key, acked, seen_clears, updated_at",
            )
            .bind(class)
            .bind(key)
            .bind(acked)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| format!("set_alert_ack: {e}"))?;
        sqlx_core::query::query(
            "DELETE FROM activity.alert_acks WHERE NOT acked AND updated_at < now() - interval '90 days'",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("set_alert_ack prune: {e}"))?;
        Ok(serde_json::json!({
            "class": row.0, "key": row.1, "acked": row.2,
            "seen_clears": row.3, "updated_at": row.4.to_rfc3339(),
        }))
    }

    pub async fn abandoned_task_list(
        &self,
        days: i32,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        let prefix = format!("{}%", crate::tasks::executor::WATCHDOG_ABANDONED);
        let sql = format!(
            "SELECT {TASK_EXECUTION_COLUMNS} FROM activity.task_executions
              WHERE status = 'failed' AND error_message LIKE $1
                AND started_at >= now() - (interval '1 day' * $2)
              ORDER BY started_at DESC LIMIT $3"
        );
        let rows: Vec<TaskExecutionRow> = sqlx_core::query_as::query_as(&sql)
            .bind(&prefix)
            .bind(days)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("abandoned_task_list: {e}"))?;
        Ok(rows.into_iter().map(task_execution_json).collect())
    }

    /// Roll up task executions older than `days` into `task_execution_daily`,
    /// then delete the raw rows — except failures, which are kept for
    /// `failed_days` because their `error_message`/`path`/`retry_number` are the
    /// whole reason to keep a log at all (and they are rare: 32,664 of 4.8M).
    ///
    /// Rollup happens BEFORE the delete, in one transaction, so a crash between
    /// the two cannot lose rows that were never aggregated.
    ///
    /// Idempotent: the upsert keys on `(day, task_kind, status)` and REPLACES
    /// the bucket rather than adding to it, so re-running over a day already
    /// rolled up yields the same numbers. That matters because the raw rows are
    /// gone after the first pass — an additive upsert would double-count every
    /// re-run and there would be no source left to correct it from.
    pub async fn rollup_and_prune_task_executions(
        &self,
        days: i32,
        failed_days: i32,
    ) -> Result<(u64, u64), String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        // Aggregate every eligible row, failures included: the daily counts
        // should describe the whole day even though the failed rows themselves
        // survive a while longer.
        let rolled = sqlx_core::query::query(
            "INSERT INTO activity.task_execution_daily
                 (day, task_kind, status, runs, failures, items_processed, p50_ms, p95_ms, max_ms, rolled_up_at)
             -- `::text::` is deliberate, not redundant: it makes the rollup work
             -- whether `task_executions.task_kind` is still text or already the
             -- enum, so the migration can roll up BEFORE converting the column
             -- (converting 4.8M rows first would rewrite the whole table).
             SELECT started_at::date, task_kind::text::sensei.task_execution_kind, status, count(*),
                    count(*) FILTER (WHERE status = 'failed'),
                    sum(items_processed),
                    percentile_disc(0.5) WITHIN GROUP (ORDER BY duration_ms)::int,
                    percentile_disc(0.95) WITHIN GROUP (ORDER BY duration_ms)::int,
                    max(duration_ms),
                    now()
               FROM activity.task_executions
              WHERE started_at < now() - (interval '1 day' * $1)
              GROUP BY 1, 2, 3
             ON CONFLICT (day, task_kind, status) DO UPDATE SET
                    runs = excluded.runs,
                    failures = excluded.failures,
                    items_processed = excluded.items_processed,
                    p50_ms = excluded.p50_ms,
                    p95_ms = excluded.p95_ms,
                    max_ms = excluded.max_ms,
                    rolled_up_at = excluded.rolled_up_at",
        )
        .bind(days)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("rollup_task_executions: {e}"))?
        .rows_affected();

        let pruned = sqlx_core::query::query(
            "DELETE FROM activity.task_executions
              WHERE started_at < now() - (interval '1 day' * $1)
                AND (status <> 'failed' OR started_at < now() - (interval '1 day' * $2))",
        )
        .bind(days)
        .bind(failed_days)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("prune_task_executions: {e}"))?
        .rows_affected();

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok((rolled, pruned))
    }

    /// Boot reconcile (D6b/W2): terminate task-execution rows still `running`
    /// from a prior daemon session. `task_id` resets per session and the queue
    /// is in-memory, so a `running` row whose `started_at` precedes this
    /// session's start can never complete — its worker died with the process.
    /// Mark those `failed` (a terminal state) with a completion time and an
    /// explanatory `error_message`, so `status='running'` reflects only live
    /// work. Rows started at/after `session_start` (this session's own
    /// in-flight tasks) are left untouched. Idempotent. Returns rows reconciled.
    pub async fn reconcile_orphaned_task_executions(
        &self,
        session_start: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE activity.task_executions
                SET status = 'failed',
                    error_message = 'orphaned: daemon restarted while task was running',
                    completed_at = now()
              WHERE status = 'running' AND started_at < $1",
        )
        .bind(session_start)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("reconcile_orphaned_task_executions: {}", e))?;
        Ok(res.rows_affected())
    }

    /// Crash orphans eligible for the boot re-queue sweep: failed rows the
    /// reconcile marked (never legit failures — those already got bounded
    /// in-session retries), unparented (wave/folder children belong to their
    /// re-drive, not the sweep), and under the retry cap so poison
    /// terminates across crashes. One row per task id: the latest attempt.
    pub async fn orphaned_task_rows(
        &self,
        max_retries: i32,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, i64, String, String, String, i32)> =
            sqlx_core::query_as::query_as(
                "SELECT DISTINCT ON (task_id) id, task_id, task_kind::text, folder_path, path, retry_number
                   FROM activity.task_executions
                  WHERE status = 'failed'
                    AND error_message = 'orphaned: daemon restarted while task was running'
                    AND parent_task_id IS NULL
                    AND retry_number < $1
                  ORDER BY task_id, started_at DESC",
            )
            .bind(max_retries)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("orphaned_task_rows: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(id, task_id, kind, folder_path, path, retry_number)| {
                serde_json::json!({
                    "id": id, "taskId": task_id, "kind": kind,
                    "folderPath": folder_path, "path": path, "retryNumber": retry_number,
                })
            })
            .collect())
    }

    // ── Knowledge Sources (federation endpoints) ──────────────────────
}
