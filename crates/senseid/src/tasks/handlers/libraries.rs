//! Library phase: classify imports as internal vs external, fetch lib docs.

use super::super::Task;
use super::super::executor::TaskContext;
use crate::languages;

// ── Resolve Libs ──────────────────────────────────────────────────────────

/// Classify imports as internal vs external libraries. Update project.libs.
pub async fn resolve_libs(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    // folder_path is the repo abs_path (Task contract); look up the row
    // once and reuse for both repo_path and the later mark_folder_indexed
    // call. folder_name comes from the DB row so subtree composite names
    // like "sensei:homebrew" survive in log messages.
    let folder = ctx.pg().get_repo_by_path(task.folder_abs_path()).await
        .map_err(|e| tracing::warn!(error = %e, path = %task.folder_path, "resolve_libs: get_repo_by_path failed")).ok().flatten();
    let folder_name =
        folder.as_ref().and_then(|f| f["name"].as_str()).unwrap_or_else(|| task.folder_name());
    let repo_path_str = task.folder_path.clone();

    // Walk source files and extract external imports on a blocking thread:
    // the walk reads and synchronously parses every file (adapter.parse), which
    // would block the async worker's poll() — the same hazard process_file
    // avoids — and on a large folder could freeze the pool. spawn_blocking keeps
    // it off the runtime so the worker yields and the watchdog stays effective.
    let libs: Vec<String> = tokio::task::spawn_blocking(move || {
        let mut lib_set = std::collections::HashSet::new();
        if !repo_path_str.is_empty() {
            let repo_path = std::path::Path::new(&repo_path_str);
            let walker = super::helpers::build_walker(repo_path).build();

            for entry in walker.flatten() {
                if !entry.path().is_file() {
                    continue;
                }
                let ext = entry
                    .path()
                    .extension()
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
                let rel_path = entry
                    .path()
                    .strip_prefix(repo_path)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .to_string();
                let parsed = adapter.parse(&content, &rel_path);

                for imp in &parsed.imports {
                    let path = &imp.target_path;
                    // Skip relative, absolute, node builtins, framework aliases
                    if path.starts_with('.') || path.starts_with('/') || path.starts_with("node:") {
                        continue;
                    }
                    if path.starts_with('$') {
                        continue;
                    }
                    if [
                        "fs",
                        "path",
                        "os",
                        "url",
                        "http",
                        "https",
                        "module",
                        "child_process",
                        "crypto",
                        "util",
                        "events",
                        "stream",
                        "buffer",
                        "net",
                        "dns",
                        "tls",
                        "cluster",
                        "worker_threads",
                        "perf_hooks",
                        "process",
                        "assert",
                        "readline",
                        "querystring",
                        "string_decoder",
                        "zlib",
                    ]
                    .contains(&path.as_str())
                    {
                        continue;
                    }
                    if path.starts_with("crate::")
                        || path.starts_with("self::")
                        || path.starts_with("super::")
                    {
                        continue;
                    }
                    if path.starts_with("std::")
                        || path.starts_with("core::")
                        || path.starts_with("alloc::")
                    {
                        continue;
                    }
                    if path.starts_with("java.") || path.starts_with("javax.") {
                        continue;
                    }
                    if path.starts_with("import_") || path.starts_with("from_") {
                        continue;
                    }
                    if path.starts_with("pub use") || path.starts_with("pub(crate)") {
                        continue;
                    }

                    let lib_name = extract_lib_name(path);
                    if !lib_name.is_empty() && lib_name.len() > 1 {
                        lib_set.insert(lib_name);
                    }
                }
            }
        }
        let mut v: Vec<String> = lib_set.into_iter().collect();
        v.sort();
        v
    })
    .await
    .unwrap_or_else(|e| {
        tracing::warn!(error = %e, "resolve_libs: spawn_blocking walk task failed");
        Vec::new()
    });

    // Check which libs are internal (match another repo in a project)
    let all_repos = ctx.pg().list_repositories().await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "resolve_libs: list_repositories failed");
        Vec::new()
    });
    let _internal_repos: std::collections::HashSet<String> =
        all_repos.iter().filter_map(|p| p["name"].as_str().map(|s| s.to_lowercase())).collect();

    // D4.1: resolve_libs stamps the walked libs (folder metadata) but does NOT
    // advance `folder_status` — the terminal barrier moved to DetectCommunities,
    // the sole writer of `indexed`. build_connections re-stamps libs from its
    // import-derived set afterwards (last write wins, as before).
    if let Some(folder) = folder.as_ref()
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"])
        && let Err(e) =
            ctx.pg().set_folder_props(&folder_id, &serde_json::json!({"libs": libs})).await
    {
        tracing::warn!(error = %e, folder = %folder_name, "resolve_libs: set libs props failed");
    }

    // Enqueue ExtractDeps to parse manifest files and populate referenced_libraries
    let extract_task = super::super::Task::new(
        super::super::TaskKind::ExtractDeps,
        &task.folder_path,
        &task.folder_path,
    )
    .with_parent(task.id);
    ctx.queue.enqueue(extract_task).await;

    tracing::info!(
        "resolve_libs: {} — {} external libs detected, enqueued ExtractDeps",
        folder_name,
        libs.len()
    );
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
    let content = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to fetch {}: {}", url, e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    match ctx
        .pg()
        .upsert_library(lib_name, "npm", None, Some(&content), Some("url"), Some(url))
        .await
    {
        Ok(lib_id) => {
            tracing::info!("import_lib: {} — indexed as {} from {}", lib_name, lib_id, url);
            Ok(1)
        }
        Err(e) => Err(format!("Failed to index lib {}: {}", lib_name, e)),
    }
}

