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

    // Resolve the EFFECTIVE watch root. Watch roots stay top-level: if this scan
    // targets a path inside an existing watch root, index under that root instead
    // of registering a redundant sub-root (a change still resolves to its own
    // repo via repo_root_for_path). Only a path under no existing root becomes a
    // new watch root.
    let root_name = root.file_name().and_then(|n| n.to_str()).unwrap_or("root");
    let (root_id, watch_root_path) = match ctx.pg().enclosing_watch_root(&task.path).await
        .map_err(|e| format!("enclosing_watch_root: {e}"))?
    {
        Some((id, path)) => {
            if path != task.path {
                tracing::info!(scan = %task.path, watch_root = %path,
                    "scan is inside an existing watch root — reusing it, not registering a new root");
            }
            (id, path)
        }
        None => {
            let id = ctx.pg().add_watch_root(&task.path, root_name, &serde_json::json!([])).await
                .map_err(|e| format!("Failed to register watch root: {}", e))?;
            (id, task.path.clone())
        }
    };

    // Per-root exclusions (`folders_to_watch.excluded` on the effective root,
    // relative entries resolved to absolute `root/entry` prefixes). Filter the
    // discovered set so excluded subtrees are never classified as projects; the
    // watcher gets the same list so it ignores changes there.
    let exclusions = ctx.pg().root_exclusion_prefixes(&watch_root_path).await
        .map_err(|e| format!("read exclusions for {watch_root_path}: {e}"))?;

    // 1. Find all git folders
    let git_folders: Vec<_> = scan_logic::find_git_folders(root, 3)
        .into_iter()
        .filter(|p| !scan_logic::is_excluded(p, &exclusions))
        .collect();

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
    let all_dirs: Vec<_> = scan_logic::all_directories(root, 3)
        .into_iter()
        .filter(|p| !scan_logic::is_excluded(p, &exclusions))
        .collect();
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

    // 3. (watch root already resolved above as `root_id` — top-level only.)

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
                // Capture the git root's remotes so a future rename can be auto-detected
                // by shared remote (reconcile_roots → find_live_root_by_remote). Only
                // write when we actually read some, so a transient git failure never
                // clobbers a previously-captured set with an empty one.
                if f.kind == FolderKind::Git {
                    let remotes = read_git_remotes(&path_str);
                    if !remotes.is_empty()
                        && let Err(e) = ctx.pg().update_folder_remotes(&fid, &serde_json::Value::Array(remotes)).await {
                        tracing::warn!(error = %e, folder_id = %fid, "scan_root: update_folder_remotes failed");
                    }
                }
            }
            Err(e) => tracing::warn!(error = %e, path = %path_str, "scan_root: upsert_repo_kind failed"),
        }
        let process_task = Task::new(TaskKind::ProcessGitFolder, &path_str, &path_str)
            .with_parent(task.id);
        // Single-writer (D6e/W5): skip if this folder is already being scanned,
        // so a concurrent ScanRoot can't fan out a second ProcessGitFolder for it.
        let _ = ctx.queue.enqueue_unique(process_task).await;
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
    let ReconcileRootsOutcome { removed, marked, remapped, archived } = reconcile_roots(ctx.pg(), &root_id, &live).await;
    // Then prune ghost folder subtrees whose directory was deleted/moved on disk
    // (e.g. a renamed sub-crate). `reconcile_roots` only prunes project ROOTS and
    // `prune_vanished` only reconciles files *within* an indexed folder, so nothing
    // else removes a non-root `kind='folder'` row whose dir vanished — it lingers
    // dragging its whole subtree of nodes/edges/scan_state.
    let ghosts = prune_vanished_folders(ctx.pg(), &root_id).await;
    // Enforce one-node-one-owner: prune any code node a structural (folder-kind)
    // subfolder still holds a duplicate of under the project's canonical root
    // owner. Twin-guarded (never removes a uniquely-held symbol); self-heals the
    // pre-fix #101 double-index residue and blocks future accumulation.
    let deduped = ctx.pg().dedup_structural_folder_nodes(&root_id).await
        .unwrap_or_else(|e| { tracing::warn!(error = %e, "scan_root: dedup_structural_folder_nodes failed"); 0 });
    let orphaned = ctx.pg().mark_orphaned_projects().await.unwrap_or_else(|e| { tracing::warn!(error = %e, "scan_root: mark_orphaned_projects failed"); 0 });
    // Delete the provably-empty phantom projects (no folder / session / artifact)
    // — pre-#101 residue that would otherwise inflate the project count.
    let pruned_projects = ctx.pg().prune_empty_projects(60).await.unwrap_or_else(|e| { tracing::warn!(error = %e, "scan_root: prune_empty_projects failed"); 0 });
    // Tag file nodes with the framework kinds of the symbols they contain
    // (hook/component) so `get_patterns` / `get_file_tags` return real files.
    let tagged = ctx.pg().tag_file_nodes_by_framework_kind(&root_id).await
        .unwrap_or_else(|e| { tracing::warn!(error = %e, "scan_root: tag_file_nodes_by_framework_kind failed"); 0 });
    if tagged > 0 {
        tracing::info!("scan_root reconcile: framework-tagged {tagged} file node(s)");
    }
    if reabsorbed > 0 {
        tracing::info!("scan_root reconcile: re-absorbed {reabsorbed} nested standalone root(s) into their enclosing repo's project");
    }
    if removed > 0 || marked > 0 || remapped > 0 || archived > 0 || ghosts > 0 || deduped > 0 || pruned_projects > 0 {
        emit(StateEvent::activity(ActivityEvent::new(
            ActivityLevel::Info,
            &format!("reconcile · {removed} stale roots removed · {remapped} moved roots remapped · {archived} vanished roots archived · {ghosts} ghost folders pruned · {deduped} duplicate nodes deduped · {pruned_projects} empty projects purged · {marked} flagged stale · {orphaned} projects re-tagged"),
            start.elapsed().as_secs_f64(),
        )));
        tracing::info!("scan_root reconcile: removed={removed} remapped={remapped} archived={archived} ghost_folders={ghosts} deduped_nodes={deduped} empty_projects_purged={pruned_projects} marked={marked} orphaned_retagged={orphaned}");
    }

    // 5. Register watcher
    {
        let watcher = crate::watcher::root_watcher::RootWatcher::instance(ctx.queue.clone());
        match watcher.lock() {
            Ok(mut w) => w.register(std::path::PathBuf::from(&task.path), exclusions.clone()),
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
/// Tally of what [`reconcile_roots`] did this pass. `remapped` + `archived` are the
/// deletion-avoidance outcomes (a renamed root re-pointed / a history-bearing root
/// retained) that would previously have been silent hard-deletes.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ReconcileRootsOutcome {
    pub removed: u32,
    pub marked: u32,
    pub remapped: u32,
    pub archived: u32,
}

/// Extract the `url` strings from a folder's `remote_urls` JSON (`[{name,url}]`).
fn remote_urls_of(v: &serde_json::Value) -> Vec<String> {
    v.get("remote_urls")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|e| e.get("url").and_then(|u| u.as_str()).map(String::from)).collect())
        .unwrap_or_default()
}

