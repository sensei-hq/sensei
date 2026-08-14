use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    /// Upsert parsed transcript turns for a (source, session). Idempotent by
    /// (source, session_id, turn_index). Returns the number of rows written.
    pub async fn upsert_transcript_turns(
        &self, source: &str, session_id: &str, family: &str,
        provider: Option<&str>, model: Option<&str>,
        turns: &[crate::transcript::TranscriptTurn],
    ) -> Result<u32, String> {
        let mut n = 0u32;
        for t in turns {
            let char_count = t.assistant_text.chars().count() as i32;
            sqlx_core::query::query(
                "INSERT INTO activity.transcript_turns
                    (source, session_id, family, provider, model, turn_index, user_text, assistant_text, char_count, started_at)
                 VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                 ON CONFLICT(source, session_id, turn_index) DO UPDATE SET
                   provider       = EXCLUDED.provider,
                   model          = EXCLUDED.model,
                   user_text      = EXCLUDED.user_text,
                   assistant_text = EXCLUDED.assistant_text,
                   char_count     = EXCLUDED.char_count,
                   started_at     = EXCLUDED.started_at"
            )
            .bind(source).bind(session_id).bind(family).bind(provider).bind(model).bind(t.turn_index)
            .bind(&t.user_text).bind(&t.assistant_text).bind(char_count).bind(t.started_at)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
            n += 1;
        }
        Ok(n)
    }

    /// Last-ingested mtime (ns) for a transcript file, or None if never seen.
    pub async fn get_transcript_cursor(&self, source: &str, file_path: &str) -> Result<Option<i64>, String> {
        let row: Option<(i64,)> = sqlx_core::query_as::query_as(
            "SELECT last_mtime_ns FROM activity.transcript_cursor WHERE source = $1 AND file_path = $2"
        ).bind(source).bind(file_path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    /// Advance the ingest cursor for a transcript file (idempotent upsert).
    pub async fn set_transcript_cursor(
        &self, source: &str, file_path: &str, session_id: Option<&str>, mtime_ns: i64, turns: i32,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO activity.transcript_cursor
                (source, file_path, session_id, last_mtime_ns, turns_ingested, updated_at)
             VALUES($1, $2, $3, $4, $5, now())
             ON CONFLICT(source, file_path) DO UPDATE SET
               session_id     = EXCLUDED.session_id,
               last_mtime_ns  = EXCLUDED.last_mtime_ns,
               turns_ingested = EXCLUDED.turns_ingested,
               updated_at     = now()"
        ).bind(source).bind(file_path).bind(session_id).bind(mtime_ns).bind(turns)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// All transcript turns for a session (by client `session_id`), ordered by
    /// `turn_index`, plus the session's assistant `family` (the ACP — e.g.
    /// `claude` — the transcript actually came from; `None` when unknown). The
    /// transcript is the GROUND-TRUTH source the analyzer derives session signals
    /// from (turns / corrections / outcome); hook events only corroborate. Empty
    /// when a session has no captured transcript, in which case the analyzer falls
    /// back to the sparse hook stream.
    pub async fn get_transcript_turns_for_session(
        &self, client_session_id: &str,
    ) -> Result<(Vec<crate::transcript::TranscriptTurn>, Option<String>), String> {
        let rows: Vec<(i32, Option<String>, String, Option<chrono::DateTime<chrono::Utc>>, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT turn_index, user_text, assistant_text, started_at, family
                   FROM activity.transcript_turns
                  WHERE session_id = $1 ORDER BY turn_index"
            ).bind(client_session_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        // family is uniform per session — take the first non-empty one.
        let family = rows.iter().find_map(|r| r.4.clone().filter(|f| !f.trim().is_empty()));
        let turns = rows.into_iter().map(|(turn_index, user_text, assistant_text, started_at, _family)| {
            crate::transcript::TranscriptTurn { turn_index, user_text, assistant_text, started_at }
        }).collect();
        Ok((turns, family))
    }

    // ── Historical-bootstrap import (#75) ────────────────────────────────────

}
