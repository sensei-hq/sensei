//! `/api/knowledge/*` — memory CRUD, proposals, outcomes, context assembly.

use crate::api::handlers::err;
use crate::api::state::AppState;
use crate::db::pg_store::{InsertMemory, OutcomeRow};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

/// Resolve a governance namespace from either an explicit `namespace_id` or a
/// `(gov_scope, folder)` pair against the repo's namespace memberships. Shared
/// by rule authoring and promotion so the resolution rule lives in one place.
async fn resolve_target_namespace(
    state: &AppState,
    namespace_id: Option<&str>,
    gov_scope: Option<&str>,
    folder: Option<&str>,
    project: Option<&str>,
) -> Result<Option<uuid::Uuid>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ns) = namespace_id.filter(|s| !s.is_empty()) {
        return Ok(Some(
            uuid::Uuid::parse_str(ns)
                .map_err(|_| err(StatusCode::BAD_REQUEST, "bad namespace_id"))?,
        ));
    }
    let Some(scope) = gov_scope.filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    // `general`/`user` are always-on rungs with no namespace row — a NULL
    // namespace_id is the CORRECT resolution for them.
    if matches!(scope, "general" | "user") {
        return Ok(None);
    }
    // No target at all → preserve the historical NULL resolution (the caller gave
    // a scope but nothing to resolve it against).
    if folder.filter(|s| !s.is_empty()).is_none() && project.filter(|s| !s.is_empty()).is_none() {
        return Ok(None);
    }
    // Resolve the target repo from folder OR project (#109: the MCP forwards a cwd
    // `folder` that mis-resolves to the container; an explicit `project` resolves
    // server-side via resolve_folder). FAIL CLOSED: a *specific* gov_scope whose
    // namespace can't be resolved errors rather than falling back to the always-on
    // `general` rung (which would govern every project at the caller's enforcement).
    let (_path, fid) = resolve_folder(state, folder, project).await?;
    match state
        .pg
        .namespace_for_folder_scope(&fid, scope)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
    {
        Some(ns) => Ok(Some(ns)),
        None => Err(err(
            StatusCode::BAD_REQUEST,
            format!(
                "cannot resolve gov_scope '{scope}': the target repo is not a member of any '{scope}'-scoped namespace — bind it to one, or pass an explicit namespace_id"
            ),
        )),
    }
}

// ============================================================================
// GET /api/knowledge/memories?status=&scope=&project_id=&limit=
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct ListQuery {
    pub status: Option<String>,
    pub scope: Option<String>,
    pub project_id: Option<String>,
    pub limit: Option<i64>,
}

pub(crate) async fn list_memories(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pid = match q.project_id {
        Some(s) => Some(
            uuid::Uuid::parse_str(&s)
                .map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?,
        ),
        None => None,
    };
    let rows = state
        .pg
        .list_memories(pid, q.status.as_deref(), q.scope.as_deref(), q.limit.unwrap_or(200))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
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
    // get_memory_detail already surfaces `evidence` (session rows + save-time source
    // notes, nullable session_id) — no extra merge needed here.
    let detail = state.pg.get_memory_detail(mid).await.map_err(|e| {
        if e.contains("not found") {
            err(StatusCode::NOT_FOUND, "memory not found")
        } else {
            err(StatusCode::INTERNAL_SERVER_ERROR, &e)
        }
    })?;
    Ok(Json(detail))
}

// ============================================================================
// GET /api/knowledge/context?project_id=&limit=&tags=csv&slot=&feature=
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct ContextQuery {
    /// Project UUID (the historical contract).
    pub project_id: Option<String>,
    /// Project name OR UUID — the shape the sensei MCP proxy sends, since
    /// `resolve_project` yields a name. Resolved daemon-side to the UUID via the
    /// shared `resolve_project_uuid`, mirroring `get_project_commands`.
    pub project: Option<String>,
    pub limit: Option<i64>,
    pub tags: Option<String>,
    /// Optional spine slot hint (`sensei.spine_slot`) — when present, memories
    /// anchored to this slot (+ optional `feature`) lead the assembled bundle
    /// (see `PgStore::assemble_context`'s `slot` param). Absent → unchanged
    /// general blend, exactly the prior behavior.
    pub slot: Option<String>,
    /// Feature name for a feature-scope slot (brief/plan/tests). Ignored when
    /// `slot` is absent.
    pub feature: Option<String>,
}

pub(crate) async fn get_context(
    State(state): State<AppState>,
    Query(q): Query<ContextQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Accept EITHER `project_id` (uuid) OR `project` (name or uuid). Both go
    // through the shared resolver so a name resolves to the real UUID and a raw
    // UUID passes straight through — additive, the uuid contract is unchanged.
    let ident = q
        .project_id
        .as_deref()
        .or(q.project.as_deref())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "project_id or project required"))?;
    let pid = crate::api::util::resolve_project_uuid(&state, ident)
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "project lookup failed"))?
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "bad project_id"))?;
    let tags: Option<Vec<String>> = q.tags.map(|s| {
        s.split(',').filter(|t| !t.trim().is_empty()).map(|t| t.trim().to_string()).collect()
    });
    let stack_ids = state
        .pg
        .get_project_stack_ids(&pid)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let slot = q
        .slot
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| (s, q.feature.as_deref().filter(|f| !f.is_empty())));
    let blob = state
        .pg
        .assemble_context(pid, &stack_ids, tags.as_deref(), q.limit.unwrap_or(200), slot)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(blob))
}

