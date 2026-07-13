//! Scan phase: discover folders, classify, enqueue ProcessGitFolder.
//! Only emits activity events. Project + folder events come from ProcessGitFolder.

use std::path::Path;
use std::time::Instant;
use super::super::executor::TaskContext;
use super::super::{Task, TaskKind};
use super::scan_logic::{self, FolderKind};
use crate::api::events::*;

// ── Scan Root ──────────────────────────────────────────────────────────────

pub async fn scan_root(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let root = Path::new(&task.path);
    if !root.exists() {
        return Err(format!("Root path does not exist: {}", task.path));
    }

    let start = Instant::now();
    let emit = |evt: StateEvent| { let _ = ctx.app_state.event_tx.send(evt); };

    // 1. Find all git folders
    let git_folders = scan_logic::find_git_folders(root, 3);

    // Emit discover activity per git folder
    for gf in &git_folders {
        emit(StateEvent::activity(ActivityEvent::new(
            ActivityLevel::Discover,
            &format!("{} · git folder", gf.display()),
            start.elapsed().as_secs_f64(),
        )));
    }

    // 2. Classify into project roots: git repos + quasi-repos (non-git project
    //    roots that contain indexable code). Subfolders are never promoted.
    let all_dirs = scan_logic::all_directories(root, 3);
    let classified = scan_logic::classify_folders(
        root, &git_folders, &all_dirs, scan_logic::has_indexable_code,
    );

    // Emit discover activity for quasi-repos (git folders were emitted above).
    for f in &classified {
        if f.kind == FolderKind::Standalone {
            emit(StateEvent::activity(ActivityEvent::new(
                ActivityLevel::Discover,
                &format!("{} · standalone folder", f.path.display()),
                start.elapsed().as_secs_f64(),
            )));
        }
    }

    // 3. Register watch root in DB
    let root_name = root.file_name().and_then(|n| n.to_str()).unwrap_or("root");
    let root_id = ctx.pg().add_watch_root(&task.path, root_name, &serde_json::json!([])).await
        .map_err(|e| format!("Failed to register watch root: {}", e))?;

    // 4. Register each project root with its kind and enqueue processing.
    //    ProcessGitFolder indexes any directory (a `.git` is not required), so a
    //    quasi-repo is indexed exactly like a real repo.
    for f in &classified {
        let kind = match f.kind {
            FolderKind::Git => "git",
            FolderKind::Standalone => "standalone",
        };
        let path_str = f.path.to_string_lossy();
        match ctx.pg().upsert_repo_kind(&root_id, kind, &f.name, &path_str).await {
            Ok(fid) => {
                // A quasi-repo with no manifest (loose source / docs) is a likely-but-
                // unconfirmed project — flag it `needs-review` so it surfaces for the
                // user to keep / organise / discard. Manifest-backed roots and real git
                // repos are confident; clear any stale flag on them.
                let needs_review = f.kind == FolderKind::Standalone
                    && matches!(scan_logic::classify_quasi_repo(&f.path), Some(scan_logic::QuasiKind::LooseCode));
                if needs_review {
                    if let Err(e) = ctx.pg().tag_folder(&fid, "needs-review").await {
                        tracing::warn!(error = %e, folder_id = %fid, "scan_root: tag_folder needs-review failed");
                    }
                } else if let Err(e) = ctx.pg().untag_folder(&fid, "needs-review").await {
                    tracing::warn!(error = %e, folder_id = %fid, "scan_root: untag_folder needs-review failed");
                }
            }
            Err(e) => tracing::warn!(error = %e, path = %path_str, "scan_root: upsert_repo_kind failed"),
        }
        let process_task = Task::new(TaskKind::ProcessGitFolder, &path_str, &path_str)
            .with_parent(task.id);
        ctx.queue.enqueue(process_task).await;
    }

    // 4.5 Reconcile: self-heal the index the scan can't fix additively.
    //     First re-absorb any `standalone` root mis-scoped inside a git repo
    //     (Bug 3) — it becomes a `kind=folder` of the repo's project, so a
    //     later `reconcile_roots` no longer sees it as a stale root. Then prune
    //     roots the scan no longer discovers (a root that lost its `.git`, was
    //     emptied, or moved lingers forever as a phantom project otherwise).
    let reabsorbed = ctx.pg().heal_nested_standalone_roots().await
        .unwrap_or_else(|e| { tracing::warn!(error = %e, "scan_root: heal_nested_standalone_roots failed"); 0 });
    let live: std::collections::HashSet<std::path::PathBuf> =
        classified.iter().map(|f| f.path.clone()).collect();
    let (removed, marked) = reconcile_roots(ctx.pg(), &root_id, &live).await;
    let orphaned = ctx.pg().mark_orphaned_projects().await.unwrap_or_else(|e| { tracing::warn!(error = %e, "scan_root: mark_orphaned_projects failed"); 0 });
    if reabsorbed > 0 {
        tracing::info!("scan_root reconcile: re-absorbed {reabsorbed} nested standalone root(s) into their enclosing repo's project");
    }
    if removed > 0 || marked > 0 {
        emit(StateEvent::activity(ActivityEvent::new(
            ActivityLevel::Info,
            &format!("reconcile · {removed} stale roots removed · {marked} flagged stale · {orphaned} projects re-tagged"),
            start.elapsed().as_secs_f64(),
        )));
        tracing::info!("scan_root reconcile: removed={removed} marked={marked} orphaned_retagged={orphaned}");
    }

    // 5. Register watcher
    {
        let watcher = crate::watcher::root_watcher::RootWatcher::instance(ctx.queue.clone());
        match watcher.lock() {
            Ok(mut w) => w.register(std::path::PathBuf::from(&task.path), vec![]),
            Err(e) => tracing::warn!(error = %e, path = %task.path, "scan_root: RootWatcher lock poisoned, watch root not registered"),
        }
    }

    // 6. Summary activity
    let git_count = classified.iter().filter(|f| f.kind == FolderKind::Git).count();
    let quasi_count = classified.iter().filter(|f| f.kind == FolderKind::Standalone).count();

    emit(StateEvent::activity(ActivityEvent::new(
        ActivityLevel::Info,
        &format!("{} git · {} standalone project roots discovered", git_count, quasi_count),
        start.elapsed().as_secs_f64(),
    )));

    tracing::info!("scan_root: {} git, {} standalone project roots in {}",
        git_count, quasi_count, task.path);
    Ok((git_count + quasi_count) as u32)
}

