use super::*;

#[allow(dead_code, clippy::type_complexity)]
impl PgStore {
    /// Upsert parsed transcript turns for a (source, session). Idempotent by
    /// (source, session_id, turn_index). Returns the number of rows written.
    pub async fn upsert_transcript_turns(
        &self,
        from: &TranscriptSource<'_>,
        turns: &[crate::transcript::TranscriptTurn],
    ) -> Result<u32, String> {
        let &TranscriptSource { source, session_id, family, provider, model } = from;
        let mut n = 0u32;
        for t in turns {
            let char_count = t.assistant_text.chars().count() as i32;
            sqlx_core::query::query(
                "INSERT INTO activity.transcript_turns
                    (source, session_id, family, provider, model, turn_index, user_text, assistant_text, char_count, started_at,
                     attrs, tokens_in, tokens_out, cache_read, cache_write, stop_reason, is_sidechain, skill, plugin, git_branch, effort, service_tier)
                 VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                        $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)
                 ON CONFLICT(source, session_id, turn_index) DO UPDATE SET
                   provider       = EXCLUDED.provider,
                   model          = EXCLUDED.model,
                   user_text      = EXCLUDED.user_text,
                   assistant_text = EXCLUDED.assistant_text,
                   char_count     = EXCLUDED.char_count,
                   started_at     = EXCLUDED.started_at,
                   -- COALESCE, not overwrite: a re-ingest by an adapter that does not
                   -- yet collect these must not erase what another pass captured.
                   -- `'{}'` AND `'null'`: an adapter that supplies neither must not
                   -- erase attributes another pass captured.
                   attrs          = CASE WHEN EXCLUDED.attrs IN ('{}'::jsonb, 'null'::jsonb)
                                         THEN activity.transcript_turns.attrs ELSE EXCLUDED.attrs END,
                   tokens_in      = COALESCE(EXCLUDED.tokens_in,    activity.transcript_turns.tokens_in),
                   tokens_out     = COALESCE(EXCLUDED.tokens_out,   activity.transcript_turns.tokens_out),
                   cache_read     = COALESCE(EXCLUDED.cache_read,   activity.transcript_turns.cache_read),
                   cache_write    = COALESCE(EXCLUDED.cache_write,  activity.transcript_turns.cache_write),
                   stop_reason    = COALESCE(EXCLUDED.stop_reason,  activity.transcript_turns.stop_reason),
                   is_sidechain   = COALESCE(EXCLUDED.is_sidechain, activity.transcript_turns.is_sidechain),
                   skill          = COALESCE(EXCLUDED.skill,        activity.transcript_turns.skill),
                   plugin         = COALESCE(EXCLUDED.plugin,       activity.transcript_turns.plugin),
                   git_branch     = COALESCE(EXCLUDED.git_branch,   activity.transcript_turns.git_branch),
                   effort         = COALESCE(EXCLUDED.effort,       activity.transcript_turns.effort),
                   service_tier   = COALESCE(EXCLUDED.service_tier, activity.transcript_turns.service_tier)"
            )
            .bind(source).bind(session_id).bind(family).bind(provider).bind(model).bind(t.turn_index)
            .bind(&t.user_text).bind(&t.assistant_text).bind(char_count).bind(t.started_at)
            .bind(&t.attrs)
            .bind(t.facts.tokens_in).bind(t.facts.tokens_out)
            .bind(t.facts.cache_read).bind(t.facts.cache_write)
            .bind(&t.facts.stop_reason).bind(t.facts.is_sidechain)
            .bind(&t.facts.skill).bind(&t.facts.plugin)
            .bind(&t.facts.git_branch).bind(&t.facts.effort).bind(&t.facts.service_tier)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
            n += 1;
        }
        Ok(n)
    }

    /// Replace a session's API messages (spec session-api-cost C2) in one
    /// transaction, so a re-read never leaves old and new rows side by side.
    pub async fn upsert_transcript_messages(
        &self,
        source: &str,
        session_id: &str,
        messages: &[crate::transcript::TranscriptMessage],
    ) -> Result<(), String> {
        let col = |f: fn(&crate::transcript::TranscriptMessage) -> i64| {
            messages.iter().map(f).collect::<Vec<i64>>()
        };
        let text = |f: fn(&crate::transcript::TranscriptMessage) -> Option<String>| {
            messages.iter().map(f).collect::<Vec<Option<String>>>()
        };
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx_core::query::query(
            "DELETE FROM activity.transcript_messages WHERE source = $1 AND session_id = $2",
        )
        .bind(source)
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        sqlx_core::query::query(
            "INSERT INTO activity.transcript_messages
                 (source, session_id, message_id, turn_index, sent_at, model, speed, inference_geo,
                  service_tier, input, cache_write_5m, cache_write_1h, cache_split_known,
                  cache_read, output, thinking)
             SELECT $1, $2, m.id, m.turn, to_timestamp(m.at_ms / 1000.0), m.model, m.speed, m.geo,
                    m.tier, m.input, m.w5, m.w1, m.known, m.cr, m.out, m.thinking
               FROM unnest($3::text[], $4::int[], $5::bigint[], $6::text[], $7::text[], $8::text[],
                           $9::text[], $10::bigint[], $11::bigint[], $12::bigint[], $13::bool[],
                           $14::bigint[], $15::bigint[], $16::bigint[])
                 AS m(id, turn, at_ms, model, speed, geo, tier, input, w5, w1, known, cr, out, thinking)",
        )
        .bind(source)
        .bind(session_id)
        .bind(messages.iter().map(|m| m.message_id.clone()).collect::<Vec<_>>())
        .bind(messages.iter().map(|m| m.turn_index).collect::<Vec<_>>())
        .bind(messages.iter().map(|m| m.at).collect::<Vec<_>>())
        .bind(messages.iter().map(|m| m.model.clone()).collect::<Vec<_>>())
        .bind(text(|m| m.speed.clone()))
        .bind(text(|m| m.inference_geo.clone()))
        .bind(text(|m| m.service_tier.clone()))
        .bind(col(|m| m.input))
        .bind(col(|m| m.cache_write_5m))
        .bind(col(|m| m.cache_write_1h))
        .bind(messages.iter().map(|m| m.cache_split_known).collect::<Vec<_>>())
        .bind(col(|m| m.cache_read))
        .bind(col(|m| m.output))
        .bind(messages.iter().map(|m| m.thinking).collect::<Vec<_>>())
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())
    }

    /// Clear the ingest watermarks of `sources` the first time `done` is seen, so
    /// each of their transcripts is read again under a changed rule. `true` when it
    /// cleared them; `false` once `done` is set.
    pub async fn reset_watermarks_once(
        &self,
        done: &str,
        sources: &[&str],
    ) -> Result<bool, String> {
        if self.get_config(done).await?.is_some() {
            return Ok(false);
        }
        sqlx_core::query::query("DELETE FROM activity.capture_watermarks WHERE source = ANY($1)")
            .bind(sources)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        self.set_config(done, &chrono::Utc::now().to_rfc3339()).await?;
        Ok(true)
    }

    /// How far ingest has read a transcript file, or `None` if never seen.
    pub async fn get_capture_watermark(
        &self,
        source: &str,
        file_path: &str,
    ) -> Result<Option<CaptureWatermark>, String> {
        let row: Option<(i64, bool)> = sqlx_core::query_as::query_as(
            "SELECT last_mtime_ns, untracked FROM activity.capture_watermarks
              WHERE source = $1 AND file_path = $2",
        )
        .bind(source)
        .bind(file_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(mtime_ns, untracked)| CaptureWatermark { mtime_ns, untracked }))
    }

    /// Advance the ingest cursor for a transcript file (idempotent upsert).
    pub async fn set_capture_watermark(
        &self,
        mark: &CaptureWatermarkRow<'_>,
    ) -> Result<(), String> {
        let &CaptureWatermarkRow { source, file_path, session_id, mtime_ns, turns } = mark;
        sqlx_core::query::query(
            "INSERT INTO activity.capture_watermarks
                (source, file_path, session_id, last_mtime_ns, turns_ingested, untracked, updated_at)
             VALUES($1, $2, $3, $4, $5, false, now())
             ON CONFLICT(source, file_path) DO UPDATE SET
               session_id     = EXCLUDED.session_id,
               last_mtime_ns  = EXCLUDED.last_mtime_ns,
               turns_ingested = EXCLUDED.turns_ingested,
               untracked      = false,
               updated_at     = now()",
        )
        .bind(source)
        .bind(file_path)
        .bind(session_id)
        .bind(mtime_ns)
        .bind(turns)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Ignore a transcript whose session belongs to no tracked folder (#125):
    /// drop the turns and messages an earlier pass kept for it, and record that
    /// it was read at `mtime_ns` and ignored, so it is not read again until it
    /// changes or a new repository is registered (which clears the mark — see
    /// `sensei.forget_untracked_transcripts`). One transaction: a crash cannot
    /// leave the mark without the cleanup, or the cleanup without the mark.
    pub async fn ignore_untracked_transcript(
        &self,
        source: &str,
        file_path: &str,
        session_id: &str,
        mtime_ns: i64,
    ) -> Result<(), String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        for table in ["activity.transcript_turns", "activity.transcript_messages"] {
            sqlx_core::query::query(&format!(
                "DELETE FROM {table} WHERE source = $1 AND session_id = $2"
            ))
            .bind(source)
            .bind(session_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("ignore_untracked_transcript {table}: {e}"))?;
        }
        sqlx_core::query::query(
            "INSERT INTO activity.capture_watermarks
                (source, file_path, session_id, last_mtime_ns, turns_ingested, untracked, updated_at)
             VALUES($1, $2, $3, $4, 0, true, now())
             ON CONFLICT(source, file_path) DO UPDATE SET
               session_id     = EXCLUDED.session_id,
               last_mtime_ns  = EXCLUDED.last_mtime_ns,
               turns_ingested = 0,
               untracked      = true,
               updated_at     = now()",
        )
        .bind(source)
        .bind(file_path)
        .bind(session_id)
        .bind(mtime_ns)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("ignore_untracked_transcript watermark: {e}"))?;
        tx.commit().await.map_err(|e| e.to_string())
    }

    /// Whether a session row exists for this client session id.
    pub async fn session_exists(&self, client_session_id: &str) -> Result<bool, String> {
        let (exists,): (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS (SELECT 1 FROM activity.sessions WHERE client_session_id = $1)",
        )
        .bind(client_session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("session_exists: {e}"))?;
        Ok(exists)
    }

    /// All transcript turns for a session (by client `session_id`), ordered by
    /// `turn_index`, plus the session's assistant `family` (the ACP — e.g.
    /// `claude` — the transcript actually came from; `None` when unknown). The
    /// transcript is the GROUND-TRUTH source the analyzer derives session signals
    /// from (turns / corrections / outcome); hook events only corroborate. Empty
    /// when a session has no captured transcript, in which case the analyzer falls
    /// back to the sparse hook stream.
    pub async fn get_transcript_turns_for_session(
        &self,
        client_session_id: &str,
    ) -> Result<(Vec<crate::transcript::TranscriptTurn>, Option<String>), String> {
        let rows: Vec<(
            i32,
            Option<String>,
            String,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<String>,
        )> = sqlx_core::query_as::query_as(
            "SELECT turn_index, user_text, assistant_text, started_at, family
                   FROM activity.transcript_turns
                  WHERE session_id = $1 ORDER BY turn_index",
        )
        .bind(client_session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        // family is uniform per session — take the first non-empty one.
        let family = rows.iter().find_map(|r| r.4.clone().filter(|f| !f.trim().is_empty()));
        let turns = rows
            .into_iter()
            .map(|(turn_index, user_text, assistant_text, started_at, _family)| {
                crate::transcript::TranscriptTurn {
                    turn_index,
                    user_text,
                    assistant_text,
                    started_at,
                    ..Default::default()
                }
            })
            .collect();
        Ok((turns, family))
    }

    // ── Historical-bootstrap import (#75) ────────────────────────────────────
}

/// A transcript file's ingest cursor — see [`PgStore::get_capture_watermark`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureWatermark {
    /// The source mtime (ns) the file was last read at.
    pub mtime_ns: i64,
    /// It was read then and ignored: its session belongs to no tracked folder.
    pub untracked: bool,
}