// ============================================================================
// GET /api/knowledge/rules?folder=<abs_path>  — governance Tier-1 resolution
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct RulesQuery {
    /// Absolute path of the repo whose governing rules to resolve.
    pub folder: Option<String>,
    /// Project name OR UUID — the shape the sensei MCP proxy knows (it resolves
    /// the caller to a project, not to the repo's abs_path). Resolved daemon-side
    /// to the project's root repo abs_path, so a caller that knows the project
    /// but not the folder still gets the right ruleset.
    pub project: Option<String>,
    /// `md` → return rendered Markdown (`markdown` field) instead of the raw
    /// `rules` array — the shape the SessionStart/PreCompact hooks inject
    /// directly (D-INJECT). Any other value / omitted → the JSON rules array.
    pub format: Option<String>,
    /// Comma-separated enforcement tiers to include when `format=md` (e.g.
    /// `mandatory,required`). Omitted → all tiers. Ignored for the JSON shape.
    pub tiers: Option<String>,
}

/// Resolve the rules governing a repo: gather its namespace memberships + the
/// always-on general/user scopes, ordered by enforcement then scope level,
/// deduped, with the mandatory (non-overridable) ones flagged.
///
/// Accepts EITHER `folder=<abs_path>` (the historical contract) OR
/// `project=<name|uuid>` → resolved to the project's root repo abs_path.
pub(crate) async fn get_rules(
    State(state): State<AppState>,
    Query(q): Query<RulesQuery>,
) -> Result<axum::response::Response, (StatusCode, Json<serde_json::Value>)> {
    use axum::response::IntoResponse;
    let (folder_path, folder_id) =
        resolve_folder(&state, q.folder.as_deref(), q.project.as_deref()).await?;
    let ruleset = resolve_repo_ruleset(&state, &folder_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    // Markdown push shape (D-INJECT): the hooks fetch rendered, tier-filtered
    // Markdown as PLAIN TEXT (no client-side jq — macOS ships without it) and
    // inject it straight into additionalContext.
    if q.format.as_deref() == Some("md") {
        let tiers: Vec<&str> = match q.tiers.as_deref().filter(|s| !s.is_empty()) {
            Some(list) => list
                .split(',')
                .map(str::trim)
                .filter(|t| crate::governance::ALL_TIERS.contains(t))
                .collect(),
            None => crate::governance::ALL_TIERS.to_vec(),
        };
        let markdown = crate::governance::render_rules_tiers(&ruleset, &tiers);
        return Ok((
            [(axum::http::header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
            markdown,
        )
            .into_response());
    }

    Ok(Json(serde_json::json!({
        "folder": folder_path,
        "total": ruleset.total,
        "mandatory_count": ruleset.mandatory_count,
        "rules": ruleset.rules,
    }))
    .into_response())
}

/// Resolve the global ruleset (user + general scope) and write it to
/// `<dir>/rules.md`, returning the path and rule count. `dir` is injected so the
/// daemon passes `~/.sensei` and tests can pass a temp dir.
pub(crate) async fn materialize_global_rules(
    pg: &crate::db::pg_store::PgStore,
    dir: &std::path::Path,
) -> Result<(std::path::PathBuf, usize), String> {
    // The always-on global set = general/user memories + the packs adopted at those
    // scopes (D-SEED: the bundled constitution + ponytail resolve into ~/.sensei/
    // rules.md, offline). `None` = no folder → only general/user pack adoptions.
    // Folded here, not in resolve_global_rules, so the LLM consolidation path
    // (rule_consolidation) keeps operating on learned memories, not curated packs.
    let mut raw = pg.resolve_global_rules().await?;
    // Fail closed: a pack-resolution DB error must NOT silently drop adopted
    // (possibly mandatory) pack rules from the governing set — propagate it.
    raw.extend(pg.resolve_local_pack_raws(None).await?);
    let ruleset = crate::governance::structure_ruleset(raw);
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
    let (path, count) = materialize_global_rules(&state.pg, &dir)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "path": path.display().to_string(), "rules": count })))
}

/// Managed-block marker used to bracket the sensei pointer inside the user's
/// global `~/.claude/CLAUDE.md`. Any content between the two lines is owned by
/// the daemon and safe to rewrite on every startup; content outside is
/// user-authored and preserved verbatim.
const CLAUDE_MD_BEGIN: &str = "<!-- sensei:global-rules-pointer BEGIN -->";
const CLAUDE_MD_END: &str = "<!-- sensei:global-rules-pointer END -->";

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
        end = CLAUDE_MD_END,
        path = rules_path.display(),
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
    std::fs::write(claude_md, out).map_err(|e| format!("write {}: {e}", claude_md.display()))?;
    Ok(Some((claude_md.to_path_buf(), true)))
}

