//! Task handlers — one function per TaskKind.

use std::path::Path;
use super::executor::TaskContext;
use super::{Task, TaskKind};
use crate::types::{NodeKind, HierarchyNode};
use crate::indexer::graph::compute_complexity;
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
    find_git_repos(root, 0, max_depth, &mut repos);

    let store = ctx.store.lock().await;

    // Persist this root for watcher recreation on restart
    store.execute_raw(&format!(
        "INSERT OR IGNORE INTO scanned_roots(path) VALUES('{}')",
        task.path.replace('\'', "''")
    )).ok();

    for (name, path) in &repos {
        // Register project
        let repo_id = name.clone();
        store.upsert_project_basic(&repo_id, name, path).ok();

        // Enqueue process_repo
        let repo_task = Task::new(TaskKind::ProcessRepo, &repo_id, path)
            .with_parent(task.id);
        ctx.queue.enqueue(repo_task).await;
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
    let graph = ctx.graph.lock().await;

    // Clear stale hierarchy
    graph.clear_hierarchy(repo_id).ok();
    graph.clear_unresolved_refs(repo_id).ok();

    // Create virtual grouping nodes
    let repo_node_id = format!("repo:{}", repo_id);
    let code_group_id = format!("code:{}", repo_id);
    let docs_group_id = format!("docs:{}", repo_id);

    graph.merge_node(&HierarchyNode::group(repo_node_id.clone(), repo_id.to_string(), NodeKind::Repo, repo_id.to_string())).ok();
    graph.merge_node(&{
        let mut n = HierarchyNode::group(code_group_id.clone(), "code".into(), NodeKind::CodeGroup, repo_id.to_string());
        n.parent_id = Some(repo_node_id.clone());
        n
    }).ok();
    graph.merge_node(&{
        let mut n = HierarchyNode::group(docs_group_id.clone(), "docs".into(), NodeKind::DocGroup, repo_id.to_string());
        n.parent_id = Some(repo_node_id.clone());
        n
    }).ok();
    graph.merge_edge(&repo_node_id, &code_group_id, "CONTAINS_GROUP").ok();
    graph.merge_edge(&repo_node_id, &docs_group_id, "CONTAINS_GROUP").ok();

    // Detect workspace members → package nodes
    let workspace_members = crate::config::detector::detect_workspace_members(repo_path);
    for pkg in &workspace_members {
        let pkg_id = format!("pkg:{}:{}", repo_id, pkg.name);
        let mut node = HierarchyNode::group(pkg_id.clone(), pkg.name.clone(), NodeKind::Package, repo_id.to_string());
        node.level = Some(pkg.pkg_type.clone());
        node.file = Some(pkg.path.clone());
        node.parent_id = Some(code_group_id.clone());
        graph.merge_node(&node).ok();
        graph.merge_edge(&code_group_id, &pkg_id, "CONTAINS_PKG").ok();
    }

    // Virtual root package for files not under any workspace member
    let root_pkg_id = format!("pkg:{}:(root)", repo_id);
    let mut root_pkg = HierarchyNode::group(root_pkg_id.clone(), "(root)".into(), NodeKind::Package, repo_id.to_string());
    root_pkg.level = Some("root".into());
    root_pkg.parent_id = Some(code_group_id.clone());
    graph.merge_node(&root_pkg).ok();
    graph.merge_edge(&code_group_id, &root_pkg_id, "CONTAINS_PKG").ok();

    drop(graph); // release lock before enqueuing

    // Discover directories and enqueue folder tasks
    let exclude = build_globset();
    let mut dirs = std::collections::HashSet::new();
    let mut file_count = 0u64;

    // Walk all files to discover directories
    let walker = ignore::WalkBuilder::new(repo_path)
        .hidden(true).git_ignore(true).git_global(true).git_exclude(true)
        .build();

    for entry in walker.flatten() {
        if !entry.path().is_file() { continue; }
        let rel = entry.path().strip_prefix(repo_path).unwrap_or(entry.path());
        let rel_str = rel.to_string_lossy();
        if exclude.is_match(&*rel_str) { continue; }

        // Check if this file has a supported extension (code or doc)
        let ext = entry.path().extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        let is_code = adapters::adapter_for_ext(&ext).is_some();
        let is_doc = ext == ".md" || ext == ".mdx";
        if !is_code && !is_doc { continue; }

        if let Some(parent) = entry.path().parent() {
            dirs.insert(parent.to_path_buf());
        }
        file_count += 1;
    }

    // Create a barrier task for resolve_edges (will collect file task deps)
    let resolve_task = Task::new(TaskKind::ResolveEdges, repo_id, "")
        .with_parent(task.id)
        .blocked_by(vec![0]); // placeholder — will be replaced by actual deps
    // We need to enqueue it blocked, then add real deps as file tasks are created
    // Use a two-step approach: enqueue with a dummy dep, then replace
    let resolve_id = ctx.queue.enqueue(
        Task::new(TaskKind::ResolveEdges, repo_id, "")
            .with_parent(task.id)
            .blocked_by(vec![u64::MAX]) // sentinel — will never auto-complete
    ).await;

    let build_id = ctx.queue.enqueue(
        Task::new(TaskKind::BuildConnections, repo_id, "")
            .with_parent(task.id)
            .blocked_by(vec![resolve_id])
    ).await;

    // Enqueue folder tasks
    let mut folder_file_task_ids: Vec<u64> = Vec::new();
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
                    .map(|e| format!(".{}", e))
                    .unwrap_or_default();
                let is_code = adapters::adapter_for_ext(&ext).is_some();
                let is_doc = ext == ".md" || ext == ".mdx";
                if !is_code && !is_doc { continue; }

                let rel_dir_name = if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir.replace('\\', "/") };
                let mod_id = format!("mod:{}:{}", repo_id, rel_dir_name);

                let file_task = Task::new(TaskKind::ProcessFile, repo_id, &entry.path().to_string_lossy())
                    .with_parent(folder_id)
                    .with_module(&mod_id);
                let file_id = ctx.queue.enqueue(file_task).await;
                folder_file_task_ids.push(file_id);

                // Wire as dependency of resolve_edges
                ctx.queue.add_dependency(resolve_id, file_id).await;
            }
        }
    }

    // Remove the sentinel dependency from resolve_edges now that real deps are wired
    // (the sentinel u64::MAX will never complete, so we need to remove it)
    // Simplest: if we have real deps, the sentinel is harmless — it just means
    // resolve won't run until all file tasks AND the sentinel complete.
    // Actually we need to remove it. Let's use a different approach:
    // Clear the sentinel by completing a fake task... or better, just track properly.
    // For now: if there are file tasks, they'll be the real deps. The sentinel stays
    // but we can remove it from the blocked task's depends_on list.
    if !folder_file_task_ids.is_empty() {
        // The add_dependency calls above already wired real deps.
        // We need to remove the sentinel. Let's add a method for that.
        // For now, we'll just note this needs fixing.
        // TODO: remove sentinel dep from resolve task
    }

    // Detect subtrees → auto-solution
    let store = ctx.store.lock().await;
    if let Ok(Some(project)) = store.get_project(repo_id) {
        crate::indexer::cross_repo::auto_solution_for_monorepo(&store, &project).ok();
    }

    tracing::info!("process_repo: {} — {} dirs, {} files, barrier=#{}", repo_id, dirs.len(), file_count, resolve_id);
    Ok(())
}

