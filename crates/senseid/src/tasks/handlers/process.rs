//! Process phase: index repos, folders, and files; handle deletions.

use std::path::Path;
use super::super::executor::TaskContext;
use super::super::{Task, TaskKind};
use super::helpers::{is_binary_ext, is_probably_binary, build_globset};

// ── Process Repo ──────────────────────────────────────────────────────────

/// Process a git folder: detect stack, count files, create/find project, emit events, enqueue file tasks.
pub async fn process_git_folder(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let repo_path = Path::new(&task.path);
    if !repo_path.exists() {
        return Err(format!("Repo path does not exist: {}", task.path));
    }

    let folder_path = &task.folder_path;
    let emit = |evt: crate::api::events::StateEvent| { let _ = ctx.app_state.event_tx.send(evt); };

    // Display name comes from the DB row (looked up by abs_path) so subtree
    // labels like "sensei:homebrew" survive — task.folder_name() would
    // return the basename only.
    let pre_registered = ctx.pg().get_repo_by_path(&task.path).await.ok().flatten();
    let folder_name_owned: String = pre_registered
        .as_ref()
        .and_then(|r| r["name"].as_str().map(String::from))
        .unwrap_or_else(|| task.folder_name().to_string());
    let folder_name: &str = &folder_name_owned;

    // A quasi-repo (non-git project root) is its own project named after the
    // folder; a real repo groups under its parent directory (the legacy
    // multi-repo grouping heuristic).
    let is_quasi = pre_registered.as_ref()
        .and_then(|r| r["kind"].as_str()) == Some("standalone");

    // ── 1. Detect stack ──────────────────────────────────────────────
    let stack = super::scan_logic::detect_stack(repo_path);

    // ── 2. Count indexable files ─────────────────────────────────────
    let (_indexable_files, files_total) = super::scan_logic::count_indexable_files(repo_path);

    // ── 3. Find or create project ────────────────────────────────────
    // Every project root is its own project, named after itself. Grouping
    // multiple folders into one project is opt-in via README frontmatter
    // `project:` (e.g. a monorepo and its subtrees that all set
    // `project: sensei`). Parent-directory grouping is deliberately NOT used —
    // it conflated unrelated repos that merely share a parent dir (e.g. every
    // repo under the scan root collapsing into one "Developer" project).
    // Scanning is read-only — frontmatter is never written back.
    let fm = crate::tasks::processors::metadata::read_frontmatter(repo_path).unwrap_or_default();
    // Project name resolution:
    //  * a git subtree (composite folder name "parent:sub") groups with its
    //    parent repository's project unless its own README sets `project:`;
    //  * every other project root is its own project, named after itself,
    //    unless its README sets `project:`.
    let project_name_owned: String = match folder_name.split_once(':') {
        Some((parent_repo, _)) if fm.project.is_none() => {
            // Inherit the parent repository's project (the monorepo it was split
            // from), falling back to the parent repo's folder name.
            let parent_project = ctx.pg().get_repo_by_name(parent_repo).await.ok().flatten()
                .and_then(|f| crate::api::util::json_uuid(&f["project_id"]));
            match parent_project {
                Some(pid) => ctx.pg().get_project(&pid).await.ok().flatten()
                    .and_then(|p| p["name"].as_str().map(String::from))
                    .unwrap_or_else(|| parent_repo.to_string()),
                None => parent_repo.to_string(),
            }
        }
        _ => fm.project.clone().unwrap_or_else(|| folder_name.to_string()),
    };
    let project_name: &str = &project_name_owned;

    // Find or create the project by its resolved name (get-or-create — idempotent).
    let project_id = match ctx.pg().get_project_by_name(project_name).await {
        Ok(Some(proj)) => {
            // Project exists — use it
            proj["id"].as_str().unwrap_or("").to_string()
        }
        _ => {
            // Create new project
            let id = ctx.pg().create_project(project_name, None, None).await
                .map(|id| id.to_string())
                .unwrap_or_else(|_| format!("p-{}", project_name));

            // Emit: project add
            emit(crate::api::events::StateEvent::project_add(crate::api::events::ScanProject {
                id: id.clone(),
                name: project_name.to_string(),
                status: crate::api::events::ProjectStatus::Indexing,
                folders: vec![],
                auto_detected: true,
                confidence: crate::api::events::Confidence::High,
            }));

            id
        }
    };

    // A quasi-repo (non-git project root) is tagged so the UI can surface it as
    // provisional — the user can discard it or promote it (git init → re-scanned
    // as a real repo). The folder kind=standalone already marks the folder; this
    // marks the project. Idempotent (tag union).
    if is_quasi
        && let Ok(pid) = uuid::Uuid::parse_str(&project_id) {
            ctx.pg().set_project_identity(&pid, None, None, &[], &["quasi-repo".to_string()]).await.ok();
        }

    // ── 4. Emit: folder add with stack + file count ──────────────────
    // Reuse the lookup we already did to derive folder_name. abs_path is
    // unique on sensei.folders so the row identifies this exact repo
    // (vs name which can collide across roots).
    let folder_by_path = pre_registered;
    let folder_uuid_str = folder_by_path.as_ref()
        .and_then(|f| f["id"].as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| format!("f-{}", folder_name));
    // Capture the project-root folder id + watch-root id now (Copy), before
    // `folder_by_path` is moved below; used later to materialize the subfolder
    // tree.
    let project_root_uuid = folder_by_path.as_ref().and_then(|f| crate::api::util::json_uuid(&f["id"]));
    let repo_root_uuid = folder_by_path.as_ref().and_then(|f| crate::api::util::json_uuid(&f["root_id"]));

    emit(crate::api::events::StateEvent::folder_add(crate::api::events::ScanFolder {
        id: folder_uuid_str.clone(),
        project_id: project_id.clone(),
        name: folder_name.to_string(),
        path: task.path.clone(),
        kind: if is_quasi {
            crate::api::events::FolderKind::Standalone
        } else {
            crate::api::events::FolderKind::Git
        },
        stack: stack.clone(),
        files_total,
        files_completed: 0,
        status: crate::api::events::FolderStatus::Queued,
    }));

    // ── 5. Emit: activity queue ──────────────────────────────────────
    emit(crate::api::events::StateEvent::activity(crate::api::events::ActivityEvent::new(
        crate::api::events::ActivityLevel::Queue,
        &format!("{} · {} files queued · {}", folder_name, files_total, stack.join(", ")),
        0.0,
    )));

    // ── Existing logic: look up folder by path, clear stale data ─────
    // Use the path-based row (same as folder_by_path above) to avoid
    // name collisions with identically-named repos from prior runs.
    let folder = folder_by_path;
    let folder_uuid = folder.as_ref()
        .and_then(|f| crate::api::util::json_uuid(&f["id"]));

    // ── Persist project_id on the folder record ──────────────────────
    // upsert_repo does not set project_id; do it now so that
    // progress_emitter::build_tracker can read it via get_repo_by_path.
    if let (Some(fid), Ok(pid)) = (&folder_uuid, uuid::Uuid::parse_str(&project_id)) {
        ctx.pg().set_folder_project(fid, &pid, "primary", None).await.ok();
    }

    // Record the indexed git branch in props.branch — preferred from the
    // BranchSwitch task that triggered this re-index, otherwise read from
    // .git/HEAD. Lets the UI show which branch is indexed and gives a later
    // switch the prior branch for context. (Quasi-repos have no HEAD → skipped.)
    if let Some(fid) = &folder_uuid {
        let branch = task.branch.clone()
            .or_else(|| crate::watcher::root_watcher::read_git_head(&format!("{}/.git/HEAD", task.path)));
        if let Some(br) = branch {
            ctx.pg().set_folder_props(fid, &serde_json::json!({ "branch": br })).await.ok();
        }
    }

    // Incremental index: load the prior per-file fingerprints so we only
    // re-process files whose mtime changed (edited / new / pulled / brought in
    // by a branch switch), skip unchanged files (their nodes + embeddings stay
    // valid), and drop files no longer on disk. The first index sees an empty
    // scan_state and processes everything, populating it. This replaces the old
    // blanket `delete_nodes_by_folder` wipe + full re-parse on every scan.
    let prior_state: std::collections::HashMap<String, i64> = match &folder_uuid {
        Some(fid) => ctx.pg().list_scan_state(fid).await.unwrap_or_default().into_iter().collect(),
        None => std::collections::HashMap::new(),
    };

    // Detect workspace members
    let workspace_members = crate::config::detector::detect_workspace_members(repo_path);

    // Discover directories and enqueue folder tasks
    let exclude = build_globset();
    let mut dirs = std::collections::HashSet::new();

    // Walk all files to discover directories
    let walker = super::helpers::build_walker(repo_path).build();

    for entry in walker.flatten() {
        if !entry.path().is_file() { continue; }
        let rel = entry.path().strip_prefix(repo_path).unwrap_or(entry.path());
        let rel_str = rel.to_string_lossy();
        if exclude.is_match(&*rel_str) { continue; }

        // Skip binary files and files without extensions
        let ext = entry.path().extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();
        if ext.is_empty() { continue; }
        if is_binary_ext(&ext) { continue; }

        if let Some(parent) = entry.path().parent() {
            dirs.insert(parent.to_path_buf());
        }
    }

    // Enumerate the working tree's indexable files with mtimes (one read_dir
    // pass per discovered dir), keyed by abs path → rel path.
    let mut current_meta: std::collections::HashMap<std::path::PathBuf, String> = std::collections::HashMap::new();
    let mut current: Vec<(String, i64)> = Vec::new();
    for dir in &dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if !entry.path().is_file() { continue; }
                let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
                if ext.is_empty() || is_binary_ext(&ext) { continue; }
                let rel = entry.path().strip_prefix(repo_path).unwrap_or(&entry.path())
                    .to_string_lossy().to_string();
                let mtime = super::helpers::file_mtime_ms(&entry.path()).unwrap_or(0);
                current.push((rel.clone(), mtime));
                current_meta.insert(entry.path(), rel);
            }
        }
    }
    // Diff against the last index → which files to (re)process, which to drop.
    let plan = super::scan_logic::incremental_plan(&current, &prior_state);

    // Enqueue ProcessFolder + ProcessFile only for changed files, grouped by dir
    // so each gets its module/package context. Unchanged dirs enqueue nothing.
    let mut all_file_task_ids: Vec<u64> = Vec::new();
    let root_pkg_id = format!("pkg:{}:(root)", folder_name);
    for dir in &dirs {
        let changed_here: Vec<&std::path::PathBuf> = current_meta.iter()
            .filter(|(abs, rel)| abs.parent() == Some(dir.as_path()) && plan.changed.contains(*rel))
            .map(|(abs, _)| abs)
            .collect();
        if changed_here.is_empty() {
            continue;
        }

        let rel_dir = dir.strip_prefix(repo_path).unwrap_or(dir).to_string_lossy().to_string();
        let abs_dir = dir.to_string_lossy().to_string();
        let pkg_id = workspace_members.iter()
            .find(|pkg| rel_dir.starts_with(&pkg.path))
            .map(|pkg| format!("pkg:{}:{}", folder_name, pkg.name))
            .unwrap_or_else(|| root_pkg_id.clone());

        let mut ft = Task::new(TaskKind::ProcessFolder, folder_path, &abs_dir)
            .with_parent(task.id);
        ft.module_id = Some(pkg_id);
        let folder_id = ctx.queue.enqueue(ft).await;

        let rel_dir_name = if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir.replace('\\', "/") };
        let mod_id = format!("mod:{}:{}", folder_name, rel_dir_name);
        for abs in changed_here {
            let file_task = Task::new(TaskKind::ProcessFile, folder_path, &abs.to_string_lossy())
                .with_parent(folder_id)
                .with_module(&mod_id);
            all_file_task_ids.push(ctx.queue.enqueue(file_task).await);
        }
    }

    // Files indexed before but gone now (deleted on disk, or removed by a branch
    // switch): un-resolve inbound edges, drop their nodes (cascades their edges),
    // and clear their scan-state rows.
    if let Some(ref fid) = folder_uuid {
        for path in &plan.removed {
            ctx.pg().unresolve_edges_to_file(fid, path).await.ok();
            ctx.pg().delete_nodes_by_file(fid, path).await.ok();
            ctx.pg().delete_scan_state_file(fid, path).await.ok();
        }
    }

    // Materialize the subfolder tree as kind=folder rows so the project's
    // directory structure is navigable. Storage starts at the project root —
    // wrapper directories above it were never registered. Built top-down so each
    // child's parent_id resolves to an already-created folder row.
    if let (Some(root_uuid), Some(proj_root_uuid), Ok(pid)) =
        (repo_root_uuid, project_root_uuid, uuid::Uuid::parse_str(&project_id))
    {
        let file_dirs: Vec<std::path::PathBuf> = dirs.iter().cloned().collect();
        let tree = super::scan_logic::subfolder_tree(repo_path, &file_dirs);
        let mut path_to_id: std::collections::HashMap<std::path::PathBuf, uuid::Uuid> =
            std::collections::HashMap::new();
        path_to_id.insert(repo_path.to_path_buf(), proj_root_uuid);
        for (dir, parent) in tree {
            let parent_id = path_to_id.get(&parent).copied().unwrap_or(proj_root_uuid);
            let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let rel = dir.strip_prefix(repo_path).unwrap_or(&dir).to_string_lossy().to_string();
            let abs = dir.to_string_lossy().to_string();
            if let Ok(fid) = ctx.pg()
                .upsert_subfolder(&root_uuid, &name, &rel, &abs, Some(&parent_id), Some(&pid))
                .await
            {
                path_to_id.insert(dir, fid);
            }
        }
    }

    // Emit FolderQueued event with file count so UI can show accurate progress
    let _ = ctx.queue.sender().send(
        crate::tasks::progress::TaskEvent::FolderQueued {
            folder_path: task.folder_path.clone(),
            files_total: all_file_task_ids.len() as u32,
        }
    );

    // Folder-level barriers (edge/lib resolution, connections, embeddings) only
    // need to run when this folder's nodes actually changed — files were
    // (re)indexed or removed. On an unchanged incremental re-scan (e.g. a branch
    // switch elsewhere) the folder's edges + embeddings are already valid, so we
    // skip the barriers entirely. This keeps a no-op re-scan cheap.
    let has_changes = !all_file_task_ids.is_empty() || !plan.removed.is_empty();
    if has_changes {
        let resolve_id = ctx.queue.enqueue(
            Task::new(TaskKind::ResolveEdges, folder_path, "")
                .with_parent(task.id)
                .blocked_by(all_file_task_ids.clone())
        ).await;

        let libs_id = ctx.queue.enqueue(
            Task::new(TaskKind::ResolveLibs, folder_path, "")
                .with_parent(task.id)
                .blocked_by(vec![resolve_id])
        ).await;

        ctx.queue.enqueue(
            Task::new(TaskKind::BuildConnections, folder_path, "")
                .with_parent(task.id)
                .blocked_by(vec![libs_id])
        ).await;

        // Embed code-graph nodes for semantic search + duplicate detection.
        // Barrier on the file tasks so every node exists before we embed it;
        // independent of edge/connection resolution so it runs in parallel.
        ctx.queue.enqueue(
            Task::new(TaskKind::EmbedNodes, folder_path, "")
                .with_parent(task.id)
                .blocked_by(all_file_task_ids.clone())
        ).await;
    }

    // Detect subtrees → register as separate repos
    {
        let folder = ctx.pg().get_repo_by_path(&task.path).await.ok().flatten();
        if folder.is_some() {
            // Detect git subtrees
            let subtrees = crate::indexer::cross_repo::detect_git_subtrees_pub(repo_path);
            if !subtrees.is_empty() {
                // Register each subtree as a separate repo via PgStore
                // Look up the root_id for upsert_repo
                let root_id = folder.as_ref()
                    .and_then(|f| crate::api::util::json_uuid(&f["root_id"]));

                if let Some(root_id) = root_id {
                    for (name, subtree_path) in &subtrees {
                        let subtree_folder_name = format!("{}:{}", folder_name, name);
                        ctx.pg().upsert_repo(&root_id, &subtree_folder_name, subtree_path).await.ok();
                    }
                }

                for (name, subtree_path) in &subtrees {
                    // folder_path is an abs_path per the Task struct contract;
                    // the composite display name "{folder_name}:{name}" lives
                    // on sensei.folders.name (upserted above) and is read
                    // back by handlers via get_repo_by_path.
                    let sub_task = Task::new(TaskKind::ProcessGitFolder, subtree_path, subtree_path)
                        .with_parent(task.id);
                    ctx.queue.enqueue(sub_task).await;
                    let subtree_folder_name = format!("{}:{}", folder_name, name);
                    tracing::info!("process_git_folder: enqueued subtree {} at {}", subtree_folder_name, subtree_path);
                }
            }
        }
    }

    // Repo-level metadata + identity reconcile from README frontmatter
    // (filesystem-only, READ-ONLY). Extracted into reconcile_repo_identity so
    // the watcher can re-run it incrementally on a README change.
    let _ = reconcile_repo_identity(ctx, &task.path).await;

    // Self-healing reconcile: re-tag orphaned discovery projects (no delete).
    ctx.pg().mark_orphaned_projects().await.ok();

    tracing::info!("process_git_folder: {} — {} dirs, {} changed files, {} removed", folder_name, dirs.len(), all_file_task_ids.len(), plan.removed.len());
    Ok(all_file_task_ids.len() as u32)
}