// ============================================================================
// POST /api/knowledge/proposals  — propose_memory
// POST /api/knowledge/memories   — save_memory (explicit)
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct MemoryBody {
    pub project_id: Option<String>,
    pub scope: String,
    pub scope_filter: Option<String>,
    #[serde(rename = "type")]
    pub mtype: String,
    pub title: String,
    pub content: String,
    pub impact: Option<String>,
    /// Optional save-time source evidence (a file:line, test name, run id) — stored
    /// as a session-less `memory_evidence` row so the memory carries its provenance.
    pub evidence: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub triage_signal: Option<String>,
    // ── Governance plane (optional) ──────────────────────────────────────
    /// Explicit namespace this rule applies to (wins over gov_scope/folder).
    pub namespace_id: Option<String>,
    /// Governance scope key (general/user/organization/client/technology/team/
    /// project/repository); resolved against `folder`'s namespace memberships.
    pub gov_scope: Option<String>,
    /// Repo abs_path used to resolve `gov_scope` to a namespace.
    pub folder: Option<String>,
    /// Authority: advisory|recommended|required|mandatory (default recommended).
    pub enforcement: Option<String>,
    // ── Spine anchoring (optional) ────────────────────────────────────────
    /// Doc-slot this memory anchors to (`sensei.spine_slot`); scope-validated
    /// against `feature` via `memory_slot::validate_scope`.
    pub spine_slot: Option<String>,
    /// Feature name for feature-scoped slots (brief/plan/tests); must be
    /// absent for project-only slots (vision/personas/journeys/roadmap/mockups).
    pub feature: Option<String>,
}

async fn insert_with_status(
    state: AppState,
    body: MemoryBody,
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
    // C4: never persist a secret into the shared memory / governance store. Fail
    // closed — reject the write, surfacing only the secret KIND, never the value.
    let hits = crate::secret_scan::scan(&format!("{}\n{}", body.title, body.content));
    if !hits.is_empty() {
        let kinds: Vec<&str> = hits.iter().map(|h| h.kind).collect();
        tracing::warn!(kinds = ?kinds, "rejected a memory write carrying a secret");
        return Err(err(
            StatusCode::BAD_REQUEST,
            format!(
                "content appears to contain a secret ({}) — not saved; remove it and retry",
                kinds.join(", ")
            ),
        ));
    }
    if body.scope == "stack" && body.scope_filter.as_deref().unwrap_or("").is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "scope_filter required for scope='stack'"));
    }
    if require_triage_signal && body.triage_signal.as_deref().unwrap_or("").is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "triage_signal required for proposals"));
    }
    // Spine anchoring: an explicit slot must be a known `sensei.spine_slot`
    // value, and (slot, feature) must satisfy the project-vs-feature scope
    // rule (crate::memory_slot::validate_scope) before it reaches the DB.
    let spine_slot = body.spine_slot.as_deref().filter(|s| !s.is_empty());
    if let Some(slot_str) = spine_slot {
        let slot = crate::memory_slot::SpineSlot::parse(slot_str).ok_or_else(|| {
            err(StatusCode::BAD_REQUEST, format!("unknown spine_slot: {slot_str}"))
        })?;
        crate::memory_slot::validate_scope(slot, body.feature.as_deref())
            .map_err(|e| err(StatusCode::BAD_REQUEST, &e))?;
    }
    let pid = match &body.project_id {
        Some(s) => Some(
            uuid::Uuid::parse_str(s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?,
        ),
        None => None,
    };
    // Governance namespace: explicit namespace_id wins; else resolve gov_scope
    // against the repo's namespace memberships.
    let namespace_id = resolve_target_namespace(
        &state,
        body.namespace_id.as_deref(),
        body.gov_scope.as_deref(),
        body.folder.as_deref(),
        body.project_id.as_deref(),
    )
    .await?;
    let id = state
        .pg
        .insert_memory(&InsertMemory {
            project_id: pid,
            scope: body.scope,
            scope_filter: body.scope_filter,
            mtype: body.mtype,
            title: body.title,
            content: body.content,
            impact: body.impact,
            tags: body.tags,
            triage_signal: body.triage_signal,
            status: status.into(),
            namespace_id,
            enforcement: body.enforcement.filter(|s| !s.is_empty()),
            origin: Some(origin.to_string()),
            source_id: None,
            spine_slot: body.spine_slot.filter(|s| !s.is_empty()),
            feature: body.feature.filter(|s| !s.is_empty()),
        })
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    // Record save-time source evidence (session-less) so the memory carries its
    // provenance. Non-fatal: the memory is saved even if the evidence note fails.
    if let Some(note) = body.evidence.as_deref().map(str::trim).filter(|s| !s.is_empty())
        && let Err(e) = state.pg.add_memory_evidence(&id, None, Some(note)).await
    {
        tracing::warn!(memory_id = %id, error = %e, "failed to record save-time evidence note");
    }
    Ok(Json(serde_json::json!({ "id": id, "status": status })))
}

pub(crate) async fn propose_memory(
    State(state): State<AppState>,
    Json(body): Json<MemoryBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    insert_with_status(state, body, "proposed", true, "learned").await
}

pub(crate) async fn save_memory(
    State(state): State<AppState>,
    Json(body): Json<MemoryBody>,
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
    pub gov_scope: Option<String>,
    pub folder: Option<String>,
    /// #109: resolve the gov_scope's repo from a project when the MCP can't send a
    /// valid folder (its cwd mis-resolves to the container).
    pub project: Option<String>,
    pub enforcement: Option<String>,
}

/// Promote a proven memory to a broader scope. Creates a `proposed` copy on the
/// target namespace (origin=promoted, source_id=original); accepting it through
/// the normal proposal flow is the approval gate, so it never auto-applies.
pub(crate) async fn promote_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PromoteBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let sid =
        uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad memory id"))?;
    let target = resolve_target_namespace(
        &state,
        body.namespace_id.as_deref(),
        body.gov_scope.as_deref(),
        body.folder.as_deref(),
        body.project.as_deref(),
    )
    .await?;
    let new_id = state
        .pg
        .promote_memory(sid, target, body.enforcement.as_deref().filter(|s| !s.is_empty()))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| {
            err(
                StatusCode::CONFLICT,
                "memory not found or not promotable (must be active/reinforced/battle_tested)",
            )
        })?;
    Ok(Json(serde_json::json!({ "id": new_id, "status": "proposed", "origin": "promoted" })))
}

