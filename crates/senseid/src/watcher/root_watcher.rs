//! Root watcher — watches registered directories for file changes and enqueues tasks.
//! Singleton pattern: use `RootWatcher::instance(queue)` to access.

use crate::db::pg_store::PgStore;
use crate::languages;
use crate::tasks::queue::TaskQueue;
use crate::tasks::{Task, TaskKind};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const DEBOUNCE_MS: u64 = 500;

const EXCLUDE_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    "target",
    ".git",
    ".next",
    ".svelte-kit",
    "__pycache__",
    ".venv",
    "venv",
];

// ── Types ────────────────────────────────────────────────────────────────

/// Status of the watcher.
#[derive(Debug, Clone, PartialEq)]
pub enum WatcherStatus {
    Watching,
    Stopped(String), // reason
}

/// A root directory with its exclusion list.
#[derive(Debug, Clone)]
pub struct WatchedRoot {
    pub excluded: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ChangeKind {
    Create,
    Modify,
    Delete,
}

// ── Watcher liveness / health ─────────────────────────────────────────────

/// Lock-free liveness + health of the watch thread, shared between the
/// singleton [`RootWatcher`] (read side — the status API + watchdog) and the
/// spawned notify thread (write side). Kept as plain atomics so the watchdog can
/// snapshot health WITHOUT contending on the watch loop or the singleton mutex.
///
/// This is what makes a silent freeze impossible: `last_event_at_ms` is a
/// heartbeat updated on every delivered fs event (previously a local var inside
/// the thread — invisible from outside), and the flags let the watchdog tell a
/// dead thread / errored stream apart from a merely-idle one.
#[derive(Debug)]
pub struct WatcherHealth {
    /// Epoch millis of the last delivered fs event (0 = none since start).
    last_event_at_ms: AtomicI64,
    /// Epoch millis of the last (re)start of the watch thread.
    started_at_ms: AtomicI64,
    /// Number of roots the notify backend is actively watching.
    roots_watched: AtomicUsize,
    /// False once the notify callback reports an error (stream degraded).
    stream_healthy: AtomicBool,
    /// True while the watch thread is alive; flipped false on exit (incl. panic,
    /// via the drop guard).
    thread_alive: AtomicBool,
    /// Watchdog verdict — the single "is the watcher OK?" flag the status API
    /// exposes. Also gates warn-once-per-episode so a stall doesn't spam logs.
    healthy: AtomicBool,
}

impl WatcherHealth {
    fn new() -> Self {
        Self {
            last_event_at_ms: AtomicI64::new(0),
            started_at_ms: AtomicI64::new(0),
            roots_watched: AtomicUsize::new(0),
            stream_healthy: AtomicBool::new(true),
            thread_alive: AtomicBool::new(false),
            healthy: AtomicBool::new(false),
        }
    }

    pub fn last_event_at_ms(&self) -> i64 {
        self.last_event_at_ms.load(Ordering::Relaxed)
    }
    pub fn started_at_ms(&self) -> i64 {
        self.started_at_ms.load(Ordering::Relaxed)
    }
    pub fn roots_watched(&self) -> usize {
        self.roots_watched.load(Ordering::Relaxed)
    }
    pub fn stream_healthy(&self) -> bool {
        self.stream_healthy.load(Ordering::Relaxed)
    }
    pub fn thread_alive(&self) -> bool {
        self.thread_alive.load(Ordering::Relaxed)
    }
    pub fn healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    /// Heartbeat — record that the stream just delivered an event.
    fn touch(&self, now_ms: i64) {
        self.last_event_at_ms.store(now_ms, Ordering::Relaxed);
    }

    /// The notify callback reported an error — the stream can no longer be
    /// trusted. Never swallowed silently: the caller also logs it.
    fn mark_stream_error(&self) {
        self.stream_healthy.store(false, Ordering::Relaxed);
    }

    /// Called by the watch thread once it is up and watching. Resets the stall
    /// clock so a fresh (re)start isn't immediately flagged stalled.
    fn on_thread_start(&self, now_ms: i64, roots_watched: usize) {
        self.started_at_ms.store(now_ms, Ordering::Relaxed);
        self.last_event_at_ms.store(now_ms, Ordering::Relaxed);
        self.roots_watched.store(roots_watched, Ordering::Relaxed);
        self.stream_healthy.store(true, Ordering::Relaxed);
        self.thread_alive.store(true, Ordering::Relaxed);
        self.healthy.store(true, Ordering::Relaxed);
    }

    /// Called when the watch thread exits (any reason, incl. panic via the guard).
    fn on_thread_exit(&self) {
        self.thread_alive.store(false, Ordering::Relaxed);
        self.healthy.store(false, Ordering::Relaxed);
    }

