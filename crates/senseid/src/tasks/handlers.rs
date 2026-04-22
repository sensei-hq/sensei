//! Task handlers — one function per TaskKind.

use std::path::Path;
use super::executor::TaskContext;
use super::{Task, TaskKind};
use crate::types::{NodeKind, HierarchyNode, Project};
use crate::adapters;

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

    let store = ctx.store().await;

    // Persist this root for watcher recreation on restart
    store.execute_raw(&format!(
        "INSERT OR IGNORE INTO scanned_roots(path) VALUES('{}')",
        task.path.replace('\'', "''")
    )).ok();

    for (name, path) in &repos {
        // Skip excluded paths
        if store.is_excluded(path) {
            tracing::debug!("scan_root: skipping excluded path {}", path);
            continue;
        }

        // Register project
        let repo_id = name.clone();
        store.upsert_repo_basic(&repo_id, name, path).ok();

        // Enqueue process_repo
        let repo_task = Task::new(TaskKind::ProcessRepo, &repo_id, path)
            .with_parent(task.id);
        ctx.queue.enqueue(repo_task).await;
    }

    // Start a root watcher for this scanned root
    let projects_map: std::collections::HashMap<String, String> = repos.iter()
        .map(|(name, path)| (name.clone(), path.clone()))
        .collect();
    crate::watcher::root_watcher::start_root_watcher(
        std::path::PathBuf::from(&task.path),
        ctx.queue.clone(),
        projects_map,
    ).ok();

    // Suggest solution groupings for discovered repos (parent-folder + name-prefix)
    if repos.len() >= 2 {
        let suggestions = crate::tasks::processors::metadata::suggest_solutions(&repos);
        for suggestion in &suggestions {
            tracing::info!(
                "scan_root: solution suggestion '{}' ({}) — {} repos",
                suggestion.name, suggestion.strategy, suggestion.repo_ids.len()
            );
        }
        // Store suggestions so the desktop setup wizard can present them
        if !suggestions.is_empty() {
            let store = ctx.store().await;
            store.set_config("solution_suggestions", &serde_json::to_string(&suggestions).unwrap_or_default()).ok();
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

// ── Process Repo ──────────────────────────────────────────────────────────

/// Process a repo: create virtual nodes, detect workspaces, enqueue folder + barrier tasks.
pub async fn process_repo(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_path = Path::new(&task.path);
    if !repo_path.exists() {
        return Err(format!("Repo path does not exist: {}", task.path));
    }

    let repo_id = &task.repo_id;
    let graph = ctx.graph().await;

    // Clear stale hierarchy
    graph.clear_hierarchy(repo_id).ok();
    graph.clear_unresolved_refs(repo_id).ok();

    // Create repo node — packages and docs wire directly to it (no code/docs grouping layer)
    let repo_node_id = format!("repo:{}", repo_id);
    graph.merge_node(&HierarchyNode::group(repo_node_id.clone(), repo_id.to_string(), NodeKind::Repo, repo_id.to_string())).ok();

    // Detect workspace members → package nodes (parent = repo, tagged as code)
    let workspace_members = crate::config::detector::detect_workspace_members(repo_path);
    for pkg in &workspace_members {
        let pkg_id = format!("pkg:{}:{}", repo_id, pkg.name);
        let mut node = HierarchyNode::group(pkg_id.clone(), pkg.name.clone(), NodeKind::Package, repo_id.to_string());
        node.level = Some(pkg.pkg_type.clone());
        node.file = Some(pkg.path.clone());
        node.parent_id = Some(repo_node_id.clone());
        node.tags = Some("src".into());
        graph.merge_node(&node).ok();
        graph.merge_edge(&repo_node_id, &pkg_id, "CONTAINS_PKG").ok();
    }

    // Virtual root package for files not under any workspace member
    let root_pkg_id = format!("pkg:{}:(root)", repo_id);
    let mut root_pkg = HierarchyNode::group(root_pkg_id.clone(), "(root)".into(), NodeKind::Package, repo_id.to_string());
    root_pkg.level = Some("root".into());
    root_pkg.parent_id = Some(repo_node_id.clone());
    root_pkg.tags = Some("src".into());
    graph.merge_node(&root_pkg).ok();
    graph.merge_edge(&repo_node_id, &root_pkg_id, "CONTAINS_PKG").ok();

    drop(graph); // release lock before enqueuing

    // Discover directories and enqueue folder tasks
    let exclude = build_globset();
    let mut dirs = std::collections::HashSet::new();

    // Walk all files to discover directories
    let walker = ignore::WalkBuilder::new(repo_path)
        .hidden(true).git_ignore(true).git_global(true).git_exclude(true)
        .build();

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

    // Enqueue folder + file tasks first, collecting all file task IDs
    let mut all_file_task_ids: Vec<u64> = Vec::new();
    for dir in &dirs {
        let rel_dir = dir.strip_prefix(repo_path).unwrap_or(dir)
            .to_string_lossy().to_string();
        let abs_dir = dir.to_string_lossy().to_string();

        // Determine parent package
        let pkg_id = workspace_members.iter()
            .find(|pkg| rel_dir.starts_with(&pkg.path))
            .map(|pkg| format!("pkg:{}:{}", repo_id, pkg.name))
            .unwrap_or_else(|| root_pkg_id.clone());

        let folder_task = Task::new(TaskKind::ProcessFolder, repo_id, &abs_dir)
            .with_parent(task.id);
        // Store pkg_id in module_id field for folder processing
        let mut ft = folder_task;
        ft.module_id = Some(pkg_id);
        let folder_id = ctx.queue.enqueue(ft).await;

        // Enumerate files in this directory (non-recursive)
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if !entry.path().is_file() { continue; }
                let ext = entry.path().extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string();
                if ext.is_empty() { continue; }
                if is_binary_ext(&ext) { continue; }

                let rel_dir_name = if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir.replace('\\', "/") };
                let mod_id = format!("mod:{}:{}", repo_id, rel_dir_name);

                let file_task = Task::new(TaskKind::ProcessFile, repo_id, &entry.path().to_string_lossy())
                    .with_parent(folder_id)
                    .with_module(&mod_id);
                let file_id = ctx.queue.enqueue(file_task).await;
                all_file_task_ids.push(file_id);

                // file_id collected in all_file_task_ids for barrier
            }
        }
    }

    // Emit RepoQueued event with file count so UI can show accurate progress
    let _ = ctx.queue.sender().send(
        crate::tasks::progress::TaskEvent::RepoQueued {
            repo_id: repo_id.to_string(),
            files_total: all_file_task_ids.len() as u32,
        }
    );

    // Remove the sentinel dependency from resolve_edges now that real deps are wired
    // (the sentinel u64::MAX will never complete, so we need to remove it)
    // Simplest: if we have real deps, the sentinel is harmless — it just means
    // resolve won't run until all file tasks AND the sentinel complete.
    // Now create barrier tasks with REAL dependencies (no sentinel)
    let resolve_id = ctx.queue.enqueue(
        Task::new(TaskKind::ResolveEdges, repo_id, "")
            .with_parent(task.id)
            .blocked_by(all_file_task_ids.clone())
    ).await;

    let libs_id = ctx.queue.enqueue(
        Task::new(TaskKind::ResolveLibs, repo_id, "")
            .with_parent(task.id)
            .blocked_by(vec![resolve_id])
    ).await;

    ctx.queue.enqueue(
        Task::new(TaskKind::BuildConnections, repo_id, "")
            .with_parent(task.id)
            .blocked_by(vec![libs_id])
    ).await;

    // Detect subtrees → register as separate repos, create project
    {
        let store = ctx.store().await;
        if let Ok(Some(repo)) = store.get_repo(repo_id) {
            // Detect git subtrees
            let subtrees = crate::indexer::cross_repo::detect_git_subtrees_pub(repo_path);
            if !subtrees.is_empty() {
                // Create project for monorepo
                crate::indexer::cross_repo::auto_project_for_monorepo(&store, &repo).ok();

                // Register and index each subtree as a separate repo
                for (name, subtree_path) in &subtrees {
                    let subtree_repo_id = format!("{}:{}", repo_id, name);
                    store.upsert_repo_basic(&subtree_repo_id, name, subtree_path).ok();
                }
                drop(store); // release lock before enqueuing

                for (name, subtree_path) in &subtrees {
                    let subtree_repo_id = format!("{}:{}", repo_id, name);
                    let sub_task = Task::new(TaskKind::ProcessRepo, &subtree_repo_id, subtree_path)
                        .with_parent(task.id);
                    ctx.queue.enqueue(sub_task).await;
                    tracing::info!("process_repo: enqueued subtree {} at {}", subtree_repo_id, subtree_path);
                }
            }
        }
    }

    // ── Repo-level metadata scanners (fast, filesystem-only) ──────────
    {
        use crate::tasks::processors::metadata;

        let icon = metadata::scan_icons(repo_path);
        let links = metadata::scan_external_links(repo_path);
        let summary = metadata::extract_summary(repo_path);

        // Persist metadata on the project record
        let store = ctx.store().await;
        let meta = serde_json::json!({
            "icon": icon,
            "external_links": links.links,
            "summary": summary,
        });
        store.set_repo_metadata(repo_id, &meta).ok();
    }

    tracing::info!("process_repo: {} — {} dirs, {} file tasks, barrier=#{}", repo_id, dirs.len(), all_file_task_ids.len(), resolve_id);
    Ok(())
}

