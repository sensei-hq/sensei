use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use ignore::WalkBuilder;
use globset::{Glob, GlobSet, GlobSetBuilder};
use crate::types::{IndexResult, SymbolKind};
use crate::adapters;
use super::manifest::{Manifest, file_hash, file_mtime};
use super::graph::{GraphDb, is_function_like, compute_complexity};

/// Additional excludes beyond .gitignore (build artifacts, test files, etc.)
const DEFAULT_EXCLUDE: &[&str] = &[
    "**/node_modules/**", "**/dist/**", "**/build/**", "**/target/**",
    "**/.next/**", "**/.svelte-kit/**",
    "**/__pycache__/**", "**/.venv/**", "**/venv/**",
    "**/*.spec.ts", "**/*.spec.tsx", "**/*.spec.js",
    "**/*.test.ts", "**/*.test.tsx", "**/*.test.js",
    "**/*_test.py", "**/*_test.go", "**/*_test.rs",
    "**/*.d.ts",
];

/// Index a repository into the graph database.
pub fn index_repo(
    graph_db: &GraphDb,
    repo_path: &str,
    repo_id: &str,
) -> Result<IndexResult, String> {
    index_repo_with_progress(graph_db, repo_path, repo_id, &super::queue::NoProgress)
}

/// Index a repository with progress reporting.
pub fn index_repo_with_progress(
    graph_db: &GraphDb,
    repo_path: &str,
    repo_id: &str,
    progress: &dyn super::queue::ProgressCallback,
) -> Result<IndexResult, String> {
    let start = Instant::now();
    let repo = Path::new(repo_path);

    if !repo.exists() {
        return Err(format!("Repo path does not exist: {}", repo_path));
    }

    let mut manifest = Manifest::load(repo_id);

    // Build additional exclude glob set (beyond .gitignore)
    let exclude = build_globset(DEFAULT_EXCLUDE);

    // Discover files — uses .gitignore automatically via ignore crate
    let files: Vec<PathBuf> = WalkBuilder::new(repo)
        .hidden(true)          // respect hidden files (skip .dotfiles)
        .git_ignore(true)      // respect .gitignore
        .git_global(true)      // respect global gitignore
        .git_exclude(true)     // respect .git/info/exclude
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            let rel = e.path().strip_prefix(repo).unwrap_or(e.path());
            let rel_str = rel.to_string_lossy();
            !exclude.is_match(&*rel_str)
        })
        .filter(|e| {
            let ext = e.path().extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e))
                .unwrap_or_default();
            adapters::adapter_for_ext(&ext).is_some()
        })
        .map(|e| e.into_path())
        .collect();

    let files_total = files.len() as u32;
    let mut files_indexed = 0u32;
    let mut files_skipped = 0u32;
    let mut files_failed = 0u32;
    let mut functions_indexed = 0u32;
    let mut types_indexed = 0u32;
    let mut packages_indexed = 0u32;
    let mut modules_indexed = 0u32;
    let mut edges_created = 0u32;

    // Collect all parsed results for cross-file edge resolution
    struct FileResult {
        abs_path: String,
        fn_ids: HashMap<String, String>, // name -> id
        type_ids: HashMap<String, String>, // name -> id (classes/structs/enums/interfaces)
        method_parents: Vec<(String, String)>, // (method_id, parent_class_name)
        imports: Vec<crate::types::ParsedImport>,
        call_edges: Vec<(String, String, String)>, // caller_name, callee_name, caller_id
    }
    let mut all_results: HashMap<String, FileResult> = HashMap::new();

    let started_at = chrono::Utc::now().to_rfc3339();

    // Pass 0: Detect and create Package nodes (workspace members)
    // Clear stale hierarchy first — packages/modules are recreated each run
    graph_db.clear_hierarchy(repo_id).ok();
    let workspace_members = crate::config::detector::detect_workspace_members(repo);
    let _file_to_package: HashMap<String, String> = HashMap::new(); // reserved for future file→package mapping
    for pkg in &workspace_members {
        let pkg_id = format!("pkg:{}:{}", repo_id, pkg.name);
        graph_db.merge_package(
            &pkg_id, &pkg.name, pkg.version.as_deref(),
            &pkg.path, &pkg.pkg_type, repo_id,
        ).ok();
        // Create CONTAINS_PKG edge: project → package
        let project_node = format!("project:{}", repo_id);
        graph_db.merge_edge(&project_node, &pkg_id, "CONTAINS_PKG").ok();
        edges_created += 1;
        packages_indexed += 1;
    }

    // Pass 1: Parse files and create nodes
    for file_path in &files {
        let abs_path = file_path.to_string_lossy().to_string();
        let rel_path = file_path.strip_prefix(repo)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        // Report progress
        progress.on_file(crate::types::IndexProgress {
            repo_id: repo_id.to_string(),
            current_file: rel_path.clone(),
            files_processed: files_indexed + files_skipped + files_failed,
            files_total,
            files_unchanged: files_skipped,
            files_skipped: files_skipped,
            files_failed,
            started_at: started_at.clone(),
        });

        // Manifest check — skip unchanged files
        if let Ok(mtime) = file_mtime(file_path) {
            if !manifest.is_changed(&abs_path, mtime) {
                files_skipped += 1;
                continue;
            }
            // Hash check for mtime-unreliable cases
            if let Ok(hash) = file_hash(file_path) {
                if !manifest.is_hash_changed(&abs_path, &hash) {
                    manifest.record(&abs_path, mtime, hash);
                    files_skipped += 1;
                    continue;
                }
            }
        }

        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();

        let adapter = match adapters::adapter_for_ext(&ext) {
            Some(a) => a,
            None => { files_skipped += 1; continue; }
        };

        // Read and parse
        let source = match std::fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                files_failed += 1;
                tracing::warn!("Failed to read {}: {}", abs_path, e);
                continue;
            }
        };

        let parsed = adapter.parse(&source, &rel_path);
        let file_lines: Vec<&str> = source.lines().collect();

        // Create File node
        let file_id = format!("file:{}", abs_path);
        let module_name = rel_path.rsplit_once('.').map(|(n, _)| n).unwrap_or(&rel_path)
            .replace('\\', "/");
        graph_db.merge_file(&file_id, &abs_path, &module_name, &parsed.language, repo_id)
            .map_err(|e| { if !e.is_empty() { tracing::warn!("merge_file: {}", e); } })
            .ok();

        let mut fn_ids: HashMap<String, String> = HashMap::new();
        let mut type_ids: HashMap<String, String> = HashMap::new();
        let mut method_parents: Vec<(String, String)> = Vec::new();
        let mut call_edges = Vec::new();

        for sym in &parsed.symbols {
            if is_function_like(&sym.kind) {
                let id = format!("fn:{}:{}:{}", abs_path, sym.name, sym.line_start);
                let body = file_lines
                    .get((sym.line_start as usize).saturating_sub(1)..sym.line_end as usize)
                    .map(|lines| lines.join("\n"))
                    .unwrap_or_default();
                let complexity = compute_complexity(&body);

                graph_db.merge_function(
                    &id, &sym.name, &abs_path, sym.line_start,
                    sym.signature.as_deref().unwrap_or(""),
                    &body, sym.docstring.as_deref().unwrap_or(""),
                    complexity, repo_id,
                ).map_err(|e| { if !e.is_empty() { tracing::warn!("merge_fn: {}", e); } }).ok();

                functions_indexed += 1;
                fn_ids.insert(sym.name.clone(), id.clone());

                graph_db.merge_edge(&file_id, &id, "EXPORTS_FN").ok();
                edges_created += 1;

                // Track parent for HAS_METHOD edges
                if let Some(ref parent_name) = sym.parent {
                    method_parents.push((id.clone(), parent_name.clone()));
                }

                // Collect call edges for later resolution
                for edge in &parsed.edges {
                    if edge.caller_name == sym.name {
                        call_edges.push((edge.caller_name.clone(), edge.callee_name.clone(), id.clone()));
                    }
                }
            } else {
                let id = format!("type:{}:{}:{}", abs_path, sym.name, sym.line_start);
                graph_db.merge_type(
                    &id, &sym.name, &abs_path, sym.line_start,
                    &sym.kind.to_string(), repo_id,
                ).map_err(|e| { if !e.is_empty() { tracing::warn!("merge_type: {}", e); } }).ok();

                types_indexed += 1;
                type_ids.insert(sym.name.clone(), id.clone());
                graph_db.merge_edge(&file_id, &id, "EXPORTS_TYPE").ok();
                edges_created += 1;
            }
        }

        // Record in manifest
        if let (Ok(mtime), Ok(hash)) = (file_mtime(file_path), file_hash(file_path)) {
            manifest.record(&abs_path, mtime, hash);
        }

        all_results.insert(abs_path, FileResult { abs_path: file_path.to_string_lossy().to_string(), fn_ids, type_ids, method_parents, imports: parsed.imports, call_edges });
        files_indexed += 1;
    }

    // Pass 2: IMPORTS edges
    for (abs_path, result) in &all_results {
        let file_id = format!("file:{}", abs_path);
        let file_dir = Path::new(abs_path).parent().unwrap_or(Path::new(""));

        for imp in &result.imports {
            if !imp.target_path.starts_with('.') { continue; }
            let resolved = file_dir.join(&imp.target_path);
            let resolved_str = resolved.to_string_lossy().to_string();
            // Try with extensions
            for suffix in &["", ".ts", ".tsx", ".js", ".jsx", ".py", ".rs"] {
                let candidate = format!("{}{}", resolved_str, suffix);
                if all_results.contains_key(&candidate) {
                    graph_db.merge_edge(&file_id, &format!("file:{}", candidate), "IMPORTS").ok();
                    edges_created += 1;
                    break;
                }
            }
        }
    }

    // Pass 3: CALLS edges
    let all_fn_ids: HashMap<String, String> = all_results.values()
        .flat_map(|r| r.fn_ids.iter())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    for result in all_results.values() {
        for (_, callee_name, caller_id) in &result.call_edges {
            if let Some(callee_id) = all_fn_ids.get(callee_name) {
                if callee_id != caller_id {
                    graph_db.merge_edge(caller_id, callee_id, "CALLS").ok();
                    edges_created += 1;
                }
            }
        }
    }

    // Pass 4: Index documentation files
    let docs_indexed = super::doc_indexer::index_docs(graph_db, repo_path, repo_id).unwrap_or(0);

    // Pass 5: Framework/pattern tagging
    {
        // Collect file imports for framework detection
        let file_imports: HashMap<String, Vec<String>> = all_results.iter().map(|(abs, r)| {
            let fid = format!("file:{}", abs);
            let targets: Vec<String> = r.imports.iter().map(|i| i.target_path.clone()).collect();
            (fid, targets)
        }).collect();
        super::framework_tagger::tag_files_by_imports(graph_db, &file_imports);

        // Collect all functions for pattern tagging
        let fns: Vec<(String, String)> = all_results.values()
            .flat_map(|r| r.fn_ids.iter().map(|(name, id)| (id.clone(), name.clone())))
            .collect();
        super::framework_tagger::tag_functions_by_pattern(graph_db, &fns);
    }

    // Pass 6: Create Module nodes and containment edges
    // Uses ALL discovered files (not just re-parsed ones) so incremental runs still build full hierarchy
    {
        // Group ALL source files by directory → module
        let mut dir_files: HashMap<String, Vec<String>> = HashMap::new();
        for file_path in &files {
            let abs_path = file_path.to_string_lossy().to_string();
            if let Some(parent) = file_path.parent() {
                let dir = parent.to_string_lossy().to_string();
                dir_files.entry(dir).or_default().push(abs_path);
            }
        }

        // Pre-build map of file→[exported fn/type IDs] from existing graph edges
        // so we can create CONTAINS_FN for skipped (unchanged) files too
        let file_exports: HashMap<String, Vec<String>> = {
            let mut map: HashMap<String, Vec<String>> = HashMap::new();
            if let Ok(edges) = graph_db.get_edges(repo_id) {
                for e in &edges {
                    if e.edge_type == "EXPORTS_FN" || e.edge_type == "EXPORTS_TYPE" {
                        map.entry(e.source.clone()).or_default().push(e.target.clone());
                    }
                }
            }
            map
        };

        // Also create a virtual (root) package for files not under any workspace member
        let root_pkg_id = format!("pkg:{}:(root)", repo_id);
        let mut has_root_files = false;

        for (dir_path, file_paths) in &dir_files {
            let rel_dir = Path::new(dir_path).strip_prefix(repo)
                .unwrap_or(Path::new(dir_path))
                .to_string_lossy()
                .to_string();
            let mod_name = if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir.replace('\\', "/") };
            let mod_id = format!("mod:{}:{}", repo_id, mod_name);

            // Find which package this module belongs to
            let pkg_id = workspace_members.iter().find(|pkg| {
                rel_dir.starts_with(&pkg.path)
            }).map(|pkg| format!("pkg:{}:{}", repo_id, pkg.name));

            graph_db.merge_module(&mod_id, &mod_name, &rel_dir, pkg_id.as_deref().or(Some(&root_pkg_id)), repo_id).ok();
            modules_indexed += 1;

            // CONTAINS_MOD edge: package → module
            if let Some(ref pid) = pkg_id {
                graph_db.merge_edge(pid, &mod_id, "CONTAINS_MOD").ok();
            } else {
                has_root_files = true;
                graph_db.merge_edge(&root_pkg_id, &mod_id, "CONTAINS_MOD").ok();
            }
            edges_created += 1;

            // CONTAINS_FILE edges: module → file
            for abs_path in file_paths {
                let file_id = format!("file:{}", abs_path);
                graph_db.merge_edge(&mod_id, &file_id, "CONTAINS_FILE").ok();
                edges_created += 1;

                // CONTAINS_FN edges: module → functions
                // For re-parsed files, use the results; for skipped files, query existing graph nodes
                if let Some(result) = all_results.get(abs_path) {
                    for (_, fn_id) in &result.fn_ids {
                        graph_db.merge_edge(&mod_id, fn_id, "CONTAINS_FN").ok();
                        edges_created += 1;
                    }
                } else if let Some(exported) = file_exports.get(&file_id) {
                    // Skipped file — use pre-built exports map
                    for target_id in exported {
                        let edge_type = if target_id.starts_with("fn:") { "CONTAINS_FN" } else { "CONTAINS_TYPE" };
                        graph_db.merge_edge(&mod_id, target_id, edge_type).ok();
                        edges_created += 1;
                    }
                }
            }
        }

        // Create virtual (root) package if any files are outside workspace members
        if has_root_files {
            graph_db.merge_package(&root_pkg_id, "(root)", None, "", "root", repo_id).ok();
            let project_node = format!("project:{}", repo_id);
            graph_db.merge_edge(&project_node, &root_pkg_id, "CONTAINS_PKG").ok();
            packages_indexed += 1;
            edges_created += 1;
        }

        // Link doc nodes to their parent modules by directory path
        // Create modules for doc-only directories that weren't covered by the source walker
        if let Ok(all_nodes) = graph_db.get_nodes(repo_id) {
            for node in &all_nodes {
                if node.kind != "doc" { continue; }
                let doc_path = Path::new(&node.file);
                if let Some(parent) = doc_path.parent() {
                    let rel_dir = parent.strip_prefix(repo)
                        .unwrap_or(parent)
                        .to_string_lossy()
                        .replace('\\', "/");
                    let mod_name = if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir.clone() };
                    let mod_id = format!("mod:{}:{}", repo_id, mod_name);

                    // Ensure module exists (may be doc-only dir)
                    let pkg_id = workspace_members.iter().find(|pkg| {
                        rel_dir.starts_with(&pkg.path)
                    }).map(|pkg| format!("pkg:{}:{}", repo_id, pkg.name));
                    let parent_pkg = pkg_id.as_deref().unwrap_or(&root_pkg_id);
                    graph_db.merge_module(&mod_id, &mod_name, &rel_dir, Some(parent_pkg), repo_id).ok();
                    graph_db.merge_edge(parent_pkg, &mod_id, "CONTAINS_MOD").ok();
                    graph_db.merge_edge(&mod_id, &node.id, "CONTAINS_DOC").ok();
                    edges_created += 1;
                }
            }
        }
    }

    // Pass 7: HAS_METHOD edges (class → method) using parent tracking from adapters
    {
        // Build global type_id map: name → id (within same file first, then cross-file)
        let all_type_ids: HashMap<String, String> = all_results.values()
            .flat_map(|r| r.type_ids.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for result in all_results.values() {
            for (method_id, parent_name) in &result.method_parents {
                // Try same-file first, then global
                let type_id = result.type_ids.get(parent_name)
                    .or_else(|| all_type_ids.get(parent_name));
                if let Some(tid) = type_id {
                    graph_db.merge_edge(tid, method_id, "HAS_METHOD").ok();
                    edges_created += 1;
                }
            }
        }
    }

    // Prune stale files: delete graph nodes for files in manifest but no longer on disk
    {
        let current_files: std::collections::HashSet<String> = files.iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect();
        let stale: Vec<String> = manifest.tracked_paths()
            .into_iter()
            .filter(|p| !current_files.contains(*p))
            .map(|s| s.to_string())
            .collect();
        for stale_path in &stale {
            graph_db.delete_file(stale_path, repo_id).ok();
            manifest.remove(stale_path);
        }
        if !stale.is_empty() {
            tracing::info!("Pruned {} deleted files from graph", stale.len());
        }
    }

    // Save manifest
    manifest.save().map_err(|e| format!("Failed to save manifest: {}", e))?;

    // Detect external libraries from imports
    let mut lib_set = std::collections::HashSet::new();
    for result in all_results.values() {
        for imp in &result.imports {
            let path = &imp.target_path;
            // Skip relative, absolute, node builtins
            if path.starts_with('.') || path.starts_with('/') { continue; }
            if path.starts_with("node:") { continue; }
            // Skip SvelteKit/framework internal aliases
            if path.starts_with('$') { continue; } // $app, $env, $lib, $service-worker
            // Skip Node.js builtins
            if ["fs", "path", "os", "url", "http", "https", "module", "child_process",
                "crypto", "util", "events", "stream", "buffer", "net", "dns", "tls",
                "cluster", "worker_threads", "perf_hooks", "process", "assert",
                "readline", "querystring", "string_decoder", "zlib", "globals",
            ].contains(&path.as_str()) { continue; }
            // Skip Rust internal paths
            if path.starts_with("crate::") || path.starts_with("self::") || path.starts_with("super::") { continue; }
            if path.starts_with("std::") || path.starts_with("core::") || path.starts_with("alloc::") { continue; }
            // Skip Python import placeholders from the parser
            if path.starts_with("import_") || path.starts_with("from_") { continue; }
            // Skip Rust pub use re-exports
            if path.starts_with("pub use") || path.starts_with("pub(crate)") { continue; }
            // Skip Java stdlib
            if path.starts_with("java.") || path.starts_with("javax.") { continue; }
            // Group by org scope: @scope/pkg → "scope"
            let lib_name = if path.starts_with('@') {
                path.split('/').next().unwrap_or("").trim_start_matches('@').to_string()
            } else if path.contains("::") {
                // Rust crate: tokio::sync → "tokio"
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

    Ok(IndexResult {
        files_indexed,
        files_skipped,
        files_failed,
        functions_indexed,
        types_indexed,
        packages_indexed,
        modules_indexed,
        edges_created,
        docs_indexed,
        libs,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Incremental re-index: only process specific dirty files.
/// Deletes old symbols for each dirty file, re-parses, and re-inserts.
/// Returns list of changed function IDs (for doc drift detection).
pub fn index_dirty_files(
    graph_db: &GraphDb,
    repo_path: &str,
    repo_id: &str,
    dirty_files: &[std::path::PathBuf],
    progress: &dyn super::queue::ProgressCallback,
) -> Result<(IndexResult, Vec<String>), String> {
    let start = Instant::now();
    let repo = Path::new(repo_path);
    let files_total = dirty_files.len() as u32;
    let mut files_indexed = 0u32;
    let mut files_failed = 0u32;
    let mut functions_indexed = 0u32;
    let mut types_indexed = 0u32;
    let mut edges_created = 0u32;
    let mut changed_fn_ids = Vec::new();
    let started_at = chrono::Utc::now().to_rfc3339();

    for file_path in dirty_files {
        let abs_path = file_path.to_string_lossy().to_string();
        let rel_path = file_path.strip_prefix(repo)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        progress.on_file(crate::types::IndexProgress {
            repo_id: repo_id.to_string(),
            current_file: rel_path.clone(),
            files_processed: files_indexed + files_failed,
            files_total,
            files_unchanged: 0,
            files_skipped: 0,
            files_failed,
            started_at: started_at.clone(),
        });

        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();

        let adapter = match adapters::adapter_for_ext(&ext) {
            Some(a) => a,
            None => continue,
        };

        let source = match std::fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                files_failed += 1;
                tracing::warn!("Failed to read {}: {}", abs_path, e);
                continue;
            }
        };

        // Delete old symbols for this file before re-inserting
        graph_db.delete_file(&abs_path, repo_id)?;

        let parsed = adapter.parse(&source, &rel_path);
        let file_lines: Vec<&str> = source.lines().collect();
        let file_id = format!("file:{}", abs_path);
        let module_name = rel_path.rsplit_once('.').map(|(n, _)| n).unwrap_or(&rel_path)
            .replace('\\', "/");

        graph_db.merge_file(&file_id, &abs_path, &module_name, &parsed.language, repo_id).ok();

        for sym in &parsed.symbols {
            if is_function_like(&sym.kind) {
                let id = format!("fn:{}:{}:{}", abs_path, sym.name, sym.line_start);
                let body = file_lines
                    .get((sym.line_start as usize).saturating_sub(1)..sym.line_end as usize)
                    .map(|lines| lines.join("\n"))
                    .unwrap_or_default();
                let complexity = compute_complexity(&body);

                graph_db.merge_function(
                    &id, &sym.name, &abs_path, sym.line_start,
                    sym.signature.as_deref().unwrap_or(""),
                    &body, sym.docstring.as_deref().unwrap_or(""),
                    complexity, repo_id,
                ).ok();

                functions_indexed += 1;
                changed_fn_ids.push(id.clone());
                graph_db.merge_edge(&file_id, &id, "EXPORTS_FN").ok();
                edges_created += 1;
            } else {
                let id = format!("type:{}:{}:{}", abs_path, sym.name, sym.line_start);
                graph_db.merge_type(
                    &id, &sym.name, &abs_path, sym.line_start,
                    &sym.kind.to_string(), repo_id,
                ).ok();
                types_indexed += 1;
                graph_db.merge_edge(&file_id, &id, "EXPORTS_TYPE").ok();
                edges_created += 1;
            }
        }

        files_indexed += 1;
    }

    Ok((IndexResult {
        files_indexed,
        files_skipped: 0,
        files_failed,
        functions_indexed,
        types_indexed,
        packages_indexed: 0,
        modules_indexed: 0,
        edges_created,
        docs_indexed: 0,
        libs: vec![],
        duration_ms: start.elapsed().as_millis() as u64,
    }, changed_fn_ids))
}

fn build_globset(patterns: &[&str]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        if let Ok(g) = Glob::new(p) {
            builder.add(g);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSetBuilder::new().build().unwrap())
}

fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.') && name != "."
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, TempDir) {
        let repo = TempDir::new().unwrap();
        let db_dir = TempDir::new().unwrap();

        // Create Python files
        std::fs::write(repo.path().join("main.py"), r#"
def greet(name: str) -> str:
    """Say hello."""
    return f"hello {name}"

class App:
    def run(self):
        greet("world")

TIMEOUT = 30
"#).unwrap();

        std::fs::write(repo.path().join("utils.py"), r#"
def helper():
    pass

MAX_RETRIES = 3
"#).unwrap();

        (repo, db_dir)
    }

    #[test]
    fn index_small_repo() {
        let (repo, db_dir) = setup_test_repo();
        let graph = GraphDb::open(db_dir.path()).unwrap();
        let result = index_repo(&graph, &repo.path().to_string_lossy(), "test-repo").unwrap();

        assert!(result.files_indexed >= 2, "expected 2+ files, got {}", result.files_indexed);
        assert!(result.functions_indexed >= 3, "expected 3+ functions, got {}", result.functions_indexed);
        assert!(result.duration_ms < 5000);
    }

    #[test]
    fn incremental_skip() {
        let (repo, db_dir) = setup_test_repo();
        let graph = GraphDb::open(db_dir.path()).unwrap();
        let repo_id = format!("test-incr-{}", std::process::id());

        // First index
        let r1 = index_repo(&graph, &repo.path().to_string_lossy(), &repo_id).unwrap();
        assert!(r1.files_indexed >= 2);

        // Second index — should skip all (unchanged)
        let r2 = index_repo(&graph, &repo.path().to_string_lossy(), &repo_id).unwrap();
        assert_eq!(r2.files_indexed, 0, "expected 0 files on re-index, got {}", r2.files_indexed);
        assert!(r2.files_skipped >= 2);

        // Clean up manifest
        let manifest_dir = dirs::home_dir().unwrap().join(".sensei").join("projects").join(&repo_id);
        std::fs::remove_dir_all(&manifest_dir).ok();
    }

    #[test]
    fn nonexistent_repo_fails() {
        let db_dir = TempDir::new().unwrap();
        let graph = GraphDb::open(db_dir.path()).unwrap();
        let result = index_repo(&graph, "/nonexistent/path", "test");
        assert!(result.is_err());
    }

    #[test]
    fn has_method_edges_created() {
        let repo = TempDir::new().unwrap();
        let db_dir = TempDir::new().unwrap();
        // Python file with a class and methods
        std::fs::write(repo.path().join("service.py"), r#"
class UserService:
    def get_user(self, uid):
        return uid
    def delete_user(self, uid):
        pass
"#).unwrap();

        let graph = GraphDb::open(db_dir.path()).unwrap();
        let repo_id = format!("test-has-method-{}", std::process::id());
        let result = index_repo(&graph, &repo.path().to_string_lossy(), &repo_id).unwrap();
        assert!(result.functions_indexed >= 2, "expected 2+ functions");
        assert!(result.types_indexed >= 1, "expected 1+ types (UserService class)");

        // Check HAS_METHOD edges exist
        let edges = graph.get_edges(&repo_id).unwrap();
        let has_method_edges: Vec<_> = edges.iter().filter(|e| e.edge_type == "HAS_METHOD").collect();
        assert!(has_method_edges.len() >= 2, "expected 2+ HAS_METHOD edges, got {}", has_method_edges.len());

        // Clean up
        let manifest_dir = dirs::home_dir().unwrap().join(".sensei").join("projects").join(&repo_id);
        std::fs::remove_dir_all(&manifest_dir).ok();
    }

    #[test]
    fn module_nodes_created() {
        let repo = TempDir::new().unwrap();
        let db_dir = TempDir::new().unwrap();
        // Create files in subdirectories
        std::fs::create_dir_all(repo.path().join("src")).unwrap();
        std::fs::write(repo.path().join("src/main.py"), "def hello():\n    pass\n").unwrap();
        std::fs::write(repo.path().join("src/utils.py"), "def helper():\n    pass\n").unwrap();

        let graph = GraphDb::open(db_dir.path()).unwrap();
        let repo_id = format!("test-modules-{}", std::process::id());
        let result = index_repo(&graph, &repo.path().to_string_lossy(), &repo_id).unwrap();
        assert!(result.modules_indexed >= 1, "expected 1+ modules, got {}", result.modules_indexed);

        // Module nodes should appear in get_nodes
        let nodes = graph.get_nodes(&repo_id).unwrap();
        let module_nodes: Vec<_> = nodes.iter().filter(|n| n.kind == "module").collect();
        assert!(!module_nodes.is_empty(), "expected module nodes in graph");

        // CONTAINS_FN edges should exist
        let edges = graph.get_edges(&repo_id).unwrap();
        let contains_fn: Vec<_> = edges.iter().filter(|e| e.edge_type == "CONTAINS_FN").collect();
        assert!(!contains_fn.is_empty(), "expected CONTAINS_FN edges");

        // Clean up
        let manifest_dir = dirs::home_dir().unwrap().join(".sensei").join("projects").join(&repo_id);
        std::fs::remove_dir_all(&manifest_dir).ok();
    }

    #[test]
    fn package_nodes_for_workspace() {
        let repo = TempDir::new().unwrap();
        let db_dir = TempDir::new().unwrap();
        // Create a monorepo with package.json workspaces
        std::fs::write(repo.path().join("package.json"), r#"{"name":"monorepo","workspaces":["packages/*"]}"#).unwrap();
        std::fs::create_dir_all(repo.path().join("packages/ui")).unwrap();
        std::fs::write(repo.path().join("packages/ui/package.json"), r#"{"name":"@test/ui"}"#).unwrap();
        std::fs::create_dir_all(repo.path().join("packages/ui/src")).unwrap();
        std::fs::write(repo.path().join("packages/ui/src/index.ts"), "export function render() {}").unwrap();

        let graph = GraphDb::open(db_dir.path()).unwrap();
        let repo_id = format!("test-pkgs-{}", std::process::id());
        let result = index_repo(&graph, &repo.path().to_string_lossy(), &repo_id).unwrap();
        assert!(result.packages_indexed >= 1, "expected 1+ packages, got {}", result.packages_indexed);

        let nodes = graph.get_nodes(&repo_id).unwrap();
        let pkg_nodes: Vec<_> = nodes.iter().filter(|n| n.kind == "package").collect();
        assert!(!pkg_nodes.is_empty(), "expected package nodes in graph");

        // CONTAINS_MOD edges from packages should exist (package → module)
        let edges = graph.get_edges(&repo_id).unwrap();
        let contains_mod: Vec<_> = edges.iter().filter(|e| e.edge_type == "CONTAINS_MOD").collect();
        assert!(!contains_mod.is_empty(), "expected CONTAINS_MOD edges from packages");

        // Clean up
        let manifest_dir = dirs::home_dir().unwrap().join(".sensei").join("projects").join(&repo_id);
        std::fs::remove_dir_all(&manifest_dir).ok();
    }
}
