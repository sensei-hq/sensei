use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

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
    let folder = state.pg.get_repo_by_name(&repo_id).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let nodes = state.pg.get_nodes_by_folder(&folder_id).await.unwrap_or_default();
            let edges = state.pg.get_edges_by_kind(&folder_id, "calls").await.unwrap_or_default();
            return Ok(Json(serde_json::json!({"nodes": nodes, "edges": edges})));
        }
    Ok(Json(serde_json::json!({"nodes": [], "edges": []})))
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
    let folder = state.pg.get_repo_by_name(&q.repo_id).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let results = state.pg.search_functions(&folder_id, &q.query).await.unwrap_or_default();
            return Ok(Json(results));
        }
    Ok(Json(vec![]))
}

pub(crate) async fn search_types(
    State(state): State<AppState>,
    Query(q): Query<SymbolQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let folder = state.pg.get_repo_by_name(&q.repo_id).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let results = state.pg.search_types(&folder_id, &q.query).await.unwrap_or_default();
            return Ok(Json(results));
        }
    Ok(Json(vec![]))
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
    let results = state.pg.get_callers_by_name(&q.repo_id, &q.name).await.unwrap_or_default();
    Ok(Json(results))
}

pub(crate) async fn fn_callees(
    State(state): State<AppState>,
    Query(q): Query<TraceQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let results = state.pg.get_callees_by_name(&q.repo_id, &q.name).await.unwrap_or_default();
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
    let results = state.pg.get_files_by_tag(&q.repo_id, &q.tag).await.unwrap_or_default();
    Ok(Json(serde_json::json!(results)))
}

pub(crate) async fn doc_drift(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    let results = state.pg.get_doc_drift(&repo_id).await.unwrap_or_default();
    Ok(Json(serde_json::json!(results)))
}

pub(crate) async fn call_flow(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    let folder = state.pg.get_repo_by_name(&repo_id).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let edges = state.pg.get_edges_by_kind(&folder_id, "calls").await.unwrap_or_default();
            let nodes = state.pg.get_nodes_by_folder(&folder_id).await.unwrap_or_default();
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
            return Ok(Json(serde_json::json!({
                "modules": modules,
                "calls": edges,
                "moduleCount": modules.len(),
                "exportCount": modules.iter().map(|m| m["exports"].as_array().map_or(0, |a| a.len())).sum::<usize>(),
                "callCount": edges.len(),
            })));
        }
    Ok(Json(serde_json::json!({"modules": [], "calls": [], "moduleCount": 0, "exportCount": 0, "callCount": 0})))
}

pub(crate) async fn detect_communities(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = body["repoId"].as_str().unwrap_or_default().to_string();
    if repo_id.is_empty() {
        return Ok(Json(serde_json::json!({"error": "repoId required"})));
    }
    let folder = state.pg.get_repo_by_name(&repo_id).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let communities = state.pg.list_communities(&folder_id).await.unwrap_or_default();
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
    let folder = state.pg.get_repo_by_name(&repo_id).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let communities = state.pg.list_communities(&folder_id).await.unwrap_or_default();
            return Ok(Json(serde_json::json!(communities)));
        }
    Ok(Json(serde_json::json!([])))
}

// ── Patterns ────────────────────────────────────────────────────────────────

pub(crate) async fn detect_patterns(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    // Look up folder UUID and query patterns from PgStore
    let folder = state.pg.get_repo_by_name(&project).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let patterns = state.pg.list_patterns_by_folder(&folder_id).await.unwrap_or_default();
            return Json(serde_json::json!({"ok": true, "patterns": patterns, "count": patterns.len()}));
        }
    Json(serde_json::json!({"ok": false, "error": "project not found"}))
}

pub(crate) async fn list_patterns(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    let folder = state.pg.get_repo_by_name(&project).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let patterns = state.pg.list_patterns_by_folder(&folder_id).await.unwrap_or_default();
            return Json(serde_json::json!({"patterns": patterns, "count": patterns.len()}));
        }
    Json(serde_json::json!({"patterns": [], "count": 0}))
}

#[derive(Deserialize)]
pub(crate) struct MatchQuery {
    pub description: Option<String>,
}

pub(crate) async fn match_pattern_handler(
    State(state): State<AppState>,
    Path(project): Path<String>,
    Query(q): Query<MatchQuery>,
) -> Json<serde_json::Value> {
    let desc = q.description.unwrap_or_default();
    // Search patterns by folder using BM25 ranking
    let folder = state.pg.get_repo_by_name(&project).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let ranked = state.pg.rank_bm25(&folder_id, &desc).await.unwrap_or_default();
            let matches: Vec<serde_json::Value> = ranked.into_iter()
                .map(|(name, score)| serde_json::json!({"name": name, "score": score}))
                .collect();
            return Json(serde_json::json!({"matches": matches, "count": matches.len()}));
        }
    Json(serde_json::json!({"matches": [], "count": 0}))
}

pub(crate) async fn pattern_for_symbol(
    State(state): State<AppState>,
    Path((project, symbol)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    // Search patterns by folder, then filter by symbol
    let folder = state.pg.get_repo_by_name(&project).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let patterns = state.pg.list_patterns_by_folder(&folder_id).await.unwrap_or_default();
            // Find pattern whose members include this symbol
            for p in &patterns {
                if let Some(members) = p.get("members").and_then(|m| m.as_array())
                    && members.iter().any(|m| m.as_str() == Some(&symbol)) {
                        return Json(p.clone());
                    }
            }
        }
    Json(serde_json::json!({"pattern": null, "message": "symbol does not belong to any detected pattern"}))
}

pub(crate) async fn find_duplicates_handler(
    State(_state): State<AppState>,
    Path(_project): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: implement duplicate detection via graph analysis
    Json(serde_json::json!({"duplicates": [], "count": 0}))
}

// ── Conventions ───────────────────────────────────────────────────────────────

/// Code-symbol node kinds whose names carry naming conventions worth reporting.
const NAMING_KINDS: &[&str] = &[
    "function", "method", "class", "interface",
    "type", "const", "enum", "enum_variant", "field", "property",
];

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
fn language_for_ext(ext: &str) -> &str {
    match ext {
        "rs" => "rust",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "typescript",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "rb" => "ruby",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "svelte" => "svelte",
        "vue" => "vue",
        "sql" => "sql",
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
    let directories: Vec<serde_json::Value> = dirs
        .into_iter()
        .map(|(path, files)| serde_json::json!({"path": path, "files": files}))
        .collect();

    let languages: serde_json::Map<String, serde_json::Value> =
        lang_files.into_iter().map(|(k, v)| (k, serde_json::json!(v))).collect();

    let pattern_summary: Vec<serde_json::Value> = patterns
        .iter()
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
            "languages": languages,
        },
        "patterns": pattern_summary,
    })
}

pub(crate) async fn project_conventions_handler(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    let folder = state.pg.get_repo_by_name(&project).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let nodes = state.pg.get_nodes_by_folder(&folder_id).await.unwrap_or_default();
            let patterns = state.pg.list_patterns_by_folder(&folder_id).await.unwrap_or_default();
            return Json(derive_conventions(&nodes, &patterns));
        }
    Json(serde_json::json!({
        "naming": [],
        "structure": {"file_count": 0, "directories": [], "languages": {}},
        "patterns": []
    }))
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
