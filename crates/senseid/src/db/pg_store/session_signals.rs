//! Derived SDLC signals per session (`activity.session_signals`,
//! `activity.session_signal_marks`; spec tracker-sdlc-metrics S2).

use super::*;
use crate::sdlc::classify::Category;
use crate::sdlc::classify::FolderCommand;
use crate::sdlc::signals::{HoldReason, Signal, SignalEvent, SignalKind, TEXT_TAIL_BYTES};
use crate::sdlc::ticket::SessionSignals;

/// A session's derivation mark (`activity.session_signal_marks`): the inputs its
/// signals reflect, or failed on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalMark {
    pub session_id: uuid::Uuid,
    /// Highest event id read; 0 for no events, -1 when the read itself failed.
    pub through_event_id: i64,
    pub event_count: i32,
    pub catalogue_hash: String,
    /// `None` when derived.
    pub error: Option<String>,
}

/// A session the derivation pass should derive (again).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalCandidate {
    pub session_id: uuid::Uuid,
    pub client_session_id: String,
    pub repo_folder_id: Option<uuid::Uuid>,
}

impl<'r> sqlx_core::from_row::FromRow<'r, sqlx_postgres::PgRow> for SignalEvent {
    fn from_row(r: &'r sqlx_postgres::PgRow) -> Result<Self, sqlx_core::Error> {
        use sqlx_core::row::Row;
        Ok(SignalEvent {
            id: r.try_get("id")?,
            event_type: r.try_get("event_type")?,
            tool_name: r.try_get("tool_name")?,
            ts: r.try_get("ts")?,
            tool_use_id: r.try_get("tool_use_id")?,
            command: r.try_get("command")?,
            path: r.try_get("path")?,
            subagent_type: r.try_get("subagent_type")?,
            state: r.try_get("state")?,
            task_id: r.try_get("task_id")?,
            message: r.try_get("message")?,
            response_type: r.try_get("response_type")?,
            agent_ref: r.try_get("agent_ref")?,
            old_string: r.try_get("old_string")?,
            new_string: r.try_get("new_string")?,
            content: r.try_get("content")?,
            original_file: r.try_get("original_file")?,
            output: r.try_get("output")?,
            notification_type: r.try_get("notification_type")?,
            is_interrupt: r.try_get("is_interrupt")?,
            last_assistant_message: r.try_get("last_assistant_message")?,
        })
    }
}