// ============================================================================
// Memory lifecycle actions — the triage / active / archive curation surface
// (see [[screen/observatory-memories]]). Thin bridges over existing PgStore
// writers; results are deterministic action outcomes, NOT generated prose, so
// they never route through narration-cache.
//   POST /api/knowledge/memories/{id}/archive    → status = archived
//   POST /api/knowledge/memories/{id}/reinforce  → strength += REINFORCE_AMOUNT
//   POST /api/knowledge/memories/{id}/challenge  → status = challenged
//   POST /api/knowledge/memories/{id}/dismiss    → status = rejected
//   POST /api/knowledge/memories/{id}/merge      → link under {into} + archive
// ============================================================================

/// Live (non-terminal) memory states a curator can still challenge / dismiss
/// from the triage + active surface. `archived` and `rejected` are terminal, so
/// the status guard won't match them → CONFLICT (can't re-terminate a memory).
const CURATABLE_STATES: &[&str] =
    &["proposed", "active", "reinforced", "challenged", "battle_tested"];

/// Strength increment applied by one reinforce action — the standard per-event
/// bump (`reinforce_memory` caps the running total).
const REINFORCE_AMOUNT: f64 = 1.0;

/// Parse an `{id}` path segment into a memory `Uuid`, or 400.
fn parse_memory_id(id: &str) -> Result<uuid::Uuid, (StatusCode, Json<serde_json::Value>)> {
    uuid::Uuid::parse_str(id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad memory id"))
}

/// Archive a memory — hides it from LLM retrieval while keeping it visible in
/// the anatomy view under the Archived filter.
pub(crate) async fn archive_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = parse_memory_id(&id)?;
    state.pg.archive_memory(&mid).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "id": mid, "status": "archived" })))
}

/// Reinforce a memory — bump its strength (capped in the writer). Status
/// promotion to `reinforced` / `battle_tested` is derived by the analyzer, not
/// this action, so only strength moves here.
pub(crate) async fn reinforce_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = parse_memory_id(&id)?;
    state
        .pg
        .reinforce_memory(&mid, REINFORCE_AMOUNT)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "id": mid, "reinforced": true })))
}

/// Challenge a memory — flag it contested. Only applies to a live state
/// ([`CURATABLE_STATES`]); a terminal memory returns CONFLICT.
pub(crate) async fn challenge_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = parse_memory_id(&id)?;
    match state
        .pg
        .set_memory_status(mid, "challenged", CURATABLE_STATES)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
    {
        Some(s) => Ok(Json(serde_json::json!({ "id": mid, "status": s }))),
        None => Err(err(StatusCode::CONFLICT, "memory is not in a challengeable state")),
    }
}

/// Dismiss a memory — reject it (the enum's terminal `rejected` value; there is
/// no separate `dismissed` variant). Only applies to a live state.
pub(crate) async fn dismiss_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = parse_memory_id(&id)?;
    match state
        .pg
        .set_memory_status(mid, "rejected", CURATABLE_STATES)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
    {
        Some(s) => Ok(Json(serde_json::json!({ "id": mid, "status": s }))),
        None => Err(err(StatusCode::CONFLICT, "memory is not in a dismissable state")),
    }
}

#[derive(Deserialize)]
pub(crate) struct MergeBody {
    /// The surviving representative memory this one is folded into.
    pub into: Option<String>,
}

