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

/// Managed-block marker used to bracket the sensei pointer inside the user's
/// global `~/.claude/CLAUDE.md`. Any content between the two lines is owned by
/// the daemon and safe to rewrite on every startup; content outside is
/// user-authored and preserved verbatim.
const CLAUDE_MD_BEGIN: &str = "<!-- sensei:global-rules-pointer BEGIN -->";
const CLAUDE_MD_END:   &str = "<!-- sensei:global-rules-pointer END -->";

/// The one-line pointer body itself. Kept short so it costs almost nothing on
/// every message. References the file the daemon just materialized so any ACP
/// (Claude Code, non-Claude ACPs that also read CLAUDE.md, or a post-compact
/// Claude session) knows where the resolved global rules live.
fn render_pointer_block(rules_path: &std::path::Path) -> String {
    format!(
        "{begin}\n\
         Global governance rules resolved by sensei live at `{path}`. \
         Rules flagged **mandatory** are non-negotiable and cannot be \
         overridden by more-specific scopes.\n\
         {end}\n",
        begin = CLAUDE_MD_BEGIN,
        end   = CLAUDE_MD_END,
        path  = rules_path.display(),
    )
}

/// Splice `new_block` into `existing`, replacing any prior sensei-managed block
/// (between `CLAUDE_MD_BEGIN` and `CLAUDE_MD_END`). If no prior block exists,
/// appends the new block after one blank line so it sits at the end of the
/// file without smashing into pre-existing user content. Pure — returns the
/// new file contents.
pub(crate) fn splice_pointer_block(existing: &str, new_block: &str) -> String {
    if let Some(begin_idx) = existing.find(CLAUDE_MD_BEGIN)
        && let Some(end_rel) = existing[begin_idx..].find(CLAUDE_MD_END)
    {
        let end_idx = begin_idx + end_rel + CLAUDE_MD_END.len();
        // Also swallow one trailing newline so we don't accumulate blank
        // lines across re-runs.
        let after = existing[end_idx..].strip_prefix('\n').unwrap_or(&existing[end_idx..]);
        let mut out = String::with_capacity(existing.len() + new_block.len());
        out.push_str(&existing[..begin_idx]);
        out.push_str(new_block);
        out.push_str(after);
        return out;
    }
    let mut out = existing.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(new_block);
    out
}