// ── Index Library ──────────────────────────────────────────────────────────

/// Resolve the library row an `index_library` task targets, then refresh its
/// source pointer.
///
/// The `IndexLibrary` enqueue passes the library UUID in `task.folder_path` (see
/// `add_library` in `mcp.rs`), so a re-index resolves the EXISTING row by that id
/// and keeps its real `ecosystem`. It must NOT re-upsert by `(name, "npm")`:
/// [`PgStore::upsert_library`]'s `ON CONFLICT(ecosystem, name)` would then INSERT
/// a phantom `(npm, name)` row for any cargo/pypi/go library — orphaning the real
/// row and its docs, stamping progress on the phantom, and re-enqueuing forever.
///
/// Fallback (`folder_path` is not a resolvable uuid) = first-index of a
/// genuinely-new library: upsert by name. (The `"npm"` default there is the
/// separate `add_library` D1 ticket, not this fix.) A uuid that no longer
/// resolves fails closed rather than fabricating a new row.
async fn resolve_index_library_id(
    pg: &crate::db::pg_store::PgStore,
    folder_path: &str,
    lib_name: &str,
    source_type: &str,
    base_url: Option<&str>,
) -> Result<uuid::Uuid, String> {
    if let Ok(id) = uuid::Uuid::parse_str(folder_path) {
        return match pg.get_library(&id).await? {
            Some(_) => {
                pg.update_library_source(&id, source_type, base_url).await?;
                Ok(id)
            }
            None => Err(format!(
                "index_library: library {id} not found (removed between enqueue and run)"
            )),
        };
    }
    pg.upsert_library(lib_name, "npm", None, None, Some(source_type), base_url)
        .await
        .map_err(|e| format!("upsert_library failed: {}", e))
}

/// Stamp the "docs applied at version" marker after a re-index (F v1a). The
/// target version is the scheduler's cached `latest` (single source of truth) —
/// so `should_reindex` reads false only once the docs actually caught up. No-op
/// when nothing was stored (`pages_stored == 0`, a failed/empty re-ingest) or no
/// latest is known (e.g. `add_library` first-index): never fabricates "applied".
pub(crate) async fn stamp_docs_applied_if_indexed(
    pg: &crate::db::pg_store::PgStore,
    lib_id: &uuid::Uuid,
    pages_stored: u32,
    now_unix: i64,
) {
    if pages_stored == 0 {
        return;
    }
    if let Some((latest, _)) = pg.get_library_latest_cache(lib_id).await.ok().flatten()
        && let Err(e) = pg.set_library_docs_applied(lib_id, &latest, now_unix).await
    {
        tracing::warn!(error = %e, %lib_id, "index_library: set_library_docs_applied failed");
    }
}