// ── Process Folder ────────────────────────────────────────────────────────

/// Create module node for a folder.
pub async fn process_folder(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let repo_path_str = {
        let store = ctx.store().await;
        store.get_repo(repo_id).ok().flatten()
            .map(|p| p.path.clone())
            .unwrap_or_default()
    };
    let repo_path = Path::new(&repo_path_str);

    let rel_dir = Path::new(&task.path).strip_prefix(repo_path)
        .unwrap_or(Path::new(&task.path))
        .to_string_lossy().to_string();
    let mod_name = if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir.replace('\\', "/") };
    let mod_id = format!("mod:{}:{}", repo_id, mod_name);

    let pkg_id = task.module_id.clone(); // parent package ID stored here by process_repo

    let graph = ctx.graph().await;
    let mut mod_node = HierarchyNode::group(mod_id.clone(), mod_name, NodeKind::Module, repo_id.to_string());
    mod_node.file = Some(rel_dir);
    mod_node.parent_id = pkg_id.clone();
    graph.merge_node(&mod_node).ok();

    if let Some(ref pid) = pkg_id {
        graph.merge_edge(pid, &mod_id, "CONTAINS_MOD").ok();
    }

    Ok(())
}

// ── Process File ──────────────────────────────────────────────────────────

/// Parse a single file using file_processor, then write results to graph.
pub async fn process_file(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let abs_path = &task.path;

    let repo_path_str = {
        let store = ctx.store().await;
        store.get_repo(repo_id).ok().flatten()
            .map(|p| p.path.clone())
            .unwrap_or_default()
    };

    // Use the testable file processor — no DB dependency in extraction
    let result = super::processors::process_file(abs_path, &repo_path_str, repo_id)?;

    // Write to graph via the separated graph writer
    let graph = ctx.graph().await;
    super::processors::write_to_graph(&graph, &result, repo_id, task.module_id.as_deref())?;

    Ok(())
}