/// Prune project roots the scan no longer discovers, healing the index after a
/// repo loses its `.git`, is emptied, or is moved. Diffs the DB's recorded roots
/// (`kind` git/standalone/subtree) under this watch root against the freshly
/// discovered `live` set and applies [`scan_logic::classify_stale_root`]:
/// provably-dead roots are deleted (cascading nodes + subtree); ambiguous ones
/// (real content, no live owner) are tagged `stale` for the user to triage.
/// Returns `(removed, marked)`.
async fn reconcile_roots(
    pg: &crate::db::pg_store::PgStore,
    root_id: &uuid::Uuid,
    live: &std::collections::HashSet<std::path::PathBuf>,
) -> (u32, u32) {
    use scan_logic::StaleAction;
    let recorded = pg.list_folders_by_root(root_id).await.unwrap_or_else(|e| { tracing::warn!(error = %e, root_id = %root_id, "reconcile_roots: list_folders_by_root failed, skipping reconcile"); Vec::new() });
    let (mut removed, mut marked) = (0u32, 0u32);
    for r in &recorded {
        // Only project roots are reconciled here; kind=folder rows are owned by
        // their root and re-materialised when it is processed.
        if !matches!(r["kind"].as_str().unwrap_or(""), "git" | "standalone" | "subtree") {
            continue;
        }
        let abs = r["abs_path"].as_str().unwrap_or("");
        if abs.is_empty() {
            continue;
        }
        let p = std::path::PathBuf::from(abs);
        if live.contains(&p) {
            continue; // re-discovered this scan
        }
        let Some(id) = crate::api::util::json_uuid(&r["id"]) else { continue };
        let exists = p.exists();
        let has_content = exists && scan_logic::dir_has_indexable_content(&p);
        match scan_logic::classify_stale_root(&p, live, exists, has_content) {
            StaleAction::Keep => {}
            StaleAction::Remove => {
                match pg.delete_folder_tree(&id).await {
                    Ok(_) => { removed += 1; tracing::info!("reconcile: removed stale root {abs}"); }
                    Err(e) => tracing::warn!(error = %e, path = %abs, "reconcile: delete_folder_tree failed"),
                }
            }
            StaleAction::MarkStale => {
                match pg.tag_folder(&id, "stale").await {
                    Ok(_) => { marked += 1; tracing::info!("reconcile: flagged stale root {abs}"); }
                    Err(e) => tracing::warn!(error = %e, path = %abs, "reconcile: tag_folder stale failed"),
                }
            }
        }
    }
    (removed, marked)
}