// ── Reconcile identity ─────────────────────────────────────────────────────

/// Reconcile a project root's identity FROM its README frontmatter — folder
/// props (incl. the frontmatter snapshot), icons, project identity, role, and
/// folder_namespaces. Filesystem-READ-ONLY (it never writes the README, so it
/// can't trigger a file-change loop), idempotent, and additive. Shared by the
/// scan pipeline (process_git_folder) and the watcher's ReconcileIdentity task.
pub async fn reconcile_repo_identity(ctx: &TaskContext, repo_abs_path: &str) -> Result<u32, String> {
    use crate::tasks::processors::metadata;
    let repo_path = Path::new(repo_abs_path);

    let Some(folder) = ctx.pg().get_repo_by_path(repo_abs_path).await.ok().flatten() else {
        return Ok(0); // not a registered folder
    };
    // Only project roots carry identity. A README inside a subfolder
    // (kind='folder') must not reconcile project/namespace/icon state.
    if !matches!(folder["kind"].as_str(), Some("git" | "standalone" | "subtree")) {
        return Ok(0);
    }
    let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) else { return Ok(0); };

    let fm = metadata::read_frontmatter(repo_path).unwrap_or_default();
    let icon = metadata::scan_icons(repo_path);
    let links = metadata::scan_external_links(repo_path);
    let summary = metadata::extract_summary(repo_path);
    let stack = super::scan_logic::detect_stack(repo_path);

    // Folder props: scanned metadata + the parsed frontmatter blob. The
    // frontmatter snapshot here is what reconcile_identity compares against to
    // suppress no-op re-reconciles.
    let meta = serde_json::json!({
        "icon": icon,
        "external_links": links.links,
        "summary": summary,
        "frontmatter": serde_json::to_value(&fm).unwrap_or(serde_json::Value::Null),
    });
    ctx.pg().set_folder_props(&folder_id, &meta).await.ok();

    // Icon variants + URL-vs-repo-relative classification (root READMEs only).
    if let Some(icon_path) = fm.icon.as_deref() {
        let mut icons = serde_json::json!({
            "custom": icon_path,
            "custom_is_url": metadata::icon_is_url(icon_path),
        });
        if let Some(dark) = fm.icon_dark.as_deref() {
            icons["custom_dark"] = serde_json::json!(dark);
            icons["custom_dark_is_url"] = serde_json::json!(metadata::icon_is_url(dark));
        }
        ctx.pg().set_folder_icons(&folder_id, &icons).await.ok();
    }

    // Project identity + role + namespaces (only when linked to a project).
    if let Some(pid) = folder["project_id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
        // Authoritative project name (matches what the scan created) for the
        // `project` namespace; fall back to frontmatter / folder name.
        let project_name = ctx.pg().get_project(&pid).await.ok().flatten()
            .and_then(|p| p["name"].as_str().map(String::from))
            .or_else(|| fm.project.clone())
            .or_else(|| folder["name"].as_str().map(String::from))
            .unwrap_or_default();

        let id_stack: Vec<String> =
            if fm.stack.is_empty() { stack.clone() } else { fm.stack.clone() };
        let mut tags: Vec<String> = Vec::new();
        if let Some(role) = fm.role.as_deref() {
            // Keep the raw role as a project tag (lossless), map known generic
            // roles onto the folder.role enum column.
            tags.push(format!("role:{role}"));
            if let Some(fr) = metadata::folder_role_from_frontmatter(role) {
                ctx.pg().update_folder_role(&folder_id, Some(fr)).await.ok();
            }
        }
        if let Some(org) = fm.organization.as_deref() {
            tags.push(format!("org:{}", metadata::slugify(org)));
        }
        ctx.pg().set_project_identity(
            &pid, fm.summary.as_deref(), fm.client.as_deref(), &id_stack, &tags,
        ).await.ok();

        let mut ns: Vec<(&str, String)> = Vec::new();
        if let Some(org) = fm.organization.as_deref() { ns.push(("organization", org.to_string())); }
        if !project_name.is_empty() { ns.push(("project", project_name.clone())); }
        if let Some(team) = fm.team.as_deref() { ns.push(("team", team.to_string())); }
        for lang in &id_stack { ns.push(("technology", lang.clone())); }
        for (scope, name) in &ns {
            let slug = metadata::slugify(name);
            if slug.is_empty() { continue; }
            if let Ok(ns_id) = ctx.pg().upsert_namespace(scope, name, &slug).await {
                ctx.pg().link_folder_namespace(&folder_id, &ns_id).await.ok();
            }
        }
    }
    Ok(1)
}

