//! `/api/knowledge/*` — memory CRUD, proposals, outcomes, context assembly.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;
use crate::db::pg_store::{InsertMemory, OutcomeRow};

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": msg })))
}

/// Resolve a governance namespace from either an explicit `namespace_id` or a
/// `(gov_scope, folder)` pair against the repo's namespace memberships. Shared
/// by rule authoring and promotion so the resolution rule lives in one place.
async fn resolve_target_namespace(
    state: &AppState,
    namespace_id: Option<&str>,
    gov_scope: Option<&str>,
    folder: Option<&str>,
) -> Result<Option<uuid::Uuid>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ns) = namespace_id.filter(|s| !s.is_empty()) {
        return Ok(Some(uuid::Uuid::parse_str(ns).map_err(|_| err(StatusCode::BAD_REQUEST, "bad namespace_id"))?));
    }
    if let (Some(scope), Some(folder)) =
        (gov_scope.filter(|s| !s.is_empty()), folder.filter(|s| !s.is_empty()))
    {
        let fid = state.pg.get_repo_by_path(folder).await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
            .and_then(|f| crate::api::util::json_uuid(&f["id"]));
        return match fid {
            Some(fid) => state.pg.namespace_for_folder_scope(&fid, scope).await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e)),
            None => Ok(None), // unknown repo → unscoped (general)
        };
    }
    Ok(None)
}

// ============================================================================
// GET /api/knowledge/memories?status=&scope=&project_id=&limit=
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct ListQuery {
    pub status:     Option<String>,
    pub scope:      Option<String>,
    pub project_id: Option<String>,
    pub limit:      Option<i64>,
}

pub(crate) async fn list_memories(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pid = match q.project_id {
        Some(s) => Some(uuid::Uuid::parse_str(&s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?),
        None => None,
    };
    let rows = state.pg.list_memories(pid, q.status.as_deref(), q.scope.as_deref(), q.limit.unwrap_or(200))
        .await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "memories": rows })))
}

// ============================================================================
// GET /api/knowledge/memories/:id
// ============================================================================

pub(crate) async fn get_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let detail = state.pg.get_memory_detail(mid).await
        .map_err(|e| {
            if e.contains("not found") { err(StatusCode::NOT_FOUND, "memory not found") }
            else { err(StatusCode::INTERNAL_SERVER_ERROR, &e) }
        })?;
    Ok(Json(detail))
}

// ============================================================================
// GET /api/knowledge/context?project_id=&limit=&tags=csv
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct ContextQuery {
    pub project_id: String,
    pub limit:      Option<i64>,
    pub tags:       Option<String>,
}

pub(crate) async fn get_context(
    State(state): State<AppState>,
    Query(q): Query<ContextQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pid = uuid::Uuid::parse_str(&q.project_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?;
    let tags: Option<Vec<String>> = q.tags.map(|s|
        s.split(',').filter(|t| !t.trim().is_empty()).map(|t| t.trim().to_string()).collect()
    );
    let stack_ids = state.pg.get_project_stack_ids(&pid).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let blob = state.pg.assemble_context(pid, &stack_ids, tags.as_deref(), q.limit.unwrap_or(200))
        .await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(blob))
}

// ============================================================================
// GET /api/knowledge/rules?folder=<abs_path>  — governance Tier-1 resolution
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct RulesQuery {
    /// Absolute path of the repo whose governing rules to resolve.
    pub folder: String,
}

/// Resolve the rules governing a repo: gather its namespace memberships + the
/// always-on general/user scopes, ordered by enforcement then scope level,
/// deduped, with the mandatory (non-overridable) ones flagged.
pub(crate) async fn get_rules(
    State(state): State<AppState>,
    Query(q): Query<RulesQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let folder = state.pg.get_repo_by_path(&q.folder).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "folder not indexed"))?;
    let folder_id = crate::api::util::json_uuid(&folder["id"])
        .ok_or_else(|| err(StatusCode::INTERNAL_SERVER_ERROR, "folder has no id"))?;
    let raw = state.pg.resolve_rules_raw(&folder_id).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let ruleset = crate::governance::structure_ruleset(raw);
    Ok(Json(serde_json::json!({
        "folder": q.folder,
        "total": ruleset.total,
        "mandatory_count": ruleset.mandatory_count,
        "rules": ruleset.rules,
    })))
}

