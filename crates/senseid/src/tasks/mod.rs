//! Hierarchical task queue for scanning, indexing, and watching.
//!
//! Tasks form a dependency tree:
//!   scan_root → process_git_folder → process_folder → process_file → resolve_libs → build_connections → detect_communities
//!
//! FQN call/import edges resolve to their target node AT EMIT (Phase 7.1), so
//! there is no `resolve_edges` pass. Barrier tasks (resolve_libs,
//! build_connections, detect_communities) wait for all dependencies to complete.

pub mod queue;
pub mod executor;
#[cfg(test)]
pub(crate) mod test_support;
pub mod retry;
pub mod handlers;
pub mod progress;
pub mod progress_emitter;
pub mod analyzer_scheduler;
pub mod metrics_scheduler;
pub mod advance_run_scheduler;
pub mod watchdog_scheduler;
pub mod contribute_scheduler;
pub mod log_pruner;
pub mod activity_pruner;
pub mod library_update_scheduler;
pub mod capture_drain;
pub mod reconcile_scheduler;
pub mod index_audit;
pub mod processors;
pub mod resume;
pub mod version_rescan;
pub mod verdict_classifier;
pub mod mcp_discovery;
pub mod mcp_probe;

use serde::{Serialize, Deserialize};
use std::time::Instant;