// ── Delete File / Folder ──────────────────────────────────────────────────

pub async fn delete_file(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let graph = ctx.graph().await;
    graph.delete_by_file(&task.path, &task.repo_id)?;
    graph.clear_unresolved_refs_from(&format!("file:{}", task.path), &task.repo_id).ok();
    // file: prefix used for both code and doc leaf files
    tracing::info!("delete_file: {}", task.path);
    Ok(())
}

pub async fn delete_folder(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let graph = ctx.graph().await;
    let nodes = graph.get_nodes(&task.repo_id)?;
    for node in &nodes {
        if node.file.starts_with(&task.path) {
            graph.delete_node(&node.id)?;
        }
    }
    // Delete the module node
    let repo_path_str = {
        let store = ctx.store().await;
        store.get_repo(&task.repo_id).ok().flatten()
            .map(|p| p.path.clone())
            .unwrap_or_default()
    };
    let rel_dir = Path::new(&task.path).strip_prefix(Path::new(&repo_path_str))
        .unwrap_or(Path::new(&task.path))
        .to_string_lossy().replace('\\', "/");
    let mod_id = format!("mod:{}:{}", task.repo_id, if rel_dir.is_empty() { "(root)" } else { &rel_dir });
    graph.delete_node(&mod_id)?;
    tracing::info!("delete_folder: {} ({} nodes checked)", task.path, nodes.len());
    Ok(())
}

// ── Resolve Edges (barrier) ───────────────────────────────────────────────

/// Resolve all unresolved references for a repo: IMPORTS, CALLS, HAS_METHOD, COVERS, MENTIONS_FN.
pub async fn resolve_edges(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let graph = ctx.graph().await;

    let refs = graph.get_unresolved_refs(repo_id)?;
    let nodes = graph.get_nodes(repo_id)?;

    // Build lookup maps
    let file_by_path: std::collections::HashMap<String, String> = nodes.iter()
        .filter(|n| n.kind == "file")
        .map(|n| (n.file.clone(), n.id.clone()))
        .collect();

    let fn_by_name: std::collections::HashMap<String, String> = nodes.iter()
        .filter(|n| ["function", "method", "component", "hook"].contains(&n.kind.as_str()))
        .map(|n| (n.name.clone(), n.id.clone()))
        .collect();

    let type_by_name: std::collections::HashMap<String, String> = nodes.iter()
        .filter(|n| ["class", "struct", "interface", "enum", "type"].contains(&n.kind.as_str()))
        .map(|n| (n.name.clone(), n.id.clone()))
        .collect();

    let _repo_path_str = {
        let store = ctx.store().await;
        store.get_repo(repo_id).ok().flatten()
            .map(|p| p.path.clone())
            .unwrap_or_default()
    };

    let mut edges_created = 0u32;

    for (source_id, ref_kind, ref_target) in &refs {
        match ref_kind.as_str() {
            "imports" => {
                // Resolve relative import: ./bar → find file:abs_path/bar.ts
                if !ref_target.starts_with('.') { continue; }
                let source_file = nodes.iter()
                    .find(|n| n.id == *source_id)
                    .map(|n| n.file.clone())
                    .unwrap_or_default();
                let source_dir = Path::new(&source_file).parent().unwrap_or(Path::new(""));
                let resolved = source_dir.join(ref_target);
                let resolved_str = resolved.to_string_lossy().to_string();

                for suffix in &["", ".ts", ".tsx", ".js", ".jsx", ".py", ".rs"] {
                    let candidate = format!("file:{}{}", resolved_str, suffix);
                    if file_by_path.values().any(|id| *id == candidate) {
                        graph.merge_edge(source_id, &candidate, "IMPORTS").ok();
                        edges_created += 1;
                        break;
                    }
                }
            }
            "calls" => {
                // Resolve function call by name
                if let Some(callee_id) = fn_by_name.get(ref_target) {
                    // Find the actual caller fn ID (source_id is a prefix like fn:path:name:)
                    let caller = fn_by_name.iter()
                        .find(|(_, id)| source_id.starts_with(id.as_str()) || id.starts_with(source_id.as_str()))
                        .or_else(|| fn_by_name.iter().find(|(name, _)| source_id.contains(name.as_str())));
                    if let Some((_, caller_id)) = caller
                        && caller_id != callee_id {
                            graph.merge_edge(caller_id, callee_id, "CALLS").ok();
                            edges_created += 1;
                        }
                }
            }
            "parent" => {
                // Resolve HAS_METHOD: find type by name
                if let Some(type_id) = type_by_name.get(ref_target) {
                    graph.merge_edge(type_id, source_id, "HAS_METHOD").ok();
                    edges_created += 1;
                }
            }
            "covers" => {
                // Resolve doc COVERS file
                let file_id = format!("file:{}", ref_target);
                if file_by_path.values().any(|id| *id == file_id) {
                    graph.merge_edge(source_id, &file_id, "COVERS").ok();
                    edges_created += 1;
                }
            }
            "mentions_fn" => {
                // Resolve doc MENTIONS_FN
                if let Some(fn_id) = fn_by_name.get(ref_target) {
                    graph.merge_edge(source_id, fn_id, "MENTIONS_FN").ok();
                    edges_created += 1;
                }
            }
            _ => {}
        }
    }

    // Framework tagging
    // TODO: migrate framework_tagger to work with hierarchy_nodes

    graph.clear_unresolved_refs(repo_id).ok();
    tracing::info!("resolve_edges: {} — {} refs processed, {} edges created", repo_id, refs.len(), edges_created);
    Ok(())
}

