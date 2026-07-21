---
type: design
---

# Workers — module

Behind-the-scenes design for the background workers behind [Setup](../features/01-setup.md)'s
folder scan and [Observatory](../features/03-observatory.md)'s Logs/diagnostics section. The
feature docs say what the user sees; this says how the daemon keeps the graph
current and surfaces what it's doing. Full task-system overview:
[`../architecture/daemon.md`](../architecture/daemon.md).

## Task system core

- Crate: `crates/senseid/src/tasks/` — `TaskKind` enum (`mod.rs`), `TaskQueue`
  (`queue.rs`), worker pool (`executor.rs`), handlers (`handlers/`).
- `executor::spawn_workers` runs N tokio tasks pulling from `TaskQueue::next_task`;
  each dispatches to a handler, then `queue.complete`/`queue.fail`.
- Tasks form a dependency tree with **barriers**: `scan_root → process_git_folder
  → process_file → resolve_edges → build_connections → embed_nodes →
  detect_communities`. A barrier task waits for all its children
  (`TaskQueue::add_dependency`).
- Every task execution is recorded (`task_executions` table) — start/end,
  success/failure, error string (never `.ok()`-swallowed).

## Folder scan (`ScanRoot`)

- Handler: `crates/senseid/src/tasks/handlers/scan.rs` +
  `handlers/scan_logic.rs` (`plan_reindex` — the two-tier stat-only /
  content-hash gate).
- `ScanRoot` classifies folders into project roots (git repos + quasi-repos to
  depth 2); `ProcessGitFolder` walks a repo and attributes every file to the
  git-root owner (one repo = one project = one owner invariant).
- Per-file `ProcessFile` parses via language adapters into adapter-IR
  nodes/edges (worker-parallel); `ResolveEdges`/`BuildConnections` are separate
  barrier phases after all file-tasks settle.
- API entry: `POST /api/scan` (see `crates/senseid/src/api/routes.rs`) enqueues
  `ScanRoot`; app roots UI is `app/src/routes/(config)/setup/{roots,scan}`.

## Incremental watcher (`RootWatcher`)

- Crate: `crates/senseid/src/watcher/root_watcher.rs` — singleton
  `RootWatcher::instance(queue)`, built on `notify` (FSEvents on macOS),
  500ms debounce, `EXCLUDE_DIRS` (node_modules, dist, target, .git, etc).
- Converts fs `Event`s into `ProcessFile`/`DeleteFile`/`DeleteFolder` tasks;
  watcher-originated tasks outrank bulk-scan in the queue so live edits jump
  ahead of a running scan.
- `WatcherHealth` (lock-free atomics: `last_event_at_ms`, thread-alive flag) is
  the heartbeat read by the watchdog scheduler — this is what makes a silent
  freeze impossible (previously a local var invisible from outside).

## Reconcile scheduler — the safety net

- `crates/senseid/src/tasks/reconcile_scheduler.rs` — long-lived tokio task,
  re-enqueues one `ScanRoot` per watch root on `reconcile.interval_secs`
  (default 300s), **boot tick always fires** so a restart can never leave
  drift.
- Cheap by construction: `plan_reindex`'s stat-only gate makes a no-op
  reconcile near-free, so it can run frequently instead of hourly.
- Overlap-guarded (skips a tick if `ScanRoot` already in flight) and
  watermarked (`sensei.config` key `reconcile.last_run`) for observability.
- `crates/senseid/src/tasks/watchdog_scheduler.rs` separately watches
  `WatcherHealth` staleness and forces a reconcile + re-establishes the fs
  stream if the watch thread looks stalled (config: watcher-stall threshold in
  `sensei.config`). Note: this file also hosts the unrelated relay-engine
  `AdvanceRun` run-watchdog (`crate::run_watchdog::assess_run`) — same pattern,
  different subsystem.
- `crates/senseid/src/tasks/version_rescan.rs` — triggers a full re-scan on a
  daemon version bump (schema/adapter changes invalidate prior parses).

## Analyzer scheduler — the learning passes

- `crates/senseid/src/tasks/analyzer_scheduler.rs` — long-lived tokio task,
  wakes on `DEFAULT_INTERVAL_SECS` (3600s) and enqueues `AnalyzeProject` +
  `ScanDocDrift` for every project whose sessions changed since a **persisted
  watermark** (`sensei.config` key `analyzer.watermark`) — so a restart doesn't
  re-analyze everything.
- `projects_due` (pure fn) computes the due set and advances the watermark in
  one pass — unit-testable without the infinite `run` loop.
- Once per tick, global passes `AggregateCorrections`, `AggregateToolInsights`,
  and verdict classification run (`enqueue_due_project` / tick helper) — see
  `handlers/analyze.rs` for `AnalyzeProject`'s L0 session-enrichment logic and
  `handlers/doc_drift.rs` for `ScanDocDrift`.
- **Daily full refresh** (`DEFAULT_FULL_REFRESH_SECS` = 86400s): all active
  projects re-analyzed regardless of new activity (decay/staleness/ranking
  insights), and this is also when `DetectCommunities` runs per indexed folder
  (expensive label-propagation, deliberately not on every hot tick).
- On-demand path: the same `AnalyzeProject` task can be enqueued directly from
  the API (see `../architecture/daemon.md` pipelines table for what each
  downstream pipeline produces — signals, patterns, inferencing, memory,
  insight-copy, traceability).

## Background-task visibility

- SSE progress: `crates/senseid/src/tasks/progress.rs` defines `TaskEvent`
  (`Queued`/`Started`/`Completed`/`Failed`/`FolderQueued`) broadcast off
  `TaskQueue::sender()`; `progress_emitter.rs` translates per-file
  `TaskEvent::Completed` into `StateEvent::folder_update` /
  `ActivityEvent` SSE frames consumed by the app (`/api/scan/events`).
  Throttling is a code constant (`THROTTLE_ENABLED`, currently `false` —
  raw per-file events forwarded verbatim; client-side state coalesces).
- Diagnostics/Logs: `POST /api/logs` ingests a structured entry from
  cli/mcp/app/daemon (`crates/senseid/src/api/handlers/logs.rs`,
  `insert_log`); `GET` side paginates with a `DEFAULT_LOG_LIMIT`/hard-cap.
  App surface: `app/src/routes/(observatory)/activity-logs/` (`LogRow.svelte`,
  `activity-logs.svelte.ts` state slice).
- `TaskQueue::status`/`snapshot`/`progress` give an in-memory queue-depth view
  (per-repo `RepoProgress`: total/pending/running/completed/failed +
  current_file) — this is what a future Instruments/queue-health panel would
  read (Observatory's `- [~] Instruments` is not yet wired to this).
- Retention: `crates/senseid/src/tasks/log_pruner.rs` and `activity_pruner.rs`
  are long-lived tokio tasks that delete rows past a configured day-count
  (mirroring the reconcile/analyzer scheduler shape).

## Future

- Instruments-Health panel joining MCP tool registry ↔ usage (parked —
  registry and usage tables don't currently join; see memory
  `project_vacation_run_2026_07`).
- Dōjō auto-discover (feature doc, not yet built) would likely be a new
  `TaskKind` off the same scan pipeline, classifying a repo's GitHub remote
  post-`ProcessGitFolder`.
</content>
