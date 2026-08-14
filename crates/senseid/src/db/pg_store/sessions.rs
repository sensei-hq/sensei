use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    /// List all sessions across all folders.
    /// List recent sessions, newest first. `range_days` (when `Some`) filters to
    /// sessions started within the last N days — powers the Observatory · Sessions
    /// digest range chips (7d/30d/90d); `None` = no time filter. `project` (when
    /// `Some`) scopes to one project. `agent` (the acp harness, e.g. "claude" /
    /// "zed") lets the digest label each row's assistant.
    pub async fn list_all_sessions(
        &self,
        limit: i64,
        range_days: Option<i64>,
        project: Option<&uuid::Uuid>,
    ) -> Result<Vec<serde_json::Value>, String> {
        // Join the project name so each session can be labelled, and return the
        // timestamps in the camelCase shape the SessionData wire type and the
        // observatory components actually read (startedAt / completedAt). `corrections`
        // powers the "Corrections" column (first-try / N× rework) per the mockup.
        type SessionRow = (
            uuid::Uuid, Option<String>, String, Option<String>, Option<String>,
            Option<bool>, i32, i32, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>,
            Option<String>,
        );
        let rows: Vec<SessionRow> = sqlx_core::query_as::query_as(
            "SELECT s.id, p.name, s.task, s.summary, s.outcome::text, s.ftr, s.turns, s.corrections,
                    s.started_at, s.completed_at, s.acp_id
             FROM activity.sessions s
             LEFT JOIN sensei.projects p ON p.id = s.project_id
             WHERE ($2::int IS NULL OR s.started_at >= now() - make_interval(days => $2::int))
               AND ($3::uuid IS NULL OR s.project_id = $3)
             ORDER BY s.started_at DESC LIMIT $1"
        ).bind(limit).bind(range_days).bind(project).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, project, task, summary, outcome, ftr, turns, corrections, started, completed, agent)| {
            serde_json::json!({
                "id": id,
                "project": project,
                "task": task,
                "summary": summary,
                "outcome": outcome,
                "ftr": ftr,
                "turns": turns,
                "corrections": corrections,
                "startedAt": started.to_rfc3339(),
                "completedAt": completed.map(|c| c.to_rfc3339()),
                "agent": agent,
            })
        }).collect())
    }

    // ── Extensions ────────────────────────────────────────────────────

    pub async fn create_snapshot(
        &self, session_id: &uuid::Uuid, folder_id: &uuid::Uuid, kind: &str,
        progress: &str, next_step: Option<&str>, completed_steps: &[String],
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.snapshots(session_id, folder_id, kind, progress_summary, next_step_hint, completed_steps) VALUES($1, $2, $3::sensei.snapshot_kind, $4, $5, $6) RETURNING id"
        ).bind(session_id).bind(folder_id).bind(kind).bind(progress).bind(next_step).bind(completed_steps)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn get_latest_snapshot(&self, session_id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, Option<String>, Vec<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, kind::text, progress_summary, next_step_hint, completed_steps, created_at FROM activity.snapshots WHERE session_id = $1 ORDER BY created_at DESC LIMIT 1"
            ).bind(session_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id, kind, progress, next, steps, ts)| {
            serde_json::json!({ "id": id, "kind": kind, "progress_summary": progress, "next_step_hint": next, "completed_steps": steps, "created_at": ts.to_rfc3339() })
        }))
    }

    // ── Detected Patterns (inference) ──────────────────────────────────

    pub async fn create_session(&self, folder_id: &uuid::Uuid, task: &str, acp_id: Option<&str>) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.sessions(folder_id, task, acp_id) VALUES($1, $2, $3) RETURNING id"
        ).bind(folder_id).bind(task).bind(acp_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn complete_session(
        &self, id: &uuid::Uuid, outcome: &str, ftr: bool,
        turns: i32, corrections: i32,
        summary: Option<&str>, tokens_in: Option<i32>, tokens_out: Option<i32>,
    ) -> Result<(), String> {
        // summary/tokens are COALESCE'd so a caller that omits them doesn't wipe a
        // previously-set value; these columns exist on activity.sessions and were
        // being silently dropped (the MCP schema advertised them).
        sqlx_core::query::query(
            "UPDATE activity.sessions SET outcome = $2::sensei.session_outcome, ftr = $3, turns = $4, corrections = $5, \
             summary = COALESCE($6, summary), tokens_in = COALESCE($7, tokens_in), tokens_out = COALESCE($8, tokens_out), \
             completed_at = now() WHERE id = $1"
        ).bind(id).bind(outcome).bind(ftr).bind(turns).bind(corrections)
            .bind(summary).bind(tokens_in).bind(tokens_out)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Find-or-create the `activity.sessions` row for an assistant
    /// `client_session_id`, attributing it to `folder_id`/`project_id`. Marks it
    /// completed when `is_end` (Stop / SessionEnd). Idempotent per
    /// client_session_id so every hook event of a session folds into one row (#31).
    pub async fn record_session_event(
        &self, client_session_id: &str, folder_id: &uuid::Uuid,
        project_id: Option<&uuid::Uuid>, family: &str, is_end: bool,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.sessions (client_session_id, folder_id, project_id, acp_id, completed_at)
             VALUES ($1, $2, $3, $4, CASE WHEN $5 THEN now() ELSE NULL END)
             ON CONFLICT (client_session_id) WHERE client_session_id IS NOT NULL
             DO UPDATE SET
               completed_at = CASE WHEN $5 THEN now() ELSE activity.sessions.completed_at END,
               project_id   = COALESCE(activity.sessions.project_id, EXCLUDED.project_id)
             RETURNING id"
        ).bind(client_session_id).bind(folder_id).bind(project_id).bind(family).bind(is_end)
         .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Resolve `activity.sessions.id` (observatory UUID) → `client_session_id`
    /// (the string the assistant's hook writer stamps on every
    /// `activity.assistant_events` row). The Replay endpoint (#84 Slice C)
    /// needs this because `assistant_events.session_id` is the client id,
    /// not the UUID.
    pub async fn get_session_client_id(&self, id: &uuid::Uuid) -> Result<Option<String>, String> {
        let row: Option<(Option<String>,)> = sqlx_core::query_as::query_as(
            "SELECT client_session_id FROM activity.sessions WHERE id = $1"
        ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.and_then(|(c,)| c))
    }

    pub async fn get_session(&self, id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, uuid::Uuid, String, Option<String>, Option<String>, Option<bool>, i32, i32, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, folder_id, task, acp_id, outcome::text, ftr, turns, corrections, started_at, completed_at FROM activity.sessions WHERE id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(row.map(|(id, fid, task, acp, outcome, ftr, turns, corr, started, completed)| {
            serde_json::json!({
                "id": id, "folder_id": fid, "task": task, "acp_id": acp,
                "outcome": outcome, "ftr": ftr, "turns": turns, "corrections": corr,
                "started_at": started.to_rfc3339(),
                "completed_at": completed.map(|t| t.to_rfc3339()),
            })
        }))
    }

    pub async fn list_sessions_by_folder(&self, folder_id: &uuid::Uuid, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, Option<bool>, i32, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, task, outcome::text, ftr, corrections, started_at FROM activity.sessions WHERE folder_id = $1 ORDER BY started_at DESC LIMIT $2"
            ).bind(folder_id).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, task, outcome, ftr, corr, started)| {
            serde_json::json!({ "id": id, "task": task, "outcome": outcome, "ftr": ftr, "corrections": corr, "started_at": started.to_rfc3339() })
        }).collect())
    }

    // ── Assistant events ───────────────────────────────────────────────

    /// Insert a hook event payload into activity.assistant_events.
    /// session_id is the assistant's string session ID (not a DB UUID).
    /// assistant_family identifies the source (claude, cursor, zed, …); defaults to 'claude'.
    pub async fn insert_hook_event(
        &self,
        session_id: &str,
        assistant_family: &str,
        event_type: &str,
        tool_name: Option<&str>,
        cwd: Option<&str>,
        ts: i64,
        success: Option<bool>,
        payload: &serde_json::Value,
    ) -> Result<i64, String> {
        let row: (i64,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.assistant_events \
             (session_id, family, event_type, tool_name, cwd, ts, success, payload) \
             VALUES($1, $2::sensei.assistant_family, $3, $4, $5, $6, $7, $8) RETURNING id"
        )
        .bind(session_id)
        .bind(assistant_family)
        .bind(event_type)
        .bind(tool_name)
        .bind(cwd)
        .bind(ts)
        .bind(success)
        .bind(payload)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Insert a hook event only if an identical one isn't already stored, and
    /// return the new id (`None` when it was a duplicate). Used by the capture
    /// drain ([`crate::tasks::capture_drain`]) to import dead-lettered events
    /// without twinning a row the daemon already committed in the rare
    /// "curl timed out after the insert succeeded" race. Dedup is on the payload
    /// (identical on both the live POST and the fallback line) so it holds even
    /// though the two paths stamp `ts` independently.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_hook_event_if_absent(
        &self,
        session_id: &str,
        assistant_family: &str,
        event_type: &str,
        tool_name: Option<&str>,
        cwd: Option<&str>,
        ts: i64,
        success: Option<bool>,
        payload: &serde_json::Value,
    ) -> Result<Option<i64>, String> {
        let row: Option<(i64,)> = sqlx_core::query_as::query_as(
            "INSERT INTO activity.assistant_events \
             (session_id, family, event_type, tool_name, cwd, ts, success, payload) \
             SELECT $1, $2::sensei.assistant_family, $3, $4, $5, $6, $7, $8 \
             WHERE NOT EXISTS ( \
               SELECT 1 FROM activity.assistant_events \
               WHERE session_id = $1 AND event_type = $3 \
                 AND tool_name IS NOT DISTINCT FROM $4 AND payload = $8 \
             ) RETURNING id",
        )
        .bind(session_id)
        .bind(assistant_family)
        .bind(event_type)
        .bind(tool_name)
        .bind(cwd)
        .bind(ts)
        .bind(success)
        .bind(payload)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    /// Newest hook_event timestamp (epoch ms) for an assistant family, or None
    /// when the daemon has never recorded one for it. `assistant_family` is a
    /// Postgres enum, so bind with the explicit cast.
    pub async fn latest_hook_event_ts(&self, family: &str) -> Result<Option<i64>, String> {
        let row: (Option<i64>,) = sqlx_core::query_as::query_as(
            "SELECT max(ts) FROM activity.assistant_events WHERE family = $1::sensei.assistant_family"
        )
        .bind(family)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// All assistant events for one session (by its string `session_id`),
    /// oldest-first, projected to the fields session enrichment reads (#66).
    pub async fn get_hook_events_for_session(&self, client_session_id: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, Option<String>, i64, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT event_type, tool_name, ts, payload FROM activity.assistant_events
             WHERE session_id = $1 ORDER BY ts"
        ).bind(client_session_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(event_type, tool_name, ts, payload)| {
            serde_json::json!({ "event_type": event_type, "tool_name": tool_name, "ts": ts, "payload": payload })
        }).collect())
    }

    /// Same as [`get_hook_events_for_session`] but also returns the DB row id
    /// so the verdict classifier (#90) can reference each `PostToolUse` by
    /// its `activity.assistant_events.id`.
    pub async fn get_hook_events_for_session_with_id(
        &self,
        client_session_id: &str,
    ) -> Result<Vec<(i64, String, Option<String>, i64, serde_json::Value)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT id, event_type, tool_name, ts, payload FROM activity.assistant_events
             WHERE session_id = $1 ORDER BY ts"
        ).bind(client_session_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }

    /// Merge `props.resumed = true` onto a session (Phase B): it carried an
    /// in-session `command_invoked{action:resume}` marker, so it was reopened /
    /// continued — the read path shows "resumed" and never treats it as abandoned.
    /// Idempotent; guarded so a steady-state re-enrich changes 0 rows.
    pub async fn set_session_resumed(&self, session_id: &uuid::Uuid, resumed: bool) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE activity.sessions
                SET props = coalesce(props, '{}'::jsonb) || jsonb_build_object('resumed', $2::bool)
              WHERE id = $1
                AND coalesce((props->>'resumed')::bool, false) IS DISTINCT FROM $2"
        ).bind(session_id).bind(resumed).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    /// The most recent `TodoWrite` event for a session: its `(payload, cwd)`.
    /// `None` when the session has no `TodoWrite` yet. Feeds the relay
    /// segment-publish path (P2) — `payload` holds the todo list
    /// (`payload.tool_input.todos`, projected by [`crate::dojo::relay_project`]);
    /// `cwd` names the working folder for the run title. Reads the jsonb column
    /// straight into `serde_json::Value` (same pattern as
    /// [`Self::get_hook_events_for_session`]).
    pub async fn latest_todowrite(
        &self,
        session_id: &str,
    ) -> Result<Option<(serde_json::Value, Option<String>)>, String> {
        let row: Option<(serde_json::Value, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT payload, cwd FROM activity.assistant_events
             WHERE session_id = $1 AND tool_name = 'TodoWrite' ORDER BY ts DESC LIMIT 1"
        ).bind(session_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Upsert a batch of tool-call verdicts (#90). Idempotent: repeated calls
    /// with a new heuristic just refresh the row (`ON CONFLICT (event_id) DO
    /// UPDATE`). Returns the number of rows written.
    pub async fn upsert_verdicts_batch(
        &self,
        rows: &[(String, i64, Option<String>, &'static str, f32, String)],
    ) -> Result<usize, String> {
        if rows.is_empty() { return Ok(0); }
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        for (session_id, event_id, tool_name, verdict, confidence, reason) in rows {
            sqlx_core::query::query(
                "INSERT INTO sensei.tool_call_verdicts \
                    (session_id, event_id, tool_name, verdict, confidence, reason, classified_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, now()) \
                 ON CONFLICT (event_id) DO UPDATE SET \
                    tool_name = EXCLUDED.tool_name, \
                    verdict = EXCLUDED.verdict, \
                    confidence = EXCLUDED.confidence, \
                    reason = EXCLUDED.reason, \
                    classified_at = now()"
            )
            .bind(session_id)
            .bind(event_id)
            .bind(tool_name)
            .bind(*verdict)
            .bind(*confidence)
            .bind(reason)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(rows.len())
    }

    /// Distinct session ids that still need verdict classification: sessions
    /// with a `PostToolUse` event inside the window that have no rows in
    /// `sensei.tool_call_verdicts` yet. Feeds the scheduled classifier
    /// (`ClassifyPendingVerdicts`) so the Health-tab aggregate reflects every
    /// session, not just the ones whose Replay tab was opened.
    ///
    /// Unclassified-only is a cheap gap-fill: it bounds the per-tick cost so we
    /// don't re-classify the whole corpus each scheduler tick. Correctness for
    /// anything already classified is covered by `upsert_verdicts_batch`'s
    /// idempotent upsert.
    ///
    /// `assistant_events.ts` is epoch millis (bigint), so the window cutoff is
    /// computed in millis — mirrors `get_tools_health`'s 14-day `PostToolUse`
    /// window; the parametrised-days form mirrors `get_verdict_split_per_tool`.
    /// Session-less events (`session_id = ''`) are excluded — they'd otherwise
    /// collapse every unattached event into one pseudo-session.
    pub async fn unclassified_verdict_sessions(&self, window_days: i32) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT h.session_id
               FROM activity.assistant_events h
              WHERE h.event_type = 'PostToolUse'
                AND h.session_id <> ''
                AND h.ts >= (extract(epoch from now() - ($1::int || ' days')::interval) * 1000)::bigint
                AND NOT EXISTS (
                    SELECT 1 FROM sensei.tool_call_verdicts v WHERE v.session_id = h.session_id
                )"
        )
        .bind(window_days)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(s,)| s).collect())
    }

    // ── #84 Track 2 Slice B — MCP tool manifest cache ─────────────────────

    /// Per-tool verdict counts (`used` / `partial` / `ignored`) over the
    /// last N days. Feeds the Health tab's "usage split %" via
    /// `aggregate_tool_insights` (#84 T2 Slice D). Zero-row tools that
    /// still appear in `tool_usage_stats` land with all-zero counts on the
    /// caller side; this method returns only tools that have at least one
    /// classified verdict in the window.
    pub async fn get_verdict_split_per_tool(
        &self,
        days: i32,
    ) -> Result<Vec<(String, i64, i64, i64)>, String> {
        let rows: Vec<(String, i64, i64, i64)> = sqlx_core::query_as::query_as(
            "SELECT COALESCE(tool_name, '') AS tool_name,
                    count(*) FILTER (WHERE verdict = 'used')::bigint    AS used,
                    count(*) FILTER (WHERE verdict = 'partial')::bigint AS partial,
                    count(*) FILTER (WHERE verdict = 'ignored')::bigint AS ignored
               FROM sensei.tool_call_verdicts
              WHERE classified_at > now() - ($1::int || ' days')::interval
              GROUP BY tool_name
              HAVING count(*) > 0"
        )
        .bind(days)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    // ── #83 T1 commands surface — project_commands writer + reader ────────

    /// Session timeline for the Replay tab (#84 T2 Slice C). Same
    /// paired-call shape as [`get_session_tool_calls`], but also joins
    /// `sensei.tool_call_verdicts` (#90) on the underlying PostToolUse
    /// event id so each row carries the usage verdict.
    ///
    /// The existing view [`sensei.session_tool_calls`] keys on the
    /// PreToolUse event id (call_id); verdicts are keyed on the
    /// PostToolUse event id. This query recomputes both directly against
    /// `activity.assistant_events` so we can LEFT JOIN verdicts on the
    /// PostToolUse id without changing either the view or the verdicts
    /// table.
    pub async fn get_session_replay_timeline(
        &self,
        session_id: &str,
        limit: i32,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(
            i64,                                              // pre_id (call_id)
            Option<i64>,                                      // post_id
            String,                                           // tool_name
            String,                                           // family
            serde_json::Value,                                // request
            Option<serde_json::Value>,                        // response
            Option<bool>,                                     // success
            i64,                                              // pre_ts
            Option<i64>,                                      // post_ts
            Option<i64>,                                      // duration_ms
            Option<String>,                                   // verdict
            Option<f32>,                                      // confidence
            Option<String>,                                   // reason
        )> = sqlx_core::query_as::query_as(
            "WITH pre AS (
                SELECT session_id, family::text AS family, tool_name,
                       id AS pre_id, ts AS pre_ts, payload AS request,
                       row_number() OVER (
                           PARTITION BY session_id, tool_name
                           ORDER BY ts, id
                       ) AS seq
                  FROM activity.assistant_events
                 WHERE event_type = 'PreToolUse'
                   AND tool_name IS NOT NULL
                   AND session_id = $1
            ),
            post AS (
                SELECT session_id, tool_name,
                       id AS post_id, ts AS post_ts,
                       payload AS response, success,
                       row_number() OVER (
                           PARTITION BY session_id, tool_name
                           ORDER BY ts, id
                       ) AS seq
                  FROM activity.assistant_events
                 WHERE event_type = 'PostToolUse'
                   AND tool_name IS NOT NULL
                   AND session_id = $1
            )
            SELECT pre.pre_id, post.post_id, pre.tool_name, pre.family,
                   pre.request, post.response, post.success,
                   pre.pre_ts, post.post_ts,
                   CASE WHEN post.post_ts IS NULL THEN NULL
                        ELSE GREATEST(post.post_ts - pre.pre_ts, 0)
                   END AS duration_ms,
                   v.verdict, v.confidence, v.reason
              FROM pre
              LEFT JOIN post ON pre.session_id = post.session_id
                            AND pre.tool_name  = post.tool_name
                            AND pre.seq        = post.seq
              LEFT JOIN sensei.tool_call_verdicts v ON v.event_id = post.post_id
             ORDER BY pre.pre_ts ASC
             LIMIT $2"
        )
        .bind(session_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(
            pre_id, post_id, tool_name, family, request, response, success,
            pre_ts, post_ts, duration_ms, verdict, confidence, reason,
        )| {
            serde_json::json!({
                "callId":         pre_id,
                "postEventId":    post_id,
                "toolName":       tool_name,
                "family":         family,
                "request":        request,
                "response":       response,
                "success":        success,
                "startedAtMs":    pre_ts,
                "completedAtMs":  post_ts,
                "durationMs":     duration_ms,
                "inFlight":       post_ts.is_none(),
                "verdict":        verdict,    // null when unclassified
                "confidence":     confidence,
                "verdictReason":  reason,
            })
        }).collect())
    }

    // ── #84 Track 2 Slice A — mcp_servers ─────────────────────────────────

    /// All verdicts for one session, ordered by the underlying event ts.
    /// Consumed by the Replay tab's timeline read path (#84 / #90).
    pub async fn get_verdicts_for_session(
        &self,
        client_session_id: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(i64, Option<String>, String, f32, Option<String>, chrono::DateTime<chrono::Utc>, i64)> =
            sqlx_core::query_as::query_as(
                "SELECT v.event_id, v.tool_name, v.verdict, v.confidence, v.reason,
                        v.classified_at, ae.ts
                   FROM sensei.tool_call_verdicts v
                   JOIN activity.assistant_events ae ON ae.id = v.event_id
                  WHERE v.session_id = $1
               ORDER BY ae.ts"
            )
            .bind(client_session_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(event_id, tool_name, verdict, confidence, reason, classified_at, ts)| {
            serde_json::json!({
                "event_id":       event_id,
                "tool_name":      tool_name,
                "verdict":        verdict,
                "confidence":     confidence,
                "reason":         reason,
                "classified_at":  classified_at.to_rfc3339(),
                "ts":             ts,
            })
        }).collect())
    }

    /// Session-level summary of verdicts — the counts by outcome. Cheap to
    /// project into a StatBlock on the Replay/Health tab.
    pub async fn get_verdict_summary_for_session(
        &self,
        client_session_id: &str,
    ) -> Result<serde_json::Value, String> {
        let rows: Vec<(String, i64)> = sqlx_core::query_as::query_as(
            "SELECT verdict, count(*)::bigint FROM sensei.tool_call_verdicts
              WHERE session_id = $1 GROUP BY verdict"
        )
        .bind(client_session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let mut used = 0i64;
        let mut partial = 0i64;
        let mut ignored = 0i64;
        for (v, n) in rows {
            match v.as_str() {
                "used" => used = n,
                "partial" => partial = n,
                "ignored" => ignored = n,
                _ => {}
            }
        }
        let total = used + partial + ignored;
        Ok(serde_json::json!({
            "used":    used,
            "partial": partial,
            "ignored": ignored,
            "total":   total,
        }))
    }

    /// `(session uuid, client_session_id)` for a project's sessions that NEED
    /// (re)enrichment — never analyzed (`analyzed_at IS NULL`), or with
    /// assistant_events newer than the last analysis. Lets the scheduler skip
    /// unchanged sessions so enrichment cost scales with NEW activity, not total
    /// history (#67 incremental).
    pub async fn get_project_sessions_needing_enrichment(&self, project_id: &uuid::Uuid) -> Result<Vec<(uuid::Uuid, String, uuid::Uuid)>, String> {
        let rows: Vec<(uuid::Uuid, String, uuid::Uuid)> = sqlx_core::query_as::query_as(
            "SELECT s.id, s.client_session_id, s.folder_id FROM activity.sessions s
             WHERE s.project_id = $1 AND s.client_session_id IS NOT NULL
               AND (s.analyzed_at IS NULL
                    OR EXISTS (SELECT 1 FROM activity.assistant_events e
                               WHERE e.session_id = s.client_session_id
                                 AND to_timestamp(e.ts / 1000.0) > s.analyzed_at))"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// The MEASURABLE sessions that ran on `day` for a project — the data behind a
    /// daily metric datapoint (the datapoint→sessions drill-down). Project scope is
    /// the folder-join (`sensei.folders.project_id`), matching how the daily metrics
    /// are computed (`session_outcomes`), so the set here is exactly the base a daily
    /// point was measured over — the fix for the old un-scoped digest that returned a
    /// fixed, project-agnostic three. `outcome IS NOT NULL` restricts to measurable
    /// (analyzed) sessions; `date_trunc('day', started_at)::date = day` pins the
    /// calendar day. Each row carries the structural fields the client renders a
    /// one-liner from (`outcome` + `ftr` + `turns` + `corrections`) plus the
    /// `client_session_id` reference, `started_at`, `task`, and the existing
    /// `summary` column (may be empty for backfilled sessions — the LLM per-session
    /// summary is a separate workstream). Newest-first. Empty when no measurable
    /// session ran that day (honest-empty, not a failure); propagates the read error.
    pub async fn get_project_sessions_for_day(
        &self,
        project_id: &uuid::Uuid,
        day: chrono::NaiveDate,
    ) -> Result<Vec<serde_json::Value>, String> {
        type Row = (
            Option<String>, chrono::DateTime<chrono::Utc>, Option<String>,
            Option<bool>, i32, i32, String, Option<String>,
        );
        let rows: Vec<Row> = sqlx_core::query_as::query_as(
            "SELECT s.client_session_id, s.started_at, s.outcome::text, s.ftr,
                    s.turns, s.corrections, s.task, s.summary
               FROM activity.sessions s
               JOIN sensei.folders  f ON f.id = s.folder_id
              WHERE f.project_id = $1
                AND s.outcome   IS NOT NULL
                AND s.outcome   <> 'empty'::sensei.session_outcome
                AND date_trunc('day', s.started_at)::date = $2
              ORDER BY s.started_at DESC",
        )
        .bind(project_id)
        .bind(day)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(client_session_id, started_at, outcome, ftr, turns, corrections, task, summary)| {
                serde_json::json!({
                    "client_session_id": client_session_id,
                    "started_at":        started_at.to_rfc3339(),
                    "outcome":           outcome,
                    "ftr":               ftr,
                    "turns":             turns,
                    "corrections":       corrections,
                    "task":              task,
                    "summary":           summary,
                })
            })
            .collect())
    }

    /// `(project_id, latest_session_activity)` for every project with attributed
    /// sessions — drives the analyzer scheduler's "what changed since last run"
    /// check (#67).
    pub async fn get_projects_with_session_activity(&self) -> Result<Vec<(uuid::Uuid, chrono::DateTime<chrono::Utc>)>, String> {
        let rows: Vec<(uuid::Uuid, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT project_id, max(GREATEST(started_at, COALESCE(completed_at, started_at)))
             FROM activity.sessions WHERE project_id IS NOT NULL GROUP BY project_id"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Per-(folder, file) re-edit churn across a project's sessions — the
    /// SignalDeriver's rework anti-pattern source (#68, L1). Tool failures aren't
    /// captured, so churn (the same file edited many times in ONE session) is the
    /// "result needed follow-ups" signal. Returns `(folder_id, file,
    /// max_session_edits, total_edits)` only for files whose busiest single
    /// session reaches `min_session_edits`.
    /// `folders = Some(ids)` scopes derivation to just those folders (incremental
    /// re-derive — only folders touched by newly-enriched sessions); `None` =
    /// the whole project (full refresh / on-demand).
    pub async fn get_file_churn_stats(&self, project_id: &uuid::Uuid, min_session_edits: i64, folders: Option<&[uuid::Uuid]>) -> Result<Vec<(uuid::Uuid, String, i64, i64)>, String> {
        let rows: Vec<(uuid::Uuid, String, i64, i64)> = sqlx_core::query_as::query_as(
            "WITH per_session AS (
                 SELECT s.folder_id,
                        ae.payload->'tool_input'->>'file_path' AS file,
                        ae.session_id,
                        count(*) AS edits
                 FROM activity.assistant_events ae
                 JOIN activity.sessions s ON s.client_session_id = ae.session_id
                 WHERE s.project_id = $1
                   AND ($3::uuid[] IS NULL OR s.folder_id = ANY($3))
                   AND ae.event_type = 'PostToolUse'
                   AND ae.tool_name IN ('Edit', 'Write', 'MultiEdit')
                   AND ae.payload->'tool_input'->>'file_path' IS NOT NULL
                 GROUP BY s.folder_id, ae.payload->'tool_input'->>'file_path', ae.session_id
             )
             SELECT folder_id, file,
                    max(edits)::bigint AS max_session_edits,
                    sum(edits)::bigint AS total_edits
             FROM per_session
             GROUP BY folder_id, file
             HAVING max(edits) >= $2"
        ).bind(project_id).bind(min_session_edits).bind(folders.map(<[uuid::Uuid]>::to_vec)).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// User-prompt text across a project's sessions — the SignalDeriver's
    /// correction / rule-candidate source (#68, L1). Returns `(folder_id,
    /// session_id, prompt)` for every UserPromptSubmit carrying prompt text.
    /// `folders = Some(ids)` scopes to those folders (incremental re-derive);
    /// `None` = whole project.
    pub async fn get_project_prompts(&self, project_id: &uuid::Uuid, folders: Option<&[uuid::Uuid]>) -> Result<Vec<(uuid::Uuid, String, String)>, String> {
        let rows: Vec<(uuid::Uuid, String, String)> = sqlx_core::query_as::query_as(
            "SELECT s.folder_id, ae.session_id, ae.payload->>'prompt'
             FROM activity.assistant_events ae
             JOIN activity.sessions s ON s.client_session_id = ae.session_id
             WHERE s.project_id = $1
               AND ($2::uuid[] IS NULL OR s.folder_id = ANY($2))
               AND ae.event_type = 'UserPromptSubmit'
               AND ae.payload->>'prompt' IS NOT NULL
             ORDER BY ae.ts"
        ).bind(project_id).bind(folders.map(<[uuid::Uuid]>::to_vec)).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    // ── Transcript backfill (#73) ────────────────────────────────────────────

    /// Re-attach orphaned sessions: `activity.assistant_events` rows whose session
    /// no longer has an `activity.sessions` row (its folder was cascade-deleted on a
    /// repo delete/rename, but the events — session-id-keyed, no FK — survived). For
    /// each, recreate the session row, resolving `folder_id` from the session's cwd
    /// via [`find_folder_for_path`] (alias-aware, so a renamed repo's history
    /// re-attaches to the current folder + project). Sessions whose cwd still doesn't
    /// resolve (no folder, no alias) are left orphaned. Idempotent — a session that
    /// already has a row isn't reprocessed. Returns the number repaired.
    pub async fn repair_orphaned_sessions(&self) -> Result<u32, String> {
        // All distinct cwds per orphaned session. We try them MOST-SPECIFIC (longest)
        // first: a renamed subdir (`…/monorepo/docs`, aliased to the new repo) is a
        // deeper — and thus stronger — signal than a still-live parent (`…/strategos`)
        // that would otherwise shadow it and misattribute. find_folder_for_path is
        // alias-aware, so the longest matching path wins via its alias.
        let orphans: Vec<(String, Vec<String>, String)> = sqlx_core::query_as::query_as(
            "SELECT e.session_id,
                    COALESCE(array_agg(DISTINCT e.cwd) FILTER (WHERE e.cwd IS NOT NULL), '{}') AS cwds,
                    (array_agg(e.family::text))[1] AS family
               FROM activity.assistant_events e
              WHERE e.session_id <> ''
                AND NOT EXISTS (
                    SELECT 1 FROM activity.sessions s WHERE s.client_session_id = e.session_id)
              GROUP BY e.session_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let mut repaired = 0u32;
        for (session_id, mut cwds, family) in orphans {
            cwds.sort_by_key(|c| std::cmp::Reverse(c.len())); // most-specific first
            let mut resolved = None;
            for cwd in &cwds {
                if let Ok(Some(fp)) = self.find_folder_for_path(cwd).await {
                    resolved = Some(fp);
                    break;
                }
            }
            let Some((folder_id, project_id)) = resolved else {
                continue; // no cwd resolves (no folder, no alias) — leave orphaned
            };
            if self
                .record_session_event(&session_id, &folder_id, project_id.as_ref(), &family, true)
                .await
                .is_ok()
            {
                repaired += 1;
            }
        }
        Ok(repaired)
    }

    /// True if a session already has captured/imported events — the dedup guard
    /// so the importer never double-counts a live-captured (or already-imported) session.
    pub async fn session_has_events(&self, client_session_id: &str) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM activity.assistant_events WHERE session_id = $1)"
        ).bind(client_session_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Mark a session as synthesized from a historical transcript (#75) and set
    /// its real historical start/end from the transcript timestamps (so it
    /// doesn't masquerade as "today" in the FTR/quality time windows).
    pub async fn set_session_history(&self, client_session_id: &str, started_ms: i64, completed_ms: i64) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.sessions SET backfilled = true,
                 started_at   = to_timestamp($2::float8 / 1000.0),
                 completed_at = to_timestamp($3::float8 / 1000.0)
             WHERE client_session_id = $1"
        ).bind(client_session_id).bind(started_ms).bind(completed_ms)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Set the inference `provider` + `model` that ran a session (captured from
    /// the transcript at synthesis, #75). Idempotent. Powers effectiveness-by-model.
    pub async fn set_session_model(
        &self, client_session_id: &str, provider: &str, model: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.sessions SET provider = $2, model = $3 WHERE client_session_id = $1",
        )
        .bind(client_session_id)
        .bind(provider)
        .bind(model)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Write enrichment metrics onto a session (#66). Sets the derived fields
    /// and merges `tool_usage` into `props` — deliberately does NOT touch
    /// `completed_at` (owned by the hook-stream session derivation, #31).
    pub async fn update_session_metrics(
        &self, session_id: &uuid::Uuid, turns: i32, corrections: i32, outcome: &str,
        ftr: bool, duration_ms: i64, module: Option<&str>, tool_usage: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.sessions
                SET outcome = $2::sensei.session_outcome, ftr = $3, turns = $4,
                    corrections = $5, duration = make_interval(secs => $6::float8 / 1000.0),
                    module = $7, analyzed_at = now(),
                    props = props || jsonb_build_object('tool_usage', $8::jsonb)
              WHERE id = $1"
        ).bind(session_id).bind(outcome).bind(ftr).bind(turns).bind(corrections)
            .bind(duration_ms).bind(module).bind(tool_usage)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Persist a session's retrospective summary — but only when the row has no
    /// summary yet (NULL or blank). `activity.sessions.summary` may be authored
    /// by the assistant at checkpoint time; this fills the (large) gap of empty
    /// summaries with the analyzer-derived narrative without ever clobbering an
    /// existing one. Idempotent and safe to re-run — a populated summary is left
    /// untouched. Called from `analyze::enrich_session` on each analysis pass.
    pub async fn set_session_summary_if_empty(&self, session_id: &uuid::Uuid, summary: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.sessions SET summary = $2
              WHERE id = $1 AND (summary IS NULL OR btrim(summary) = '')"
        ).bind(session_id).bind(summary).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Replace a session's per-turn rows (#66). Deletes the session's existing
    /// turns and re-inserts from a JSON array `[{turn_number, segment,
    /// started_ms, ended_ms, duration_ms, is_correction, triage_signal,
    /// tool_calls}]` — ms epochs/durations are converted to timestamptz/interval
    /// here. Idempotent (delete + reinsert), so re-enrichment never duplicates.
    pub async fn replace_session_turns(&self, session_id: &uuid::Uuid, turns: &serde_json::Value) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM activity.turns WHERE session_id = $1")
            .bind(session_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        sqlx_core::query::query(
            "INSERT INTO activity.turns
               (session_id, turn_number, segment, started_at, ended_at, duration, is_correction, triage_signal, tool_calls)
             SELECT $1, (t->>'turn_number')::int, (t->>'segment')::int,
                    to_timestamp((t->>'started_ms')::bigint / 1000.0),
                    to_timestamp((t->>'ended_ms')::bigint / 1000.0),
                    make_interval(secs => (t->>'duration_ms')::bigint / 1000.0),
                    (t->>'is_correction')::bool, t->>'triage_signal', (t->>'tool_calls')::int
             FROM jsonb_array_elements($2::jsonb) t"
        ).bind(session_id).bind(turns).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Projects ──────────────────────────────────────────────────────

    /// Recent sessions for a project with the folder role they ran in (the
    /// multi-repo membership chip). Newest first, capped at `limit`. Duration
    /// and relative time are formatted client-side from the ISO timestamps, the
    /// same as the shared RecentSessions component.
    pub async fn list_recent_project_sessions_with_role(&self, project_id: &uuid::Uuid, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(uuid::Uuid, Option<String>, Option<bool>, i32, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT s.id, s.task, s.ftr, s.corrections, s.started_at, s.completed_at, f.role::text
                 FROM activity.sessions s
                 LEFT JOIN sensei.folders f ON f.id = s.folder_id
                 WHERE s.project_id = $1
                 ORDER BY s.started_at DESC LIMIT $2"
            ).bind(project_id).bind(limit).fetch_all(&self.pool).await
            .map_err(|e| { tracing::error!(error = %e, "list_recent_project_sessions_with_role failed"); e.to_string() })?;
        Ok(rows.into_iter().map(|(id, task, ftr, corrections, started, completed, role)| {
            serde_json::json!({
                "id": id,
                "title": task,
                "ftr": ftr,
                "corrections": corrections,
                "startedAt": started.to_rfc3339(),
                "completedAt": completed.map(|t| t.to_rfc3339()),
                "role": role,
            })
        }).collect())
    }

    /// Return the paired PreToolUse / PostToolUse timeline for an assistant
    /// session, ordered by call start. Each row carries the request payload,
    /// the response payload (null when the call is still in-flight or the
    /// PostToolUse was dropped), the success flag, and duration_ms. Backed
    /// by the `sensei.session_tool_calls` view — see its DDL for the
    /// pairing rule.
    pub async fn get_session_tool_calls(
        &self,
        session_id: &str,
        limit: i32,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(
            i64,                                            // call_id
            String,                                         // tool_name
            String,                                         // family
            serde_json::Value,                              // request
            Option<serde_json::Value>,                      // response
            Option<bool>,                                   // success
            i64,                                            // started_at_ms
            Option<i64>,                                    // completed_at_ms
            Option<i64>,                                    // duration_ms
            chrono::DateTime<chrono::Utc>,                  // started_at
            Option<chrono::DateTime<chrono::Utc>>,          // completed_at
        )> = sqlx_core::query_as::query_as(
            "SELECT call_id, tool_name, family::text, request, response, success,
                    started_at_ms, completed_at_ms, duration_ms,
                    started_at, completed_at
               FROM sensei.session_tool_calls
              WHERE session_id = $1
              ORDER BY started_at_ms ASC
              LIMIT $2"
        )
        .bind(session_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(
            call_id, tool_name, family, request, response, success,
            started_at_ms, completed_at_ms, duration_ms,
            started_at, completed_at,
        )| {
            serde_json::json!({
                "callId":         call_id,
                "toolName":       tool_name,
                "family":         family,
                "request":        request,
                "response":       response,
                "success":        success,
                "startedAtMs":    started_at_ms,
                "completedAtMs":  completed_at_ms,
                "durationMs":     duration_ms,
                "startedAt":      started_at.to_rfc3339(),
                "completedAt":    completed_at.map(|t| t.to_rfc3339()),
                "inFlight":       completed_at_ms.is_none(),
            })
        }).collect())
    }

    pub async fn list_sessions_by_project(&self, project_id: &uuid::Uuid, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        // Extended shape (T3 Slice 1.4): the Sessions screen needs model,
        // provider, turns, corrections, and completed_at so the row can
        // render date / model / turns / corrections / FTR / outcome
        // side-by-side per the mockup. `outcome` is nullable while a
        // session is still in-flight — decode it Option to keep the query
        // resilient.
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            uuid::Uuid,                                     // id
            String,                                         // task
            Option<bool>,                                   // ftr
            Option<String>,                                 // outcome
            chrono::DateTime<chrono::Utc>,                  // started_at
            Option<chrono::DateTime<chrono::Utc>>,          // completed_at
            i32,                                            // turns
            i32,                                            // corrections
            Option<String>,                                 // provider
            Option<String>,                                 // model
        )> = sqlx_core::query_as::query_as(
                "SELECT id, task, ftr, outcome::text, started_at, completed_at,
                        turns, corrections, provider, model
                 FROM activity.sessions WHERE project_id = $1
                 ORDER BY started_at DESC LIMIT $2"
            ).bind(project_id).bind(limit)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, task, ftr, outcome, started, completed, turns, corrections, provider, model)| {
            serde_json::json!({
                "id":           id,
                "task":         task,
                "ftr":          ftr,
                "outcome":      outcome,
                "startedAt":    started.to_rfc3339(),
                "completedAt":  completed.map(|t| t.to_rfc3339()),
                "turns":        turns,
                "corrections":  corrections,
                "provider":     provider,
                "model":        model,
            })
        }).collect())
    }

}
