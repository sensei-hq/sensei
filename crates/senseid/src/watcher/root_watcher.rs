//! Root watcher — one watcher per scanned root directory.
//! Detects file changes and feeds tasks directly into the queue.
//! Replaces per-repo watchers + dirty tracker.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::collections::{HashMap, HashSet};
use notify::{Watcher, RecursiveMode, Event, EventKind};
use crate::tasks::{Task, TaskKind};
use crate::tasks::queue::TaskQueue;
use crate::adapters;

const DEBOUNCE_MS: u64 = 500;

const EXCLUDE_DIRS: &[&str] = &[
    "node_modules", "dist", "build", "target", ".git",
    ".next", ".svelte-kit", "__pycache__", ".venv", "venv",
];

/// Start a watcher on a root directory. Returns a handle to stop it.
/// File changes are translated into tasks and enqueued directly.
pub fn start_root_watcher(
    root: PathBuf,
    queue: Arc<TaskQueue>,
    projects: HashMap<String, String>, // repo_id → repo_path (for identifying which repo a file belongs to)
) -> Result<(), String> {
    let root_clone = root.clone();

    // Capture tokio handle BEFORE spawning the OS thread
    let rt = tokio::runtime::Handle::try_current()
        .map_err(|_| "Root watcher requires tokio runtime".to_string())?;

    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        }).expect("failed to create watcher");

        watcher.watch(&root_clone, RecursiveMode::Recursive)
            .expect("failed to watch root directory");

        tracing::info!("Root watcher started: {}", root_clone.display());

        let mut pending: HashMap<PathBuf, ChangeKind> = HashMap::new();
        let mut last_event = std::time::Instant::now();

        // Sort projects by path length (longest first) for best prefix match
        let mut sorted_projects: Vec<(String, String)> = projects.into_iter().collect();
        sorted_projects.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        loop {
            match rx.recv_timeout(Duration::from_millis(DEBOUNCE_MS)) {
                Ok(event) => {
                    let change_kind = match event.kind {
                        EventKind::Remove(_) => ChangeKind::Delete,
                        EventKind::Create(_) => ChangeKind::Create,
                        _ => ChangeKind::Modify,
                    };

                    for path in event.paths {
                        let path_str = path.to_string_lossy().to_string();

                        // Detect branch switch: .git/HEAD changed
                        if path_str.ends_with(".git/HEAD") || path_str.ends_with(".git\\HEAD") {
                            if let Some((repo_id, _repo_path)) = sorted_projects.iter()
                                .find(|(_, rp)| path_str.starts_with(rp.as_str()))
                            {
                                let new_branch = read_git_head(&path_str);
                                if let Some(branch) = new_branch {
                                    let queue_clone = queue.clone();
                                    let rid = repo_id.clone();
                                    let br = branch.clone();
                                    rt.spawn(async move {
                                        let task = crate::tasks::Task::new(
                                            crate::tasks::TaskKind::BranchSwitch, &rid, ""
                                        ).with_branch(&br);
                                        queue_clone.enqueue(task).await;
                                    });
                                    tracing::info!("Branch switch detected: {} → {}", repo_id, branch);
                                }
                            }
                            continue;
                        }

                        // Skip directories (we only care about files)
                        if change_kind != ChangeKind::Delete && !path.is_file() { continue; }

                        // Check extension
                        let ext = path.extension()
                            .and_then(|e| e.to_str())
                            .map(|e| format!(".{}", e))
                            .unwrap_or_default();
                        let is_code = adapters::adapter_for_ext(&ext).is_some();
                        let is_doc = ext == ".md" || ext == ".mdx";
                        if !is_code && !is_doc { continue; }

                        // Skip excluded directories
                        if EXCLUDE_DIRS.iter().any(|d| path_str.contains(&format!("/{}/", d))) {
                            continue;
                        }

                        pending.insert(path, change_kind);
                        last_event = std::time::Instant::now();
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if !pending.is_empty() && last_event.elapsed() >= Duration::from_millis(DEBOUNCE_MS) {
                        let batch: HashMap<PathBuf, ChangeKind> = pending.drain().collect();
                        let queue_clone = queue.clone();
                        let projects_ref = sorted_projects.clone();

                        rt.spawn(async move {
                            process_batch(batch, &queue_clone, &projects_ref).await;
                        });
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ChangeKind {
    Create,
    Modify,
    Delete,
}

/// Process a batch of file changes into tasks.
async fn process_batch(
    changes: HashMap<PathBuf, ChangeKind>,
    queue: &TaskQueue,
    projects: &[(String, String)],
) {
    // Group changes by repo
    let mut repo_changes: HashMap<String, Vec<(PathBuf, ChangeKind)>> = HashMap::new();

    for (path, kind) in changes {
        let path_str = path.to_string_lossy().to_string();
        // Find which repo this file belongs to
        if let Some((repo_id, _)) = projects.iter().find(|(_, rp)| path_str.starts_with(rp.as_str())) {
            repo_changes.entry(repo_id.clone()).or_default().push((path, kind));
        }
    }

    for (repo_id, changes) in repo_changes {
        let mut file_task_ids = Vec::new();

        // Check if any folder deletions
        let mut deleted_dirs: HashSet<PathBuf> = HashSet::new();
        for (path, kind) in &changes {
            if *kind == ChangeKind::Delete {
                if let Some(parent) = path.parent() {
                    // If the parent dir no longer exists, it's a folder deletion
                    if !parent.exists() && !deleted_dirs.contains(parent) {
                        deleted_dirs.insert(parent.to_path_buf());
                    }
                }
            }
        }

        // Enqueue folder deletions
        for dir in &deleted_dirs {
            queue.enqueue(Task::new(TaskKind::DeleteFolder, &repo_id, &dir.to_string_lossy())).await;
        }

        // Enqueue file-level tasks
        for (path, kind) in &changes {
            // Skip if parent dir was already deleted
            if let Some(parent) = path.parent() {
                if deleted_dirs.contains(parent) { continue; }
            }

            let abs_path = path.to_string_lossy().to_string();
            match kind {
                ChangeKind::Delete => {
                    let id = queue.enqueue(Task::new(TaskKind::DeleteFile, &repo_id, &abs_path)).await;
                    file_task_ids.push(id);
                }
                ChangeKind::Create | ChangeKind::Modify => {
                    // Determine module_id from directory
                    let repo_path = projects.iter().find(|(rid, _)| *rid == repo_id).map(|(_, p)| p.as_str()).unwrap_or("");
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

        // Enqueue resolve_edges barrier (depends on all file tasks in this batch)
        if !file_task_ids.is_empty() {
            queue.enqueue(
                Task::new(TaskKind::ResolveEdges, &repo_id, "")
                    .blocked_by(file_task_ids)
            ).await;
        }
    }
}

// start_all_watchers removed — watchers started via scan_root handler + server.rs spawn_root_watchers

/// Read current branch from .git/HEAD file.
/// Returns None if detached HEAD or unreadable.
fn read_git_head(head_path: &str) -> Option<String> {
    let content = std::fs::read_to_string(head_path).ok()?;
    let trimmed = content.trim();
    // Format: "ref: refs/heads/branch-name"
    if trimmed.starts_with("ref: refs/heads/") {
        Some(trimmed.strip_prefix("ref: refs/heads/")?.to_string())
    } else {
        None // detached HEAD (commit hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclude_dirs_filter() {
        let path = PathBuf::from("/repo/node_modules/foo/bar.js");
        let path_str = path.to_string_lossy();
        assert!(EXCLUDE_DIRS.iter().any(|d| path_str.contains(&format!("/{}/", d))));

        let path2 = PathBuf::from("/repo/src/main.ts");
        let path2_str = path2.to_string_lossy();
        assert!(!EXCLUDE_DIRS.iter().any(|d| path2_str.contains(&format!("/{}/", d))));
    }

    // ── EXCLUDE_DIRS coverage ──────────────────────────────────────────

    #[test]
    fn exclude_dirs_all_entries_matched() {
        // Every entry in EXCLUDE_DIRS should match when embedded in a path
        for dir in EXCLUDE_DIRS {
            let p = format!("/project/{}/some_file.rs", dir);
            assert!(
                EXCLUDE_DIRS.iter().any(|d| p.contains(&format!("/{}/", d))),
                "EXCLUDE_DIRS entry '{}' was not matched in path '{}'",
                dir,
                p,
            );
        }
    }

    #[test]
    fn exclude_dirs_no_false_positive_for_prefix() {
        // A directory name that merely starts with an excluded name should not match
        let path = PathBuf::from("/repo/node_modules_extra/foo.ts");
        let path_str = path.to_string_lossy();
        assert!(
            !EXCLUDE_DIRS.iter().any(|d| path_str.contains(&format!("/{}/", d))),
            "partial directory name should not be excluded",
        );
    }

    // ── ChangeKind ─────────────────────────────────────────────────────

    #[test]
    fn change_kind_equality() {
        assert_eq!(ChangeKind::Create, ChangeKind::Create);
        assert_eq!(ChangeKind::Modify, ChangeKind::Modify);
        assert_eq!(ChangeKind::Delete, ChangeKind::Delete);
        assert_ne!(ChangeKind::Create, ChangeKind::Delete);
        assert_ne!(ChangeKind::Create, ChangeKind::Modify);
        assert_ne!(ChangeKind::Delete, ChangeKind::Modify);
    }

    #[test]
    fn change_kind_clone() {
        let k = ChangeKind::Create;
        let k2 = k;
        assert_eq!(k, k2);
    }

    #[test]
    fn change_kind_debug() {
        let s = format!("{:?}", ChangeKind::Modify);
        assert_eq!(s, "Modify");
    }

    // ── read_git_head ──────────────────────────────────────────────────

    #[test]
    fn read_git_head_valid_branch() {
        let dir = tempfile::tempdir().unwrap();
        let head = dir.path().join("HEAD");
        std::fs::write(&head, "ref: refs/heads/main\n").unwrap();
        assert_eq!(
            read_git_head(head.to_str().unwrap()),
            Some("main".to_string()),
        );
    }

    #[test]
    fn read_git_head_feature_branch() {
        let dir = tempfile::tempdir().unwrap();
        let head = dir.path().join("HEAD");
        std::fs::write(&head, "ref: refs/heads/feature/foo-bar\n").unwrap();
        assert_eq!(
            read_git_head(head.to_str().unwrap()),
            Some("feature/foo-bar".to_string()),
        );
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

    #[test]
    fn read_git_head_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let head = dir.path().join("HEAD");
        std::fs::write(&head, "").unwrap();
        assert_eq!(read_git_head(head.to_str().unwrap()), None);
    }

    #[test]
    fn read_git_head_whitespace_only() {
        let dir = tempfile::tempdir().unwrap();
        let head = dir.path().join("HEAD");
        std::fs::write(&head, "   \n").unwrap();
        assert_eq!(read_git_head(head.to_str().unwrap()), None);
    }

    // ── process_batch ──────────────────────────────────────────────────

    #[tokio::test]
    async fn process_batch_groups_by_repo() {
        let queue = TaskQueue::new();
        let projects = vec![
            ("repo-a".to_string(), "/projects/repo-a".to_string()),
            ("repo-b".to_string(), "/projects/repo-b".to_string()),
        ];

        let mut changes = HashMap::new();
        changes.insert(
            PathBuf::from("/projects/repo-a/src/lib.rs"),
            ChangeKind::Modify,
        );
        changes.insert(
            PathBuf::from("/projects/repo-b/src/main.rs"),
            ChangeKind::Create,
        );

        process_batch(changes, &queue, &projects).await;

        let status = queue.status().await;
        // 2 file tasks (ProcessFile) + 2 ResolveEdges barriers
        assert_eq!(status.pending + status.blocked, 4);
    }

    #[tokio::test]
    async fn process_batch_delete_file() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().to_string_lossy().to_string();
        let projects = vec![("repo".to_string(), repo_path.clone())];

        // Create a file so parent dir exists (not a folder deletion)
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let file = src.join("old.rs");
        std::fs::write(&file, "fn main() {}").unwrap();
        // Now remove the file but keep dir
        std::fs::remove_file(&file).unwrap();

        let mut changes = HashMap::new();
        changes.insert(file, ChangeKind::Delete);

        let queue = TaskQueue::new();
        process_batch(changes, &queue, &projects).await;

        let status = queue.status().await;
        // 1 DeleteFile + 1 ResolveEdges
        assert_eq!(status.pending + status.blocked, 2);
    }

    #[tokio::test]
    async fn process_batch_folder_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().to_string_lossy().to_string();
        let projects = vec![("repo".to_string(), repo_path.clone())];

        // Parent directory does NOT exist — this triggers folder deletion logic
        let gone_dir = dir.path().join("gone_dir");
        let gone_file = gone_dir.join("removed.rs");

        let mut changes = HashMap::new();
        changes.insert(gone_file, ChangeKind::Delete);

        let queue = TaskQueue::new();
        process_batch(changes, &queue, &projects).await;

        let status = queue.status().await;
        // 1 DeleteFolder (the parent) — the file is skipped because parent was deleted
        // No file-level tasks → no ResolveEdges barrier
        assert_eq!(status.pending, 1);
        assert_eq!(status.blocked, 0);
    }

    #[tokio::test]
    async fn process_batch_module_id_from_subdir() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().to_string_lossy().to_string();
        let projects = vec![("repo".to_string(), repo_path.clone())];

        let src = dir.path().join("src").join("util");
        std::fs::create_dir_all(&src).unwrap();
        let file = src.join("helper.rs");
        std::fs::write(&file, "pub fn help() {}").unwrap();

        let mut changes = HashMap::new();
        changes.insert(file, ChangeKind::Create);

        let queue = TaskQueue::new();
        process_batch(changes, &queue, &projects).await;

        let status = queue.status().await;
        // 1 ProcessFile + 1 ResolveEdges
        assert_eq!(status.pending + status.blocked, 2);
    }

    #[tokio::test]
    async fn process_batch_root_level_file() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().to_string_lossy().to_string();
        let projects = vec![("repo".to_string(), repo_path.clone())];

        let file = dir.path().join("main.rs");
        std::fs::write(&file, "fn main() {}").unwrap();

        let mut changes = HashMap::new();
        changes.insert(file, ChangeKind::Modify);

        let queue = TaskQueue::new();
        process_batch(changes, &queue, &projects).await;

        let status = queue.status().await;
        // 1 ProcessFile + 1 ResolveEdges
        assert_eq!(status.pending + status.blocked, 2);
    }

    #[tokio::test]
    async fn process_batch_no_changes() {
        let queue = TaskQueue::new();
        let projects = vec![("repo".to_string(), "/projects/repo".to_string())];

        let changes = HashMap::new();
        process_batch(changes, &queue, &projects).await;

        let status = queue.status().await;
        assert_eq!(status.pending, 0);
        assert_eq!(status.blocked, 0);
    }

    #[tokio::test]
    async fn process_batch_file_outside_any_repo() {
        let queue = TaskQueue::new();
        let projects = vec![("repo".to_string(), "/projects/repo".to_string())];

        let mut changes = HashMap::new();
        changes.insert(
            PathBuf::from("/other/location/file.rs"),
            ChangeKind::Modify,
        );

        process_batch(changes, &queue, &projects).await;

        let status = queue.status().await;
        // File doesn't belong to any repo — nothing enqueued
        assert_eq!(status.pending, 0);
        assert_eq!(status.blocked, 0);
    }

    #[tokio::test]
    async fn process_batch_resolve_edges_blocked_by_file_tasks() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().to_string_lossy().to_string();
        let projects = vec![("repo".to_string(), repo_path.clone())];

        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let f1 = src.join("a.rs");
        let f2 = src.join("b.rs");
        std::fs::write(&f1, "").unwrap();
        std::fs::write(&f2, "").unwrap();

        let mut changes = HashMap::new();
        changes.insert(f1, ChangeKind::Create);
        changes.insert(f2, ChangeKind::Modify);

        let queue = TaskQueue::new();
        process_batch(changes, &queue, &projects).await;

        let status = queue.status().await;
        // 2 ProcessFile (pending) + 1 ResolveEdges (blocked)
        assert_eq!(status.pending, 2);
        assert_eq!(status.blocked, 1);
    }

    // ── DEBOUNCE_MS constant ───────────────────────────────────────────

    #[test]
    fn debounce_constant_is_reasonable() {
        assert!(DEBOUNCE_MS >= 100, "debounce should be at least 100ms");
        assert!(DEBOUNCE_MS <= 5000, "debounce should be at most 5s");
    }
}