/// Merge a memory into a surviving representative: the member (`{id}`) is linked
/// under the representative (`into`) and archived, so it leaves the active set
/// while staying visible in the anatomy view. Matches the consolidation
/// invariant — "members archive with `merged_into: representative_id`" (see
/// [[pipeline/memory]]). Absorbing the survivor's strength / reinforcement
/// counts is the consolidation flow's job; this is the thin manual-merge bridge.
pub(crate) async fn merge_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<MergeBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = parse_memory_id(&id)?;
    let into = body
        .into
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "into (surviving memory id) required"))?;
    let into =
        uuid::Uuid::parse_str(into).map_err(|_| err(StatusCode::BAD_REQUEST, "bad into id"))?;
    if into == mid {
        return Err(err(StatusCode::BAD_REQUEST, "cannot merge a memory into itself"));
    }
    // The survivor must exist — otherwise the link FK would surface as a 500.
    state
        .pg
        .get_memory(&into)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "merge target (into) not found"))?;
    // parent = surviving representative; child = merged-in member (DDL semantics).
    state
        .pg
        .link_memories(&into, &mid)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    state.pg.archive_memory(&mid).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "id": mid, "into": into, "status": "archived" })))
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

    let mid =
        uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad memory id"))?;
    let memory = state
        .pg
        .get_memory(&mid)
        .await
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
            messages: vec![Message::text(
                MessageRole::User,
                build_generalise_message(title, content),
            )],
            system: Some(GENERALISE_SYSTEM.to_string()),
            max_tokens: Some(GENERALISE_MAX_TOKENS),
            temperature: None,
            tools: Vec::new(),
        },
        budget: None,
        auth: None,
        panel: None,
        consensus: None,
        allow_fallback: true,
        credentials: std::collections::HashMap::new(),
    };

    // Timeout / gateway error / empty-or-unparseable output all degrade the SAME
    // way: surface it (503 + tracing), leave the flag unset, never fabricate.
    let generalised =
        match tokio::time::timeout(GENERALISE_TIMEOUT, state.gateway.execute(&request)).await {
            Ok(Ok(resp)) if resp.success => {
                resp.content.as_deref().and_then(parse_generalise_response)
            }
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
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "could not generalise this memory right now — the model was unavailable or returned nothing usable; try again",
        ));
    };

    state
        .pg
        .set_memory_generalisation(mid, &text)
        .await
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
    let rows = state
        .pg
        .list_promotion_candidates()
        .await
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
    use crate::analysis::rule_consolidation::{ConsolidationOutcome, consolidate_global_rules};
    // Shared with the scheduled `ConsolidateGovernance` task — identical pipeline.
    match consolidate_global_rules(&state.pg, &state.gateway).await {
        Ok(ConsolidationOutcome::Skipped(reason)) => {
            Ok(Json(serde_json::json!({ "skipped": true, "reason": reason })))
        }
        Ok(ConsolidationOutcome::Created { id, version, model, content }) => {
            Ok(Json(serde_json::json!({
                "id": id, "version": version, "status": "proposed", "model": model, "content": content,
            })))
        }
        // A model/gateway failure is a 502; anything else (DB) a 500.
        Err(e) => {
            let code = if e.contains("merge model") {
                StatusCode::BAD_GATEWAY
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            Err(err(code, &e))
        }
    }
}

/// Fetch the global consolidated ruleset — the approved one if present, else the
/// latest proposed.
pub(crate) async fn get_consolidated(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let approved = state
        .pg
        .get_consolidated_ruleset("global", Some("approved"))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let row = match approved {
        Some(r) => Some(r),
        None => state
            .pg
            .get_consolidated_ruleset("global", None)
            .await
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
    let rid =
        uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad ruleset id"))?;
    let (scope, _content) = state
        .pg
        .approve_consolidated_ruleset(&rid)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "ruleset not found"))?;
    // The approved global merge now feeds ~/.sensei/rules.md.
    if scope == "global" {
        materialize_global_rules(&state.pg, &crate::paths::sensei_dir())
            .await
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
    let new_status = state
        .pg
        .set_memory_status(mid, "active", &["proposed"])
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    match new_status {
        Some(s) => {
            // Federation: if this was a promoted rule at a shareable scope, push it.
            // Fire-and-forget — federation must not block or fail the approval.
            let pg = state.pg.clone();
            tokio::spawn(async move {
                crate::federation::push_promoted(&pg, mid).await;
            });
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
    let new_status = state
        .pg
        .set_memory_status(mid, "rejected", &["proposed"])
        .await
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
    pub memory_id: String,
    pub outcome: String,
    pub session_id: Option<String>,
    pub context: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct OutcomesBatch {
    pub outcomes: Vec<OutcomeBody>,
}

pub(crate) async fn record_outcomes(
    State(state): State<AppState>,
    Json(body): Json<OutcomesBatch>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let valid_outcomes = ["applied", "consulted", "violated", "ignored"];
    let mut rows: Vec<OutcomeRow> = Vec::with_capacity(body.outcomes.len());
    for o in body.outcomes {
        if !valid_outcomes.contains(&o.outcome.as_str()) {
            return Err(err(StatusCode::BAD_REQUEST, format!("invalid outcome: {}", o.outcome)));
        }
        let mid = uuid::Uuid::parse_str(&o.memory_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "bad memory_id"))?;
        let sess = match o.session_id {
            Some(s) => Some(
                uuid::Uuid::parse_str(&s)
                    .map_err(|_| err(StatusCode::BAD_REQUEST, "bad session_id"))?,
            ),
            None => None,
        };
        rows.push(OutcomeRow {
            memory_id: mid,
            session_id: sess,
            outcome: o.outcome,
            context: o.context,
        });
    }
    let total = rows.len();
    let skipped = state
        .pg
        .record_outcomes_batch(&rows)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({
        "recorded": total - skipped.len(),
        "skipped":  skipped,
    })))
}

// ============================================================================
// Federation sources — /api/knowledge/sources*
// GET    /api/knowledge/sources             — list registered Dōjō rules sources
// POST   /api/knowledge/sources             — register a source (+ Keychain cred)
// DELETE /api/knowledge/sources/{id}         — deregister (+ purge Keychain cred)
// POST   /api/knowledge/sources/{id}/sync    — pull this source now
// GET    /api/knowledge/sources/{id}/status  — current cursor / enabled state
//
// D1: a rules source `url` is the Worker tenant base `{registry}/v1/t/{origin}/{org}`
// (the daemon appends `/rules`), and `api_key` carries the per-membership device
// token — the SAME tenant-path + device-token plane the artifacts client uses.
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
    // Require https unless the host is exactly a loopback — see
    // `api::util::require_secure_url` (shared with the Dōjō registration path).
    crate::api::util::require_secure_url(&url, "source url")
        .map_err(|e| err(StatusCode::BAD_REQUEST, &e))?;
    let namespace_id = match b.namespace_id.as_deref() {
        Some(s) => Some(
            uuid::Uuid::parse_str(s)
                .map_err(|_| err(StatusCode::BAD_REQUEST, "bad namespace_id"))?,
        ),
        None => None,
    };
    let credential_ref = format!("dojo-{}", uuid::Uuid::new_v4());
    let cref = credential_ref.clone();
    let api_key = b.api_key.clone();
    tokio::task::spawn_blocking(move || crate::gateway_keys::set_key(&cref, &api_key))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let id = state
        .pg
        .create_knowledge_source(&crate::db::pg_store::NewKnowledgeSource {
            kind: b.kind.unwrap_or_else(|| "hive_mind".into()),
            name: b.name,
            url,
            namespace_id,
            credential_ref,
            direction: b.direction.unwrap_or_else(|| "both".into()),
        })
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "id": id.to_string() })))
}

