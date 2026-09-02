//! Hierarchical task queue for scanning, indexing, and watching.
//!
//! Tasks form a dependency tree:
//!   scan_root → process_git_folder → process_folder → process_file → resolve_libs → build_connections → detect_communities
//!
//! FQN call edges, and LOCAL import edges, resolve to their target node AT EMIT
//! (Phase 7.1) — so there is no `resolve_edges` pass, and adding one would be the
//! wrong shape. Emit-time resolution is order-independent because a miss creates a
//! stub on the target's own fqn which the target's later definition enriches in
//! place, keeping the id.
//!
//! EXTERNAL import edges stay unresolved by design and keep `target_name`: the
//! package name IS the useful answer to "what does this file depend on", and it is
//! the only place that string survives (a resolved edge has a NULL `target_name`).
//! This sentence previously claimed ALL import edges resolved at emit, which was
//! false for every one of them — `process.rs` passed `target_id = None`
//! unconditionally, so 0 of 162,690 resolved.
//!
//! Barrier tasks (resolve_libs, build_connections, detect_communities) wait for all
//! dependencies to complete.

pub mod activity_pruner;
pub mod advance_run_scheduler;
pub mod analyzer_scheduler;
pub mod capture_drain;
pub mod contribute_scheduler;
pub mod dojo_sync;
pub mod executor;
pub mod forge_token_check;
pub mod handlers;
pub mod index_audit;
pub mod library_update_scheduler;
pub mod log_pruner;
pub mod mcp_discovery;
pub mod mcp_probe;
pub mod metrics_scheduler;
pub mod processors;
pub mod progress;
pub mod progress_emitter;
pub mod queue;
pub mod reconcile_scheduler;
pub mod resume;
pub mod retry;
pub mod schedule;
#[cfg(test)]
pub(crate) mod test_support;
pub mod ticker;
pub mod verdict_classifier;
pub mod version_rescan;
pub mod watchdog_scheduler;

use serde::{Deserialize, Serialize};
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
    /// Dispatcher: enqueue one `IngestCapture` per changed transcript
    /// so ingestion chunks + interleaves with other work (#73).
    IngestCaptures,
    /// Ingest one transcript file into activity.transcript_turns (resumable,
    /// per-file cursor). folder_path = capture source, path = file (#73).
    IngestCapture,
    /// Reconstruct historical coverage for one project: check out sampled past
    /// commits and run the configured `metrics.coverage_command` in each.
    /// `path` = project id, `folder_path` = week bound ("" = all history).
    ///
    /// Deliberately ONE task per project rather than a dispatcher + per-commit
    /// children, which is the shape `IngestCaptures` uses. Coverage runs the
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
    /// recommendations (via [`crate::analysis::narration_cache::generate_and_cache`])
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
    /// The wire/log name, from [`TaskKind::info`] — one definition, so a rename
    /// cannot drift between the log, the database enum, and the code.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.info().name)
    }
}

// ── Kind metadata ───────────────────────────────────────────────────────────

/// Which pipeline a task kind belongs to.
///
/// The pipelines already existed as convention — nothing in the code named them,
/// so "which stage is this" lived only in reviewers' heads and in the ordering of
/// a `match`. Naming them makes grouping queryable and gives a new kind an
/// obvious home.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pipeline {
    /// Walk a checkout into the code graph.
    Index,
    /// Resolve and index third-party library docs.
    Library,
    /// Ingest and enrich assistant activity.
    Activity,
    /// Compute metric values from the above.
    Metrics,
    /// Distil signals, learn, and publish.
    Inference,
}

/// Where in its pipeline a kind sits.
///
/// `Coordinate` is the load-bearing one: a coordinator decides WHAT needs doing
/// and enqueues workers, so it finishes in milliseconds having done none of the
/// work. Anything reporting on a coordinator has to look at its children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Decides what needs doing and enqueues workers. Does no work itself.
    Coordinate,
    /// Finds the units to process.
    Discover,
    /// Reads one unit into the store.
    Ingest,
    /// Computes from what was ingested.
    Derive,
    /// Combines derived values across units.
    Aggregate,
    /// Sends results outward.
    Publish,
}

