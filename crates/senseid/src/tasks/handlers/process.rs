//! Process phase: index repos, folders, and files; handle deletions.

use std::path::Path;
use std::time::Instant;
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
    // Timer for the queue activity's elapsed (was hardcoded 0.0 → SSE showed
    // +0.00s for every queue event). Local start, same idiom as scan_root.
    let start = Instant::now();

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

    // D6a: capture this folder's id now (pre_registered is moved below). The
    // `indexing` mark is written later, INSIDE the has_changes block, so it is
    // symmetric with the barrier's `indexed` write — a no-op re-scan of an
    // already-indexed folder is never downgraded. A folder not registered at
    // scan time (None) is simply not marked; it isn't in the DB, so there is
    // nothing to recover anyway.
    let this_folder_id: Option<uuid::Uuid> = pre_registered.as_ref()
        .and_then(|r| r["id"].as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

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

    // Find or create the project by its resolved name. Race-safe get-or-adopt
    // (advisory-locked per name) so concurrent scan workers resolving the same
    // name can't each mint a row — the select-then-insert race that produced the
    // 0-folder phantom project. `created` gates the one-time project_add event.
    let (project_id, created) = match ctx.pg().get_or_create_project_by_name(project_name).await {
        Ok((id, created)) => (id.to_string(), created),
        Err(e) => {
            // FAIL, don't fabricate: minting a synthetic `p-<name>` id here pushed a
            // phantom project into UI/SSE state that never matched a real row and
            // never reconciled. Abort this folder — the scan task retries next tick.
            tracing::error!(project = %project_name, error = %e, "get_or_create_project_by_name failed — aborting folder (will retry); NOT emitting a phantom project");
            return Err(e);
        }
    };
    if created {
        // Emit: project add (only when a new project row was actually minted).
        emit(crate::api::events::StateEvent::project_add(crate::api::events::ScanProject {
            id: project_id.clone(),
            name: project_name.to_string(),
            status: crate::api::events::ProjectStatus::Indexing,
            folders: vec![],
            auto_detected: true,
            confidence: crate::api::events::Confidence::High,
        }));
    }

    // A quasi-repo (non-git project root) is tagged so the UI can surface it as
    // provisional — the user can discard it or promote it (git init → re-scanned
    // as a real repo). The folder kind=standalone already marks the folder; this
    // marks the project. Idempotent (tag union).
    if is_quasi
        && let Ok(pid) = uuid::Uuid::parse_str(&project_id)
            && let Err(e) = ctx.pg().set_project_identity(&pid, None, None, &[], &["quasi-repo".to_string()]).await {
                tracing::warn!(project_id = %pid, error = %e, "set_project_identity (quasi-repo tag) failed");
            }

    // ── 4. Emit: folder add with stack + file count ──────────────────
    // Reuse the lookup we already did to derive folder_name. abs_path is
    // unique on sensei.folders so the row identifies this exact repo
    // (vs name which can collide across roots).
    let folder_by_path = pre_registered;
    // The folder's REAL DB id, or None when the row isn't registered yet. NEVER
    // fabricate an `f-<name>` id — that orphaned a folder card in UI state.
    let folder_uuid_str = folder_by_path.as_ref()
        .and_then(|f| f["id"].as_str().map(|s| s.to_string()));
    // Capture the project-root folder id + watch-root id now (Copy), before
    // `folder_by_path` is moved below; used later to materialize the subfolder
    // tree.
    let project_root_uuid = folder_by_path.as_ref().and_then(|f| crate::api::util::json_uuid(&f["id"]));
    let repo_root_uuid = folder_by_path.as_ref().and_then(|f| crate::api::util::json_uuid(&f["root_id"]));

    // Announce the folder only when we know its real id; otherwise skip the event
    // (it's picked up once the row is registered) rather than emit a fabricated id.
    if let Some(folder_uuid_str) = folder_uuid_str {
        emit(crate::api::events::StateEvent::folder_add(crate::api::events::ScanFolder {
            id: folder_uuid_str,
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
    } else {
        tracing::warn!(folder = %folder_name, path = %task.path,
            "folder not registered at process time — skipping folder_add (no fabricated id)");
    }

    // ── 5. Emit: activity queue ──────────────────────────────────────
    emit(crate::api::events::StateEvent::activity(crate::api::events::ActivityEvent::new(
        crate::api::events::ActivityLevel::Queue,
        &format!("{} · {} files queued · {}", folder_name, files_total, stack.join(", ")),
        start.elapsed().as_secs_f64(),
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
    if let (Some(fid), Ok(pid)) = (&folder_uuid, uuid::Uuid::parse_str(&project_id))
        && let Err(e) = ctx.pg().set_folder_project(fid, &pid, "primary", None).await {
            tracing::warn!(folder_id = %fid, project_id = %pid, error = %e, "set_folder_project failed");
        }

    // Record the indexed git branch in props.branch — preferred from the
    // BranchSwitch task that triggered this re-index, otherwise read from
    // .git/HEAD. Lets the UI show which branch is indexed and gives a later
    // switch the prior branch for context. (Quasi-repos have no HEAD → skipped.)
    if let Some(fid) = &folder_uuid {
        let branch = task.branch.clone()
            .or_else(|| crate::watcher::root_watcher::read_git_head(&format!("{}/.git/HEAD", task.path)));
        if let Some(br) = branch
            && let Err(e) = ctx.pg().set_folder_props(fid, &serde_json::json!({ "branch": br })).await {
                tracing::warn!(folder_id = %fid, branch = %br, error = %e, "set_folder_props (branch) failed");
            }
    }

    // Incremental index: load the prior per-file fingerprints `(mtime, hash)` so
    // the two-tier gate can (a) skip files whose mtime is unchanged without any
    // read/hash, (b) re-hash only the mtime-drifted candidates and skip
    // reindexing the ones whose content is byte-identical, (c) reindex genuine
    // edits + new files, and (d) drop files no longer on disk. The first index
    // sees an empty scan_state and processes everything, populating it. This is
    // what makes a frequent no-op reconcile near-free.
    let prior_state: std::collections::HashMap<String, (i64, String)> = match &folder_uuid {
        Some(fid) => ctx.pg().list_scan_state_full(fid).await.unwrap_or_default()
            .into_iter().map(|(p, m, h)| (p, (m, h))).collect(),
        None => std::collections::HashMap::new(),
    };

    // Detect workspace members
    let workspace_members = crate::config::detector::detect_workspace_members(repo_path);

    // Discover directories and enqueue folder tasks
    let exclude = build_globset();
    let mut dirs = std::collections::HashSet::new();

    // Walk all files to discover directories
    let walker = super::helpers::build_walker(repo_path).build();

    // Every indexable file the ignore rules leave VISIBLE, as (abs, rel). This is
    // collected from the walker itself, so `.gitignore` (including nested ones),
    // `.ignore`, the global gitignore and `.git/info/exclude` are all honoured.
    //
    // The `exclude` globset is applied only to DIRECTORY discovery below, not to
    // this list: spec/test files are deliberately indexed and flagged via
    // `nodes.is_test`, so the globset must not be used to drop them.
    let mut visible: Vec<(std::path::PathBuf, String)> = Vec::new();

    for entry in walker.flatten() {
        if !entry.path().is_file() { continue; }
        let rel = entry.path().strip_prefix(repo_path).unwrap_or(entry.path());
        let rel_str = rel.to_string_lossy().to_string();

        // Skip binary files and files without extensions
        let ext = entry.path().extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();
        if ext.is_empty() { continue; }
        if is_binary_ext(&ext) { continue; }

        visible.push((entry.path().to_path_buf(), rel_str.clone()));

        if exclude.is_match(&rel_str) { continue; }
        if let Some(parent) = entry.path().parent() {
            dirs.insert(parent.to_path_buf());
        }
    }

    // The working tree's indexable files with mtimes, keyed by abs path → rel path.
    //
    // Derived from the walker's own output rather than re-reading each discovered
    // directory. The previous `read_dir` pass re-enumerated every file in a
    // discovered dir and applied ONLY the extension checks, so it silently
    // re-admitted files the ignore rules had just excluded — any gitignored file
    // sitting in a directory that also held tracked files was indexed. Sourcing
    // from `visible` removes that asymmetry by construction: one enumeration, one
    // set of rules.
    let mut current_meta: std::collections::HashMap<std::path::PathBuf, String> = std::collections::HashMap::new();
    let mut current: Vec<(String, i64)> = Vec::new();
    for (abs, rel) in visible {
        // Keep the previous membership rule — only files under a discovered
        // (non-excluded) directory participate — so this change subtracts the
        // ignored files and nothing else.
        if !abs.parent().is_some_and(|p| dirs.contains(p)) { continue; }
        let mtime = super::helpers::file_mtime_ms(&abs).unwrap_or(0);
        current.push((rel.clone(), mtime));
        current_meta.insert(abs, rel);
    }
    // Diff against the last index with the two-tier gate. The injected hasher
    // reads+hashes ONLY the mtime-drifted candidates (an unchanged-mtime file is
    // never touched), so a no-op re-scan is stat-only. `plan.changed` needs
    // reindexing, `plan.touched` only needs its mtime refreshed, `plan.removed`
    // is gone from disk.
    let mut plan = super::scan_logic::plan_reindex(&current, &prior_state, |rel| {
        super::helpers::hash_file(&repo_path.join(rel))
    });

    // Touched-but-identical files: refresh the stored mtime so the cheap gate
    // hits next pass, but DON'T reindex — their nodes/embeddings are still valid.
    // (mtime drift with no content change: touch, checkout, branch-switch-to-same.)
    if let Some(ref fid) = folder_uuid {
        for (path, mtime, hash) in &plan.touched {
            if let Err(e) = ctx.pg().upsert_scan_state(fid, path, *mtime, hash).await {
                tracing::warn!(folder_id = %fid, file = %path, error = %e, "upsert_scan_state (touched refresh) failed");
            }
        }
    }

    // Files the per-file stage could never index — an unsupported/binary format,
    // or content that isn't UTF-8 source text — are FINGERPRINTED here together
    // with the reason, and dropped from `changed` so no doomed ProcessFile is
    // enqueued.
    //
    // This is what stops an infinite re-index loop. `process_file` returns early
    // for these files WITHOUT writing scan_state, and `plan_reindex` treats a
    // file with no prior row as changed — so an unfingerprinted skip is
    // re-enqueued on every reconcile, the folder never reaches `indexed`, and the
    // whole downstream pipeline (embeddings → deps → connections → communities)
    // re-runs every tick. Recording the fingerprint makes the mtime gate skip it
    // for free next pass, while keying the skip to the fingerprint keeps it
    // self-healing: fix the file (re-encode it) and it is re-indexed on its own.
    if let Some(ref fid) = folder_uuid {
        let skippable: Vec<_> = plan.changed.iter()
            .filter_map(|rel| {
                let abs = repo_path.join(rel);
                let ext = abs.extension().and_then(|e| e.to_str()).unwrap_or("");
                super::helpers::classify_unscannable(&abs, ext).map(|reason| (rel.clone(), reason))
            })
            .collect();
        for (rel, reason) in skippable {
            // Only fingerprint what we actually observed: if the file can't be
            // read we record nothing and leave it queued, rather than storing a
            // fingerprint we didn't measure.
            let Some((mtime, hash)) = super::helpers::file_fingerprint(&repo_path.join(&rel)) else {
                tracing::debug!(file = %rel, "unscannable file not fingerprinted (unreadable) — left queued");
                continue;
            };
            // Drop from `changed` only once the write succeeded, so a DB failure
            // retries next pass instead of silently losing the file.
            match ctx.pg().upsert_scan_state_skipped(fid, &rel, mtime, &hash, reason).await {
                Ok(()) => { plan.changed.remove(&rel); }
                Err(e) => tracing::warn!(folder_id = %fid, file = %rel, error = %e,
                    "upsert_scan_state (skip) failed — file stays queued for the next pass"),
            }
        }
    }

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

        let mut ft = Task::for_file(TaskKind::ProcessFolder, folder_path, &abs_dir)
            .with_parent(task.id);
        ft.module_id = Some(pkg_id);
        let folder_id = ctx.queue.enqueue(ft).await;

        let rel_dir_name = if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir.replace('\\', "/") };
        let mod_id = format!("mod:{}:{}", folder_name, rel_dir_name);
        for abs in changed_here {
            let file_task = Task::for_file(TaskKind::ProcessFile, folder_path, &abs.to_string_lossy())
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
            if let Err(e) = ctx.pg().unresolve_edges_to_file(fid, path).await {
                tracing::warn!(folder_id = %fid, file = %path, error = %e, "unresolve_edges_to_file (removed) failed");
            }
            if let Err(e) = ctx.pg().delete_nodes_by_file(fid, path).await {
                tracing::warn!(folder_id = %fid, file = %path, error = %e, "delete_nodes_by_file (removed) failed");
            }
            if let Err(e) = ctx.pg().delete_scan_state_file(fid, path).await {
                tracing::warn!(folder_id = %fid, file = %path, error = %e, "delete_scan_state_file (removed) failed");
            }
        }

        // Safety net (Bug 2): the incremental diff above only drops files that
        // were in scan_state. Nodes can outlive their scan_state row — an
        // interrupted index, or a moved dir the fs-watcher missed (e.g.
        // crates/hive-mind → crates/dojo-mind, leaving Hive* struct nodes at
        // vanished paths). `current` is the complete live working-tree file set,
        // so prune any indexed node whose file is no longer present. Idempotent.
        let live: std::collections::HashSet<String> =
            current.iter().map(|(rel, _)| rel.clone()).collect();
        let pruned = super::scan::prune_vanished(ctx.pg(), fid, &live).await;
        if pruned > 0 {
            tracing::info!(folder = %folder_name, pruned, "process_git_folder: pruned orphan nodes for vanished files");
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
            match ctx.pg()
                .upsert_subfolder(&root_uuid, &name, &rel, &abs, Some(&parent_id), Some(&pid))
                .await
            {
                Ok(fid) => { path_to_id.insert(dir, fid); }
                Err(e) => tracing::warn!(name = %name, rel = %rel, error = %e, "upsert_subfolder failed"),
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
    // Recovery (D6b/D6d): also re-drive the barrier for a folder left in a
    // NON-TERMINAL state — `failed` (a prior fatal file, D6c-trigger, possibly
    // since healed by a bounded retry) or `indexing` (a crash mid-scan). Without
    // this, a healed transient failure whose scan_state is now complete
    // (has_changes=false) would strand the folder at `failed` forever, since the
    // terminal barrier that flips it to `indexed` only runs when there are
    // changes. An `indexed`/`archived`/`discovered` folder with no changes is
    // left as-is (no spurious downgrade — the inc2 invariant).
    let non_terminal = if let Some(fid) = this_folder_id {
        matches!(
            ctx.pg().get_folder_status(&fid).await.ok().flatten().as_deref(),
            Some("failed") | Some("indexing")
        )
    } else {
        false
    };
    if has_changes || non_terminal {
        // D6a: mark the scan in-flight — a crash then leaves a recoverable
        // `indexing` state, and the barrier flips it to `indexed` on success.
        // Gated on has_changes||non_terminal so an unchanged already-`indexed`
        // folder is never downgraded to `indexing`, but a `failed`/`indexing`
        // folder is re-driven to recovery.
        if let Some(fid) = this_folder_id
            && let Err(e) = ctx.pg().update_folder_status(&fid, "indexing").await
        {
            tracing::warn!(error = %e, folder = %folder_name, "process_git_folder: mark indexing failed");
        }

        // Phase 7.1: no ResolveEdges pass — FQN edges resolve at emit (source_id →
        // target_id in process_file). ResolveLibs (the first barrier) blocks on the
        // file tasks directly; degree is recomputed at DetectCommunities (its sole
        // consumer, the terminal barrier).
        let libs_id = ctx.queue.enqueue(
            Task::new(TaskKind::ResolveLibs, folder_path, "")
                .with_parent(task.id)
                .blocked_by(all_file_task_ids.clone())
        ).await;

        let build_id = ctx.queue.enqueue(
            Task::new(TaskKind::BuildConnections, folder_path, "")
                .with_parent(task.id)
                .blocked_by(vec![libs_id])
        ).await;

        // D4.1: DetectCommunities is the TERMINAL barrier — chained after
        // BuildConnections so the whole edge set exists before detection, and it
        // is the sole writer of `indexed` (so `indexed` implies communities are
        // computed). Its atomic per-folder replace (D4.2) makes a re-detect of an
        // unchanged graph a no-op, so re-driving it on recovery is cheap.
        ctx.queue.enqueue(
            Task::new(TaskKind::DetectCommunities, folder_path, "")
                .with_parent(task.id)
                .blocked_by(vec![build_id])
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
                        // D5a: a nested git repo is a `subtree`, not a `git` root.
                        // upsert_repo_kind relabels an existing git/standalone row and
                        // preserves an existing subtree, so this converges regardless
                        // of whether scan_root discovered the nested repo first.
                        if let Err(e) = ctx.pg().upsert_repo_kind(&root_id, "subtree", &subtree_folder_name, subtree_path).await {
                            tracing::warn!(name = %subtree_folder_name, path = %subtree_path, error = %e, "upsert_repo_kind (subtree) failed");
                        }
                    }
                }

                for (name, subtree_path) in &subtrees {
                    // folder_path is an abs_path per the Task struct contract;
                    // the composite display name "{folder_name}:{name}" lives
                    // on sensei.folders.name (upserted above) and is read
                    // back by handlers via get_repo_by_path.
                    let sub_task = Task::new(TaskKind::ProcessGitFolder, subtree_path, subtree_path)
                        .with_parent(task.id);
                    let subtree_folder_name = format!("{}:{}", folder_name, name);
                    // Single-writer (D6e/W5): skip if this subtree is already being
                    // scanned, and log the skip so the guard is observable.
                    if ctx.queue.enqueue_unique(sub_task).await.is_some() {
                        tracing::info!("process_git_folder: enqueued subtree {} at {}", subtree_folder_name, subtree_path);
                    } else {
                        tracing::debug!("process_git_folder: subtree {} already in flight, skipped", subtree_folder_name);
                    }
                }
            }
        }
    }

    // Repo-level metadata + identity reconcile from README frontmatter
    // (filesystem-only, READ-ONLY). Extracted into reconcile_repo_identity so
    // the watcher can re-run it incrementally on a README change.
    if let Err(e) = reconcile_repo_identity(ctx, &task.path).await {
        tracing::warn!(path = %task.path, error = %e, "reconcile_repo_identity failed");
    }

    // Self-healing reconcile: first deterministically prune any name-duplicate
    // phantom (a 0-folder discovery dupe of a folder-bearing project — merged
    // into the survivor), then re-tag the genuinely orphaned discovery projects.
    if let Err(e) = ctx.pg().heal_duplicate_name_projects().await {
        tracing::warn!(error = %e, "heal_duplicate_name_projects failed");
    }
    if let Err(e) = ctx.pg().mark_orphaned_projects().await {
        tracing::warn!(error = %e, "mark_orphaned_projects failed");
    }

    tracing::info!(
        "process_git_folder: {} — {} dirs, {} changed files, {} touched (mtime-only), {} unchanged (stat-only), {} removed",
        folder_name, dirs.len(), all_file_task_ids.len(), plan.touched.len(), plan.unchanged, plan.removed.len()
    );
    Ok(all_file_task_ids.len() as u32)
}

// ── Reconcile identity ─────────────────────────────────────────────────────

/// Reconcile a project root's identity FROM its README frontmatter — folder
/// props (incl. the frontmatter snapshot), icons, project identity, role, and
/// folder_namespaces. Filesystem-READ-ONLY (it never writes the README, so it
/// can't trigger a file-change loop), idempotent, and additive. Shared by the
/// scan pipeline (process_git_folder) and the watcher's ReconcileRepoMetadata task.
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
    // frontmatter snapshot here is what reconcile_repo_metadata compares against to
    // suppress no-op re-reconciles.
    let meta = serde_json::json!({
        "icon": icon,
        "external_links": links.links,
        "summary": summary,
        "frontmatter": serde_json::to_value(&fm).unwrap_or(serde_json::Value::Null),
    });
    if let Err(e) = ctx.pg().set_folder_props(&folder_id, &meta).await {
        tracing::warn!(folder_id = %folder_id, error = %e, "set_folder_props (reconcile meta) failed");
    }

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
        if let Err(e) = ctx.pg().set_folder_icons(&folder_id, &icons).await {
            tracing::warn!(folder_id = %folder_id, error = %e, "set_folder_icons failed");
        }
    }

    // Project identity + role + namespaces (only when linked to a project).
    if let Some(pid) = folder["project_id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
        // Authoritative project name (matches what the scan created) for the
        // `project` namespace; fall back to frontmatter / folder name. Fetched
        // once and reused below for icon inference (name + current icon).
        let project = ctx.pg().get_project(&pid).await.ok().flatten();
        let project_name = project.as_ref()
            .and_then(|p| p["name"].as_str().map(String::from))
            .or_else(|| fm.project.clone())
            .or_else(|| folder["name"].as_str().map(String::from))
            .unwrap_or_default();

        let id_stack: Vec<String> =
            if fm.stack.is_empty() { stack.clone() } else { fm.stack.clone() };
        let mut tags: Vec<String> = Vec::new();
        if let Some(role) = fm.role.as_deref() {
            // Keep the raw role as a project tag (lossless).
            tags.push(format!("role:{role}"));
        }
        // folder.role enum: explicit README frontmatter wins; otherwise infer it
        // from the folder's manifest + layout so monorepo members are classified
        // automatically (library / tool / website). See `role_reconciliation` for
        // the write-vs-skip decision that reconciles stale pre-refactor rows.
        let folder_role = fm.role.as_deref()
            .and_then(metadata::folder_role_from_frontmatter)
            .or_else(|| super::scan_logic::infer_role(repo_path));
        if let Some(role_arg) = role_reconciliation(folder_role, fm.role.as_deref()) {
            ctx.pg().update_folder_role(&folder_id, role_arg).await
                .unwrap_or_else(|e| tracing::warn!(folder_id = %folder_id, error = %e, "update_folder_role failed"));
        }
        if let Some(org) = fm.organization.as_deref() {
            tags.push(format!("org:{}", metadata::slugify(org)));
        }
        if let Err(e) = ctx.pg().set_project_identity(
            &pid, fm.summary.as_deref(), fm.client.as_deref(), &id_stack, &tags,
        ).await {
            tracing::warn!(project_id = %pid, error = %e, "set_project_identity (reconcile) failed");
        }

        // Deterministic project-icon inference — fills the generic 場 fallback
        // so the project card shows something recognisable (repo logo /
        // kanji-from-stack / letter initial). Never overrides an author choice;
        // only upgrades a prior machine icon; non-fatal. [[pipeline/project-icon]].
        //
        // The logo tier is LIVE: the scanned repo-relative asset path
        // (`icon.path`, from `scan_icons` above) is passed through so a detected
        // logo wins as `{kind:"image", value:<rel path>}`. The daemon serves the
        // bytes at `GET /api/projects/{id}/icon` (path-safety in
        // `analysis::project_icon::read_icon_bytes`), and the app renders it with
        // a kanji fallback on image error.
        use crate::analysis::project_icon::{infer_icon, IconDecision};
        let logo_paths: Vec<String> = icon.path.iter().cloned().collect();
        let existing_icon = project.as_ref()
            .map(|p| p["icon"].clone())
            .unwrap_or(serde_json::Value::Null);
        if let IconDecision::Set(inferred) =
            infer_icon(&project_name, &id_stack, &existing_icon, &logo_paths)
            && let Err(e) = ctx.pg()
                .set_project_icon(&pid, &serde_json::to_value(&inferred).unwrap_or(serde_json::Value::Null))
                .await
        {
            tracing::warn!(project_id = %pid, error = %e, "set_project_icon (reconcile) failed");
        }

        let mut ns: Vec<(&str, String)> = Vec::new();
        if let Some(org) = fm.organization.as_deref() { ns.push(("organization", org.to_string())); }
        if !project_name.is_empty() { ns.push(("project", project_name.clone())); }
        if let Some(team) = fm.team.as_deref() { ns.push(("team", team.to_string())); }
        for lang in &id_stack { ns.push(("technology", lang.clone())); }
        for (scope, name) in &ns {
            let slug = metadata::slugify(name);
            if slug.is_empty() { continue; }
            if let Ok(ns_id) = ctx.pg().upsert_namespace(scope, name, &slug).await {
                ctx.pg().link_folder_namespace(&folder_id, &ns_id).await
                    .unwrap_or_else(|e| tracing::warn!(folder_id = %folder_id, ns_id = %ns_id, error = %e, "link_folder_namespace failed"));
            }
        }
    }

    // Sub-project roles: classify each nested sub-project (declared workspace
    // members and standalone sub-apps like a `site/`) so a monorepo's packages,
    // crates and apps are individually typed (library / tool / website). Only
    // runs for monorepo roots — a single-package repo has nothing nested to
    // find. Role assignment is independent of project membership.
    if super::scan_logic::is_monorepo(repo_path)
        && let Some(root_id) = crate::api::util::json_uuid(&folder["root_id"])
    {
        let project_id = folder["project_id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok());
        for sub in super::scan_logic::find_subprojects(repo_path, 3) {
            let Some(role) = super::scan_logic::infer_role(&sub) else { continue };
            let sub_abs = sub.to_string_lossy().to_string();
            let rel = sub.strip_prefix(repo_path).unwrap_or(&sub).to_string_lossy().to_string();
            let name = sub.file_name().and_then(|n| n.to_str()).unwrap_or(rel.as_str()).to_string();
            // D5a: a monorepo sub-project is a `workspace_member` (not a plain
            // structural `folder`) — its own boundary in the graph, keeping the
            // inferred role. The kind-aware upsert relabels an existing `folder`
            // member but never reclassifies a nested project root.
            match ctx.pg().upsert_subfolder_kind(
                &root_id, "workspace_member", &name, &rel, &sub_abs, Some(&folder_id), project_id.as_ref(),
            ).await {
                Ok(sub_id) => {
                    if let Err(e) = ctx.pg().update_folder_role(&sub_id, Some(role)).await {
                        tracing::warn!(sub = %sub_abs, error = %e, "sub-project update_folder_role failed");
                    }
                }
                Err(e) => tracing::warn!(sub = %sub_abs, error = %e, "sub-project upsert_subfolder_kind failed"),
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
pub async fn reconcile_repo_metadata(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    use crate::tasks::processors::metadata;
    let repo_path = Path::new(&task.path);

    let fresh = serde_json::to_value(metadata::read_frontmatter(repo_path).unwrap_or_default())
        .unwrap_or(serde_json::Value::Null);
    let stored = ctx.pg().get_repo_by_path(&task.path).await.ok().flatten()
        .and_then(|f| f.get("props").and_then(|p| p.get("frontmatter")).cloned());
    if stored.as_ref() == Some(&fresh) {
        tracing::debug!("reconcile_repo_metadata: {} — frontmatter unchanged, skipping", task.path);
        return Ok(0);
    }

    tracing::info!("reconcile_repo_metadata: {} — frontmatter changed, reconciling", task.path);
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
        if let Err(e) = ctx.pg().upsert_node(fid, "module", &mod_name, &task.path, None, None, None, None).await {
            tracing::warn!(folder_id = %fid, module = %mod_name, error = %e, "upsert_node (module) failed");
        }
    }

    Ok(0)
}

// ── Process File ──────────────────────────────────────────────────────────

/// Parse a single file using file_processor, then write results to graph.
/// Test-only fault seam (D6c-trigger): lets a test force a fatal DB-write
/// failure for a specific file path, so the fatal path (folder → `failed`,
/// `Err`, no `scan_state` advance) is exercised without needing a live DB fault.
#[cfg(test)]
pub(super) mod fault {
    use std::collections::HashSet;
    use std::sync::Mutex;

    static FAIL_PATHS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

    /// Force the next `process_file` on `abs_path` to hit the fatal path.
    pub fn fail_for(abs_path: &str) {
        FAIL_PATHS.lock().unwrap().get_or_insert_with(HashSet::new).insert(abs_path.to_string());
    }
    /// Stop forcing failure for `abs_path`.
    pub fn clear(abs_path: &str) {
        if let Some(set) = FAIL_PATHS.lock().unwrap().as_mut() {
            set.remove(abs_path);
        }
    }
    /// Whether `abs_path` is currently marked to fail.
    pub fn should_fail(abs_path: &str) -> bool {
        FAIL_PATHS.lock().unwrap().as_ref().is_some_and(|s| s.contains(abs_path))
    }
}

pub async fn process_file(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let abs_path = &task.path;

    // Skip files we can't parse as source text. Returning Ok (not Err) is
    // critical: a failed ProcessFile task would block its folder's
    // post-processing barrier, leaving the folder stuck at 'discovered'. Binary
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
    // blocks the folder's post-processing barrier.
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
            // UI) instead of silently dropping it. If THAT write also fails,
            // surface the second failure so an operator can see the DB is unhappy;
            // otherwise the parse-error observability itself becomes silent.
            if let Some(fid) = &folder_id
                && let Err(log_err) = ctx.pg().log_index_error(fid, abs_path, &e, Some(ext), Some("parse")).await
            {
                tracing::warn!(error = %log_err, path = %abs_path, "log_index_error failed for parse error");
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
            if let Some(fid) = &folder_id
                && let Err(log_err) = ctx.pg().log_index_error(fid, abs_path, &msg, Some(ext), Some("parse")).await
            {
                tracing::warn!(error = %log_err, path = %abs_path, "log_index_error failed for parser panic");
            }
            tracing::warn!("process_file: {abs_path}: {msg}");
            return Ok(0);
        }
    };

    // Write parsed symbols to PG. A DB-write failure here is FATAL (D6c-trigger):
    // the file isn't correctly indexed, so we must NOT advance its scan_state and
    // must surface it — mark the folder `failed` (the fail-closed barrier D6d
    // checks this) and propagate `Err` (recorded to task_executions and
    // bounded-retried, D6c). Parse/read errors were TOLERATED above (Ok). A
    // folder row that doesn't exist yet is a no-op.
    let symbols_count = result.symbols.len();
    let Some(folder_id) = folder_id else {
        return Ok(symbols_count as u32);
    };

    // Test seam (D6c-trigger): exercise the fatal path without a live DB fault.
    #[cfg(test)]
    if fault::should_fail(abs_path) {
        return fail_folder(ctx, &folder_id, &result.rel_path,
            "injected fatal DB-write failure (test fault seam)".to_string()).await;
    }

    // Any DB write in here failing is fatal — `?` propagates it out of the async
    // block and we handle it uniformly below (no partial "success").
    let write_result: Result<(), String> = async {
        // D3 upsert-then-prune: UPSERT the file's current nodes (surviving symbols
        // keep their id → community_id/embedding/inbound edges), then prune the
        // ones that vanished from the parse. No destructive delete-then-insert.

        // File node.
        let file_node_id = ctx.pg().upsert_node(
            &folder_id, &result.kind, &result.rel_path, &result.rel_path, None, None, None, None
        ).await.map_err(|e| format!("upsert file node: {e}"))?;

        // Symbol nodes (functions, classes, types, …), captured by (name,
        // line_start) so call edges can be sourced from the caller node — not the
        // file. Keyed on line because same-named methods across impl blocks are
        // legal in Rust.
        // The Rust FQN path (result.fqn) get-or-creates each symbol node keyed on
        // its canonical FQN — so a definition and every reference to it share one
        // node — via `upsert_node_by_fqn`. Every other language keeps the line-based
        // bare-name path (`upsert_node_ex`). `fqn_ids` maps fqn→id for edge sourcing.
        let mut sym_ids: std::collections::HashMap<(String, i32), uuid::Uuid> =
            std::collections::HashMap::new();
        let mut fqn_ids: std::collections::HashMap<String, uuid::Uuid> =
            std::collections::HashMap::new();
        if let Some(fqn_out) = &result.fqn {
            // D5c: a `module` container per file (nested under the file). Top-level
            // items nest under it (or the file node at the crate root); methods nest
            // under their TYPE node — so the graph is file → module → type → method.
            // The `nodes.language` column for every FQN node is the FILE's language
            // (derived from its extension) — NOT the fqn's grouping lang and NOT a
            // hardcoded "rust". Keeps the same-language fallback (0.8) honest across
            // the migrated languages.
            let file_lang = crate::languages::language_for_path(&result.rel_path);
            let mut top_parent = file_node_id;
            if !fqn_out.module.is_empty() {
                // The module container's fqn language matches the file's defs (the
                // first fqn's leading segment), so a Python/TS module node isn't
                // mislabelled as rust.
                //
                // When the parse yields a module path but NO top-level defs there is
                // no leading segment to copy, so fall back to the FILE's language
                // rather than a hardcoded "rust" — that fallback minted
                // `rust·<pkg>·lib/components/Foo` for a .svelte file, which is both
                // wrong (it corrupts the same-language scoring the `language` column
                // feeds) and unstable: the fqn flipped as soon as defs reappeared.
                // Only `adopt_node_by_identity` keeps such a flip from wedging the
                // file forever, so don't rely on it — emit a stable value here.
                let lang = fqn_out.defs.first()
                    .and_then(|d| d.fqn.split('·').next())
                    .filter(|seg| !seg.is_empty())
                    .or(file_lang)
                    .unwrap_or("rust");
                let mfqn = crate::languages::fqn::item(lang, &fqn_out.package, "", &fqn_out.module);
                let mname = fqn_out.module.rsplit("::").next().unwrap_or(&fqn_out.module);
                let mid = ctx.pg().upsert_node_by_fqn(
                    &folder_id, &mfqn, "module", mname, file_lang,
                    Some(crate::db::pg_store::FqnDef {
                        file_path: &result.rel_path, signature: None, line_start: None,
                        line_end: None, is_exported: false, parent_id: Some(&file_node_id),
                    }),
                ).await.map_err(|e| format!("upsert module node {mfqn}: {e}"))?;
                fqn_ids.insert(mfqn, mid);
                top_parent = mid;
            }
            for d in &fqn_out.defs {
                let kind = crate::types::NodeKind::from_symbol_kind(&d.kind);
                // Structural parent: a method → its enclosing type node (get-or-create
                // — a stub if the type is defined in another file); a top-level item →
                // the module container (or the file at crate root).
                let parent_id: uuid::Uuid = match &d.parent_fqn {
                    Some(pf) => match fqn_ids.get(pf) {
                        Some(id) => *id,
                        None => ctx.pg().upsert_node_by_fqn(
                            &folder_id, pf, "class", pf.rsplit('·').next().unwrap_or(pf), file_lang, None,
                        ).await.map_err(|e| format!("upsert fqn parent {pf}: {e}"))?,
                    },
                    None => top_parent,
                };
                let id = ctx.pg().upsert_node_by_fqn(
                    &folder_id, &d.fqn, kind.as_str(), &d.name, file_lang,
                    Some(crate::db::pg_store::FqnDef {
                        file_path: &result.rel_path,
                        signature: d.signature.as_deref(),
                        line_start: Some(d.line_start as i32),
                        line_end: Some(d.line_end as i32),
                        is_exported: d.is_exported,
                        parent_id: Some(&parent_id),
                    }),
                ).await.map_err(|e| format!("upsert fqn def {}: {e}", d.fqn))?;
                fqn_ids.insert(d.fqn.clone(), id);
            }
        } else {
            for sym in &result.symbols {
                let id = ctx.pg().upsert_node_ex(
                    &folder_id, &sym.kind, &sym.name, &result.rel_path,
                    Some(&file_node_id), sym.signature.as_deref(),
                    Some(sym.line as i32), Some(sym.line_end as i32), sym.is_exported,
                ).await.map_err(|e| format!("upsert symbol node {}: {e}", sym.name))?;
                sym_ids.insert((sym.name.clone(), sym.line as i32), id);
            }
        }

        // D5b: nested doc `section` nodes (file → H1 → H2 → H3). Identity is the
        // full heading PATH ("Design > Auth > Refresh") with a NULL `line_start`,
        // so a section keeps its id across line edits (line-independent identity,
        // 0.4) — the real line + level live in `props`. Written through this same
        // upsert/prune path, so a re-index reconciles the section set (a removed
        // heading is pruned, no duplicates). Empty for code files.
        let mut section_ids: Vec<uuid::Uuid> = Vec::with_capacity(result.sections.len());
        // Stack of (level, heading_segment, node_id) for the current ancestor chain.
        let mut path_stack: Vec<(u8, String, uuid::Uuid)> = Vec::new();
        // Disambiguate identical-text siblings under the same parent: the Nth
        // (N>1) occurrence of a full heading path gets a " #N" suffix, so two
        // `## Setup` under one H1 are DISTINCT nodes rather than the second's
        // upsert colliding onto the first (which would silently clobber it). The
        // suffix flows into the stacked segment, so children of the second Setup
        // ("… > Setup #2 > …") don't collide with children of the first either.
        // Deterministic per document ⇒ idempotent on re-index.
        let mut seen_paths: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for sec in &result.sections {
            while path_stack.last().is_some_and(|(lvl, _, _)| *lvl >= sec.level) {
                path_stack.pop();
            }
            let parent_id = path_stack.last().map(|(_, _, id)| *id).unwrap_or(file_node_id);
            let base_path = path_stack.iter()
                .map(|(_, h, _)| h.as_str())
                .chain(std::iter::once(sec.heading.as_str()))
                .collect::<Vec<_>>()
                .join(" > ");
            let occ = { let c = seen_paths.entry(base_path.clone()).or_insert(0); *c += 1; *c };
            let (heading_path, segment) = if occ == 1 {
                (base_path, sec.heading.clone())
            } else {
                (format!("{base_path} #{occ}"), format!("{} #{occ}", sec.heading))
            };
            let sec_id = ctx.pg().upsert_node(
                &folder_id, "section", &heading_path, &result.rel_path,
                Some(&parent_id), None, None, None,
            ).await.map_err(|e| format!("upsert section node {}: {e}", heading_path))?;
            let props = serde_json::json!({
                "level": sec.level,
                "line_start": sec.line_start,
                "line_end": sec.line_end,
                "preview": sec.content_preview,
            });
            ctx.pg().set_node_props(&sec_id, &props).await
                .map_err(|e| format!("set section props {}: {e}", heading_path))?;
            section_ids.push(sec_id);
            // Stack holds each heading's OWN (disambiguated) segment so the next
            // child's path joins ancestors + itself exactly once, carrying the suffix.
            path_stack.push((sec.level, segment, sec_id));
        }

        // D5b: rationale nodes (NOTE/WHY/HACK/TODO/IMPORTANT) — the design "why".
        // Parented to the file node (finer function/section parenting is a
        // follow-up), keyed on (name=text, line_start) so identical markers on
        // different lines are distinct and a re-index of unchanged text is a no-op.
        let mut rationale_ids: Vec<uuid::Uuid> = Vec::with_capacity(result.rationales.len());
        for r in &result.rationales {
            let id = ctx.pg().upsert_node(
                &folder_id, "rationale", &r.text, &result.rel_path,
                Some(&file_node_id), None, Some(r.line as i32), Some(r.line as i32),
            ).await.map_err(|e| format!("upsert rationale node: {e}"))?;
            ctx.pg().set_node_props(&id, &serde_json::json!({ "marker": r.marker })).await
                .map_err(|e| format!("set rationale props: {e}"))?;
            rationale_ids.push(id);
        }

        // Everything just upserted is this file's current node set (its source
        // nodes for out-edges too). `kept` is never empty — the file node always
        // survives.
        let kept: Vec<uuid::Uuid> = std::iter::once(file_node_id)
            .chain(sym_ids.values().copied())
            .chain(fqn_ids.values().copied())
            .chain(section_ids.iter().copied())
            .chain(rationale_ids.iter().copied())
            .collect();

        // D3 prune: delete the file's nodes that vanished from the parse (their
        // out-edges cascade). Inbound FQN edges (target_id set) cascade-delete with
        // the node — the demote-to-stub refinement (plan 0.5) is a deferred
        // follow-up; a full reindex heals a removed-but-referenced def.
        ctx.pg().prune_file_nodes(&folder_id, &result.rel_path, &kept).await
            .map_err(|e| format!("prune_file_nodes: {e}"))?;

        // D2/D3 per-file out-edge reconcile: a SURVIVING node keeps its id, so its
        // stale out-edges (a call/import a re-edit removed) don't cascade — clear
        // this file's out-edges, then re-insert the current set below (replace,
        // not append).
        ctx.pg().delete_edges_from_sources(&folder_id, &kept).await
            .map_err(|e| format!("delete_edges_from_sources: {e}"))?;

        // Unresolved import edges.
        for import in &result.unresolved_imports {
            ctx.pg().insert_edge(&folder_id, &file_node_id, None, Some(import), None, "imports").await
                .map_err(|e| format!("insert_edge (imports): {e}"))?;
        }

        // Call edges. The FQN path emits RESOLVED node→node edges AT EMIT: the
        // target is get-or-created by FQN (a stub if its definition isn't indexed
        // yet — enriched later, keeping the same id; a `lib_symbol` for an external
        // crate). An out-of-0.7 receiver (unresolvable) or an un-migrated language
        // keeps an honest bare-name edge (target_name only) — the `dyn`/residual
        // tail. Phase 7.1 retired the resolve_edges fallback, so these stay
        // unresolved rather than being bare-name-matched to an arbitrary node.
        if let Some(fqn_out) = &result.fqn {
            let file_lang = crate::languages::language_for_path(&result.rel_path);
            for r in &fqn_out.refs {
                let source = fqn_ids.get(&r.caller_fqn).copied().unwrap_or(file_node_id);
                match &r.target_fqn {
                    Some(tf) if r.is_lib => {
                        let pkg = tf.split('·').nth(1).unwrap_or("");
                        let tid = ctx.pg().upsert_lib_node_by_fqn(&folder_id, tf, &r.target_name, pkg).await
                            .map_err(|e| format!("upsert lib node {tf}: {e}"))?;
                        ctx.pg().insert_edge(&folder_id, &source, Some(&tid), None, None, "calls").await
                            .map_err(|e| format!("insert_edge (fqn call, lib): {e}"))?;
                    }
                    Some(tf) => {
                        // Target defined in THIS file → reuse its id; else get-or-create a stub.
                        let tid = match fqn_ids.get(tf) {
                            Some(id) => *id,
                            None => ctx.pg().upsert_node_by_fqn(&folder_id, tf, "function", &r.target_name, file_lang, None).await
                                .map_err(|e| format!("upsert fqn target {tf}: {e}"))?,
                        };
                        ctx.pg().insert_edge(&folder_id, &source, Some(&tid), None, None, "calls").await
                            .map_err(|e| format!("insert_edge (fqn call): {e}"))?;
                    }
                    None => {
                        ctx.pg().insert_edge(&folder_id, &source, None, Some(&r.target_name), None, "calls").await
                            .map_err(|e| format!("insert_edge (fqn call, unresolved): {e}"))?;
                    }
                }
            }
        } else {
            for call in &result.unresolved_calls {
                let source = sym_ids
                    .get(&(call.caller_name.clone(), call.caller_line as i32))
                    .copied()
                    .unwrap_or(file_node_id);
                ctx.pg().insert_edge(&folder_id, &source, None, Some(&call.callee_name), None, "calls").await
                    .map_err(|e| format!("insert_edge (calls): {e}"))?;
            }
        }

        // Parent refs (HAS_METHOD: type → method).
        for pref in &result.parent_refs {
            ctx.pg().insert_edge(&folder_id, &file_node_id, None, Some(&pref.parent_name), None, "extends").await
                .map_err(|e| format!("insert_edge (extends): {e}"))?;
        }

        // Doc references (D2): an explicit doc→file path ref AND a doc→symbol
        // mention are both `references` edges — per the edge_kind contract
        // ("doc section references a symbol or file"). `covers` is reserved for
        // BuildConnections' folder-derived stem-proximity set, which it REPLACES
        // wholesale; a doc→file ref must not be `covers` or that replace would
        // wipe it (the two-producer data-loss D2 review caught).
        if result.kind == "doc" {
            for file_ref in &result.file_refs {
                ctx.pg().insert_edge(&folder_id, &file_node_id, None, Some(file_ref), None, "references").await
                    .map_err(|e| format!("insert_edge (references, file): {e}"))?;
            }
            for fn_ref in &result.fn_mentions {
                ctx.pg().insert_edge(&folder_id, &file_node_id, None, Some(fn_ref), None, "references").await
                    .map_err(|e| format!("insert_edge (references, symbol): {e}"))?;
            }
        }

        // is_test: a FILE-level flag stamped on every one of this file's nodes so
        // the UI can filter tests out when focusing on production code. Set after
        // emit (all nodes exist); IS DISTINCT FROM makes a no-op re-scan cheap and
        // a test↔prod rename flips the file's nodes.
        let is_test = crate::languages::is_test_path(
            &result.rel_path,
            crate::languages::language_for_path(&result.rel_path),
        );
        ctx.pg().set_nodes_is_test_for_file(&folder_id, &result.rel_path, is_test).await
            .map_err(|e| format!("set_nodes_is_test_for_file: {e}"))?;
        Ok::<(), String>(())
    }.await;

    if let Err(e) = write_result {
        return fail_folder(ctx, &folder_id, &result.rel_path, e).await;
    }

    // Record this file's fingerprint LAST — only a fully-written file is "seen",
    // so a fatal failure above leaves scan_state unadvanced and the next scan
    // retries it. A scan_state write failure is itself fatal.
    if let Some((mtime, hash)) = super::helpers::file_fingerprint(fpath)
        && let Err(e) = ctx.pg().upsert_scan_state(&folder_id, &result.rel_path, mtime, &hash).await {
            return fail_folder(ctx, &folder_id, &result.rel_path, format!("upsert_scan_state: {e}")).await;
        }

    Ok(symbols_count as u32)
}

/// Mark a folder `failed` and return the fatal error (D6c-trigger / D6a): a DB
/// write for one of its files failed, so the folder must not advance to
/// `indexed` (the fail-closed barrier D6d checks this status) and boot-reconcile
/// / bounded-retry re-drives it. Marking the status is best-effort — if THAT
/// write also fails we still surface the original fatal error, never swallow it.
async fn fail_folder(
    ctx: &TaskContext,
    folder_id: &uuid::Uuid,
    rel_path: &str,
    err: String,
) -> Result<u32, String> {
    if let Err(se) = ctx.pg().update_folder_status(folder_id, "failed").await {
        tracing::warn!(error = %se, folder_id = %folder_id, "process_file: marking folder failed also failed");
    }
    tracing::warn!(folder_id = %folder_id, file = %rel_path, error = %err,
        "process_file: fatal DB write — folder left `failed`, scan_state not advanced");
    Err(format!("process_file fatal DB write ({rel_path}): {err}"))
}

// ── Delete File / Folder ──────────────────────────────────────────────────

pub async fn delete_file(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    // folder_path is the repo abs_path (Task contract).
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"])
            && let Err(e) = ctx.pg().delete_nodes_by_file(&folder_id, &task.path).await {
                tracing::warn!(folder_id = %folder_id, file = %task.path, error = %e, "delete_nodes_by_file (delete_file) failed");
            }
    tracing::info!("delete_file: {}", task.path);
    Ok(0)
}

pub async fn delete_folder(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"])
            && let Err(e) = ctx.pg().delete_nodes_by_path_prefix(&folder_id, &task.path).await {
                tracing::warn!(folder_id = %folder_id, path = %task.path, error = %e, "delete_nodes_by_path_prefix (delete_folder) failed");
            }
    tracing::info!("delete_folder: {}", task.path);
    Ok(0)
}

/// Decide what to write to a folder's role column given the classifier's
/// output and the frontmatter's raw `role:` value.
///
/// Reconciles stale rows from before the classifier refactor (#8): the old
/// classifier had a `backend` fallback and left rows tagged `backend`
/// wherever it could not infer a real role. The new classifier returns
/// `None` for un-inferrable folders, but the writer used to skip those,
/// leaving the stale `backend` in place forever. This helper makes the
/// silent-frontmatter + no-classification case actively clear the DB.
///
/// - `classified`: the classifier's decision (`None` means "cannot classify").
/// - `raw_frontmatter_role`: whatever the README's `role:` field literally said,
///   before it was mapped through `folder_role_from_frontmatter`. `Some("")`
///   still counts as "the user wrote something" — only true `None` means the
///   frontmatter is silent.
///
/// Returns `Some(role_arg)` when we should call `update_folder_role`, where
/// `role_arg` may itself be `Some(&str)` (write that value) or `None` (clear
/// the DB column). Returns outer `None` to skip the write entirely — used
/// when the user wrote an unrecognised role that we neither map nor override.
pub fn role_reconciliation<'a>(
    classified: Option<&'a str>,
    raw_frontmatter_role: Option<&str>,
) -> Option<Option<&'a str>> {
    match (classified, raw_frontmatter_role) {
        (Some(fr), _) => Some(Some(fr)),
        (None, None) => Some(None),
        (None, Some(_)) => None,
    }
}

#[cfg(test)]
mod role_reconciliation_tests {
    use super::role_reconciliation;

    #[test]
    fn writes_classifier_result_when_classified() {
        assert_eq!(role_reconciliation(Some("library"), None), Some(Some("library")));
    }

    #[test]
    fn writes_classifier_result_even_when_frontmatter_says_something_else() {
        // Frontmatter took precedence upstream (folder_role_from_frontmatter
        // returned the mapped value). If we got here with a classified value,
        // trust it — the caller already resolved precedence.
        assert_eq!(
            role_reconciliation(Some("website"), Some("backend")),
            Some(Some("website")),
        );
    }

    #[test]
    fn clears_stale_value_when_frontmatter_silent_and_no_classification() {
        // The #8 fix: a rescan of a folder with no manifest signals AND no
        // frontmatter role must clear the DB, not skip. Otherwise pre-refactor
        // "backend" rows persist forever.
        assert_eq!(role_reconciliation(None, None), Some(None));
    }

    #[test]
    fn preserves_db_when_frontmatter_has_unrecognised_role() {
        // The user wrote `role: platform` (or similar not-mapped value). We
        // don't know if the DB already holds their previous choice — leave it
        // alone rather than clobber.
        assert_eq!(role_reconciliation(None, Some("platform")), None);
        assert_eq!(role_reconciliation(None, Some("")), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    
    use crate::tasks::{Task, TaskKind};
    
    use super::super::super::executor::TaskContext;

    /// Build a TaskContext backed by PgStore and a fresh TaskQueue.
    use crate::tasks::test_support::make_ctx;

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

        let mut task = Task::for_file(TaskKind::ProcessFolder, &repo_path, &src_dir.to_string_lossy());
        task.module_id = Some(pkg_id.clone());

        process_folder(&ctx, &task).await.unwrap();

        // TODO: verify module node once module writes are implemented
    }

    /// Reconciling a monorepo git root classifies each nested sub-project with
    /// its own folder.role: a lib crate → library, a bin crate → tool, and a
    /// (non-member) SvelteKit sub-app → website. Guards the end-to-end wiring
    /// (is_monorepo → find_subprojects → upsert_subfolder → update_folder_role).
    #[tokio::test]
    async fn reconcile_classifies_monorepo_member_roles() {
        async fn role_of(ctx: &TaskContext, abs: &str) -> Option<String> {
            let row: Option<(Option<String>,)> = sqlx_core::query_as::query_as(
                "SELECT role::text FROM sensei.folders WHERE abs_path = $1",
            ).bind(abs).fetch_optional(ctx.pg().pool()).await.unwrap();
            row.and_then(|r| r.0)
        }
        async fn kind_of(ctx: &TaskContext, abs: &str) -> Option<String> {
            let row: Option<(String,)> = sqlx_core::query_as::query_as(
                "SELECT kind::text FROM sensei.folders WHERE abs_path = $1",
            ).bind(abs).fetch_optional(ctx.pg().pool()).await.unwrap();
            row.map(|r| r.0)
        }

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers=[\"crates/*\"]").unwrap();
        std::fs::create_dir_all(root.join("crates/mylib/src")).unwrap();
        std::fs::write(root.join("crates/mylib/Cargo.toml"), "[package]\nname=\"mylib\"").unwrap();
        std::fs::write(root.join("crates/mylib/src/lib.rs"), "pub fn a() {}").unwrap();
        std::fs::create_dir_all(root.join("crates/mytool/src")).unwrap();
        std::fs::write(root.join("crates/mytool/Cargo.toml"), "[package]\nname=\"mytool\"\n\n[[bin]]\nname=\"mytool\"").unwrap();
        std::fs::write(root.join("crates/mytool/src/main.rs"), "fn main() {}").unwrap();
        std::fs::create_dir_all(root.join("site/src/routes")).unwrap();
        std::fs::write(root.join("site/package.json"), "{\"name\":\"site\",\"devDependencies\":{\"@sveltejs/kit\":\"^2\"}}").unwrap();

        let ctx = make_ctx().await;
        let repo_path = root.to_string_lossy().to_string();
        let root_id = ctx.pg().add_watch_root(&repo_path, "mono", &serde_json::json!([])).await.unwrap();
        ctx.pg().upsert_repo_kind(&root_id, "git", "mono", &repo_path).await.unwrap();

        reconcile_repo_identity(&ctx, &repo_path).await.unwrap();

        assert_eq!(role_of(&ctx, &root.join("crates/mylib").to_string_lossy()).await.as_deref(), Some("library"));
        assert_eq!(role_of(&ctx, &root.join("crates/mytool").to_string_lossy()).await.as_deref(), Some("tool"));
        assert_eq!(role_of(&ctx, &root.join("site").to_string_lossy()).await.as_deref(), Some("website"));

        // D5a: each sub-project is classified `workspace_member` (not a plain
        // structural `folder`), keeping its inferred role.
        assert_eq!(kind_of(&ctx, &root.join("crates/mylib").to_string_lossy()).await.as_deref(), Some("workspace_member"));
        assert_eq!(kind_of(&ctx, &root.join("crates/mytool").to_string_lossy()).await.as_deref(), Some("workspace_member"));
        assert_eq!(kind_of(&ctx, &root.join("site").to_string_lossy()).await.as_deref(), Some("workspace_member"));

        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    #[tokio::test]
    async fn process_git_folder_marks_folder_indexing() {
        // D6a: process_git_folder marks the folder `indexing` at start. The
        // `indexed` transition happens later at the DetectCommunities terminal
        // barrier (D4.1), NOT here — so after process_git_folder alone the folder
        // is left `indexing`, the recoverable in-flight state a crash would leave
        // behind.
        async fn status_of(ctx: &TaskContext, abs: &str) -> Option<String> {
            let row: Option<(String,)> = sqlx_core::query_as::query_as(
                "SELECT status::text FROM sensei.folders WHERE abs_path = $1",
            ).bind(abs).fetch_optional(ctx.pg().pool()).await.unwrap();
            row.map(|r| r.0)
        }

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        std::fs::write(root.join("repo/Cargo.toml"), "[package]\nname=\"repo\"").unwrap();
        std::fs::write(root.join("repo/src/lib.rs"), "pub fn a() {}").unwrap();

        let ctx = make_ctx().await;
        let repo_path = root.join("repo").to_string_lossy().to_string();
        let rid = ctx.pg().add_watch_root(&root.to_string_lossy(), "d6a_root", &serde_json::json!([])).await.unwrap();
        // Register the folder as scan_root would, so process_git_folder resolves it.
        ctx.pg().upsert_repo_kind(&rid, "git", "repo", &repo_path).await.unwrap();
        assert_eq!(status_of(&ctx, &repo_path).await.as_deref(), Some("discovered"), "starts discovered");

        let task = Task::new(TaskKind::ProcessGitFolder, &repo_path, &repo_path);
        process_git_folder(&ctx, &task).await.unwrap();

        assert_eq!(status_of(&ctx, &repo_path).await.as_deref(), Some("indexing"),
            "process_git_folder leaves the folder indexing (the barrier marks indexed later)");

        // D4.1: DetectCommunities is chained as the terminal barrier on a scan
        // with changes, so the folder can later reach `indexed` through it.
        let has_detect = ctx.queue.snapshot().await.iter()
            .any(|(kind, fp, _)| *kind == TaskKind::DetectCommunities && fp == &repo_path);
        assert!(has_detect, "process_git_folder chains DetectCommunities as the terminal barrier");

        ctx.pg().remove_watch_root(&rid).await.unwrap();
    }

    #[tokio::test]
    async fn pipeline_has_no_resolve_pass() {
        // Phase 7.1: FQN edges are resolved AT EMIT (source_id → target_id in
        // process_file), so the scan pipeline no longer enqueues a `resolve_edges`
        // pass — the barrier chain is file → resolve_libs → build_connections →
        // detect_communities. `target_name` is vestigial (only the `dyn` residual).
        // (Kinds are compared by string so the removed variant isn't referenced.)
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        std::fs::write(root.join("repo/Cargo.toml"), "[package]\nname=\"norsv\"").unwrap();
        std::fs::write(root.join("repo/src/lib.rs"), "fn helper() {}\nfn compute() { helper(); }\n").unwrap();

        let ctx = make_ctx().await;
        let repo_path = root.join("repo").to_string_lossy().to_string();
        let rid = ctx.pg().add_watch_root(&root.to_string_lossy(), "norsv_root", &serde_json::json!([])).await.unwrap();
        ctx.pg().upsert_repo_kind(&rid, "git", "norsv", &repo_path).await.unwrap();

        process_git_folder(&ctx, &Task::new(TaskKind::ProcessGitFolder, &repo_path, &repo_path)).await.unwrap();

        let kinds: Vec<String> = ctx.queue.snapshot().await.iter().map(|(k, _, _)| k.to_string()).collect();
        assert!(!kinds.iter().any(|k| k == "resolve_edges"),
            "the scan pipeline has NO resolve_edges pass — edges resolve at emit, got {kinds:?}");
        // The surviving barrier chain is intact (build_connections + the terminal detect).
        assert!(kinds.iter().any(|k| k == "build_connections"), "build_connections still enqueued, got {kinds:?}");
        assert!(kinds.iter().any(|k| k == "detect_communities"), "detect_communities (terminal) still enqueued, got {kinds:?}");

        ctx.pg().remove_watch_root(&rid).await.unwrap();
    }

    #[tokio::test]
    async fn process_git_folder_keeps_unchanged_indexed_folder_indexed() {
        // D6a regression: a no-op re-scan of an already-`indexed` folder (no file
        // changes → the barrier that writes `indexed` is skipped) must NOT be
        // downgraded to `indexing`. scan_root re-enqueues ProcessGitFolder for
        // every folder on every scan, so this is the common steady-state case —
        // an unconditional mark would strand it at `indexing` forever and make
        // resume re-index it on every boot. The mark is gated on has_changes.
        async fn status_of(ctx: &TaskContext, abs: &str) -> Option<String> {
            let row: Option<(String,)> = sqlx_core::query_as::query_as(
                "SELECT status::text FROM sensei.folders WHERE abs_path = $1",
            ).bind(abs).fetch_optional(ctx.pg().pool()).await.unwrap();
            row.map(|r| r.0)
        }

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // Empty repo → no indexable files → has_changes is false → no barrier.
        std::fs::create_dir_all(root.join("repo")).unwrap();

        let ctx = make_ctx().await;
        let repo_path = root.join("repo").to_string_lossy().to_string();
        let rid = ctx.pg().add_watch_root(&root.to_string_lossy(), "d6a_noop", &serde_json::json!([])).await.unwrap();
        ctx.pg().upsert_repo_kind(&rid, "git", "repo", &repo_path).await.unwrap();
        // Simulate a prior completed index.
        let (fid,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.folders WHERE abs_path = $1"
        ).bind(&repo_path).fetch_one(ctx.pg().pool()).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexed").await.unwrap();

        let task = Task::new(TaskKind::ProcessGitFolder, &repo_path, &repo_path);
        process_git_folder(&ctx, &task).await.unwrap();

        assert_eq!(status_of(&ctx, &repo_path).await.as_deref(), Some("indexed"),
            "an unchanged already-indexed folder must not be downgraded to indexing");

        ctx.pg().remove_watch_root(&rid).await.unwrap();
    }

    /// Register a repo folder on disk + in the DB, mark it `indexing`, and
    /// return (watch_root_id, folder_id, repo_abs_path). Mirrors what
    /// scan_root/process_git_folder set up before ProcessFile runs.
    #[tokio::test]
    async fn process_file_rust_emits_fqn_nodes_and_resolved_edges() {
        // Phase 3.1: a Rust file with a resolvable crate context goes through the
        // FQN path — defs get-or-created by fqn (language='rust'), call edges
        // resolved to their target node AT EMIT (no resolve_edges run).
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("fqncrate");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"fqncrate\"\n").unwrap();
        std::fs::write(
            repo.join("src/lib.rs"),
            "pub fn compute() -> i32 { helper() + 1 }\npub fn helper() -> i32 { 41 }\n",
        ).unwrap();
        let repo_path = repo.to_string_lossy().to_string();

        let rid = ctx.pg().add_watch_root(&tmp.path().to_string_lossy(), "fqn", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "fqncrate", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        // Process ONLY the file — deliberately NO resolve_edges. Edges must be
        // resolved at emit for this to pass.
        let abs = repo.join("src/lib.rs").to_string_lossy().to_string();
        process_file(&ctx, &Task::for_file(TaskKind::ProcessFile, &repo_path, &abs)).await.unwrap();

        // Defs carry their canonical fqn + language.
        let (compute_id, compute_fqn, compute_lang): (uuid::Uuid, Option<String>, Option<String>) =
            sqlx_core::query_as::query_as(
                "SELECT id, fqn, language FROM sensei.nodes WHERE folder_id=$1 AND name='compute' AND kind='function'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(compute_fqn.as_deref(), Some("rust·fqncrate·compute"));
        assert_eq!(compute_lang.as_deref(), Some("rust"));
        let (helper_id, helper_fqn): (uuid::Uuid, Option<String>) =
            sqlx_core::query_as::query_as(
                "SELECT id, fqn FROM sensei.nodes WHERE folder_id=$1 AND name='helper' AND kind='function'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(helper_fqn.as_deref(), Some("rust·fqncrate·helper"));

        // compute → helper resolves to the FQN target node AT EMIT.
        let (target,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT target_id FROM sensei.edges WHERE folder_id=$1 AND source_id=$2 AND kind='calls'::sensei.edge_kind")
            .bind(fid).bind(compute_id).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(target, Some(helper_id), "compute→helper resolves to the FQN target at emit (no resolve_edges)");

        // No bare-name 'calls' residue for this file — the helper() call is resolved.
        let (unresolved,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.edges WHERE folder_id=$1 AND kind='calls'::sensei.edge_kind AND target_id IS NULL")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(unresolved, 0, "the helper() call is resolved at emit, not left bare");

        ctx.pg().delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn rust_call_before_def_creates_stub_then_enriched() {
        // Phase 3.2: process the CALLER first — its target is a get-or-created stub
        // (resolved=false, NULL file), and the edge is already resolved to it. Then
        // process the callee's file — the SAME node is enriched in place (stable id),
        // and the edge still points to it. Order-independent resolution.
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("twofile");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"twofile\"\n").unwrap();
        std::fs::write(repo.join("src/caller.rs"), "use crate::callee::run;\npub fn drive() { run(); }\n").unwrap();
        std::fs::write(repo.join("src/callee.rs"), "pub fn run() -> i32 { 7 }\n").unwrap();
        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx.pg().add_watch_root(&tmp.path().to_string_lossy(), "twofile", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "twofile", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        // Caller first.
        let abs_caller = repo.join("src/caller.rs").to_string_lossy().to_string();
        process_file(&ctx, &Task::for_file(TaskKind::ProcessFile, &repo_path, &abs_caller)).await.unwrap();

        // `run` is a STUB awaiting its definition.
        let (run_id, run_resolved, run_file): (uuid::Uuid, bool, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT id, resolved, file_path FROM sensei.nodes WHERE folder_id=$1 AND fqn='rust·twofile·callee·run'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert!(!run_resolved, "callee target is an unresolved stub before its def is indexed");
        assert_eq!(run_file, None, "a stub has no file");

        // The call edge already resolves to the stub node.
        let (drive_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.nodes WHERE folder_id=$1 AND fqn='rust·twofile·caller·drive'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        let (tid,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT target_id FROM sensei.edges WHERE folder_id=$1 AND source_id=$2 AND kind='calls'::sensei.edge_kind")
            .bind(fid).bind(drive_id).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(tid, Some(run_id), "the call edge resolves to the stub node at emit");

        // Now index the callee — the SAME node is enriched.
        let abs_callee = repo.join("src/callee.rs").to_string_lossy().to_string();
        process_file(&ctx, &Task::for_file(TaskKind::ProcessFile, &repo_path, &abs_callee)).await.unwrap();

        let (run_id2, run_resolved2, run_file2): (uuid::Uuid, bool, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT id, resolved, file_path FROM sensei.nodes WHERE folder_id=$1 AND fqn='rust·twofile·callee·run'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(run_id2, run_id, "the definition enriches the SAME node (stable id)");
        assert!(run_resolved2, "the node is resolved once its def is seen");
        assert_eq!(run_file2.as_deref(), Some("src/callee.rs"), "file filled in on enrich");

        // The edge still points to the (now enriched) node.
        let (tid2,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT target_id FROM sensei.edges WHERE folder_id=$1 AND source_id=$2 AND kind='calls'::sensei.edge_kind")
            .bind(fid).bind(drive_id).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(tid2, Some(run_id), "edge still resolved to the enriched node after enrichment");

        ctx.pg().delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn external_calls_link_to_lib_nodes() {
        // Phase 4: a call into a dependency links to a first-class `lib_symbol` node
        // grouped (props.package + parent_id) under a per-package `lib_package`
        // container, and the dependency is queryable per repo. No external call dropped.
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("libcrate");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"libcrate\"\n").unwrap();
        std::fs::write(repo.join("src/lib.rs"), "pub fn load(s: &str) { serde_json::from_str(s); }\n").unwrap();
        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx.pg().add_watch_root(&tmp.path().to_string_lossy(), "libc", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "libcrate", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        let abs = repo.join("src/lib.rs").to_string_lossy().to_string();
        process_file(&ctx, &Task::for_file(TaskKind::ProcessFile, &repo_path, &abs)).await.unwrap();

        // The external symbol is a `lib_symbol`, grouped by package.
        let (sym_id, sym_pkg, parent): (uuid::Uuid, Option<String>, Option<uuid::Uuid>) = sqlx_core::query_as::query_as(
            "SELECT id, props->>'package', parent_id FROM sensei.nodes WHERE folder_id=$1 AND kind='lib_symbol'::sensei.node_kind AND name='from_str'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(sym_pkg.as_deref(), Some("serde_json"), "lib symbol grouped by package");

        // …under a per-package `lib_package` container.
        let (container_id, container_name): (uuid::Uuid, String) = sqlx_core::query_as::query_as(
            "SELECT id, name FROM sensei.nodes WHERE folder_id=$1 AND kind='lib_package'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(container_name, "serde_json", "a lib_package container per dependency");
        assert_eq!(parent, Some(container_id), "the lib symbol is parented under its package container");

        // A RESOLVED call edge load → from_str (external call not dropped).
        let (edge_target,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT e.target_id FROM sensei.edges e JOIN sensei.nodes s ON s.id=e.source_id
              WHERE e.folder_id=$1 AND s.name='load' AND e.kind='calls'::sensei.edge_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(edge_target, Some(sym_id), "the external call resolves to the lib symbol node");

        // Queryable per repo.
        let deps = ctx.pg().list_dependencies(&fid).await.unwrap();
        assert!(
            deps.iter().any(|d| d["package"] == "serde_json" && d["symbol_count"] == 1),
            "serde_json is a queryable dependency with one used symbol, got {deps:?}"
        );

        ctx.pg().delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn process_file_flags_test_file_nodes_is_test() {
        // Every node in a test file gets is_test=true (UI filters tests out);
        // production-file nodes stay is_test=false.
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("istest");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::create_dir_all(repo.join("tests")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname=\"istest\"\n").unwrap();
        std::fs::write(repo.join("src/lib.rs"), "pub fn compute() -> i32 { 1 }\n").unwrap();
        std::fs::write(repo.join("tests/it.rs"), "fn helper() {}\nfn check() { helper(); }\n").unwrap();
        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx.pg().add_watch_root(&tmp.path().to_string_lossy(), "istest", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "istest", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        for rel in ["src/lib.rs", "tests/it.rs"] {
            let abs = repo.join(rel).to_string_lossy().to_string();
            process_file(&ctx, &Task::for_file(TaskKind::ProcessFile, &repo_path, &abs)).await.unwrap();
        }

        let count = |sql: &'static str| {
            let pool = ctx.pg().pool().clone();
            async move { let (n,): (i64,) = sqlx_core::query_as::query_as(sql).bind(fid).fetch_one(&pool).await.unwrap(); n }
        };
        // Every node of the test file is flagged; none left unflagged.
        assert!(count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND file_path='tests/it.rs' AND is_test").await >= 1,
            "test-file nodes are is_test=true");
        assert_eq!(count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND file_path='tests/it.rs' AND NOT is_test").await, 0,
            "no test-file node left unflagged");
        // Production file nodes exist and are NOT flagged.
        assert!(count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND file_path='src/lib.rs'").await >= 1,
            "prod file produced nodes");
        assert_eq!(count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND file_path='src/lib.rs' AND is_test").await, 0,
            "production-file nodes are not is_test");

        ctx.pg().delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn rust_impl_type_container_nesting() {
        // Phase 5 (D5c): the graph nests file → module → type → method, instead of
        // every symbol flat under the file node.
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("w");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"w\"\n").unwrap();
        std::fs::write(
            repo.join("src/widget.rs"),
            "pub struct Widget;\nimpl Widget {\n    pub fn new() -> Self { Widget }\n    pub fn spin(&self) {}\n}\npub fn helper() {}\n",
        ).unwrap();
        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx.pg().add_watch_root(&tmp.path().to_string_lossy(), "w", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "w", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        let abs = repo.join("src/widget.rs").to_string_lossy().to_string();
        process_file(&ctx, &Task::for_file(TaskKind::ProcessFile, &repo_path, &abs)).await.unwrap();

        async fn node(ctx: &TaskContext, fid: uuid::Uuid, fqn: &str) -> (uuid::Uuid, Option<uuid::Uuid>, String) {
            sqlx_core::query_as::query_as("SELECT id, parent_id, kind::text FROM sensei.nodes WHERE folder_id=$1 AND fqn=$2")
                .bind(fid).bind(fqn).fetch_one(ctx.pg().pool()).await
                .unwrap_or_else(|e| panic!("node {fqn} not found: {e}"))
        }
        let (file_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.nodes WHERE folder_id=$1 AND kind='file'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();

        let (module_id, module_parent, module_kind) = node(&ctx, fid, "rust·w·widget").await;
        assert_eq!(module_kind, "module", "a module container node exists");
        assert_eq!(module_parent, Some(file_id), "the module nests under the file");

        let (widget_id, widget_parent, _) = node(&ctx, fid, "rust·w·widget·Widget").await;
        assert_eq!(widget_parent, Some(module_id), "the type nests under the module");

        let (_, new_parent, _) = node(&ctx, fid, "rust·w·widget·Widget·new").await;
        assert_eq!(new_parent, Some(widget_id), "a method nests under its type");
        let (_, spin_parent, _) = node(&ctx, fid, "rust·w·widget·Widget·spin").await;
        assert_eq!(spin_parent, Some(widget_id), "sibling methods nest under the same type");

        let (_, helper_parent, _) = node(&ctx, fid, "rust·w·widget·helper").await;
        assert_eq!(helper_parent, Some(module_id), "a free fn nests under the module");

        ctx.pg().delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn process_file_ts_emits_fqn_nodes() {
        // Phase 6.1: a TypeScript file with a package.json → the FQN path. Validates
        // the oxc producer end-to-end, the src-stripped module, resolved edges, AND
        // that the node language column is 'typescript' (not the old hardcoded rust).
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("tsapp");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("package.json"), "{\"name\": \"tsapp\"}").unwrap();
        std::fs::write(repo.join("src/util.ts"),
            "export function compute() { return helper(); }\nexport function helper() { return 1; }\n").unwrap();
        let repo_path = repo.to_string_lossy().to_string();
        let rid = ctx.pg().add_watch_root(&tmp.path().to_string_lossy(), "ts", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "tsapp", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        let abs = repo.join("src/util.ts").to_string_lossy().to_string();
        process_file(&ctx, &Task::for_file(TaskKind::ProcessFile, &repo_path, &abs)).await.unwrap();

        let (compute_id, compute_fqn, compute_lang): (uuid::Uuid, Option<String>, Option<String>) =
            sqlx_core::query_as::query_as(
                "SELECT id, fqn, language FROM sensei.nodes WHERE folder_id=$1 AND name='compute' AND kind='function'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(compute_fqn.as_deref(), Some("typescript·tsapp·util·compute"), "src/ stripped module + oxc def");
        assert_eq!(compute_lang.as_deref(), Some("typescript"), "language column is the file's language, not hardcoded rust");

        let (helper_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.nodes WHERE folder_id=$1 AND fqn='typescript·tsapp·util·helper'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        let (target,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT target_id FROM sensei.edges WHERE folder_id=$1 AND source_id=$2 AND kind='calls'::sensei.edge_kind")
            .bind(fid).bind(compute_id).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(target, Some(helper_id), "compute→helper resolves to the FQN target at emit");

        ctx.pg().delete_nodes_by_folder(&fid).await.unwrap();
    }

    async fn seed_indexing_repo(ctx: &TaskContext, root: &std::path::Path, name: &str) -> (uuid::Uuid, uuid::Uuid, String) {
        let repo_path = root.join("repo").to_string_lossy().to_string();
        let rid = ctx.pg().add_watch_root(&root.to_string_lossy(), name, &serde_json::json!([])).await.unwrap();
        ctx.pg().upsert_repo_kind(&rid, "git", "repo", &repo_path).await.unwrap();
        let (fid,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.folders WHERE abs_path = $1"
        ).bind(&repo_path).fetch_one(ctx.pg().pool()).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();
        (rid, fid, repo_path)
    }

    #[tokio::test]
    async fn graph_scan_end_to_end() {
        // 7.5: scan the committed fixture repo through the real handler chain and
        // assert the WHOLE graph at once — code + doc-section + rationale nodes,
        // resolved edges (dup-factor 1.0), deterministic communities, the folder
        // reaching `indexed`, and the retrieval contract (tree + per-node
        // community_id + live overview). Then re-run → convergent (zero net rows),
        // then mutate a doc → scoped incremental. (The monorepo `workspace_member`
        // kind is covered separately by `reconcile_classifies_monorepo_member_roles`;
        // this fixture is manifest-free by design.)
        let ctx = make_ctx().await;
        // Materialise the committed fixture into a tempdir — read the real committed
        // files by their known relative paths — so the incremental step can mutate
        // it without dirtying the repo.
        let src_fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/graph-scan");
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("graph-scan");
        // Cargo.toml is materialised too so the Rust FQN producer can derive the
        // package (nearest manifest) and resolve compute→helper AT EMIT (7.1).
        for rel in ["Cargo.toml", "src/lib.rs", "docs/design.md"] {
            let content = std::fs::read_to_string(src_fixture.join(rel)).unwrap();
            let dst = repo.join(rel);
            std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
            std::fs::write(&dst, content).unwrap();
        }
        let repo_path = repo.to_string_lossy().to_string();

        let rid = ctx.pg().add_watch_root(&tmp.path().to_string_lossy(), "gse", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo_kind(&rid, "git", "graph-scan", &repo_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        // Drive the real handler chain (deterministic — the queue's next_task
        // blocks, so tests drive handlers directly, the codebase idiom).
        async fn scan_files(ctx: &TaskContext, repo: &std::path::Path, repo_path: &str, rels: &[&str]) {
            for rel in rels {
                let abs = repo.join(rel).to_string_lossy().to_string();
                process_file(ctx, &Task::for_file(TaskKind::ProcessFile, repo_path, &abs)).await.unwrap();
            }
            crate::tasks::handlers::build_connections(ctx, &Task::new(TaskKind::BuildConnections, repo_path, repo_path)).await.unwrap();
            crate::tasks::handlers::detect_communities(ctx, &Task::new(TaskKind::DetectCommunities, repo_path, "")).await.unwrap();
        }
        let files = ["src/lib.rs", "docs/design.md"];
        scan_files(&ctx, &repo, &repo_path, &files).await;

        let count = |sql: &'static str| {
            let pool = ctx.pg().pool().clone();
            async move {
                let (n,): (i64,) = sqlx_core::query_as::query_as(sql).bind(fid).fetch_one(&pool).await.unwrap();
                n
            }
        };

        // ── Whole-graph: kinds present ──
        assert!(count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='file'::sensei.node_kind").await >= 1, "file node(s)");
        assert!(count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='function'::sensei.node_kind").await >= 2, "compute + helper function nodes");
        assert!(count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='section'::sensei.node_kind").await >= 3, "nested section nodes (Design/Auth/Refresh/Storage)");
        assert_eq!(count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='rationale'::sensei.node_kind").await, 1, "one TODO rationale");

        // ── Section nesting (Refresh under Auth under a doc/file node) ──
        let (nested,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes s JOIN sensei.nodes p ON s.parent_id=p.id
              WHERE s.folder_id=$1 AND s.kind='section'::sensei.node_kind AND p.kind IN ('doc'::sensei.node_kind,'file'::sensei.node_kind,'section'::sensei.node_kind)")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert!(nested >= 3, "sections nest via parent_id");

        // ── Resolved call edge (compute → helper) ──
        let (resolved_calls,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.edges WHERE folder_id=$1 AND kind='calls'::sensei.edge_kind AND target_id IS NOT NULL")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert!(resolved_calls >= 1, "the compute→helper call resolved");

        // ── dup-factor 1.0 for every edge kind ──
        let (dup_kinds,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM (
               SELECT kind, count(*) c, count(DISTINCT (source_id,target_id,target_name,target_file)) d
                 FROM sensei.edges WHERE folder_id=$1 GROUP BY kind) t WHERE c <> d")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(dup_kinds, 0, "no duplicate edges — dup-factor 1.0 for every kind");

        // ── Communities deterministic + coverage ──
        let code_uncovered = count("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind IN ('function'::sensei.node_kind,'file'::sensei.node_kind) AND community_id IS NULL").await;
        assert_eq!(code_uncovered, 0, "every code/file node carries a community_id (coverage)");

        // ── Folder reached `indexed` (terminal barrier) ──
        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("indexed"), "the terminal DetectCommunities barrier flipped the folder to indexed");

        // ── Retrieval contract: tree nests, node projection carries community_id, live overview ──
        let folders = ctx.pg().get_folders_scoped(&[fid]).await.unwrap();
        let nodes = ctx.pg().get_nodes_scoped(&[fid]).await.unwrap();
        assert!(nodes.iter().any(|n| n.get("community_id").is_some()), "get_nodes_scoped projects community_id");
        let tree = crate::api::handlers::codebase::build_tree_pub(&folders, &nodes);
        let roots = tree["tree"].as_array().unwrap();
        assert!(!roots.is_empty(), "tree has a root folder");
        // The root folder exposes file/doc nodes, and a doc node has section children.
        let has_section_child = |v: &serde_json::Value| -> bool {
            v["nodes"].as_array().map(|ns| ns.iter().any(|f| {
                f["children"].as_array().map(|c| c.iter().any(|ch| ch["kind"] == "section")).unwrap_or(false)
            })).unwrap_or(false)
        };
        assert!(roots.iter().any(has_section_child), "the tree nests a doc → section subtree");
        let live = ctx.pg().list_communities_live_scoped(&[fid]).await.unwrap();
        assert!(!live.is_empty() && live.iter().all(|c| c["node_count"].as_i64().unwrap_or(0) > 0), "live overview sized by real membership");

        // ── Idempotency / convergence: re-run is IDENTITY-STABLE, not just
        // count-stable. Capture every node's id keyed on its natural key
        // (file_path,kind,name,line_start); after a second scan assert the id map
        // is byte-identical — so a regression to delete-then-insert (which keeps
        // counts equal but MINTS NEW UUIDs, nulling embeddings/community) fails
        // here, per invariant 2 (identical nodes.id set on re-run).
        let ids_before: std::collections::BTreeMap<(String, String, String, Option<i32>), uuid::Uuid> = {
            let rows: Vec<(String, String, String, Option<i32>, uuid::Uuid)> = sqlx_core::query_as::query_as(
                "SELECT file_path, kind::text, name, line_start, id FROM sensei.nodes WHERE folder_id=$1")
                .bind(fid).fetch_all(ctx.pg().pool()).await.unwrap();
            rows.into_iter().map(|(fp, k, n, ls, id)| ((fp, k, n, ls), id)).collect()
        };
        let e0 = count("SELECT count(*) FROM sensei.edges WHERE folder_id=$1").await;
        scan_files(&ctx, &repo, &repo_path, &files).await;
        let ids_after: std::collections::BTreeMap<(String, String, String, Option<i32>), uuid::Uuid> = {
            let rows: Vec<(String, String, String, Option<i32>, uuid::Uuid)> = sqlx_core::query_as::query_as(
                "SELECT file_path, kind::text, name, line_start, id FROM sensei.nodes WHERE folder_id=$1")
                .bind(fid).fetch_all(ctx.pg().pool()).await.unwrap();
            rows.into_iter().map(|(fp, k, n, ls, id)| ((fp, k, n, ls), id)).collect()
        };
        let e1 = count("SELECT count(*) FROM sensei.edges WHERE folder_id=$1").await;
        assert_eq!(ids_before, ids_after,
            "a second scan is identity-stable — every node keeps its exact id (not delete-then-insert)");
        assert_eq!(e0, e1, "a second scan adds no edges (dup-factor 1.0, convergent)");

        // ── Scoped incremental: add a heading to the doc → new section, and an
        // unrelated code node keeps its exact id AND community (upsert-then-prune,
        // not a wholesale re-mint) ──
        let (compute_id_before, compute_comm_before): (uuid::Uuid, Option<i32>) = sqlx_core::query_as::query_as(
            "SELECT id, community_id FROM sensei.nodes WHERE folder_id=$1 AND kind='function'::sensei.node_kind AND name='compute'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        let design = repo.join("docs/design.md");
        let mut doc = std::fs::read_to_string(&design).unwrap();
        doc.push_str("\n## Extra\n\nAdded section.\n");
        std::fs::write(&design, &doc).unwrap();
        scan_files(&ctx, &repo, &repo_path, &files).await;

        let (extra,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='section'::sensei.node_kind AND name='Design > Extra'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(extra, 1, "the new heading became a section node");
        let (compute_id_after, compute_comm_after): (uuid::Uuid, Option<i32>) = sqlx_core::query_as::query_as(
            "SELECT id, community_id FROM sensei.nodes WHERE folder_id=$1 AND kind='function'::sensei.node_kind AND name='compute'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(compute_id_after, compute_id_before, "an unrelated code node keeps its EXACT id across a scoped doc edit (upsert-then-prune)");
        assert_eq!(compute_comm_after, compute_comm_before, "and keeps its community_id");
        // Still no duplicate edges after the incremental edit.
        let (dup2,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM (SELECT kind, count(*) c, count(DISTINCT (source_id,target_id,target_name,target_file)) d FROM sensei.edges WHERE folder_id=$1 GROUP BY kind) t WHERE c <> d")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(dup2, 0, "still dup-factor 1.0 after the incremental edit");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn processing_order_invariant() {
        // 7.4: processing the same files in DIFFERENT orders yields an identical
        // graph — the node set (by natural key), per-kind edge counts, and the
        // deterministic community_id per node all match. This is the convergence
        // guarantee D1–D4 exist to provide.
        use std::collections::BTreeMap;
        // Natural key = (file_path, kind, name, line_start); a graph snapshot maps
        // each to its community_id, plus per-kind edge counts.
        type NatKey = (String, String, String, Option<i32>);
        type GraphSnap = (BTreeMap<NatKey, Option<i32>>, BTreeMap<String, i64>);
        type NodeRow = (String, String, String, Option<i32>, Option<i32>);

        // Snapshot a folder's graph by NATURAL key (not the random UUID):
        // {(file_path,kind,name,line_start) → community_id} + per-kind edge counts.
        async fn snapshot(ctx: &TaskContext, fid: uuid::Uuid) -> GraphSnap {
            let node_rows: Vec<NodeRow> = sqlx_core::query_as::query_as(
                "SELECT file_path, kind::text, name, line_start, community_id FROM sensei.nodes WHERE folder_id=$1")
                .bind(fid).fetch_all(ctx.pg().pool()).await.unwrap();
            let nodes = node_rows.into_iter()
                .map(|(fp, k, n, ls, cid)| ((fp, k, n, ls), cid)).collect();
            let edge_rows: Vec<(String, i64)> = sqlx_core::query_as::query_as(
                "SELECT kind::text, count(*) FROM sensei.edges WHERE folder_id=$1 GROUP BY kind::text")
                .bind(fid).fetch_all(ctx.pg().pool()).await.unwrap();
            (nodes, edge_rows.into_iter().collect())
        }

        // Build a repo with identical content under `root/repo`, then process the
        // given file order (edges resolve at emit) and detect communities.
        async fn build_and_scan(ctx: &TaskContext, root: &std::path::Path, name: &str, order: &[&str]) -> uuid::Uuid {
            std::fs::create_dir_all(root.join("repo/src")).unwrap();
            std::fs::create_dir_all(root.join("repo/docs")).unwrap();
            std::fs::write(root.join("repo/src/a.rs"), "pub fn caller() { helper(); }\n").unwrap();
            std::fs::write(root.join("repo/src/b.rs"), "pub fn helper() {}\n").unwrap();
            std::fs::write(root.join("repo/docs/design.md"),
                "# Design\n\n## Auth\n\nAuth text.\n\n<!-- TODO: wire the retry path -->\n").unwrap();
            let (_rid, fid, repo_path) = seed_indexing_repo(ctx, root, name).await;
            for rel in order {
                let abs = root.join("repo").join(rel).to_string_lossy().to_string();
                process_file(ctx, &Task::for_file(TaskKind::ProcessFile, &repo_path, &abs)).await.unwrap();
            }
            crate::indexer::community::detect_communities_for_folder(ctx.pg(), &fid).await.unwrap();
            fid
        }

        let ctx = make_ctx().await;
        let tmp_a = tempfile::tempdir().unwrap();
        let tmp_b = tempfile::tempdir().unwrap();
        // Opposite processing orders over identical content.
        let fid_a = build_and_scan(&ctx, tmp_a.path(), "order_a",
            &["src/a.rs", "src/b.rs", "docs/design.md"]).await;
        let fid_b = build_and_scan(&ctx, tmp_b.path(), "order_b",
            &["docs/design.md", "src/b.rs", "src/a.rs"]).await;

        let (nodes_a, edges_a) = snapshot(&ctx, fid_a).await;
        let (nodes_b, edges_b) = snapshot(&ctx, fid_b).await;

        assert!(!nodes_a.is_empty(), "the scan produced nodes");
        assert_eq!(nodes_a.keys().collect::<Vec<_>>(), nodes_b.keys().collect::<Vec<_>>(),
            "identical node set (by natural key) regardless of processing order");
        assert_eq!(nodes_a, nodes_b,
            "identical community_id per node regardless of processing order (deterministic)");
        assert_eq!(edges_a, edges_b,
            "identical per-kind edge counts regardless of processing order");
        // The resolved call edge exists (so this isn't a vacuously-empty comparison).
        assert_eq!(edges_a.get("calls").copied(), Some(1), "caller→helper call edge resolved");
    }

    #[tokio::test]
    async fn process_file_fatal_db_write_marks_folder_failed_and_skips_scan_state() {
        // D6c-trigger: a fatal DB-write failure (simulated via the test fault
        // seam) propagates as Err, marks the folder `failed` (so the fail-closed
        // barrier D6d won't mark it indexed), and does NOT advance the file's
        // scan_state — so the next scan retries it.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        let file = root.join("repo/src/lib.rs");
        std::fs::write(&file, "pub fn a() {}").unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d6c_fatal").await;

        let abs = file.to_string_lossy().to_string();
        super::fault::fail_for(&abs);
        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &abs);
        let res = process_file(&ctx, &task).await;
        super::fault::clear(&abs);

        assert!(res.is_err(), "a fatal DB write propagates as Err, not Ok");
        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("failed"),
            "the folder is left `failed` (fail-closed)");
        assert!(ctx.pg().list_scan_state(&fid).await.unwrap().is_empty(),
            "scan_state is NOT advanced for a fatally-failed file");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn process_file_success_advances_scan_state_and_keeps_folder_status() {
        // The success counterpart: a fully-written file advances scan_state and
        // does NOT spuriously mark the folder failed (it stays `indexing` for the
        // barrier to flip to `indexed`).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        let file = root.join("repo/src/lib.rs");
        std::fs::write(&file, "pub fn a() {}").unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d6c_ok").await;

        let abs = file.to_string_lossy().to_string();
        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &abs);
        process_file(&ctx, &task).await.unwrap();

        assert_eq!(ctx.pg().list_scan_state(&fid).await.unwrap().len(), 1,
            "a fully-written file advances scan_state");
        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("indexing"),
            "a successful file does not spuriously mark the folder failed");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn process_file_fatal_on_one_file_does_not_block_a_sibling() {
        // The fatal path is per-file: a sibling file still indexes (its scan_state
        // is written) even though another file in the same folder failed fatally.
        // The folder ends `failed` (fail-closed), but the healthy file's work is
        // durable.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        let bad = root.join("repo/src/bad.rs");
        let good = root.join("repo/src/good.rs");
        std::fs::write(&bad, "pub fn a() {}").unwrap();
        std::fs::write(&good, "pub fn b() {}").unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d6c_sibling").await;

        let bad_abs = bad.to_string_lossy().to_string();
        super::fault::fail_for(&bad_abs);
        let bad_res = process_file(&ctx, &Task::for_file(TaskKind::ProcessFile, &repo_path, &bad_abs)).await;
        super::fault::clear(&bad_abs);
        // The sibling processes independently and succeeds.
        let good_abs = good.to_string_lossy().to_string();
        process_file(&ctx, &Task::for_file(TaskKind::ProcessFile, &repo_path, &good_abs)).await.unwrap();

        assert!(bad_res.is_err(), "the bad file fails fatally");
        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("failed"),
            "the folder is `failed` because one of its files failed");
        let scan = ctx.pg().list_scan_state(&fid).await.unwrap();
        assert_eq!(scan.len(), 1, "only the healthy sibling advanced scan_state");
        assert!(scan.iter().any(|(p, _)| p.ends_with("good.rs")), "the sibling's fingerprint is recorded");
        assert!(!scan.iter().any(|(p, _)| p.ends_with("bad.rs")), "the failed file did NOT advance scan_state");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn process_file_reindex_keeps_surviving_node_and_prunes_removed() {
        // D3 end-to-end: re-indexing a file KEEPS a surviving symbol's node id
        // (and its community_id — proving upsert-then-prune, not delete-then-
        // insert) and PRUNES a symbol removed from the source.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        let file = root.join("repo/src/lib.rs");
        std::fs::write(&file, "pub fn keep() {}\npub fn gone() {}\n").unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d3_reindex").await;
        let abs = file.to_string_lossy().to_string();
        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &abs);

        process_file(&ctx, &task).await.unwrap();
        let keep_id: uuid::Uuid = sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM sensei.nodes WHERE folder_id=$1 AND name='keep' AND kind='function'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await
            .expect("first index creates the `keep` function node").0;
        sqlx_core::query::query("UPDATE sensei.nodes SET community_id=42 WHERE id=$1")
            .bind(keep_id).execute(ctx.pg().pool()).await.unwrap();

        // Edit: remove `gone`; `keep` stays at line 1 (unchanged identity), so it
        // survives with the same id (upsert-then-prune, not delete-then-insert).
        std::fs::write(&file, "pub fn keep() {}\n").unwrap();
        process_file(&ctx, &task).await.unwrap();

        let keep_after: Option<(uuid::Uuid, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, community_id FROM sensei.nodes WHERE folder_id=$1 AND name='keep' AND kind='function'::sensei.node_kind")
            .bind(fid).fetch_optional(ctx.pg().pool()).await.unwrap();
        assert_eq!(keep_after, Some((keep_id, Some(42))),
            "surviving symbol keeps its id AND community_id across a reindex (upsert-then-prune)");
        let (gone_cnt,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND name='gone'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(gone_cnt, 0, "the removed symbol is pruned");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn doc_decomposes_into_nested_sections() {
        // D5b: a design doc decomposes into nested `section` nodes (file → H1 → H2
        // → H3 via parent_id, level in props), keyed on the heading PATH so a
        // re-index reconciles the set (no duplicate headings). Line-independent
        // identity: a body edit that shifts a heading's line keeps the section's id.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/docs")).unwrap();
        let file = root.join("repo/docs/design.md");
        std::fs::write(&file,
            "# Design\n\nIntro.\n\n## Auth\n\nAuth overview.\n\n### Refresh\n\nToken refresh.\n\n## Storage\n\nStorage overview.\n"
        ).unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d5b_sections").await;
        let abs = file.to_string_lossy().to_string();
        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &abs);

        process_file(&ctx, &task).await.unwrap();

        // Four section nodes: Design(H1), Auth(H2), Refresh(H3), Storage(H2).
        let (sec_cnt,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='section'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(sec_cnt, 4, "one section node per heading");

        // Nesting: Refresh's parent is Auth, Auth's parent is the H1 Design, and
        // Design's parent is the file node. Identity is the heading PATH.
        let parent_kind_name = |name: &str| {
            let pool = ctx.pg().pool().clone();
            let name = name.to_string();
            async move {
                let row: (String, String) = sqlx_core::query_as::query_as(
                    "SELECT p.kind::text, p.name FROM sensei.nodes s JOIN sensei.nodes p ON s.parent_id=p.id
                      WHERE s.folder_id=$1 AND s.kind='section'::sensei.node_kind AND s.name=$2")
                    .bind(fid).bind(&name).fetch_one(&pool).await.unwrap();
                row
            }
        };
        assert_eq!(parent_kind_name("Design > Auth > Refresh").await, ("section".into(), "Design > Auth".into()),
            "H3 Refresh nests under H2 Auth");
        assert_eq!(parent_kind_name("Design > Auth").await, ("section".into(), "Design".into()),
            "H2 Auth nests under H1 Design");
        assert_eq!(parent_kind_name("Design").await.0, "doc",
            "the top-level H1 nests under the doc/file node");

        // level lives in props; identity carries a NULL line (line-independent).
        let (level, line_start_col): (Option<i32>, Option<i32>) = sqlx_core::query_as::query_as(
            "SELECT (props->>'level')::int, line_start FROM sensei.nodes
              WHERE folder_id=$1 AND kind='section'::sensei.node_kind AND name='Design > Auth > Refresh'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(level, Some(3), "H3 level stamped in props");
        assert_eq!(line_start_col, None, "identity line_start is NULL (line-independent section identity)");

        // Capture Refresh's id, then re-index with the heading MOVED down (extra
        // intro line) — same heading path ⇒ same id, and still exactly 4 sections.
        let (refresh_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.nodes WHERE folder_id=$1 AND name='Design > Auth > Refresh'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        std::fs::write(&file,
            "# Design\n\nIntro paragraph one.\nIntro paragraph two.\n\n## Auth\n\nAuth overview.\n\n### Refresh\n\nToken refresh.\n\n## Storage\n\nStorage overview.\n"
        ).unwrap();
        process_file(&ctx, &task).await.unwrap();

        let (sec_cnt2,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='section'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(sec_cnt2, 4, "re-index reconciles — no duplicate sections");
        let (refresh_id2,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.nodes WHERE folder_id=$1 AND name='Design > Auth > Refresh'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(refresh_id2, refresh_id, "a moved heading keeps its id (line-independent identity)");

        // Remove the Refresh heading → it is pruned (no stale section).
        std::fs::write(&file,
            "# Design\n\nIntro.\n\n## Auth\n\nAuth overview.\n\n## Storage\n\nStorage overview.\n"
        ).unwrap();
        process_file(&ctx, &task).await.unwrap();
        let (refresh_gone,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND name='Design > Auth > Refresh'")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(refresh_gone, 0, "a removed heading is pruned");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn doc_duplicate_sibling_headings_are_distinct_sections() {
        // D5b review fix: two identical-text siblings under the same parent must be
        // DISTINCT section nodes — else the second's upsert collides onto the first
        // (same heading-path + parent_id) and silently clobbers it. The Nth (N>1)
        // occurrence gets a " #N" suffix that also flows to its children, so a child
        // of the second sibling doesn't collide with a child of the first.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/docs")).unwrap();
        let file = root.join("repo/docs/faq.md");
        std::fs::write(&file,
            "# FAQ\n\n## Setup\n\nFirst setup.\n\n### Step\n\nA.\n\n## Setup\n\nSecond setup.\n\n### Step\n\nB.\n"
        ).unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d5b_dupe").await;
        let abs = file.to_string_lossy().to_string();
        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &abs);

        process_file(&ctx, &task).await.unwrap();

        // 5 distinct sections: FAQ, "FAQ > Setup", "FAQ > Setup > Step",
        // "FAQ > Setup #2", "FAQ > Setup #2 > Step".
        let names: Vec<String> = sqlx_core::query_as::query_as::<_, (String,)>(
            "SELECT name FROM sensei.nodes WHERE folder_id=$1 AND kind='section'::sensei.node_kind ORDER BY name")
            .bind(fid).fetch_all(ctx.pg().pool()).await.unwrap()
            .into_iter().map(|(n,)| n).collect();
        assert_eq!(names, vec![
            "FAQ".to_string(),
            "FAQ > Setup".to_string(),
            "FAQ > Setup #2".to_string(),
            "FAQ > Setup #2 > Step".to_string(),
            "FAQ > Setup > Step".to_string(),
        ], "duplicate siblings + their children are distinct nodes");

        // Both Setup sections exist with their OWN preview (neither clobbered).
        let previews: Vec<Option<String>> = sqlx_core::query_as::query_as::<_, (Option<String>,)>(
            "SELECT props->>'preview' FROM sensei.nodes
              WHERE folder_id=$1 AND kind='section'::sensei.node_kind AND name IN ('FAQ > Setup','FAQ > Setup #2')
              ORDER BY name")
            .bind(fid).fetch_all(ctx.pg().pool()).await.unwrap()
            .into_iter().map(|(p,)| p).collect();
        assert!(previews[0].as_deref().unwrap_or("").contains("First setup"), "first Setup keeps its own content");
        assert!(previews[1].as_deref().unwrap_or("").contains("Second setup"), "second Setup keeps its own content (not clobbered)");

        // Idempotent: re-index yields the same 5, no growth.
        process_file(&ctx, &task).await.unwrap();
        let (cnt,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='section'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(cnt, 5, "re-index of duplicate-sibling doc is idempotent");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn doc_rationale_comment_emits_rationale_node() {
        // D5b: a NOTE/WHY/HACK/TODO/IMPORTANT marker in a doc becomes a `rationale`
        // node parented to the file, with the marker in props. Re-indexing the
        // unchanged doc is idempotent (no duplicate rationale).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/docs")).unwrap();
        let file = root.join("repo/docs/plan.md");
        std::fs::write(&file,
            "# Plan\n\nSome design text.\n\n<!-- TODO: wire the retry path -->\n\nMore text noting nothing.\n"
        ).unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d5b_rationale").await;
        let abs = file.to_string_lossy().to_string();
        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &abs);

        process_file(&ctx, &task).await.unwrap();

        // Exactly one rationale (the lowercase "noting" prose must NOT match).
        let rows: Vec<(String, Option<uuid::Uuid>, String)> = sqlx_core::query_as::query_as(
            "SELECT name, parent_id, props->>'marker' FROM sensei.nodes
              WHERE folder_id=$1 AND kind='rationale'::sensei.node_kind")
            .bind(fid).fetch_all(ctx.pg().pool()).await.unwrap();
        assert_eq!(rows.len(), 1, "one rationale node (prose 'noting' does not match)");
        assert!(rows[0].0.starts_with("TODO"), "rationale text keeps the marker: {}", rows[0].0);
        assert_eq!(rows[0].2, "TODO", "marker stamped in props");

        // Parent is the doc/file node.
        let (pkind,): (String,) = sqlx_core::query_as::query_as(
            "SELECT p.kind::text FROM sensei.nodes r JOIN sensei.nodes p ON r.parent_id=p.id
              WHERE r.id=(SELECT id FROM sensei.nodes WHERE folder_id=$1 AND kind='rationale'::sensei.node_kind LIMIT 1)")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(pkind, "doc", "rationale is parented to the doc file node");

        // Idempotent re-index.
        process_file(&ctx, &task).await.unwrap();
        let (cnt2,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='rationale'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(cnt2, 1, "re-index does not duplicate the rationale");

        // Remove the marker → the rationale is pruned.
        std::fs::write(&file, "# Plan\n\nSome design text.\n").unwrap();
        process_file(&ctx, &task).await.unwrap();
        let (cnt3,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='rationale'::sensei.node_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(cnt3, 0, "a removed rationale marker is pruned");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn process_file_persists_is_exported_from_visibility() {
        // A `pub` symbol is exported; a private one is not. The parser computes
        // is_exported from the visibility modifier; process_file must PERSIST it
        // (via upsert_node_ex) — it was previously dropped, so every symbol read
        // back as is_exported=false (the "0 exports" call-flow bug).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        let file = root.join("repo/src/lib.rs");
        std::fs::write(&file, "pub fn exported_fn() {}\nfn private_fn() {}\n").unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "isexp").await;
        let abs = file.to_string_lossy().to_string();
        process_file(&ctx, &Task::for_file(TaskKind::ProcessFile, &repo_path, &abs)).await.unwrap();

        let exported = |name: &str| {
            let pool = ctx.pg().pool().clone();
            let name = name.to_string();
            async move {
                let (e,): (bool,) = sqlx_core::query_as::query_as(
                    "SELECT is_exported FROM sensei.nodes WHERE folder_id=$1 AND name=$2 AND kind='function'::sensei.node_kind")
                    .bind(fid).bind(&name).fetch_one(&pool).await.unwrap();
                e
            }
        };
        assert!(exported("exported_fn").await, "a pub fn is is_exported=true");
        assert!(!exported("private_fn").await, "a private fn is is_exported=false");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn process_file_reindex_reconciles_a_removed_out_edge() {
        // D3 per-file out-edge reconcile (end-to-end): a SURVIVING symbol whose
        // call is removed on re-edit drops its stale edge — the surviving node
        // isn't deleted, so the edge doesn't cascade; delete_edges_from_sources
        // clears the file's out-edges before re-inserting the current set.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/src")).unwrap();
        let file = root.join("repo/src/lib.rs");
        std::fs::write(&file, "pub fn keep() { gone(); }\npub fn gone() {}\n").unwrap();

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d3_edge").await;
        let abs = file.to_string_lossy().to_string();
        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &abs);

        process_file(&ctx, &task).await.unwrap();
        let (calls_before,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.edges WHERE folder_id=$1 AND kind='calls'::sensei.edge_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(calls_before, 1, "keep→gone call edge is created on first index");

        // Re-edit: keep no longer calls gone (both fns stay at their lines).
        std::fs::write(&file, "pub fn keep() {}\npub fn gone() {}\n").unwrap();
        process_file(&ctx, &task).await.unwrap();

        let (calls_after,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.edges WHERE folder_id=$1 AND kind='calls'::sensei.edge_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(calls_after, 0, "the removed call's stale edge is reconciled away (replace, not append)");

        ctx.pg().remove_watch_root(&rid).await.ok();
    }

    #[tokio::test]
    async fn process_git_folder_recovers_a_failed_folder_with_no_changes() {
        // Recovery (D6b/D6d): a transient fatal failure a bounded retry later
        // heals leaves the folder `failed` with scan_state complete (no changes).
        // The next scan must RE-DRIVE the barrier — not skip it on
        // has_changes=false — so the folder can reach `indexed`. Here an empty
        // repo marked `failed` is reset to `indexing` and the terminal
        // DetectCommunities barrier is re-enqueued.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo")).unwrap(); // empty → has_changes=false

        let ctx = make_ctx().await;
        let (rid, fid, repo_path) = seed_indexing_repo(&ctx, root, "d6d_recover").await;
        ctx.pg().update_folder_status(&fid, "failed").await.unwrap();

        let task = Task::new(TaskKind::ProcessGitFolder, &repo_path, &repo_path);
        process_git_folder(&ctx, &task).await.unwrap();

        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("indexing"),
            "a `failed` folder is re-driven (reset to `indexing`) even with no changes");
        let has_barrier = ctx.queue.snapshot().await.iter()
            .any(|(kind, fp, _)| *kind == TaskKind::DetectCommunities && fp == &repo_path);
        assert!(has_barrier, "the terminal barrier (DetectCommunities) is re-enqueued so recovery can reach `indexed`");

        ctx.pg().remove_watch_root(&rid).await.ok();
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

    /// A skipped file must be fingerprinted WITH its reason, and re-indexing it
    /// later must CLEAR that reason. Without the fingerprint, `plan_reindex`
    /// treats the file as changed on every pass and re-enqueues it forever; if
    /// the reason were left stale, a file the user fixed would keep reporting as
    /// unscannable.
    #[tokio::test]
    async fn scan_state_records_skip_reason_and_clears_it_on_reindex() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();
        let root_id = ctx.pg().add_watch_root(&repo_path, "skipreason", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "skipreason-repo", &repo_path).await.unwrap();

        // Skipped: fingerprint + reason recorded (exercises the ::enum cast).
        ctx.pg().upsert_scan_state_skipped(
            &fid, "docs/License.txt", 111, "hashA",
            crate::classifiers::ScanSkipReason::InvalidUtf8,
        ).await.unwrap();

        let reason: Option<String> = sqlx_core::query_scalar::query_scalar(
            "SELECT skip_reason::text FROM sensei.scan_state WHERE folder_id = $1 AND file_path = $2"
        ).bind(fid).bind("docs/License.txt")
            .fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(reason.as_deref(), Some("invalid_utf8"), "skip reason persisted");

        // The fingerprint itself must be visible to the change-detection gate —
        // that is what stops the re-enqueue loop.
        let state = ctx.pg().list_scan_state(&fid).await.unwrap();
        assert!(
            state.iter().any(|(p, _)| p == "docs/License.txt"),
            "a skipped file must still carry a fingerprint the mtime gate can match"
        );

        // User fixes the encoding → the file indexes normally → reason cleared.
        ctx.pg().upsert_scan_state(&fid, "docs/License.txt", 222, "hashB").await.unwrap();
        let cleared: Option<String> = sqlx_core::query_scalar::query_scalar(
            "SELECT skip_reason::text FROM sensei.scan_state WHERE folder_id = $1 AND file_path = $2"
        ).bind(fid).bind("docs/License.txt")
            .fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(cleared, None, "re-indexing a fixed file must clear the stale skip reason");
    }

    #[tokio::test]
    async fn unresolve_edges_to_file_nulls_target_keeps_name() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();
        let root_id = ctx.pg().add_watch_root(&repo_path, "ur", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "ur-repo", &repo_path).await.unwrap();

        // funcA lives in a.rs; funcB in b.rs calls it. A call starts UNRESOLVED
        // (target_name only); resolve_edge points it at funcA — the production
        // path that preserves target_name for later re-resolution (D1).
        let node_a = ctx.pg().upsert_node(&fid, "function", "funcA", "a.rs", None, None, None, None).await.unwrap();
        let node_b = ctx.pg().upsert_node(&fid, "function", "funcB", "b.rs", None, None, None, None).await.unwrap();
        let edge = ctx.pg().insert_edge(&fid, &node_b, None, Some("funcA"), None, "calls").await.unwrap();
        ctx.pg().resolve_edge(&edge, &node_a).await.unwrap();

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

    #[tokio::test]
    async fn calls_edge_sourced_from_caller_function_node() {
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let file_abs = src_dir.join("lib.rs");
        std::fs::write(&file_abs, "pub fn caller() { callee(); }\npub fn callee() {}").unwrap();

        let repo_path = tmp.path().to_string_lossy().to_string();
        let root_id = ctx.pg().add_watch_root(&repo_path, "cg", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "cg-repo", &repo_path).await.unwrap();

        let task = Task::for_file(TaskKind::ProcessFile, &repo_path, &file_abs.to_string_lossy());
        process_file(&ctx, &task).await.unwrap();

        let nodes = ctx.pg().get_nodes_by_folder(&fid).await.unwrap();
        let caller_id = nodes.iter()
            .find(|n| n["name"].as_str() == Some("caller") && n["kind"].as_str() == Some("function"))
            .and_then(|n| crate::api::util::json_uuid(&n["id"]))
            .expect("caller function node exists");
        let file_id = nodes.iter()
            .find(|n| n["kind"].as_str() == Some("file"))
            .and_then(|n| crate::api::util::json_uuid(&n["id"]))
            .expect("file node exists");

        let edges = ctx.pg().get_edges_by_kind(&fid, "calls").await.unwrap();
        let edge = edges.iter()
            .find(|e| e["target_name"].as_str() == Some("callee"))
            .expect("a calls edge to callee exists");
        let source_id = crate::api::util::json_uuid(&edge["source_id"]).unwrap();

        assert_eq!(source_id, caller_id, "edge sourced from the caller fn node");
        assert_ne!(source_id, file_id, "edge NOT sourced from the file node");
    }
}
