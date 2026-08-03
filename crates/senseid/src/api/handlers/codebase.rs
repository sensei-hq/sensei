use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

/// Resolve a repo/project name to its folder UUID, failing closed on a store error.
///
/// `Ok(Some(id))` = the name resolves to an indexed folder, `Ok(None)` = the name
/// genuinely isn't indexed, `Err(500)` = the store read itself failed. A lookup
/// failure is never masked as a miss (see the #109 fail-closed audit and the
/// CLAUDE.md "never fabricate on a failure path" rule). This is name-exact via
/// `get_repo_by_name`; for project-scope (all folders) resolution use
/// `query::resolve_folder_id`/`resolve_scope_ids`.
async fn repo_folder_id(state: &AppState, name: &str) -> Result<Option<uuid::Uuid>, StatusCode> {
    let folder = state.pg.get_repo_by_name(name).await.map_err(|e| {
        tracing::warn!(error = %e, name = %name, "repo_folder_id: get_repo_by_name failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(folder.as_ref().and_then(|f| crate::api::util::json_uuid(&f["id"])))
}

// ── Graph Queries ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct GraphQuery {
    #[serde(rename = "repoId")]
    pub repo_id: Option<String>,
}

pub(crate) async fn graph_nodes(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    if repo_id.is_empty() {
        return Ok(Json(serde_json::json!({"nodes": [], "edges": []})));
    }
    let ids = state.pg.scope_folder_ids(&repo_id).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "graph_nodes: scope_folder_ids failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    if ids.is_empty() {
        return Ok(Json(serde_json::json!({"nodes": [], "edges": []})));
    }
    let nodes = state.pg.get_nodes_scoped(&ids).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "graph_nodes: get_nodes_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    let edges = state.pg.get_edges_scoped(&ids, "calls").await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "graph_nodes: get_edges_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(serde_json::json!({"nodes": nodes, "edges": edges})))
}

#[derive(Deserialize)]
pub(crate) struct SymbolQuery {
    #[serde(rename = "repoId")]
    pub repo_id: String,
    #[serde(rename = "q")]
    pub query: String,
}

pub(crate) async fn search_functions(
    State(state): State<AppState>,
    Query(q): Query<SymbolQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let ids = state.pg.scope_folder_ids(&q.repo_id).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %q.repo_id, "search_functions: scope_folder_ids failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    if ids.is_empty() {
        return Ok(Json(vec![]));
    }
    let results = state.pg.search_functions_scoped(&ids, &q.query).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %q.repo_id, "search_functions: search_functions_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(results))
}

pub(crate) async fn search_types(
    State(state): State<AppState>,
    Query(q): Query<SymbolQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let ids = state.pg.scope_folder_ids(&q.repo_id).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %q.repo_id, "search_types: scope_folder_ids failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    if ids.is_empty() {
        return Ok(Json(vec![]));
    }
    let results = state.pg.search_types_scoped(&ids, &q.query).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %q.repo_id, "search_types: search_types_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(results))
}

#[derive(Deserialize)]
pub(crate) struct TraceQuery {
    #[serde(rename = "repoId")]
    pub repo_id: String,
    pub name: String,
}

pub(crate) async fn fn_callers(
    State(state): State<AppState>,
    Query(q): Query<TraceQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let results = state.pg.get_callers_by_name(&q.repo_id, &q.name).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %q.repo_id, name = %q.name, "fn_callers: get_callers_by_name failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(results))
}

pub(crate) async fn fn_callees(
    State(state): State<AppState>,
    Query(q): Query<TraceQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let results = state.pg.get_callees_by_name(&q.repo_id, &q.name).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %q.repo_id, name = %q.name, "fn_callees: get_callees_by_name failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(results))
}

#[derive(Deserialize)]
pub(crate) struct TagQuery {
    #[serde(rename = "repoId")]
    pub repo_id: String,
    pub tag: String,
}

pub(crate) async fn files_by_tag(
    State(state): State<AppState>,
    Query(q): Query<TagQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let results = state.pg.get_files_by_tag(&q.repo_id, &q.tag).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %q.repo_id, tag = %q.tag, "files_by_tag: get_files_by_tag failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(serde_json::json!(results)))
}