/// Fetch a library's llms docs, split into PER-COMPONENT pages, and store each.
///
/// `task.url` may be a local directory, a `github.com/.../tree/...` URL, or a
/// website llms URL — [`detect_lib_source`] classifies it and the matching
/// resolver fetches + derives pages (`component` set per page). Each page's
/// `component` is what makes `get_lib_docs(name, component)` resolve.
pub async fn index_library(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    use crate::indexer::lib_indexer::{LibSource, detect_lib_source, resolve_library_pages};

    let lib_name = &task.path;
    let url = task.url.as_deref().unwrap_or("");

    if url.is_empty() {
        return Err("index_library requires a URL (set via task.url)".into());
    }

    let source = detect_lib_source(url);

    // Resolve the library row this task targets + refresh its source pointer.
    let (base_source_type, base_url): (&str, Option<&str>) = match &source {
        LibSource::LocalDir(p) => ("local", Some(p.as_str())),
        LibSource::GitHubTree { .. } => ("http", Some(url)),
        LibSource::Website(u) => ("llms.txt", Some(u.as_str())),
    };
    let lib_id =
        resolve_index_library_id(ctx.pg(), &task.folder_path, lib_name, base_source_type, base_url)
            .await?;

    // Resolve the source into per-component pages (fetch + parse).
    let pages = resolve_library_pages(&source, lib_name)
        .await
        .map_err(|e| format!("resolve_library_pages failed for {}: {}", url, e))?;

    // Store each page. The page location goes to the right column: a filesystem
    // path (local sources) → local_path; an http(s) URL (website/GitHub) → url.
    // `source_type` records whether it was 'local', 'llms.txt', or 'http'.
    let mut pages_stored = 0u32;
    for page in &pages {
        let (url, local_path) = if page.source_type == "local" {
            (None, Some(page.location.as_str()))
        } else {
            (Some(page.location.as_str()), None)
        };
        match ctx
            .pg()
            .upsert_library_page(
                &lib_id,
                &page.doc.title,
                url,
                local_path,
                Some(&page.doc.summary),
                Some(&page.doc.content),
                page.source_type,
                page.doc.component.as_deref(),
            )
            .await
        {
            Ok(_) => pages_stored += 1,
            Err(e) => {
                tracing::warn!(error = %e, title = %page.doc.title, "index_library: store page failed")
            }
        }
    }

    if let Err(e) = ctx.pg().update_library_page_count(&lib_id).await {
        tracing::warn!(error = %e, lib = %lib_name, "index_library: update_library_page_count failed");
    }

    // F v1a: stamp the applied marker after a CONFIRMED, non-empty re-index so the
    // update scheduler's should_reindex gate stops re-enqueuing this library.
    stamp_docs_applied_if_indexed(ctx.pg(), &lib_id, pages_stored, chrono::Utc::now().timestamp())
        .await;

    // Workstream D: ingest the library's own sensei.library.json (local sources only
    // in v1 — a manifest read needs the files on disk) so its declared skills/agents
    // become associable capabilities. Non-fatal: a bad/absent manifest never fails
    // the doc index. Manifest-authoritative — removed entries disappear on re-index.
    if let LibSource::LocalDir(p) = &source
        && let Some((version, skills, agents)) =
            crate::libraries::load_manifest_from_root(std::path::Path::new(p))
    {
        match ctx
            .pg()
            .replace_library_capabilities(&lib_id, "manifest", Some(&version), &skills, &agents)
            .await
        {
            Ok((ns, na)) => tracing::info!(
                "index_library: {lib_name} sensei.library.json → {ns} skill(s), {na} agent(s)"
            ),
            Err(e) => {
                tracing::warn!(error = %e, lib = %lib_name, "index_library: replace_library_capabilities failed")
            }
        }
    }

    tracing::info!(
        "index_library: {} — {} pages stored ({} components) from {}",
        lib_name,
        pages_stored,
        pages.iter().filter(|p| p.doc.component.is_some()).count(),
        url,
    );
    Ok(pages_stored)
}

// ── Index Library Page ─────────────────────────────────────────────────────

