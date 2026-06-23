//! Transcript ingestion (#73) — backfill the assistant/user prose the hook
//! stream lacks, from agent transcripts. Resumable per-adapter workers: each
//! adapter knows a source's cache layout + id scheme and parses files into
//! turns; ingestion advances a per-file cursor so re-runs only touch changed
//! files. Ingest is LLM-free — the analyzer's LLM tiers consume the corpus
//! selectively.
//!
//! Chunked execution: `BackfillTranscripts` is a *dispatcher* that enqueues one
//! `BackfillTranscriptFile` task per changed transcript, so ingestion interleaves
//! with other work (scans, etc.) and one huge/bad file can't block the rest.

pub mod claude;

use crate::tasks::executor::TaskContext;
use crate::tasks::{Task, TaskKind};
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

/// A synthesized hook-stream event reconstructed from a transcript (#75) — the
/// transcript is a superset of the live hook stream, so we can rebuild the
/// events the analyzer enriches from.
#[derive(Debug, Clone, PartialEq)]
pub struct SynthEvent {
    pub event_type: String, // UserPromptSubmit | PostToolUse | Stop
    pub tool_name: Option<String>,
    pub file_path: Option<String>,
    pub prompt: Option<String>,
    pub ts: i64, // ms epoch
}

/// A session reconstructed from a transcript: the cwds seen (for project
/// resolution) + the synthesized event stream (#75).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SynthSession {
    pub cwds: Vec<String>,
    pub events: Vec<SynthEvent>,
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
    /// Parse file content into prose turns.
    fn parse(&self, content: &str) -> Vec<TranscriptTurn>;
    /// Reconstruct a session's event stream for the historical-bootstrap import
    /// (#75). Adapters that can't synthesize return None.
    fn parse_session(&self, _content: &str) -> Option<SynthSession> {
        None
    }
}

#[cfg(test)]
#[derive(Debug, Default, PartialEq)]
pub struct BackfillReport {
    pub files_seen: u32,
    pub files_ingested: u32,
    pub files_skipped: u32,
    pub turns_upserted: u32,
}

/// Outcome of ingesting one transcript file.
struct IngestOutcome {
    #[cfg_attr(not(test), allow(dead_code))] // read by backfill_all (test helper)
    skipped: bool,
    turns: u32,
    /// Project to (re)analyze because a historical session was synthesized (#75).
    analyze_project: Option<uuid::Uuid>,
}

impl IngestOutcome {
    fn skipped() -> Self {
        Self { skipped: true, turns: 0, analyze_project: None }
    }
}