pub(crate) async fn doc_drift(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    let results = state.pg.get_doc_drift(&repo_id).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "doc_drift: get_doc_drift failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(serde_json::json!(results)))
}

pub(crate) async fn call_flow(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    let ids = state.pg.scope_folder_ids(&repo_id).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "call_flow: scope_folder_ids failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    if ids.is_empty() {
        return Ok(Json(serde_json::json!({"modules": [], "calls": [], "moduleCount": 0, "exportCount": 0, "callCount": 0})));
    }
    let edges = state.pg.get_edges_scoped(&ids, "calls").await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "call_flow: get_edges_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    let nodes = state.pg.get_nodes_scoped(&ids).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "call_flow: get_nodes_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    let modules: Vec<serde_json::Value> = nodes.iter()
        .filter(|n| n["kind"].as_str() == Some("file"))
        .map(|n| serde_json::json!({
            "path": n["file_path"],
            "exports": nodes.iter()
                .filter(|c| c["file_path"] == n["file_path"] && c["is_exported"].as_bool() == Some(true))
                .filter_map(|c| c["name"].as_str())
                .collect::<Vec<_>>(),
        }))
        .collect();
    Ok(Json(serde_json::json!({
        "modules": modules,
        "calls": edges,
        "moduleCount": modules.len(),
        "exportCount": modules.iter().map(|m| m["exports"].as_array().map_or(0, |a| a.len())).sum::<usize>(),
        "callCount": edges.len(),
    })))
}

pub(crate) async fn detect_communities(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = body["repoId"].as_str().unwrap_or_default().to_string();
    if repo_id.is_empty() {
        return Ok(Json(serde_json::json!({"error": "repoId required"})));
    }
    if let Some(folder_id) = repo_folder_id(&state, &repo_id).await? {
        let communities = state.pg.list_communities(&folder_id).await
            .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "detect_communities: list_communities failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
        let num = communities.len();
        return Ok(Json(serde_json::json!({
            "ok": true,
            "communities": num,
            "assignments": num,
        })));
    }
    Ok(Json(serde_json::json!({"ok": false, "error": "project not found"})))
}

pub(crate) async fn community_info(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    if let Some(folder_id) = repo_folder_id(&state, &repo_id).await? {
        let communities = state.pg.list_communities(&folder_id).await
            .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "community_info: list_communities failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
        return Ok(Json(serde_json::json!(communities)));
    }
    Ok(Json(serde_json::json!([])))
}

// ── Patterns ────────────────────────────────────────────────────────────────