pub(crate) async fn list_sources(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let rows = state
        .pg
        .list_knowledge_sources()
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let out: Vec<_> = rows
        .into_iter()
        .map(|s| {
            serde_json::json!({
        "id": s.id, "kind": s.kind, "name": s.name, "url": s.url,
        "namespace_id": s.namespace_id, "direction": s.direction,
        "last_seq": s.last_seq, "enabled": s.enabled })
        })
        .collect();
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
            Ok(Err(e)) => {
                tracing::warn!(source = %s.name, error = %e, "federation: keychain delete failed; entry may be orphaned")
            }
            Err(e) => {
                tracing::warn!(source = %s.name, error = %e, "federation: keychain delete task join failed")
            }
        }
    }
    let removed = state
        .pg
        .delete_knowledge_source(&sid)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    if removed {
        Ok(Json(serde_json::json!({ "deleted": true })))
    } else {
        Err(err(StatusCode::NOT_FOUND, "no such source"))
    }
}

pub(crate) async fn sync_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let sid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let src = state
        .pg
        .get_knowledge_source(&sid)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such source"))?;
    let client = crate::federation::http_client();
    let stats = crate::federation::pull_source(&state.pg, &client, &src)
        .await
        .map_err(|e| err(StatusCode::BAD_GATEWAY, &e))?;
    Ok(Json(serde_json::to_value(stats).unwrap()))
}

pub(crate) async fn source_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let sid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let src = state
        .pg
        .get_knowledge_source(&sid)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such source"))?;
    Ok(Json(serde_json::json!({ "id": src.id, "name": src.name, "url": src.url,
        "direction": src.direction, "last_seq": src.last_seq, "enabled": src.enabled })))
}

/// Map a Dōjō resolved pack rule to the daemon's `RawRule`: the pack's `area` is
/// the display/grouping scope label and `source` the authority namespace. Pure.
fn pack_rule_to_raw(w: crate::dojo::client::PackRuleWire) -> crate::governance::RawRule {
    crate::governance::RawRule {
        id: w.rule_id,
        title: w.statement,
        content: w.body,
        impact: w.rationale,
        enforcement: w.enforcement,
        // The GOVERNANCE scope the pack was adopted at, NOT the pack's own
        // area/category — so a remote pack rule lands on the same ladder rung a
        // memory would (parity with the LOCAL `resolve_local_pack_raws` fix). A
        // stale Worker sends no `scope_key` → fall back to the broad `general`
        // scope (mirrors that resolver's `COALESCE(n.scope_key, 'general')`).
        scope: if w.scope_key.trim().is_empty() { "general".to_string() } else { w.scope_key },
        namespace: if w.source.is_empty() { None } else { Some(w.source) },
    }
}

/// Resolve the rules of packs adopted at a folder's namespaces, via the folder's
/// project → bound Dōjō membership → the Worker `rules/resolved` leg. Best-effort:
/// no binding / membership / namespaces, or any call fault → `[]` (logged, never
/// silent). The daemon can't query the Dōjō DB directly (Fork 1).
async fn resolve_adopted_pack_raws(
    state: &AppState,
    folder_id: &uuid::Uuid,
) -> Vec<crate::governance::RawRule> {
    let resolved = async {
        let project_id = state.pg.folder_project_id(folder_id).await.ok()??;
        let membership_id = state.pg.project_bound_membership(&project_id).await.ok()??;
        let membership = state.pg.get_dojo_membership(&membership_id).await.ok()??;
        let pairs = state.pg.folder_namespace_pairs(folder_id).await.ok()?;
        if pairs.is_empty() {
            return None;
        }
        let client = crate::dojo::client::DojoClient::for_membership(&membership);
        match client.resolved_pack_rules(&pairs).await {
            Ok(wires) => Some(wires.into_iter().map(pack_rule_to_raw).collect::<Vec<_>>()),
            Err(e) => {
                tracing::warn!(error = %e, "get_rules: adopted-pack resolve failed");
                None
            }
        }
    }
    .await;
    resolved.unwrap_or_default()
}