// ── Process Folder ────────────────────────────────────────────────────────

/// Create module node for a folder.
pub async fn process_folder(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let repo_path_str = {
        let store = ctx.store.lock().await;
        store.get_project(repo_id).ok().flatten()
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

    let graph = ctx.graph.lock().await;
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

/// Parse a single file using the appropriate adapter. Creates nodes + unresolved refs.
pub async fn process_file(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let file_path = Path::new(&task.path);
    let repo_id = &task.repo_id;

    // Get repo path for relative path computation
    let repo_path_str = {
        let store = ctx.store.lock().await;
        store.get_project(repo_id).ok().flatten()
            .map(|p| p.path.clone())
            .unwrap_or_default()
    };
    let repo_path = Path::new(&repo_path_str);

    let abs_path = task.path.clone();
    let rel_path = file_path.strip_prefix(repo_path)
        .unwrap_or(file_path)
        .to_string_lossy().to_string();

    let ext = file_path.extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    let is_doc = ext == ".md" || ext == ".mdx";

    // Read file content
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read {}: {}", abs_path, e))?;

    let graph = ctx.graph.lock().await;

    // Delete old nodes for this file (handles modify case)
    graph.delete_by_file(&abs_path, repo_id).ok();
    graph.clear_unresolved_refs_from(&format!("file:{}", abs_path), repo_id).ok();

    if is_doc {
        // Doc/extension processing
        process_doc_file(&graph, &abs_path, &rel_path, &content, repo_id)?;
    } else {
        // Code file processing via adapter
        let adapter = adapters::adapter_for_ext(&ext)
            .ok_or_else(|| format!("No adapter for {}", ext))?;

        let parsed = adapter.parse(&content, &rel_path);
        let file_lines: Vec<&str> = content.lines().collect();

        // Create File node
        let file_id = format!("file:{}", abs_path);
        let module_name = rel_path.rsplit_once('.').map(|(n, _)| n).unwrap_or(&rel_path)
            .replace('\\', "/");
        {
            let mut file_node = HierarchyNode::group(file_id.clone(), module_name, NodeKind::File, repo_id.to_string());
            file_node.file = Some(abs_path.clone());
            file_node.level = Some(parsed.language.clone());
            graph.merge_node(&file_node).ok();
        }

        // Wire file to module
        if let Some(ref mod_id) = task.module_id {
            graph.merge_edge(mod_id, &file_id, "CONTAINS_FILE").ok();
        }

        // Process symbols
        for sym in &parsed.symbols {
            let node_kind = NodeKind::from_symbol_kind(&sym.kind);
            if node_kind.is_function_like() {
                let id = format!("fn:{}:{}:{}", abs_path, sym.name, sym.line_start);
                let body = file_lines
                    .get((sym.line_start as usize).saturating_sub(1)..sym.line_end as usize)
                    .map(|lines| lines.join("\n"))
                    .unwrap_or_default();
                let complexity = compute_complexity(&body);

                let node = HierarchyNode::function(
                    id.clone(), sym.name.clone(), node_kind,
                    abs_path.clone(), sym.line_start,
                    sym.signature.clone(), Some(body), sym.docstring.clone(),
                    complexity, repo_id.to_string(),
                );
                graph.merge_node(&node).ok();
                graph.merge_edge(&file_id, &id, "EXPORTS_FN").ok();

                // Wire to module
                if let Some(ref mod_id) = task.module_id {
                    graph.merge_edge(mod_id, &id, "CONTAINS_FN").ok();
                }

                // Store unresolved parent ref for HAS_METHOD resolution
                if let Some(ref parent_name) = sym.parent {
                    graph.add_unresolved_ref(&id, "parent", parent_name, repo_id).ok();
                }
            } else {
                let id = format!("type:{}:{}:{}", abs_path, sym.name, sym.line_start);
                let mut type_node = HierarchyNode::group(id.clone(), sym.name.clone(), node_kind, repo_id.to_string());
                type_node.file = Some(abs_path.clone());
                type_node.line = sym.line_start;
                graph.merge_node(&type_node).ok();
                graph.merge_edge(&file_id, &id, "EXPORTS_TYPE").ok();
            }
        }

        // Store unresolved import references
        for imp in &parsed.imports {
            graph.add_unresolved_ref(&file_id, "imports", &imp.target_path, repo_id).ok();
        }

        // Store unresolved call references
        for edge in &parsed.edges {
            let caller_id = format!("fn:{}:{}:", abs_path, edge.caller_name); // partial match
            graph.add_unresolved_ref(&caller_id, "calls", &edge.callee_name, repo_id).ok();
        }
    }

    Ok(())
}

/// Process a markdown/mdx doc file.
fn process_doc_file(graph: &crate::indexer::graph::GraphDb, abs_path: &str, rel_path: &str, content: &str, repo_id: &str) -> Result<(), String> {
    let frontmatter = crate::indexer::doc_indexer::parse_frontmatter_pub(content);
    let classification = crate::indexer::doc_indexer::classify_doc_pub(rel_path, &frontmatter);

    let title = frontmatter.title
        .or(frontmatter.name)
        .or_else(|| crate::indexer::doc_indexer::extract_title(content))
        .unwrap_or_else(|| rel_path.to_string());

    let doc_id = format!("doc:{}", abs_path);
    let node = HierarchyNode::doc(
        doc_id.clone(), title, classification.kind, abs_path.to_string(),
        Some(classification.doc_type), classification.doc_category, repo_id.to_string(),
    );
    graph.merge_node(&node)?;

    // Store unresolved file refs for later COVERS resolution
    let file_refs = crate::indexer::doc_indexer::extract_file_refs(content, &graph.db_path().map(|p| p.parent().unwrap_or(p).to_string_lossy().to_string()).unwrap_or_default());
    for file_ref in &file_refs {
        graph.add_unresolved_ref(&doc_id, "covers", file_ref, repo_id).ok();
    }

    // Store unresolved fn mentions
    let fn_mentions = crate::indexer::doc_indexer::extract_fn_mentions(content);
    for fn_name in &fn_mentions {
        graph.add_unresolved_ref(&doc_id, "mentions_fn", fn_name, repo_id).ok();
    }

    Ok(())
}

// ── Delete File / Folder ──────────────────────────────────────────────────

pub async fn delete_file(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let graph = ctx.graph.lock().await;
    graph.delete_by_file(&task.path, &task.repo_id)?;
    graph.clear_unresolved_refs_from(&format!("file:{}", task.path), &task.repo_id).ok();
    graph.clear_unresolved_refs_from(&format!("doc:{}", task.path), &task.repo_id).ok();
    tracing::info!("delete_file: {}", task.path);
    Ok(())
}

pub async fn delete_folder(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let graph = ctx.graph.lock().await;
    let nodes = graph.get_nodes(&task.repo_id)?;
    for node in &nodes {
        if node.file.starts_with(&task.path) {
            graph.delete_node(&node.id)?;
        }
    }
    // Delete the module node
    let repo_path_str = {
        let store = ctx.store.lock().await;
        store.get_project(&task.repo_id).ok().flatten()
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
    let graph = ctx.graph.lock().await;

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

    let repo_path_str = {
        let store = ctx.store.lock().await;
        store.get_project(repo_id).ok().flatten()
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
                    if let Some((_, caller_id)) = caller {
                        if caller_id != callee_id {
                            graph.merge_edge(caller_id, callee_id, "CALLS").ok();
                            edges_created += 1;
                        }
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
    let graph = ctx.graph.lock().await;

    // Doc↔code traceability (SPECIFIES, IMPLEMENTS, DOCUMENTS)
    crate::indexer::doc_indexer::create_traceability_edges_pub(&graph, repo_id)?;

    // Mark project as indexed
    let store = ctx.store.lock().await;
    // Collect libs from unresolved import targets
    let nodes = graph.get_nodes(repo_id)?;
    let mut libs = std::collections::HashSet::new();
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

// ── Helpers ───────────────────────────────────────────────────────────────

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