/// Store a single library documentation page. Currently stores content only
/// (embedding generation deferred to gateway integration).
pub async fn index_library_page(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let lib_id_str = &task.folder_path; // library UUID stored in folder_path
    let title = &task.path; // page title stored in path

    let lib_id = uuid::Uuid::parse_str(lib_id_str)
        .map_err(|_| format!("Invalid library UUID: {}", lib_id_str))?;

    let url = task.url.as_deref();

    // Verify library exists
    ctx.pg()
        .get_library(&lib_id)
        .await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Library {} not found", lib_id))?;

    // Page content passed via module_id field (reusing available Task fields)
    let content = task.module_id.as_deref().unwrap_or("");
    if content.is_empty() {
        return Err("index_library_page requires content in module_id".into());
    }

    let summary: String = content.lines().take(3).collect::<Vec<_>>().join(" ");
    let summary = &summary[..summary.len().min(200)];

    ctx.pg()
        .upsert_library_page(&lib_id, title, url, None, Some(summary), Some(content), "http", None)
        .await
        .map_err(|e| format!("upsert_library_page failed: {}", e))?;

    Ok(1)
}

// ── Extract Deps ───────────────────────────────────────────────────────

/// Parse manifest files (package.json, Cargo.toml, pyproject.toml) and upsert
/// detected dependencies into libraries + referenced_libraries.
pub async fn extract_deps(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder = ctx
        .pg()
        .get_repo_by_path(task.folder_abs_path())
        .await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Folder '{}' not found", task.folder_path))?;

    let folder_name = folder["name"].as_str().unwrap_or_else(|| task.folder_name());
    let folder_id = crate::api::util::json_uuid(&folder["id"]).ok_or("Invalid folder id")?;
    // The folder's project (NULL for standalone folders) — used to roll each
    // detected dependency up to project_libraries so it shows on the Projects
    // screen (#30). project_libraries is the project↔library M2M the indexer
    // owns; referenced_libraries below is only folder-grained.
    let project_id = crate::api::util::json_uuid(&folder["project_id"]);

    let repo_path = folder["abs_path"].as_str().ok_or("Folder has no abs_path")?;

    let deps = crate::indexer::lib_indexer::extract_dep_versions(folder_name, repo_path)?;

    let mut count = 0u32;
    let mut local_edge_count = 0u32;
    for dep in &deps {
        // Route the manifest filename through the ManifestAdapter registry —
        // one source of truth for the manifest→ecosystem mapping.
        let ecosystem = crate::adapters::manifest::manifest_adapter_for_filename(&dep.source)
            .map(|a| a.ecosystem())
            .unwrap_or("npm");

        // Local-protocol deps (link: / workspace: / file: / path=) do NOT
        // resolve to external registry libraries; skip the library upsert and
        // try to record a project → project edge instead. If resolution
        // fails (target not indexed yet, workspace-name lookup, etc.), log
        // and move on — the next scan will retry.
        if let Some(target) = &dep.local_source {
            let protocol = local_source_protocol(&dep.source, &dep.raw_version);
            let resolved = resolve_local_target(repo_path, protocol, target);
            if let Some(pid) = project_id
                && let Some(abs_target) = resolved.as_ref().and_then(|p| p.to_str())
                && let Ok(Some(target_folder)) = ctx.pg().get_repo_by_path(abs_target).await
                && let Some(to_pid) = crate::api::util::json_uuid(&target_folder["project_id"])
                && to_pid != pid
                && let Err(e) = ctx
                    .pg()
                    .upsert_project_dependency(
                        &pid,
                        &to_pid,
                        &folder_id,
                        protocol,
                        &dep.source,
                        Some(target),
                    )
                    .await
            {
                tracing::warn!(
                    error = %e, lib = %dep.lib_name, folder = %folder_name,
                    "extract_deps: upsert_project_dependency failed"
                );
            }
            if resolved.is_some() {
                local_edge_count += 1;
            }
            continue;
        }

        // Upsert the library record (creates if not exists)
        let lib_id = match ctx
            .pg()
            .upsert_library(&dep.lib_name, ecosystem, Some(&dep.version), None, None, None)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!("extract_deps: skip {} — {}", dep.lib_name, e);
                continue;
            }
        };

        // Link folder → library via referenced_libraries
        if let Err(e) =
            ctx.pg().upsert_referenced_library(&folder_id, &lib_id, Some(&dep.version), None).await
        {
            tracing::warn!(error = %e, lib = %dep.lib_name, folder = %folder_name, "extract_deps: upsert_referenced_library failed");
            continue;
        }
        count += 1;

        // Roll the reference up to the project so it surfaces in
        // project_libraries_resolved (the Projects screen). Idempotent;
        // skips folders with no project (standalone).
        if let Some(pid) = project_id
            && let Err(e) = ctx.pg().upsert_project_library(&lib_id, &pid).await
        {
            tracing::warn!(error = %e, lib = %dep.lib_name, folder = %folder_name, "extract_deps: upsert_project_library failed");
        }
    }

    // First-party workspace packages are libraries this project PROVIDES — a
    // monorepo's own publishable packages (e.g. every @scope/* under
    // packages/*). Register the PUBLIC ones so they appear in the Libraries
    // view attributed to this folder, independent of whether any indexed repo
    // depends on them. Private (non-publishable) members are skipped (#63).
    let members =
        crate::config::detector::detect_workspace_members(std::path::Path::new(repo_path));
    let mut member_count = 0u32;
    for (name, ecosystem, version) in public_member_libs(&members) {
        let lib_id = match ctx
            .pg()
            .upsert_library(&name, ecosystem, version.as_deref(), None, None, None)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!(error = %e, lib = %name, "extract_deps: upsert_library (workspace member) failed");
                continue;
            }
        };
        if let Err(e) =
            ctx.pg().upsert_referenced_library(&folder_id, &lib_id, version.as_deref(), None).await
        {
            tracing::warn!(error = %e, lib = %name, folder = %folder_name, "extract_deps: upsert_referenced_library (workspace member) failed");
            continue;
        }
        member_count += 1;
        if let Some(pid) = project_id
            && let Err(e) = ctx.pg().upsert_project_library(&lib_id, &pid).await
        {
            tracing::warn!(error = %e, lib = %name, "extract_deps: upsert_project_library (workspace member) failed");
        }
    }

    tracing::info!(
        "extract_deps: {} — {} deps from manifests, {} first-party workspace packages, {} local-protocol edges",
        folder_name,
        count,
        member_count,
        local_edge_count,
    );

    // #83 T1 commands surface — one pass over the root's known manifests,
    // calling each ManifestAdapter's `parse_commands` and replacing the
    // folder's rows in `sensei.project_commands`. Idempotent (delete+insert
    // per ecosystem); cheap enough to run alongside the dep pass since it
    // re-reads only the manifest at the folder root, not the whole tree.
    let cmd_count = extract_and_persist_commands(ctx, &folder_id, repo_path).await;
    tracing::info!("extract_deps: {folder_name} — {cmd_count} discoverable command(s) persisted");

    Ok(count + member_count + local_edge_count + cmd_count)
}