/// Upsert the sensei pointer block into the user's global `~/.claude/CLAUDE.md`.
/// Only touches an existing file (no-op if the file is missing) — sensei never
/// creates a global CLAUDE.md on behalf of the user; if they haven't set up
/// Claude Code globally, the session-start hook at
/// `marketplace/plugins/sensei/hooks/session-start` covers rules injection
/// on its own. Idempotent: rerun replaces the managed block, preserves
/// everything else. Returns the path written and whether it changed.
pub(crate) fn upsert_pointer_in_claude_md(
    claude_md: &std::path::Path,
    rules_path: &std::path::Path,
) -> Result<Option<(std::path::PathBuf, bool)>, String> {
    if !claude_md.exists() {
        return Ok(None);
    }
    let existing = std::fs::read_to_string(claude_md)
        .map_err(|e| format!("read {}: {e}", claude_md.display()))?;
    let new_block = render_pointer_block(rules_path);
    let out = splice_pointer_block(&existing, &new_block);
    if out == existing {
        return Ok(Some((claude_md.to_path_buf(), false)));
    }
    std::fs::write(claude_md, out)
        .map_err(|e| format!("write {}: {e}", claude_md.display()))?;
    Ok(Some((claude_md.to_path_buf(), true)))
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

// ============================================================================
// POST /api/knowledge/memories/{id}/generalise  — rewrite project-agnostic
// ============================================================================

/// System prompt for the generalise rewrite: turn a project-specific memory
/// into a portable rule. Mirrors `corrections_llm::SYSTEM` — a strict
/// JSON-only instruction the `reasoning` chain can honour deterministically.
const GENERALISE_SYSTEM: &str = "You rewrite a developer's project-specific memory into a portable, project-agnostic rule. \
Strip every project-specific identifier — project names, repository names, file paths, service names, person names, ticket ids — \
and restate the learning as a general principle that would apply across projects. \
Stay faithful: do not invent scope, do not add advice the original did not contain, do not reproduce the identifiers you removed. \
Reply with ONLY a JSON object: {\"generalised\": <the rewritten rule as one or two plain sentences>}. No prose, no code fences.";

/// Token budget for the rewrite (one short JSON object; reasoning headroom).
const GENERALISE_MAX_TOKENS: u32 = 512;
/// Cap the memory body shown to the model so a runaway `content` can't blow the
/// request budget.
const GENERALISE_MAX_CONTENT: usize = 4000;
/// Wall-clock cap on the user-triggered rewrite so a hung model can't wedge the
/// request forever (the UI waits on this with a loading state).
const GENERALISE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Build the user message: the memory's title + (bounded) content, asking the
/// model to restate it project-agnostic. Pure — mirrors
/// `corrections_llm::build_user_message`.
pub(crate) fn build_generalise_message(title: &str, content: &str) -> String {
    let body: String = content.chars().take(GENERALISE_MAX_CONTENT).collect();
    format!(
        "Memory title: {}\n\nMemory content:\n{}\n\nRewrite this as a project-agnostic rule.",
        title.replace('\n', " "),
        body,
    )
}

/// Parse the model's `{ "generalised": "..." }` object. Tolerates surrounding
/// prose / code fences by extracting the first `{ … }`. Returns `None` when
/// there is no usable text — the caller then degrades (503) rather than
/// fabricating a generalisation. Mirrors `corrections_llm::parse_response`.
pub(crate) fn parse_generalise_response(content: &str) -> Option<String> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end <= start {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(&content[start..=end]).ok()?;
    v.get("generalised")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// Rewrite a project-scoped memory into a project-agnostic rule that is ready to
/// widen up the scope ladder (see [[pipeline/memory]]). User-triggered (the UI
/// shows a loading state), so this is a request-time gateway call bounded by
/// `GENERALISE_TIMEOUT`. Graceful and honest: if the model is unavailable,
/// times out, or returns nothing usable, the `generalised` flag stays unset and
/// the request returns 503 — it never fabricates a rewrite or silently succeeds.
/// Widening itself reuses the existing `promote_memory` path; this only produces
/// the portable text + flag.
pub(crate) async fn generalise_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    use gateway::types::capability::Capability;
    use gateway::types::request::*;

    let mid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad memory id"))?;
    let memory = state.pg.get_memory(&mid).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "memory not found"))?;
    let title = memory["title"].as_str().unwrap_or_default();
    let content = memory["content"].as_str().unwrap_or_default();
    if content.trim().is_empty() {
        return Err(err(StatusCode::UNPROCESSABLE_ENTITY, "memory has no content to generalise"));
    }

    let request = InferenceRequest {
        capability: Capability::TextChat,
        model: None,
        router: None,
        // Faithful rewrite is synthesis — pin the seed `reasoning` chain
        // (embedded → ollama → cloud), same as the corrections summariser.
        chain: Some("reasoning".into()),
        payload: Payload::Chat {
            messages: vec![Message::text(MessageRole::User, build_generalise_message(title, content))],
            system: Some(GENERALISE_SYSTEM.to_string()),
            max_tokens: Some(GENERALISE_MAX_TOKENS),
            temperature: None,
            tools: Vec::new(),
        },
        budget: None,
    };

    // Timeout / gateway error / empty-or-unparseable output all degrade the SAME
    // way: surface it (503 + tracing), leave the flag unset, never fabricate.
    let generalised = match tokio::time::timeout(GENERALISE_TIMEOUT, state.gateway.execute(&request)).await {
        Ok(Ok(resp)) if resp.success => resp.content.as_deref().and_then(parse_generalise_response),
        Ok(Ok(_)) => None,
        Ok(Err(e)) => {
            tracing::warn!(memory_id = %mid, error = %e, "generalise: gateway call failed");
            None
        }
        Err(_) => {
            tracing::warn!(memory_id = %mid, "generalise: gateway call timed out");
            None
        }
    };
    let Some(text) = generalised else {
        tracing::warn!(memory_id = %mid, "generalise: no usable rewrite — flag left unset");
        return Err(err(StatusCode::SERVICE_UNAVAILABLE,
            "could not generalise this memory right now — the model was unavailable or returned nothing usable; try again"));
    };

    state.pg.set_memory_generalisation(mid, &text).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "memory not found"))?;

    Ok(Json(serde_json::json!({
        "id": mid,
        "original": content,
        "generalised": text,
    })))
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
        model: None, router: None,
        // Governance Tier-2 consolidation is heavy synthesis — pin the seed
        // `reasoning` chain (embedded → ollama → cloud) rather than relying on
        // non-deterministic capability resolution across the text chains (#76).
        chain: Some("reasoning".into()),
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
    // Parse the URL rather than substring-match: require https unless the host is
    // exactly a loopback (so `http://127.0.0.1.evil.com` can't smuggle the bearer
    // token to an attacker host in cleartext).
    let parsed = reqwest::Url::parse(&url).map_err(|_| err(StatusCode::BAD_REQUEST, "invalid source url"))?;
    let is_loopback = matches!(parsed.host_str(), Some("127.0.0.1") | Some("::1") | Some("localhost"));
    if parsed.scheme() != "https" && !is_loopback {
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
        match tokio::task::spawn_blocking(move || crate::gateway_keys::delete_key(&cref)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!(source = %s.name, error = %e, "federation: keychain delete failed; entry may be orphaned"),
            Err(e) => tracing::warn!(source = %s.name, error = %e, "federation: keychain delete task join failed"),
        }
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
    let client = crate::federation::http_client();
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

    // ── generalise (project-agnostic rewrite) pure helpers ──────────────────

    #[test]
    fn build_generalise_message_includes_title_and_bounds_content() {
        let long = "z".repeat(GENERALISE_MAX_CONTENT + 500);
        let msg = build_generalise_message("Use dbd for migrations", &long);
        assert!(msg.contains("Memory title: Use dbd for migrations"), "title carried");
        assert!(msg.contains("project-agnostic rule"), "instruction carried");
        // Content bounded to GENERALISE_MAX_CONTENT chars (title has no 'z').
        assert_eq!(msg.matches('z').count(), GENERALISE_MAX_CONTENT, "content bounded");
    }

    #[test]
    fn build_generalise_message_flattens_title_newlines() {
        let msg = build_generalise_message("line1\nline2", "body");
        assert!(msg.contains("Memory title: line1 line2"), "title newlines flattened");
        assert!(msg.contains("body"));
    }

    #[test]
    fn parse_generalise_extracts_text() {
        let c = r#"{"generalised":"Prefer a dedicated migration tool over hand-rolled SQL."}"#;
        assert_eq!(
            parse_generalise_response(c).as_deref(),
            Some("Prefer a dedicated migration tool over hand-rolled SQL."),
        );
    }

    #[test]
    fn parse_generalise_tolerates_fences_and_surrounding_prose() {
        let c = "Here you go:\n```json\n{\"generalised\":\"Run pre-commit before opening a PR.\"}\n```";
        assert_eq!(
            parse_generalise_response(c).as_deref(),
            Some("Run pre-commit before opening a PR."),
        );
    }

    #[test]
    fn parse_generalise_none_on_empty_or_missing() {
        assert_eq!(parse_generalise_response(r#"{"generalised":""}"#), None, "empty string → None");
        assert_eq!(parse_generalise_response(r#"{"generalised":"   "}"#), None, "whitespace → None");
        assert_eq!(parse_generalise_response(r#"{"other":"x"}"#), None, "missing key → None");
        assert_eq!(parse_generalise_response("not json"), None, "no object → None");
        assert_eq!(parse_generalise_response(""), None, "empty input → None");
    }

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

    // #13 — Global rules pointer in ~/.claude/CLAUDE.md
    // -------------------------------------------------------------------

    #[test]
    fn splice_appends_when_no_managed_block_present() {
        let rules_path = std::path::PathBuf::from("/home/u/.sensei/rules.md");
        let existing = "# My CLAUDE.md\n\nUser content here.\n";
        let out = splice_pointer_block(existing, &render_pointer_block(&rules_path));
        assert!(out.starts_with("# My CLAUDE.md\n\nUser content here.\n"), "prior user content preserved verbatim");
        assert!(out.contains(CLAUDE_MD_BEGIN));
        assert!(out.contains(CLAUDE_MD_END));
        assert!(out.contains("/home/u/.sensei/rules.md"));
        assert!(out.contains("**mandatory**"));
    }

    #[test]
    fn splice_replaces_prior_block_and_does_not_accumulate() {
        let rules_path = std::path::PathBuf::from("/home/u/.sensei/rules.md");
        let existing_block = render_pointer_block(&std::path::PathBuf::from("/OLD/path/rules.md"));
        let user_before = "# User CLAUDE.md\n\nFirst paragraph.\n\n";
        let user_after = "\n## Later section\n\nMore user content.\n";
        let existing = format!("{user_before}{existing_block}{user_after}");

        let out = splice_pointer_block(&existing, &render_pointer_block(&rules_path));

        // New block present, old one gone.
        assert!(out.contains("/home/u/.sensei/rules.md"), "new path landed");
        assert!(!out.contains("/OLD/path/rules.md"), "old path swept");

        // Only one begin marker (idempotent — never doubles).
        assert_eq!(out.matches(CLAUDE_MD_BEGIN).count(), 1, "no marker accumulation");
        assert_eq!(out.matches(CLAUDE_MD_END).count(), 1);

        // Surrounding user content intact.
        assert!(out.contains("# User CLAUDE.md"));
        assert!(out.contains("First paragraph."));
        assert!(out.contains("## Later section"));
        assert!(out.contains("More user content."));
    }

    #[test]
    fn splice_is_a_fixed_point_on_second_run() {
        // A pointer block written once and re-spliced with the same path must
        // produce a byte-identical output — the daemon's startup upsert fires
        // on every boot and mustn't churn the file.
        let rules_path = std::path::PathBuf::from("/home/u/.sensei/rules.md");
        let block = render_pointer_block(&rules_path);
        let first = splice_pointer_block("# CLAUDE.md\n\nHello.\n", &block);
        let second = splice_pointer_block(&first, &block);
        assert_eq!(first, second, "idempotent: second run leaves the file unchanged");
    }

    #[test]
    fn upsert_is_noop_when_file_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_md = tmp.path().join("CLAUDE.md"); // does not exist
        let rules_path = tmp.path().join("rules.md");
        let result = upsert_pointer_in_claude_md(&claude_md, &rules_path).unwrap();
        assert!(result.is_none(), "must not create CLAUDE.md that isn't there");
    }

    #[test]
    fn upsert_writes_pointer_into_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_md = tmp.path().join("CLAUDE.md");
        std::fs::write(&claude_md, "# CLAUDE.md\n\nUser content.\n").unwrap();
        let rules_path = tmp.path().join("rules.md");

        let (path, changed) = upsert_pointer_in_claude_md(&claude_md, &rules_path).unwrap().unwrap();
        assert_eq!(path, claude_md);
        assert!(changed, "first upsert reports change=true");

        let after = std::fs::read_to_string(&claude_md).unwrap();
        assert!(after.contains(CLAUDE_MD_BEGIN));
        assert!(after.contains(rules_path.to_string_lossy().as_ref()));
        assert!(after.contains("# CLAUDE.md"), "user content preserved");

        // Rerun with same path → no change (byte-identical), reported as unchanged.
        let (_, changed2) = upsert_pointer_in_claude_md(&claude_md, &rules_path).unwrap().unwrap();
        assert!(!changed2, "second upsert reports change=false");
    }
}