/// Watcher-triggered re-reconcile: re-apply identity when a project-root README
/// changes — but only if its frontmatter actually differs from the snapshot we
/// last stored. The change-detection makes a frontmatter write-back (#22) or a
/// body-only README edit a no-op, so a UI-driven change → README write → watcher
/// event neither loops nor churns the DB. `task.path` is the project-root abs
/// path (set by the watcher).
pub async fn reconcile_identity(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    use crate::tasks::processors::metadata;
    let repo_path = Path::new(&task.path);

    let fresh = serde_json::to_value(metadata::read_frontmatter(repo_path).unwrap_or_default())
        .unwrap_or(serde_json::Value::Null);
    let stored = ctx.pg().get_repo_by_path(&task.path).await.ok().flatten()
        .and_then(|f| f.get("props").and_then(|p| p.get("frontmatter")).cloned());
    if stored.as_ref() == Some(&fresh) {
        tracing::debug!("reconcile_identity: {} — frontmatter unchanged, skipping", task.path);
        return Ok(0);
    }

    tracing::info!("reconcile_identity: {} — frontmatter changed, reconciling", task.path);
    reconcile_repo_identity(ctx, &task.path).await
}

// ── Process Folder ────────────────────────────────────────────────────────

/// Create module node for a folder.
pub async fn process_folder(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    // folder_path is the repo's abs_path by contract — look the row up
    // directly instead of round-tripping through name (which can collide
    // across roots and breaks for subtrees whose DB name is a composite
    // like "sensei:homebrew").
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    let folder_id = folder.as_ref()
        .and_then(|f| crate::api::util::json_uuid(&f["id"]));

    let rel_dir = Path::new(&task.path).strip_prefix(Path::new(&task.folder_path))
        .unwrap_or(Path::new(&task.path))
        .to_string_lossy().to_string();

    // Write module node to PG
    if let Some(ref fid) = folder_id {
        let mod_name = if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir.replace('\\', "/") };
        ctx.pg().upsert_node(fid, "module", &mod_name, &task.path, None, None, None, None).await.ok();
    }

    Ok(0)
}