/// Resolve `folder=<abs_path>` OR `project=<name|uuid>` → `(folder_path,
/// folder_id)`. Shared by the rules + constitution endpoints.
pub(crate) async fn resolve_folder(
    state: &AppState,
    folder: Option<&str>,
    project: Option<&str>,
) -> Result<(String, uuid::Uuid), (StatusCode, Json<serde_json::Value>)> {
    let folder_path = match folder.filter(|s| !s.is_empty()) {
        Some(f) => f.to_string(),
        None => {
            let project = project
                .filter(|s| !s.is_empty())
                .ok_or_else(|| err(StatusCode::BAD_REQUEST, "folder or project required"))?;
            let pid = crate::api::util::resolve_project_uuid(state, project)
                .await
                .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "project lookup failed"))?
                .ok_or_else(|| err(StatusCode::NOT_FOUND, "unknown project"))?;
            let repos = state
                .pg
                .get_project_repos(&pid)
                .await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
            repos
                .first()
                .and_then(|r| r["path"].as_str())
                .map(str::to_string)
                .ok_or_else(|| err(StatusCode::NOT_FOUND, "project has no indexed repo"))?
        }
    };
    let folder = state
        .pg
        .get_repo_by_path(&folder_path)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "folder not indexed"))?;
    let folder_id = crate::api::util::json_uuid(&folder["id"])
        .ok_or_else(|| err(StatusCode::INTERNAL_SERVER_ERROR, "folder has no id"))?;
    Ok((folder_path, folder_id))
}

/// A repo's full governing ruleset: local memories + every adopted pack's rules
/// (P2 fold-in), structured strongest-first. Shared by get_rules + constitution.
async fn resolve_repo_ruleset(
    state: &AppState,
    folder_id: &uuid::Uuid,
) -> Result<crate::governance::ResolvedRuleset, String> {
    let mut raw = state.pg.resolve_rules_raw(folder_id).await?;
    // Rule packs resolve from TWO planes, in tandem (D-LOCAL-PACKS): the LOCAL
    // sensei.rule_packs replica (offline — bundled/adopted/synced packs) and the
    // remote Dōjō fold-in (a member's live org packs). structure_ruleset dedups by
    // content, so a pack present in both planes surfaces once.
    // Fail closed on the LOCAL plane (offline DB): a read error must not silently
    // weaken governance by dropping adopted pack rules. (The remote Dōjō fold-in
    // below stays best-effort — a remote hiccup can't fail local governance.)
    raw.extend(state.pg.resolve_local_pack_raws(Some(folder_id)).await?);
    raw.extend(resolve_adopted_pack_raws(state, folder_id).await);
    Ok(crate::governance::structure_ruleset(raw))
}

/// Group a resolved ruleset into the CONSTITUTION LADDER: one rung per scope,
/// ordered ascending scope level (most-general first, most-specific last — the
/// ladder's reading order), each rung's rules kept strongest-first. `general` /
/// unknown scopes sort first (level -1). Pure.
fn group_into_ladder(
    set: &crate::governance::ResolvedRuleset,
    scopes: &[(String, String, i32)],
) -> Vec<serde_json::Value> {
    use std::collections::BTreeMap;
    let meta = |key: &str| scopes.iter().find(|(k, _, _)| k == key).cloned();
    // (level, scope_key) → rules in resolved (strongest-first) order.
    let mut by_scope: BTreeMap<(i32, String), Vec<&crate::governance::ResolvedRule>> =
        BTreeMap::new();
    for r in &set.rules {
        let key = if r.scope.is_empty() { "general".to_string() } else { r.scope.clone() };
        let level = meta(&key).map(|(_, _, l)| l).unwrap_or(-1);
        by_scope.entry((level, key)).or_default().push(r);
    }
    by_scope
        .into_iter()
        .map(|((level, key), rules)| {
            let mandatory = rules.iter().filter(|r| r.mandatory).count();
            let name = meta(&key).map(|(_, n, _)| n).unwrap_or_else(|| key.clone());
            serde_json::json!({
                "scope_key": key,
                "scope_name": name,
                "level": level,
                "mandatory_count": mandatory,
                "rules": rules,
            })
        })
        .collect()
}