/// Prune indexed nodes whose file no longer exists on disk (Bug 2 safety net).
/// Compares the folder's indexed file paths (`sensei.nodes`, module nodes
/// excluded) against `live_paths` — the repo-relative paths present on disk now
/// — and drops nodes for any indexed path not in the live set. This catches
/// orphans the incremental `scan_state` diff and the fs-watcher missed (e.g. a
/// moved sub-crate whose files vanished but whose struct nodes lingered). For
/// each vanished file it un-resolves inbound edges (preserving `target_name` for
/// re-resolution), deletes the nodes (cascading their edges) and clears the
/// scan-state row. Non-fatal — every DB error is logged and skipped. Returns the
/// number of files pruned.
pub async fn prune_vanished(
    pg: &crate::db::pg_store::PgStore,
    folder_id: &uuid::Uuid,
    live_paths: &std::collections::HashSet<String>,
) -> u64 {
    let indexed = pg.list_indexed_files(folder_id).await.unwrap_or_else(|e| {
        tracing::warn!(folder_id = %folder_id, error = %e, "prune_vanished: list_indexed_files failed");
        Vec::new()
    });
    let mut pruned = 0u64;
    for path in indexed {
        if live_paths.contains(&path) {
            continue;
        }
        if let Err(e) = pg.unresolve_edges_to_file(folder_id, &path).await {
            tracing::warn!(folder_id = %folder_id, file = %path, error = %e, "prune_vanished: unresolve_edges_to_file failed");
        }
        if let Err(e) = pg.delete_nodes_by_file(folder_id, &path).await {
            tracing::warn!(folder_id = %folder_id, file = %path, error = %e, "prune_vanished: delete_nodes_by_file failed");
            continue;
        }
        if let Err(e) = pg.delete_scan_state_file(folder_id, &path).await {
            tracing::warn!(folder_id = %folder_id, file = %path, error = %e, "prune_vanished: delete_scan_state_file failed");
        }
        pruned += 1;
    }
    pruned
}

// ── Branch Switch ─────────────────────────────────────────────────────────

pub async fn branch_switch(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let new_branch = task.branch.as_deref().ok_or("branch_switch requires branch field")?;

    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await
        .unwrap_or_else(|e| { tracing::warn!(error = %e, path = %task.folder_path, "branch_switch: get_repo_by_path failed"); None });
    let folder_name = folder.as_ref()
        .and_then(|f| f["name"].as_str())
        .unwrap_or_else(|| task.folder_name());

    // No wipe. A branch switch is just an incremental re-index: git rewrites
    // exactly the files that differ between the two branches (updating their
    // mtime), so process_git_folder's scan_state diff re-indexes only those —
    // unchanged files keep their nodes + embeddings, and files that exist on the
    // old branch but not the new one are dropped as "removed". process_git_folder
    // records the new branch (from the task) in props.branch.
    let git_task = Task::new(TaskKind::ProcessGitFolder, &task.folder_path, &task.path)
        .with_parent(task.id)
        .with_branch(new_branch);
    ctx.queue.enqueue(git_task).await;

    tracing::info!("branch_switch: {} → {} (incremental)", folder_name, new_branch);
    Ok(0)
}

