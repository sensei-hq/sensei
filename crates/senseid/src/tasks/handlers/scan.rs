//! Scan phase: discover git repos, register projects, handle branch switches.

use std::path::Path;
use super::super::executor::TaskContext;
use super::super::{Task, TaskKind};

// ── Scan Root ──────────────────────────────────────────────────────────────

/// Scan a root directory for git repos, register projects, enqueue process_repo tasks.
pub async fn scan_root(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let root = Path::new(&task.path);
    if !root.exists() {
        return Err(format!("Root path does not exist: {}", task.path));
    }

    let max_depth = 3u32;
    let mut repos = Vec::new();

    // If root itself is a git repo, register it directly
    if root.join(".git").is_dir() {
        let name = root.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        repos.push((name, root.to_string_lossy().to_string()));
    } else {
        find_git_repos(root, 0, max_depth, &mut repos);
    }

    // Register watch root in PG and get its UUID for folder FK
    let root_name = root.file_name().and_then(|n| n.to_str()).unwrap_or("root");
    let root_id = ctx.pg().add_watch_root(&task.path, root_name, &serde_json::json!([])).await
        .map_err(|e| format!("Failed to register watch root: {}", e))?;

    for (name, path) in &repos {
        // Register each discovered repo as a folder in PG
        ctx.pg().upsert_repo(&root_id, name, path).await.ok();

        // Enqueue process_repo
        let repo_task = Task::new(TaskKind::ProcessRepo, name, path)
            .with_parent(task.id);
        ctx.queue.enqueue(repo_task).await;
    }

    // Register this root with the watcher (caller is responsible for starting)
    {
        let watcher = crate::watcher::root_watcher::RootWatcher::instance(ctx.queue.clone());
        if let Ok(mut w) = watcher.lock() {
            w.register(std::path::PathBuf::from(&task.path), vec![]);
        }
    }

    // Suggest solution groupings for discovered repos (parent-folder + name-prefix)
    if repos.len() >= 2 {
        let suggestions = crate::tasks::processors::metadata::suggest_solutions(&repos);
        for suggestion in &suggestions {
            tracing::info!(
                "scan_root: solution suggestion '{}' ({}) — {} repos",
                suggestion.name, suggestion.strategy, suggestion.repo_ids.len()
            );
        }
        if !suggestions.is_empty() {
            ctx.pg().set_config("solution_suggestions", &serde_json::to_string(&suggestions).unwrap_or_default()).await.ok();
        }
    }

    tracing::info!("scan_root: {} repos found in {}", repos.len(), task.path);
    Ok(())
}

fn find_git_repos(dir: &Path, depth: u32, max_depth: u32, repos: &mut Vec<(String, String)>) {
    if depth > max_depth { return; }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if !path.is_dir() || name.starts_with('.') { continue; }
        if ["node_modules", "dist", "build", "target", ".git"].contains(&name.as_str()) { continue; }

        if path.join(".git").is_dir() {
            repos.push((name, path.to_string_lossy().to_string()));
        } else {
            find_git_repos(&path, depth + 1, max_depth, repos);
        }
    }
}

// ── Branch Switch ─────────────────────────────────────────────────────────

/// Handle a git branch switch: snapshot current graph, reindex for new branch.
pub async fn branch_switch(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let new_branch = task.branch.as_deref().ok_or("branch_switch requires branch field")?;

    // Detect current branch from git
    let repo_path = ctx.pg().get_repo_by_name(repo_id).await
        .map_err(|e| format!("PgStore error: {}", e))?
        .and_then(|r| r["abs_path"].as_str().map(String::from))
        .ok_or("Project not found")?;

    let old_branch = detect_git_branch(&repo_path).unwrap_or_else(|| "unknown".to_string());

    // TODO: branch snapshots not in PG yet — noop for clone/restore
    // Old code used graph.project_exists / clone_project_graph / delete_project_graph.
    // For now, always do a full reindex on branch switch.

    tracing::info!("branch_switch: {} → {} — full reindex (no branch snapshots in PG yet)", old_branch, new_branch);

    // Clear manifest for full re-parse
    let manifest_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".sensei").join("projects").join(repo_id).join("manifest.json");
    std::fs::remove_file(&manifest_path).ok();

    // 3. Enqueue full repo reindex + reconcile cross-repo links after
    let repo_task = Task::new(TaskKind::ProcessRepo, repo_id, &repo_path)
        .with_branch(new_branch);
    let repo_task_id = ctx.queue.enqueue(repo_task).await;

    // 4. Reconcile cross-repo connections after reindex completes
    ctx.queue.enqueue(
        Task::new(TaskKind::ReconcileConnections, repo_id, "")
            .blocked_by(vec![repo_task_id])
    ).await;

    tracing::info!("branch_switch: {} → {} — reindex + reconcile queued", old_branch, new_branch);
    Ok(())
}

