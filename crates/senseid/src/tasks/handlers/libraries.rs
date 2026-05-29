//! Library phase: classify imports as internal vs external, fetch lib docs.

use super::super::executor::TaskContext;
use super::super::Task;
use crate::languages;

// ── Resolve Libs ──────────────────────────────────────────────────────────

/// Classify imports as internal vs external libraries. Update project.libs.
pub async fn resolve_libs(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let mut lib_set = std::collections::HashSet::new();

    // folder_path is the repo abs_path (Task contract); look up the row
    // once and reuse for both repo_path and the later mark_folder_indexed
    // call. folder_name comes from the DB row so subtree composite names
    // like "sensei:homebrew" survive in log messages.
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    let folder_name = folder.as_ref()
        .and_then(|f| f["name"].as_str())
        .unwrap_or_else(|| task.folder_name());
    let repo_path_str = task.folder_path.clone();

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

    // Update folder libs via PgStore (reusing the folder row from above)
    if let Some(folder) = folder.as_ref()
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            ctx.pg().mark_folder_indexed(&folder_id, &libs).await.ok();
        }

    // Enqueue ExtractDeps to parse manifest files and populate referenced_libraries
    let extract_task = super::super::Task::new(
        super::super::TaskKind::ExtractDeps,
        &task.folder_path,
        &task.folder_path,
    ).with_parent(task.id);
    ctx.queue.enqueue(extract_task).await;

    tracing::info!("resolve_libs: {} — {} external libs detected, enqueued ExtractDeps", folder_name, libs.len());
    Ok(libs.len() as u32)
}

// ── Import Lib ────────────────────────────────────────────────────────────

/// User-triggered: fetch and index documentation for an external library.
pub async fn import_lib(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
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
            Ok(1)
        }
        Err(e) => Err(format!("Failed to index lib {}: {}", lib_name, e))
    }
}

// ── Index Library ──────────────────────────────────────────────────────────

/// Fetch library docs from URL, parse into pages, and store each page.
/// Enqueues IndexLibraryPage child tasks for each parsed doc section.
pub async fn index_library(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let lib_name = &task.path;
    let url = task.url.as_deref().unwrap_or("");

    if url.is_empty() {
        return Err("index_library requires a URL (set via task.url)".into());
    }

    let content = crate::indexer::lib_indexer::fetch_lib_url_with_timeout(url, 15).await?;

    if content.len() < 50 {
        return Err(format!("Content too short from {}: {} bytes", url, content.len()));
    }

    // Upsert the library record
    let lib_id = ctx.pg().upsert_library(lib_name, "npm", None, None, Some("url"), Some(url)).await
        .map_err(|e| format!("upsert_library failed: {}", e))?;

    // Parse content into doc sections
    let result = crate::indexer::lib_indexer::index_lib_content(lib_name, url, &content, None)
        .map_err(|e| format!("index_lib_content failed: {}", e))?;

    // Get parsed docs for page storage
    let source_type = crate::indexer::lib_indexer::detect_source_type(url, &content);
    let docs = crate::indexer::lib_indexer::parse_docs(&content, lib_name, url);

    // Store each parsed doc as a library_page
    let mut pages_stored = 0u32;
    for doc in &docs {
        match ctx.pg().upsert_library_page(
            &lib_id,
            &doc.title,
            Some(url),
            Some(&doc.summary),
            Some(&doc.content),
            &source_type,
            doc.component.as_deref(),
        ).await {
            Ok(_) => pages_stored += 1,
            Err(e) => tracing::warn!("Failed to store page '{}': {}", doc.title, e),
        }
    }

    // Update denormalized page_count
    ctx.pg().update_library_page_count(&lib_id).await.ok();

    tracing::info!(
        "index_library: {} — {} pages stored (parsed {} sections) from {}",
        lib_name, pages_stored, result.docs_indexed, url
    );
    Ok(pages_stored)
}

// ── Index Library Page ─────────────────────────────────────────────────────

/// Store a single library documentation page. Currently stores content only
/// (embedding generation deferred to gateway integration).
pub async fn index_library_page(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let lib_id_str = &task.folder_path; // library UUID stored in folder_path
    let title = &task.path;             // page title stored in path

    let lib_id = uuid::Uuid::parse_str(lib_id_str)
        .map_err(|_| format!("Invalid library UUID: {}", lib_id_str))?;

    let url = task.url.as_deref();

    // Verify library exists
    ctx.pg().get_library(&lib_id).await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Library {} not found", lib_id))?;

    // Page content passed via module_id field (reusing available Task fields)
    let content = task.module_id.as_deref().unwrap_or("");
    if content.is_empty() {
        return Err("index_library_page requires content in module_id".into());
    }

    let summary: String = content.lines().take(3).collect::<Vec<_>>().join(" ");
    let summary = &summary[..summary.len().min(200)];

    ctx.pg().upsert_library_page(&lib_id, title, url, Some(summary), Some(content), "http", None).await
        .map_err(|e| format!("upsert_library_page failed: {}", e))?;

    Ok(1)
}

// ── Extract Deps ───────────────────────────────────────────────────────

/// Parse manifest files (package.json, Cargo.toml, pyproject.toml) and upsert
/// detected dependencies into libraries + referenced_libraries.
pub async fn extract_deps(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Folder '{}' not found", task.folder_path))?;

    let folder_name = folder["name"].as_str()
        .unwrap_or_else(|| task.folder_name());
    let folder_id = crate::api::util::json_uuid(&folder["id"])
        .ok_or("Invalid folder id")?;

    let repo_path = folder["abs_path"].as_str()
        .ok_or("Folder has no abs_path")?;

    let deps = crate::indexer::lib_indexer::extract_dep_versions(folder_name, repo_path)?;

    let mut count = 0u32;
    for dep in &deps {
        let ecosystem = match dep.source.as_str() {
            "package.json" => "npm",
            "Cargo.toml" => "crates",
            "pyproject.toml" => "pypi",
            _ => "npm",
        };

        // Upsert the library record (creates if not exists)
        let lib_id = match ctx.pg().upsert_library(&dep.lib_name, ecosystem, Some(&dep.version), None, None, None).await {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!("extract_deps: skip {} — {}", dep.lib_name, e);
                continue;
            }
        };

        // Link folder → library via referenced_libraries
        ctx.pg().upsert_referenced_library(&folder_id, &lib_id, Some(&dep.version)).await.ok();
        count += 1;
    }

    tracing::info!("extract_deps: {} — {} deps from manifests", folder_name, count);
    Ok(count)
}