/// Resolve the global ruleset (user + general scope) and write it to
/// `<dir>/rules.md`, returning the path and rule count. `dir` is injected so the
/// daemon passes `~/.sensei` and tests can pass a temp dir.
pub(crate) async fn materialize_global_rules(
    pg: &crate::db::pg_store::PgStore,
    dir: &std::path::Path,
) -> Result<(std::path::PathBuf, usize), String> {
    let ruleset = crate::governance::structure_ruleset(pg.resolve_global_rules().await?);
    // Prefer an approved Tier-2 (LLM-merged) ruleset; fall back to the Tier-1 render.
    let md = match pg.get_consolidated_ruleset("global", Some("approved")).await? {
        Some(row) => crate::governance::wrap_managed(row["content"].as_str().unwrap_or_default()),
        None => crate::governance::render_rules_md(&ruleset),
    };
    std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let path = dir.join("rules.md");
    std::fs::write(&path, md).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok((path, ruleset.total))
}

/// POST /api/knowledge/rules/materialize — regenerate the global ~/.sensei/rules.md now.
pub(crate) async fn materialize_rules(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let dir = crate::paths::sensei_dir();
    let (path, count) = materialize_global_rules(&state.pg, &dir).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "path": path.display().to_string(), "rules": count })))
}

// ============================================================================
// POST /api/knowledge/proposals  — propose_memory
// POST /api/knowledge/memories   — save_memory (explicit)
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct MemoryBody {
    pub project_id:    Option<String>,
    pub scope:         String,
    pub scope_filter:  Option<String>,
    #[serde(rename = "type")]
    pub mtype:         String,
    pub title:         String,
    pub content:       String,
    pub impact:        Option<String>,
    #[serde(default)]
    pub tags:          Vec<String>,
    pub triage_signal: Option<String>,
    // ── Governance plane (optional) ──────────────────────────────────────
    /// Explicit namespace this rule applies to (wins over gov_scope/folder).
    pub namespace_id:  Option<String>,
    /// Governance scope key (general/user/organization/client/technology/team/
    /// project/repository); resolved against `folder`'s namespace memberships.
    pub gov_scope:     Option<String>,
    /// Repo abs_path used to resolve `gov_scope` to a namespace.
    pub folder:        Option<String>,
    /// Authority: advisory|recommended|required|mandatory (default recommended).
    pub enforcement:   Option<String>,
}

async fn insert_with_status(
    state: AppState,
    body:  MemoryBody,
    status: &str,
    require_triage_signal: bool,
    origin: &str,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if body.title.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "title must not be empty"));
    }
    if body.content.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "content must not be empty"));
    }
    if body.scope == "stack" && body.scope_filter.as_deref().unwrap_or("").is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "scope_filter required for scope='stack'"));
    }
    if require_triage_signal && body.triage_signal.as_deref().unwrap_or("").is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "triage_signal required for proposals"));
    }
    let pid = match &body.project_id {
        Some(s) => Some(uuid::Uuid::parse_str(s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?),
        None => None,
    };
    // Governance namespace: explicit namespace_id wins; else resolve gov_scope
    // against the repo's namespace memberships.
    let namespace_id = resolve_target_namespace(
        &state, body.namespace_id.as_deref(), body.gov_scope.as_deref(), body.folder.as_deref(),
    ).await?;
    let id = state.pg.insert_memory(&InsertMemory {
        project_id:    pid,
        scope:         body.scope,
        scope_filter:  body.scope_filter,
        mtype:         body.mtype,
        title:         body.title,
        content:       body.content,
        impact:        body.impact,
        tags:          body.tags,
        triage_signal: body.triage_signal,
        status:        status.into(),
        namespace_id,
        enforcement:   body.enforcement.filter(|s| !s.is_empty()),
        origin:        Some(origin.to_string()),
        source_id:     None,
    }).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "id": id, "status": status })))
}

