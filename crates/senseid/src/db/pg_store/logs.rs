use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    pub async fn log_index_error(
        &self, folder_id: &uuid::Uuid, file_path: &str, error: &str,
        adapter: Option<&str>, phase: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.index_errors(folder_id, file_path, error, adapter, phase) VALUES($1, $2, $3, $4, $5)"
        )
            .bind(folder_id).bind(file_path).bind(error).bind(adapter).bind(phase)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_index_errors(&self, folder_id: Option<&uuid::Uuid>) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<String>, chrono::DateTime<chrono::Utc>)> = match folder_id {
            Some(fid) => sqlx_core::query_as::query_as(
                "SELECT folder_id, file_path, error, adapter, phase, created_at FROM sensei.index_errors WHERE folder_id = $1 ORDER BY created_at DESC"
            ).bind(fid).fetch_all(&self.pool).await,
            None => sqlx_core::query_as::query_as(
                "SELECT folder_id, file_path, error, adapter, phase, created_at FROM sensei.index_errors ORDER BY created_at DESC LIMIT 200"
            ).fetch_all(&self.pool).await,
        }.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(fid, fp, err, adapter, phase, ts)| {
            serde_json::json!({
                "folder_id": fid, "file_path": fp, "error": err,
                "adapter": adapter, "phase": phase, "created_at": ts.to_rfc3339(),
            })
        }).collect())
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
            "DELETE FROM public.logs WHERE logged_at < now() - (interval '1 day' * $1)"
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
    ///   daily `scope = 'user'` `sensei.project_metrics` row for the session's
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
    pub async fn prune_activity(&self, days: i32, backstop_days: i32) -> Result<ActivityPruneCounts, String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        // (1) Snapshot eligible sessions once — used for every child delete
        //     so we don't re-scan the guard SQL four times. Capture-before-
        //     reclaim (invariant I20): a session is eligible only when its day is
        //     already captured by a CAPTURE-AUTHORIZING metric in
        //     sensei.project_metrics (daily grain, scope = 'user', on the
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
        //     just its project, keeps the guard honest at repo grain: a session
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
                               FROM sensei.project_metrics pm
                               JOIN sensei.metrics m ON m.id = pm.metric_id
                               JOIN sensei.folders rf ON rf.id = s.repo_folder_id
                              WHERE pm.repository_id = rf.repository_id
                                AND pm.scope = 'user'
                                AND pm.grain = 'daily'
                                AND pm.computed_on = date_trunc('day', s.started_at)::date
                                AND m.capture_source = 'session')
                     OR s.started_at < now() - (interval '1 day' * $2))"
        )
            .bind(days)
            .bind(backstop_days)
            .fetch_all(&mut *tx).await.map_err(|e| e.to_string())?;
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
                         WHERE s.client_session_id = ae.session_id))"
            )
                .bind(cutoff_ms)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            tx.commit().await.map_err(|e| e.to_string())?;
            return Ok(ActivityPruneCounts { assistant_events: ae.rows_affected(), ..Default::default() });
        }
        let session_uuids: Vec<uuid::Uuid> = eligible.iter().map(|(u, _)| *u).collect();
        let client_ids:    Vec<String>     = eligible.iter().map(|(_, c)| c.clone()).collect();

        // (2) Count turns that will cascade on the session delete — for the
        //     log line; the DELETE itself happens via CASCADE below.
        let turns_count: (i64,) = sqlx_core::query_as::query_as(
            "SELECT COUNT(*) FROM activity.turns WHERE session_id = ANY($1::uuid[])"
        )
            .bind(&session_uuids)
            .fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;

        // (3) transcript_turns keyed by client_session_id (text, no FK).
        let tt = sqlx_core::query::query(
            "DELETE FROM activity.transcript_turns WHERE session_id = ANY($1::text[])"
        )
            .bind(&client_ids)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;

        // (4) assistant_events for the same client_session_ids.
        let ae_session = sqlx_core::query::query(
            "DELETE FROM activity.assistant_events WHERE session_id = ANY($1::text[])"
        )
            .bind(&client_ids)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;

        // (5) sessions — cascades turns.
        let sess = sqlx_core::query::query(
            "DELETE FROM activity.sessions WHERE id = ANY($1::uuid[])"
        )
            .bind(&session_uuids)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;

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
                         WHERE s.client_session_id = ae.session_id))"
        )
            .bind(cutoff_ms)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;

        Ok(ActivityPruneCounts {
            sessions:         sess.rows_affected(),
            turns:            turns_count.0.max(0) as u64,
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
             activity.assistant_events, activity.transcript_turns",
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    /// Execute a parameterized query returning unresolved edges.
    pub async fn execute_raw_query(&self, sql: &str, folder_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, Option<String>, String)> = sqlx_core::query_as::query_as(sql)
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
    pub async fn insert_log(
        &self,
        level: &str,
        running_on: &str,
        module: Option<&str>,
        logged_at: &str,
        message: &str,
        context: &serde_json::Value,
        data: &Option<serde_json::Value>,
        error: &Option<serde_json::Value>,
    ) -> Result<(), String> {
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
    pub async fn query_logs(
        &self,
        level: Option<&str>,
        source: Option<&str>,
        module: Option<&str>,
        since: Option<chrono::DateTime<chrono::Utc>>,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
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
        task_id: i64,
        parent_task_id: Option<i64>,
        task_kind: &str,
        folder_path: &str,
        path: &str,
        retry_number: i32,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.task_executions(task_id, parent_task_id, task_kind, folder_path, path, status, retry_number)
             VALUES($1, $2, $3, $4, $5, 'running', $6) RETURNING id"
        )
        .bind(task_id)
        .bind(parent_task_id)
        .bind(task_kind)
        .bind(folder_path)
        .bind(path)
        .bind(retry_number)
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
              WHERE id = $1"
        )
        .bind(id)
        .bind(duration_ms)
        .bind(error_message)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("fail_task_execution: {}", e))?;
        Ok(())
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
              WHERE status = 'running' AND started_at < $1"
        )
        .bind(session_start)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("reconcile_orphaned_task_executions: {}", e))?;
        Ok(res.rows_affected())
    }

    // ── Knowledge Sources (federation endpoints) ──────────────────────

}