    /// Watchdog sets the overall verdict (surfaced by the status API).
    pub fn set_healthy(&self, v: bool) {
        self.healthy.store(v, Ordering::Relaxed);
    }
}

/// Flips `thread_alive` false whenever the watch thread unwinds — break, return,
/// OR panic. Without this a panicked thread would leave `thread_alive == true`
/// forever and the watchdog would never restart it.
struct AliveGuard(Arc<WatcherHealth>);
impl Drop for AliveGuard {
    fn drop(&mut self) {
        self.0.on_thread_exit();
    }
}

/// Pure watchdog verdict: is the watch thread stalled? A dead thread is stalled
/// unconditionally; a live thread that has delivered no event for `threshold_ms`
/// is *suspected* stalled (indistinguishable from merely idle, so the watchdog's
/// response — a cheap reconcile + stream re-establish — is safe either way).
/// Clock injected so it is testable without sleeping.
pub(crate) fn watcher_is_stalled(
    last_event_at_ms: i64,
    now_ms: i64,
    threshold_ms: i64,
    thread_alive: bool,
) -> bool {
    !thread_alive || now_ms.saturating_sub(last_event_at_ms) >= threshold_ms
}

/// The registered watch root that contains `path` (longest matching prefix), or
/// `None` if `path` is outside every root. Uses component-wise `Path::starts_with`
/// (so `/a/proj` is NOT treated as a prefix of `/a/project`). Pure — reused by
/// the branch-switch and rescan reconcile paths to pick which root to re-scan.
pub(crate) fn watch_root_for_path(path: &Path, roots: &[PathBuf]) -> Option<PathBuf> {
    roots.iter().filter(|r| path.starts_with(r)).max_by_key(|r| r.as_os_str().len()).cloned()
}

/// The path to reconcile after a branch switch: the REPOSITORY whose `.git/HEAD`
/// moved, not the watch root containing it.
///
/// A `ScanRoot` walks from `task.path` (`scan.rs` resolves the enclosing watch
/// root for `root_id`, but discovers folders from the path it was given), so
/// handing it the repository scopes the reconcile to that repository while the
/// folder rows still land under the right root.
///
/// This matters because of scale, not correctness of the old behaviour: a watch
/// root holds MANY repositories (measured on the dev install: 18 at depth 2, 30
/// registered as git folders), so reconciling the root turned one `git checkout`
/// into a discovery walk and a per-folder stat sweep across all of them. The
/// per-FILE cost was never the issue — `process_git_folder`'s
/// two-tier gate skips an unchanged file without reading it, and its own comment
/// names `branch-switch-to-same`.
///
/// `None` when the path is not a `.git/HEAD` or the repository lies outside every
/// watch root. Both are fail-closed on SCOPE: a repository nobody asked us to
/// watch must not pull a scan in, and a helper that reconciled the grandparent of
/// any path would be a trap for the next caller.
pub(crate) fn branch_switch_reconcile_target(
    head_path: &Path,
    roots: &[PathBuf],
) -> Option<PathBuf> {
    // Shape check, component-wise: `<repo>/.git/HEAD`.
    if head_path.file_name()? != "HEAD" {
        return None;
    }
    let git_dir = head_path.parent()?;
    if git_dir.file_name()? != ".git" {
        return None;
    }
    let repo = git_dir.parent()?;
    // Must be inside a watch root — the repo itself being the root is fine, and
    // is then the narrowest scope available.
    watch_root_for_path(repo, roots).map(|_| repo.to_path_buf())
}

/// Given an FSEvents rescan/overflow event's paths and the watch roots, return
/// the roots to force-reconcile. An empty path list (a global overflow with no
/// specific path), or a path outside every root, conservatively reconciles ALL
/// roots — a dropped-events signal must never be under-served. Pure/testable.
pub(crate) fn rescan_reconcile_roots(paths: &[PathBuf], roots: &[PathBuf]) -> Vec<PathBuf> {
    if paths.is_empty() {
        return roots.to_vec();
    }
    let mut out: Vec<PathBuf> = Vec::new();
    for p in paths {
        if let Some(r) = watch_root_for_path(p, roots)
            && !out.contains(&r)
        {
            out.push(r);
        }
    }
    if out.is_empty() { roots.to_vec() } else { out }
}

/// Enqueue one `ScanRoot` reconcile per target — the same task the `scan_folder`
/// API, version-rescan, and reconcile-scheduler use. A target is a watch root for
/// an overflow rescan, or a single REPOSITORY for a branch switch (see
/// [`branch_switch_reconcile_target`]).
///
/// Overlap-guarded per TARGET PATH so watcher-driven reconciles never stack on the
/// same scope, while two different repositories switching branches still both get
/// one. Fire-and-forget onto the tokio runtime because the caller is the
/// (non-async) watch thread.
fn enqueue_scanroot_reconcile(
    rt: &tokio::runtime::Handle,
    queue: &Arc<TaskQueue>,
    roots: Vec<PathBuf>,
) {
    if roots.is_empty() {
        return;
    }
    let q = queue.clone();
    rt.spawn(async move {
        for r in roots {
            let path = r.to_string_lossy().to_string();
            // Per-PATH, not per-kind. The guard was global, which was safe while
            // every reconcile targeted a whole watch root — but branch switches
            // now target a REPOSITORY, and a global guard would let a pending
            // scan of one repository silently drop another's reconcile.
            //
            // Known tradeoff: a pending scan of an ANCESTOR (the whole root) also
            // covers this repository, and this will still enqueue a scoped scan
            // for it. That is one extra repo-scoped, idempotent pass — cheaper
            // than the containment logic it would take to avoid, and far cheaper
            // than the dropped reconcile the global guard risked.
            if q.has_pending_kind_path(TaskKind::ScanRoot, &path).await {
                continue; // a reconcile of this exact target is already queued
            }
            q.enqueue(Task::new(TaskKind::ScanRoot, "", &path)).await;
        }
    });
}

// ── Singleton ────────────────────────────────────────────────────────────

static INSTANCE: OnceLock<Mutex<RootWatcher>> = OnceLock::new();

// ── RootWatcher ──────────────────────────────────────────────────────────

/// Watches registered root directories for file changes and enqueues tasks.
/// Singleton — use `RootWatcher::instance(queue)` to access.
pub struct RootWatcher {
    roots: HashMap<PathBuf, WatchedRoot>,
    queue: Arc<TaskQueue>,
    /// Set at daemon boot (`set_store`). `process_batch` uses it to resolve each
    /// changed file to its owning indexed repo so incremental tasks target the
    /// right folder_path. `None` before boot wiring (e.g. in isolated tests) — the
    /// watch loop then can't resolve and logs a warning instead of enqueueing.
    store: Option<PgStore>,
    status: WatcherStatus,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// Liveness/health shared with the watch thread; survives restarts so the
    /// status API + watchdog always read the same handle.
    health: Arc<WatcherHealth>,
}

impl RootWatcher {
    /// Get or initialize the singleton RootWatcher instance.
    pub fn instance(queue: Arc<TaskQueue>) -> &'static Mutex<RootWatcher> {
        INSTANCE.get_or_init(|| Mutex::new(RootWatcher::new(queue)))
    }

    fn new(queue: Arc<TaskQueue>) -> Self {
        Self {
            roots: HashMap::new(),
            queue,
            store: None,
            status: WatcherStatus::Stopped("no roots".into()),
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            thread: None,
            health: Arc::new(WatcherHealth::new()),
        }
    }

    /// Give the watcher a DB handle so the watch loop can resolve each changed
    /// file to its owning repo. Called once at boot (where `AppState.pg` exists);
    /// persists in the singleton across start/stop restarts. `PgStore` is cheaply
    /// cloneable (Arc'd pool).
    pub fn set_store(&mut self, store: PgStore) {
        self.store = Some(store);
    }

    pub fn register(&mut self, root: PathBuf, exclusions: Vec<String>) {
        self.roots.insert(root, WatchedRoot { excluded: exclusions });
    }

    pub fn unregister(&mut self, root: &PathBuf) {
        self.roots.remove(root);
    }

    pub fn status(&self) -> &WatcherStatus {
        &self.status
    }

    /// Shared liveness/health handle — cloned by the watchdog + status API so a
    /// watcher freeze is queryable from OUTSIDE the watch thread.
    pub fn health(&self) -> Arc<WatcherHealth> {
        self.health.clone()
    }

    pub fn roots(&self) -> &HashMap<PathBuf, WatchedRoot> {
        &self.roots
    }