async fn reconcile_roots(
    pg: &crate::db::pg_store::PgStore,
    root_id: &uuid::Uuid,
    live: &std::collections::HashSet<std::path::PathBuf>,
) -> ReconcileRootsOutcome {
    use scan_logic::{StaleAction, StaleDisposition};
    let recorded = pg.list_folders_by_root(root_id).await.unwrap_or_else(|e| { tracing::warn!(error = %e, root_id = %root_id, "reconcile_roots: list_folders_by_root failed, skipping reconcile"); Vec::new() });
    // Live root paths as strings — the candidate set a renamed root can remap onto.
    let live_abs: Vec<String> = live.iter().filter_map(|p| p.to_str().map(String::from)).collect();
    let mut out = ReconcileRootsOutcome::default();
    for r in &recorded {
        // Only project roots are reconciled here; kind=folder rows are owned by
        // their root and re-materialised when it is processed.
        if !matches!(r["kind"].as_str().unwrap_or(""), "git" | "standalone" | "subtree") {
            continue;
        }
        // An already-archived root is deliberately retained (its dir is gone but its
        // history is kept) — never re-delete or re-touch it.
        if r["status"].as_str() == Some("archived") {
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
        let base = scan_logic::classify_stale_root(&p, live, exists, has_content);

        // Only a Remove candidate (path gone) is eligible for remap/archive; resolve
        // the two impure signals lazily. On a lookup ERROR, fail safe — skip the row
        // (keep it this pass) rather than fall through to a delete on uncertainty.
        let (remote_match, has_history) = if base == StaleAction::Remove {
            let remotes = remote_urls_of(r);
            let remote_match = match pg.find_live_root_by_remote(&remotes, &live_abs).await {
                Ok(m) => m,
                Err(e) => { tracing::warn!(error = %e, path = %abs, "reconcile: find_live_root_by_remote failed, keeping root this pass"); continue; }
            };
            let has_history = if remote_match.is_none() {
                match pg.folder_has_sessions(&id).await {
                    Ok(h) => h,
                    Err(e) => { tracing::warn!(error = %e, path = %abs, "reconcile: folder_has_sessions failed, keeping root this pass"); continue; }
                }
            } else { false };
            (remote_match, has_history)
        } else {
            (None, false)
        };

        match scan_logic::decide_stale_root(base, remote_match, has_history) {
            StaleDisposition::Keep => {}
            StaleDisposition::Remap(to) => {
                match pg.remap_folder(&id, abs, &to).await {
                    Ok(_) => { out.remapped += 1; tracing::info!("reconcile: remapped moved root {abs} → {to} (git remote match)"); }
                    Err(e) => tracing::warn!(error = %e, path = %abs, "reconcile: remap_folder failed"),
                }
            }
            StaleDisposition::Archive => {
                match pg.archive_folder(&id).await {
                    Ok(_) => { out.archived += 1; tracing::info!("reconcile: archived vanished history-bearing root {abs}"); }
                    Err(e) => tracing::warn!(error = %e, path = %abs, "reconcile: archive_folder failed"),
                }
            }
            StaleDisposition::Remove => {
                match pg.delete_folder_tree(&id).await {
                    Ok(_) => { out.removed += 1; tracing::info!("reconcile: removed stale root {abs}"); }
                    Err(e) => tracing::warn!(error = %e, path = %abs, "reconcile: delete_folder_tree failed"),
                }
            }
            StaleDisposition::MarkStale => {
                match pg.tag_folder(&id, "stale").await {
                    Ok(_) => { out.marked += 1; tracing::info!("reconcile: flagged stale root {abs}"); }
                    Err(e) => tracing::warn!(error = %e, path = %abs, "reconcile: tag_folder stale failed"),
                }
            }
        }
    }
    out
}

/// Prune ghost folder subtrees the scan can never re-materialise: a non-root
/// `kind='folder'` row whose DIRECTORY was deleted or moved on disk (e.g.
/// `crates/hive-mind` renamed to `crates/dojo-mind`). The scanner never descends
/// into a directory that is gone, [`prune_vanished`] only reconciles FILE nodes
/// *within* one indexed folder, and [`reconcile_roots`] only prunes project
/// ROOTS — so nothing else ever removes such a row, and it lingers dragging its
/// whole subtree of nodes/edges/scan_state (137 orphan nodes in the live sensei
/// index). Each ghost folder row is deleted via [`crate::db::pg_store::PgStore::delete_folder_tree`],
/// cascading its nodes, edges, scan_state and descendant folder rows.
///
/// SAFETY: a subfolder is pruned only when its enclosing project ROOT (kind
/// git/standalone/subtree) is confirmed PRESENT on disk. A root whose own
/// directory is absent (unmounted drive, moved repo) is skipped entirely so a
/// temporarily-missing volume never nukes a live index — [`reconcile_roots`] owns
/// absent roots. A folder whose existence check *errors* (permission/symlink) is
/// treated as present and kept (never delete on uncertainty). Root folders are
/// never pruned here. Non-fatal — every DB error is logged and skipped.
/// Idempotent. Returns the number of ghost folder rows pruned.
///
/// Orchestration only: it delegates detection to [`detect_vanished_folders`] and
/// deletion to [`apply_folder_prune`], so the read-only index integrity audit can
/// reuse the exact same ghost-folder detection without mutating anything.
async fn prune_vanished_folders(
    pg: &crate::db::pg_store::PgStore,
    root_id: &uuid::Uuid,
) -> u64 {
    apply_folder_prune(pg, &detect_vanished_folders(pg, root_id).await).await
}

/// Detect (WITHOUT mutating) the ghost folder subtrees under one watch root: a
/// non-root `kind='folder'` row whose directory is *confirmed* gone on disk, and
/// which sits under a confirmed-PRESENT project root (the unmounted-volume guard —
/// see [`prune_vanished_folders`]). Returns `(folder_id, abs_path)` per ghost.
///
/// This is the detection half shared by [`prune_vanished_folders`] (which deletes
/// what this finds) and [`crate::tasks::index_audit`] (which reports it read-only
/// in `sensei index doctor`, and repairs via [`apply_folder_prune`]). Keeping one
/// detector means both paths agree on exactly what a "ghost folder" is.
pub(crate) async fn detect_vanished_folders(
    pg: &crate::db::pg_store::PgStore,
    root_id: &uuid::Uuid,
) -> Vec<(uuid::Uuid, String)> {
    let recorded = pg.list_folders_by_root(root_id).await.unwrap_or_else(|e| { tracing::warn!(error = %e, root_id = %root_id, "detect_vanished_folders: list_folders_by_root failed, skipping"); Vec::new() });

    // Present project roots — a subfolder is eligible only when the root it lives
    // under is confirmed on disk (guards against an unmounted volume / moved repo).
    let present_roots: Vec<String> = recorded.iter()
        .filter(|r| matches!(r["kind"].as_str().unwrap_or(""), "git" | "standalone" | "subtree"))
        .filter_map(|r| r["abs_path"].as_str().map(String::from))
        .filter(|abs| dir_present(std::path::Path::new(abs)))
        .collect();
    if present_roots.is_empty() {
        return Vec::new(); // no confirmed-present root under this watch root — nothing eligible
    }

    let mut ghosts = Vec::new();
    for r in &recorded {
        // Only structural subfolders (`folder` + the monorepo-member `workspace_member`,
        // D5a); project ROOTS (git/standalone/subtree) are owned by reconcile_roots.
        if !matches!(r["kind"].as_str(), Some("folder") | Some("workspace_member")) {
            continue;
        }
        let Some(abs) = r["abs_path"].as_str() else { continue };
        // Must sit under a confirmed-present project root (exact-prefix + '/',
        // mirroring heal_nested_standalone_roots' starts_with).
        if !present_roots.iter().any(|root| abs.starts_with(&format!("{root}/"))) {
            continue; // enclosing root absent/unknown → leave the subtree intact
        }
        // Keep unless the directory is *confirmed* gone (errored check → present).
        if dir_present(std::path::Path::new(abs)) {
            continue;
        }
        let Some(id) = crate::api::util::json_uuid(&r["id"]) else { continue };
        ghosts.push((id, abs.to_string()));
    }
    ghosts
}

/// Delete each detected ghost folder subtree via
/// [`crate::db::pg_store::PgStore::delete_folder_tree`], cascading its nodes,
/// edges, scan_state and descendant folder rows. Non-fatal — a failed delete is
/// logged and skipped. Idempotent. Returns the number pruned. The apply half of
/// [`detect_vanished_folders`], shared by the scan reconcile and the audit.
pub(crate) async fn apply_folder_prune(
    pg: &crate::db::pg_store::PgStore,
    ghosts: &[(uuid::Uuid, String)],
) -> u64 {
    let mut pruned = 0u64;
    for (id, abs) in ghosts {
        match pg.delete_folder_tree(id).await {
            Ok(()) => { pruned += 1; tracing::info!("prune_vanished_folders: pruned ghost folder subtree {abs} (dir gone)"); }
            Err(e) => tracing::warn!(error = %e, path = %abs, "prune_vanished_folders: delete_folder_tree failed"),
        }
    }
    pruned
}

/// True unless the path is *confirmed* absent (`try_exists() == Ok(false)`). An
/// errored check (permission, symlink loop) is treated as present so reconcile
/// never deletes a live index on uncertainty (fail-safe). Works for both files
/// and directories — the index audit reuses it to test a node's file existence.
pub(crate) fn dir_present(p: &std::path::Path) -> bool {
    !matches!(p.try_exists(), Ok(false))
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
    // NOTE: branch_switch deliberately uses plain `enqueue`, NOT the single-writer
    // `enqueue_unique`. The dedup identity (kind, folder_path, path) omits the
    // branch, so guarding here could silently drop a branch switch that races an
    // in-flight plain scan — leaving props.branch stale. Proper single-writer +
    // branch handling for branch_switch is a tracked follow-up (docs/backlog.md).
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

/// Parse `git config --get-regexp '^remote\..*\.url$'` stdout into `[{name,url}]`.
/// Pure over the captured text so it's unit-testable without a git subprocess.
/// Each line is `remote.<name>.url <url>`; blank urls and unparseable lines drop.
fn parse_git_remote_config(stdout: &str) -> Vec<serde_json::Value> {
    stdout
        .lines()
        .filter_map(|line| {
            let (key, url) = line.split_once(char::is_whitespace)?;
            let name = key.strip_prefix("remote.")?.strip_suffix(".url")?;
            let url = url.trim();
            if name.is_empty() || url.is_empty() {
                return None;
            }
            Some(serde_json::json!({ "name": name, "url": url }))
        })
        .collect()
}

/// Best-effort read of a git repo's configured remotes as `[{name,url}]` for
/// `folders.remote_urls` — the producing half of git-remote rename detection.
/// Shells out to `git config` (the codebase carries no git2 dep). Empty on any
/// failure or a repo with no remote: an honest "no remote", never a fabricated
/// one — a remote-less repo simply can't be rename-detected by remote.
fn read_git_remotes(repo_path: &str) -> Vec<serde_json::Value> {
    let Ok(output) = std::process::Command::new("git")
        .args(["config", "--get-regexp", r"^remote\..*\.url$"])
        .current_dir(repo_path)
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_git_remote_config(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::tasks::queue::TaskQueue;
    use crate::tasks::Task;
    use crate::api::state::SharedState;
    use super::super::super::executor::TaskContext;

    #[test]
    fn parse_git_remote_config_extracts_name_url_pairs() {
        let out = "remote.origin.url git@github.com:sensei-hq/sensei.git\n\
                   remote.upstream.url https://github.com/acme/sensei.git\n";
        let got = parse_git_remote_config(out);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0]["name"], "origin");
        assert_eq!(got[0]["url"], "git@github.com:sensei-hq/sensei.git");
        assert_eq!(got[1]["name"], "upstream");
        assert_eq!(got[1]["url"], "https://github.com/acme/sensei.git");
        // Empty input (no remotes) → no pairs, never a fabricated entry.
        assert!(parse_git_remote_config("").is_empty());
        // A malformed line (no url) is dropped, not half-parsed.
        assert!(parse_git_remote_config("remote.origin.url \n").is_empty());
    }

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
            provisioning: None,
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
            provisioning: None,
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
    async fn dedup_structural_folder_nodes_removes_root_twins_keeps_uniques() {
        // #101 self-heal: a structural (folder-kind) subfolder must not carry a
        // code node the project's ROOT owner already holds. Twin-guarded: prune
        // only a duplicate the root owns (name+kind+path-suffix), NEVER a symbol
        // held uniquely under the subfolder. This is the reconcile that cleans
        // the pre-fix double-index residue without any risk of data loss.
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let root_str = root.to_string_lossy().to_string();

        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let root_id = ctx.pg().add_watch_root(&root_str, "dedup", &serde_json::json!([])).await.unwrap();
        // Both the repo root and its member share ONE project (as in prod, where
        // ProcessGitFolder attributes them) — the twin-guard scopes by project.
        let pid = ctx.pg().create_project("dedup-proj", None, None).await.unwrap();
        let repo_fid = ctx.pg().upsert_repo_kind(&root_id, "git", "repo", &repo.to_string_lossy()).await.unwrap();
        ctx.pg().set_folder_project(&repo_fid, &pid, "root", None).await.unwrap();

        // The canonical git-root copies, stored repo-relative: a code symbol AND
        // a `file` node (whose `name` IS the repo-relative path).
        let root_twin = ctx.pg().upsert_node(&repo_fid, "function", "run_task", "crates/member/src/lib.rs", None, None, None, None).await.unwrap();
        let root_file = ctx.pg().upsert_node(&repo_fid, "file", "crates/member/src/lib.rs", "crates/member/src/lib.rs", None, None, None, None).await.unwrap();

        // A structural member subfolder carrying residue (subfolder-relative paths).
        let member = repo.join("crates/member");
        std::fs::create_dir_all(&member).unwrap();
        let member_fid = ctx.pg().upsert_subfolder(&root_id, "member", "crates/member", &member.to_string_lossy(), Some(&repo_fid), Some(&pid)).await.unwrap();
        // A duplicate of the git-root symbol → pruned (name equal, path-suffix twin).
        let dup = ctx.pg().upsert_node(&member_fid, "function", "run_task", "src/lib.rs", None, None, None, None).await.unwrap();
        // A duplicate `file` node → pruned via NAME path-suffix ("src/lib.rs" ⊂
        // "crates/member/src/lib.rs"), the case a name-EQUAL rule would miss.
        let dup_file = ctx.pg().upsert_node(&member_fid, "file", "src/lib.rs", "src/lib.rs", None, None, None, None).await.unwrap();
        // A symbol the root does NOT hold → KEPT (never lose a unique).
        let unique = ctx.pg().upsert_node(&member_fid, "function", "orphan_only", "src/gone.rs", None, None, None, None).await.unwrap();
        // A `file` node the root does NOT hold → KEPT (guard protects unique files too).
        let unique_file = ctx.pg().upsert_node(&member_fid, "file", "src/gone.rs", "src/gone.rs", None, None, None, None).await.unwrap();

        let pruned = ctx.pg().dedup_structural_folder_nodes(&root_id).await.unwrap();
        assert_eq!(pruned, 2, "the two root-twin duplicates (symbol + file) are pruned");

        for (id, msg) in [(dup, "symbol duplicate pruned"), (dup_file, "file-node duplicate pruned")] {
            let (alive,): (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.nodes WHERE id=$1")
                .bind(id).fetch_one(ctx.pg().pool()).await.unwrap();
            assert_eq!(alive, 0, "{msg}");
        }
        for (id, msg) in [(unique, "unique symbol preserved"), (unique_file, "unique file preserved"),
                          (root_twin, "canonical git-root symbol untouched"), (root_file, "canonical git-root file untouched")] {
            let (alive,): (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.nodes WHERE id=$1")
                .bind(id).fetch_one(ctx.pg().pool()).await.unwrap();
            assert_eq!(alive, 1, "{msg}");
        }
    }

    #[tokio::test]
    async fn list_communities_scoped_aggregates_across_folders() {
        // #G5a regression: `get_communities` must aggregate across ALL scope
        // folders, not one lowest-UUID leaf. Communities live per-folder (the repo
        // root usually owns them), so a single-folder lookup missed them.
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let root_id = ctx.pg().add_watch_root(&root.to_string_lossy(), "comm", &serde_json::json!([])).await.unwrap();
        let repo_fid = ctx.pg().upsert_repo_kind(&root_id, "git", "repo", &root.join("repo").to_string_lossy()).await.unwrap();
        let sub_fid = ctx.pg().upsert_subfolder(&root_id, "sub", "repo/sub", &root.join("repo/sub").to_string_lossy(), Some(&repo_fid), None).await.unwrap();
        ctx.pg().upsert_community(&repo_fid, 1, "auth cluster", 42).await.unwrap();
        ctx.pg().upsert_community(&sub_fid, 2, "util cluster", 7).await.unwrap();

        // Aggregated across both folders → both communities.
        let all = ctx.pg().list_communities_scoped(&[repo_fid, sub_fid]).await.unwrap();
        assert_eq!(all.len(), 2, "aggregates communities from every scope folder");
        // The old single-folder path only ever saw one folder — the bug.
        let one = ctx.pg().list_communities(&sub_fid).await.unwrap();
        assert_eq!(one.len(), 1, "single-folder lookup sees only its own (the pre-fix behaviour)");
    }

    #[tokio::test]
    async fn prune_empty_projects_deletes_phantoms_keeps_populated() {
        // Phantom project-count fix: a `discovery` project with no folder / session
        // / artifact is deleted; a project with a folder + nodes is kept.
        let ctx = make_ctx().await;
        let phantom = ctx.pg().create_project("phantom-crate", None, None).await.unwrap();
        // Age it past the mid-population grace window so the prune can see it.
        sqlx_core::query::query("UPDATE sensei.projects SET modified_at = now() - interval '2 minutes' WHERE id=$1")
            .bind(phantom).execute(ctx.pg().pool()).await.unwrap();

        let tmp = tempfile::tempdir().unwrap();
        let root_id = ctx.pg().add_watch_root(&tmp.path().to_string_lossy(), "pp", &serde_json::json!([])).await.unwrap();
        let live = ctx.pg().create_project("live-repo", None, None).await.unwrap();
        let repo_fid = ctx.pg().upsert_repo_kind(&root_id, "git", "live", &tmp.path().join("live").to_string_lossy()).await.unwrap();
        ctx.pg().set_folder_project(&repo_fid, &live, "root", None).await.unwrap();
        ctx.pg().upsert_node(&repo_fid, "function", "f", "live/lib.rs", None, None, None, None).await.unwrap();

        // Global `rows_affected` is racy in the shared test DB (a concurrent
        // scan's reconcile prunes too), so assert on our specific rows below.
        ctx.pg().prune_empty_projects(60).await.unwrap();
        let (ph_alive,): (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.projects WHERE id=$1")
            .bind(phantom).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(ph_alive, 0, "empty phantom project deleted");
        let (live_alive,): (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.projects WHERE id=$1")
            .bind(live).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(live_alive, 1, "populated project kept");
    }

    #[tokio::test]
    async fn scan_root_skips_excluded_subtree() {
        // Exclusion path: a git repo under an excluded prefix is never
        // classified as a project → no ProcessGitFolder enqueued for it, while a
        // repo outside the prefix is still processed.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("keep/.git")).unwrap();
        std::fs::write(root.join("keep/Cargo.toml"), "[package]\nname=\"keep\"").unwrap();
        std::fs::create_dir_all(root.join("Archive/skipme/.git")).unwrap();
        std::fs::write(root.join("Archive/skipme/Cargo.toml"), "[package]\nname=\"skipme\"").unwrap();

        let ctx = make_ctx().await;
        // Per-root exclusion: the watch root has excluded=["Archive"] (relative).
        // scan_root reads it via root_exclusion_prefixes(task.path).
        ctx.pg().add_watch_root(&root.to_string_lossy(), "wt", &serde_json::json!(["Archive"])).await.unwrap();

        let task = Task::new(TaskKind::ScanRoot, "", &root.to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        let pgf: Vec<String> = ctx.queue.snapshot().await.into_iter()
            .filter(|(k, _, _)| *k == TaskKind::ProcessGitFolder)
            .map(|(_, _, path)| path)
            .collect();
        assert!(pgf.iter().any(|p| p.ends_with("/keep")), "kept repo enqueued, got {pgf:?}");
        assert!(!pgf.iter().any(|p| p.contains("/skipme")), "excluded repo NOT enqueued, got {pgf:?}");
    }

    #[tokio::test]
    async fn scan_under_existing_root_reuses_it_not_a_new_root() {
        // Watch roots stay top-level: scanning a sub-path of an existing root
        // indexes under that root, never registering a redundant sub-root.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("sub/repo/.git")).unwrap();
        std::fs::write(root.join("sub/repo/Cargo.toml"), "[package]\nname=\"r\"").unwrap();

        let ctx = make_ctx().await;
        let parent_id = ctx.pg().add_watch_root(&root.to_string_lossy(), "top", &serde_json::json!([])).await.unwrap();

        // Scan the SUB-PATH — must reuse the top-level root, not create a new one.
        let task = Task::new(TaskKind::ScanRoot, "", &root.join("sub").to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        let enc = ctx.pg().enclosing_watch_root(&root.join("sub").to_string_lossy()).await.unwrap();
        assert_eq!(enc.map(|(_, p)| p), Some(root.to_string_lossy().to_string()),
            "no watch root registered at root/sub — the top-level root encloses it");

        let pool = ctx.pg().pool();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id=$1").bind(parent_id).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id=$1").bind(parent_id).execute(pool).await.ok();
    }

    #[tokio::test]
    async fn scan_root_enqueues_no_processgitfolder_for_a_workspace_member() {
        // #101 regression at the ENQUEUE/TRIGGER layer: a monorepo (git repo with
        // a workspace member crate) must enqueue exactly ONE ProcessGitFolder —
        // the repo — and NONE for the member. The member is structural; the repo
        // owns and indexes its files. This is the guard against re-promoting a
        // member to a second index owner (the 2026-07-13 double-owner residue).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("mono/.git")).unwrap();
        std::fs::write(root.join("mono/Cargo.toml"), "[workspace]\nmembers=[\"crates/*\"]").unwrap();
        std::fs::create_dir_all(root.join("mono/crates/mycrate/src")).unwrap();
        std::fs::write(root.join("mono/crates/mycrate/Cargo.toml"), "[package]\nname=\"mycrate\"").unwrap();
        std::fs::write(root.join("mono/crates/mycrate/src/lib.rs"), "pub fn a() {}").unwrap();

        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ScanRoot, "", &root.to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        let pgf: Vec<String> = ctx.queue.snapshot().await.into_iter()
            .filter(|(k, _, _)| *k == TaskKind::ProcessGitFolder)
            .map(|(_, _, path)| path)
            .collect();
        assert_eq!(pgf.len(), 1, "exactly one ProcessGitFolder (the repo), got {pgf:?}");
        assert!(pgf[0].ends_with("/mono"), "the ProcessGitFolder targets the repo root, got {pgf:?}");
        let member = root.join("mono/crates/mycrate").to_string_lossy().to_string();
        assert!(!pgf.iter().any(|p| p == &member), "no ProcessGitFolder for a workspace member, got {pgf:?}");
    }

    #[tokio::test]
    async fn scan_root_does_not_double_enqueue_a_folder_already_scanning() {
        // Single-writer (D6e/W5): if a git folder is already being scanned, the
        // ScanRoot fan-out must not enqueue a second ProcessGitFolder for it.
        // Without the guard the repo shows two ProcessGitFolder tasks.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/.git")).unwrap();
        std::fs::write(root.join("repo/Cargo.toml"), "[package]\nname=\"repo\"").unwrap();

        let ctx = make_ctx().await;
        let repo_path = root.join("repo").to_string_lossy().to_string();
        // Pre-seed: this repo is already being scanned.
        ctx.queue.enqueue(Task::new(TaskKind::ProcessGitFolder, &repo_path, &repo_path)).await;

        let task = Task::new(TaskKind::ScanRoot, "", &root.to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        let pgf: Vec<String> = ctx.queue.snapshot().await.into_iter()
            .filter(|(k, _, p)| *k == TaskKind::ProcessGitFolder && *p == repo_path)
            .map(|(_, _, p)| p)
            .collect();
        assert_eq!(pgf.len(), 1, "the already-scanning repo is not double-enqueued, got {pgf:?}");
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
    async fn prune_vanished_folders_drops_ghost_subtree_keeps_live() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let root_str = root.to_string_lossy().to_string();

        // A present git repo — the enclosing project root, confirmed on disk.
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let root_id = ctx.pg().add_watch_root(&root_str, "pvf", &serde_json::json!([])).await.unwrap();
        let repo_fid = ctx.pg().upsert_repo_kind(&root_id, "git", "repo", &repo.to_string_lossy()).await.unwrap();

        // A LIVE subfolder: its dir exists on disk → must be KEPT with its node.
        let live_dir = repo.join("live");
        std::fs::create_dir_all(&live_dir).unwrap();
        let live_fid = ctx.pg().upsert_subfolder(&root_id, "live", "live", &live_dir.to_string_lossy(), Some(&repo_fid), None).await.unwrap();
        let live_node = ctx.pg().upsert_node(&live_fid, "struct", "Kept", "live/mod.rs", None, None, None, None).await.unwrap();

        // A GHOST subtree: `gone/` (renamed/moved away) + its child `gone/sub/` no
        // longer exist on disk, yet still carry folder rows + nodes/edge/scan_state.
        let gone_dir = repo.join("gone");     // NOT created on disk
        let gone_sub = gone_dir.join("sub");  // NOT created on disk
        let gone_fid = ctx.pg().upsert_subfolder(&root_id, "gone", "gone", &gone_dir.to_string_lossy(), Some(&repo_fid), None).await.unwrap();
        let sub_fid = ctx.pg().upsert_subfolder(&root_id, "sub", "gone/sub", &gone_sub.to_string_lossy(), Some(&gone_fid), None).await.unwrap();
        let ghost_a = ctx.pg().upsert_node(&gone_fid, "struct", "HiveConfig", "gone/config.rs", None, None, None, None).await.unwrap();
        let ghost_b = ctx.pg().upsert_node(&sub_fid, "struct", "HiveStore", "gone/sub/store.rs", None, None, None, None).await.unwrap();
        let ghost_edge = ctx.pg().insert_edge(&gone_fid, &ghost_a, Some(&ghost_b), None, None, "references").await.unwrap();
        ctx.pg().upsert_scan_state(&gone_fid, "gone/config.rs", 1, "h").await.unwrap();

        // Prune: both ghost folder rows go, the live one stays.
        let pruned = prune_vanished_folders(ctx.pg(), &root_id).await;
        assert_eq!(pruned, 2, "both ghost folder rows (gone + gone/sub) should be pruned");

        // Ghost folder rows removed; repo + live folder rows survive.
        let abs_paths: std::collections::HashSet<String> = ctx.pg().list_folders_by_root(&root_id).await.unwrap()
            .iter().filter_map(|r| r["abs_path"].as_str().map(String::from)).collect();
        assert!(!abs_paths.contains(&gone_dir.to_string_lossy().to_string()), "ghost folder row pruned");
        assert!(!abs_paths.contains(&gone_sub.to_string_lossy().to_string()), "ghost child folder row pruned");
        assert!(abs_paths.contains(&live_dir.to_string_lossy().to_string()), "live folder row kept");
        assert!(abs_paths.contains(&repo.to_string_lossy().to_string()), "repo root kept");

        // Cascade: ghost nodes + edge + scan_state gone; live node survives.
        let (ghost_nodes,): (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.nodes WHERE id = ANY($1)")
            .bind(vec![ghost_a, ghost_b]).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(ghost_nodes, 0, "ghost nodes cascade-deleted");
        let (live_nodes,): (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.nodes WHERE id = $1")
            .bind(live_node).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(live_nodes, 1, "live node preserved");
        let (edge_count,): (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.edges WHERE id = $1")
            .bind(ghost_edge).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(edge_count, 0, "ghost edge cascade-deleted");
        assert!(ctx.pg().list_scan_state(&gone_fid).await.unwrap().is_empty(), "ghost scan_state cascade-deleted");
        assert_eq!(ctx.pg().list_indexed_files(&live_fid).await.unwrap(), vec!["live/mod.rs".to_string()], "live folder's nodes intact");

        // Idempotent: a second run prunes nothing.
        assert_eq!(prune_vanished_folders(ctx.pg(), &root_id).await, 0, "re-run is a no-op");
    }

    #[tokio::test]
    async fn prune_vanished_folders_drops_ghost_workspace_member() {
        // D5a: a monorepo member (kind='workspace_member') whose dir vanished is
        // ghost-pruned like a structural `folder` — detect_vanished_folders was
        // extended to include workspace_member so a deleted member doesn't linger.
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let root_id = ctx.pg().add_watch_root(&root.to_string_lossy(), "pvfwm", &serde_json::json!([])).await.unwrap();
        let repo_fid = ctx.pg().upsert_repo_kind(&root_id, "git", "repo", &repo.to_string_lossy()).await.unwrap();

        // A LIVE member (dir exists) and a GHOST member (dir never created).
        let live = repo.join("packages/live");
        std::fs::create_dir_all(&live).unwrap();
        ctx.pg().upsert_subfolder_kind(&root_id, "workspace_member", "live", "packages/live", &live.to_string_lossy(), Some(&repo_fid), None).await.unwrap();
        let gone = repo.join("packages/gone"); // NOT created on disk
        ctx.pg().upsert_subfolder_kind(&root_id, "workspace_member", "gone", "packages/gone", &gone.to_string_lossy(), Some(&repo_fid), None).await.unwrap();

        let pruned = prune_vanished_folders(ctx.pg(), &root_id).await;
        assert_eq!(pruned, 1, "the vanished workspace_member is ghost-pruned");
        let abs_paths: std::collections::HashSet<String> = ctx.pg().list_folders_by_root(&root_id).await.unwrap()
            .iter().filter_map(|r| r["abs_path"].as_str().map(String::from)).collect();
        assert!(!abs_paths.contains(&gone.to_string_lossy().to_string()), "ghost member row pruned");
        assert!(abs_paths.contains(&live.to_string_lossy().to_string()), "live member row kept");
        assert!(abs_paths.contains(&repo.to_string_lossy().to_string()), "repo root kept");
    }

    #[tokio::test]
    async fn prune_vanished_folders_skips_when_enclosing_root_absent() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let root_str = root.to_string_lossy().to_string();
        let root_id = ctx.pg().add_watch_root(&root_str, "pvf_absent", &serde_json::json!([])).await.unwrap();

        // A project root whose OWN directory is absent on disk (never created) —
        // simulates an unmounted volume / moved repo. reconcile_roots owns this;
        // prune_vanished_folders must NOT touch anything under it.
        let absent_repo = root.join("absent-repo"); // NOT created on disk
        let repo_fid = ctx.pg().upsert_repo_kind(&root_id, "git", "absent-repo", &absent_repo.to_string_lossy()).await.unwrap();
        let sub = absent_repo.join("src");           // also absent
        let sub_fid = ctx.pg().upsert_subfolder(&root_id, "src", "src", &sub.to_string_lossy(), Some(&repo_fid), None).await.unwrap();
        let node = ctx.pg().upsert_node(&sub_fid, "struct", "Untouched", "src/lib.rs", None, None, None, None).await.unwrap();

        // Prune: enclosing root absent → nothing under it is pruned.
        let pruned = prune_vanished_folders(ctx.pg(), &root_id).await;
        assert_eq!(pruned, 0, "an absent root must not have its subtree pruned (unmounted-volume safety)");

        // The subfolder row + its node survive.
        let (exists,): (bool,) = sqlx_core::query_as::query_as("SELECT EXISTS(SELECT 1 FROM sensei.nodes WHERE id = $1)")
            .bind(node).fetch_one(ctx.pg().pool()).await.unwrap();
        assert!(exists, "subtree under an absent root must be preserved");
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
