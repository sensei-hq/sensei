//! Root watcher — watches registered directories for file changes and enqueues tasks.
//! Singleton pattern: use `RootWatcher::instance(queue)` to access.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use std::collections::{HashMap, HashSet};
use notify::{Watcher, RecursiveMode, Event, EventKind};
use crate::tasks::{Task, TaskKind};
use crate::tasks::queue::TaskQueue;
use crate::languages;

const DEBOUNCE_MS: u64 = 500;

const EXCLUDE_DIRS: &[&str] = &[
    "node_modules", "dist", "build", "target", ".git",
    ".next", ".svelte-kit", "__pycache__", ".venv", "venv",
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

// ── Singleton ────────────────────────────────────────────────────────────

static INSTANCE: OnceLock<Mutex<RootWatcher>> = OnceLock::new();

// ── RootWatcher ──────────────────────────────────────────────────────────

/// Watches registered root directories for file changes and enqueues tasks.
/// Singleton — use `RootWatcher::instance(queue)` to access.
pub struct RootWatcher {
    roots: HashMap<PathBuf, WatchedRoot>,
    queue: Arc<TaskQueue>,
    status: WatcherStatus,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
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
            status: WatcherStatus::Stopped("no roots".into()),
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            thread: None,
        }
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
        let exclusions: Vec<String> = self.roots.values()
            .flat_map(|r| r.excluded.clone())
            .collect();
        let queue = self.queue.clone();

        let rt = tokio::runtime::Handle::try_current()
            .map_err(|_| "RootWatcher requires tokio runtime".to_string())?;

        let thread = std::thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            }).expect("failed to create watcher");

            for root in &roots {
                watcher.watch(root, RecursiveMode::Recursive).ok();
            }

            tracing::info!("RootWatcher started: {} roots", roots.len());

            let mut pending: HashMap<PathBuf, ChangeKind> = HashMap::new();
            let mut last_event = std::time::Instant::now();

            // Build projects list (root name → root path) for batch processing
            let projects: Vec<(String, String)> = roots.iter()
                .map(|r| {
                    let name = r.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    (name, r.to_string_lossy().to_string())
                })
                .collect();

            loop {
                if stop.load(std::sync::atomic::Ordering::Acquire) {
                    drop(watcher);
                    break;
                }
                match rx.recv_timeout(Duration::from_millis(DEBOUNCE_MS)) {
                    Ok(event) => {
                        let change_kind = RootWatcher::classify_event(&event.kind);

                        for path in event.paths {
                            if RootWatcher::is_branch_switch(&path) {
                                let new_branch = read_git_head(&path.to_string_lossy());
                                if let Some(branch) = new_branch {
                                    if let Some((repo_id, _)) = projects.iter()
                                        .find(|(_, rp)| path.to_string_lossy().starts_with(rp.as_str()))
                                    {
                                        let q = queue.clone();
                                        let rid = repo_id.clone();
                                        let br = branch.clone();
                                        rt.spawn(async move {
                                            let task = Task::new(TaskKind::BranchSwitch, &rid, "")
                                                .with_branch(&br);
                                            q.enqueue(task).await;
                                        });
                                        tracing::info!("Branch switch: {} → {}", repo_id, branch);
                                    }
                                }
                                continue;
                            }

                            if change_kind != ChangeKind::Delete && !path.is_file() { continue; }
                            if !RootWatcher::should_watch_path(&path, &exclusions) { continue; }

                            pending.insert(path, change_kind);
                            last_event = std::time::Instant::now();
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if !pending.is_empty() && last_event.elapsed() >= Duration::from_millis(DEBOUNCE_MS) {
                            let batch: HashMap<PathBuf, ChangeKind> = std::mem::take(&mut pending);
                            let q = queue.clone();
                            let p = projects.clone();
                            rt.spawn(async move {
                                RootWatcher::process_batch(batch, &q, &p).await;
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
        if let Some(handle) = self.thread.take() {
            handle.join().ok();
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

    pub(crate) fn should_watch_path(path: &PathBuf, exclusions: &[String]) -> bool {
        let path_str = path.to_string_lossy();

        if EXCLUDE_DIRS.iter().any(|d| path_str.contains(&format!("/{}/", d))) {
            return false;
        }
        if exclusions.iter().any(|d| path_str.contains(&format!("/{}/", d))) {
            return false;
        }

        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        let is_code = languages::adapter_for_ext(&ext).is_some();
        let is_doc = ext == ".md" || ext == ".mdx";
        is_code || is_doc
    }

    pub(crate) fn is_branch_switch(path: &PathBuf) -> bool {
        let s = path.to_string_lossy();
        s.ends_with(".git/HEAD") || s.ends_with(".git\\HEAD")
    }

    pub(crate) async fn process_batch(
        changes: HashMap<PathBuf, ChangeKind>,
        queue: &TaskQueue,
        projects: &[(String, String)],
    ) {
        let mut repo_changes: HashMap<String, Vec<(PathBuf, ChangeKind)>> = HashMap::new();
        for (path, kind) in changes {
            let path_str = path.to_string_lossy().to_string();
            if let Some((repo_id, _)) = projects.iter().find(|(_, rp)| path_str.starts_with(rp.as_str())) {
                repo_changes.entry(repo_id.clone()).or_default().push((path, kind));
            }
        }

        for (repo_id, changes) in repo_changes {
            let mut file_task_ids = Vec::new();

            let mut deleted_dirs: HashSet<PathBuf> = HashSet::new();
            for (path, kind) in &changes {
                if *kind == ChangeKind::Delete {
                    if let Some(parent) = path.parent() {
                        if !parent.exists() && !deleted_dirs.contains(parent) {
                            deleted_dirs.insert(parent.to_path_buf());
                        }
                    }
                }
            }

            for dir in &deleted_dirs {
                queue.enqueue(Task::new(TaskKind::DeleteFolder, &repo_id, &dir.to_string_lossy())).await;
            }

            for (path, kind) in &changes {
                if let Some(parent) = path.parent() {
                    if deleted_dirs.contains(parent) { continue; }
                }

                let abs_path = path.to_string_lossy().to_string();
                let repo_path = projects.iter().find(|(rid, _)| *rid == repo_id).map(|(_, p)| p.as_str()).unwrap_or("");
                match kind {
                    ChangeKind::Delete => {
                        let id = queue.enqueue(Task::new(TaskKind::DeleteFile, &repo_id, &abs_path)).await;
                        file_task_ids.push(id);
                    }
                    ChangeKind::Create | ChangeKind::Modify => {
                        let rel_dir = path.parent()
                            .and_then(|p| p.strip_prefix(repo_path).ok())
                            .map(|p| p.to_string_lossy().replace('\\', "/"))
                            .unwrap_or_default();
                        let mod_name = if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir };
                        let mod_id = format!("mod:{}:{}", repo_id, mod_name);

                        let task = Task::new(TaskKind::ProcessFile, &repo_id, &abs_path)
                            .with_module(&mod_id);
                        let id = queue.enqueue(task).await;
                        file_task_ids.push(id);
                    }
                }
            }

            if !file_task_ids.is_empty() {
                queue.enqueue(
                    Task::new(TaskKind::ResolveEdges, &repo_id, "")
                        .blocked_by(file_task_ids)
                ).await;
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Read current branch from .git/HEAD file.
fn read_git_head(head_path: &str) -> Option<String> {
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
        watcher.register(
            PathBuf::from("/tmp/project"),
            vec!["node_modules".into(), "dist".into()],
        );
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
            RootWatcher::classify_event(&EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Content))),
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
        assert!(!RootWatcher::should_watch_path(&PathBuf::from("/project/node_modules/foo/index.js"), &[]));
    }

    #[test]
    fn should_not_watch_custom_exclusion() {
        assert!(!RootWatcher::should_watch_path(&PathBuf::from("/project/vendor/lib.rs"), &["vendor".into()]));
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

    #[tokio::test]
    async fn process_batch_enqueues_file_tasks() {
        let queue = Arc::new(TaskQueue::new());
        let projects = vec![("repo-a".to_string(), "/projects/repo-a".to_string())];
        let mut changes = HashMap::new();
        changes.insert(PathBuf::from("/projects/repo-a/src/lib.rs"), ChangeKind::Modify);
        RootWatcher::process_batch(changes, &queue, &projects).await;
        let status = queue.status().await;
        assert_eq!(status.pending + status.blocked, 2);
    }

    #[tokio::test]
    async fn process_batch_delete_enqueues_delete_task() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().to_string_lossy().to_string();
        let projects = vec![("repo".to_string(), repo_path.clone())];
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let file = src.join("old.rs");
        std::fs::write(&file, "fn main() {}").unwrap();
        std::fs::remove_file(&file).unwrap();
        let mut changes = HashMap::new();
        changes.insert(file, ChangeKind::Delete);
        let queue = Arc::new(TaskQueue::new());
        RootWatcher::process_batch(changes, &queue, &projects).await;
        let status = queue.status().await;
        assert_eq!(status.pending + status.blocked, 2);
    }

    #[tokio::test]
    async fn process_batch_ignores_files_outside_projects() {
        let queue = Arc::new(TaskQueue::new());
        let projects = vec![("repo".to_string(), "/projects/repo".to_string())];
        let mut changes = HashMap::new();
        changes.insert(PathBuf::from("/other/file.rs"), ChangeKind::Modify);
        RootWatcher::process_batch(changes, &queue, &projects).await;
        let status = queue.status().await;
        assert_eq!(status.pending, 0);
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