/// Everything the system needs to know about a kind, in ONE place.
///
/// Before this, adding a kind meant editing five: the enum, `Display`, the
/// watchdog match, `queue::kind_priority`, and `retry::is_retryable` — plus a
/// test list that enforced nothing. Four of those were `match` arms that a new
/// variant silently fell through to a default on. Now [`TaskKind::info`] is the
/// single exhaustive match, so the compiler refuses to build until a new kind
/// declares its budget, priority and retry policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindInfo {
    /// Wire/log name. Must match `sensei.task_execution_kind` — a test asserts it.
    pub name: &'static str,
    pub pipeline: Pipeline,
    pub stage: Stage,
    /// Watchdog budget. A wedged handler would otherwise hold its worker forever,
    /// keep its task `running`, and block the folder's barrier — freezing the pool.
    pub budget_secs: u64,
    /// Preempts bulk work in the queue. Reserved for the metrics chain, so a
    /// compute wave is not starved behind a boot re-index.
    pub high_priority: bool,
    /// Whether a failure is worth re-driving. False for kinds that fail for
    /// permanent reasons (a missing URL, a bad id, a deleted root) — retrying
    /// those just burns cycles.
    pub retryable: bool,
}

impl TaskKind {
    /// Every kind. Kept beside [`Self::info`] and checked against both the
    /// descriptor and the database enum by `all_kinds_match_the_database_enum`,
    /// so a kind cannot be half-added.
    pub const ALL: &'static [TaskKind] = &[
        Self::ScanRoot,
        Self::ProcessGitFolder,
        Self::ProcessFolder,
        Self::ProcessFile,
        Self::DeleteFile,
        Self::DeleteFolder,
        Self::BranchSwitch,
        Self::ExtractDeps,
        Self::EmbedNodes,
        Self::DetectCommunities,
        Self::ResolveLibs,
        Self::ImportLib,
        Self::IndexLibrary,
        Self::IndexLibraryPage,
        Self::IngestCaptures,
        Self::IngestCapture,
        Self::AnalyzeProject,
        Self::AnalyzeSessionProcess,
        Self::ReconcileRepoMetadata,
        Self::ComputeProjectMetrics,
        Self::ComputeGroupMetrics,
        Self::ComputeHealth,
        Self::BackfillCoverage,
        Self::MeasureVerdicts,
        Self::ClassifyPendingVerdicts,
        Self::AggregateCorrections,
        Self::AggregateToolInsights,
        Self::ConsolidateGovernance,
        Self::WarmInsightCopy,
        Self::LearnPlaybooks,
        Self::ScanDocDrift,
        Self::PublishRelaySegments,
        Self::AdvanceRun,
        Self::PublishRun,
    ];

    /// The single source of per-kind truth.
    pub const fn info(&self) -> KindInfo {
        match self {
            Self::ScanRoot => KindInfo {
                name: "scan_root",
                pipeline: Pipeline::Index,
                stage: Stage::Discover,
                budget_secs: 600,
                high_priority: false,
                retryable: false,
            },
            Self::ProcessGitFolder => KindInfo {
                name: "process_git_folder",
                pipeline: Pipeline::Index,
                stage: Stage::Discover,
                budget_secs: 600,
                high_priority: false,
                retryable: true,
            },
            Self::ProcessFolder => KindInfo {
                name: "process_folder",
                pipeline: Pipeline::Index,
                stage: Stage::Discover,
                budget_secs: 180,
                high_priority: false,
                retryable: true,
            },
            Self::ProcessFile => KindInfo {
                name: "process_file",
                pipeline: Pipeline::Index,
                stage: Stage::Ingest,
                budget_secs: 180,
                high_priority: false,
                retryable: true,
            },
            Self::DeleteFile => KindInfo {
                name: "delete_file",
                pipeline: Pipeline::Index,
                stage: Stage::Ingest,
                budget_secs: 180,
                high_priority: false,
                retryable: false,
            },
            Self::DeleteFolder => KindInfo {
                name: "delete_folder",
                pipeline: Pipeline::Index,
                stage: Stage::Ingest,
                budget_secs: 180,
                high_priority: false,
                retryable: false,
            },
            Self::BranchSwitch => KindInfo {
                name: "branch_switch",
                pipeline: Pipeline::Index,
                stage: Stage::Ingest,
                budget_secs: 180,
                high_priority: false,
                retryable: false,
            },
            Self::ExtractDeps => KindInfo {
                name: "extract_deps",
                pipeline: Pipeline::Index,
                stage: Stage::Derive,
                budget_secs: 180,
                high_priority: false,
                retryable: false,
            },
            Self::EmbedNodes => KindInfo {
                name: "embed_nodes",
                pipeline: Pipeline::Index,
                stage: Stage::Derive,
                budget_secs: 600,
                high_priority: false,
                retryable: false,
            },
            Self::DetectCommunities => KindInfo {
                name: "detect_communities",
                pipeline: Pipeline::Index,
                stage: Stage::Aggregate,
                budget_secs: 1800,
                high_priority: false,
                retryable: true,
            },
            Self::ResolveLibs => KindInfo {
                name: "resolve_libs",
                pipeline: Pipeline::Library,
                stage: Stage::Discover,
                budget_secs: 600,
                high_priority: false,
                retryable: false,
            },
            Self::ImportLib => KindInfo {
                name: "import_lib",
                pipeline: Pipeline::Library,
                stage: Stage::Ingest,
                budget_secs: 600,
                high_priority: false,
                retryable: false,
            },
            Self::IndexLibrary => KindInfo {
                name: "index_library",
                pipeline: Pipeline::Library,
                stage: Stage::Ingest,
                budget_secs: 600,
                high_priority: false,
                retryable: false,
            },
            Self::IndexLibraryPage => KindInfo {
                name: "index_library_page",
                pipeline: Pipeline::Library,
                stage: Stage::Ingest,
                budget_secs: 600,
                high_priority: false,
                retryable: false,
            },
            Self::IngestCaptures => KindInfo {
                name: "ingest_captures",
                pipeline: Pipeline::Activity,
                stage: Stage::Coordinate,
                budget_secs: 180,
                high_priority: false,
                retryable: false,
            },
            Self::IngestCapture => KindInfo {
                name: "ingest_capture",
                pipeline: Pipeline::Activity,
                stage: Stage::Ingest,
                budget_secs: 180,
                high_priority: false,
                retryable: false,
            },
            Self::AnalyzeProject => KindInfo {
                name: "analyze_project",
                pipeline: Pipeline::Activity,
                stage: Stage::Derive,
                budget_secs: 600,
                high_priority: true,
                retryable: false,
            },
            Self::AnalyzeSessionProcess => KindInfo {
                name: "analyze_session_process",
                pipeline: Pipeline::Activity,
                stage: Stage::Derive,
                budget_secs: 600,
                high_priority: false,
                retryable: false,
            },
            Self::ReconcileRepoMetadata => KindInfo {
                name: "reconcile_repo_metadata",
                pipeline: Pipeline::Activity,
                stage: Stage::Derive,
                budget_secs: 180,
                high_priority: false,
                retryable: false,
            },
            Self::ComputeProjectMetrics => KindInfo {
                name: "compute_project_metrics",
                pipeline: Pipeline::Metrics,
                stage: Stage::Coordinate,
                budget_secs: 180,
                high_priority: true,
                retryable: false,
            },
            Self::ComputeGroupMetrics => KindInfo {
                name: "compute_group_metrics",
                pipeline: Pipeline::Metrics,
                stage: Stage::Derive,
                budget_secs: 600,
                high_priority: true,
                retryable: true,
            },
            Self::ComputeHealth => KindInfo {
                name: "compute_health",
                pipeline: Pipeline::Metrics,
                stage: Stage::Aggregate,
                budget_secs: 600,
                high_priority: true,
                retryable: true,
            },
            Self::BackfillCoverage => KindInfo {
                name: "backfill_coverage",
                pipeline: Pipeline::Metrics,
                stage: Stage::Ingest,
                budget_secs: 7200,
                high_priority: false,
                retryable: false,
            },
            Self::MeasureVerdicts => KindInfo {
                name: "measure_verdicts",
                pipeline: Pipeline::Inference,
                stage: Stage::Derive,
                budget_secs: 180,
                high_priority: false,
                retryable: false,
            },
            Self::ClassifyPendingVerdicts => KindInfo {
                name: "classify_pending_verdicts",
                pipeline: Pipeline::Inference,
                stage: Stage::Derive,
                budget_secs: 600,
                high_priority: false,
                retryable: false,
            },
            Self::AggregateCorrections => KindInfo {
                name: "aggregate_corrections",
                pipeline: Pipeline::Inference,
                stage: Stage::Aggregate,
                budget_secs: 600,
                high_priority: false,
                retryable: false,
            },
            Self::AggregateToolInsights => KindInfo {
                name: "aggregate_tool_insights",
                pipeline: Pipeline::Inference,
                stage: Stage::Aggregate,
                budget_secs: 180,
                high_priority: false,
                retryable: false,
            },
            Self::ConsolidateGovernance => KindInfo {
                name: "consolidate_governance",
                pipeline: Pipeline::Inference,
                stage: Stage::Aggregate,
                budget_secs: 600,
                high_priority: false,
                retryable: false,
            },
            Self::WarmInsightCopy => KindInfo {
                name: "warm_narration_cache",
                pipeline: Pipeline::Inference,
                stage: Stage::Derive,
                budget_secs: 600,
                high_priority: false,
                retryable: false,
            },
            Self::LearnPlaybooks => KindInfo {
                name: "learn_playbooks",
                pipeline: Pipeline::Inference,
                stage: Stage::Derive,
                budget_secs: 180,
                high_priority: false,
                retryable: false,
            },
            Self::ScanDocDrift => KindInfo {
                name: "scan_doc_drift",
                pipeline: Pipeline::Inference,
                stage: Stage::Derive,
                budget_secs: 600,
                high_priority: false,
                retryable: false,
            },
            Self::PublishRelaySegments => KindInfo {
                name: "publish_relay_segments",
                pipeline: Pipeline::Inference,
                stage: Stage::Publish,
                budget_secs: 180,
                high_priority: false,
                retryable: false,
            },
            Self::AdvanceRun => KindInfo {
                name: "advance_run",
                pipeline: Pipeline::Inference,
                stage: Stage::Publish,
                budget_secs: 180,
                high_priority: false,
                retryable: false,
            },
            Self::PublishRun => KindInfo {
                name: "publish_run",
                pipeline: Pipeline::Inference,
                stage: Stage::Publish,
                budget_secs: 180,
                high_priority: false,
                retryable: false,
            },
        }
    }

    pub const fn pipeline(&self) -> Pipeline {
        self.info().pipeline
    }
    pub const fn stage(&self) -> Stage {
        self.info().stage
    }
    pub const fn is_retryable(&self) -> bool {
        self.info().retryable
    }
    pub const fn is_high_priority(&self) -> bool {
        self.info().high_priority
    }

    /// Watchdog cap for this kind.
    pub const fn watchdog_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.info().budget_secs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Blocked, // has unmet dependencies
    Running,
    Completed,
    Failed,
}