/// For each known manifest at the folder root, ask its ManifestAdapter for
/// discoverable commands and replace the folder's rows. Errors are
/// logged, never propagated — a failing command scan mustn't fail the
/// wider dep pass. Returns the total rows written across ecosystems.
async fn extract_and_persist_commands(
    ctx: &TaskContext,
    folder_id: &uuid::Uuid,
    repo_path: &str,
) -> u32 {
    let repo = std::path::Path::new(repo_path);
    let mut written: u32 = 0;
    for adapter in crate::adapters::manifest::registered_adapters() {
        for filename in adapter.manifest_filenames() {
            let path = repo.join(filename);
            let Ok(content) = std::fs::read_to_string(&path) else { continue };
            let cmds = adapter.parse_commands(&content);
            if cmds.is_empty() {
                continue;
            }
            let rows: Vec<(String, String, Option<&str>)> = cmds
                .iter()
                .map(|c| (c.raw_name.clone(), c.command_line.clone(), c.category))
                .collect();
            match ctx
                .pg()
                .replace_folder_commands(folder_id, adapter.ecosystem(), path.to_str(), &rows)
                .await
            {
                Ok(n) => written += n as u32,
                Err(e) => tracing::warn!(
                    error = %e, folder = %folder_id, ecosystem = adapter.ecosystem(),
                    "extract_deps: replace_folder_commands failed"
                ),
            }
        }
    }
    written
}

