//! Library phase: classify imports as internal vs external, fetch lib docs.

use super::super::executor::TaskContext;
use super::super::Task;
use crate::languages;

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
        let adapter = match languages::adapter_for_ext(&ext) {
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