// ── Process File ──────────────────────────────────────────────────────────

/// Parse a single file using file_processor, then write results to graph.
pub async fn process_file(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let abs_path = &task.path;

    // Skip files we can't parse as source text. Returning Ok (not Err) is
    // critical: a failed ProcessFile task would block its folder's
    // resolve_edges barrier, leaving the folder stuck at 'discovered'. Binary
    // (by extension) and non-UTF8 (by content sniff) files are skipped so
    // indexing always completes.
    let fpath = std::path::Path::new(abs_path);
    let ext = fpath.extension().and_then(|e| e.to_str()).unwrap_or("");
    if is_binary_ext(ext) || is_probably_binary(fpath) {
        return Ok(0);
    }

    // Lookup once by abs_path; folder name comes from the DB row so subtree
    // composite names ("sensei:homebrew") survive as the repo_id passed to
    // downstream processors that namespace symbol IDs by repo.
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    let folder_name = folder.as_ref()
        .and_then(|r| r["name"].as_str())
        .unwrap_or_else(|| task.folder_name());

    // Parse on a blocking thread. Parsing is synchronous, CPU-bound work; left
    // on the async runtime it blocks the worker's poll() — and a parse that
    // wedges (a pathological input, or a shared non-Sync parser contended by
    // concurrent files) would freeze the thread *inside* poll(), which the
    // executor watchdog cannot preempt (tokio::timeout only fires when the
    // future yields Pending). spawn_blocking moves it off the runtime so the
    // worker yields, the watchdog can fire, and one bad file can't wedge the
    // pool. A read/parse error is tolerated (skip, don't fail) so it never
    // blocks the folder's resolve_edges barrier.
    let folder_id = folder.as_ref().and_then(|f| crate::api::util::json_uuid(&f["id"]));
    let abs_owned = abs_path.clone();
    let folder_path_owned = task.folder_path.clone();
    let folder_name_owned = folder_name.to_string();
    let parsed = tokio::task::spawn_blocking(move || {
        crate::tasks::processors::process_file(&abs_owned, &folder_path_owned, &folder_name_owned)
    }).await;
    let result = match parsed {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            // Read/parse error — record per-file in index_errors (surfaced in the
            // UI) instead of silently dropping it.
            if let Some(fid) = &folder_id {
                ctx.pg().log_index_error(fid, abs_path, &e, Some(ext), Some("parse")).await.ok();
            }
            tracing::debug!("process_file: skipping unparseable {abs_path}: {e}");
            return Ok(0);
        }
        Err(join_err) => {
            // The parser panicked (e.g. a tree-sitter node byte-range past the
            // source). spawn_blocking turned it into a JoinError so the worker
            // survives; record which file so panicking inputs are observable
            // rather than vanishing.
            let msg = format!("parser panicked: {join_err}");
            if let Some(fid) = &folder_id {
                ctx.pg().log_index_error(fid, abs_path, &msg, Some(ext), Some("parse")).await.ok();
            }
            tracing::warn!("process_file: {abs_path}: {msg}");
            return Ok(0);
        }
    };

    // Write parsed symbols to PG
    let symbols_count = result.symbols.len();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            // Re-indexing this file: un-resolve inbound edges (preserve their
            // target_name so resolve_edges re-points them to the new nodes) and
            // drop the file's prior nodes (cascading their outgoing edges) so the
            // rewrite is clean. No-op for a brand-new file.
            ctx.pg().unresolve_edges_to_file(&folder_id, &result.rel_path).await.ok();
            ctx.pg().delete_nodes_by_file(&folder_id, &result.rel_path).await.ok();

            // Write file node
            let file_node_id = ctx.pg().upsert_node(
                &folder_id, &result.kind, &result.rel_path, &result.rel_path, None, None, None, None
            ).await.ok();

            // Write symbol nodes (functions, classes, types, etc.)
            for sym in &result.symbols {
                let parent_uuid = file_node_id; // symbols are children of the file
                ctx.pg().upsert_node(
                    &folder_id, &sym.kind, &sym.name, &result.rel_path,
                    parent_uuid.as_ref(), sym.signature.as_deref(),
                    Some(sym.line as i32), Some(sym.line_end as i32),
                ).await.ok();
            }

            // Write unresolved import edges
            for import in &result.unresolved_imports {
                if let Some(ref fid) = file_node_id {
                    ctx.pg().insert_edge(&folder_id, fid, None, Some(import), "imports").await.ok();
                }
            }

            // Write unresolved call edges
            for call in &result.unresolved_calls {
                if let Some(ref fid) = file_node_id {
                    ctx.pg().insert_edge(&folder_id, fid, None, Some(&call.callee_name), "calls").await.ok();
                }
            }

            // Write parent refs (HAS_METHOD: type → method)
            for pref in &result.parent_refs {
                if let Some(ref fid) = file_node_id {
                    ctx.pg().insert_edge(&folder_id, fid, None, Some(&pref.parent_name), "extends").await.ok();
                }
            }

            // Write doc references (file_refs → COVERS, fn_mentions → references)
            if result.kind == "doc"
                && let Some(ref fid) = file_node_id {
                    for file_ref in &result.file_refs {
                        ctx.pg().insert_edge(&folder_id, fid, None, Some(file_ref), "covers").await.ok();
                    }
                    for fn_ref in &result.fn_mentions {
                        ctx.pg().insert_edge(&folder_id, fid, None, Some(fn_ref), "references").await.ok();
                    }
                }

            // Record this file's fingerprint so the next scan skips it when
            // unchanged. Written last so a row exists only for a fully-indexed file.
            if let Some((mtime, hash)) = super::helpers::file_fingerprint(fpath) {
                ctx.pg().upsert_scan_state(&folder_id, &result.rel_path, mtime, &hash).await.ok();
            }
        }

    Ok(symbols_count as u32)
}

