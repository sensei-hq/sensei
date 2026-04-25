//! Library phase: classify imports as internal vs external, fetch lib docs.

use super::super::executor::TaskContext;
use super::super::Task;
use crate::languages;

// ── Resolve Libs ──────────────────────────────────────────────────────────

/// Classify imports as internal vs external libraries. Update project.libs.
pub async fn resolve_libs(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;

    // TODO: migrate edge-based lib detection to PgStore
    // Old code read nodes/edges from GraphDb to classify imports.
    // For now, use the file-scanning heuristic only.

    let mut lib_set = std::collections::HashSet::new();

    // Re-read source files and extract non-relative imports
    // This is expensive but accurate. TODO: store raw imports in a metadata field.
    let repo_path_str = ctx.pg().get_repo_by_name(repo_id).await
        .ok().flatten()
        .and_then(|r| r["abs_path"].as_str().map(String::from))
        .unwrap_or_default();

    // Walk source files in the repo directory and extract external imports
    if !repo_path_str.is_empty() {
        let repo_path = std::path::Path::new(&repo_path_str);
        let walker = ignore::WalkBuilder::new(repo_path)
            .hidden(true).git_ignore(true).git_global(true).git_exclude(true)
            .build();

        for entry in walker.flatten() {
            if !entry.path().is_file() { continue; }
            let ext = entry.path().extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e))
                .unwrap_or_default();
            let adapter = match languages::adapter_for_ext(&ext) {
                Some(a) => a,
                None => continue,
            };
            let file_path = entry.path().to_string_lossy().to_string();
            let content = match std::fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let rel_path = entry.path().strip_prefix(repo_path)
                .unwrap_or(entry.path())
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
    }

    let mut libs: Vec<String> = lib_set.into_iter().collect();
    libs.sort();

    // Check which libs are internal (match another repo in a project)
    let all_repos = ctx.pg().list_repositories().await.unwrap_or_default();
    let _internal_repos: std::collections::HashSet<String> = all_repos.iter()
        .filter_map(|p| p["name"].as_str().map(|s| s.to_lowercase()))
        .collect();

    // Update folder libs via PgStore
    let folder = ctx.pg().get_repo_by_name(repo_id).await.ok().flatten();
    if let Some(folder) = folder {
        if let Some(folder_id) = folder["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
            ctx.pg().mark_folder_indexed(&folder_id, &libs).await.ok();
        }
    }

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

    match ctx.pg().upsert_library(lib_name, "npm", None, Some(&content), Some("url"), Some(url)).await {
        Ok(lib_id) => {
            tracing::info!("import_lib: {} — indexed as {} from {}", lib_name, lib_id, url);
            Ok(())
        }
        Err(e) => Err(format!("Failed to index lib {}: {}", lib_name, e))
    }
}
