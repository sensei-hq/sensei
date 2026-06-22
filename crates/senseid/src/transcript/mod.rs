//! Transcript ingestion (#73) — backfill the assistant/user prose the hook
//! stream lacks, from agent transcripts. Resumable per-adapter workers: each
//! adapter knows a source's cache layout + id scheme and parses files into
//! turns; the coordinator upserts them and advances a per-file cursor so
//! re-runs only ingest changed files. Ingest is LLM-free — the analyzer's LLM
//! tiers consume the corpus selectively.

pub mod claude;

use crate::tasks::executor::TaskContext;
use crate::tasks::Task;
use std::path::{Path, PathBuf};

/// Skip transcript files larger than this (logged). A multi-hundred-MB file
/// would spike memory on read and block the executor on parse; rare outlier.
const MAX_TRANSCRIPT_BYTES: u64 = 64 * 1024 * 1024;

/// One user-prompt -> assistant-response turn parsed from a transcript.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptTurn {
    pub turn_index: i32,
    pub user_text: Option<String>,
    pub assistant_text: String,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// A per-source transcript reader. Parsing is pure + deterministic (testable);
/// the coordinator owns IO + DB so new sources (Zed, ACP) are drop-in.
pub trait TranscriptAdapter: Send + Sync {
    /// Capture origin (distinct from the model `family`), e.g. "claude_code".
    fn source(&self) -> &'static str;
    /// Model family, e.g. "claude".
    fn family(&self) -> &'static str;
    /// Transcript files this adapter can ingest.
    fn transcript_files(&self) -> Vec<PathBuf>;
    /// Session id for a transcript file (Claude: the filename stem).
    fn session_id_for(&self, path: &Path) -> Option<String>;
    /// Parse file content into turns.
    fn parse(&self, content: &str) -> Vec<TranscriptTurn>;
}

#[derive(Debug, Default, PartialEq)]
pub struct BackfillReport {
    pub files_seen: u32,
    pub files_ingested: u32,
    pub files_skipped: u32,
    pub turns_upserted: u32,
}

fn mtime_ns(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

/// Run all adapters: for each transcript file, skip if unchanged since the last
/// ingest (cursor mtime), else parse + upsert turns + advance the cursor.
/// Idempotent — safe to re-run; only changed files do work.
pub async fn backfill(
    pg: &crate::db::pg_store::PgStore,
    adapters: &[Box<dyn TranscriptAdapter>],
) -> BackfillReport {
    let mut report = BackfillReport::default();
    for ad in adapters {
        for path in ad.transcript_files() {
            report.files_seen += 1;
            let path_str = path.to_string_lossy().to_string();
            let mtime = mtime_ns(&path);
            // resumable: skip files unchanged since last ingest.
            if let Ok(Some(prev)) = pg.get_transcript_cursor(ad.source(), &path_str).await
                && prev >= mtime
            {
                report.files_skipped += 1;
                continue;
            }
            let Some(session_id) = ad.session_id_for(&path) else {
                report.files_skipped += 1;
                continue;
            };
            // skip pathological oversized transcripts (logged, not silent).
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            if size > MAX_TRANSCRIPT_BYTES {
                tracing::warn!(file = %path_str, size_mb = size / 1_048_576, "transcript backfill: skipping oversized transcript");
                report.files_skipped += 1;
                continue;
            }
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(error = %e, file = %path_str, "transcript backfill: read failed");
                    report.files_skipped += 1;
                    continue;
                }
            };
            let turns = ad.parse(&content);
            match pg
                .upsert_transcript_turns(ad.source(), &session_id, ad.family(), &turns)
                .await
            {
                Ok(n) => report.turns_upserted += n,
                Err(e) => {
                    tracing::warn!(error = %e, file = %path_str, "transcript backfill: upsert failed");
                    continue;
                }
            }
            let _ = pg
                .set_transcript_cursor(ad.source(), &path_str, Some(&session_id), mtime, turns.len() as i32)
                .await;
            report.files_ingested += 1;
        }
    }
    report
}

/// Handler for `TaskKind::BackfillTranscripts`: backfill from all known
/// transcript sources under the user's home (currently Claude Code). Returns
/// the number of turns upserted.
pub async fn run_backfill(ctx: &TaskContext, _task: &Task) -> Result<u32, String> {
    let root = crate::paths::home().join(".claude/projects");
    let adapters: Vec<Box<dyn TranscriptAdapter>> = vec![Box::new(claude::ClaudeAdapter::new(root))];
    let report = backfill(ctx.pg(), &adapters).await;
    tracing::info!(
        files_seen = report.files_seen,
        ingested = report.files_ingested,
        skipped = report.files_skipped,
        turns = report.turns_upserted,
        "transcript backfill complete"
    );
    Ok(report.turns_upserted)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
{"type":"user","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"add a login page"}}
{"type":"assistant","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","content":[{"type":"text","text":"On it."}]}}
{"type":"user","timestamp":"2026-06-22T10:00:04.000Z","message":{"role":"user","content":[{"type":"tool_result","content":"ok"}]}}
{"type":"user","timestamp":"2026-06-22T10:01:00.000Z","message":{"role":"user","content":"now wire it up"}}
{"type":"assistant","timestamp":"2026-06-22T10:01:01.000Z","message":{"role":"assistant","content":[{"type":"text","text":"Wired."}]}}
"#;

    #[tokio::test]
    async fn backfill_ingests_then_skips_unchanged() {
        let pg = crate::db::pg_store::PgStore::connect_test().await.unwrap();
        let sid = format!("_test-tx-{}", uuid::Uuid::new_v4());
        // temp transcript: <root>/proj/<sid>.jsonl (mirrors ~/.claude layout)
        let root = std::env::temp_dir().join(format!("sensei-tx-{}", uuid::Uuid::new_v4()));
        let proj = root.join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join(format!("{sid}.jsonl")), SAMPLE).unwrap();

        let adapters: Vec<Box<dyn TranscriptAdapter>> =
            vec![Box::new(claude::ClaudeAdapter::new(root.clone()))];

        // first run ingests the two turns
        let r1 = backfill(&pg, &adapters).await;
        assert_eq!(r1.files_ingested, 1);
        assert_eq!(r1.turns_upserted, 2);

        let row: (i64, String, String) = sqlx_core::query_as::query_as(
            "SELECT count(*), max(source), max(assistant_text) FILTER (WHERE turn_index=2)
             FROM activity.transcript_turns WHERE session_id=$1"
        ).bind(&sid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(row.0, 2, "two turns stored");
        assert_eq!(row.1, "claude_code", "tagged with capture source");
        assert_eq!(row.2, "Wired.", "assistant prose captured per turn");

        // re-run: unchanged file ⇒ skipped via the cursor, no new work
        let r2 = backfill(&pg, &adapters).await;
        assert_eq!(r2.files_skipped, 1);
        assert_eq!(r2.files_ingested, 0);

        // cleanup
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM activity.transcript_turns WHERE session_id=$1").bind(&sid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM activity.transcript_cursor WHERE session_id=$1").bind(&sid).execute(pool).await.ok();
        std::fs::remove_dir_all(&root).ok();
    }
}