// ── Delete File / Folder ──────────────────────────────────────────────────

pub async fn delete_file(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    // folder_path is the repo abs_path (Task contract).
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            ctx.pg().delete_nodes_by_file(&folder_id, &task.path).await.ok();
        }
    tracing::info!("delete_file: {}", task.path);
    Ok(0)
}

pub async fn delete_folder(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            ctx.pg().delete_nodes_by_path_prefix(&folder_id, &task.path).await.ok();
        }
    tracing::info!("delete_folder: {}", task.path);
    Ok(0)
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
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
            gateway,
            event_tx: { let (tx, _) = tokio::sync::broadcast::channel(16); tx },
        });
        Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
            logger: sensei_logger::Logger::noop(),
        })
    }

    #[tokio::test]
    async fn process_git_folder_errors_on_nonexistent_path() {
        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ProcessGitFolder, "/nonexistent/repo", "/nonexistent/repo");
        let result = process_git_folder(&ctx, &task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn process_folder_creates_module_node() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();

        let ctx = make_ctx().await;
        let folder_name = "test-repo";
        let repo_path = tmp.path().to_string_lossy().to_string();

        // Register the project so process_folder can look up its path
        {
            let root_id = ctx.pg().add_watch_root(&repo_path, "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, folder_name, &repo_path).await.unwrap();
        }

        let pkg_id = format!("pkg:{}:(root)", folder_name);

        let mut task = Task::new(TaskKind::ProcessFolder, &repo_path, &src_dir.to_string_lossy());
        task.module_id = Some(pkg_id.clone());

        process_folder(&ctx, &task).await.unwrap();

        // TODO: verify module node once module writes are implemented
    }

    #[tokio::test]
    async fn scan_state_list_and_delete_file_roundtrip() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();
        let root_id = ctx.pg().add_watch_root(&repo_path, "ss", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "ss-repo", &repo_path).await.unwrap();

        ctx.pg().upsert_scan_state(&fid, "a.rs", 111, "hashA").await.unwrap();
        ctx.pg().upsert_scan_state(&fid, "b.rs", 222, "hashB").await.unwrap();
        let state = ctx.pg().list_scan_state(&fid).await.unwrap();
        assert_eq!(state.len(), 2, "two fingerprints recorded");

        ctx.pg().delete_scan_state_file(&fid, "a.rs").await.unwrap();
        let after = ctx.pg().list_scan_state(&fid).await.unwrap();
        assert_eq!(after.len(), 1);
        assert!(after.iter().all(|(p, _)| p != "a.rs"), "a.rs dropped, b.rs kept");
    }

    #[tokio::test]
    async fn unresolve_edges_to_file_nulls_target_keeps_name() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();
        let root_id = ctx.pg().add_watch_root(&repo_path, "ur", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "ur-repo", &repo_path).await.unwrap();

        // funcA lives in a.rs; funcB in b.rs calls it (resolved edge B → A).
        let node_a = ctx.pg().upsert_node(&fid, "function", "funcA", "a.rs", None, None, None, None).await.unwrap();
        let node_b = ctx.pg().upsert_node(&fid, "function", "funcB", "b.rs", None, None, None, None).await.unwrap();
        ctx.pg().insert_edge(&fid, &node_b, Some(&node_a), Some("funcA"), "calls").await.unwrap();

        // Re-indexing a.rs un-resolves inbound edges instead of letting the
        // cascade delete them: target_id cleared, target_name preserved.
        let n = ctx.pg().unresolve_edges_to_file(&fid, "a.rs").await.unwrap();
        assert_eq!(n, 1, "the one inbound edge should be un-resolved");

        let edges = ctx.pg().get_edges_by_kind(&fid, "calls").await.unwrap();
        assert_eq!(edges.len(), 1);
        assert!(edges[0]["target_id"].is_null(), "target_id cleared");
        assert_eq!(edges[0]["target_name"].as_str(), Some("funcA"), "target_name preserved for re-resolution");
    }

    #[tokio::test]
    async fn delete_file_succeeds() {
        let ctx = make_ctx().await;
        let folder_name = "test-repo";

        // Register a project
        {
            let root_id = ctx.pg().add_watch_root("/tmp/test", "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, folder_name, "/tmp/test").await.unwrap();
        }

        let task = Task::new(TaskKind::DeleteFile, "/tmp/test", "/tmp/a.rs");
        let result = delete_file(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_folder_removes_module_and_child_nodes() {
        let ctx = make_ctx().await;
        let folder_name = "test-repo";
        let repo_path = "/tmp/myrepo";

        // Register project
        {
            let root_id = ctx.pg().add_watch_root(repo_path, "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, folder_name, repo_path).await.unwrap();
        }

        let task = Task::new(TaskKind::DeleteFolder, repo_path, "/tmp/myrepo/src");
        delete_folder(&ctx, &task).await.unwrap();
    }
}