pub(crate) async fn detect_patterns(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Look up folder UUID and query patterns from PgStore
    if let Some(folder_id) = repo_folder_id(&state, &project).await? {
        let patterns = state.pg.list_patterns_by_folder(&folder_id).await
            .map_err(|e| { tracing::warn!(error = %e, project = %project, "detect_patterns: list_patterns_by_folder failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
        return Ok(Json(serde_json::json!({"ok": true, "patterns": patterns, "count": patterns.len()})));
    }
    Ok(Json(serde_json::json!({"ok": false, "error": "project not found"})))
}

pub(crate) async fn list_patterns(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Some(folder_id) = repo_folder_id(&state, &project).await? {
        let patterns = state.pg.list_patterns_by_folder(&folder_id).await
            .map_err(|e| { tracing::warn!(error = %e, project = %project, "list_patterns: list_patterns_by_folder failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
        return Ok(Json(serde_json::json!({"patterns": patterns, "count": patterns.len()})));
    }
    Ok(Json(serde_json::json!({"patterns": [], "count": 0})))
}

#[derive(Deserialize)]
pub(crate) struct MatchQuery {
    pub description: Option<String>,
}

pub(crate) async fn match_pattern_handler(
    State(state): State<AppState>,
    Path(project): Path<String>,
    Query(q): Query<MatchQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let desc = q.description.unwrap_or_default();
    // Search patterns by folder using BM25 ranking
    if let Some(folder_id) = repo_folder_id(&state, &project).await? {
        let ranked = state.pg.rank_bm25(&folder_id, &desc).await
            .map_err(|e| { tracing::warn!(error = %e, project = %project, "match_pattern_handler: rank_bm25 failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
        let matches: Vec<serde_json::Value> = ranked.into_iter()
            .map(|(name, score)| serde_json::json!({"name": name, "score": score}))
            .collect();
        return Ok(Json(serde_json::json!({"matches": matches, "count": matches.len()})));
    }
    Ok(Json(serde_json::json!({"matches": [], "count": 0})))
}

pub(crate) async fn pattern_for_symbol(
    State(state): State<AppState>,
    Path((project, symbol)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // File-level membership: the pattern(s) whose file instances include the
    // symbol's file. Scoped to the project (patterns are project-attributed) with
    // node lookup across all the project's folders. Genuinely empty when the
    // symbol's file is in no detected pattern — NOT the old always-null mask.
    let scope = state.pg.scope_folder_ids(&project).await
        .map_err(|e| { tracing::warn!(error = %e, project = %project, "pattern_for_symbol: scope_folder_ids failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    let project_id = crate::api::util::resolve_project_uuid(&state, &project).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let patterns = match project_id {
        Some(pid) if !scope.is_empty() => state.pg.patterns_for_symbol(&pid, &scope, &symbol).await
            .map_err(|e| { tracing::warn!(error = %e, project = %project, "pattern_for_symbol: patterns_for_symbol failed"); StatusCode::INTERNAL_SERVER_ERROR })?,
        _ => Vec::new(),
    };
    Ok(Json(serde_json::json!({ "symbol": symbol, "count": patterns.len(), "patterns": patterns })))
}

#[derive(Deserialize)]
pub(crate) struct DuplicatesQuery {
    pub min_similarity: Option<f64>,
    pub limit: Option<i64>,
}

/// GET /api/patterns/{project}/duplicates — near-duplicate functions found by
/// cosine similarity on code embeddings. `min_similarity` (default 0.92) and
/// `limit` (default 50) are query params.
///
/// #54 — Scope is the WHOLE project, not just one folder: when the caller
/// passes a project name we resolve every folder that belongs to it and
/// run the cosine-similarity self-join across all of them, so a
/// duplicate that spans two crates in the same project surfaces. If the
/// caller passes a bare repo name that resolves to a folder without a
/// project, we fall back to the single-folder variant.
pub(crate) async fn find_duplicates_handler(
    State(state): State<AppState>,
    Path(project): Path<String>,
    Query(q): Query<DuplicatesQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let min_similarity = q.min_similarity.unwrap_or(0.92).clamp(0.0, 1.0);
    let limit = q.limit.unwrap_or(50).clamp(1, 500);

    // Project scope first — every folder that belongs to the named project.
    let scope = state.pg.scope_folder_ids(&project).await
        .map_err(|e| { tracing::warn!(error = %e, project = %project, "find_duplicates_handler: scope_folder_ids failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    if !scope.is_empty() {
        let dups = state.pg.find_duplicates_scoped(&scope, min_similarity, limit).await
            .map_err(|e| { tracing::warn!(error = %e, project = %project, "find_duplicates_handler: find_duplicates_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
        return Ok(Json(serde_json::json!({
            "count": dups.len(),
            "min_similarity": min_similarity,
            "scope": "project",
            "folder_count": scope.len(),
            "duplicates": dups,
        })));
    }

    // Fall back to a single-folder lookup when the caller passed a repo
    // name (or the project resolution returned nothing).
    let Some(folder_id) = repo_folder_id(&state, &project).await? else {
        return Ok(Json(serde_json::json!({ "duplicates": [], "count": 0, "message": "project not indexed" })));
    };
    let dups = state.pg.find_duplicates(&folder_id, min_similarity, limit).await
        .map_err(|e| { tracing::warn!(error = %e, project = %project, "find_duplicates_handler: find_duplicates failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(serde_json::json!({
        "count": dups.len(),
        "min_similarity": min_similarity,
        "scope": "folder",
        "duplicates": dups,
    })))
}

// ── Conventions ───────────────────────────────────────────────────────────────

/// Code-symbol node kinds whose names carry naming conventions worth reporting.
const NAMING_KINDS: &[&str] = &[
    "function", "method", "class", "interface",
    "type", "const", "enum", "enum_variant", "field", "property",
];

/// Cap on the number of `directories` / `patterns` rows returned by the
/// conventions summary. The directory list is otherwise one row per source
/// directory (unbounded — ~1k for sensei), which pushed the response past the
/// MCP client's token cap (~60K chars). We keep the top-N by weight (files /
/// instance_count) and expose the true total alongside so the trim is visible,
/// not silent.
const MAX_CONVENTION_ROWS: usize = 25;

/// Classify an identifier into a canonical naming style. Leading/trailing
/// underscores are ignored (e.g. `_unused`, `__init__`) so they don't mask the
/// underlying style. Returns one of: `snake_case`, `SCREAMING_SNAKE_CASE`,
/// `camelCase`, `PascalCase`, `kebab-case`, `other`.
fn classify_case(name: &str) -> &'static str {
    let core = name.trim_matches('_');
    if core.is_empty() {
        return "other";
    }
    let has_underscore = core.contains('_');
    let has_hyphen = core.contains('-');
    let has_upper = core.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = core.chars().any(|c| c.is_ascii_lowercase());
    let first_alpha = core.chars().find(|c| c.is_ascii_alphabetic());

    // Single uppercase letter (a generic type param like T or E) reads as Pascal.
    if core.len() == 1 && has_upper {
        return "PascalCase";
    }
    if has_hyphen && !has_underscore && !has_upper {
        return "kebab-case";
    }
    if has_underscore && !has_hyphen {
        if has_upper && !has_lower {
            return "SCREAMING_SNAKE_CASE";
        }
        if !has_upper {
            return "snake_case";
        }
        return "other"; // mixed, e.g. My_Var
    }
    if !has_underscore && !has_hyphen {
        if has_upper && has_lower {
            return match first_alpha {
                Some(c) if c.is_ascii_uppercase() => "PascalCase",
                Some(_) => "camelCase",
                None => "other",
            };
        }
        if has_upper {
            return "SCREAMING_SNAKE_CASE"; // all caps, no separators (FOO, MAX)
        }
        if has_lower {
            return "snake_case"; // single lowercase word (main, parse)
        }
    }
    "other"
}

/// Map a file extension to a language label for the structure summary.
///
/// For extensions with a `LanguageAdapter`, we consult the adapter directly so
/// this handler doesn't duplicate the language slug knowledge. For extensions
/// with no adapter yet (go, ruby, shell, config-file types) we fall through
/// to the small hardcoded table below.
fn language_for_ext(ext: &str) -> &str {
    let dotted = format!(".{ext}");
    if let Some(adapter) = crate::languages::adapter_for_ext(&dotted) {
        return adapter_language_static(adapter.language());
    }
    match ext {
        "go" => "go",
        "rb" => "ruby",
        "sh" | "bash" => "shell",
        "md" | "markdown" => "markdown",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "css" => "css",
        "html" => "html",
        other => other,
    }
}

/// Return-value adapter: `adapter.language()` yields a `&str` bound to a
/// short-lived `Box<dyn LanguageAdapter>`, but every impl returns one of a
/// closed set of static slugs. Map through this table so the caller gets a
/// `&'static str` and doesn't have to clone on the hot path.
fn adapter_language_static(slug: &str) -> &'static str {
    match slug {
        "rust" => "rust",
        "typescript" => "typescript",
        "javascript" => "javascript",
        "python" => "python",
        "java" => "java",
        "kotlin" => "kotlin",
        "swift" => "swift",
        "svelte" => "svelte",
        "vue" => "vue",
        "sql" => "sql",
        "c" => "c",
        // Fallback for any future adapter — treat as unknown so this handler
        // never surfaces stale labels; add a case above when a new adapter
        // lands.
        _ => "other",
    }
}

/// Aggregate naming conventions (dominant case style per node kind), directory
/// and language structure, and detected design patterns from the code graph.
/// Pure over its inputs so it can be unit-tested without a database.
fn derive_conventions(
    nodes: &[serde_json::Value],
    patterns: &[serde_json::Value],
) -> serde_json::Value {
    use std::collections::BTreeMap;

    let mut by_kind: BTreeMap<&'static str, BTreeMap<&'static str, u32>> = BTreeMap::new();
    let mut dir_files: BTreeMap<String, u32> = BTreeMap::new();
    let mut lang_files: BTreeMap<String, u32> = BTreeMap::new();
    let mut file_count: u32 = 0;

    for n in nodes {
        let kind = n["kind"].as_str().unwrap_or("");
        let name = n["name"].as_str().unwrap_or("");

        if let Some(&kind_s) = NAMING_KINDS.iter().find(|k| **k == kind)
            && !name.is_empty()
        {
            let style = classify_case(name);
            *by_kind.entry(kind_s).or_default().entry(style).or_insert(0) += 1;
        }

        if kind == "file" {
            file_count += 1;
            if let Some(fp) = n["file_path"].as_str() {
                let dir = fp.rsplit_once('/').map(|(d, _)| d).unwrap_or(".").to_string();
                *dir_files.entry(dir).or_insert(0) += 1;
                if let Some((_, ext)) = fp.rsplit_once('.') {
                    *lang_files.entry(language_for_ext(ext).to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    let naming: Vec<serde_json::Value> = by_kind
        .iter()
        .map(|(kind, styles)| {
            let total: u32 = styles.values().sum();
            let (dominant, dom_count) = styles
                .iter()
                .max_by_key(|(_, c)| **c)
                .map(|(s, c)| (*s, *c))
                .unwrap_or(("other", 0));
            let conformance = if total > 0 {
                ((dom_count as f64 / total as f64) * 100.0).round() / 100.0
            } else {
                0.0
            };
            let styles_obj: serde_json::Map<String, serde_json::Value> =
                styles.iter().map(|(s, c)| ((*s).to_string(), serde_json::json!(c))).collect();
            serde_json::json!({
                "kind": kind,
                "dominant": dominant,
                "conformance": conformance,
                "count": total,
                "styles": styles_obj,
            })
        })
        .collect();

    let mut dirs: Vec<(String, u32)> = dir_files.into_iter().collect();
    dirs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let directories_total = dirs.len();
    let directories: Vec<serde_json::Value> = dirs
        .into_iter()
        .take(MAX_CONVENTION_ROWS)
        .map(|(path, files)| serde_json::json!({"path": path, "files": files}))
        .collect();

    let languages: serde_json::Map<String, serde_json::Value> =
        lang_files.into_iter().map(|(k, v)| (k, serde_json::json!(v))).collect();

    // Top-N patterns by instance_count so the summary stays bounded; the true
    // count rides along in `patterns_total` so the trim is never silent.
    let patterns_total = patterns.len();
    let mut ranked: Vec<&serde_json::Value> = patterns.iter().collect();
    ranked.sort_by(|a, b| {
        let ac = a.get("instance_count").and_then(|v| v.as_i64()).unwrap_or(0);
        let bc = b.get("instance_count").and_then(|v| v.as_i64()).unwrap_or(0);
        bc.cmp(&ac)
    });
    let pattern_summary: Vec<serde_json::Value> = ranked
        .into_iter()
        .take(MAX_CONVENTION_ROWS)
        .map(|p| {
            serde_json::json!({
                "name": p.get("name").cloned().unwrap_or(serde_json::Value::Null),
                "family": p.get("family").cloned().unwrap_or(serde_json::Value::Null),
                "lifecycle": p.get("lifecycle").cloned().unwrap_or(serde_json::Value::Null),
                "instance_count": p.get("instance_count").cloned().unwrap_or(serde_json::json!(0)),
            })
        })
        .collect();

    serde_json::json!({
        "naming": naming,
        "structure": {
            "file_count": file_count,
            "directories": directories,
            "directories_total": directories_total,
            "languages": languages,
        },
        "patterns": pattern_summary,
        "patterns_total": patterns_total,
    })
}

pub(crate) async fn project_conventions_handler(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let ids = state.pg.scope_folder_ids(&project).await
        .map_err(|e| { tracing::warn!(error = %e, project = %project, "project_conventions_handler: scope_folder_ids failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    if ids.is_empty() {
        return Ok(Json(serde_json::json!({
            "naming": [],
            "structure": {"file_count": 0, "directories": [], "languages": {}},
            "patterns": []
        })));
    }
    let nodes = state.pg.get_nodes_scoped(&ids).await
        .map_err(|e| { tracing::warn!(error = %e, project = %project, "project_conventions_handler: get_nodes_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    // Patterns are still per-folder; aggregate across all folders in scope.
    let mut patterns = Vec::new();
    for fid in &ids {
        let mut fp = state.pg.list_patterns_by_folder(fid).await
            .map_err(|e| { tracing::warn!(error = %e, project = %project, "project_conventions_handler: list_patterns_by_folder failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
        patterns.append(&mut fp);
    }
    Ok(Json(derive_conventions(&nodes, &patterns)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classify_case_covers_common_styles() {
        assert_eq!(classify_case("get_repo_by_name"), "snake_case");
        assert_eq!(classify_case("MAX_RETRIES"), "SCREAMING_SNAKE_CASE");
        assert_eq!(classify_case("repoId"), "camelCase");
        assert_eq!(classify_case("AppState"), "PascalCase");
        assert_eq!(classify_case("HTTPServer"), "PascalCase");
        assert_eq!(classify_case("my-component"), "kebab-case");
        assert_eq!(classify_case("main"), "snake_case");
        assert_eq!(classify_case("_private"), "snake_case");
        assert_eq!(classify_case("T"), "PascalCase");
        assert_eq!(classify_case("FOO"), "SCREAMING_SNAKE_CASE");
        assert_eq!(classify_case("My_Var"), "other");
    }

    #[test]
    fn derive_conventions_reports_dominant_naming_per_kind() {
        let nodes = vec![
            json!({"kind":"function","name":"get_user","file_path":"src/api.rs"}),
            json!({"kind":"function","name":"list_items","file_path":"src/api.rs"}),
            json!({"kind":"function","name":"fetchData","file_path":"src/api.rs"}),
            json!({"kind":"class","name":"UserStore","file_path":"src/store.rs"}),
            json!({"kind":"const","name":"MAX_SIZE","file_path":"src/store.rs"}),
            json!({"kind":"section","name":"Overview","file_path":"README.md"}),
            json!({"kind":"file","name":"api.rs","file_path":"src/api.rs"}),
            json!({"kind":"file","name":"store.rs","file_path":"src/store.rs"}),
            json!({"kind":"file","name":"App.svelte","file_path":"ui/App.svelte"}),
        ];
        let conv = derive_conventions(&nodes, &[]);
        let naming = conv["naming"].as_array().unwrap();

        let func = naming.iter().find(|n| n["kind"] == "function").unwrap();
        assert_eq!(func["dominant"], "snake_case");
        assert_eq!(func["count"], 3);
        assert!((func["conformance"].as_f64().unwrap() - 0.67).abs() < 0.01);

        let class = naming.iter().find(|n| n["kind"] == "class").unwrap();
        assert_eq!(class["dominant"], "PascalCase");

        // `section` is not a naming kind — excluded from naming analysis.
        assert!(naming.iter().all(|n| n["kind"] != "section"));

        assert_eq!(conv["structure"]["file_count"], 3);
        assert_eq!(conv["structure"]["languages"]["rust"], 2);
        assert_eq!(conv["structure"]["languages"]["svelte"], 1);
    }

    #[test]
    fn derive_conventions_empty_is_safe() {
        let conv = derive_conventions(&[], &[]);
        assert_eq!(conv["naming"].as_array().unwrap().len(), 0);
        assert_eq!(conv["structure"]["file_count"], 0);
        assert_eq!(conv["patterns"].as_array().unwrap().len(), 0);
    }

    // ── language_for_ext (1b Step 1 migration) ──────────────────────────
    //
    // The mapping used to be a 20-branch match. It now consults LanguageAdapter
    // for the extensions that have one and only carries hardcoded fallbacks for
    // langs that don't (go, ruby, shell, config files). Same output, one source
    // of truth for the adapter-backed languages.

    #[test]
    fn language_for_ext_uses_adapter_for_backed_languages() {
        assert_eq!(language_for_ext("rs"), "rust");
        assert_eq!(language_for_ext("ts"), "typescript");
        assert_eq!(language_for_ext("tsx"), "typescript");
        assert_eq!(language_for_ext("js"), "javascript");
        assert_eq!(language_for_ext("jsx"), "javascript");
        assert_eq!(language_for_ext("py"), "python");
        assert_eq!(language_for_ext("java"), "java");
        assert_eq!(language_for_ext("kt"), "kotlin");
        assert_eq!(language_for_ext("kts"), "kotlin");
        assert_eq!(language_for_ext("svelte"), "svelte");
        assert_eq!(language_for_ext("vue"), "vue");
        assert_eq!(language_for_ext("sql"), "sql");
        assert_eq!(language_for_ext("swift"), "swift");
        assert_eq!(language_for_ext("c"), "c");
        assert_eq!(language_for_ext("h"), "c");
        assert_eq!(language_for_ext("cpp"), "c");
    }

    #[test]
    fn language_for_ext_falls_back_for_adapterless_languages() {
        assert_eq!(language_for_ext("go"), "go");
        assert_eq!(language_for_ext("rb"), "ruby");
        assert_eq!(language_for_ext("sh"), "shell");
        assert_eq!(language_for_ext("bash"), "shell");
        assert_eq!(language_for_ext("md"), "markdown");
        assert_eq!(language_for_ext("toml"), "toml");
        assert_eq!(language_for_ext("yaml"), "yaml");
        assert_eq!(language_for_ext("json"), "json");
        assert_eq!(language_for_ext("css"), "css");
        assert_eq!(language_for_ext("html"), "html");
    }

    #[test]
    fn language_for_ext_unknown_passes_through() {
        // Preserved from the original: unknown ext returns the ext itself so
        // callers can still bucket them ("go" isn't in the adapter set — it
        // has a fallback; "elixir" would pass through as "elixir").
        assert_eq!(language_for_ext("elixir"), "elixir");
    }

    #[test]
    fn derive_conventions_caps_directories_and_reports_total() {
        // A project with far more than MAX_CONVENTION_ROWS directories must not
        // dump every one (that's the ~60K bloat) — cap the list but surface the
        // true total so the trim is visible, not silent. Highest-file-count dirs
        // survive the cap.
        let n = MAX_CONVENTION_ROWS + 40;
        let mut nodes = Vec::new();
        for i in 0..n {
            // Deeper index → more files, so the ranking is deterministic.
            let files = i + 1;
            for f in 0..files {
                nodes.push(json!({
                    "kind": "file",
                    "name": format!("f{f}.rs"),
                    "file_path": format!("src/dir{i:04}/f{f}.rs"),
                }));
            }
        }
        let conv = derive_conventions(&nodes, &[]);
        let dirs = conv["structure"]["directories"].as_array().unwrap();
        assert_eq!(dirs.len(), MAX_CONVENTION_ROWS, "directories must be capped");
        assert_eq!(
            conv["structure"]["directories_total"].as_u64().unwrap(),
            n as u64,
            "directories_total must report the true (uncapped) count",
        );
        // The densest directory (highest file count) must survive the cap.
        let top = &dirs[0];
        assert_eq!(top["path"], format!("src/dir{:04}", n - 1));
        // Still genuine: capped list is non-empty and well-formed.
        assert!(dirs.iter().all(|d| d["path"].is_string() && d["files"].is_number()));
    }

    #[test]
    fn derive_conventions_caps_patterns_by_instance_count() {
        let n = MAX_CONVENTION_ROWS + 10;
        let patterns: Vec<serde_json::Value> = (0..n)
            .map(|i| json!({
                "name": format!("P{i}"), "family": "structural",
                "lifecycle": "rule", "instance_count": i,
            }))
            .collect();
        let conv = derive_conventions(&[], &patterns);
        let ps = conv["patterns"].as_array().unwrap();
        assert_eq!(ps.len(), MAX_CONVENTION_ROWS, "patterns must be capped");
        assert_eq!(conv["patterns_total"].as_u64().unwrap(), n as u64);
        // Highest instance_count kept first.
        assert_eq!(ps[0]["name"], format!("P{}", n - 1));
    }

    #[test]
    fn derive_conventions_passes_through_patterns() {
        let patterns = vec![json!({
            "name":"Adapter","family":"structural","lifecycle":"rule","instance_count":4
        })];
        let conv = derive_conventions(&[], &patterns);
        let p = conv["patterns"].as_array().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0]["name"], "Adapter");
        assert_eq!(p[0]["family"], "structural");
        assert_eq!(p[0]["instance_count"], 4);
    }
}