fn mtime_ns(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

fn claude_root() -> PathBuf {
    crate::paths::home().join(".claude/projects")
}

/// All configured transcript adapters.
fn adapters() -> Vec<Box<dyn TranscriptAdapter>> {
    vec![Box::new(claude::ClaudeAdapter::new(claude_root()))]
}

/// Resolve an adapter for a capture source (used by the per-file handler — the
/// root only matters for `transcript_files`, which the per-file path doesn't use).
fn adapter_for_source(source: &str) -> Option<Box<dyn TranscriptAdapter>> {
    match source {
        "claude_code" => Some(Box::new(claude::ClaudeAdapter::new(claude_root()))),
        _ => None,
    }
}

/// Ingest one transcript file: skip if unchanged since last ingest (cursor) or
/// oversized, else parse + upsert turns + advance the cursor. Idempotent.
async fn ingest_one(
    pg: &crate::db::pg_store::PgStore,
    adapter: &dyn TranscriptAdapter,
    path: &Path,
) -> Result<IngestOutcome, String> {
    let path_str = path.to_string_lossy().to_string();
    let mtime = mtime_ns(path);
    // resumable: skip files unchanged since last ingest.
    if let Ok(Some(prev)) = pg.get_transcript_cursor(adapter.source(), &path_str).await
        && prev >= mtime
    {
        return Ok(IngestOutcome::skipped());
    }
    let Some(session_id) = adapter.session_id_for(path) else {
        return Ok(IngestOutcome::skipped());
    };
    // skip pathological oversized transcripts (logged, not silent).
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size > MAX_TRANSCRIPT_BYTES {
        tracing::warn!(file = %path_str, size_mb = size / 1_048_576, "transcript ingest: skipping oversized transcript");
        return Ok(IngestOutcome::skipped());
    }
    let content = std::fs::read_to_string(path).map_err(|e| format!("read {path_str}: {e}"))?;
    // 1. prose turns (#73)
    let turns = adapter.parse(&content);
    let n = pg
        .upsert_transcript_turns(adapter.source(), &session_id, adapter.family(), &turns)
        .await?;
    pg.set_transcript_cursor(adapter.source(), &path_str, Some(&session_id), mtime, turns.len() as i32)
        .await?;
    // 2. historical-bootstrap: synthesize the session + events if not already
    // captured (#75), so the existing enricher can derive its metrics.
    let analyze_project = synthesize_session(pg, adapter, &session_id, &content).await;
    Ok(IngestOutcome { skipped: false, turns: n, analyze_project })
}

/// Historical-bootstrap (#75): if this session has no events yet (not
/// live-captured / not already imported), reconstruct it from the transcript —
/// resolve the project from a cwd, create the session, and synthesize its event
/// stream so `analyze_project` can enrich it. Returns the project to analyze.
async fn synthesize_session(
    pg: &crate::db::pg_store::PgStore,
    adapter: &dyn TranscriptAdapter,
    session_id: &str,
    content: &str,
) -> Option<uuid::Uuid> {
    let synth = adapter.parse_session(content)?;
    // dedup: never double-count a live-captured / already-imported session.
    match pg.session_has_events(session_id).await {
        Ok(true) => return None,
        Ok(false) => {}
        Err(e) => {
            tracing::warn!(error = %e, "synthesize_session: events check failed");
            return None;
        }
    }
    // project mapping: first cwd that is a tracked folder wins.
    let mut resolved = None;
    for cwd in &synth.cwds {
        if let Ok(Some((folder_id, project_id))) = pg.get_folder_ids_by_path(cwd).await {
            resolved = Some((cwd.clone(), folder_id, project_id));
            break;
        }
    }
    let Some((cwd, folder_id, project_id)) = resolved else {
        tracing::debug!(session = %session_id, "synthesize_session: no tracked folder for cwds — skipping");
        return None;
    };
    if let Err(e) = pg
        .record_session_event(session_id, &folder_id, project_id.as_ref(), adapter.family(), true)
        .await
    {
        tracing::warn!(error = %e, "synthesize_session: record_session_event failed");
        return None;
    }
    let started = synth.events.iter().map(|e| e.ts).min().unwrap_or(0);
    let completed = synth.events.iter().map(|e| e.ts).max().unwrap_or(0);
    let _ = pg.set_session_history(session_id, started, completed).await;
    for ev in &synth.events {
        let payload = match ev.event_type.as_str() {
            "UserPromptSubmit" => serde_json::json!({ "prompt": ev.prompt }),
            "PostToolUse" => serde_json::json!({ "tool_input": { "file_path": ev.file_path } }),
            _ => serde_json::json!({}),
        };
        if let Err(e) = pg
            .insert_hook_event(session_id, adapter.family(), &ev.event_type, ev.tool_name.as_deref(), Some(&cwd), ev.ts, None, &payload)
            .await
        {
            tracing::warn!(error = %e, session = %session_id, "synthesize_session: insert_hook_event failed");
        }
    }
    tracing::info!(session = %session_id, events = synth.events.len(), "synthesize_session: imported historical session");
    project_id
}

/// Ingest every file across all adapters in-process (no queue). Test helper
/// exercising the ingest+skip path end-to-end; the production path is the
/// chunked dispatcher (`run_backfill` -> per-file `run_backfill_file`).
#[cfg(test)]
pub async fn backfill_all(
    pg: &crate::db::pg_store::PgStore,
    adapters: &[Box<dyn TranscriptAdapter>],
) -> BackfillReport {
    let mut report = BackfillReport::default();
    for ad in adapters {
        for path in ad.transcript_files() {
            report.files_seen += 1;
            match ingest_one(pg, ad.as_ref(), &path).await {
                Ok(o) if o.skipped => report.files_skipped += 1,
                Ok(o) => {
                    report.files_ingested += 1;
                    report.turns_upserted += o.turns;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "transcript backfill: ingest failed");
                    report.files_skipped += 1;
                }
            }
        }
    }
    report
}

/// Dispatcher for `TaskKind::BackfillTranscripts`: enqueue one
/// `BackfillTranscriptFile` task per changed/new transcript so ingestion
/// interleaves with other work and a single huge/bad file can't block the rest.
/// Skips files unchanged since last ingest (the per-file task re-checks to stay
/// race-safe). Returns the number of files enqueued.
pub async fn run_backfill(ctx: &TaskContext, _task: &Task) -> Result<u32, String> {
    let (_seen, enqueued) = dispatch(ctx.pg(), &ctx.queue).await;
    Ok(enqueued)
}

/// Scan all adapters and enqueue one `BackfillTranscriptFile` task per
/// changed/new transcript. Returns `(files_seen, enqueued)`. Callable from the
/// dispatcher task or directly from the trigger endpoint (for immediate feedback).
pub async fn dispatch(
    pg: &crate::db::pg_store::PgStore,
    queue: &crate::tasks::queue::TaskQueue,
) -> (u32, u32) {
    let mut files_seen = 0u32;
    let mut enqueued = 0u32;
    for ad in adapters() {
        for path in ad.transcript_files() {
            files_seen += 1;
            let path_str = path.to_string_lossy().to_string();
            if let Ok(Some(prev)) = pg.get_transcript_cursor(ad.source(), &path_str).await
                && prev >= mtime_ns(&path)
            {
                continue;
            }
            // folder_path = capture source, path = transcript file path.
            queue
                .enqueue(Task::new(TaskKind::BackfillTranscriptFile, ad.source(), &path_str))
                .await;
            enqueued += 1;
        }
    }
    tracing::info!(files_seen, enqueued, "transcript backfill: dispatched per-file tasks");
    (files_seen, enqueued)
}