/// Classify a `DepVersion` with a `local_source` into a protocol tag.
///
/// The protocol drives how the target is looked up:
/// - `path` → filesystem path relative to the declaring folder (Cargo).
/// - `link` / `file` → filesystem path relative to the declaring folder (npm).
/// - `workspace` → name lookup inside the same monorepo (intra-project by
///   design — no cross-project edge).
fn local_source_protocol(source: &str, raw_version: &str) -> &'static str {
    if source == "Cargo.toml" {
        return "path";
    }
    if raw_version.starts_with("link:") {
        return "link";
    }
    if raw_version.starts_with("workspace:") {
        return "workspace";
    }
    if raw_version.starts_with("file:") {
        return "file";
    }
    "path"
}

/// Resolve a manifest-relative local-source target to an absolute filesystem
/// path.
///
/// - `link:` / `path=` / `file:` payloads are filesystem paths; resolve
///   against the declaring folder's abs_path and normalize `..` / `.` segments.
/// - `workspace:` payloads name a member (not a path); return `None` so the
///   caller knows to fall back to the name-based lookup path.
///
/// The result is NOT `.canonicalize`d — canonicalization requires the target
/// to exist and follows symlinks in a way that surprises the caller when the
/// folder is symlinked. Lexical normalization is enough for looking up in
/// `sensei.folders` by `abs_path`, which itself stores the pre-canonical path.
fn resolve_local_target(
    from_abs_path: &str,
    protocol: &str,
    target: &str,
) -> Option<std::path::PathBuf> {
    if protocol == "workspace" {
        return None;
    }
    let base = std::path::Path::new(from_abs_path);
    let joined = base.join(target);
    Some(lexical_normalize(&joined))
}