/// Detect the current git branch from HEAD.
fn detect_git_branch(repo_path: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::tasks::queue::TaskQueue;
    use crate::tasks::{Task, TaskKind};
    use crate::api::state::SharedState;
    use super::super::super::executor::TaskContext;

    /// Build a TaskContext backed by PgStore and a fresh TaskQueue.
    async fn make_ctx() -> Arc<TaskContext> {
        let queue = Arc::new(TaskQueue::new());
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
        });
        Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
        })
    }

    // ── Pure function tests ──────────────────────────────────────────

    #[test]
    fn find_git_repos_discovers_repos_in_tempdir() {
        let tmp = tempfile::tempdir().unwrap();
        // Create two nested git repos
        let repo_a = tmp.path().join("repo-a");
        let repo_b = tmp.path().join("repo-b");
        std::fs::create_dir_all(repo_a.join(".git")).unwrap();
        std::fs::create_dir_all(repo_b.join(".git")).unwrap();

        let mut repos = Vec::new();
        find_git_repos(tmp.path(), 0, 3, &mut repos);

        let names: Vec<&str> = repos.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"repo-a"));
        assert!(names.contains(&"repo-b"));
        assert_eq!(repos.len(), 2);
    }

    #[test]
    fn find_git_repos_respects_max_depth() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a repo at depth=2 and one at depth=4
        let shallow = tmp.path().join("level1").join("shallow-repo");
        let deep = tmp.path().join("l1").join("l2").join("l3").join("deep-repo");
        std::fs::create_dir_all(shallow.join(".git")).unwrap();
        std::fs::create_dir_all(deep.join(".git")).unwrap();

        let mut repos = Vec::new();
        find_git_repos(tmp.path(), 0, 2, &mut repos);

        let names: Vec<&str> = repos.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"shallow-repo"));
        assert!(!names.contains(&"deep-repo"), "should not find repos beyond max_depth");
    }

    #[test]
    fn find_git_repos_skips_dotdirs_and_node_modules() {
        let tmp = tempfile::tempdir().unwrap();
        // Hidden dir with a repo inside
        let hidden = tmp.path().join(".hidden").join("repo");
        std::fs::create_dir_all(hidden.join(".git")).unwrap();
        // node_modules with a repo inside
        let nm = tmp.path().join("node_modules").join("pkg-repo");
        std::fs::create_dir_all(nm.join(".git")).unwrap();
        // Normal repo
        let normal = tmp.path().join("real-repo");
        std::fs::create_dir_all(normal.join(".git")).unwrap();

        let mut repos = Vec::new();
        find_git_repos(tmp.path(), 0, 3, &mut repos);

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].0, "real-repo");
    }

    // ── Handler integration tests (in-memory DB) ─────────────────────

    #[tokio::test]
    async fn scan_root_errors_on_nonexistent_path() {
        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ScanRoot, "", "/nonexistent/path/xyz");
        let result = scan_root(&ctx, &task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn scan_root_discovers_repos_and_enqueues() {
        let tmp = tempfile::tempdir().unwrap();
        // Create two repos under the scan root
        let repo_a = tmp.path().join("alpha");
        let repo_b = tmp.path().join("beta");
        std::fs::create_dir_all(repo_a.join(".git")).unwrap();
        std::fs::create_dir_all(repo_b.join(".git")).unwrap();
        // Add a file so directories aren't completely empty
        std::fs::write(repo_a.join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(repo_b.join("main.py"), "print('hi')").unwrap();

        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ScanRoot, "", &tmp.path().to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        // Two ProcessRepo tasks should have been enqueued
        let status = ctx.queue.status().await;
        assert_eq!(status.pending, 2, "expected 2 process_repo tasks enqueued");
    }

    #[tokio::test]
    async fn scan_root_registers_root_itself_when_it_is_a_git_repo() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::write(tmp.path().join("lib.rs"), "pub fn x() {}").unwrap();

        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ScanRoot, "", &tmp.path().to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        let status = ctx.queue.status().await;
        assert_eq!(status.pending, 1, "root repo itself should be enqueued");
    }
}