/// Handler for `TaskKind::BackfillTranscriptFile`: ingest one transcript.
/// `task.folder_path` = capture source, `task.path` = transcript file path.
pub async fn run_backfill_file(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let Some(adapter) = adapter_for_source(&task.folder_path) else {
        return Err(format!("unknown transcript source '{}'", task.folder_path));
    };
    let outcome = ingest_one(ctx.pg(), adapter.as_ref(), Path::new(&task.path)).await?;
    // A freshly-synthesized historical session needs enrichment to light up its
    // FTR/churn/correction signals (#75). AnalyzeProject is idempotent + incremental.
    if let Some(project_id) = outcome.analyze_project {
        ctx.queue
            .enqueue(Task::new(TaskKind::AnalyzeProject, "", &project_id.to_string()))
            .await;
    }
    Ok(outcome.turns)
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

        let ads: Vec<Box<dyn TranscriptAdapter>> =
            vec![Box::new(claude::ClaudeAdapter::new(root.clone()))];

        // first run ingests the two turns
        let r1 = backfill_all(&pg, &ads).await;
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
        let r2 = backfill_all(&pg, &ads).await;
        assert_eq!(r2.files_skipped, 1);
        assert_eq!(r2.files_ingested, 0);

        // cleanup
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM activity.transcript_turns WHERE session_id=$1").bind(&sid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM activity.transcript_cursor WHERE session_id=$1").bind(&sid).execute(pool).await.ok();
        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn synthesize_imports_historical_session() {
        let pg = crate::db::pg_store::PgStore::connect_test().await.unwrap();
        let pid = pg.create_project(&format!("_test:imp-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let root = pg.add_watch_root(&format!("/_test/imp-root-{}", uuid::Uuid::new_v4()), "t", &serde_json::json!([])).await.unwrap();
        let repo_path = format!("/_test/imp-repo-{}", uuid::Uuid::new_v4());
        let fid = pg.upsert_repo(&root, "imp-repo", &repo_path).await.unwrap();
        // link folder → project (scan/reconcile does this in production; the
        // importer resolves project_id from the folder via cwd).
        sqlx_core::query::query("UPDATE sensei.folders SET project_id=$1 WHERE id=$2")
            .bind(pid).bind(fid).execute(pg.pool()).await.unwrap();
        let sid = format!("_test-imp-{}", uuid::Uuid::new_v4());

        // a historical transcript whose cwd == the tracked folder's abs_path
        let content = format!(
            "{{\"type\":\"user\",\"cwd\":\"{cwd}\",\"timestamp\":\"2026-06-20T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"add the parser\"}}}}\n\
             {{\"type\":\"assistant\",\"cwd\":\"{cwd}\",\"timestamp\":\"2026-06-20T10:00:05.000Z\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"tool_use\",\"name\":\"Edit\",\"input\":{{\"file_path\":\"{cwd}/src/x.rs\"}}}}]}}}}\n",
            cwd = repo_path
        );
        let root_dir = std::env::temp_dir().join(format!("sensei-imp-{}", uuid::Uuid::new_v4()));
        let proj = root_dir.join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join(format!("{sid}.jsonl")), &content).unwrap();

        let ads: Vec<Box<dyn TranscriptAdapter>> = vec![Box::new(claude::ClaudeAdapter::new(root_dir.clone()))];
        backfill_all(&pg, &ads).await;

        // session synthesized, attributed to the project, flagged backfilled,
        // with a historical started_at (not "today").
        let s: (Option<uuid::Uuid>, bool, bool) = sqlx_core::query_as::query_as(
            "SELECT project_id, backfilled, (started_at < now() - interval '1 day') FROM activity.sessions WHERE client_session_id=$1"
        ).bind(&sid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(s.0, Some(pid), "attributed to the project resolved from cwd");
        assert!(s.1, "flagged backfilled");
        assert!(s.2, "started_at set from the transcript timestamp, not now()");

        // events synthesized: prompt + tool-edit + terminal Stop.
        let kinds: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT event_type FROM activity.assistant_events WHERE session_id=$1 ORDER BY ts"
        ).bind(&sid).fetch_all(pg.pool()).await.unwrap();
        let kinds: Vec<&str> = kinds.iter().map(|k| k.0.as_str()).collect();
        assert!(kinds.contains(&"UserPromptSubmit") && kinds.contains(&"PostToolUse") && kinds.contains(&"Stop"), "got {kinds:?}");
        let n_before = kinds.len();

        // re-run: file unchanged ⇒ cursor-skip, no duplicate events.
        backfill_all(&pg, &ads).await;
        let n: (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM activity.assistant_events WHERE session_id=$1")
            .bind(&sid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(n.0, n_before as i64, "re-run does not duplicate events");

        // cleanup
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM activity.assistant_events WHERE session_id=$1").bind(&sid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM activity.transcript_turns WHERE session_id=$1").bind(&sid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM activity.transcript_cursor WHERE session_id=$1").bind(&sid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE client_session_id=$1").bind(&sid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id=$1").bind(fid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id=$1").bind(root).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id=$1").bind(pid).execute(pool).await.ok();
        std::fs::remove_dir_all(&root_dir).ok();
    }
}