// ── Task kinds ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    ScanRoot,
    ProcessGitFolder,
    ProcessFolder,
    ProcessFile,
    DeleteFile,
    DeleteFolder,
    ResolveLibs,
    ImportLib,
    BranchSwitch,
    BuildConnections,
    EmbedNodes,
    IndexLibrary,
    IndexLibraryPage,
    DetectCommunities,
    ExtractDeps,
    MeasureVerdicts,
    /// Re-reconcile a project root's identity from its README frontmatter
    /// (watcher-triggered on a root README change). Lightweight: no file walk.
    ReconcileRepoMetadata,
    /// Enrich a project's sessions from the captured hook-event stream —
    /// derive turns/corrections/outcome/ftr/duration/module (analyzer L0, #66).
    AnalyzeProject,
    /// LLM process-quality pass over a project's un-scored transcripts — spec
    /// depth/deviation + refuted-findings + incomplete-analysis into
    /// `sessions.props.process` + evidence (spec 2026-08-20). Local reasoning
    /// chain, batch-capped, watermark-gated; `path` carries the project id.
    /// Enqueued on the analyzer scheduler's daily full-refresh window.
    AnalyzeSessionProcess,
    /// Scan a project's doc nodes for backtick identifier mentions that no
    /// longer resolve to a live code node, materialising `inference.drift_items`
    /// (analyzer-driven counterpart to the manual `/drift/scan` endpoint).
    /// Per-project: `path` carries the project id, exactly like `AnalyzeProject`.
    ScanDocDrift,
    /// Dispatcher: enqueue one `BackfillTranscriptFile` per changed transcript
    /// so ingestion chunks + interleaves with other work (#73).
    BackfillTranscripts,
    /// Ingest one transcript file into activity.transcript_turns (resumable,
    /// per-file cursor). folder_path = capture source, path = file (#73).
    BackfillTranscriptFile,
    /// Reconstruct historical coverage for one project: check out sampled past
    /// commits and run the configured `metrics.coverage_command` in each.
    /// `path` = project id, `folder_path` = week bound ("" = all history).
    ///
    /// Deliberately ONE task per project rather than a dispatcher + per-commit
    /// children, which is the shape `BackfillTranscripts` uses. Coverage runs the
    /// project's REAL TEST SUITE per commit, and the executor runs N workers — so
    /// per-commit children would put N test suites on the machine at once. The
    /// serial loop inside one task is the concurrency control.
    BackfillCoverage,
    /// Global: cluster recurring corrective prompts across all projects into
    /// inference.corrections (analyzer #65 step 5). Enqueued once per scheduler tick.
    AggregateCorrections,
    /// Global: snapshot the per-tool signal cards (unused / warn / opportunity /
    /// win) into sensei.tool_insights so the observatory Insights tab reads a
    /// cached row per tool instead of re-computing on every request (T2 Slice D).
    /// Enqueued once per scheduler tick alongside AggregateCorrections.
    AggregateToolInsights,
    /// Global: classify per-tool-call usage verdicts for sessions with a recent
    /// `PostToolUse` that have no rows in `sensei.tool_call_verdicts` yet —
    /// gap-filling the sessions never opened in Replay so the Health-tab
    /// aggregate reflects the whole window. Enqueued each tick BEFORE
    /// `AggregateToolInsights` so the same tick's aggregate reads the fresh
    /// verdicts. Reuses the idempotent `verdict_classifier::classify_session`.
    ClassifyPendingVerdicts,
    /// Global: governance Tier-2 consolidation — merge the always-on global rules
    /// into one `proposed` consolidated ruleset via the model, skipped when the
    /// Tier-1 input is unchanged (source-hash guard). Enqueued once per scheduler
    /// tick alongside the other global passes; the manual path is
    /// `POST /api/knowledge/rules/consolidate`.
    ConsolidateGovernance,
    /// Global: **eagerly** pre-generate the mentor-voice insight copy for pending
    /// recommendations (via [`crate::analysis::insight_copy::generate_and_cache`])
    /// so the Insights / Today board reads cached copy on the FIRST view — no
    /// fallback→warm text transition, no inference on the wire. Idempotent
    /// (cached recs skipped) and bounded per tick; enqueued each analyzer tick.
    WarmInsightCopy,
    /// Global: §9 learning loop — attribute confirmed `playbook_run`s' outcomes
    /// from `activity.sessions`, aggregate per-(axes×playbook) FTR stats, run the
    /// pure `crate::playbook::learn` policy (bounded reweight + propose), and
    /// apply the plan (UPDATE priorities off `base_priority`; UPSERT
    /// `source='learned', enabled=false` proposals). Idempotent; enqueued once
    /// per scheduler tick alongside the other global passes.
    LearnPlaybooks,
    /// Relay segment-publish (A2): project a session's latest `TodoWrite` into
    /// the relay outline and push it to every enrolled Dōjō. The assistant
    /// `session_id` is carried in `task.path` (and used directly as the relay
    /// `run_id`). Enqueued by `ingest_hook_event` on each `TodoWrite`.
    PublishRelaySegments,
    /// Relay-engine (P3.2): advance one autonomous run by a tick — the run id is
    /// carried in `task.path`. Enqueued each scheduler tick per active run (and
    /// per just-resumed run). P3.2 only heartbeats + logs a housekeeping event;
    /// the agent spawn/drive plugs in at the `// P3.3 SEAM` in the handler.
    AdvanceRun,
    /// Relay run→relay publish bridge (P1): federate one daemon-owned run
    /// (`activity.runs`) to `dojo.relay_sessions` so Jerry can watch the build —
    /// the run id is carried in `task.path`. Enqueued each scheduler tick per
    /// active run (beside `AdvanceRun`). STATUS only (publishes status + heartbeat
    /// + stall + plan segments); never drives the run.
    PublishRun,
    /// Metrics pipeline (watermark engine): the per-(project, group) CHILD —
    /// compute ONE base metric group for ONE project. The group (the registry
    /// `task_name`, e.g. `"session_outcomes"`) rides in `task.path`, the project
    /// id in `task.folder_path`, and the frozen `as_of` in `task.as_of` — one kind
    /// handles every group (the group is payload, not enum), which is why there is
    /// no `TaskKind` per group. Enqueued by the `ComputeProjectMetrics` parent;
    /// dispatched to `handlers::metrics::compute_group`, which schedules + seals
    /// each day via `sensei.metric_watermarks`.
    ComputeGroupMetrics,
    /// Metrics pipeline (watermark engine): the per-project HEALTH barrier — a
    /// SEPARATE kind from [`TaskKind::ComputeGroupMetrics`] because it must run
    /// AFTER the base groups land. The `ComputeProjectMetrics` parent enqueues it
    /// `blocked_by` the project's `ComputeGroupMetrics` child ids (project id in
    /// `task.folder_path`); dispatched to `handlers::metrics::compute_health`.
    ComputeHealth,
    /// Metrics pipeline (watermark engine): the per-project PARENT. The project id
    /// rides in `task.folder_path`. It FREEZES one `as_of` (`super::today`) shared
    /// by every child, enqueues one `ComputeGroupMetrics{as_of}` per active base
    /// group, then enqueues `ComputeHealth` `blocked_by` those child ids. Each
    /// child schedules its own days off the per-(repo, group) watermark cursor in
    /// `sensei.metric_watermarks`, so a re-tick recomputes only today + any gaps.
    /// Enqueued each tick by `metrics_scheduler`; dispatched to
    /// `handlers::metrics::compute_project`.
    ComputeProjectMetrics,
}