// ── Build Connections ─────────────────────────────────────────────────────

/// Build doc↔code traceability and cross-repo links.
pub async fn build_connections(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let graph = ctx.graph().await;

    // Doc↔code traceability (SPECIFIES, IMPLEMENTS, DOCUMENTS)
    crate::indexer::doc_indexer::create_traceability_edges_pub(&graph, repo_id)?;

    // Mark project as indexed
    let store = ctx.store().await;
    // Collect libs from unresolved import targets
    let nodes = graph.get_nodes(repo_id)?;
    let libs = std::collections::HashSet::new();
    // Simple lib detection from file-level imports
    for node in &nodes {
        if node.kind == "file" {
            // Libs are detected from imports during resolve, but we can also check here
        }
    }
    let lib_vec: Vec<String> = libs.into_iter().collect();
    store.mark_indexed(repo_id, &lib_vec).ok();

    tracing::info!("build_connections: {} complete", repo_id);
    Ok(())
}

// ── Resolve Libs ──────────────────────────────────────────────────────────

/// Classify imports as internal vs external libraries. Update project.libs.
pub async fn resolve_libs(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let graph = ctx.graph().await;

    let nodes = graph.get_nodes(repo_id)?;
    let edges = graph.get_edges(repo_id)?;

    // Collect all import targets from IMPORTS edges
    let mut lib_set = std::collections::HashSet::new();
    let file_ids: std::collections::HashSet<String> = nodes.iter()
        .filter(|n| n.kind == "file")
        .map(|n| n.id.clone())
        .collect();

    // Get raw import paths from unresolved_refs (already cleared by resolve_edges,
    // so we need to look at IMPORTS edges + node metadata)
    // Actually, imports are resolved into IMPORTS edges. External libs are imports
    // that did NOT resolve to a local file. We can detect them by looking at
    // the original import targets stored during process_file.
    // Since unresolved_refs were cleared, let's scan node-level data instead.

    // Simpler approach: scan all file nodes' level field (which stores language)
    // and use the existing lib detection logic from the old pipeline
    for edge in &edges {
        if edge.edge_type == "IMPORTS" {
            // If target is NOT a local file node, it's an external lib
            if !file_ids.contains(&edge.target) {
                // Extract lib name from the target
                let target = &edge.target;
                if !target.starts_with("file:") {
                    lib_set.insert(target.clone());
                }
            }
        }
    }

    // Also check what we can infer from existing data
    // For now, use a simpler heuristic: re-read source files and extract non-relative imports
    // This is expensive but accurate. TODO: store raw imports in a metadata field.
    let repo_path_str = {
        let store = ctx.store().await;
        store.get_repo(repo_id).ok().flatten()
            .map(|p| p.path.clone())
            .unwrap_or_default()
    };

    // Walk source files and extract external imports
    for node in &nodes {
        if node.kind != "file" || node.file.is_empty() { continue; }
        let ext = std::path::Path::new(&node.file).extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        let adapter = match adapters::adapter_for_ext(&ext) {
            Some(a) => a,
            None => continue,
        };
        let content = match std::fs::read_to_string(&node.file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let rel_path = std::path::Path::new(&node.file).strip_prefix(&repo_path_str)
            .unwrap_or(std::path::Path::new(&node.file))
            .to_string_lossy().to_string();
        let parsed = adapter.parse(&content, &rel_path);

        for imp in &parsed.imports {
            let path = &imp.target_path;
            // Skip relative, absolute, node builtins, framework aliases
            if path.starts_with('.') || path.starts_with('/') || path.starts_with("node:") { continue; }
            if path.starts_with('$') { continue; }
            if ["fs","path","os","url","http","https","module","child_process","crypto","util",
                "events","stream","buffer","net","dns","tls","cluster","worker_threads",
                "perf_hooks","process","assert","readline","querystring","string_decoder","zlib"]
                .contains(&path.as_str()) { continue; }
            if path.starts_with("crate::") || path.starts_with("self::") || path.starts_with("super::") { continue; }
            if path.starts_with("std::") || path.starts_with("core::") || path.starts_with("alloc::") { continue; }
            if path.starts_with("java.") || path.starts_with("javax.") { continue; }
            if path.starts_with("import_") || path.starts_with("from_") { continue; }
            if path.starts_with("pub use") || path.starts_with("pub(crate)") { continue; }

            let lib_name = if path.starts_with('@') {
                path.split('/').next().unwrap_or("").trim_start_matches('@').to_string()
            } else if path.contains("::") {
                path.split("::").next().unwrap_or("").to_string()
            } else {
                path.split('/').next().unwrap_or("").to_string()
            };
            if !lib_name.is_empty() && lib_name.len() > 1 {
                lib_set.insert(lib_name);
            }
        }
    }

    let mut libs: Vec<String> = lib_set.into_iter().collect();
    libs.sort();

    // Check which libs are internal (match another repo in a project)
    let store = ctx.store().await;
    let all_repos = store.list_repos().unwrap_or_default();
    let _internal_repos: std::collections::HashSet<String> = all_repos.iter()
        .map(|p| p.name.to_lowercase())
        .collect();

    // Update repo libs
    store.mark_indexed(repo_id, &libs).ok();

    tracing::info!("resolve_libs: {} — {} external libs detected", repo_id, libs.len());
    Ok(())
}

// ── Import Lib ────────────────────────────────────────────────────────────

/// User-triggered: fetch and index documentation for an external library.
pub async fn import_lib(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let lib_name = &task.path; // lib name stored in path field
    let url = task.url.as_deref().unwrap_or("");

    if url.is_empty() {
        return Err("import_lib requires a URL".into());
    }

    // Fetch content from URL
    let content = reqwest::get(url).await
        .map_err(|e| format!("Failed to fetch {}: {}", url, e))?
        .text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let store = ctx.store().await;
    match crate::indexer::lib_indexer::index_lib_content(&store, lib_name, url, &content, None) {
        Ok(result) => {
            tracing::info!("import_lib: {} — {} docs indexed from {}", lib_name, result.docs_indexed, result.source_type);
            Ok(())
        }
        Err(e) => Err(format!("Failed to index lib {}: {}", lib_name, e))
    }
}

// ── Branch Switch ─────────────────────────────────────────────────────────

/// Handle a git branch switch: snapshot current graph, reindex for new branch.
pub async fn branch_switch(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let new_branch = task.branch.as_deref().ok_or("branch_switch requires branch field")?;

    // Detect current branch from git
    let repo_path = {
        let store = ctx.store().await;
        store.get_repo(repo_id).ok().flatten()
            .map(|p| p.path.clone())
            .ok_or("Project not found")?
    };

    let old_branch = detect_git_branch(&repo_path).unwrap_or_else(|| "unknown".to_string());
    let old_branch_project = format!("{}@{}", repo_id, old_branch);
    let new_branch_project = format!("{}@{}", repo_id, new_branch);

    let graph = ctx.graph().await;

    // Check if we already have a snapshot for the new branch
    if graph.project_exists(&new_branch_project) {
        // Restore: swap current graph with the branch snapshot
        // 1. Save current as old_branch snapshot (if not already saved)
        if !graph.project_exists(&old_branch_project) {
            graph.clone_project_graph(repo_id, &old_branch_project)?;
            tracing::info!("branch_switch: saved {}  snapshot ({} nodes)", old_branch, graph.count_by_kind(&old_branch_project)?.values().sum::<u32>());
        }
        // 2. Clear current graph
        graph.delete_project_graph(repo_id)?;
        // 3. Restore new branch snapshot as current
        graph.clone_project_graph(&new_branch_project, repo_id)?;
        tracing::info!("branch_switch: restored {} from snapshot", new_branch);
        return Ok(());
    }

    // No snapshot for new branch — save current and reindex
    // 1. Save current as old_branch snapshot
    if !graph.project_exists(&old_branch_project) {
        graph.clone_project_graph(repo_id, &old_branch_project)?;
        tracing::info!("branch_switch: saved {} snapshot", old_branch);
    }

    drop(graph); // release lock before enqueuing

    // 2. Clear manifest for full re-parse
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

// ── Reconcile Connections ──────────────────────────────────────────────────

/// Re-evaluate cross-repo edges after a branch switch or repo update.
pub async fn reconcile_connections(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let store = ctx.store().await;
    let graph = ctx.graph().await;

    // Find the project this repo belongs to
    let repo_data = store.get_repo(repo_id).ok().flatten();
    let my_projects: Vec<Project> = if let Some(pid) = repo_data.as_ref().and_then(|r| r.project_id.as_ref()) {
        store.list_projects().unwrap_or_default().into_iter().filter(|p| p.id == *pid).collect()
    } else {
        vec![]
    };

    if my_projects.is_empty() {
        tracing::info!("reconcile_connections: {} not in any project", repo_id);
        return Ok(());
    }

    for proj in &my_projects {
        match crate::indexer::cross_repo::analyze_project(&store, &graph, proj) {
            Ok(analysis) => {
                tracing::info!(
                    "reconcile_connections: project {} — {} links, {} shared libs",
                    proj.id, analysis.links.len(), analysis.shared_libs.len()
                );
            }
            Err(e) => tracing::warn!("reconcile failed for {}: {}", proj.id, e),
        }
    }

    // Rebuild doc↔code traceability
    crate::indexer::doc_indexer::create_traceability_edges_pub(&graph, repo_id)?;

    tracing::info!("reconcile_connections: {} — {} projects", repo_id, my_projects.len());
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────


/// Classify a file as src, test, or e2e based on path/name patterns.
fn is_binary_ext(ext: &str) -> bool {
    ["png","jpg","jpeg","gif","ico","svg","woff","woff2","ttf","eot",
     "zip","tar","gz","bz2","xz","7z","rar",
     "exe","dll","so","dylib","o","a","lib",
     "db","sqlite","sqlite3","profraw",
     "wasm","map","DS_Store","lock"].contains(&ext)
}

fn build_globset() -> globset::GlobSet {
    let patterns = &[
        "**/node_modules/**", "**/dist/**", "**/build/**", "**/target/**",
        "**/.next/**", "**/.svelte-kit/**",
        "**/__pycache__/**", "**/.venv/**", "**/venv/**",
        "**/*.spec.ts", "**/*.spec.tsx", "**/*.spec.js",
        "**/*.test.ts", "**/*.test.tsx", "**/*.test.js",
        "**/*_test.py", "**/*_test.go", "**/*_test.rs",
        "**/*.d.ts",
    ];
    let mut builder = globset::GlobSetBuilder::new();
    for p in patterns {
        if let Ok(g) = globset::Glob::new(p) { builder.add(g); }
    }
    builder.build().unwrap_or_else(|_| globset::GlobSetBuilder::new().build().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use crate::db::Store;
    use crate::indexer::graph::GraphDb;
    use crate::tasks::queue::TaskQueue;
    use crate::tasks::{Task, TaskKind};
    use crate::api::routes::SharedState;
    use super::super::executor::TaskContext;

    /// Build a TaskContext backed by in-memory Store + GraphDb and a fresh TaskQueue.
    fn make_ctx() -> Arc<TaskContext> {
        let store = Store::open_memory().unwrap();
        let graph = GraphDb::open_memory().unwrap();
        let queue = Arc::new(TaskQueue::new());
        let app_state = Arc::new(SharedState {
            store: Mutex::new(store),
            graph: Mutex::new(graph),
            task_queue: queue.clone(),
        });
        Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
        })
    }

    // ── Pure function tests ──────────────────────────────────────────

    #[test]
    fn is_binary_ext_recognises_binaries() {
        assert!(is_binary_ext("png"));
        assert!(is_binary_ext("exe"));
        assert!(is_binary_ext("wasm"));
        assert!(is_binary_ext("sqlite3"));
        assert!(is_binary_ext("lock"));
    }

    #[test]
    fn is_binary_ext_rejects_source_extensions() {
        assert!(!is_binary_ext("rs"));
        assert!(!is_binary_ext("ts"));
        assert!(!is_binary_ext("py"));
        assert!(!is_binary_ext("md"));
        assert!(!is_binary_ext("json"));
        assert!(!is_binary_ext(""));
    }

    #[test]
    fn build_globset_matches_excluded_paths() {
        let gs = build_globset();
        assert!(gs.is_match("node_modules/foo/bar.js"));
        assert!(gs.is_match("src/foo.spec.ts"));
        assert!(gs.is_match("src/foo.test.tsx"));
        assert!(gs.is_match("tests/foo_test.py"));
        assert!(gs.is_match("pkg/foo_test.go"));
        assert!(gs.is_match("src/types.d.ts"));
        assert!(gs.is_match("dist/bundle.js"));
        assert!(gs.is_match("target/debug/foo"));
        assert!(gs.is_match("__pycache__/foo.pyc"));
    }

    #[test]
    fn build_globset_allows_normal_source_files() {
        let gs = build_globset();
        assert!(!gs.is_match("src/main.rs"));
        assert!(!gs.is_match("lib/utils.ts"));
        assert!(!gs.is_match("app.py"));
        assert!(!gs.is_match("docs/readme.md"));
    }

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
        let ctx = make_ctx();
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

        let ctx = make_ctx();
        let task = Task::new(TaskKind::ScanRoot, "", &tmp.path().to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        // Two ProcessRepo tasks should have been enqueued
        let status = ctx.queue.status().await;
        assert_eq!(status.pending, 2, "expected 2 process_repo tasks enqueued");

        // Verify repos were registered in store
        let store = ctx.store().await;
        let repos = store.list_repos().unwrap();
        let names: Vec<&str> = repos.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
    }

    #[tokio::test]
    async fn scan_root_registers_root_itself_when_it_is_a_git_repo() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::write(tmp.path().join("lib.rs"), "pub fn x() {}").unwrap();

        let ctx = make_ctx();
        let task = Task::new(TaskKind::ScanRoot, "", &tmp.path().to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        let status = ctx.queue.status().await;
        assert_eq!(status.pending, 1, "root repo itself should be enqueued");
    }

    #[tokio::test]
    async fn process_repo_errors_on_nonexistent_path() {
        let ctx = make_ctx();
        let task = Task::new(TaskKind::ProcessRepo, "test-repo", "/nonexistent/repo");
        let result = process_repo(&ctx, &task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn process_folder_creates_module_node() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();

        let ctx = make_ctx();
        let repo_id = "test-repo";
        let repo_path = tmp.path().to_string_lossy().to_string();

        // Register the project so process_folder can look up its path
        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, &repo_path).unwrap();
        }

        let pkg_id = format!("pkg:{}:(root)", repo_id);

        // Create the parent package node so get_edges can find it
        {
            let graph = ctx.graph().await;
            let pkg_node = HierarchyNode::group(
                pkg_id.clone(), "(root)".to_string(),
                NodeKind::Package, repo_id.to_string(),
            );
            graph.merge_node(&pkg_node).unwrap();
        }

        let mut task = Task::new(TaskKind::ProcessFolder, repo_id, &src_dir.to_string_lossy());
        task.module_id = Some(pkg_id.clone());

        process_folder(&ctx, &task).await.unwrap();

        // Verify module node was created (package + module = 2 nodes)
        let graph = ctx.graph().await;
        let nodes = graph.get_nodes(repo_id).unwrap();
        assert_eq!(nodes.len(), 2);
        let module_node = nodes.iter().find(|n| n.kind == "module").expect("module node");
        assert_eq!(module_node.name, "src");

        // Verify edge from package to module
        let edges = graph.get_edges(repo_id).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, "CONTAINS_MOD");
        assert_eq!(edges[0].source, pkg_id);
    }

    #[tokio::test]
    async fn delete_file_removes_nodes_and_edges() {
        let ctx = make_ctx();
        let repo_id = "test-repo";

        // Seed graph with two file nodes and an edge
        {
            let graph = ctx.graph().await;
            graph.merge_file("file:/tmp/a.rs", "/tmp/a.rs", "mod_a", "rust", repo_id).unwrap();
            graph.merge_function("fn:/tmp/a.rs:foo:1", "foo", "/tmp/a.rs", 1, "", "", "", 1, repo_id).unwrap();
            graph.merge_file("file:/tmp/b.rs", "/tmp/b.rs", "mod_b", "rust", repo_id).unwrap();
            graph.merge_edge("file:/tmp/a.rs", "file:/tmp/b.rs", "IMPORTS").unwrap();
        }

        let task = Task::new(TaskKind::DeleteFile, repo_id, "/tmp/a.rs");
        delete_file(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let nodes = graph.get_nodes(repo_id).unwrap();
        // Only file b should remain; a.rs and its function were deleted
        let remaining_files: Vec<&str> = nodes.iter()
            .filter(|n| n.kind == "file")
            .map(|n| n.file.as_str())
            .collect();
        assert_eq!(remaining_files, vec!["/tmp/b.rs"]);
    }

    #[tokio::test]
    async fn delete_folder_removes_module_and_child_nodes() {
        let ctx = make_ctx();
        let repo_id = "test-repo";
        let repo_path = "/tmp/myrepo";

        // Register project
        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, repo_path).unwrap();
        }

        // Seed graph: module node + file node under /tmp/myrepo/src/
        {
            let graph = ctx.graph().await;
            graph.merge_module("mod:test-repo:src", "src", "src", None, repo_id).unwrap();
            graph.merge_file("file:src/main.rs", "/tmp/myrepo/src/main.rs", "main", "rust", repo_id).unwrap();
            graph.merge_function("fn:src/main.rs:main:1", "main", "/tmp/myrepo/src/main.rs", 1, "", "", "", 1, repo_id).unwrap();
        }

        let task = Task::new(TaskKind::DeleteFolder, repo_id, "/tmp/myrepo/src");
        delete_folder(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let nodes = graph.get_nodes(repo_id).unwrap();
        // All nodes under /tmp/myrepo/src should be deleted, plus the module node
        assert!(nodes.is_empty(), "all nodes under deleted folder should be removed, got: {:?}",
            nodes.iter().map(|n| &n.id).collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn resolve_edges_resolves_calls_and_parent_refs() {
        let ctx = make_ctx();
        let repo_id = "test-repo";

        // Seed graph with functions and types
        {
            let graph = ctx.graph().await;
            graph.merge_function("fn:a:caller:1", "caller", "a.rs", 1, "", "", "", 1, repo_id).unwrap();
            graph.merge_function("fn:a:callee:10", "callee", "a.rs", 10, "", "", "", 1, repo_id).unwrap();
            graph.merge_type("type:a:MyStruct:1", "MyStruct", "a.rs", 1, "struct", repo_id).unwrap();
            graph.merge_function("fn:a:my_method:20", "my_method", "a.rs", 20, "", "", "", 1, repo_id).unwrap();

            // Add unresolved refs: a call and a parent (HAS_METHOD)
            graph.add_unresolved_ref("fn:a:caller:1", "calls", "callee", repo_id).unwrap();
            graph.add_unresolved_ref("fn:a:my_method:20", "parent", "MyStruct", repo_id).unwrap();
        }

        // Register project (resolve_edges reads project path)
        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, "/tmp/repo").unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, repo_id, "");
        resolve_edges(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let edges = graph.get_edges(repo_id).unwrap();
        let edge_types: Vec<&str> = edges.iter().map(|e| e.edge_type.as_str()).collect();
        assert!(edge_types.contains(&"CALLS"), "expected a CALLS edge, got: {:?}", edge_types);
        assert!(edge_types.contains(&"HAS_METHOD"), "expected a HAS_METHOD edge, got: {:?}", edge_types);

        // Unresolved refs should be cleared
        let refs = graph.get_unresolved_refs(repo_id).unwrap();
        assert!(refs.is_empty(), "unresolved refs should be cleared after resolve_edges");
    }

    #[tokio::test]
    async fn resolve_edges_resolves_covers_refs() {
        let ctx = make_ctx();
        let repo_id = "test-repo";

        {
            let graph = ctx.graph().await;
            graph.merge_file("file:src/main.rs", "src/main.rs", "main", "rust", repo_id).unwrap();
            graph.merge_doc("doc:docs/arch.md", "docs/arch.md", "Architecture", "design", repo_id).unwrap();
            // Unresolved ref: doc covers file
            graph.add_unresolved_ref("doc:docs/arch.md", "covers", "src/main.rs", repo_id).unwrap();
        }

        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, "/tmp/repo").unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, repo_id, "");
        resolve_edges(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let edges = graph.get_edges(repo_id).unwrap();
        let covers: Vec<_> = edges.iter().filter(|e| e.edge_type == "COVERS").collect();
        assert_eq!(covers.len(), 1);
        assert_eq!(covers[0].source, "doc:docs/arch.md");
        assert_eq!(covers[0].target, "file:src/main.rs");
    }

    #[tokio::test]
    async fn resolve_edges_resolves_mentions_fn_refs() {
        let ctx = make_ctx();
        let repo_id = "test-repo";

        {
            let graph = ctx.graph().await;
            graph.merge_function("fn:a:process:5", "process", "a.rs", 5, "", "", "", 1, repo_id).unwrap();
            graph.merge_doc("doc:docs/api.md", "docs/api.md", "API", "design", repo_id).unwrap();
            graph.add_unresolved_ref("doc:docs/api.md", "mentions_fn", "process", repo_id).unwrap();
        }

        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, "/tmp/repo").unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, repo_id, "");
        resolve_edges(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let edges = graph.get_edges(repo_id).unwrap();
        let mentions: Vec<_> = edges.iter().filter(|e| e.edge_type == "MENTIONS_FN").collect();
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].target, "fn:a:process:5");
    }

    #[tokio::test]
    async fn resolve_edges_skips_non_relative_imports() {
        let ctx = make_ctx();
        let repo_id = "test-repo";

        {
            let graph = ctx.graph().await;
            graph.merge_file("file:src/main.rs", "src/main.rs", "main", "rust", repo_id).unwrap();
            // Non-relative import: should be skipped
            graph.add_unresolved_ref("file:src/main.rs", "imports", "react", repo_id).unwrap();
        }

        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, "/tmp/repo").unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, repo_id, "");
        resolve_edges(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let edges = graph.get_edges(repo_id).unwrap();
        let imports: Vec<_> = edges.iter().filter(|e| e.edge_type == "IMPORTS").collect();
        assert!(imports.is_empty(), "non-relative imports should not produce edges");
    }

    #[tokio::test]
    async fn resolve_edges_with_no_refs_is_noop() {
        let ctx = make_ctx();
        let repo_id = "test-repo";

        {
            let graph = ctx.graph().await;
            graph.merge_file("file:a.rs", "a.rs", "a", "rust", repo_id).unwrap();
        }
        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, "/tmp/repo").unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, repo_id, "");
        resolve_edges(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let edges = graph.get_edges(repo_id).unwrap();
        assert!(edges.is_empty());
    }

    // ── Tests for handlers that need external services ────────────────
    // import_lib: requires network (reqwest). Tested via integration tests.
    // branch_switch: requires git repo with branch history. Tested via integration tests.
    // reconcile_connections: requires cross_repo module + solutions. Tested via integration tests.
    // build_connections: requires doc_indexer traceability logic. Tested via integration tests.
    // process_repo: deeply integrated (walks FS, detects workspaces, enqueues many tasks,
    //   starts root watcher). Covered by integration_tests.rs with a real temp repo.
    // process_file: delegates to processors::process_file + write_to_graph, which have
    //   dedicated tests in processors/tests.rs.
}
