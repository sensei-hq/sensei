use super::query::{resolve_folder_id, resolve_scope_ids};
use crate::api::state::AppState;
use crate::api::util::json_uuid;
use crate::db::pg_store::CallDirection;
use axum::{extract::State, http::StatusCode, response::Json};

// ── MCP Tool Proxy ──────────────────────────────────────────────────────────

/// List every MCP tool this daemon dispatches on, with the shape the
/// Instruments playground consumes: `kind`, `summary`, structured `inputs[]`,
/// and an `example` response.
///
/// Sourced from `mcp_manifests::manifests()` — kept in that module so the
/// listing stays in lockstep with `mcp_call_tool` (guarded by a unit test).
pub(crate) async fn mcp_list_tools() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "tools": super::mcp_manifests::manifests() }))
}

/// Wrap a caller/callee list in an envelope that answers "does this symbol even
/// exist" and "is this list complete" alongside the list itself.
///
/// A bare `{"callers": []}` is three different answers wearing one shape, and an
/// assistant must act differently on each: the symbol is absent from the graph
/// (recheck the name/scope, or grep); the symbol exists and truly has no callers
/// (safe to delete); or the symbol exists and the graph holds `calls` edges it
/// could not resolve, so the list is INCOMPLETE and a grep is still owed. Only
/// `symbol.found` plus `coverage.unresolved` can distinguish them — a list
/// length cannot, which is why an empty list alone was never a usable answer.
///
/// `symbol.found = false` is a genuine not-found, never a masked failure: a DB
/// error on either read propagates as a 500 instead of degrading to "not found".
async fn symbol_relation_envelope(
    state: &AppState,
    folder_ids: &[uuid::Uuid],
    name: &str,
    list_key: &str,
    list: Vec<serde_json::Value>,
) -> Result<serde_json::Value, StatusCode> {
    let direction =
        if list_key == "callers" { CallDirection::Incoming } else { CallDirection::Outgoing };

    let definitions = state.pg.symbol_definitions(folder_ids, name).await.map_err(|e| {
        tracing::warn!(error = %e, name, "mcp symbol_relation_envelope: symbol_definitions failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let (resolved, unresolved) =
        state.pg.call_coverage(folder_ids, name, direction).await.map_err(|e| {
            tracing::warn!(error = %e, name, "mcp symbol_relation_envelope: call_coverage failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(serde_json::json!({
        "symbol": {
            "name":        name,
            "found":       !definitions.is_empty(),
            "defined_at":  definitions,
        },
        list_key: list,
        "coverage": {
            "resolved":   resolved,
            "unresolved": unresolved,
            // Spelled out so a reader does not have to infer the rule from two
            // integers: `complete` means every recorded edge was placed.
            "complete":   unresolved == 0,
        },
    }))
}

pub(crate) async fn mcp_call_tool(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tool = body["tool"].as_str().unwrap_or("");
    let params = &body["params"];
    let repo_id = params["repoId"].as_str().unwrap_or("");
    let query =
        params["query"].as_str().or(params["q"].as_str()).or(params["name"].as_str()).unwrap_or("");

    let result = match tool {
        "search" => {
            // Delegate to the hybrid query path (lexical ILIKE + semantic
            // embedding NN, fused by RRF) so the MCP `search` tool ranks by BOTH
            // keyword AND concept relevance — not substring only. Fail-open: with
            // no query embedding it degrades to the lexical order. Previously this
            // arm was lexical-only, so a concept query that didn't share a
            // substring with any symbol name returned nothing (G4).
            let general = super::query::query_general(&state, query, repo_id).await?;
            serde_json::json!({
                "functions": general.get("functions").cloned().unwrap_or_else(|| serde_json::json!([])),
                "types":     general.get("types").cloned().unwrap_or_else(|| serde_json::json!([])),
            })
        }
        "context_pack" => {
            // Concept-level retrieval in one call: top hybrid hits + their code
            // snippets, so an assistant doesn't need a search then N file reads.
            super::query::context_pack(&state, query, repo_id).await?
        }
        "get_symbol" => {
            let ids = resolve_scope_ids(&state, repo_id).await?;
            let fns = if !ids.is_empty() {
                state.pg.search_functions_scoped(&ids, query).await.map_err(|e| { tracing::warn!(error = %e, tool, query, "mcp get_symbol: search_functions_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?
            } else {
                vec![]
            };
            serde_json::json!({"results": fns})
        }
        // `get_callers` / `get_callees` report WHETHER THE SYMBOL EXISTS
        // alongside the list, because a bare `[]` conflates three different
        // answers an assistant must act on differently: the symbol is not in
        // the graph (check the spelling, or the scope, or fall back to grep);
        // the symbol exists and nothing calls it (safe to delete); the symbol
        // exists and the graph holds calls it could not resolve (the list is
        // INCOMPLETE — grep before concluding anything). `coverage.unresolved`
        // is the only honest way to say that third case; a list length cannot.
        "get_callers" => {
            let name = params["name"].as_str().unwrap_or(query);
            let ids = resolve_scope_ids(&state, repo_id).await?;
            let callers = state.pg.get_callers_by_name(repo_id, name).await.map_err(|e| { tracing::warn!(error = %e, repo_id, name, "mcp get_callers: get_callers_by_name failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
            symbol_relation_envelope(&state, &ids, name, "callers", callers).await?
        }
        "get_callees" => {
            let name = params["name"].as_str().unwrap_or(query);
            let ids = resolve_scope_ids(&state, repo_id).await?;
            let callees = state.pg.get_callees_by_name(repo_id, name).await.map_err(|e| { tracing::warn!(error = %e, repo_id, name, "mcp get_callees: get_callees_by_name failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
            symbol_relation_envelope(&state, &ids, name, "callees", callees).await?
        }
        "get_file_tags" => {
            let tag = params["tag"].as_str().unwrap_or(query);
            let files = state.pg.get_files_by_tag(repo_id, tag).await.map_err(|e| { tracing::warn!(error = %e, repo_id, tag, "mcp get_file_tags: get_files_by_tag failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
            serde_json::json!({"files": files})
        }
        "get_communities" => {
            // Communities are stored per-folder; aggregate across ALL scope folders
            // (#G5a — the single-folder resolve_folder_id missed the repo root's).
            let ids = resolve_scope_ids(&state, repo_id).await?;
            let communities = state.pg.list_communities_scoped(&ids).await.map_err(|e| { tracing::warn!(error = %e, repo_id, "mcp get_communities: list_communities_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
            serde_json::json!({"communities": communities})
        }
        "get_doc_drift" => {
            let drift = state.pg.get_doc_drift(repo_id).await.map_err(|e| {
                tracing::warn!(error = %e, repo_id, "mcp get_doc_drift: get_doc_drift failed");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            serde_json::json!({"drift": drift})
        }
        "search_lib_docs" => {
            let results = state.pg.search_library_pages(query).await.map_err(|e| { tracing::warn!(error = %e, query, "mcp search_lib_docs: search_library_pages failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
            serde_json::json!({ "query": query, "results": results })
        }
        "get_lib_docs" => {
            let name = params["name"].as_str().filter(|s| !s.is_empty()).unwrap_or(query);
            let component = params["component"].as_str().filter(|s| !s.is_empty());
            let pages = state.pg.get_library_pages(name, component).await.map_err(|e| {
                tracing::warn!(error = %e, name, "mcp get_lib_docs: get_library_pages failed");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            if pages.is_empty() {
                serde_json::json!({
                    "library": name,
                    "component": component,
                    "error": format!(
                        "No docs indexed for '{}'{}. Index it with add_library, or check the name/component.",
                        name,
                        component.map(|c| format!(" (component '{}')", c)).unwrap_or_default(),
                    ),
                })
            } else if component.is_some() {
                // Specific component → return its page content.
                serde_json::json!({ "library": name, "component": component, "pages": pages })
            } else {
                // No component → the overview (null-component pages) + the list of
                // available components so the caller can drill in.
                let overview: Vec<_> =
                    pages.iter().filter(|p| p["component"].is_null()).cloned().collect();
                let components: Vec<_> = pages
                    .iter()
                    .filter_map(|p| p["component"].as_str().map(str::to_string))
                    .collect();
                serde_json::json!({ "library": name, "overview": overview, "components": components })
            }
        }
        "list_projects" => {
            let repos = state.pg.list_repositories().await.map_err(|e| {
                tracing::warn!(error = %e, "mcp list_projects: list_repositories failed");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            serde_json::json!({"projects": repos})
        }
        "create_session" => {
            let repo_id_str = params["repoId"].as_str().unwrap_or(query);
            let task = params["task"].as_str().unwrap_or("untitled");
            // Look up folder UUID from repo name
            let folder = state.pg.get_repo_by_name(repo_id_str).await.map_err(|e| { tracing::warn!(error = %e, repo_id = repo_id_str, "mcp create_session: get_repo_by_name failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
            if let Some(folder) = folder {
                if let Some(folder_id) = json_uuid(&folder["id"]) {
                    match state.pg.create_session(&folder_id, task, None).await {
                        Ok(session_id) => {
                            serde_json::json!({"ok": true, "sessionId": session_id.to_string()})
                        }
                        Err(e) => serde_json::json!({"error": e}),
                    }
                } else {
                    serde_json::json!({"error": "invalid folder id"})
                }
            } else {
                serde_json::json!({"error": "repo not found"})
            }
        }
        "update_session" => {
            let session_id_str = params["sessionId"].as_str().unwrap_or("");
            if let Ok(session_id) = uuid::Uuid::parse_str(session_id_str) {
                let outcome = params["outcome"].as_str().unwrap_or("completed");
                let ftr = outcome == "completed";
                let turns = params["turns"].as_i64().unwrap_or(0) as i32;
                let corrections = params["corrections"].as_i64().unwrap_or(0) as i32;
                // A failed write is a 500, not a fabricated `{"ok": true}`. summary +
                // tokensIn/tokensOut are persisted (were previously advertised but dropped).
                let summary = params["summary"].as_str().filter(|s| !s.is_empty());
                let tokens_in =
                    params["tokensIn"].as_str().and_then(|s| s.trim().parse::<i32>().ok());
                let tokens_out =
                    params["tokensOut"].as_str().and_then(|s| s.trim().parse::<i32>().ok());
                state.pg.complete_session(
                    &session_id,
                    outcome,
                    ftr,
                    turns,
                    corrections,
                    summary,
                    tokens_in,
                    tokens_out,
                ).await.map_err(|e| { tracing::warn!(error = %e, %session_id, outcome, "mcp update_session: complete_session failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
                serde_json::json!({"ok": true})
            } else {
                serde_json::json!({"error": "invalid sessionId"})
            }
        }
        "add_library" => {
            let name = params["name"].as_str().unwrap_or("");
            let explicit_url = params["url"].as_str().unwrap_or("");
            let version = params["version"].as_str();
            if name.is_empty() {
                serde_json::json!({"error": "name required"})
            } else {
                // Resolve the ingestion target. An explicit `url` may be a local
                // directory path, a github.com tree URL, or a website llms URL —
                // classify it and store the matching source_type. If no url is
                // given, fall back to the auto-discovery probes.
                let (target, source_type): (Option<String>, &str) = if !explicit_url.is_empty() {
                    use crate::indexer::lib_indexer::{LibSource, detect_lib_source};
                    let st = match detect_lib_source(explicit_url) {
                        LibSource::LocalDir(_) => "local",
                        LibSource::GitHubTree { .. } => "http",
                        LibSource::Website(_) => "llms.txt",
                    };
                    (Some(explicit_url.to_string()), st)
                } else {
                    (discover_lib_url(name, "").await, "llms.txt")
                };

                match target {
                    Some(url) => {
                        // Upsert the library record with the resolved source_type.
                        match state
                            .pg
                            .upsert_library(
                                name,
                                "npm",
                                version,
                                None,
                                Some(source_type),
                                Some(&url),
                            )
                            .await
                        {
                            Ok(lib_id) => {
                                // Enqueue IndexLibrary task for async ingestion.
                                let task = crate::tasks::Task::new(
                                    crate::tasks::TaskKind::IndexLibrary,
                                    &lib_id.to_string(),
                                    name,
                                )
                                .with_url(&url);
                                let task_id = state.task_queue.enqueue(task).await;

                                serde_json::json!({
                                    "ok": true,
                                    "libName": name,
                                    "libId": lib_id.to_string(),
                                    "taskId": task_id,
                                    "url": url,
                                    "sourceType": source_type,
                                    "status": "indexing",
                                })
                            }
                            Err(e) => {
                                serde_json::json!({"error": format!("Failed to create library: {}", e)})
                            }
                        }
                    }
                    None => {
                        let clean = name.trim_start_matches('@').replace('/', "-");
                        serde_json::json!({
                            "error": format!(
                                "Could not find docs for '{}'. Tried common patterns ({}.com, .dev, .io, GitHub). Provide an explicit url (local dir, github tree URL, or website llms URL).",
                                name, clean
                            ),
                        })
                    }
                }
            }
        }
        "query" => {
            // Reuse unified query logic — delegate to POST /api/query handler
            serde_json::json!({"hint": "Use POST /api/query directly"})
        }
        "get_project_summary" => {
            let ids = resolve_scope_ids(&state, repo_id).await?;
            let (fns, types) = if !ids.is_empty() {
                let counts = state.pg.count_nodes_by_kind_scoped(&ids).await.map_err(|e| { tracing::warn!(error = %e, repo_id, "mcp get_project_summary: count_nodes_by_kind_scoped failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
                let f = counts.get("function").copied().unwrap_or(0)
                    + counts.get("method").copied().unwrap_or(0);
                let t = counts.get("class").copied().unwrap_or(0)
                    + counts.get("struct").copied().unwrap_or(0)
                    + counts.get("interface").copied().unwrap_or(0);
                (f, t)
            } else {
                (0, 0)
            };
            // Prefer project row for name/metadata; fall back to folder row.
            let project = match state.pg.get_project_by_name(repo_id).await
                .map_err(|e| { tracing::warn!(error = %e, repo_id, "mcp get_project_summary: get_project_by_name failed"); StatusCode::INTERNAL_SERVER_ERROR })?
            {
                Some(proj) => Some(proj),
                None => state.pg.get_repo_by_name(repo_id).await
                    .map_err(|e| { tracing::warn!(error = %e, repo_id, "mcp get_project_summary: get_repo_by_name fallback failed"); StatusCode::INTERNAL_SERVER_ERROR })?,
            };
            // How far to trust the graph tools, stated rather than inferred. A
            // symbol count says nothing about whether `get_callers` can answer:
            // an edge kind at 0% resolved means every lookup over it comes back
            // empty for reasons that have nothing to do with the code.
            let graph_health = if ids.is_empty() {
                serde_json::json!([])
            } else {
                let by_kind = state.pg.edge_resolution_by_kind(&ids).await.map_err(|e| { tracing::warn!(error = %e, repo_id, "mcp get_project_summary: edge_resolution_by_kind failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
                serde_json::Value::Array(
                    by_kind
                        .into_iter()
                        .map(|(kind, resolved, total)| {
                            serde_json::json!({
                                "kind":     kind,
                                "edges":    total,
                                "resolved": resolved,
                                // Integer percent — enough to decide "trust it"
                                // vs "grep first", without implying precision.
                                "resolved_pct": if total > 0 { resolved * 100 / total } else { 0 },
                            })
                        })
                        .collect(),
                )
            };
            serde_json::json!({
                "project": project,
                "functions": fns,
                "types": types,
                "graphHealth": graph_health,
                // What each language CAN do, derived from the adapter impls. Pairs
                // with graphHealth: that says how much of the graph resolved, this
                // says which languages are even capable of resolving. A repo that
                // is mostly Kotlin and a Kotlin adapter with no FQN support explain
                // a disappointing number far better than the number alone.
                "languageCapabilities": crate::languages::capability_matrix(),
            })
        }
        "get_metrics" => {
            let folder = state.pg.get_repo_by_name(repo_id).await.map_err(|e| {
                tracing::warn!(error = %e, repo_id, "mcp get_metrics: get_repo_by_name failed");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            if let Some(folder) = folder {
                if let Some(folder_id) = json_uuid(&folder["id"]) {
                    let sessions = state.pg.list_sessions_by_folder(&folder_id, 100).await.map_err(|e| { tracing::warn!(error = %e, %folder_id, "mcp get_metrics: list_sessions_by_folder failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
                    let session_count = sessions.len();
                    let completed = sessions
                        .iter()
                        .filter(|s| s["outcome"].as_str() == Some("completed"))
                        .count();
                    // FTR is store-backed (project_metrics, metric='ftr') — the SAME
                    // number the Phase-7 endpoints serve. Honest-absent (null) when
                    // the folder has no project or no ftr rows; NEVER a fabricated 0.
                    let ftr: Option<f64> = match json_uuid(&folder["project_id"]) {
                        Some(pid) => state.pg.get_project_ftr_rate(&pid).await.map_err(|e| { tracing::warn!(error = %e, repo_id, "mcp get_metrics: get_project_ftr_rate failed"); StatusCode::INTERNAL_SERVER_ERROR })?,
                        None => None,
                    };
                    serde_json::json!({
                        "project": repo_id,
                        "sessions": session_count,
                        "completed": completed,
                        "ftr": ftr,
                    })
                } else {
                    serde_json::json!({"error": "invalid folder id"})
                }
            } else {
                serde_json::json!({"error": "project not found"})
            }
        }
        "get_ftr_daily" => {
            let days = params["days"].as_i64().unwrap_or(14) as i32;
            let project_id = resolve_folder_id(&state, repo_id).await?;
            let data = state.pg.get_ftr_daily(project_id.as_ref(), days).await.map_err(|e| { tracing::warn!(error = %e, repo_id, days, "mcp get_ftr_daily: get_ftr_daily failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
            serde_json::json!({"ftr_daily": data})
        }
        "get_hotspots" => {
            let days = params["days"].as_i64().unwrap_or(7) as i32;
            if let Some(fid) = resolve_folder_id(&state, repo_id).await? {
                let data = state.pg.get_hotspots(&fid, days).await.map_err(|e| { tracing::warn!(error = %e, repo_id, days, "mcp get_hotspots: get_hotspots failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
                serde_json::json!({"hotspots": data})
            } else {
                serde_json::json!({"hotspots": []})
            }
        }
        "get_quality_signals" => {
            if let Some(fid) = resolve_folder_id(&state, repo_id).await? {
                state.pg.get_quality_signals(&fid).await.map_err(|e| { tracing::warn!(error = %e, repo_id, "mcp get_quality_signals: get_quality_signals failed"); StatusCode::INTERNAL_SERVER_ERROR })?
            } else {
                serde_json::json!({"error": "project not found"})
            }
        }
        _ => serde_json::json!({"error": format!("Unknown tool: {}", tool)}),
    };

    Ok(Json(result))
}

/// Probe common URL patterns to find library documentation.
/// Returns the first URL that responds with content > 50 bytes.
async fn discover_lib_url(name: &str, explicit_url: &str) -> Option<String> {
    let urls: Vec<String> = if !explicit_url.is_empty() {
        vec![explicit_url.to_string()]
    } else {
        let clean = name.trim_start_matches('@').replace('/', "-");
        vec![
            format!("https://{}.com/llms.txt", clean),
            format!("https://{}.dev/llms.txt", clean),
            format!("https://{}.com/llms-full.txt", clean),
            format!("https://{}.io/llms.txt", clean),
            format!("https://www.{}.com/llms.txt", clean),
            format!("https://raw.githubusercontent.com/{name}/main/llms.txt"),
            format!("https://raw.githubusercontent.com/{name}/master/README.md"),
        ]
    };

    let timeout = if explicit_url.is_empty() { 5 } else { 15 };

    for url in &urls {
        if let Ok(content) =
            crate::indexer::lib_indexer::fetch_lib_url_with_timeout(url, timeout).await
            && content.len() > 50
        {
            return Some(url.clone());
        }
    }

    None
}
