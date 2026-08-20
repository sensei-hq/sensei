use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

// ── Unified Query ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct QueryBody {
    /// The query string, e.g. "find auth functions in kavach"
    #[serde(rename = "q")]
    query: String,
    /// Optional repo scope
    #[serde(rename = "repoId")]
    repo_id: Option<String>,
    /// Optional solution scope
    #[serde(rename = "solutionId")]
    solution_id: Option<String>,
}

/// POST /api/query — unified query endpoint for desktop and MCP.
/// Routes queries to appropriate backends based on keywords.
pub(crate) async fn unified_query(
    State(state): State<AppState>,
    Json(body): Json<QueryBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let q = body.query.to_lowercase();
    let repo_id = body.repo_id.clone().unwrap_or_default();

    // Determine query type from keywords
    let result = if q.contains("lib") || q.contains("dependenc") || q.contains("package") {
        // Library query
        query_libs(&state, &q, &repo_id, &body.solution_id).await?
    } else if q.contains("function") || q.contains("method") || q.contains("fn ") || q.contains("def ") {
        // Function search
        query_functions(&state, &q, &repo_id).await?
    } else if q.contains("type") || q.contains("interface") || q.contains("class") || q.contains("struct") {
        // Type search
        query_types(&state, &q, &repo_id).await?
    } else if q.contains("who calls") || q.contains("callers") || q.contains("called by") {
        // Caller traceability
        query_callers(&state, &q, &repo_id).await?
    } else if q.contains("calls") || q.contains("callees") || q.contains("depends on") {
        // Callee traceability
        query_callees(&state, &q, &repo_id).await?
    } else if q.contains("file") || q.contains("component") || q.contains("tagged") || q.contains("framework") {
        // File/tag search
        query_files(&state, &q, &repo_id).await?
    } else if q.contains("pattern") || q.contains("hook") || q.contains("middleware") || q.contains("route") {
        // Pattern search (via tags)
        query_patterns(&state, &q, &repo_id).await?
    } else if q.contains("doc") || q.contains("readme") || q.contains("drift") {
        // Doc query
        query_docs(&state, &q, &repo_id).await?
    } else if q.contains("communit") || q.contains("cluster") || q.contains("module") {
        // Community/architecture query
        query_communities(&state, &repo_id).await?
    } else {
        // Default: search functions then types then lib docs
        query_general(&state, &q, &repo_id).await?
    };

    Ok(Json(result))
}

/// Resolve a repo_id / project name / project UUID to a list of folder UUIDs
/// (project-scoped). Returns an empty Vec if unknown.
pub(crate) async fn resolve_scope_ids(state: &AppState, repo_id: &str) -> Result<Vec<uuid::Uuid>, StatusCode> {
    if repo_id.is_empty() { return Ok(vec![]); } // genuine: no repo → empty scope
    // A DB error is NOT an empty scope — propagate it so callers 500 rather than
    // return "no results" for a repo that in fact has code.
    state.pg.scope_folder_ids(repo_id).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id, "resolve_scope_ids: scope_folder_ids failed"); StatusCode::INTERNAL_SERVER_ERROR })
}

/// Resolve a repo_id string to a single folder UUID.
/// Kept for callers that still need a single UUID (e.g. session/community ops).
pub(crate) async fn resolve_folder_id(state: &AppState, repo_id: &str) -> Result<Option<uuid::Uuid>, StatusCode> {
    Ok(resolve_scope_ids(state, repo_id).await?.into_iter().next())
}