impl PgStore {
    /// Sessions whose signals are missing or stale (spec S2 step 2), oldest first:
    /// no mark, or the events' `max(id)` / `count(*)` differ from the mark, or the
    /// catalogue changed. A failed mark is selected again only when one of those
    /// inputs changes. Sessions without a `client_session_id` have no events and
    /// are never selected.
    pub async fn sessions_needing_signals(
        &self,
        catalogue_hash: &str,
        limit: i64,
    ) -> Result<Vec<SignalCandidate>, String> {
        let db = |e: sqlx_core::Error| format!("sessions_needing_signals: {e}");
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx_core::query::query("SET LOCAL statement_timeout = '120s'")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let rows: Vec<(uuid::Uuid, String, Option<uuid::Uuid>)> = sqlx_core::query_as::query_as(
            "SELECT s.id, s.client_session_id, s.repo_folder_id
               FROM activity.sessions s
               LEFT JOIN activity.session_signal_marks m ON m.session_id = s.id
               CROSS JOIN LATERAL (
                   SELECT coalesce(max(e.id), 0) AS max_id, count(*)::int AS n
                     FROM activity.assistant_events e
                    WHERE e.session_id = s.client_session_id
               ) ev
              WHERE s.client_session_id IS NOT NULL
                AND (m.session_id IS NULL
                     OR m.through_event_id <> ev.max_id
                     OR m.event_count <> ev.n
                     OR m.catalogue_hash <> $1)
              ORDER BY s.started_at, s.id
              LIMIT $2",
        )
        .bind(catalogue_hash)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(session_id, client_session_id, repo_folder_id)| SignalCandidate {
                session_id,
                client_session_id,
                repo_folder_id,
            })
            .collect())
    }

    /// A session's events, projected to what derivation reads and ordered by
    /// `(ts, id)` (spec S2 "Reading a session's events"). Output and review text is
    /// capped at its last [`TEXT_TAIL_BYTES`]; edit bodies are read only for
    /// `tasks.md`.
    pub async fn session_events_for_signals(
        &self,
        client_session_id: &str,
    ) -> Result<Vec<SignalEvent>, String> {
        sqlx_core::query_as::query_as(
            "SELECT id, event_type, tool_name, ts, tool_use_id,
                    payload->'tool_input'->>'command' AS command,
                    coalesce(payload->'tool_input'->>'file_path',
                             payload->'tool_input'->>'notebook_path') AS path,
                    payload->'tool_input'->>'subagent_type' AS subagent_type,
                    payload->'tool_input'->>'state' AS state,
                    payload->'tool_input'->>'task_id' AS task_id,
                    right(payload->'tool_input'->>'message', $2) AS message,
                    payload->'tool_response'->>'type' AS response_type,
                    coalesce(payload->'tool_response'->>'agentId', payload->>'agent_id') AS agent_ref,
                    CASE WHEN t.tasks THEN payload->'tool_input'->>'old_string' END AS old_string,
                    CASE WHEN t.tasks THEN payload->'tool_input'->>'new_string' END AS new_string,
                    CASE WHEN t.tasks THEN payload->'tool_input'->>'content' END AS content,
                    CASE WHEN t.tasks THEN payload->'tool_response'->>'originalFile' END AS original_file,
                    CASE
                      WHEN tool_name LIKE 'mcp\\_\\_%' THEN right((payload->'tool_response')::text, $2)
                      WHEN payload ? 'tool_response' OR payload ? 'error' THEN right(
                          concat_ws(E'\\n', payload->'tool_response'->>'stdout',
                                            payload->'tool_response'->>'stderr',
                                            payload->>'error'), $2)
                    END AS output,
                    payload->>'notification_type' AS notification_type,
                    CASE WHEN jsonb_typeof(payload->'is_interrupt') = 'boolean'
                         THEN (payload->>'is_interrupt')::boolean END AS is_interrupt,
                    right(payload->>'last_assistant_message', $2) AS last_assistant_message
               FROM activity.assistant_events,
                    LATERAL (SELECT coalesce(payload->'tool_input'->>'file_path', '') LIKE '%tasks.md' AS tasks) t
              WHERE session_id = $1
              ORDER BY ts, id",
        )
        .bind(client_session_id)
        .bind(TEXT_TAIL_BYTES as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("session_events_for_signals({client_session_id}): {e}"))
    }

    /// The discovered commands of a repository folder and every folder beneath it
    /// (spec S3.2): what a session in that repository may run by name.
    pub async fn folder_commands_under(
        &self,
        folder_id: uuid::Uuid,
    ) -> Result<Vec<FolderCommand>, String> {
        let rows: Vec<(String, Option<String>)> = sqlx_core::query_as::query_as(
            "WITH RECURSIVE under AS (
                 SELECT id FROM sensei.folders WHERE id = $1
                 UNION
                 SELECT f.id FROM sensei.folders f JOIN under u ON f.parent_id = u.id
             )
             SELECT c.command_line, c.category
               FROM sensei.folder_commands c
               JOIN under u ON u.id = c.folder_id
              ORDER BY c.command_line",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("folder_commands_under({folder_id}): {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(command_line, category)| FolderCommand { command_line, category })
            .collect())
    }

    /// The stored signals of these sessions, each with whether they reflect its
    /// current events and `catalogue_hash` (spec S6 "Derivation must be current"),
    /// in the sessions' start order.
    pub async fn session_signals_for(
        &self,
        session_ids: &[uuid::Uuid],
        catalogue_hash: &str,
    ) -> Result<Vec<(uuid::Uuid, SessionSignals)>, String> {
        let db = |e: sqlx_core::Error| format!("session_signals_for: {e}");
        let state: Vec<(uuid::Uuid, bool, bool)> = sqlx_core::query_as::query_as(
            "SELECT s.id, coalesce(m.error IS NULL AND fresh, false),
                    coalesce(m.error IS NOT NULL AND fresh, false)
               FROM activity.sessions s
               LEFT JOIN activity.session_signal_marks m ON m.session_id = s.id
               CROSS JOIN LATERAL (
                   SELECT coalesce(max(e.id), 0) AS max_id, count(*)::int AS n
                     FROM activity.assistant_events e
                    WHERE e.session_id = s.client_session_id
               ) ev
               CROSS JOIN LATERAL (
                   SELECT m.catalogue_hash = $2 AND m.through_event_id = ev.max_id
                          AND m.event_count = ev.n AS fresh
               ) f
              WHERE s.id = ANY($1)
              ORDER BY s.started_at, s.id",
        )
        .bind(session_ids)
        .bind(catalogue_hash)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        type Row = (
            uuid::Uuid,
            String,
            chrono::DateTime<chrono::Utc>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<String>,
            Option<String>,
            Option<String>,
        );
        let rows: Vec<Row> = sqlx_core::query_as::query_as(
            "SELECT session_id, kind::text, at, until, category, reason::text, ref
               FROM activity.session_signals
              WHERE session_id = ANY($1)
              ORDER BY session_id, seq",
        )
        .bind(session_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        let mut by_session: std::collections::HashMap<uuid::Uuid, Vec<Signal>> = Default::default();
        for (session_id, kind, at, until, category, reason, reference) in rows {
            let kind =
                SignalKind::parse(&kind).ok_or_else(|| format!("unknown signal kind {kind}"))?;
            by_session.entry(session_id).or_default().push(Signal {
                kind,
                at: at.timestamp_millis(),
                until: until.map(|u| u.timestamp_millis()),
                category: category.as_deref().and_then(Category::parse),
                reason: reason.as_deref().and_then(HoldReason::parse),
                reference,
            });
        }
        Ok(state
            .into_iter()
            .map(|(id, current, failed)| {
                let signals = by_session.remove(&id).unwrap_or_default();
                (id, SessionSignals { current, failed, signals })
            })
            .collect())
    }

    /// Replace a session's signals and mark it derived through the events read, in
    /// one transaction (spec S2 step 3).
    pub async fn store_session_signals(
        &self,
        mark: &SignalMark,
        signals: &[Signal],
    ) -> Result<(), String> {
        let session_id = mark.session_id;
        let db = |e: sqlx_core::Error| format!("store_session_signals({session_id}): {e}");
        let ms = |t: i64| chrono::DateTime::<chrono::Utc>::from_timestamp_millis(t);
        let mut seq = Vec::with_capacity(signals.len());
        let mut kind = Vec::with_capacity(signals.len());
        let mut at = Vec::with_capacity(signals.len());
        let mut until = Vec::with_capacity(signals.len());
        let mut category = Vec::with_capacity(signals.len());
        let mut reason = Vec::with_capacity(signals.len());
        let mut reference = Vec::with_capacity(signals.len());
        for (i, s) in signals.iter().enumerate() {
            seq.push(i as i32);
            kind.push(s.kind.as_str());
            at.push(ms(s.at).ok_or_else(|| format!("signal time {} out of range", s.at))?);
            until.push(s.until.and_then(ms));
            category.push(s.category.map(|c| c.as_str()));
            reason.push(s.reason.map(|r| r.as_str()));
            reference.push(s.reference.as_deref());
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx_core::query::query("DELETE FROM activity.session_signals WHERE session_id = $1")
            .bind(session_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx_core::query::query(
            "INSERT INTO activity.session_signals
                 (session_id, seq, kind, at, until, category, reason, ref)
             SELECT $1, s, k::sensei.session_signal_kind, a, u, c, r::sensei.hold_reason, f
               FROM unnest($2::int[], $3::text[], $4::timestamptz[], $5::timestamptz[],
                           $6::text[], $7::text[], $8::text[]) AS x(s, k, a, u, c, r, f)",
        )
        .bind(session_id)
        .bind(&seq)
        .bind(&kind)
        .bind(&at)
        .bind(&until)
        .bind(&category)
        .bind(&reason)
        .bind(&reference)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        upsert_mark(&mut tx, mark).await.map_err(db)?;
        tx.commit().await.map_err(db)
    }

    /// Record that a session's derivation failed, with the inputs it failed on: its
    /// signals are removed and it is not selected again until they change.
    pub async fn store_session_signal_failure(&self, mark: &SignalMark) -> Result<(), String> {
        let session_id = mark.session_id;
        let db = |e: sqlx_core::Error| format!("store_session_signal_failure({session_id}): {e}");
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx_core::query::query("DELETE FROM activity.session_signals WHERE session_id = $1")
            .bind(session_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        upsert_mark(&mut tx, mark).await.map_err(db)?;
        tx.commit().await.map_err(db)
    }
}

async fn upsert_mark(
    tx: &mut sqlx_core::transaction::Transaction<'_, sqlx_postgres::Postgres>,
    mark: &SignalMark,
) -> Result<(), sqlx_core::Error> {
    sqlx_core::query::query(
        "INSERT INTO activity.session_signal_marks
             (session_id, through_event_id, event_count, catalogue_hash, error, derived_at)
         VALUES ($1, $2, $3, $4, $5, now())
         ON CONFLICT (session_id) DO UPDATE SET
             through_event_id = excluded.through_event_id
           , event_count      = excluded.event_count
           , catalogue_hash   = excluded.catalogue_hash
           , error            = excluded.error
           , derived_at       = now()",
    )
    .bind(mark.session_id)
    .bind(mark.through_event_id)
    .bind(mark.event_count)
    .bind(&mark.catalogue_hash)
    .bind(&mark.error)
    .execute(&mut **tx)
    .await
    .map(|_| ())
}