pub(crate) async fn propose_memory(
    State(state): State<AppState>,
    Json(body):   Json<MemoryBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    insert_with_status(state, body, "proposed", true, "learned").await
}

pub(crate) async fn save_memory(
    State(state): State<AppState>,
    Json(body):   Json<MemoryBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    insert_with_status(state, body, "active", false, "authored").await
}

// ============================================================================
// POST /api/knowledge/memories/{id}/promote  — elevate to a broader scope
// GET  /api/knowledge/promotion-candidates    — battle_tested, not yet promoted
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct PromoteBody {
    pub namespace_id: Option<String>,
    pub gov_scope:    Option<String>,
    pub folder:       Option<String>,
    pub enforcement:  Option<String>,
}

/// Promote a proven memory to a broader scope. Creates a `proposed` copy on the
/// target namespace (origin=promoted, source_id=original); accepting it through
/// the normal proposal flow is the approval gate, so it never auto-applies.
pub(crate) async fn promote_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PromoteBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let sid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad memory id"))?;
    let target = resolve_target_namespace(
        &state, body.namespace_id.as_deref(), body.gov_scope.as_deref(), body.folder.as_deref(),
    ).await?;
    let new_id = state.pg.promote_memory(sid, target, body.enforcement.as_deref().filter(|s| !s.is_empty())).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::CONFLICT,
            "memory not found or not promotable (must be active/reinforced/battle_tested)"))?;
    Ok(Json(serde_json::json!({ "id": new_id, "status": "proposed", "origin": "promoted" })))
}

/// List battle_tested memories that have not yet been promoted — the candidates
/// a UI surfaces for "elevate to a broader scope".
pub(crate) async fn promotion_candidates(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let rows = state.pg.list_promotion_candidates().await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "candidates": rows })))
}

// ============================================================================
// Governance Tier-2 — LLM consolidation merge of the global ruleset
// POST /api/knowledge/rules/consolidate            — run the merge → proposed
// GET  /api/knowledge/rules/consolidated           — current merged ruleset
// POST /api/knowledge/rules/consolidate/{id}/approve — approve → feeds rules.md
// ============================================================================

/// Run the Tier-2 consolidation for the global ruleset: gather the Tier-1
/// rules, ask the chat model (gemma4) to merge them into one coherent markdown
/// ruleset, and store it as a new `proposed` version. Skips when there are no
/// rules or the input is unchanged since the last consolidation.
pub(crate) async fn consolidate_rules(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    use gateway::types::capability::Capability;
    use gateway::types::request::*;

    let set = crate::governance::structure_ruleset(
        state.pg.resolve_global_rules().await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?,
    );
    if set.total == 0 {
        return Ok(Json(serde_json::json!({ "skipped": true, "reason": "no global rules to merge" })));
    }
    let hash = crate::governance::ruleset_source_hash(&set);
    if state.pg.latest_ruleset_source_hash("global").await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))? == Some(hash.clone())
    {
        return Ok(Json(serde_json::json!({ "skipped": true, "reason": "unchanged since last consolidation" })));
    }

    let prompt = crate::governance::build_merge_prompt(&set);
    let request = InferenceRequest {
        capability: Capability::TextChat,
        model: None, router: None, chain: None,
        payload: Payload::Chat {
            messages: vec![Message::text(MessageRole::User, prompt)],
            system: Some("You merge governance rules into a single clean markdown ruleset.".to_string()),
            max_tokens: Some(1500),
            temperature: None,
            tools: Vec::new(),
        },
        budget: None,
    };
    let resp = state.gateway.execute(&request).await
        .map_err(|e| err(StatusCode::BAD_GATEWAY, &format!("merge model call failed: {e}")))?;
    let content = match (resp.success, resp.content) {
        (true, Some(c)) if !c.trim().is_empty() => c,
        _ => return Err(err(StatusCode::BAD_GATEWAY, "merge model returned no content")),
    };

    let version = state.pg.next_ruleset_version("global").await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let id = state.pg.insert_consolidated_ruleset(
        "global", version, &content, &serde_json::json!([]), resp.model.as_deref(), &hash, "proposed",
    ).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    Ok(Json(serde_json::json!({
        "id": id, "version": version, "status": "proposed", "model": resp.model, "content": content,
    })))
}