pub(crate) async fn query_libs(state: &AppState, q: &str, repo_id: &str, _solution_id: &Option<String>) -> Result<serde_json::Value, StatusCode> {
    let repos = state.pg.list_repositories().await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "query_libs: list_repositories failed"); StatusCode::INTERNAL_SERVER_ERROR })?;

    let filtered: Vec<&serde_json::Value> = if !repo_id.is_empty() {
        repos.iter().filter(|p| p["name"].as_str() == Some(repo_id)).collect()
    } else {
        repos.iter().collect()
    };

    let mut all_libs: Vec<serde_json::Value> = Vec::new();
    for p in &filtered {
        let repo_name = p["name"].as_str().unwrap_or("");
        if let Some(libs_arr) = p["libs"].as_array() {
            for lib in libs_arr {
                if let Some(lib_str) = lib.as_str() {
                    all_libs.push(serde_json::json!({"name": lib_str, "repoId": repo_name}));
                }
            }
        }
    }

    // Also search libraries from PgStore
    let lib_docs = state.pg.list_libraries().await
        .map_err(|e| { tracing::warn!(error = %e, "query_libs: list_libraries failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    let _term = extract_search_term(q);

    Ok(serde_json::json!({
        "type": "libs",
        "query": q,
        "libs": all_libs,
        "libDocs": lib_docs.iter().take(5).map(|d| serde_json::json!({
            "title": d["name"], "summary": d.get("description").unwrap_or(&serde_json::json!(null)), "url": d.get("url"),
        })).collect::<Vec<_>>(),
    }))
}

pub(crate) async fn query_functions(state: &AppState, q: &str, repo_id: &str) -> Result<serde_json::Value, StatusCode> {
    let term = extract_search_term(q);
    let ids = resolve_scope_ids(state, repo_id).await?;
    let results = if !ids.is_empty() {
        let lexical = state.pg.search_functions_scoped(&ids, &term).await
            .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "query_functions: search_functions_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
        let query_vec = embed_query(state, q).await;
        fuse_semantic(state, query_vec.as_ref(), &ids, lexical, FUNCTION_KINDS, function_hit).await
    } else {
        vec![]
    };
    Ok(serde_json::json!({
        "type": "functions",
        "query": q,
        "results": results,
    }))
}

pub(crate) async fn query_types(state: &AppState, q: &str, repo_id: &str) -> Result<serde_json::Value, StatusCode> {
    let term = extract_search_term(q);
    let ids = resolve_scope_ids(state, repo_id).await?;
    let results = if !ids.is_empty() {
        let lexical = state.pg.search_types_scoped(&ids, &term).await
            .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "query_types: search_types_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
        let query_vec = embed_query(state, q).await;
        fuse_semantic(state, query_vec.as_ref(), &ids, lexical, TYPE_KINDS, type_hit).await
    } else {
        vec![]
    };
    Ok(serde_json::json!({
        "type": "types",
        "query": q,
        "results": results,
    }))
}

pub(crate) async fn query_callers(state: &AppState, q: &str, repo_id: &str) -> Result<serde_json::Value, StatusCode> {
    let term = extract_search_term(q);
    let results = state.pg.get_callers_by_name(repo_id, &term).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "query_callers: get_callers_by_name failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(serde_json::json!({
        "type": "callers",
        "query": q,
        "function": term,
        "results": results,
    }))
}

pub(crate) async fn query_callees(state: &AppState, q: &str, repo_id: &str) -> Result<serde_json::Value, StatusCode> {
    let term = extract_search_term(q);
    let results = state.pg.get_callees_by_name(repo_id, &term).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "query_callees: get_callees_by_name failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(serde_json::json!({
        "type": "callees",
        "query": q,
        "function": term,
        "results": results,
    }))
}

pub(crate) async fn query_files(state: &AppState, q: &str, repo_id: &str) -> Result<serde_json::Value, StatusCode> {
    let term = extract_search_term(q);
    let results = state.pg.get_files_by_tag(repo_id, &term).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "query_files: get_files_by_tag failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(serde_json::json!({ "type": "files", "query": q, "results": results }))
}

pub(crate) async fn query_patterns(state: &AppState, q: &str, repo_id: &str) -> Result<serde_json::Value, StatusCode> {
    let tag = if q.contains("hook") { "hook" }
        else if q.contains("middleware") { "middleware" }
        else if q.contains("route") { "route" }
        else if q.contains("handler") { "handler" }
        else if q.contains("component") { "component" }
        else { &extract_search_term(q) };
    let results = state.pg.get_files_by_tag(repo_id, tag).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, tag = %tag, "query_patterns: get_files_by_tag failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(serde_json::json!({ "type": "patterns", "query": q, "pattern": tag, "results": results }))
}

pub(crate) async fn query_docs(state: &AppState, q: &str, repo_id: &str) -> Result<serde_json::Value, StatusCode> {
    let drifted = state.pg.get_doc_drift(repo_id).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "query_docs: get_doc_drift failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(serde_json::json!({
        "type": "docs",
        "query": q,
        "driftedDocs": drifted,
    }))
}