// ── Task ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Task {
    pub id: u64,
    pub kind: TaskKind,
    pub folder_path: String, // git folder abs path — used for grouping and DB lookups
    pub path: String,        // file/folder/root path (what this task operates on)
    pub parent_task_id: Option<u64>, // for hierarchy tracking
    pub module_id: Option<String>, // for process_file: which module this file belongs to
    pub branch: Option<String>, // git branch name (for branch-aware indexing)
    pub url: Option<String>, // for import_lib: library docs URL
    /// Target `computed_on` day for a metrics compute (`ComputeGroupMetrics`).
    /// `None` = the incremental "today" run (rolling-window behavior preserved).
    /// `Some(D)` = compute the single historical day `D` (the backfill/gap-fill
    /// path) — see `handlers::metrics`. Carried through `retry()` so an interrupted
    /// backfill resumes on the same day.
    pub as_of: Option<chrono::NaiveDate>,
    pub status: TaskStatus,
    pub depends_on: Vec<u64>, // won't run until these complete
    pub error: Option<String>,
    pub retry_number: u32, // 0 = first attempt; bumped per bounded retry (D6c)
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

    // ── Typed payload ───────────────────────────────────────────────────
    //
    // `folder_path` and `path` are two untyped strings that mean something
    // different for almost every kind. The code says so itself — there are
    // comments reading "lib name stored in path field", "library UUID stored in
    // folder_path", "page title stored in path" — and `BackfillCoverage` went as
    // far as putting a WEEK COUNT in `folder_path`.
    //
    // The consequence is that nothing checks an enqueue. Pass a project id where
    // a folder path belongs and it fails much later, inside a handler, as a parse
    // error or (worse) a lookup miss that reads as "no data". Every handler also
    // re-parsed its own inputs with its own error text: four different phrasings
    // of "invalid project id" existed.
    //
    // These constructors and accessors are the contract. The storage is still the
    // two columns — a jsonb payload is a later, separate change — but both ends
    // now name what they carry, in one place.

    /// A task about one PROJECT. The id rides in `path`.
    pub fn for_project(kind: TaskKind, project_id: &uuid::Uuid) -> Self {
        Self::new(kind, "", &project_id.to_string())
    }

    /// A task about one FOLDER on disk — the git checkout root it operates in.
    pub fn for_folder(kind: TaskKind, abs_path: &str) -> Self {
        Self::new(kind, abs_path, abs_path)
    }

    /// A task about one FILE within a folder.
    pub fn for_file(kind: TaskKind, folder_abs_path: &str, file_abs_path: &str) -> Self {
        Self::new(kind, folder_abs_path, file_abs_path)
    }

    /// A task about one captured unit — a transcript file or thread — from a
    /// named capture source (`claude_code`, `zed`, `opencode`).
    pub fn for_capture(kind: TaskKind, source: &str, unit: &str) -> Self {
        Self::new(kind, source, unit)
    }

    /// A coverage backfill for one project, optionally bounded to the most recent
    /// `weeks` sampled anchors.
    ///
    /// The bound used to be stringified into `folder_path` at the call site, which
    /// is exactly the kind of thing this constructor exists to stop being ad hoc.
    pub fn for_coverage_backfill(project_id: &uuid::Uuid, weeks: Option<u32>) -> Self {
        Self::new(
            TaskKind::BackfillCoverage,
            &weeks.map(|w| w.to_string()).unwrap_or_default(),
            &project_id.to_string(),
        )
    }

    /// The project id this task is about.
    ///
    /// One parse, one error phrasing. Handlers previously each wrote their own —
    /// "ScanDocDrift: invalid project id", "AnalyzeSessionProcess: invalid project
    /// id", "coverage: bad project id" — so the same failure read three ways
    /// depending on which handler hit it.
    pub fn project_id(&self) -> Result<uuid::Uuid, String> {
        uuid::Uuid::parse_str(&self.path).map_err(|e| {
            format!("{}: expected a project id in `path`, got {:?} ({e})", self.kind, self.path)
        })
    }

    /// The folder abs path this task operates in.
    pub fn folder_abs_path(&self) -> &str {
        &self.folder_path
    }

    /// The capture source this task's unit came from.
    pub fn capture_source(&self) -> &str {
        &self.folder_path
    }

    /// `as_of` as a capture change-stamp (epoch nanoseconds), for kinds that
    /// ingest units carrying an mtime-style stamp.
    ///
    /// `as_of` already meant "the day this task is for" on the metrics side. Using
    /// the SAME field for captures is the point of the phase: a backfill is a
    /// date parameter on the normal work, not a separate kind — so both pipelines
    /// express "from this day" the same way rather than inventing a second idiom.
    ///
    /// `None` = no bound = ingest everything (the cursor still skips unchanged
    /// units, so this stays correct, just less selective).
    pub fn as_of_stamp_ns(&self) -> Option<i64> {
        self.as_of.map(|d| {
            d.and_hms_opt(0, 0, 0)
                .map(|dt| dt.and_utc().timestamp_nanos_opt().unwrap_or(0))
                .unwrap_or(0)
        })
    }

    /// The week bound on a coverage backfill: `None` = all history.
    ///
    /// An unparseable bound is an ERROR, not a fallback to `None`. Defaulting
    /// would silently run the project's real test suite across its entire history
    /// because a string was malformed — far more work than was asked for.
    pub fn coverage_weeks(&self) -> Result<Option<u32>, String> {
        match self.folder_path.as_str() {
            "" => Ok(None),
            w => w.parse::<u32>().map(Some).map_err(|e| {
                format!("{}: expected a week count in `folder_path`, got {w:?} ({e})", self.kind)
            }),
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
        matches!(self.kind, TaskKind::ResolveLibs | TaskKind::DetectCommunities)
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
        let t = Task::new(TaskKind::DetectCommunities, "/code/myrepo", "/code/myrepo")
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
        assert_eq!(
            next.url,
            Some("https://example.test/pkg".to_string()),
            "retry preserves url identity"
        );
        assert_eq!(
            next.as_of,
            chrono::NaiveDate::from_ymd_opt(2025, 6, 1),
            "retry preserves the target computed_on day so an interrupted backfill resumes"
        );
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
        // get the long cap. Spot-checks of the tiering, now read off the descriptor.
        let short = std::time::Duration::from_secs(180);
        let long = std::time::Duration::from_secs(600);
        assert_eq!(TaskKind::ProcessFile.watchdog_timeout(), short);
        assert_eq!(TaskKind::ScanRoot.watchdog_timeout(), long);
        assert_eq!(TaskKind::ComputeGroupMetrics.watchdog_timeout(), long);
        // The per-project parent only freezes as_of + enqueues → short bucket.
        assert_eq!(TaskKind::ComputeProjectMetrics.watchdog_timeout(), short);
        // The terminal barrier gets a WIDER budget: community detection on a huge
        // edge-heavy folder legitimately runs many minutes and must not be
        // watchdog-killed into a retry-timeout loop.
        assert!(TaskKind::DetectCommunities.watchdog_timeout() > long);
        // Coverage runs the project's real test suite per commit — the widest.
        assert!(
            TaskKind::BackfillCoverage.watchdog_timeout()
                > TaskKind::DetectCommunities.watchdog_timeout()
        );

        // Iterating ALL rather than a hand-copied list: the old version repeated
        // every variant here, so a new kind was covered only if someone
        // remembered to add it in two places.
        for k in TaskKind::ALL {
            assert!(k.watchdog_timeout().as_secs() > 0, "{k} must have a positive cap");
        }
    }

    #[test]
    fn every_kind_is_in_all_exactly_once() {
        // `info()` is exhaustive, so the compiler already forces a new variant to
        // declare a descriptor. This catches the other half — that it also lands
        // in ALL, which nothing else enforces.
        let mut names: Vec<&str> = TaskKind::ALL.iter().map(|k| k.info().name).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "a kind appears twice in ALL");
        assert_eq!(total, 34, "ALL is missing a kind — add it beside its info() arm");
    }

    #[test]
    fn kind_names_are_snake_case_and_stable() {
        // The name is the wire value written to activity.task_executions.task_kind,
        // which is a Postgres enum. A name that does not match a value there fails
        // at INSERT, on a fire-and-forget path that only logs — so drift would be
        // invisible until someone read the log. Check the shape here.
        for k in TaskKind::ALL {
            let n = k.info().name;
            assert!(!n.is_empty(), "{k} has an empty name");
            assert!(
                n.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{k} name {n:?} must be snake_case to match the DB enum"
            );
            assert_eq!(k.to_string(), n, "Display must be the descriptor name");
        }
    }

    #[test]
    fn pipelines_and_stages_are_assigned_coherently() {
        // Coordinators do no work themselves — they enqueue and finish in
        // milliseconds — so a coordinator with a long budget is a modelling
        // mistake worth catching.
        for k in TaskKind::ALL {
            let i = k.info();
            if i.stage == Stage::Coordinate {
                assert!(
                    i.budget_secs <= 180,
                    "{k} coordinates but claims a {}s budget — coordinators enqueue, they do not work",
                    i.budget_secs
                );
            }
        }
        // The metrics chain is what preempts a boot re-index; nothing else should.
        for k in TaskKind::ALL.iter().filter(|k| k.is_high_priority()) {
            assert!(
                matches!(k.pipeline(), Pipeline::Metrics | Pipeline::Activity),
                "{k} claims high priority outside the metrics chain"
            );
        }
    }

    #[test]
    fn typed_constructors_and_accessors_round_trip() {
        let pid = uuid::Uuid::new_v4();
        let t = Task::for_project(TaskKind::AnalyzeProject, &pid);
        assert_eq!(t.project_id().unwrap(), pid, "a project task reads its id back");

        let t = Task::for_capture(TaskKind::IngestCapture, "zed", "thread-1");
        assert_eq!(t.capture_source(), "zed");
        assert_eq!(t.path, "thread-1");

        let t = Task::for_file(TaskKind::ProcessFile, "/repo", "/repo/a.rs");
        assert_eq!(t.folder_abs_path(), "/repo");
        assert_eq!(t.path, "/repo/a.rs");
    }

    #[test]
    fn a_wrong_payload_is_an_error_not_a_silent_miss() {
        // The failure this contract exists to surface. Before it, a folder path
        // handed to a project-scoped kind parsed as garbage deep inside a handler
        // — or worse, looked up nothing and read as "no data".
        let t = Task::new(TaskKind::AnalyzeProject, "", "/not/a/uuid");
        let err = t.project_id().unwrap_err();
        assert!(err.contains("expected a project id"), "got: {err}");
        assert!(err.contains("analyze_project"), "the error names the kind: {err}");
    }

    #[test]
    fn a_malformed_week_bound_refuses_rather_than_running_all_history() {
        // Defaulting to None here would silently run the project's real test suite
        // across its ENTIRE history because a string was malformed — far more work
        // than was asked for, and expensive.
        let pid = uuid::Uuid::new_v4();
        assert_eq!(Task::for_coverage_backfill(&pid, Some(4)).coverage_weeks().unwrap(), Some(4));
        assert_eq!(Task::for_coverage_backfill(&pid, None).coverage_weeks().unwrap(), None);

        let bad = Task::new(TaskKind::BackfillCoverage, "not-a-number", &pid.to_string());
        assert!(bad.coverage_weeks().is_err(), "a malformed bound must not default to all history");
    }

    #[test]
    fn as_of_is_the_one_way_a_task_says_from_this_day() {
        // The phase's claim, as a test: a backfill is a DATE PARAMETER on the
        // normal work, not a separate kind. Metrics already used `as_of` this way;
        // captures now read the same field rather than inventing a second idiom.
        let mut t = Task::new(TaskKind::IngestCaptures, "", "");
        assert_eq!(t.as_of_stamp_ns(), None, "unbounded by default — ingest everything");

        t.as_of = chrono::NaiveDate::from_ymd_opt(2026, 6, 1);
        let stamp = t.as_of_stamp_ns().expect("a bounded task yields a stamp");
        // 2026-06-01T00:00:00Z in epoch nanoseconds.
        let expected = chrono::NaiveDate::from_ymd_opt(2026, 6, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_nanos_opt()
            .unwrap();
        assert_eq!(stamp, expected, "the bound is the day's UTC midnight, in ns");

        // The units it must compare against are mtime nanoseconds, so a unit from
        // the day before the bound sorts below it and a later one above.
        let day_before = expected - 86_400_000_000_000;
        assert!(day_before < stamp, "an older unit is skipped by the bound");
        assert!(expected + 1 > stamp, "a newer unit is kept");
    }
}