impl std::fmt::Display for TaskKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScanRoot => write!(f, "scan_root"),
            Self::ProcessGitFolder => write!(f, "process_git_folder"),
            Self::ProcessFolder => write!(f, "process_folder"),
            Self::ProcessFile => write!(f, "process_file"),
            Self::DeleteFile => write!(f, "delete_file"),
            Self::DeleteFolder => write!(f, "delete_folder"),
            Self::ResolveLibs => write!(f, "resolve_libs"),
            Self::ImportLib => write!(f, "import_lib"),
            Self::BranchSwitch => write!(f, "branch_switch"),
            Self::BuildConnections => write!(f, "build_connections"),
            Self::EmbedNodes => write!(f, "embed_nodes"),
            Self::IndexLibrary => write!(f, "index_library"),
            Self::IndexLibraryPage => write!(f, "index_library_page"),
            Self::DetectCommunities => write!(f, "detect_communities"),
            Self::ExtractDeps => write!(f, "extract_deps"),
            Self::MeasureVerdicts => write!(f, "measure_verdicts"),
            Self::ReconcileRepoMetadata => write!(f, "reconcile_repo_metadata"),
            Self::AnalyzeProject => write!(f, "analyze_project"),
            Self::AnalyzeSessionProcess => write!(f, "analyze_session_process"),
            Self::ScanDocDrift => write!(f, "scan_doc_drift"),
            Self::BackfillTranscripts => write!(f, "backfill_transcripts"),
            Self::BackfillTranscriptFile => write!(f, "backfill_transcript_file"),
            Self::BackfillCoverage => write!(f, "backfill_coverage"),
            Self::AggregateCorrections => write!(f, "aggregate_corrections"),
            Self::AggregateToolInsights => write!(f, "aggregate_tool_insights"),
            Self::ClassifyPendingVerdicts => write!(f, "classify_pending_verdicts"),
            Self::ConsolidateGovernance => write!(f, "consolidate_governance"),
            Self::WarmInsightCopy => write!(f, "warm_insight_copy"),
            Self::LearnPlaybooks => write!(f, "learn_playbooks"),
            Self::PublishRelaySegments => write!(f, "publish_relay_segments"),
            Self::AdvanceRun => write!(f, "advance_run"),
            Self::PublishRun => write!(f, "publish_run"),
            Self::ComputeGroupMetrics => write!(f, "compute_group_metrics"),
            Self::ComputeHealth => write!(f, "compute_health"),
            Self::ComputeProjectMetrics => write!(f, "compute_project_metrics"),
        }
    }
}