pub(crate) async fn query_communities(state: &AppState, repo_id: &str) -> Result<serde_json::Value, StatusCode> {
    // Communities are stored per-folder; aggregate across all scoped folders.
    let ids = resolve_scope_ids(state, repo_id).await?;
    let communities = state.pg.list_communities_scoped(&ids).await
        .map_err(|e| { tracing::warn!(error = %e, repo_id = %repo_id, "query_communities: list_communities_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(serde_json::json!({
        "type": "communities",
        "results": communities,
    }))
}

pub(crate) async fn query_general(state: &AppState, q: &str, repo_id: &str) -> Result<serde_json::Value, StatusCode> {
    let term = extract_search_term(q);
    let ids = resolve_scope_ids(state, repo_id).await?;
    let (functions, types) = if !ids.is_empty() {
        let fns_lex = state.pg.search_functions_scoped(&ids, &term).await
            .map_err(|e| { tracing::warn!(error = %e, "query_general: search_functions_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
        let tys_lex = state.pg.search_types_scoped(&ids, &term).await
            .map_err(|e| { tracing::warn!(error = %e, "query_general: search_types_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
        // Embed once, fuse the semantic candidates into both node result sets.
        // (fuse_semantic/embed_query stay fail-open — a missing embedding degrades
        // to the lexical order, which is additive, not error-masking.)
        let query_vec = embed_query(state, q).await;
        let fns = fuse_semantic(state, query_vec.as_ref(), &ids, fns_lex, FUNCTION_KINDS, function_hit).await;
        let tys = fuse_semantic(state, query_vec.as_ref(), &ids, tys_lex, TYPE_KINDS, type_hit).await;
        (fns, tys)
    } else {
        (vec![], vec![])
    };

    let lib_docs = state.pg.list_libraries().await
        .map_err(|e| { tracing::warn!(error = %e, "query_general: list_libraries failed"); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok(serde_json::json!({
        "type": "general",
        "query": q,
        "functions": functions,
        "types": types,
        "libDocs": lib_docs.iter().take(5).map(|d| serde_json::json!({
            "title": d["name"], "summary": d.get("description").unwrap_or(&serde_json::json!(null)),
        })).collect::<Vec<_>>(),
    }))
}

/// `context_pack` — assemble a ready-to-use context bundle for `q`: relevant code
/// with actual snippets read from disk, so an assistant gets concept-level
/// retrieval in one call instead of a search followed by N file reads. Two recall
/// arms: (1) the **symbol arm** — top hybrid (lexical ILIKE + semantic NN) hits
/// over indexed `sensei.nodes`; (2) the **content-grep arm** — a bounded raw
/// file-content grep over the scoped repo roots, which finds concepts that live
/// only in file *content* (comments, string literals, config values, enum
/// variants, string-dispatched tool names, serde-renamed fields) and are never
/// indexed as a symbol. Each item is tagged `via: "symbol" | "grep"`. Fail-open:
/// no repo roots / an empty grep / an unreadable file degrade to the symbol arm
/// (or an empty snippet), never an error.
pub(crate) async fn context_pack(state: &AppState, q: &str, repo_id: &str) -> Result<serde_json::Value, StatusCode> {
    let general = query_general(state, q, repo_id).await?;
    // Collect top-k symbol ids in ranked order (functions first, then types).
    let mut ordered_ids: Vec<uuid::Uuid> = Vec::new();
    for key in ["functions", "types"] {
        if let Some(arr) = general.get(key).and_then(|v| v.as_array()) {
            for it in arr {
                if ordered_ids.len() >= CONTEXT_PACK_K { break; }
                if let Some(id) = it.get("id").and_then(|v| v.as_str()).and_then(|s| uuid::Uuid::parse_str(s).ok()) {
                    ordered_ids.push(id);
                }
            }
        }
        if ordered_ids.len() >= CONTEXT_PACK_K { break; }
    }

    let locs = state.pg.node_locations(&ordered_ids).await
        .map_err(|e| { tracing::warn!(error = %e, "context_pack: node_locations failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    let mut items: Vec<serde_json::Value> = Vec::new();
    // Files already packed by the symbol arm — the grep arm skips them so a
    // symbol and a comment in the same file don't produce two near-dupe entries.
    let mut packed_files: std::collections::HashSet<String> = std::collections::HashSet::new();
    for id in &ordered_ids {
        let Some((_, abs_path, file_path, ls, le, kind, name, sig)) = locs.iter().find(|l| &l.0 == id) else { continue };
        let snippet = std::fs::read_to_string(std::path::Path::new(abs_path).join(file_path))
            .ok()
            .map(|c| extract_snippet(&c, *ls, *le, SNIPPET_MAX_LINES))
            .unwrap_or_default();
        packed_files.insert(file_path.clone());
        items.push(serde_json::json!({
            "name": name, "kind": kind, "file": file_path,
            "lines": format!("{ls}-{le}"), "signature": sig, "snippet": snippet, "via": "symbol",
        }));
    }

    // Content-grep arm: concepts that live only in file content.
    items.extend(grep_context_items(state, repo_id, &extract_search_term(q), &packed_files).await);

    Ok(serde_json::json!({ "type": "context_pack", "query": q, "count": items.len(), "items": items }))
}

/// The content-grep arm of `context_pack`: up to `CONTEXT_PACK_GREP_K` raw
/// file-content matches over the scoped repo roots, each rendered as a small
/// snippet centered on the match. Skips files already packed by the symbol arm
/// and de-dupes by (file, line). Fail-open: no scope / no repo roots / empty
/// grep → no items.
async fn grep_context_items(
    state: &AppState,
    repo_id: &str,
    term: &str,
    packed_files: &std::collections::HashSet<String>,
) -> Vec<serde_json::Value> {
    // Fail-open (supplementary grep arm): a scope-resolution error yields no grep
    // items rather than 500-ing the whole context_pack.
    let Ok(ids) = resolve_scope_ids(state, repo_id).await else { return vec![] };
    if ids.is_empty() {
        return vec![];
    }
    let roots = state.pg.scope_repo_roots(&ids).await.unwrap_or_default();
    if roots.is_empty() {
        return vec![];
    }
    let mut grep_roots: Vec<(std::path::PathBuf, Vec<String>)> = Vec::with_capacity(roots.len());
    for r in &roots {
        // Exclusions are keyed by watch-root path; a repo that is also a watch
        // root contributes its excluded prefixes, otherwise this is empty. Fail
        // closed: on a read error skip this root rather than grep it with no
        // exclusions (which could leak excluded-folder content into the context
        // pack). The grep otherwise stays best-effort.
        let excl = match state.pg.root_exclusion_prefixes(r).await {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(error = %e, root = %r,
                    "context_pack grep: exclusion read failed; skipping root");
                continue;
            }
        };
        grep_roots.push((std::path::PathBuf::from(r), excl));
    }
    let opts = GrepOpts { max_matches: CONTEXT_PACK_GREP_K, ..GrepOpts::default() };
    let matches = content_grep(&grep_roots, term, &opts);

    let mut out: Vec<serde_json::Value> = Vec::new();
    let mut seen: std::collections::HashSet<(String, usize)> = std::collections::HashSet::new();
    for m in matches {
        if packed_files.contains(&m.rel_path) || !seen.insert((m.rel_path.clone(), m.line_no)) {
            continue;
        }
        let snippet = std::fs::read_to_string(&m.abs_path)
            .ok()
            .map(|c| {
                let start = m.line_no.saturating_sub(GREP_CONTEXT_LINES).max(1) as i32;
                let end = (m.line_no + GREP_CONTEXT_LINES) as i32;
                extract_snippet(&c, start, end, GREP_SNIPPET_MAX_LINES)
            })
            .unwrap_or_default();
        out.push(serde_json::json!({
            "file": m.rel_path,
            "lines": m.line_no.to_string(),
            "match": m.line,
            "snippet": snippet,
            "via": "grep",
        }));
    }
    out
}

/// Max content-grep hits packed into a `context_pack` response.
const CONTEXT_PACK_GREP_K: usize = 6;
/// Lines of context on each side of a grep match in its packed snippet.
const GREP_CONTEXT_LINES: usize = 3;
/// Hard cap on a grep snippet's line span.
const GREP_SNIPPET_MAX_LINES: usize = 12;

/// Extract lines `[start, end]` (1-based, inclusive) from `content`, capped at
/// `max_lines`. Out-of-range bounds clamp; an empty or past-EOF start yields "".
pub(crate) fn extract_snippet(content: &str, start: i32, end: i32, max_lines: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let n = lines.len();
    if n == 0 || start < 1 || (start as usize) > n {
        return String::new();
    }
    let s = start as usize - 1; // 0-based first line
    let want_end = end.max(start) as usize; // 1-based inclusive last line
    let e = want_end.min(n).min(s + max_lines).max(s + 1);
    lines[s..e.min(n)].join("\n")
}

/// Max symbols packed into a `context_pack` response — bounds token cost.
const CONTEXT_PACK_K: usize = 8;
/// Max lines per packed snippet — bounds a single large function's footprint.
const SNIPPET_MAX_LINES: usize = 40;

// ── Content grep (raw file-content recall floor) ─────────────────────────────
//
// The symbol arms (lexical ILIKE + semantic NN) only surface things indexed as
// `sensei.nodes`. Concepts that live in *file content* — comments, string
// literals, config values, Rust enum variants, string-dispatched MCP tool
// names, serde-renamed API fields — aren't retrievable that way (the G3/G4b
// recall gap). This is a bounded, in-process content grep using ripgrep's
// `ignore` walker (so `.gitignore`/hidden files are skipped) as the missing
// floor. Fail-open and hard-bounded so it can never stall a query.

/// A single content-grep match.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GrepMatch {
    /// Absolute path to the file (for reading a snippet).
    pub abs_path: std::path::PathBuf,
    /// Path relative to the repo root (for display + dedup vs symbol hits).
    pub rel_path: String,
    /// 1-based line number of the match.
    pub line_no: usize,
    /// The matching line, trimmed and length-capped.
    pub line: String,
}

/// Hard bounds so a grep over a large tree can never run away.
pub(crate) struct GrepOpts {
    /// Total matches across all roots.
    pub max_matches: usize,
    /// Matches kept from a single file (stops one file dominating).
    pub max_per_file: usize,
    /// Files inspected before the walk gives up (bounds worst-case work).
    pub max_files: usize,
    /// Skip files larger than this many bytes (binaries, minified bundles).
    pub max_file_bytes: u64,
    /// Truncate a returned line to this many characters.
    pub max_line_len: usize,
}

impl Default for GrepOpts {
    fn default() -> Self {
        Self { max_matches: 12, max_per_file: 2, max_files: 5000, max_file_bytes: 512 * 1024, max_line_len: 240 }
    }
}

/// Case-insensitive substring content grep over `roots` — each an
/// `(abs_root, excluded_relative_prefixes)` pair — using the `ignore` walker.
/// Deterministic (roots in order, then walk order, then ascending line). Fail-
/// open: unreadable / non-UTF8 / oversized files are skipped silently. A
/// sensei-excluded prefix prunes the whole subtree (dirs and files).
pub(crate) fn content_grep(
    roots: &[(std::path::PathBuf, Vec<String>)],
    term: &str,
    opts: &GrepOpts,
) -> Vec<GrepMatch> {
    let needle = term.trim().to_lowercase();
    if needle.is_empty() {
        return vec![];
    }
    let mut out: Vec<GrepMatch> = Vec::new();
    let mut files_seen = 0usize;
    for (root, excluded) in roots {
        if out.len() >= opts.max_matches || files_seen >= opts.max_files {
            break;
        }
        let excluded_owned = excluded.clone();
        let root_owned = root.clone();
        let walker = ignore::WalkBuilder::new(root)
            .hidden(true)
            .git_ignore(true)
            .git_global(false)
            .parents(false)
            .filter_entry(move |dent| {
                // Prune anything under a sensei-excluded prefix (dir or file).
                let Ok(rel) = dent.path().strip_prefix(&root_owned) else { return true };
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                !excluded_owned
                    .iter()
                    .any(|p| rel_str == *p || rel_str.starts_with(&format!("{p}/")))
            })
            .build();
        for dent in walker {
            if out.len() >= opts.max_matches || files_seen >= opts.max_files {
                break;
            }
            let Ok(dent) = dent else { continue };
            if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            files_seen += 1;
            // Missing metadata or oversized → skip (conservative).
            if dent.metadata().map(|m| m.len() > opts.max_file_bytes).unwrap_or(true) {
                continue;
            }
            let path = dent.path();
            let Ok(content) = std::fs::read_to_string(path) else { continue };
            let rel_str = path
                .strip_prefix(root)
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| path.to_string_lossy().into_owned());
            let mut per_file = 0usize;
            for (i, line) in content.lines().enumerate() {
                if per_file >= opts.max_per_file || out.len() >= opts.max_matches {
                    break;
                }
                if line.to_lowercase().contains(&needle) {
                    let capped: String = line.trim().chars().take(opts.max_line_len).collect();
                    out.push(GrepMatch {
                        abs_path: path.to_path_buf(),
                        rel_path: rel_str.clone(),
                        line_no: i + 1,
                        line: capped,
                    });
                    per_file += 1;
                }
            }
        }
    }
    out
}

// ── Hybrid ranking (lexical + semantic fusion) ───────────────────────────────
//
// `/api/query`'s node searches are keyword-only (ILIKE over sensei.nodes). This
// layer fuses in embedding nearest-neighbours so a query is ranked by BOTH
// lexical and semantic relevance. It is strictly additive and fail-open: with no
// query embedding (gateway down / no embed chain) or no semantic hits, the
// lexical order is returned unchanged — never worse than keyword-only.

/// Node kinds treated as "functions" for the scoped lexical search + semantic NN.
const FUNCTION_KINDS: &[&str] = &["function", "method"];
/// Node kinds treated as "types" for the scoped lexical search + semantic NN.
const TYPE_KINDS: &[&str] = &["class", "struct", "interface", "enum", "type"];
/// Max semantic NN candidates fused per query — bounds the extra work so the
/// common path doesn't get materially slower.
const SEM_CANDIDATES: i64 = 25;
/// Upper bound on fused results returned (mirrors the lexical `LIMIT 50`).
const HYBRID_MAX: usize = 50;
/// Query-embed timeout. Semantic ranking is additive, so a slow embed backend
/// must never stall the query — on timeout we return keyword-only results.
const EMBED_QUERY_TIMEOUT_SECS: u64 = 10;
/// RRF constant. 60 is the value from Cormack et al. and the de-facto default;
/// it damps low-ranked items so a hit near the top of either list dominates.
const RRF_K: f64 = 60.0;

/// Row shape returned by `PgStore::semantic_search_nodes`.
type SemRow = (uuid::Uuid, String, String, Option<String>, Option<i32>);

/// A single ranked search hit: a de-duplication key (`id`) plus the JSON item
/// returned to the caller unchanged.
#[derive(Clone, Debug)]
pub(crate) struct Hit {
    pub id: String,
    pub item: serde_json::Value,
}

/// Project a semantic row into a function hit — identical shape to
/// `PgStore::search_functions_scoped` so fused results stay homogeneous.
fn function_hit(r: SemRow) -> serde_json::Value {
    serde_json::json!({ "id": r.0, "name": r.1, "file_path": r.2, "signature": r.3, "line_start": r.4 })
}

/// Project a semantic row into a type hit — identical shape to
/// `PgStore::search_types_scoped` (no `signature`).
fn type_hit(r: SemRow) -> serde_json::Value {
    serde_json::json!({ "id": r.0, "name": r.1, "file_path": r.2, "line_start": r.4 })
}

/// Fuse a lexical (keyword) and a semantic (embedding NN) ranked list with
/// Reciprocal Rank Fusion. Each hit contributes `1 / (RRF_K + rank)` (rank
/// 1-based); an item present in both lists sums both contributions, so it
/// outranks an item ranked highly in only one list. De-dupes by `id`.
///
/// Fail-open by construction: an empty `semantic` list reproduces the lexical
/// order exactly (scores strictly decrease with rank) and an empty `lexical`
/// list reproduces the semantic order — so a missing query embedding degrades to
/// keyword-only ranking with no special-casing.
pub(crate) fn fuse_rankings(lexical: &[Hit], semantic: &[Hit]) -> Vec<Hit> {
    use std::collections::HashMap;
    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut items: HashMap<String, serde_json::Value> = HashMap::new();
    // First-seen (lexical-first) order gives a stable tie-break on equal scores.
    let mut order: Vec<String> = Vec::new();
    for list in [lexical, semantic] {
        for (rank, hit) in list.iter().enumerate() {
            let contrib = 1.0 / (RRF_K + (rank + 1) as f64);
            let entry = scores.entry(hit.id.clone()).or_insert_with(|| {
                order.push(hit.id.clone());
                items.insert(hit.id.clone(), hit.item.clone());
                0.0
            });
            *entry += contrib;
        }
    }
    // Stable sort by score desc keeps first-seen order on exact ties.
    order.sort_by(|a, b| {
        scores[b]
            .partial_cmp(&scores[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    order
        .into_iter()
        .filter_map(|id| items.remove(&id).map(|item| Hit { id, item }))
        .collect()
}

/// Convert JSON result items into `Hit`s keyed by their `id` field. Items
/// without a string `id` are dropped (they can't be de-duplicated or fused).
fn to_hits(items: Vec<serde_json::Value>) -> Vec<Hit> {
    items
        .into_iter()
        .filter_map(|item| {
            let id = item.get("id")?.as_str()?.to_string();
            Some(Hit { id, item })
        })
        .collect()
}

/// Embed the query string via the pinned 384-dim `embed` chain — the same chain
/// `EmbedNodes` uses, so query and node vectors share a space. Fail-open: any
/// gateway error, missing chain, timeout, or empty result logs a warning and
/// yields `None`, so the caller falls back to keyword-only ranking.
async fn embed_query(state: &AppState, text: &str) -> Option<Vec<f32>> {
    use gateway::types::capability::Capability;
    use gateway::types::request::{InferenceRequest, Payload};
    if text.trim().is_empty() {
        return None;
    }
    let request = InferenceRequest {
        capability: Capability::TextEmbed,
        model: None,
        router: None,
        chain: Some("embed".to_string()),
        payload: Payload::Embed { texts: vec![text.to_string()] },
        budget: None,
        auth: None,
        panel: None,
        consensus: None,
        allow_fallback: true,
        credentials: std::collections::HashMap::new(),
    };
    match tokio::time::timeout(
        std::time::Duration::from_secs(EMBED_QUERY_TIMEOUT_SECS),
        state.gateway.execute(&request),
    )
    .await
    {
        Ok(Ok(resp)) => match resp.embeddings.and_then(|mut e| e.pop()) {
            Some(v) if !v.is_empty() => Some(v),
            _ => {
                tracing::warn!("embed_query: empty embedding — keyword-only ranking");
                None
            }
        },
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "embed_query: gateway embed failed — keyword-only ranking");
            None
        }
        Err(_) => {
            tracing::warn!("embed_query: embed timed out — keyword-only ranking");
            None
        }
    }
}

/// Fetch semantic NN candidates for the same folders + node kinds and fuse them
/// with the lexical results. Additive + fail-open: no query embedding
/// (`query_vec = None`), an empty NN, or a failed NN all return the lexical
/// order unchanged.
async fn fuse_semantic(
    state: &AppState,
    query_vec: Option<&Vec<f32>>,
    ids: &[uuid::Uuid],
    lexical: Vec<serde_json::Value>,
    kinds: &[&str],
    projector: fn(SemRow) -> serde_json::Value,
) -> Vec<serde_json::Value> {
    let Some(query_vec) = query_vec else {
        return lexical;
    };
    let sem_rows = match state.pg.semantic_search_nodes(ids, query_vec, kinds, SEM_CANDIDATES).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "fuse_semantic: semantic_search_nodes failed — keyword-only ranking");
            return lexical;
        }
    };
    if sem_rows.is_empty() {
        return lexical;
    }
    let semantic: Vec<serde_json::Value> = sem_rows.into_iter().map(projector).collect();
    let mut fused: Vec<serde_json::Value> =
        fuse_rankings(&to_hits(lexical), &to_hits(semantic))
            .into_iter()
            .map(|h| h.item)
            .collect();
    fused.truncate(HYBRID_MAX);
    fused
}

/// Extract the most meaningful search term from a natural language query.
pub(crate) fn extract_search_term(q: &str) -> String {
    let stop_words = ["find", "search", "show", "get", "list", "what", "which", "where",
        "how", "the", "in", "for", "from", "all", "me", "that", "are", "is",
        "function", "functions", "method", "methods", "type", "types", "class",
        "interface", "lib", "libs", "library", "libraries", "file", "files",
        "who", "calls", "called", "by", "does", "do", "callers", "callees",
        "pattern", "patterns", "doc", "docs", "a", "an", "of", "with"];

    let words: Vec<&str> = q.split_whitespace()
        .filter(|w| !stop_words.contains(&w.to_lowercase().as_str()))
        .collect();

    // Return the longest non-stop word (likely the most specific)
    words.into_iter()
        .max_by_key(|w| w.len())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_snippet_clamps_range_and_caps() {
        let content = "l1\nl2\nl3\nl4\nl5";
        // Inclusive 1-based range.
        assert_eq!(extract_snippet(content, 2, 4, 40), "l2\nl3\nl4");
        // end past EOF clamps to the last line.
        assert_eq!(extract_snippet(content, 4, 99, 40), "l4\nl5");
        // max_lines caps the span.
        assert_eq!(extract_snippet(content, 1, 5, 2), "l1\nl2");
        // start past EOF or invalid → empty.
        assert_eq!(extract_snippet(content, 9, 9, 40), "");
        assert_eq!(extract_snippet(content, 0, 3, 40), "");
        assert_eq!(extract_snippet("", 1, 3, 40), "");
        // A single-line symbol (start == end).
        assert_eq!(extract_snippet(content, 3, 3, 40), "l3");
    }

    #[test]
    fn content_grep_is_case_insensitive_bounded_and_excludes() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // Match lives only in a comment + a string literal — never a symbol name.
        fs::write(root.join("a.rs"), "// TextEmbed capability\nfn foo() {}\nlet x = \"TextEmbed\";\n").unwrap();
        fs::write(root.join("b.txt"), "nothing relevant here\n").unwrap();
        fs::create_dir_all(root.join("vendor")).unwrap();
        fs::write(root.join("vendor/c.rs"), "TextEmbed inside an excluded subtree\n").unwrap();

        let roots = vec![(root.to_path_buf(), vec!["vendor".to_string()])];
        let opts = GrepOpts { max_matches: 10, max_per_file: 1, max_files: 100, max_file_bytes: 1 << 20, max_line_len: 240 };

        // Case-insensitive query; per-file cap 1 keeps only a.rs's first hit;
        // the `vendor/` subtree is pruned; b.txt doesn't match.
        let hits = content_grep(&roots, "textembed", &opts);
        assert_eq!(hits.len(), 1, "one match: a.rs line 1 (per-file cap 1, vendor excluded, b.txt no match)");
        assert_eq!(hits[0].rel_path, "a.rs");
        assert_eq!(hits[0].line_no, 1);
        assert!(hits[0].line.contains("TextEmbed"), "returns the matching line text");

        // Raising the per-file cap surfaces the second in-file match (the string).
        let opts2 = GrepOpts { max_per_file: 5, ..opts };
        let more = content_grep(&roots, "textembed", &opts2);
        assert_eq!(more.len(), 2, "both a.rs matches with a higher per-file cap; vendor still excluded");
        assert_eq!(more[1].line_no, 3);

        // Empty / whitespace term → no matches (never greps for nothing).
        assert!(content_grep(&roots, "   ", &opts2).is_empty());
    }

    /// Build a hit whose id doubles as its item, so ranking order is readable.
    fn hit(id: &str) -> Hit {
        Hit { id: id.to_string(), item: serde_json::json!({ "id": id }) }
    }

    fn ids(hits: &[Hit]) -> Vec<String> {
        hits.iter().map(|h| h.id.clone()).collect()
    }

    #[test]
    fn fuse_ranks_shared_hit_first_and_dedupes() {
        // C is ranked in both lists → its RRF score sums both contributions and
        // beats every item present in only one list. Each id appears once.
        let lexical = [hit("A"), hit("B"), hit("C")];
        let semantic = [hit("C"), hit("D")];
        let fused = fuse_rankings(&lexical, &semantic);
        assert_eq!(ids(&fused), vec!["C", "A", "B", "D"], "shared-in-both C leads; single-list order preserved");
        assert_eq!(fused.len(), 4, "C is de-duplicated, not counted twice");
    }

    #[test]
    fn fuse_empty_semantic_preserves_lexical_order() {
        // Fallback path: no query embedding ⇒ empty semantic ⇒ lexical unchanged.
        let lexical = [hit("A"), hit("B"), hit("C")];
        let fused = fuse_rankings(&lexical, &[]);
        assert_eq!(ids(&fused), vec!["A", "B", "C"]);
    }

    #[test]
    fn fuse_empty_lexical_uses_semantic_order() {
        // Pure-semantic path (no keyword matches) ⇒ semantic order.
        let semantic = [hit("X"), hit("Y"), hit("Z")];
        let fused = fuse_rankings(&[], &semantic);
        assert_eq!(ids(&fused), vec!["X", "Y", "Z"]);
    }

    #[test]
    fn fuse_both_empty_is_empty() {
        assert!(fuse_rankings(&[], &[]).is_empty());
    }

    #[test]
    fn to_hits_keys_by_id_and_drops_idless() {
        let items = vec![
            serde_json::json!({ "id": "n1", "name": "a" }),
            serde_json::json!({ "name": "no id" }),
            serde_json::json!({ "id": 42 }), // non-string id is dropped
        ];
        let hits = to_hits(items);
        assert_eq!(ids(&hits), vec!["n1"], "only the string-id item survives");
    }
}