    pub fn start(&mut self) -> Result<(), String> {
        // Stop existing thread if restarting
        self.stop();

        if self.roots.is_empty() {
            self.status = WatcherStatus::Stopped("no roots".into());
            return Ok(());
        }

        // Reset stop flag for (re)start
        self.stop_flag.store(false, std::sync::atomic::Ordering::Release);

        let stop = self.stop_flag.clone();
        let roots: Vec<PathBuf> = self.roots.keys().cloned().collect();
        let exclusions: Vec<String> =
            self.roots.values().flat_map(|r| r.excluded.clone()).collect();
        let queue = self.queue.clone();
        let health = self.health.clone();
        let store = self.store.clone();

        let rt = tokio::runtime::Handle::try_current()
            .map_err(|_| "RootWatcher requires tokio runtime".to_string())?;

        let thread = std::thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            // The callback used to drop `Err(_)` silently — a stream error would
            // vanish. Now it logs AND marks the stream degraded so the watchdog
            // re-establishes it (no silent errors).
            let health_cb = health.clone();
            let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => { let _ = tx.send(event); }
                    Err(e) => {
                        tracing::warn!(error = %e, "RootWatcher: notify stream error — marking stream degraded");
                        health_cb.mark_stream_error();
                    }
                }
            }).expect("failed to create watcher");

            let mut watched = 0usize;
            for root in &roots {
                match watcher.watch(root, RecursiveMode::Recursive) {
                    Ok(()) => watched += 1,
                    Err(e) => {
                        tracing::warn!(error = %e, root = %root.display(), "failed to watch root")
                    }
                }
            }

            // Publish liveness BEFORE the loop, and flip it false on ANY exit
            // (break / return / panic) via the drop guard.
            health.on_thread_start(chrono::Utc::now().timestamp_millis(), watched);
            let _alive = AliveGuard(health.clone());

            tracing::info!("RootWatcher started: {} roots ({} watched)", roots.len(), watched);

            let mut pending: HashMap<PathBuf, ChangeKind> = HashMap::new();
            let mut last_event = std::time::Instant::now();

            loop {
                if stop.load(std::sync::atomic::Ordering::Acquire) {
                    drop(watcher);
                    break;
                }
                match rx.recv_timeout(Duration::from_millis(DEBOUNCE_MS)) {
                    Ok(event) => {
                        // Heartbeat: any delivered event is liveness proof.
                        health.touch(chrono::Utc::now().timestamp_millis());

                        // FSEvents overflow / kernel-or-user-dropped events. The
                        // notify Rescan flag means "events were lost — the state
                        // you have can no longer be trusted". classify_event would
                        // silently fold this into a Modify and drop it, so handle
                        // it explicitly: force a reconcile of the affected root(s).
                        if event.need_rescan() {
                            let targets = rescan_reconcile_roots(&event.paths, &roots);
                            tracing::warn!(
                                targets = targets.len(),
                                paths = ?event.paths,
                                "RootWatcher: FSEvents rescan/overflow — forcing reconcile of affected root(s)",
                            );
                            enqueue_scanroot_reconcile(&rt, &queue, targets);
                            continue;
                        }

                        let change_kind = RootWatcher::classify_event(&event.kind);

                        for path in event.paths {
                            // A branch switch / rebase / checkout rewrote .git/HEAD.
                            // This is where drift is born (a rename/move under a
                            // switched tree), so force a FULL repo reconcile
                            // (ScanRoot → prunes ghost folders, re-scopes phantom
                            // standalone roots, re-indexes changed files) rather
                            // than only an incremental re-index. Fire even when the
                            // branch is unreadable (detached HEAD mid-rebase).
                            if RootWatcher::is_branch_switch(&path) {
                                // The REPOSITORY, not the watch root containing it:
                                // a root here holds 67 repositories, and one
                                // checkout should not re-walk the other 66.
                                if let Some(repo) = branch_switch_reconcile_target(&path, &roots) {
                                    let branch = read_git_head(&path.to_string_lossy());
                                    tracing::info!(
                                        repo = %repo.display(),
                                        branch = ?branch,
                                        ".git/HEAD changed — reconciling this repository",
                                    );
                                    enqueue_scanroot_reconcile(&rt, &queue, vec![repo]);
                                }
                                continue;
                            }

                            if change_kind != ChangeKind::Delete && !path.is_file() {
                                continue;
                            }
                            if !RootWatcher::should_watch_path(&path, &exclusions) {
                                continue;
                            }

                            pending.insert(path, change_kind);
                            last_event = std::time::Instant::now();
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if !pending.is_empty()
                            && last_event.elapsed() >= Duration::from_millis(DEBOUNCE_MS)
                        {
                            let batch: HashMap<PathBuf, ChangeKind> = std::mem::take(&mut pending);
                            let q = queue.clone();
                            let s = store.clone();
                            rt.spawn(async move {
                                RootWatcher::process_batch(batch, &q, s.as_ref()).await;
                            });
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        self.thread = Some(thread);
        self.status = WatcherStatus::Watching;
        Ok(())
    }

    pub fn stop(&mut self) {
        if self.thread.is_none() {
            return;
        }
        self.stop_flag.store(true, std::sync::atomic::Ordering::Release);
        if let Some(handle) = self.thread.take()
            && handle.join().is_err()
        {
            tracing::error!("RootWatcher thread panicked during shutdown");
        }
        self.status = WatcherStatus::Stopped("manual".into());
    }

    // ── Pure helpers (testable without threads) ──────────────────────

    pub(crate) fn classify_event(kind: &EventKind) -> ChangeKind {
        match kind {
            EventKind::Remove(_) => ChangeKind::Delete,
            EventKind::Create(_) => ChangeKind::Create,
            _ => ChangeKind::Modify,
        }
    }

    pub(crate) fn should_watch_path(path: &Path, exclusions: &[String]) -> bool {
        let path_str = path.to_string_lossy();

        if EXCLUDE_DIRS.iter().any(|d| path_str.contains(&format!("/{}/", d))) {
            return false;
        }
        // ONE owner for the exclusion rule. This used to be a second copy that
        // supported both the absolute and bare forms while `is_excluded`
        // supported only the first — so a bare exclusion gated the watcher here
        // and pruned nothing in the scanner. Delegating is what keeps the two
        // from disagreeing again.
        if crate::tasks::handlers::scan_logic::is_excluded(path, exclusions) {
            return false;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        let is_code = languages::adapter_for_ext(&ext).is_some();
        let is_doc = ext == ".md" || ext == ".mdx";
        is_code || is_doc
    }

    pub(crate) fn is_branch_switch(path: &Path) -> bool {
        let s = path.to_string_lossy();
        s.ends_with(".git/HEAD") || s.ends_with(".git\\HEAD")
    }

    /// Group a debounced batch of changes by their OWNING INDEXED REPO (resolved
    /// from each path via `PgStore::repo_root_for_path`) and enqueue the
    /// incremental tasks — `ProcessFile`/`DeleteFile`/`DeleteFolder` targeting the
    /// repo root's abs_path (the folder_path the handlers resolve by), plus the
    /// post-processing `EmbedNodes` barrier so a live edit shows up in semantic
    /// search without waiting for a full scan. FQN edges resolve at emit (Phase
    /// 7.1), so no ResolveEdges pass is needed for the graph to be current.
    ///
    /// Two invariants from the design:
    ///  - a change in `~/Dev/kavach/src/x.ts` resolves to the kavach repo (not the
    ///    watch root that happens to be `~/Dev`), so `folder_path` is a real repo
    ///    abs_path — the previous code passed the watch-root NAME, so every task
    ///    silently no-op'd;
    ///  - exclusions are applied BEFORE enqueueing (a change under an excluded
    ///    prefix costs nothing).
    ///
    /// A path under no indexed repo (a brand-new repo not yet scanned) is skipped
    /// — a full `ScanRoot` indexes it first. `store` is `None` only before boot
    /// wiring (isolated tests); the batch is then dropped with a warning.
    pub(crate) async fn process_batch(
        changes: HashMap<PathBuf, ChangeKind>,
        queue: &TaskQueue,
        store: Option<&PgStore>,
    ) {
        let Some(store) = store else {
            tracing::warn!(
                count = changes.len(),
                "process_batch: no PgStore — cannot resolve owning repos; batch dropped"
            );
            return;
        };
        // Exclusions are enforced at the event level by `should_watch_path` (each
        // root's `folders_to_watch.excluded`, resolved to absolute prefixes at
        // register), so excluded paths never reach this batch.
        let mut repo_changes: HashMap<String, Vec<(PathBuf, ChangeKind)>> = HashMap::new();
        for (path, kind) in changes {
            match store.repo_root_for_path(&path.to_string_lossy()).await {
                Ok(Some((repo_path, _project))) => {
                    repo_changes.entry(repo_path).or_default().push((path, kind));
                }
                // Not under any indexed repo yet — a full scan must index it first.
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(error = %e, path = %path.display(), "process_batch: repo_root_for_path failed")
                }
            }
        }

        for (repo_path, changes) in repo_changes {
            let mut file_task_ids = Vec::new();

            // FSEvents knows nothing about `.gitignore`, so a raw batch includes
            // generated artifacts the scan deliberately never indexes. Enqueueing
            // those was not merely wasteful, it churned: the scan's walker doesn't
            // see them, so they landed in `plan.removed` and had their nodes
            // deleted and edges unresolved on the next reconcile — then the next
            // build re-created them and the watcher re-added them, forever.
            //
            // Filter Create/Modify through the SAME ignore rules the scan uses,
            // grouped by directory so it costs one read per directory rather than
            // one per file. Deletions are deliberately NOT filtered: a file that
            // has just been removed cannot be "visible", and a previously-indexed
            // file must still be pruned when it disappears.
            //
            // The test is "exists AND is ignored", never bare "not visible" —
            // those are different things. A path absent from the directory listing
            // may simply not be on disk yet (FSEvents can outrun the write, and
            // tests seed repo rows without materialising files), and silently
            // dropping that would lose a real edit until the next reconcile. Only
            // a file we can positively see AND that the ignore rules hide is
            // skipped.
            let mut visible_by_dir: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();
            let is_ignored =
                |path: &PathBuf, cache: &mut HashMap<PathBuf, HashSet<PathBuf>>| -> bool {
                    if !path.exists() {
                        return false;
                    }
                    match path.parent() {
                        Some(parent) => !cache
                            .entry(parent.to_path_buf())
                            .or_insert_with(|| {
                                crate::tasks::handlers::helpers::visible_files_in_dir(parent)
                            })
                            .contains(path),
                        None => false,
                    }
                };

            let mut deleted_dirs: HashSet<PathBuf> = HashSet::new();
            for (path, kind) in &changes {
                if *kind == ChangeKind::Delete
                    && let Some(parent) = path.parent()
                    && !parent.exists()
                    && !deleted_dirs.contains(parent)
                {
                    deleted_dirs.insert(parent.to_path_buf());
                }
            }

            for dir in &deleted_dirs {
                queue
                    .enqueue(Task::new(TaskKind::DeleteFolder, &repo_path, &dir.to_string_lossy()))
                    .await;
            }

            for (path, kind) in &changes {
                if let Some(parent) = path.parent()
                    && deleted_dirs.contains(parent)
                {
                    continue;
                }

                let abs_path = path.to_string_lossy().to_string();
                match kind {
                    ChangeKind::Delete => {
                        let id = queue
                            .enqueue(Task::new(TaskKind::DeleteFile, &repo_path, &abs_path))
                            .await;
                        file_task_ids.push(id);
                    }
                    ChangeKind::Create | ChangeKind::Modify => {
                        if is_ignored(path, &mut visible_by_dir) {
                            tracing::debug!(path = %path.display(),
                                "watcher: path is ignored by the scan's ignore rules — not enqueueing");
                            continue;
                        }
                        let rel_dir = path
                            .parent()
                            .and_then(|p| p.strip_prefix(&repo_path).ok())
                            .map(|p| p.to_string_lossy().replace('\\', "/"))
                            .unwrap_or_default();
                        let mod_name =
                            if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir };
                        let mod_id = format!("mod:{}:{}", repo_path, mod_name);

                        let task = Task::new(TaskKind::ProcessFile, &repo_path, &abs_path)
                            .with_module(&mod_id);
                        let id = queue.enqueue(task).await;
                        file_task_ids.push(id);

                        // If a README changed, re-reconcile its directory's
                        // identity from frontmatter. Enqueued for the README's
                        // parent; the handler no-ops unless that parent is a
                        // project root AND the frontmatter actually changed (so a
                        // subfolder README, or a write-back echo, costs nothing).
                        if path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                            n.eq_ignore_ascii_case("readme.md") || n.eq_ignore_ascii_case("readme")
                        }) && let Some(parent) = path.parent()
                        {
                            queue
                                .enqueue(Task::new(
                                    TaskKind::ReconcileRepoMetadata,
                                    &parent.to_string_lossy(),
                                    &parent.to_string_lossy(),
                                ))
                                .await;
                        }
                    }
                }
            }

            if !file_task_ids.is_empty() {
                // Post-processing barrier for the changed repo, blocked on the file
                // tasks: embed the new/changed nodes so the edit is reflected in the
                // hybrid semantic search immediately. FQN call/import edges resolve
                // at emit (Phase 7.1) — there is no ResolveEdges pass. (Community
                // detection + degree recompute stay periodic — the analyzer
                // scheduler runs DetectCommunities; per-edit clustering is wasteful.)
                queue
                    .enqueue(
                        Task::new(TaskKind::EmbedNodes, &repo_path, "").blocked_by(file_task_ids),
                    )
                    .await;
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Read current branch from .git/HEAD file.
pub(crate) fn read_git_head(head_path: &str) -> Option<String> {
    let content = std::fs::read_to_string(head_path).ok()?;
    let trimmed = content.trim();
    if trimmed.starts_with("ref: refs/heads/") {
        Some(trimmed.strip_prefix("ref: refs/heads/")?.to_string())
    } else {
        None
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_queue() -> Arc<TaskQueue> {
        Arc::new(TaskQueue::new())
    }

    fn make_watcher() -> RootWatcher {
        RootWatcher::new(make_queue())
    }

    // ── new ───────────────────────────────────────────────────────────

    #[test]
    fn new_creates_empty_watcher_with_stopped_status() {
        let watcher = make_watcher();
        assert_eq!(*watcher.status(), WatcherStatus::Stopped("no roots".into()));
        assert!(watcher.roots().is_empty());
    }

    // ── register ──────────────────────────────────────────────────────

    #[test]
    fn register_adds_root_to_map() {
        let mut watcher = make_watcher();
        watcher.register(PathBuf::from("/tmp/project"), vec![]);
        assert_eq!(watcher.roots().len(), 1);
        assert!(watcher.roots().contains_key(&PathBuf::from("/tmp/project")));
    }

    #[test]
    fn register_stores_exclusions() {
        let mut watcher = make_watcher();
        watcher.register(PathBuf::from("/tmp/project"), vec!["node_modules".into(), "dist".into()]);
        let root = &watcher.roots()[&PathBuf::from("/tmp/project")];
        assert_eq!(root.excluded, vec!["node_modules", "dist"]);
    }

    #[test]
    fn register_same_root_twice_updates_exclusions() {
        let mut watcher = make_watcher();
        watcher.register(PathBuf::from("/tmp/project"), vec!["old".into()]);
        watcher.register(PathBuf::from("/tmp/project"), vec!["new".into()]);
        assert_eq!(watcher.roots().len(), 1);
        assert_eq!(watcher.roots()[&PathBuf::from("/tmp/project")].excluded, vec!["new"]);
    }

    #[test]
    fn register_multiple_roots() {
        let mut watcher = make_watcher();
        watcher.register(PathBuf::from("/tmp/a"), vec![]);
        watcher.register(PathBuf::from("/tmp/b"), vec![]);
        assert_eq!(watcher.roots().len(), 2);
    }

    // ── unregister ────────────────────────────────────────────────────

    #[test]
    fn unregister_removes_root() {
        let mut watcher = make_watcher();
        watcher.register(PathBuf::from("/tmp/project"), vec![]);
        watcher.unregister(&PathBuf::from("/tmp/project"));
        assert!(watcher.roots().is_empty());
    }

    #[test]
    fn unregister_nonexistent_root_is_noop() {
        let mut watcher = make_watcher();
        watcher.register(PathBuf::from("/tmp/a"), vec![]);
        watcher.unregister(&PathBuf::from("/tmp/b"));
        assert_eq!(watcher.roots().len(), 1);
    }

    // ── status ────────────────────────────────────────────────────────

    #[test]
    fn status_is_stopped_after_new() {
        let watcher = make_watcher();
        assert_eq!(*watcher.status(), WatcherStatus::Stopped("no roots".into()));
    }

    // ── singleton ─────────────────────────────────────────────────────

    #[test]
    fn instance_returns_same_reference() {
        let q = make_queue();
        let a = RootWatcher::instance(q.clone()) as *const Mutex<RootWatcher>;
        let b = RootWatcher::instance(q) as *const Mutex<RootWatcher>;
        assert_eq!(a, b, "instance() must return the same singleton");
    }

    // ── classify_event ────────────────────────────────────────────────

    #[test]
    fn classify_create_event() {
        assert_eq!(
            RootWatcher::classify_event(&EventKind::Create(notify::event::CreateKind::File)),
            ChangeKind::Create,
        );
    }

    #[test]
    fn classify_modify_event() {
        assert_eq!(
            RootWatcher::classify_event(&EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content
            ))),
            ChangeKind::Modify,
        );
    }

    #[test]
    fn classify_remove_event() {
        assert_eq!(
            RootWatcher::classify_event(&EventKind::Remove(notify::event::RemoveKind::File)),
            ChangeKind::Delete,
        );
    }

    // ── should_watch_path ─────────────────────────────────────────────

    #[test]
    fn should_watch_rust_file() {
        assert!(RootWatcher::should_watch_path(&PathBuf::from("/project/src/main.rs"), &[]));
    }

    #[test]
    fn should_watch_typescript_file() {
        assert!(RootWatcher::should_watch_path(&PathBuf::from("/project/src/app.tsx"), &[]));
    }

    #[test]
    fn should_watch_markdown_file() {
        assert!(RootWatcher::should_watch_path(&PathBuf::from("/project/docs/README.md"), &[]));
    }

    #[test]
    fn should_not_watch_image_file() {
        assert!(!RootWatcher::should_watch_path(&PathBuf::from("/project/logo.png"), &[]));
    }

    #[test]
    fn should_not_watch_node_modules() {
        assert!(!RootWatcher::should_watch_path(
            &PathBuf::from("/project/node_modules/foo/index.js"),
            &[]
        ));
    }

    #[test]
    fn should_not_watch_custom_exclusion() {
        assert!(!RootWatcher::should_watch_path(
            &PathBuf::from("/project/vendor/lib.rs"),
            &["vendor".into()]
        ));
    }

    #[test]
    fn should_not_watch_absolute_prefix_exclusion() {
        // An absolute-path exclusion (starts with `/`) is a subtree prefix, not a
        // path segment — a file under it is ignored, a sibling sharing the prefix
        // string is not.
        let ex = vec!["/Users/x/Developer/Code".to_string()];
        assert!(!RootWatcher::should_watch_path(
            &PathBuf::from("/Users/x/Developer/Code/repo/lib.rs"),
            &ex
        ));
        assert!(RootWatcher::should_watch_path(
            &PathBuf::from("/Users/x/Developer/Coder/lib.rs"),
            &ex
        ));
    }

    // ── is_branch_switch ──────────────────────────────────────────────

    #[test]
    fn detects_git_head_as_branch_switch() {
        assert!(RootWatcher::is_branch_switch(&PathBuf::from("/project/.git/HEAD")));
    }

    #[test]
    fn non_git_file_is_not_branch_switch() {
        assert!(!RootWatcher::is_branch_switch(&PathBuf::from("/project/src/main.rs")));
    }

    // ── watcher_is_stalled (watchdog decision) ────────────────────────

    #[test]
    fn watcher_is_stalled_dead_thread_is_always_stalled() {
        // A dead thread is stalled regardless of how recent the last event was.
        assert!(watcher_is_stalled(1_000_000, 1_000_000, 900_000, false));
    }

    #[test]
    fn watcher_is_stalled_live_and_fresh_is_not_stalled() {
        let now = 1_000_000;
        assert!(!watcher_is_stalled(now - 10_000, now, 900_000, true));
    }

    #[test]
    fn watcher_is_stalled_live_but_quiet_past_threshold() {
        let now = 2_000_000;
        // Exactly at the threshold counts as stalled (>=), and beyond it too.
        assert!(watcher_is_stalled(now - 900_000, now, 900_000, true));
        assert!(watcher_is_stalled(now - 900_001, now, 900_000, true));
        // One ms under the threshold is still healthy.
        assert!(!watcher_is_stalled(now - 899_999, now, 900_000, true));
    }

    // ── enqueue_scanroot_reconcile dedup ─────────────────────────────

    /// Two repositories switching branches must BOTH get a reconcile.
    ///
    /// The guard used to be `has_pending_kind(ScanRoot)` — global. That was safe
    /// while every target was a whole watch root, but with per-repository targets
    /// it would let the first repository's queued scan silently swallow the
    /// second's. A dropped reconcile is invisible: the graph just keeps the old
    /// branch's folders.
    #[tokio::test]
    async fn reconcile_dedups_per_target_not_across_repos() {
        let q = std::sync::Arc::new(TaskQueue::new());
        let rt = tokio::runtime::Handle::current();
        let a = PathBuf::from("/dev/repo-a");
        let b = PathBuf::from("/dev/repo-b");

        enqueue_scanroot_reconcile(&rt, &q, vec![a.clone()]);
        enqueue_scanroot_reconcile(&rt, &q, vec![b.clone()]);
        // The enqueue is fire-and-forget onto the runtime; let both land.
        for _ in 0..50 {
            if q.has_pending_kind_path(TaskKind::ScanRoot, &b.to_string_lossy()).await {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        assert!(
            q.has_pending_kind_path(TaskKind::ScanRoot, &a.to_string_lossy()).await,
            "repo-a's reconcile is queued",
        );
        assert!(
            q.has_pending_kind_path(TaskKind::ScanRoot, &b.to_string_lossy()).await,
            "repo-b's reconcile is queued too — a global guard would have dropped it",
        );
    }

    /// The same target twice collapses, which is what the guard is for.
    #[tokio::test]
    async fn reconcile_dedups_the_same_target() {
        let q = std::sync::Arc::new(TaskQueue::new());
        let rt = tokio::runtime::Handle::current();
        let a = PathBuf::from("/dev/repo-dedup");

        enqueue_scanroot_reconcile(&rt, &q, vec![a.clone()]);
        for _ in 0..50 {
            if q.has_pending_kind_path(TaskKind::ScanRoot, &a.to_string_lossy()).await {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        enqueue_scanroot_reconcile(&rt, &q, vec![a.clone()]);
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;

        let queued = q
            .snapshot()
            .await
            .iter()
            .filter(|(kind, _, p)| {
                *kind == TaskKind::ScanRoot && p == &a.to_string_lossy().to_string()
            })
            .count();
        assert_eq!(queued, 1, "one reconcile per target, not one per event");
    }

    // ── branch_switch_reconcile_target ───────────────────────────────

    /// A branch switch must reconcile the REPOSITORY whose HEAD moved, not the
    /// watch root containing it.
    ///
    /// MEASURED: `/Users/Jerry/Developer` holds 67 repositories, so the previous
    /// behaviour turned one `git checkout` into a folder-discovery walk and a
    /// per-folder stat sweep across all 67. The per-FILE cost was never the
    /// problem — `process_git_folder`'s two-tier gate already skips an unchanged
    /// file without reading it, and its own comment names
    /// `branch-switch-to-same`. The problem was the scope.
    #[test]
    fn branch_switch_reconciles_the_repo_not_the_whole_root() {
        let roots = vec![PathBuf::from("/dev")];
        assert_eq!(
            branch_switch_reconcile_target(&PathBuf::from("/dev/sensei/.git/HEAD"), &roots),
            Some(PathBuf::from("/dev/sensei")),
            "the repo, not /dev — reconciling the root re-walks every sibling repository",
        );
    }

    #[test]
    fn branch_switch_target_handles_a_nested_repo() {
        let roots = vec![PathBuf::from("/dev")];
        assert_eq!(
            branch_switch_reconcile_target(&PathBuf::from("/dev/group/repo/.git/HEAD"), &roots),
            Some(PathBuf::from("/dev/group/repo")),
        );
    }

    /// A repository that IS its own watch root reconciles that root — there is no
    /// narrower scope available, and returning None would drop the reconcile.
    #[test]
    fn branch_switch_target_allows_the_repo_to_be_the_root() {
        let roots = vec![PathBuf::from("/dev/sensei")];
        assert_eq!(
            branch_switch_reconcile_target(&PathBuf::from("/dev/sensei/.git/HEAD"), &roots),
            Some(PathBuf::from("/dev/sensei")),
        );
    }

    /// Outside every root there is nothing to reconcile. Fail closed on scope:
    /// a repository nobody asked us to watch must not pull a scan in.
    #[test]
    fn branch_switch_target_is_none_outside_every_root() {
        let roots = vec![PathBuf::from("/dev")];
        assert_eq!(
            branch_switch_reconcile_target(&PathBuf::from("/elsewhere/repo/.git/HEAD"), &roots),
            None,
        );
    }

    /// Defensive: only a `.git/HEAD` shape yields a target. `is_branch_switch`
    /// gates the caller today, but a helper that silently reconciled the
    /// grandparent of ANY path would be a trap for the next caller.
    #[test]
    fn branch_switch_target_is_none_for_a_non_head_path() {
        let roots = vec![PathBuf::from("/dev")];
        assert_eq!(
            branch_switch_reconcile_target(&PathBuf::from("/dev/sensei/src/main.rs"), &roots),
            None,
        );
        assert_eq!(
            branch_switch_reconcile_target(&PathBuf::from("/dev/sensei/HEAD"), &roots),
            None,
            "HEAD not under .git is not a branch switch",
        );
    }

    // ── watch_root_for_path ───────────────────────────────────────────

    #[test]
    fn watch_root_for_path_matches_containing_root() {
        let roots = vec![PathBuf::from("/a/project")];
        assert_eq!(
            watch_root_for_path(&PathBuf::from("/a/project/src/main.rs"), &roots),
            Some(PathBuf::from("/a/project")),
        );
    }

    #[test]
    fn watch_root_for_path_none_when_outside() {
        let roots = vec![PathBuf::from("/a/project")];
        assert_eq!(watch_root_for_path(&PathBuf::from("/b/other/x.rs"), &roots), None);
    }

    #[test]
    fn watch_root_for_path_is_component_wise_not_string_prefix() {
        // /a/project must NOT match /a/projectX (the classic string-prefix bug).
        let roots = vec![PathBuf::from("/a/project")];
        assert_eq!(watch_root_for_path(&PathBuf::from("/a/projectX/f.rs"), &roots), None);
    }

    #[test]
    fn watch_root_for_path_picks_longest_prefix_for_nested_roots() {
        let roots = vec![PathBuf::from("/a"), PathBuf::from("/a/nested")];
        assert_eq!(
            watch_root_for_path(&PathBuf::from("/a/nested/src/x.rs"), &roots),
            Some(PathBuf::from("/a/nested")),
        );
    }

    #[test]
    fn watch_root_for_path_maps_git_head_to_its_root() {
        // The branch-switch path: .git/HEAD resolves to the repo's watch root.
        let roots = vec![PathBuf::from("/a/repo")];
        assert_eq!(
            watch_root_for_path(&PathBuf::from("/a/repo/.git/HEAD"), &roots),
            Some(PathBuf::from("/a/repo")),
        );
    }

    // ── rescan_reconcile_roots (FSEvents overflow) ────────────────────

    #[test]
    fn rescan_empty_paths_reconciles_all_roots() {
        let roots = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        assert_eq!(rescan_reconcile_roots(&[], &roots), roots);
    }

    #[test]
    fn rescan_targets_only_affected_root_and_dedupes() {
        let roots = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        let paths = vec![
            PathBuf::from("/a/one.rs"),
            PathBuf::from("/a/two.rs"), // same root → deduped
        ];
        assert_eq!(rescan_reconcile_roots(&paths, &roots), vec![PathBuf::from("/a")]);
    }

    #[test]
    fn rescan_path_outside_all_roots_falls_back_to_all() {
        let roots = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        let paths = vec![PathBuf::from("/z/orphan.rs")];
        assert_eq!(rescan_reconcile_roots(&paths, &roots), roots);
    }

    #[test]
    fn need_rescan_detects_the_flag() {
        // Guards our reliance on notify's Rescan flag: a flagged event reports
        // need_rescan(), a plain one does not.
        use notify::event::Flag;
        let rescan = Event::new(EventKind::Any).set_flag(Flag::Rescan);
        assert!(rescan.need_rescan());
        let plain = Event::new(EventKind::Modify(notify::event::ModifyKind::Any));
        assert!(!plain.need_rescan());
    }

    // ── WatcherHealth ─────────────────────────────────────────────────

    #[test]
    fn watcher_health_lifecycle() {
        let h = WatcherHealth::new();
        // Fresh: not alive, not healthy, stream assumed ok.
        assert!(!h.thread_alive());
        assert!(!h.healthy());
        assert!(h.stream_healthy());
        assert_eq!(h.last_event_at_ms(), 0);

        // Thread comes up → alive + healthy, clock reset, roots recorded.
        h.on_thread_start(1_000, 3);
        assert!(h.thread_alive());
        assert!(h.healthy());
        assert_eq!(h.roots_watched(), 3);
        assert_eq!(h.last_event_at_ms(), 1_000);

        // Heartbeat advances the clock.
        h.touch(5_000);
        assert_eq!(h.last_event_at_ms(), 5_000);

        // A stream error degrades the stream (but doesn't touch liveness).
        h.mark_stream_error();
        assert!(!h.stream_healthy());
        assert!(h.thread_alive());

        // Exit flips alive + healthy false.
        h.on_thread_exit();
        assert!(!h.thread_alive());
        assert!(!h.healthy());
    }

    // ── read_git_head ─────────────────────────────────────────────────

    #[test]
    fn read_git_head_valid_branch() {
        let dir = tempfile::tempdir().unwrap();
        let head = dir.path().join("HEAD");
        std::fs::write(&head, "ref: refs/heads/main\n").unwrap();
        assert_eq!(read_git_head(head.to_str().unwrap()), Some("main".to_string()));
    }

    #[test]
    fn read_git_head_detached() {
        let dir = tempfile::tempdir().unwrap();
        let head = dir.path().join("HEAD");
        std::fs::write(&head, "abc123def456\n").unwrap();
        assert_eq!(read_git_head(head.to_str().unwrap()), None);
    }

    #[test]
    fn read_git_head_missing_file() {
        assert_eq!(read_git_head("/nonexistent/path/HEAD"), None);
    }

    // ── process_batch ─────────────────────────────────────────────────

    /// Seed a `git` repo folder in the DB so `repo_root_for_path` resolves a
    /// change under it. Returns `(pg, repo_abs_path, root_id)`.
    async fn seed_watch_repo() -> (PgStore, String, uuid::Uuid) {
        let pg = PgStore::connect_test().await.unwrap();
        let uniq = uuid::Uuid::new_v4();
        let root = format!("/_test/watch/{uniq}");
        let repo = format!("{root}/repo");
        let root_id = pg.add_watch_root(&root, "wt", &serde_json::json!([])).await.unwrap();
        pg.upsert_repo_kind(&root_id, "git", "repo", &repo).await.unwrap();
        (pg, repo, root_id)
    }

    async fn cleanup_watch_repo(pg: &PgStore, root_id: &uuid::Uuid) {
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id=$1")
            .bind(root_id)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id=$1")
            .bind(root_id)
            .execute(pool)
            .await
            .ok();
    }

    /// The watcher must not enqueue a file the scan's ignore rules hide, and must
    /// still enqueue its tracked neighbour.
    ///
    /// This is the churn the filter exists to stop: FSEvents knows nothing about
    /// `.gitignore`, so it reported 131 generated i18n files; the scan's walker
    /// correctly never saw them, so every reconcile put them in `plan.removed`,
    /// deleted their nodes and unresolved their edges — and the next build
    /// re-created them and the watcher re-added them. Forever.
    #[tokio::test]
    async fn process_batch_skips_gitignored_paths_but_keeps_tracked_ones() {
        let pg = PgStore::connect_test().await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let repo_dir = tmp.path().join("repo");
        let gen_dir = repo_dir.join("generated");
        std::fs::create_dir_all(&gen_dir).unwrap();
        // The real-world shape: a generated dir carrying a `.gitignore` of `*`.
        std::fs::write(gen_dir.join(".gitignore"), "*\n").unwrap();
        let ignored = gen_dir.join("messages.js");
        std::fs::write(&ignored, "export const a = 1\n").unwrap();
        let tracked = repo_dir.join("lib.rs");
        std::fs::write(&tracked, "fn main() {}\n").unwrap();

        let root = tmp.path().to_string_lossy().to_string();
        let repo = repo_dir.to_string_lossy().to_string();
        let root_id = pg.add_watch_root(&root, "wt_ignored", &serde_json::json!([])).await.unwrap();
        pg.upsert_repo_kind(&root_id, "git", "repo", &repo).await.unwrap();

        let queue = Arc::new(TaskQueue::new());
        let mut changes = HashMap::new();
        changes.insert(ignored.clone(), ChangeKind::Modify);
        changes.insert(tracked.clone(), ChangeKind::Modify);
        RootWatcher::process_batch(changes, &queue, Some(&pg)).await;

        let snap = queue.snapshot().await;
        let queued: Vec<String> = snap
            .iter()
            .filter(|(k, _, _)| *k == TaskKind::ProcessFile)
            .map(|(_, _, p)| p.clone())
            .collect();
        assert!(
            queued.iter().any(|p| p == &tracked.to_string_lossy()),
            "the tracked file must still be enqueued, got {queued:?}"
        );
        assert!(
            !queued.iter().any(|p| p == &ignored.to_string_lossy()),
            "a gitignored file must NOT be enqueued, got {queued:?}"
        );

        cleanup_watch_repo(&pg, &root_id).await;
    }

    /// Deletions are deliberately NOT ignore-filtered: a previously-indexed file
    /// must still be pruned when it disappears, and a deleted path can never be
    /// "visible" in a directory listing.
    #[tokio::test]
    async fn process_batch_still_deletes_a_vanished_path() {
        let pg = PgStore::connect_test().await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        // The DIRECTORY must still exist — a missing parent is the separate
        // folder-deletion case, which enqueues DeleteFolder instead.
        let src = tmp.path().join("repo/src");
        std::fs::create_dir_all(&src).unwrap();
        let gone = src.join("gone.rs"); // deliberately never created

        let root = tmp.path().to_string_lossy().to_string();
        let repo = tmp.path().join("repo").to_string_lossy().to_string();
        let root_id = pg.add_watch_root(&root, "wt_del", &serde_json::json!([])).await.unwrap();
        pg.upsert_repo_kind(&root_id, "git", "repo", &repo).await.unwrap();

        let queue = Arc::new(TaskQueue::new());
        let mut changes = HashMap::new();
        changes.insert(gone, ChangeKind::Delete);
        RootWatcher::process_batch(changes, &queue, Some(&pg)).await;

        let snap = queue.snapshot().await;
        assert!(
            snap.iter().any(|(k, _, _)| *k == TaskKind::DeleteFile),
            "a delete must still be enqueued even though the path is gone"
        );
        cleanup_watch_repo(&pg, &root_id).await;
    }

    #[tokio::test]
    async fn process_batch_resolves_owning_repo_and_adds_postprocessing() {
        // A change under the repo resolves to the repo abs_path (NOT a watch-root
        // name) and enqueues ProcessFile + EmbedNodes. Phase 7.1: FQN edges resolve
        // at emit, so there is NO ResolveEdges pass in the incremental path.
        let (pg, repo, root_id) = seed_watch_repo().await;
        let queue = Arc::new(TaskQueue::new());
        let mut changes = HashMap::new();
        changes.insert(PathBuf::from(format!("{repo}/src/lib.rs")), ChangeKind::Modify);
        RootWatcher::process_batch(changes, &queue, Some(&pg)).await;

        let snap = queue.snapshot().await;
        assert!(snap.iter().any(|(k, _, _)| *k == TaskKind::ProcessFile), "ProcessFile enqueued");
        assert!(
            snap.iter().any(|(k, _, _)| *k == TaskKind::EmbedNodes),
            "EmbedNodes enqueued (search freshness)"
        );
        assert!(
            !snap.iter().any(|(k, _, _)| k.to_string() == "resolve_edges"),
            "no ResolveEdges pass — FQN edges resolve at emit"
        );
        let pf = snap.iter().find(|(k, _, _)| *k == TaskKind::ProcessFile).unwrap();
        assert_eq!(pf.1, repo, "ProcessFile folder_path is the resolved repo abs_path, not a name");
        cleanup_watch_repo(&pg, &root_id).await;
    }

    #[tokio::test]
    async fn process_batch_delete_targets_repo() {
        // Seed the DB repo at a REAL tempdir so the DELETE branch's parent.exists()
        // check passes → DeleteFile (not DeleteFolder), targeting the repo abs_path.
        let pg = PgStore::connect_test().await.unwrap();
        let dir = tempfile::tempdir().unwrap(); // unique watch root
        let repo = dir.path().join("repo").to_string_lossy().to_string();
        let root_id = pg
            .add_watch_root(
                &dir.path().to_string_lossy(),
                &format!("wt-{}", uuid::Uuid::new_v4()),
                &serde_json::json!([]),
            )
            .await
            .unwrap();
        pg.upsert_repo_kind(&root_id, "git", "repo", &repo).await.unwrap();

        let src = dir.path().join("repo").join("src");
        std::fs::create_dir_all(&src).unwrap();
        let file = src.join("old.rs"); // parent (src) exists → DeleteFile branch
        let queue = Arc::new(TaskQueue::new());
        let mut changes = HashMap::new();
        changes.insert(file, ChangeKind::Delete);
        RootWatcher::process_batch(changes, &queue, Some(&pg)).await;

        let snap = queue.snapshot().await;
        let df = snap.iter().find(|(k, _, _)| *k == TaskKind::DeleteFile);
        assert!(df.is_some(), "DeleteFile enqueued");
        assert_eq!(df.unwrap().1, repo, "DeleteFile folder_path is the repo abs_path");
        cleanup_watch_repo(&pg, &root_id).await;
    }

    #[tokio::test]
    async fn process_batch_skips_paths_under_no_indexed_repo() {
        // A change under no indexed repo resolves to nothing → no tasks.
        let pg = PgStore::connect_test().await.unwrap();
        let queue = Arc::new(TaskQueue::new());
        let mut changes = HashMap::new();
        changes.insert(
            PathBuf::from(format!("/_test/nonexistent/{}/file.rs", uuid::Uuid::new_v4())),
            ChangeKind::Modify,
        );
        RootWatcher::process_batch(changes, &queue, Some(&pg)).await;
        let status = queue.status().await;
        assert_eq!(status.pending + status.blocked, 0);
    }

    // ── start/stop lifecycle ──────────────────────────────────────────

    #[tokio::test]
    async fn start_with_no_roots_stays_stopped() {
        let mut watcher = make_watcher();
        let result = watcher.start();
        assert!(result.is_ok());
        assert_eq!(*watcher.status(), WatcherStatus::Stopped("no roots".into()));
    }

    #[tokio::test]
    async fn start_with_roots_becomes_watching() {
        let tmp = tempfile::tempdir().unwrap();
        let mut watcher = make_watcher();
        watcher.register(tmp.path().to_path_buf(), vec![]);
        watcher.start().unwrap();
        assert_eq!(*watcher.status(), WatcherStatus::Watching);
        watcher.stop();
    }

    #[tokio::test]
    async fn stop_sets_status_to_stopped() {
        let tmp = tempfile::tempdir().unwrap();
        let mut watcher = make_watcher();
        watcher.register(tmp.path().to_path_buf(), vec![]);
        watcher.start().unwrap();
        watcher.stop();
        assert_eq!(*watcher.status(), WatcherStatus::Stopped("manual".into()));
    }

    #[tokio::test]
    async fn stop_on_stopped_watcher_is_noop() {
        let mut watcher = make_watcher();
        watcher.stop();
        assert_eq!(*watcher.status(), WatcherStatus::Stopped("no roots".into()));
    }

    #[tokio::test]
    async fn start_stop_start_works() {
        let tmp = tempfile::tempdir().unwrap();
        let mut watcher = make_watcher();
        watcher.register(tmp.path().to_path_buf(), vec![]);
        watcher.start().unwrap();
        assert_eq!(*watcher.status(), WatcherStatus::Watching);
        watcher.stop();
        assert_eq!(*watcher.status(), WatcherStatus::Stopped("manual".into()));
        watcher.start().unwrap();
        assert_eq!(*watcher.status(), WatcherStatus::Watching);
        watcher.stop();
    }
}