fn _detect_current_branch(repo_path: &str) -> Option<String> {
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
    use crate::tasks::Task;
    use crate::api::state::SharedState;
    use super::super::super::executor::TaskContext;

    async fn make_ctx() -> Arc<TaskContext> {
        let queue = Arc::new(TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let (event_tx, _) = tokio::sync::broadcast::channel(256);
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
            gateway,
            event_tx,
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        });
        Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
            logger: sensei_logger::Logger::noop(),
        })
    }

    async fn make_ctx_with_events() -> (Arc<TaskContext>, tokio::sync::broadcast::Receiver<StateEvent>) {
        let queue = Arc::new(TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let (event_tx, event_rx) = tokio::sync::broadcast::channel(256);
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
            gateway,
            event_tx,
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        });
        let ctx = Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
            logger: sensei_logger::Logger::noop(),
        });
        (ctx, event_rx)
    }

    #[tokio::test]
    async fn scan_root_errors_on_nonexistent_path() {
        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ScanRoot, "", "/nonexistent/path/xyz");
        let result = scan_root(&ctx, &task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn scan_root_enqueues_only_git_folders() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("alpha/.git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("beta/.git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("notes")).unwrap(); // non-git

        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ScanRoot, "", &tmp.path().to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        let status = ctx.queue.status().await;
        assert_eq!(status.pending, 2, "only git folders should be enqueued");
    }

    #[tokio::test]
    async fn scan_reconciles_stale_roots() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // A live git repo.
        std::fs::create_dir_all(root.join("alive/.git")).unwrap();
        std::fs::write(root.join("alive/Cargo.toml"), "[package]\nname=\"alive\"").unwrap();
        // A git repo nested under a grouping dir; `group` is an ancestor-of-git,
        // so the scan never classifies it as a root — but it still has content.
        std::fs::create_dir_all(root.join("group/repo/.git")).unwrap();
        std::fs::write(root.join("group/repo/Cargo.toml"), "[package]\nname=\"repo\"").unwrap();
        // A former git repo that lost its `.git` but still holds code → the scan
        // re-classifies it as a quasi-repo (standalone), not a dead row.
        std::fs::create_dir_all(root.join("revived")).unwrap();
        std::fs::write(root.join("revived/Cargo.toml"), "[package]\nname=\"revived\"").unwrap();

        let ctx = make_ctx().await;
        let root_str = root.to_string_lossy().to_string();

        // Pre-register three stale roots as a prior scan would have:
        //  - `ghost`: kind=git whose path no longer exists on disk → must be removed
        //  - `group`: kind=git now an ancestor-of-git (no live owner) → must be flagged
        //  - `revived`: kind=git that lost `.git` but still has code → relabel standalone
        let root_id = ctx.pg().add_watch_root(&root_str, "root", &serde_json::json!([])).await.unwrap();
        let ghost = root.join("ghost"); // never created on disk
        ctx.pg().upsert_repo_kind(&root_id, "git", "ghost", &ghost.to_string_lossy()).await.unwrap();
        let group = root.join("group");
        ctx.pg().upsert_repo_kind(&root_id, "git", "group", &group.to_string_lossy()).await.unwrap();
        let revived = root.join("revived");
        ctx.pg().upsert_repo_kind(&root_id, "git", "revived", &revived.to_string_lossy()).await.unwrap();

        let task = Task::new(TaskKind::ScanRoot, "", &root_str);
        scan_root(&ctx, &task).await.unwrap();

        // ghost (path gone) → removed entirely
        assert!(
            ctx.pg().get_repo_by_path(&ghost.to_string_lossy()).await.unwrap().is_none(),
            "stale root with no path on disk should be removed"
        );

        // group (exists with content, but no live owner) → kept and tagged `stale`
        let group_row = ctx.pg().get_repo_by_path(&group.to_string_lossy()).await.unwrap()
            .expect("contentful stale root should be marked, not deleted");
        let tags: Vec<String> = group_row["tags"].as_array().unwrap_or(&vec![]).iter()
            .filter_map(|t| t.as_str().map(String::from)).collect();
        assert!(tags.contains(&"stale".to_string()), "group should be tagged stale, got {tags:?}");

        // alive remains a live git root, untouched
        assert!(
            ctx.pg().get_repo_by_path(&root.join("alive").to_string_lossy()).await.unwrap().is_some(),
            "live git repo should remain"
        );

        // revived (lost .git, still has code) → relabelled standalone, not stale/removed
        let revived_row = ctx.pg().get_repo_by_path(&revived.to_string_lossy()).await.unwrap()
            .expect("revived quasi-repo should remain");
        assert_eq!(revived_row["kind"], "standalone", "former git root with code should relabel standalone");
    }

    #[tokio::test]
    async fn upsert_repo_kind_relabels_git_standalone_but_preserves_subtree() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let root_id = ctx.pg()
            .add_watch_root(&tmp.path().to_string_lossy(), "r", &serde_json::json!([]))
            .await.unwrap();

        // git ⇄ standalone is authoritative on re-registration.
        let p1 = tmp.path().join("a").to_string_lossy().to_string();
        ctx.pg().upsert_repo_kind(&root_id, "git", "a", &p1).await.unwrap();
        ctx.pg().upsert_repo_kind(&root_id, "standalone", "a", &p1).await.unwrap();
        assert_eq!(ctx.pg().get_repo_by_path(&p1).await.unwrap().unwrap()["kind"], "standalone");
        ctx.pg().upsert_repo_kind(&root_id, "git", "a", &p1).await.unwrap();
        assert_eq!(ctx.pg().get_repo_by_path(&p1).await.unwrap().unwrap()["kind"], "git");

        // A subtree must NOT be clobbered by a root re-registration.
        let p2 = tmp.path().join("b").to_string_lossy().to_string();
        ctx.pg().upsert_folder(&root_id, "subtree", "b", "b", &p2, None, None).await.unwrap();
        ctx.pg().upsert_repo_kind(&root_id, "git", "b", &p2).await.unwrap();
        assert_eq!(ctx.pg().get_repo_by_path(&p2).await.unwrap().unwrap()["kind"], "subtree");
    }

    #[tokio::test]
    async fn scan_flags_loose_quasi_repos_and_skips_data_only() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // manifest-backed quasi-repo → confident, no flag
        std::fs::create_dir_all(root.join("manifest-proj")).unwrap();
        std::fs::write(root.join("manifest-proj/Cargo.toml"), "[package]\nname=\"m\"").unwrap();
        // loose source, no manifest → flagged needs-review
        std::fs::create_dir_all(root.join("loose-code")).unwrap();
        std::fs::write(root.join("loose-code/run.py"), "print('hi')\n").unwrap();
        // data only → not a project root at all
        std::fs::create_dir_all(root.join("data-only")).unwrap();
        std::fs::write(root.join("data-only/rows.csv"), "a,b\n1,2\n").unwrap();

        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ScanRoot, "", &root.to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        let tags_of = |row: &serde_json::Value| -> Vec<String> {
            row["tags"].as_array().map(|a| a.iter().filter_map(|t| t.as_str().map(String::from)).collect())
                .unwrap_or_default()
        };

        // manifest → standalone, NOT flagged
        let manifest = ctx.pg().get_repo_by_path(&root.join("manifest-proj").to_string_lossy())
            .await.unwrap().expect("manifest quasi-repo should be registered");
        assert_eq!(manifest["kind"], "standalone");
        assert!(!tags_of(&manifest).contains(&"needs-review".to_string()),
            "manifest-backed quasi-repo should not be flagged");

        // loose code → standalone, flagged needs-review
        let loose = ctx.pg().get_repo_by_path(&root.join("loose-code").to_string_lossy())
            .await.unwrap().expect("loose quasi-repo should be registered");
        assert_eq!(loose["kind"], "standalone");
        assert!(tags_of(&loose).contains(&"needs-review".to_string()),
            "loose-code quasi-repo should be flagged needs-review");

        // data only → not promoted
        assert!(
            ctx.pg().get_repo_by_path(&root.join("data-only").to_string_lossy()).await.unwrap().is_none(),
            "data-only folder should not be promoted to a project root"
        );
    }

    #[tokio::test]
    async fn prune_vanished_drops_orphan_nodes_and_keeps_live() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();
        let root_id = ctx.pg().add_watch_root(&repo_path, "pv", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "pv-repo", &repo_path).await.unwrap();

        // Two indexed files: a.rs (still on disk) and a moved-away b.rs (orphan),
        // plus a module node (abs dir path) that must never be pruned.
        ctx.pg().upsert_node(&fid, "file", "a.rs", "a.rs", None, None, None, None).await.unwrap();
        ctx.pg().upsert_node(&fid, "struct", "Gone", "crates/hive-mind/src/config.rs", None, None, None, None).await.unwrap();
        ctx.pg().upsert_node(&fid, "module", "src", &format!("{repo_path}/src"), None, None, None, None).await.unwrap();
        ctx.pg().upsert_scan_state(&fid, "crates/hive-mind/src/config.rs", 1, "h").await.unwrap();

        // Live working-tree set: only a.rs survives.
        let live: std::collections::HashSet<String> = ["a.rs".to_string()].into_iter().collect();
        let pruned = prune_vanished(ctx.pg(), &fid, &live).await;
        assert_eq!(pruned, 1, "the one vanished file's nodes should be pruned");

        let files = ctx.pg().list_indexed_files(&fid).await.unwrap();
        assert!(files.contains(&"a.rs".to_string()), "live file's nodes survive");
        assert!(!files.iter().any(|p| p.contains("hive-mind")), "vanished file's nodes are gone");
        // The vanished file's scan-state row was cleared too.
        let ss = ctx.pg().list_scan_state(&fid).await.unwrap();
        assert!(ss.iter().all(|(p, _)| !p.contains("hive-mind")), "scan_state for the vanished file cleared");
    }

    #[tokio::test]
    async fn scan_emits_only_activity_events() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("proj/a/.git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("proj/b/.git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("proj/notes")).unwrap(); // non-git, no code → not a quasi-repo
        // quasi-repo: top-level non-git dir with a manifest
        std::fs::create_dir_all(tmp.path().join("loose")).unwrap();
        std::fs::write(tmp.path().join("loose/go.mod"), "module loose").unwrap();

        let (ctx, mut rx) = make_ctx_with_events().await;
        let task = Task::new(TaskKind::ScanRoot, "", &tmp.path().to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        let mut events = vec![];
        while let Ok(evt) = rx.try_recv() { events.push(evt); }

        // ALL events must be activity — no project or folder events from ScanRoot
        for evt in &events {
            assert_eq!(evt.entity, "activity",
                "ScanRoot should only emit activity events, got entity={}", evt.entity);
        }

        // Discover events: 2 git + 1 quasi-repo = 3 (code-less `notes` is skipped)
        let discovers: Vec<_> = events.iter()
            .filter(|e| e.data["level"] == "discover")
            .collect();
        assert_eq!(discovers.len(), 3, "expected 3 discover events, got {}", discovers.len());

        // Info summary
        let infos: Vec<_> = events.iter()
            .filter(|e| e.data["level"] == "info")
            .collect();
        assert_eq!(infos.len(), 1);
        let msg = infos[0].data["message"].as_str().unwrap();
        assert!(msg.contains("2 git"), "summary: {}", msg);
        assert!(msg.contains("1 standalone"), "summary: {}", msg);
    }
}