/// GET /api/knowledge/constitution?folder=X|project=Y → the repo's resolved
/// ruleset grouped into the constitution ladder (one rung per scope, general →
/// specific). What the console renders as "the rules in force here."
pub(crate) async fn get_constitution(
    State(state): State<AppState>,
    Query(q): Query<RulesQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (folder_path, folder_id) =
        resolve_folder(&state, q.folder.as_deref(), q.project.as_deref()).await?;
    let ruleset = resolve_repo_ruleset(&state, &folder_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let scopes =
        state.pg.list_scopes().await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({
        "folder": folder_path,
        "total": ruleset.total,
        "mandatory_count": ruleset.mandatory_count,
        "rungs": group_into_ladder(&ruleset, &scopes),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_into_ladder_orders_rungs_by_scope_level() {
        use crate::governance::{ResolvedRule, ResolvedRuleset};
        let mk = |title: &str, scope: &str, enf: &str| ResolvedRule {
            id: title.into(),
            title: title.into(),
            content: String::new(),
            impact: None,
            enforcement: enf.into(),
            scope: scope.into(),
            namespace: None,
            mandatory: enf == "mandatory",
        };
        // strongest-first input across two scopes (org level 20, project level 60)
        let set = ResolvedRuleset {
            rules: vec![
                mk("no secrets", "organization", "mandatory"),
                mk("idempotency key", "project", "mandatory"),
                mk("small commits", "project", "advisory"),
            ],
            total: 3,
            mandatory_count: 2,
        };
        let scopes = vec![
            ("organization".into(), "Organization".into(), 20),
            ("project".into(), "Project".into(), 60),
        ];
        let rungs = group_into_ladder(&set, &scopes);
        assert_eq!(rungs.len(), 2);
        // most-general first
        assert_eq!(rungs[0]["scope_key"], "organization");
        assert_eq!(rungs[0]["level"], 20);
        assert_eq!(rungs[1]["scope_key"], "project");
        assert_eq!(rungs[1]["mandatory_count"], 1);
        assert_eq!(rungs[1]["rules"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn pack_rule_to_raw_maps_fields() {
        let w = crate::dojo::client::PackRuleWire {
            rule_id: "r1".into(),
            statement: "Never log tokens".into(),
            body: "Applies at any log level.".into(),
            rationale: Some("leaks are a top breach vector".into()),
            enforcement: "mandatory".into(),
            source: "OWASP".into(),
            scope_key: "organization".into(),
        };
        let r = pack_rule_to_raw(w);
        assert_eq!(r.title, "Never log tokens");
        assert_eq!(r.content, "Applies at any log level.");
        assert_eq!(r.impact.as_deref(), Some("leaks are a top breach vector"));
        assert_eq!(r.enforcement, "mandatory");
        // scope = the ADOPTION scope (a governance scope), NOT the pack area 'security'.
        assert_eq!(r.scope, "organization", "adoption scope_key → scope (not pack area)");
        assert_eq!(r.namespace.as_deref(), Some("OWASP"));
    }

    #[test]
    fn pack_rule_to_raw_empty_source_is_no_namespace() {
        let w = crate::dojo::client::PackRuleWire {
            rule_id: "r2".into(),
            statement: "s".into(),
            body: String::new(),
            rationale: None,
            enforcement: "advisory".into(),
            source: String::new(),
            scope_key: String::new(),
        };
        let r = pack_rule_to_raw(w);
        assert!(r.namespace.is_none());
        // A stale Worker that sends no scope_key falls back to the broad 'general'
        // scope (never the pack area) — mirrors resolve_local_pack_raws' COALESCE.
        assert_eq!(r.scope, "general", "empty wire scope_key → general fallback");
    }

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
        let c =
            "Here you go:\n```json\n{\"generalised\":\"Run pre-commit before opening a PR.\"}\n```";
        assert_eq!(
            parse_generalise_response(c).as_deref(),
            Some("Run pre-commit before opening a PR."),
        );
    }

    #[test]
    fn parse_generalise_none_on_empty_or_missing() {
        assert_eq!(parse_generalise_response(r#"{"generalised":""}"#), None, "empty string → None");
        assert_eq!(
            parse_generalise_response(r#"{"generalised":"   "}"#),
            None,
            "whitespace → None"
        );
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

    #[tokio::test]
    async fn materialize_folds_general_adopted_packs_into_global_rules() {
        // D-SEED: a pack adopted at the always-on general scope must land in the
        // global ~/.sensei/rules.md (via resolve_local_pack_raws(None)), not just
        // in the per-repo get_rules path. Unique slug/namespace so this never
        // races the constitution seed test on the shared sensei_test DB.
        let pg = crate::db::pg_store::PgStore::connect_test().await.unwrap();
        let pool = pg.pool();
        sqlx_core::query::query(
            "DELETE FROM sensei.rule_packs WHERE slug = 'global-materialize-test'",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('general', 'General', 0, false) ON CONFLICT (key) DO NOTHING",
        )
        .execute(pool)
        .await
        .unwrap();
        let (pack,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.rule_packs
                (slug, name, area, attribution, summary, enforcement, owner_namespace_id, status, published_by)
             VALUES ('global-materialize-test', 'GM', 'principles', 'GMSource', 's',
                     'mandatory', NULL, 'active', 'test')
             RETURNING id")
            .fetch_one(pool).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_pack_rules(pack_id, ordinal, statement, body, enforcement)
             VALUES ($1, 1, 'Global materialize marker rule', 'B', 'mandatory')",
        )
        .bind(pack)
        .execute(pool)
        .await
        .unwrap();
        let (ns,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.namespaces(scope_key, slug, name)
             VALUES ('general', 'global-mat-test', 'GM') ON CONFLICT (scope_key, slug) DO UPDATE SET name=excluded.name
             RETURNING id")
            .fetch_one(pool).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_pack_adoptions(pack_id, namespace_id, pinned_version, adopted_by)
             VALUES ($1, $2, 1, 'test') ON CONFLICT (pack_id, namespace_id) DO NOTHING")
            .bind(pack).bind(ns).execute(pool).await.unwrap();

        let tmp = tempfile::tempdir().unwrap();
        let (path, _count) = materialize_global_rules(&pg, tmp.path()).await.unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(
            body.contains("Global materialize marker rule"),
            "a general-adopted pack rule is folded into the global rules file"
        );

        sqlx_core::query::query(
            "DELETE FROM sensei.rule_packs WHERE slug = 'global-materialize-test'",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE id = $1")
            .bind(ns)
            .execute(pool)
            .await
            .unwrap();
    }

    // #13 — Global rules pointer in ~/.claude/CLAUDE.md
    // -------------------------------------------------------------------

    #[test]
    fn splice_appends_when_no_managed_block_present() {
        let rules_path = std::path::PathBuf::from("/home/u/.sensei/rules.md");
        let existing = "# My CLAUDE.md\n\nUser content here.\n";
        let out = splice_pointer_block(existing, &render_pointer_block(&rules_path));
        assert!(
            out.starts_with("# My CLAUDE.md\n\nUser content here.\n"),
            "prior user content preserved verbatim"
        );
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

        let (path, changed) =
            upsert_pointer_in_claude_md(&claude_md, &rules_path).unwrap().unwrap();
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