impl TaskKind {
    /// Wall-clock safety cap for a single task in the worker executor. A task
    /// that exceeds this is abandoned by the watchdog and marked failed, so a
    /// wedged handler (a stalled network/DB call) can't occupy a worker forever
    /// and starve the resolve barrier — one stuck task otherwise freezes the
    /// whole pool. This is a last-resort net, not a tight SLA: the cap is
    /// generous and abandoned work is retried or backfilled.
    pub fn watchdog_timeout(&self) -> std::time::Duration {
        use std::time::Duration;
        match self {
            // Per-file / light tasks finish in well under a second normally.
            TaskKind::ProcessFile
            | TaskKind::ProcessFolder
            | TaskKind::DeleteFile
            | TaskKind::DeleteFolder
            | TaskKind::ExtractDeps
            | TaskKind::BranchSwitch
            | TaskKind::ReconcileRepoMetadata
            // Transcript ingestion is chunked per-file; the dispatcher just
            // lists + enqueues, each per-file task parses one transcript.
            | TaskKind::BackfillTranscripts
            | TaskKind::BackfillTranscriptFile
            | TaskKind::MeasureVerdicts
            // Relay segment-publish: one DB read + a couple of bounded HTTP
            // posts per enrolled dojo — light and network-bounded already.
            | TaskKind::PublishRelaySegments
            // AdvanceRun tick (P3.2): a single run read + a heartbeat + one event
            // append — trivially fast. (The agent drive that P3.3 adds will keep
            // its own budget/timeouts; the tick itself stays light.)
            | TaskKind::AdvanceRun
            // PublishRun (P1): one run read + one events read + a bounded HTTP
            // post per owning dojo — the same light, network-bounded shape as
            // PublishRelaySegments.
            | TaskKind::PublishRun
            // Tool-insights snapshot: a couple of small aggregations + one
            // multi-row insert — well under a minute in practice, but keep
            // the same 3-minute budget as the other analyzer touch-ups so a
            // pathological corpus doesn't wedge the queue.
            | TaskKind::AggregateToolInsights
            // Learning-loop pass: one UPDATE join + one GROUP BY aggregate +
            // a bounded number of UPDATE/UPSERT statements off the (small)
            // playbook_rules table — same order of cost as the tool-insights
            // snapshot.
            | TaskKind::LearnPlaybooks
            // Metrics per-project parent: freezes one as_of + a bounded burst of
            // child/health enqueues (no compute of its own) — light and DB-bound,
            // so it shares the short cap.
            | TaskKind::ComputeProjectMetrics => Duration::from_secs(180),
            // Whole-repo, barrier, embedding and network-bound doc-indexing
            // tasks can legitimately run for minutes on a large repository.
            TaskKind::ScanRoot
            | TaskKind::ProcessGitFolder
            | TaskKind::ResolveLibs
            | TaskKind::ImportLib
            | TaskKind::BuildConnections
            | TaskKind::EmbedNodes
            | TaskKind::IndexLibrary
            | TaskKind::IndexLibraryPage
            | TaskKind::AnalyzeProject
            // Doc-drift scan walks up to 500 doc nodes, reading each file off
            // disk — on a large repo that's the same order as AnalyzeProject.
            | TaskKind::ScanDocDrift
            // Verdict gap-fill loops every unclassified in-window session, each
            // a couple of DB round-trips + string classification — a batch that
            // scales with the backlog, so it shares the generous batch budget.
            | TaskKind::ClassifyPendingVerdicts
            // Tier-2 consolidation is one model (reasoning-chain) call; a cold
            // embedded model can take minutes, so it shares the batch budget.
            | TaskKind::ConsolidateGovernance
            // Eager insight-copy warming is up to WARM_CAP sequential model calls
            // (a cold embedded model is slow first); the breaker caps a down model.
            | TaskKind::WarmInsightCopy
            // Process-quality pass is up to batch_per_tick (default 25) sequential
            // reasoning-chain calls over per-session transcripts — a batch that
            // scales with the backlog, so it shares the generous batch budget.
            | TaskKind::AnalyzeSessionProcess
            // Metrics group compute (base groups, all planned days) + the
            // per-project health barrier each scan a project's window (sessions /
            // churn / duplication …) and write sensei.project_metrics — a
            // whole-project batch that can run for minutes on a large corpus, so
            // they share the generous batch budget.
            | TaskKind::ComputeGroupMetrics
            | TaskKind::ComputeHealth
            | TaskKind::AggregateCorrections => Duration::from_secs(600),
            // Community detection is the terminal barrier and, on a huge edge-heavy
            // folder post-FQN (observed: 141k nodes / 287k edges / 11k communities),
            // runs label-propagation + an atomic replace of every community and node
            // community_id — legitimately many minutes. 600s watchdog-killed those
            // into a retry-timeout loop that stranded the folder at `indexing`, so
            // the terminal barrier gets a wider budget.
            TaskKind::DetectCommunities => Duration::from_secs(1800),
            // Coverage backfill is the longest task in the system by a wide margin
            // and is not comparable to the others: it runs the project's REAL TEST
            // SUITE once per sampled commit, serially. A repo with two years of
            // history is ~100 suite runs in one task, so no budget derived from the
            // other kinds fits. The `weeks` bound on the request — not the watchdog
            // — is the intended control; the watchdog is only here to stop a hung
            // command wedging a worker forever.
            TaskKind::BackfillCoverage => Duration::from_secs(7200),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Blocked,    // has unmet dependencies
    Running,
    Completed,
    Failed,
}

// ── Task ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Task {
    pub id: u64,
    pub kind: TaskKind,
    pub folder_path: String,             // git folder abs path — used for grouping and DB lookups
    pub path: String,                    // file/folder/root path (what this task operates on)
    pub parent_task_id: Option<u64>,     // for hierarchy tracking
    pub module_id: Option<String>,       // for process_file: which module this file belongs to
    pub branch: Option<String>,          // git branch name (for branch-aware indexing)
    pub url: Option<String>,             // for import_lib: library docs URL
    /// Target `computed_on` day for a metrics compute (`ComputeGroupMetrics`).
    /// `None` = the incremental "today" run (rolling-window behavior preserved).
    /// `Some(D)` = compute the single historical day `D` (the backfill/gap-fill
    /// path) — see `handlers::metrics`. Carried through `retry()` so an interrupted
    /// backfill resumes on the same day.
    pub as_of: Option<chrono::NaiveDate>,
    pub status: TaskStatus,
    pub depends_on: Vec<u64>,            // won't run until these complete
    pub error: Option<String>,
    pub retry_number: u32,               // 0 = first attempt; bumped per bounded retry (D6c)
    pub _created_at: Instant,
    pub started_at: Option<Instant>,
    pub completed_at: Option<Instant>,
}

impl Task {
    pub fn new(kind: TaskKind, folder_path: &str, path: &str) -> Self {
        Self {
            id: 0, // assigned by queue
            kind,
            folder_path: folder_path.to_string(),
            path: path.to_string(),
            parent_task_id: None,
            module_id: None,
            branch: None,
            url: None,
            as_of: None,
            status: TaskStatus::Pending,
            depends_on: Vec::new(),
            error: None,
            retry_number: 0,
            _created_at: Instant::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// The next retry attempt for a failed task (D6c): same identity
    /// (kind/paths/module/branch/url/parent), `retry_number` incremented, and
    /// all runtime state reset — the queue assigns a fresh `id` on re-enqueue,
    /// and a retry carries no inherited deps (a re-driven leaf runs on its own).
    pub fn retry(&self) -> Self {
        Self {
            id: 0,
            kind: self.kind.clone(),
            folder_path: self.folder_path.clone(),
            path: self.path.clone(),
            parent_task_id: self.parent_task_id,
            module_id: self.module_id.clone(),
            branch: self.branch.clone(),
            url: self.url.clone(),
            as_of: self.as_of,
            status: TaskStatus::Pending,
            depends_on: Vec::new(),
            error: None,
            retry_number: self.retry_number + 1,
            _created_at: Instant::now(),
            started_at: None,
            completed_at: None,
        }
    }

    pub fn with_parent(mut self, parent_id: u64) -> Self {
        self.parent_task_id = Some(parent_id);
        self
    }

    pub fn with_module(mut self, module_id: &str) -> Self {
        self.module_id = Some(module_id.to_string());
        self
    }

    pub fn with_branch(mut self, branch: &str) -> Self {
        self.branch = Some(branch.to_string());
        self
    }

    #[allow(dead_code)]
    pub fn with_url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }

    /// Set the target `computed_on` day for a metrics compute (`ComputeGroupMetrics`).
    /// `None` (the default) is the incremental "today" run; `Some(D)` targets the
    /// single historical day `D`. The `ComputeProjectMetrics` parent stamps the
    /// frozen `as_of` here on every `ComputeGroupMetrics` child.
    #[allow(dead_code)]
    pub fn with_as_of(mut self, as_of: chrono::NaiveDate) -> Self {
        self.as_of = Some(as_of);
        self
    }

    /// Derive folder name from folder_path (basename).
    pub fn folder_name(&self) -> &str {
        std::path::Path::new(&self.folder_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
    }

    pub fn blocked_by(mut self, deps: Vec<u64>) -> Self {
        if !deps.is_empty() {
            self.status = TaskStatus::Blocked;
            self.depends_on = deps;
        }
        self
    }

    #[allow(dead_code)]
    pub fn is_runnable(&self) -> bool {
        self.status == TaskStatus::Pending
    }

    #[allow(dead_code)]
    pub fn is_barrier(&self) -> bool {
        matches!(self.kind, TaskKind::ResolveLibs | TaskKind::BuildConnections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_creation() {
        let t = Task::new(TaskKind::ProcessFile, "/code/myrepo", "/code/myrepo/src/file.ts");
        assert_eq!(t.kind, TaskKind::ProcessFile);
        assert_eq!(t.folder_path, "/code/myrepo");
        assert_eq!(t.path, "/code/myrepo/src/file.ts");
        assert_eq!(t.folder_name(), "myrepo");
        assert_eq!(t.status, TaskStatus::Pending);
        assert!(t.is_runnable());
        assert!(!t.is_barrier());
    }

    #[test]
    fn blocked_task() {
        let t = Task::new(TaskKind::BuildConnections, "/code/myrepo", "/code/myrepo")
            .blocked_by(vec![1, 2, 3]);
        assert_eq!(t.status, TaskStatus::Blocked);
        assert!(!t.is_runnable());
        assert!(t.is_barrier());
        assert_eq!(t.depends_on, vec![1, 2, 3]);
    }

    #[test]
    fn task_retry_bumps_number_and_resets_runtime() {
        let mut base = Task::new(TaskKind::ProcessFile, "/code/repo", "/code/repo/src/a.rs")
            .with_parent(7)
            .with_module("mod:repo:src")
            .with_branch("main")
            .with_url("https://example.test/pkg");
        base.id = 42;
        base.retry_number = 1;
        base.error = Some("boom".into());
        base.status = TaskStatus::Failed;
        base.depends_on = vec![1, 2];
        base.as_of = chrono::NaiveDate::from_ymd_opt(2025, 6, 1);

        let next = base.retry();
        // Identity is preserved — every field that names WHAT to run.
        assert_eq!(next.kind, base.kind);
        assert_eq!(next.folder_path, base.folder_path);
        assert_eq!(next.path, base.path);
        assert_eq!(next.parent_task_id, Some(7));
        assert_eq!(next.module_id, Some("mod:repo:src".to_string()));
        assert_eq!(next.branch, Some("main".to_string()), "retry preserves branch identity");
        assert_eq!(next.url, Some("https://example.test/pkg".to_string()), "retry preserves url identity");
        assert_eq!(next.as_of, chrono::NaiveDate::from_ymd_opt(2025, 6, 1),
            "retry preserves the target computed_on day so an interrupted backfill resumes");
        // The attempt count advances by exactly one.
        assert_eq!(next.retry_number, 2, "retry() bumps retry_number");
        // Runtime state is reset — a fresh, re-enqueueable attempt.
        assert_eq!(next.id, 0, "queue assigns a new id");
        assert_eq!(next.status, TaskStatus::Pending);
        assert!(next.depends_on.is_empty(), "a retry carries no inherited deps");
        assert!(next.error.is_none());
    }

    #[test]
    fn new_task_starts_at_retry_zero() {
        assert_eq!(Task::new(TaskKind::ProcessFile, "r", "p").retry_number, 0);
    }

    #[test]
    fn task_with_parent_and_module() {
        let t = Task::new(TaskKind::ProcessFile, "/code/repo", "/code/repo/src/main.ts")
            .with_parent(42)
            .with_module("mod:repo:src");
        assert_eq!(t.parent_task_id, Some(42));
        assert_eq!(t.module_id, Some("mod:repo:src".to_string()));
    }

    #[test]
    fn task_kind_display() {
        assert_eq!(TaskKind::ScanRoot.to_string(), "scan_root");
        assert_eq!(TaskKind::ProcessFile.to_string(), "process_file");
        assert_eq!(TaskKind::ResolveLibs.to_string(), "resolve_libs");
        assert_eq!(TaskKind::IndexLibrary.to_string(), "index_library");
        assert_eq!(TaskKind::IndexLibraryPage.to_string(), "index_library_page");
        assert_eq!(TaskKind::DetectCommunities.to_string(), "detect_communities");
        assert_eq!(TaskKind::ExtractDeps.to_string(), "extract_deps");
        assert_eq!(TaskKind::MeasureVerdicts.to_string(), "measure_verdicts");
        assert_eq!(TaskKind::ScanDocDrift.to_string(), "scan_doc_drift");
        assert_eq!(TaskKind::ClassifyPendingVerdicts.to_string(), "classify_pending_verdicts");
        assert_eq!(TaskKind::AdvanceRun.to_string(), "advance_run");
        assert_eq!(TaskKind::PublishRelaySegments.to_string(), "publish_relay_segments");
        assert_eq!(TaskKind::ComputeGroupMetrics.to_string(), "compute_group_metrics");
        assert_eq!(TaskKind::ComputeHealth.to_string(), "compute_health");
        assert_eq!(TaskKind::ComputeProjectMetrics.to_string(), "compute_project_metrics");
    }

    #[test]
    fn watchdog_timeout_is_bounded_and_tiered() {
        // Light per-file tasks get the short cap; heavy/whole-repo/network tasks
        // get the long cap. Every variant returns a finite, positive bound.
        let short = std::time::Duration::from_secs(180);
        let long = std::time::Duration::from_secs(600);
        assert_eq!(TaskKind::ProcessFile.watchdog_timeout(), short);
        assert_eq!(TaskKind::DeleteFile.watchdog_timeout(), short);
        assert_eq!(TaskKind::ResolveLibs.watchdog_timeout(), long);
        assert_eq!(TaskKind::EmbedNodes.watchdog_timeout(), long);
        assert_eq!(TaskKind::ScanRoot.watchdog_timeout(), long);
        assert_eq!(TaskKind::ScanDocDrift.watchdog_timeout(), long);
        assert_eq!(TaskKind::ClassifyPendingVerdicts.watchdog_timeout(), long);
        // Metrics group compute + health barrier are whole-project batches → long bucket.
        assert_eq!(TaskKind::ComputeGroupMetrics.watchdog_timeout(), long);
        assert_eq!(TaskKind::ComputeHealth.watchdog_timeout(), long);
        // The per-project parent only freezes as_of + enqueues → short bucket.
        assert_eq!(TaskKind::ComputeProjectMetrics.watchdog_timeout(), short);
        // DetectCommunities (terminal barrier) gets a WIDER budget than `long` —
        // community detection on a huge edge-heavy folder legitimately runs many
        // minutes and must not be watchdog-killed into a retry-timeout loop.
        assert!(TaskKind::DetectCommunities.watchdog_timeout() > long,
            "DetectCommunities gets a wider-than-long watchdog for huge graphs");
        for k in [
            TaskKind::ScanRoot, TaskKind::ProcessGitFolder, TaskKind::ProcessFolder,
            TaskKind::ProcessFile, TaskKind::DeleteFile, TaskKind::DeleteFolder,
            TaskKind::ResolveLibs, TaskKind::ImportLib,
            TaskKind::BranchSwitch, TaskKind::BuildConnections,
            TaskKind::EmbedNodes, TaskKind::IndexLibrary, TaskKind::IndexLibraryPage,
            TaskKind::DetectCommunities, TaskKind::ExtractDeps, TaskKind::MeasureVerdicts,
            TaskKind::ReconcileRepoMetadata, TaskKind::AnalyzeProject,
            TaskKind::AnalyzeSessionProcess, TaskKind::ScanDocDrift,
            TaskKind::BackfillTranscripts,
            TaskKind::BackfillTranscriptFile, TaskKind::BackfillCoverage,
            TaskKind::AggregateCorrections,
            TaskKind::AggregateToolInsights, TaskKind::ClassifyPendingVerdicts,
            TaskKind::ConsolidateGovernance, TaskKind::WarmInsightCopy,
            TaskKind::LearnPlaybooks,
            TaskKind::PublishRelaySegments, TaskKind::AdvanceRun,
            TaskKind::PublishRun,
            TaskKind::ComputeGroupMetrics, TaskKind::ComputeHealth,
            TaskKind::ComputeProjectMetrics,
        ] {
            assert!(k.watchdog_timeout().as_secs() > 0, "{k} must have a positive cap");
        }
    }
}