/// Fetch the global consolidated ruleset — the approved one if present, else the
/// latest proposed.
pub(crate) async fn get_consolidated(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let approved = state.pg.get_consolidated_ruleset("global", Some("approved")).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let row = match approved {
        Some(r) => Some(r),
        None => state.pg.get_consolidated_ruleset("global", None).await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?,
    };
    Ok(Json(row.unwrap_or(serde_json::Value::Null)))
}

/// Approve a consolidated ruleset version (the approval gate). Supersedes the
/// prior approved version and re-materializes ~/.sensei/rules.md from the
/// approved merge.
pub(crate) async fn approve_consolidated(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let rid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad ruleset id"))?;
    let (scope, _content) = state.pg.approve_consolidated_ruleset(&rid).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "ruleset not found"))?;
    // The approved global merge now feeds ~/.sensei/rules.md.
    if scope == "global" {
        materialize_global_rules(&state.pg, &crate::paths::sensei_dir()).await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    }
    Ok(Json(serde_json::json!({ "id": rid, "status": "approved", "scope": scope })))
}

// ============================================================================
// POST /api/knowledge/proposals/:id/accept
// POST /api/knowledge/proposals/:id/reject
// ============================================================================

pub(crate) async fn accept_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let new_status = state.pg.set_memory_status(mid, "active", &["proposed"]).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    match new_status {
        Some(s) => {
            // Federation: if this was a promoted rule at a shareable scope, push it.
            // Fire-and-forget — federation must not block or fail the approval.
            let pg = state.pg.clone();
            tokio::spawn(async move { crate::federation::push_promoted(&pg, mid).await; });
            Ok(Json(serde_json::json!({ "id": mid, "status": s })))
        }
        None => Err(err(StatusCode::CONFLICT, "proposal not in 'proposed' state")),
    }
}

#[derive(Deserialize)]
pub(crate) struct RejectBody {
    pub reason: Option<String>,
}

pub(crate) async fn reject_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<RejectBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    if let Some(reason) = body.reason.as_deref().filter(|s| !s.trim().is_empty()) {
        tracing::info!(memory_id = %mid, reason, "proposal rejected");
    }
    let new_status = state.pg.set_memory_status(mid, "rejected", &["proposed"]).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    match new_status {
        Some(s) => Ok(Json(serde_json::json!({ "id": mid, "status": s }))),
        None => Err(err(StatusCode::CONFLICT, "proposal not in 'proposed' state")),
    }
}

// ============================================================================
// POST /api/knowledge/outcomes
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct OutcomeBody {
    pub memory_id:  String,
    pub outcome:    String,
    pub session_id: Option<String>,
    pub context:    Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct OutcomesBatch {
    pub outcomes: Vec<OutcomeBody>,
}

pub(crate) async fn record_outcomes(
    State(state): State<AppState>,
    Json(body):   Json<OutcomesBatch>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let valid_outcomes = ["applied", "consulted", "violated", "ignored"];
    let mut rows: Vec<OutcomeRow> = Vec::with_capacity(body.outcomes.len());
    for o in body.outcomes {
        if !valid_outcomes.contains(&o.outcome.as_str()) {
            return Err(err(StatusCode::BAD_REQUEST, &format!("invalid outcome: {}", o.outcome)));
        }
        let mid = uuid::Uuid::parse_str(&o.memory_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "bad memory_id"))?;
        let sess = match o.session_id {
            Some(s) => Some(uuid::Uuid::parse_str(&s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad session_id"))?),
            None => None,
        };
        rows.push(OutcomeRow {
            memory_id: mid, session_id: sess, outcome: o.outcome, context: o.context,
        });
    }
    let total = rows.len();
    let skipped = state.pg.record_outcomes_batch(&rows).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({
        "recorded": total - skipped.len(),
        "skipped":  skipped,
    })))
}