/// Lexical `..` / `.` normalization — no filesystem access. Behaves like
/// `Path::canonicalize` for lexically absolute inputs but does not require the
/// target to exist.
fn lexical_normalize(p: &std::path::Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Extract the library name from an import path. Preserves scoped npm package
/// names (`@rokkit/core`) as a two-segment atom instead of dropping the scope.
///
/// - `@rokkit/core` → `@rokkit/core`
/// - `@rokkit/actions/sub/module` → `@rokkit/actions` (deeper subpaths dropped)
/// - `@rokkit` (bare scope) → `@rokkit` (rare; kept as-is)
/// - `svelte/store` → `svelte`
/// - `serde::de::Deserializer` → `serde`
/// - single-segment `react` → `react`
///
/// Returns an empty string if `path` is empty. Callers apply the `len > 1`
/// filter to reject noise.
fn extract_lib_name(path: &str) -> String {
    if path.starts_with('@') {
        // Scoped npm package: keep the "@scope/name" pair, drop deeper paths.
        let mut parts = path.splitn(3, '/');
        return match (parts.next(), parts.next()) {
            (Some(scope), Some(name)) => format!("{scope}/{name}"),
            (Some(only), None) => only.to_string(),
            _ => String::new(),
        };
    }
    if path.contains("::") {
        return path.split("::").next().unwrap_or("").to_string();
    }
    path.split('/').next().unwrap_or("").to_string()
}

/// Public (non-private) workspace members mapped to `(name, ecosystem, version)`
/// for registration as first-party libraries. Private members are excluded so
/// only publishable packages reach the global Libraries view (#63).
fn public_member_libs(
    members: &[crate::types::PackageInfo],
) -> Vec<(String, &'static str, Option<String>)> {
    members
        .iter()
        .filter(|m| !m.private)
        .map(|m| {
            let ecosystem = match m.pkg_type.as_str() {
                "cargo_crate" => "cargo",
                "go_module" => "go",
                // Gradle + Maven both resolve against the same Maven Central
                // namespace, so their members roll up as `"maven"` libs.
                "maven_module" | "gradle_module" => "maven",
                "dotnet_project" => "nuget",
                _ => "npm", // npm_workspace + any future JS variant
            };
            (m.name.clone(), ecosystem, m.version.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PackageInfo;

    fn member(name: &str, pkg_type: &str, private: bool) -> PackageInfo {
        PackageInfo {
            name: name.into(),
            path: name.into(),
            version: Some("1.0.0".into()),
            pkg_type: pkg_type.into(),
            private,
        }
    }

    // ── extract_lib_name ──────────────────────────────────────────────
    //
    // #30 residual: the scoped-npm branch used to drop everything after the
    // first `/` and strip `@`, so `@rokkit/core` became `rokkit`. Now we keep
    // the two-segment scope+name atom.

    #[test]
    fn extract_lib_name_keeps_scoped_npm_package() {
        assert_eq!(extract_lib_name("@rokkit/core"), "@rokkit/core");
        assert_eq!(extract_lib_name("@sveltejs/kit"), "@sveltejs/kit");
    }

    #[test]
    fn extract_lib_name_drops_deeper_subpaths_under_scope() {
        // Deep import: `import { foo } from "@rokkit/actions/sub/module"`
        // → the library is `@rokkit/actions`; the deeper path is where inside
        //   the package the symbol lives.
        assert_eq!(extract_lib_name("@rokkit/actions/sub/module"), "@rokkit/actions");
    }

    #[test]
    fn extract_lib_name_handles_bare_scope() {
        // Defensive: a lone `@scope` shouldn't crash — keep as-is.
        // Caller's len > 1 filter will still let this through, but it's fine.
        assert_eq!(extract_lib_name("@rokkit"), "@rokkit");
    }

    #[test]
    fn extract_lib_name_strips_subpath_from_unscoped_npm() {
        assert_eq!(extract_lib_name("svelte/store"), "svelte");
        assert_eq!(extract_lib_name("react"), "react");
    }

    #[test]
    fn extract_lib_name_strips_rust_module_path() {
        assert_eq!(extract_lib_name("serde::de::Deserializer"), "serde");
        assert_eq!(extract_lib_name("tokio::spawn"), "tokio");
    }

    #[test]
    fn extract_lib_name_empty_input_returns_empty() {
        assert_eq!(extract_lib_name(""), "");
    }

    // ── local_source_protocol + resolve_local_target (1a Step 5) ──────

    #[test]
    fn local_source_protocol_maps_cargo_and_npm_prefixes() {
        assert_eq!(local_source_protocol("Cargo.toml", "1.2.3"), "path");
        assert_eq!(local_source_protocol("package.json", "link:../actions"), "link");
        assert_eq!(local_source_protocol("package.json", "workspace:*"), "workspace");
        assert_eq!(local_source_protocol("package.json", "file:./x.tgz"), "file");
    }

    #[test]
    fn resolve_local_target_joins_relative_and_normalizes() {
        let resolved =
            resolve_local_target("/Users/j/Developer/rokkit/packages/ui", "link", "../actions");
        assert_eq!(
            resolved.as_deref(),
            Some(std::path::Path::new("/Users/j/Developer/rokkit/packages/actions"))
        );
    }

    #[test]
    fn resolve_local_target_handles_double_parent() {
        let resolved = resolve_local_target("/root/proj/a/b", "path", "../../other/c");
        assert_eq!(resolved.as_deref(), Some(std::path::Path::new("/root/proj/other/c")));
    }

    #[test]
    fn resolve_local_target_workspace_returns_none() {
        // workspace: is a name-based lookup, not a path — caller must
        // fall back to member-name resolution when it wants intra-project
        // routing. For 1a this branch is intentionally not wired.
        assert_eq!(resolve_local_target("/x/y", "workspace", "*"), None);
    }

    // ── public_member_libs (existing) ─────────────────────────────────

    #[test]
    fn public_member_libs_skips_private_and_maps_valid_ecosystems() {
        let members = vec![
            member("@m/ui", "npm_workspace", false),
            member("@m/secret", "npm_workspace", true), // private → excluded
            member("my-crate", "cargo_crate", false),
            member("example.com/mod", "go_module", false),
            member("com.example:core", "maven_module", false),
            member("MyLib", "dotnet_project", false),
        ];
        let libs = public_member_libs(&members);
        let names: Vec<&str> = libs.iter().map(|(n, _, _)| n.as_str()).collect();
        assert!(names.contains(&"@m/ui"));
        assert!(!names.contains(&"@m/secret"), "private members are excluded");
        let eco = |n: &str| libs.iter().find(|(name, _, _)| name == n).map(|(_, e, _)| *e);
        // Must be valid `library_ecosystem` enum labels (npm/cargo/go/maven)
        // — NOT "crates", which silently failed the cast and dropped every
        // cargo lib (#63).
        assert_eq!(eco("@m/ui"), Some("npm"));
        assert_eq!(eco("my-crate"), Some("cargo"));
        assert_eq!(eco("example.com/mod"), Some("go"));
        assert_eq!(eco("com.example:core"), Some("maven"));
        assert_eq!(eco("MyLib"), Some("nuget"));
    }

    // ── resolve_index_library_id (F v1 prereq: resolve BY lib_id) ─────
    //
    // The IndexLibrary enqueue passes the library UUID in `task.folder_path`
    // (mcp.rs `add_library`). A re-index must resolve THAT row and keep its real
    // ecosystem — never re-upsert by `(name, "npm")`, which
    // `ON CONFLICT(ecosystem, name)` turns into a phantom `(npm, name)` row for a
    // cargo/pypi/go library, orphaning the real row + its docs.

    use crate::db::pg_store::PgStore;

    async fn npm_row_exists(pg: &PgStore, name: &str) -> bool {
        let n: (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.libraries \
               WHERE ecosystem = 'npm'::sensei.library_ecosystem AND name = $1",
        )
        .bind(name)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        n.0 > 0
    }

    #[tokio::test]
    async fn resolve_by_lib_id_preserves_ecosystem_and_creates_no_phantom() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let name = format!("_fidxlib_{}", uuid::Uuid::new_v4());
        // A cargo library, exactly as the dependency-extraction path creates it.
        let real_id = pg
            .upsert_library(&name, "cargo", Some("1.0.0"), None, Some("local"), Some("/x"))
            .await
            .unwrap();

        let resolved =
            resolve_index_library_id(&pg, &real_id.to_string(), &name, "local", Some("/x/docs"))
                .await
                .unwrap();

        assert_eq!(resolved, real_id, "resolves the EXISTING row by its lib_id");
        let lib = pg.get_library(&real_id).await.unwrap().expect("cargo row still exists");
        assert_eq!(lib["ecosystem"].as_str(), Some("cargo"), "ecosystem is NOT clobbered to npm");
        assert!(
            !npm_row_exists(&pg, &name).await,
            "no phantom (npm, name) row is created for a cargo library"
        );
    }

    #[tokio::test]
    async fn resolve_non_uuid_folder_path_first_indexes_by_name() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let name = format!("_fidxnew_{}", uuid::Uuid::new_v4());
        // Not a uuid → genuinely-new library first-index (upsert by name).
        let id = resolve_index_library_id(
            &pg,
            "/some/repo/path",
            &name,
            "llms.txt",
            Some("https://x/llms.txt"),
        )
        .await
        .unwrap();
        let lib = pg.get_library(&id).await.unwrap().expect("first-index created the row");
        assert_eq!(lib["name"].as_str(), Some(name.as_str()));
    }

    #[tokio::test]
    async fn stamp_docs_applied_only_on_nonempty_index() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let lib = format!("_fstamp_{}", uuid::Uuid::new_v4());
        let lid = pg.upsert_library(&lib, "cargo", Some("1.0.0"), None, None, None).await.unwrap();
        pg.set_library_latest_cache(&lid, "1.2.4", 10).await.unwrap();
        // pages_stored == 0 → NO stamp (a failed/empty re-ingest must not claim applied).
        stamp_docs_applied_if_indexed(&pg, &lid, 0, 99).await;
        assert_eq!(
            pg.get_library_docs_applied(&lid).await.unwrap(),
            None,
            "empty index does not stamp"
        );
        // pages_stored > 0 → stamp the scheduler's cached latest.
        stamp_docs_applied_if_indexed(&pg, &lid, 3, 99).await;
        assert_eq!(
            pg.get_library_docs_applied(&lid).await.unwrap().as_deref(),
            Some("1.2.4"),
            "a confirmed non-empty re-index stamps the cached latest"
        );
    }

    #[tokio::test]
    async fn resolve_unknown_uuid_fails_closed() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let ghost = uuid::Uuid::new_v4();
        let name = format!("_fidxghost_{}", uuid::Uuid::new_v4());
        // A uuid that no longer resolves → error, never a fabricated fallback row.
        let r = resolve_index_library_id(&pg, &ghost.to_string(), &name, "local", Some("/x")).await;
        assert!(r.is_err(), "an unresolvable lib_id fails closed");
        assert!(!npm_row_exists(&pg, &name).await, "and creates no fallback (npm, name) row");
    }
}
