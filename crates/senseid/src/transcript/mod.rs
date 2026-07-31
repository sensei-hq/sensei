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
pub mod zed;

use crate::tasks::executor::TaskContext;
use crate::tasks::{Task, TaskKind};
use std::path::PathBuf;

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

/// A logical ingest unit — a transcript file (Claude) or a DB row (Zed) — with a
/// monotonic change-stamp the cursor uses to skip unchanged units. `key`
/// uniquely identifies the unit within its source.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitRef {
    pub key: String,
    pub stamp: i64,
}

/// A per-source transcript reader. Parsing is pure + deterministic (testable);
/// the coordinator owns the cursor + DB writes. Sources expose their content as
/// logical *units* (files for Claude, threads-in-a-SQLite-DB for Zed) rather
/// than assuming a file-per-session layout, so new sources are drop-in.
pub trait TranscriptAdapter: Send + Sync {
    /// Capture origin (distinct from the model `family`), e.g. "claude_code" / "zed".
    fn source(&self) -> &'static str;
    /// Harness/agent family (the `sensei.assistant_family` enum: "claude", "zed", …).
    /// The precise per-unit model, when known, comes from `model_for`.
    fn family(&self) -> &'static str;
    /// Units this adapter can ingest, each with a change-stamp for cursor skipping.
    fn units(&self) -> Vec<UnitRef>;
    /// Cheap change-stamp for a unit key (no content read) — the cursor
    /// pre-check reads this. `None` ⇒ the unit no longer exists.
    fn stamp_for(&self, key: &str) -> Option<i64>;
    /// Session id for a unit key (Claude: filename stem; Zed: `zed-<thread_id>`).
    fn session_id_for(&self, key: &str) -> Option<String>;
    /// Load a unit's raw content (the expensive read/decompress). `None` ⇒ gone
    /// or oversized (logged, never panics on bad data).
    fn load_content(&self, key: &str) -> Option<String>;
    /// Parse unit content into prose turns.
    fn parse(&self, content: &str) -> Vec<TranscriptTurn>;
    /// Reconstruct a session's event stream for the historical-bootstrap import
    /// (#75). Adapters that can't synthesize return None.
    fn parse_session(&self, _content: &str) -> Option<SynthSession> {
        None
    }
    /// Optional `(provider, model)` captured for a unit's content — Zed records
    /// it per thread; Claude returns `None`.
    fn model_for(&self, _content: &str) -> Option<(String, String)> {
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

fn claude_root() -> PathBuf {
    crate::paths::home().join(".claude/projects")
}

fn zed_db_path() -> PathBuf {
    crate::paths::home().join("Library/Application Support/Zed/threads/threads.db")
}

/// All configured transcript adapters.
fn adapters() -> Vec<Box<dyn TranscriptAdapter>> {
    vec![
        Box::new(claude::ClaudeAdapter::new(claude_root())),
        Box::new(zed::ZedAdapter::new(zed_db_path())),
    ]
}

/// Resolve an adapter for a capture source (used by the per-unit handler — the
/// root/db-path only matters for `units`, which the per-unit path doesn't use).
fn adapter_for_source(source: &str) -> Option<Box<dyn TranscriptAdapter>> {
    match source {
        "claude_code" => Some(Box::new(claude::ClaudeAdapter::new(claude_root()))),
        "zed" => Some(Box::new(zed::ZedAdapter::new(zed_db_path()))),
        _ => None,
    }
}

/// Ingest one transcript unit (a file for Claude, a thread row for Zed): skip if
/// unchanged since last ingest (cursor) or gone/oversized, else parse + upsert
/// turns + advance the cursor. Idempotent.
async fn ingest_one(
    pg: &crate::db::pg_store::PgStore,
    adapter: &dyn TranscriptAdapter,
    key: &str,
) -> Result<IngestOutcome, String> {
    let Some(session_id) = adapter.session_id_for(key) else {
        return Ok(IngestOutcome::skipped());
    };
    let Some(stamp) = adapter.stamp_for(key) else {
        return Ok(IngestOutcome::skipped()); // unit gone
    };
    // The cursor gates the expensive PROSE re-ingest; synthesis (#75) gates on
    // whether the session has events yet (so an already-prose-ingested but
    // never-synthesized historical session still gets imported). Load the
    // content only if there's something to do.
    let prose_fresh = matches!(
        pg.get_transcript_cursor(adapter.source(), key).await,
        Ok(Some(prev)) if prev >= stamp
    );
    let needs_synth = !pg.session_has_events(&session_id).await.unwrap_or(true);
    if prose_fresh && !needs_synth {
        return Ok(IngestOutcome::skipped());
    }
    // Adapter owns the read + its own oversize/format guards (returns None to skip).
    let Some(content) = adapter.load_content(key) else {
        return Ok(IngestOutcome::skipped());
    };
    // 1. prose turns (#73) — only when stale.
    let mut turns = 0u32;
    if !prose_fresh {
        let parsed = adapter.parse(&content);
        let model = adapter.model_for(&content);
        let (provider, model_name) = match &model {
            Some((p, m)) => (Some(p.as_str()), Some(m.as_str())),
            None => (None, None),
        };
        turns = pg
            .upsert_transcript_turns(adapter.source(), &session_id, adapter.family(), provider, model_name, &parsed)
            .await?;
        pg.set_transcript_cursor(adapter.source(), key, Some(&session_id), stamp, parsed.len() as i32)
            .await?;
    }
    // 2. historical-bootstrap: synthesize the session + events if not already
    // captured (#75), so the existing enricher can derive its metrics.
    let analyze_project = if needs_synth {
        synthesize_session(pg, adapter, &session_id, &content).await
    } else {
        None
    };
    Ok(IngestOutcome { skipped: false, turns, analyze_project })
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
    // project mapping: first cwd that is a tracked folder (exact) wins. Both the
    // exact lookup and the ancestor fallback are alias-aware, so a cwd recorded
    // under a since-renamed path resolves to the current folder. When no cwd is an
    // exact folder, fall back to the nearest tracked ANCESTOR — so a subdirectory
    // cwd (or a subdir of a renamed repo covered by a single root alias) still
    // attributes to the right project instead of being dropped.
    let mut resolved = None;
    for cwd in &synth.cwds {
        if let Ok(Some((folder_id, project_id))) = pg.get_folder_ids_by_path(cwd).await {
            resolved = Some((cwd.clone(), folder_id, project_id));
            break;
        }
    }
    if resolved.is_none() {
        for cwd in &synth.cwds {
            if let Ok(Some((folder_id, project_id))) = pg.find_folder_for_path(cwd).await {
                resolved = Some((cwd.clone(), folder_id, project_id));
                break;
            }
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
    // Session start/completed timestamps power the "Duration" column
    // on the observatory. If this write fails silently the row still
    // shows up but with a blank duration — surface the failure so the
    // observability isn't itself invisible.
    if let Err(e) = pg.set_session_history(session_id, started, completed).await {
        tracing::warn!(error = %e, session = %session_id, "synthesize_session: set_session_history failed");
    }
    // Capture the inference model that ran this session (Zed: per-thread; Claude:
    // dominant transcript model) so insights can be attributed by model.
    if let Some((provider, model)) = adapter.model_for(content)
        && let Err(e) = pg.set_session_model(session_id, &provider, &model).await
    {
        tracing::warn!(error = %e, session = %session_id, "synthesize_session: set_session_model failed");
    }
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
        for unit in ad.units() {
            report.files_seen += 1;
            match ingest_one(pg, ad.as_ref(), &unit.key).await {
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
    let (_seen, enqueued) = dispatch(&ctx.queue).await;
    // Re-attach sessions orphaned by a repo delete/rename (events survived, the
    // session row was cascade-deleted) — resolves each via its cwd, now alias-aware.
    // Idempotent + cheap (only sessions with no row are touched); logged, never fatal.
    match ctx.pg().repair_orphaned_sessions().await {
        Ok(n) if n > 0 => tracing::info!(repaired = n, "run_backfill: re-attached orphaned sessions"),
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "run_backfill: repair_orphaned_sessions failed"),
    }
    Ok(enqueued)
}

/// Scan all adapters and enqueue one `BackfillTranscriptFile` task per
/// transcript. Returns `(files_seen, enqueued)`. Each per-file task does the
/// smart skip (cursor for prose + session-has-events for synthesis), so the
/// dispatcher stays trivial and correct across upgrades. Callable from the
/// dispatcher task or directly from the trigger endpoint (immediate feedback).
pub async fn dispatch(queue: &crate::tasks::queue::TaskQueue) -> (u32, u32) {
    let mut count = 0u32;
    for ad in adapters() {
        for unit in ad.units() {
            // folder_path = capture source, path = unit key (file path or thread id).
            queue
                .enqueue(Task::new(TaskKind::BackfillTranscriptFile, ad.source(), &unit.key))
                .await;
            count += 1;
        }
    }
    tracing::info!(count, "transcript backfill: dispatched per-unit tasks");
    (count, count)
}

/// Handler for `TaskKind::BackfillTranscriptFile`: ingest one transcript.
/// `task.folder_path` = capture source, `task.path` = transcript file path.
pub async fn run_backfill_file(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let Some(adapter) = adapter_for_source(&task.folder_path) else {
        return Err(format!("unknown transcript source '{}'", task.folder_path));
    };
    let outcome = ingest_one(ctx.pg(), adapter.as_ref(), &task.path).await?;
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

        // pre-existing event ⇒ session already captured, so this test isolates
        // the PROSE cursor-skip path (synthesis is a dedup no-op).
        pg.insert_hook_event(&sid, "claude", "Stop", None, None, 1, None, &serde_json::json!({})).await.unwrap();

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
        sqlx_core::query::query("DELETE FROM activity.assistant_events WHERE session_id=$1").bind(&sid).execute(pool).await.ok();
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
             {{\"type\":\"assistant\",\"cwd\":\"{cwd}\",\"timestamp\":\"2026-06-20T10:00:05.000Z\",\"message\":{{\"role\":\"assistant\",\"model\":\"claude-opus-4-8\",\"content\":[{{\"type\":\"tool_use\",\"name\":\"Edit\",\"input\":{{\"file_path\":\"{cwd}/src/x.rs\"}}}}]}}}}\n",
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
        let s: (Option<uuid::Uuid>, bool, bool, Option<String>, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT project_id, backfilled, (started_at < now() - interval '1 day'), provider, model FROM activity.sessions WHERE client_session_id=$1"
        ).bind(&sid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(s.0, Some(pid), "attributed to the project resolved from cwd");
        assert!(s.1, "flagged backfilled");
        assert!(s.2, "started_at set from the transcript timestamp, not now()");
        assert_eq!(s.3.as_deref(), Some("anthropic"), "provider captured at synthesis");
        assert_eq!(s.4.as_deref(), Some("claude-opus-4-8"), "model captured from the transcript");

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