// ============================================================================
// Federation sources — /api/knowledge/sources*
// GET    /api/knowledge/sources             — list registered hive-mind sources
// POST   /api/knowledge/sources             — register a source (+ Keychain cred)
// DELETE /api/knowledge/sources/{id}         — deregister (+ purge Keychain cred)
// POST   /api/knowledge/sources/{id}/sync    — pull this source now
// GET    /api/knowledge/sources/{id}/status  — current cursor / enabled state
// ============================================================================

#[derive(serde::Deserialize)]
pub(crate) struct NewSourceBody {
    pub kind: Option<String>,
    pub name: String,
    pub url: String,
    pub namespace_id: Option<String>,
    pub direction: Option<String>,
    pub api_key: String,
}

pub(crate) async fn create_source(
    State(state): State<AppState>,
    Json(b): Json<NewSourceBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let url = b.url.trim().to_string();
    if !url.starts_with("https://") && !url.contains("://127.0.0.1") && !url.contains("://localhost") {
        return Err(err(StatusCode::BAD_REQUEST, "non-loopback source url must be https"));
    }
    let namespace_id = match b.namespace_id.as_deref() {
        Some(s) => Some(uuid::Uuid::parse_str(s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad namespace_id"))?),
        None => None,
    };
    let credential_ref = format!("hive-{}", uuid::Uuid::new_v4());
    let cref = credential_ref.clone();
    let api_key = b.api_key.clone();
    tokio::task::spawn_blocking(move || crate::gateway_keys::set_key(&cref, &api_key))
        .await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    let id = state.pg.create_knowledge_source(&crate::db::pg_store::NewKnowledgeSource {
        kind: b.kind.unwrap_or_else(|| "hive_mind".into()),
        name: b.name,
        url,
        namespace_id,
        credential_ref,
        direction: b.direction.unwrap_or_else(|| "both".into()),
    }).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "id": id.to_string() })))
}

pub(crate) async fn list_sources(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let rows = state.pg.list_knowledge_sources().await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let out: Vec<_> = rows.into_iter().map(|s| serde_json::json!({
        "id": s.id, "kind": s.kind, "name": s.name, "url": s.url,
        "namespace_id": s.namespace_id, "direction": s.direction,
        "last_seq": s.last_seq, "enabled": s.enabled })).collect();
    Ok(Json(serde_json::json!({ "sources": out })))
}

pub(crate) async fn delete_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let sid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    if let Ok(Some(s)) = state.pg.get_knowledge_source(&sid).await {
        let cref = s.credential_ref.clone();
        let _ = tokio::task::spawn_blocking(move || crate::gateway_keys::delete_key(&cref)).await;
    }
    let removed = state.pg.delete_knowledge_source(&sid).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    if removed { Ok(Json(serde_json::json!({ "deleted": true }))) } else { Err(err(StatusCode::NOT_FOUND, "no such source")) }
}

pub(crate) async fn sync_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let sid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let src = state.pg.get_knowledge_source(&sid).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such source"))?;
    let client = reqwest::Client::new();
    let stats = crate::federation::pull_source(&state.pg, &client, &src).await
        .map_err(|e| err(StatusCode::BAD_GATEWAY, &e))?;
    Ok(Json(serde_json::to_value(stats).unwrap()))
}

pub(crate) async fn source_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let sid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let src = state.pg.get_knowledge_source(&sid).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such source"))?;
    Ok(Json(serde_json::json!({ "id": src.id, "name": src.name, "url": src.url,
        "direction": src.direction, "last_seq": src.last_seq, "enabled": src.enabled })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn materialize_writes_managed_global_rules_file() {
        let pg = crate::db::pg_store::PgStore::connect_test().await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        // Writes to the injected temp dir, never the real ~/.sensei.
        let (path, _count) = materialize_global_rules(&pg, tmp.path()).await.unwrap();
        assert_eq!(path, tmp.path().join("rules.md"));
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("Managed by sensei"), "managed header present");
        assert!(body.contains("# Sensei Rules"), "title present");
    }
}
