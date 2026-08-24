use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde::Deserialize;
use crate::api::state::AppState;

#[derive(Deserialize)]
pub(crate) struct RecoQuery {
    status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct SessionsQuery {
    limit: Option<i64>,
}

#[derive(Deserialize)]
pub(crate) struct CommandsQuery {
    /// Optional canonical verb (`test` / `build` / `lint` / `e2e` / …).
    /// Absent → return every category.
    pub category: Option<String>,
}

/// GET /api/projects/{id}/commands — #83 T1 commands surface. Returns every
/// discoverable command across the project's folders, or a category subset
/// when `?category=<verb>` is set. Populated by
/// `ManifestAdapter::parse_commands` during `extract_deps` — a re-scan
/// refreshes each folder's rows atomically.
///
/// Accepts `{id}` as a project name OR UUID, matching the read-side pattern
/// the MCP tool `get_commands` uses (repo_id from `resolve_project` is a
/// name).
pub(crate) async fn get_project_commands(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<CommandsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Resolve name → uuid, or accept a raw UUID.
    let uuid = if let Some(row) = state.pg.get_project_by_name(&id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        crate::api::util::json_uuid(&row["id"]).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
    } else if let Ok(uid) = uuid::Uuid::parse_str(&id) {
        state.pg.get_project(&uid).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;
        uid
    } else {
        return Err(StatusCode::NOT_FOUND);
    };

    let rows = state.pg.get_project_commands(&uuid, q.category.as_deref()).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"commands": rows, "count": rows.len()})))
}

/// Body for `PUT /api/preferences/commands` — a capability→preferred-tool bias.
#[derive(serde::Deserialize)]
pub(crate) struct CommandPreferenceBody {
    /// Canonical verb (test / build / lint / …).
    capability: String,
    /// Match token for the preferred command (e.g. `nextest`).
    preferred: String,
    note: Option<String>,
}

/// `PUT /api/preferences/commands` — set the user-scope capability→preferred-tool
/// bias that `get_commands` uses to rank the preferred command first (G10).
pub(crate) async fn set_command_preference(
    State(state): State<AppState>,
    Json(body): Json<CommandPreferenceBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let (cap, pref) = (body.capability.trim(), body.preferred.trim());
    if cap.is_empty() || pref.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    state.pg.upsert_command_preference("user", cap, pref, body.note.as_deref()).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "scope": "user", "capability": cap, "preferred": pref })))
}

/// `GET /api/preferences/commands` — the user-scope command preferences.
pub(crate) async fn get_command_preferences(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let preferences = state.pg.command_preferences("user").await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "scope": "user", "preferences": preferences })))
}

pub(crate) async fn get_project_ftr(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let data = state.pg.get_project_ftr(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

/// GET /api/projects/{id}/icon — serve a project's inferred image icon.
///
/// The project-icon pipeline can store `projects.icon = {kind:"image",
/// value:"<repo-relative path>"}` when a repo logo is detected. This streams
/// those bytes so the app can render `<img src=".../icon">` (it falls back to
/// the kanji glyph on any 404). A kanji-kind or absent icon → 404 by design.
///
/// Security: the stored `value` is a repo-relative path, so this is a
/// file-serving endpoint. Path safety is enforced in two layers by
/// `analysis::project_icon`: a pure lexical check (rejects `..`, absolute
/// paths, non-image extensions) and an on-disk canonicalize-inside-root
/// assertion (defeats symlink-out). Anything escaping the repo root → 404.
pub(crate) async fn get_project_icon(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    match serve_project_icon(&state, &id).await {
        Some((content_type, bytes)) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            // A repo logo is effectively immutable between re-scans; a short
            // private cache keeps the Projects list snappy without pinning a
            // stale asset across a re-scan that changes the icon.
            .header(header::CACHE_CONTROL, "private, max-age=300")
            .body(Body::from(bytes))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Resolve + read a project's image icon, or `None` (→ 404) when the icon is
/// not an image, the path is unsafe, or the file is missing. The repo-relative
/// `value` belongs to one of the project's repos, so each repo root is tried in
/// turn; the disk work runs in `spawn_blocking`.
async fn serve_project_icon(state: &AppState, id: &str) -> Option<(&'static str, Vec<u8>)> {
    use crate::analysis::project_icon::{icon_content_type, serve_icon_from_roots};

    // This icon helper maps every failure (incl. a resolver DB error) to None →
    // 404, which is acceptable for a static asset; hence `.ok().flatten()`.
    let uuid = crate::api::util::resolve_project_uuid(state, id).await.ok().flatten()?;
    let project = state.pg.get_project(&uuid).await.ok().flatten()?;
    let icon = &project["icon"];
    if icon.get("kind").and_then(|v| v.as_str()) != Some("image") {
        return None;
    }
    let rel = icon.get("value").and_then(|v| v.as_str())?.to_string();
    // Fail fast on a non-image extension before hitting the DB for repo roots.
    let ext = std::path::Path::new(&rel).extension().and_then(|e| e.to_str())?;
    icon_content_type(ext)?;

    let roots: Vec<String> = state.pg.list_root_folders_by_project(&uuid).await.ok()?
        .iter()
        .filter_map(|r| r.get("abs_path").and_then(|v| v.as_str()).map(String::from))
        .collect();
    if roots.is_empty() {
        return None;
    }
    tokio::task::spawn_blocking(move || serve_icon_from_roots(&roots, &rel))
        .await
        .ok()?
}

/// GET /api/projects/{id}/overview — the Project window · Overview pane
/// (Slot 4). A server-side assembler that composes the project header, top
/// recommendation, stat blocks, and recent sessions into one payload so the
/// pane stays a pure renderer.
pub(crate) async fn get_project_overview(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::project_overview as po;
    use crate::analysis::insight_copy::{copy_or_warm, CopyLimits, FallbackCopy, InsightKind};

    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    let project = state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Stats first — the assembler reads the authoritative 7-day session count
    // from here so the warn rule and the project header agree (one source).
    let stats = state.pg.get_project_overview_stats(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let sessions_7d = stats["sessions7d"].as_i64().unwrap_or(0);

    let ftr = state.pg.get_project_ftr(&uuid).await
        .map_err(|e| { tracing::warn!(error = %e, project = %uuid, "get_project_overview: get_project_ftr failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    // Honest-null when the project has no analyzed sessions in the window — the
    // header renders "—", never a fabricated 0% (`ftr14d` is already null-or-number).
    let ftr_14d: Option<f64> = ftr["ftr14d"].as_f64();

    // The warn rule reads the SAME drift count the stat block displays
    // (`stats.docDrift.open`, status IN drifted/broken) so the warning dot can
    // never disagree with the number the user sees. `get_quality_signals`
    // (a different `status != 'current'` predicate) supplies only the 7-day FTR.
    let open_drift = stats["docDrift"]["open"].as_i64().unwrap_or(0);
    let signals = state.pg.get_quality_signals(&uuid).await
        .map_err(|e| { tracing::warn!(error = %e, project = %uuid, "get_project_overview: get_quality_signals failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    let ftr_7d = signals["ftr_7d"].as_f64().unwrap_or(0.0);
    let warn = po::is_warn(sessions_7d, open_drift, ftr_7d);

    let kanji = po::kanji_from_icon(&project["icon"]);

    // Multi-repo membership: the project's repo folders (git/standalone), not
    // the thousands of nested dirs. The first repo is flagged primary.
    let mut folders: Vec<serde_json::Value> = state.pg.list_folders_by_project(&uuid).await
        .map_err(|e| { tracing::warn!(error = %e, project = %uuid, "get_project_overview: list_folders_by_project failed"); StatusCode::INTERNAL_SERVER_ERROR })?
        .into_iter()
        .filter(|f| matches!(f["kind"].as_str(), Some("git") | Some("standalone")))
        .map(|f| serde_json::json!({ "id": f["id"], "name": f["name"], "role": f["role"] }))
        .collect();
    if let Some(first) = folders.first_mut() {
        first["primary"] = serde_json::json!(true);
    }

    // Hero headline + body come through insight-copy when a top recommendation
    // exists — the model owns the sentence; the code owns the evidence /
    // defaultAcp / action fields (left untouched). All-quiet (top == None) stays
    // static: the pane renders the "Sensei is observing…" copy client-side, and
    // routing a teaching where there is no signal is the spec's wrong-gate.
    // `copy_or_warm` is a wire-path cache read (+ a detached background warm on a
    // miss) — this await never blocks on inference. Mirrors `observatory_today`.
    let top = match state.pg.get_top_recommendation(&uuid).await
        .map_err(|e| { tracing::warn!(error = %e, project = %uuid, "get_project_overview: get_top_recommendation failed"); StatusCode::INTERNAL_SERVER_ERROR })? {
        Some(mut rec) => {
            let facts = serde_json::json!({
                "title":   rec["title"],
                "why":     rec["why"],
                "impact":  rec["impact"],
                "project": project["name"],
            });
            let fallback = FallbackCopy {
                title:  rec["title"].as_str().unwrap_or_default().to_string(),
                detail: rec["why"].as_str().unwrap_or_default().to_string(),
            };
            let copy = copy_or_warm(
                &state.pg, &state.gateway, InsightKind::HeroKoanMature,
                &facts, CopyLimits::default(), fallback,
            ).await;
            rec["title"] = copy.title.into();
            rec["why"] = copy.detail.into();
            Some(rec)
        }
        None => None,
    };
    let recent = state.pg.list_recent_project_sessions_with_role(&uuid, 4).await
        .map_err(|e| { tracing::warn!(error = %e, project = %uuid, "get_project_overview: list_recent_project_sessions_with_role failed"); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok(Json(serde_json::json!({
        "project": {
            "id":         project["id"],
            "name":       project["name"],
            "kanji":      kanji,
            "client":     project["client"],
            "goal":       project["goal"],
            "ftr":        ftr_14d,
            "warn":       warn,
            "sessions7d": sessions_7d,
            "folders":    folders,
        },
        "top_recommendation": top,
        "stats":             stats,
        "recentSessions":    recent,
    })))
}

pub(crate) async fn get_project_repos(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let repos = state.pg.get_project_repos(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "repos": repos })))
}

pub(crate) async fn get_project_drift(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let data = state.pg.get_project_drift(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

pub(crate) async fn get_project_patterns(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let data = state.pg.get_project_patterns(&uuid).await
        .map_err(|e| { tracing::error!(error = %e, project = %uuid, "get_project_patterns failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(data))
}

pub(crate) async fn get_project_libraries(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let libs = state.pg.get_project_libraries(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "libraries": libs })))
}

pub(crate) async fn get_project_instruments(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let tools = state.pg.get_project_extensions(&uuid, Some(&["skill", "command", "agent"])).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "tools": tools })))
}

/// GET /api/projects/{id}/mcp-tool-stats — per-tool call/error/duration/FTR
/// aggregation scoped to a project. Joins the daemon's tool manifests (the
/// full known catalogue) with per-project usage rows so tools with zero
/// calls still appear in the response (with count=0). T2 Slice F.
pub(crate) async fn get_project_mcp_tool_stats(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;

    let stats = state.pg.get_project_mcp_tool_stats(&uuid).await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "get_project_mcp_tool_stats failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Merge with the tool manifest so the response covers every tool the
    // daemon dispatches (not just the ones this project called). Manifest
    // fields (kind, summary) let the UI render a full catalogue with usage
    // overlays without a second daemon round-trip.
    let manifests = crate::api::handlers::mcp_manifests::manifests();
    let mut by_name: std::collections::HashMap<String, &serde_json::Value> = std::collections::HashMap::new();
    for stat in &stats {
        if let Some(name) = stat.get("toolName").and_then(|v| v.as_str()) {
            by_name.insert(name.to_string(), stat);
        }
    }
    let tools: Vec<serde_json::Value> = manifests.iter().map(|m| {
        let empty = serde_json::json!({});
        let usage = by_name.get(m.name).copied().unwrap_or(&empty);
        serde_json::json!({
            "id":            m.id,
            "name":          m.name,
            "mcp":           m.mcp,
            "kind":          m.kind,
            "summary":       m.summary,
            "calls":         usage.get("calls").cloned().unwrap_or(serde_json::json!(0)),
            "errors":        usage.get("errors").cloned().unwrap_or(serde_json::json!(0)),
            "avgDurationMs": usage.get("avgDurationMs").cloned().unwrap_or(serde_json::Value::Null),
            "ftr":           usage.get("ftr").cloned().unwrap_or(serde_json::Value::Null),
            "lastUsedAt":    usage.get("lastUsedAt").cloned().unwrap_or(serde_json::Value::Null),
        })
    }).collect();

    Ok(Json(serde_json::json!({ "tools": tools })))
}

pub(crate) async fn get_project_memories(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let data = state.pg.get_project_memories(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

pub(crate) async fn get_project_recommendations(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<RecoQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::analysis::insight_copy::{copy_or_warm, CopyLimits};
    use crate::insights;

    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let mut recs = state.pg.get_project_recommendations(&uuid, q.status.as_deref()).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Mentor-voice copy (insight-copy) — route each rec's user-facing title +
    // why through the shared pipeline, exactly as `observatory::get_insights`
    // does. Wire path only: a `sensei.insight_copy` cache read plus a detached
    // background warm on a miss — never a blocking model call, so the endpoint
    // never stalls or errors on the gateway (a miss ships the raw DB prose).
    // `insights::rec_copy_inputs` builds the SAME facts the Learnings board
    // uses, so both screens share one `(kind, facts_hash)` cache entry — one
    // warm serves both. Capped at the top COPY_CAP recs (ordered focal/score)
    // to bound the first-load warm burst; the remainder ship their raw prose.
    const COPY_CAP: usize = 8;
    for r in recs.iter_mut().take(COPY_CAP) {
        let (kind, facts, fallback) = insights::rec_copy_inputs(r);
        let copy = copy_or_warm(&state.pg, &state.gateway, kind,
            &facts, CopyLimits::default(), fallback).await;
        insights::apply_rec_copy(r, copy);
    }

    Ok(Json(serde_json::json!(recs)))
}

/// GET /api/projects/{id}/impact — acted-on / consolidation recommendations
/// joined to their reasoning trace (before/after FTR + MOE reasoning). Powers
/// the Observatory Impact view (#70 read-path).
pub(crate) async fn get_project_impact(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    let data = state.pg.get_project_impact(&uuid).await
        .map_err(|e| { tracing::error!(error = %e, project = %uuid, "get_project_impact failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(serde_json::json!(data)))
}

/// GET /api/projects/{id}/library-version-conflicts — per-library version drift
/// across the project's folders (excluding local-protocol deps). Powers the
/// Track 3 Libraries screen "version conflicts" signal.
pub(crate) async fn get_project_library_version_conflicts(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let conflicts = state.pg.list_project_library_version_conflicts(&uuid).await
        .map_err(|e| { tracing::error!(error = %e, project = %uuid, "list_project_library_version_conflicts failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(serde_json::json!({ "conflicts": conflicts })))
}

/// GET /api/projects/{id}/project-deps — outgoing project → project edges
/// detected from local-path protocols (npm link:/workspace:/file:,
/// Cargo path=). Powers the Track 3 Libraries screen "depends on other
/// project" section.
pub(crate) async fn get_project_project_deps(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let deps = state.pg.list_project_dependencies(&uuid).await
        .map_err(|e| { tracing::error!(error = %e, project = %uuid, "list_project_dependencies failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(serde_json::json!({ "dependencies": deps })))
}

pub(crate) async fn get_project_sessions(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SessionsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let limit = q.limit.unwrap_or(50);
    let sessions = state.pg.list_sessions_by_project(&uuid, limit).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "sessions": sessions })))
}

// ── Recommendation accept / reject (Gap 1 fix) ──────────────────────────────
//
// The analyzer writes recommendations in `pending` state, and `MeasureVerdicts`
// only computes an FTR delta for recs that reached `accepted`. Without a UI
// affordance to accept, every rec sits at `pending` forever and the verdict
// column stays empty. These two endpoints close the loop.

/// POST /api/projects/{id}/recommendations/{rec_id}/accept
pub(crate) async fn accept_project_recommendation(
    State(state): State<AppState>,
    Path((_project_id, rec_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let rec_uuid = uuid::Uuid::parse_str(&rec_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.accept_recommendation(&rec_uuid).await
        .map_err(|e| {
            if e.contains("not found") || e.contains("already decided") {
                StatusCode::CONFLICT
            } else {
                tracing::error!(error = %e, rec = %rec_uuid, "accept_recommendation failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    // Close the FTR feedback loop on-demand: accepting a recommendation schedules
    // a MeasureVerdicts follow-up so the before/after impact is captured now, not
    // only on the next periodic analyzer cycle. Verdicts re-measure globally, so
    // the task carries no target args (matches the scheduler's enqueue).
    state.task_queue.enqueue(
        crate::tasks::Task::new(crate::tasks::TaskKind::MeasureVerdicts, "", ""),
    ).await;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Body for materializing a rule-class recommendation (spec 2026-08-20 P-A). All
/// fields optional: `gov_scope` defaults to `project`; `enforcement` to the DB
/// default (`recommended`); `title`/`body` override the rec's `title`/`why`.
#[derive(serde::Deserialize, Default)]
pub(crate) struct MaterializeBody {
    pub gov_scope:    Option<String>,
    pub namespace_id: Option<String>,
    pub enforcement:  Option<String>,
    pub title:        Option<String>,
    pub body:         Option<String>,
}

/// The enforcement tiers a rule may be authored at (advisory < recommended <
/// required < mandatory). Guarded so a typo can't reach the enum cast.
const ENFORCEMENT_TIERS: [&str; 4] = ["advisory", "recommended", "required", "mandatory"];

/// Resolve the governance namespace for a project + scope: `general`/`user` →
/// always-on rung (NULL); an explicit `namespace_id` wins; otherwise the project's
/// repo folder's namespace of that scope. Fail-closed: a specific scope with no
/// bound namespace errors (never silently lands the rule at the broad general rung).
async fn resolve_project_namespace(
    state: &AppState, project_id: &uuid::Uuid, gov_scope: &str, namespace_id: Option<&str>,
) -> Result<Option<uuid::Uuid>, (StatusCode, Json<serde_json::Value>)> {
    let merr = |c: StatusCode, m: &str| (c, Json(serde_json::json!({ "error": m })));
    if let Some(ns) = namespace_id.filter(|s| !s.is_empty()) {
        return Ok(Some(uuid::Uuid::parse_str(ns).map_err(|_| merr(StatusCode::BAD_REQUEST, "bad namespace_id"))?));
    }
    if matches!(gov_scope, "general" | "user") {
        return Ok(None);
    }
    let Some(root) = state.pg.project_root_path(project_id).await
        .map_err(|e| merr(StatusCode::INTERNAL_SERVER_ERROR, &e))?
    else {
        return Err(merr(StatusCode::BAD_REQUEST, "project has no repo folder to resolve a scope against; pass namespace_id or gov_scope=general"));
    };
    let fid = match state.pg.get_folder_ids_by_path(&root).await.map_err(|e| merr(StatusCode::INTERNAL_SERVER_ERROR, &e))? {
        Some((fid, _)) => fid,
        None => return Err(merr(StatusCode::BAD_REQUEST, "project repo folder not found; pass namespace_id or gov_scope=general")),
    };
    match state.pg.namespace_for_folder_scope(&fid, gov_scope).await.map_err(|e| merr(StatusCode::INTERNAL_SERVER_ERROR, &e))? {
        Some(ns) => Ok(Some(ns)),
        None => Err(merr(StatusCode::BAD_REQUEST, &format!(
            "no '{gov_scope}'-scoped namespace bound to this project — pass an explicit namespace_id, or use gov_scope=general"
        ))),
    }
}

/// GET /api/projects/{id}/recommendations/{rec_id}/preview — render WHAT accepting
/// would materialize (rule text + scope/tier, or the full SKILL.md/agent .md +
/// target path) WITHOUT writing anything. The review-before-apply surface for the
/// consent-first accept flow.
pub(crate) async fn preview_recommendation_materialization(
    State(state): State<AppState>,
    Path((_project_id, rec_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::materialize::{ArtifactKind, artifact_path, render_agent_md, render_skill_md, slugify};
    let rec_uuid = uuid::Uuid::parse_str(&rec_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let Some((action_type, title, why, _impact, project_id, _based_on)) =
        state.pg.recommendation_for_materialize(&rec_uuid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    else {
        return Err(StatusCode::CONFLICT); // absent or already decided
    };

    // Rule-class (P-A): a governance rule preview.
    if crate::db::pg_store::PgStore::is_rule_class_action(&action_type) {
        return Ok(Json(serde_json::json!({
            "materializable": true, "kind": "rule", "action_type": action_type,
            "title": title, "body": why, "gov_scope": "project", "enforcement": "recommended",
        })));
    }
    // File-class (P-B): render the exact SKILL.md / agent .md + target path.
    if let Some(kind) = ArtifactKind::from_action(&action_type) {
        let prompt = state.pg.recommendation_prompt(&rec_uuid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let body = prompt.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or(why.trim());
        let slug = slugify(&title, kind.as_str());
        let content = match kind {
            ArtifactKind::Skill => render_skill_md(&slug, &why, body),
            ArtifactKind::Agent => render_agent_md(&slug, &why, body),
        };
        let rel = artifact_path(std::path::Path::new(""), kind, &slug);
        return Ok(Json(serde_json::json!({
            "materializable": true, "kind": kind.as_str(), "action_type": action_type,
            "name": slug, "path": rel.to_string_lossy(), "content": content,
            // A file write is consent-sensitive — the UI confirms before Apply.
            "consent_required": true, "project_id": project_id,
        })));
    }
    Ok(Json(serde_json::json!({
        "materializable": false, "action_type": action_type,
        "reason": "not a materializable action (rule: revise_rule/promote_pattern/enrich_memory; file: write_skill/create_agent)",
    })))
}

/// POST /api/projects/{id}/recommendations/{rec_id}/materialize — accept a
/// recommendation AND produce its durable artifact: a rule-class rec → a governance
/// rule (`sensei.memories`, P-A); a `write_skill`/`create_agent` rec → a project
/// file (`.claude/skills|agents/…`, P-B). Then schedule the before/after FTR
/// measurement. Fail-closed on scope resolution, non-materializable action, and file
/// collision (never clobbers a hand-authored file).
pub(crate) async fn materialize_recommendation(
    State(state): State<AppState>,
    Path((project_id, rec_id)): Path<(String, String)>,
    body: Option<Json<MaterializeBody>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    use crate::materialize::{ArtifactKind, artifact_path, render_agent_md, render_skill_md, slugify, write_artifact};
    let merr = |c: StatusCode, m: &str| (c, Json(serde_json::json!({ "error": m })));
    let rec_uuid = uuid::Uuid::parse_str(&rec_id).map_err(|_| merr(StatusCode::BAD_REQUEST, "bad rec id"))?;
    let pid = uuid::Uuid::parse_str(&project_id).map_err(|_| merr(StatusCode::BAD_REQUEST, "bad project id"))?;
    let b = body.map(|Json(b)| b).unwrap_or_default();

    // Peek the action WITHOUT deciding, so a file collision is caught while the rec
    // is still pending (re-triable) rather than after the accept flip.
    let Some((action_type, title, _why, _impact, _pid, _based_on)) = state.pg
        .recommendation_for_materialize(&rec_uuid).await
        .map_err(|e| merr(StatusCode::INTERNAL_SERVER_ERROR, &e))?
    else {
        return Err(merr(StatusCode::CONFLICT, "recommendation not found or already decided"));
    };

    // ── Rule-class (P-A) ──────────────────────────────────────────────────
    if crate::db::pg_store::PgStore::is_rule_class_action(&action_type) {
        let gov_scope = b.gov_scope.as_deref().filter(|s| !s.is_empty()).unwrap_or("project").to_string();
        let enforcement = b.enforcement.as_deref().map(str::trim).filter(|s| !s.is_empty());
        if let Some(e) = enforcement
            && !ENFORCEMENT_TIERS.contains(&e)
        {
            return Err(merr(StatusCode::BAD_REQUEST, "enforcement must be advisory|recommended|required|mandatory"));
        }
        let namespace_id = resolve_project_namespace(&state, &pid, &gov_scope, b.namespace_id.as_deref()).await?;
        let materialized = state.pg.accept_recommendation_as_rule(
            &rec_uuid, namespace_id.as_ref(), enforcement, &gov_scope, b.title.as_deref(), b.body.as_deref(),
        ).await.map_err(|e| {
            if e.contains("not found") || e.contains("already decided") { merr(StatusCode::CONFLICT, &e) }
            else { tracing::error!(error = %e, rec = %rec_uuid, "materialize rule failed"); merr(StatusCode::INTERNAL_SERVER_ERROR, "materialization failed") }
        })?;
        state.task_queue.enqueue(crate::tasks::Task::new(crate::tasks::TaskKind::MeasureVerdicts, "", "")).await;
        return Ok(Json(serde_json::json!({ "ok": true, "materialized": materialized })));
    }

    // ── File-class (P-B): write a project skill/agent ─────────────────────
    let Some(kind) = ArtifactKind::from_action(&action_type) else {
        return Err(merr(StatusCode::BAD_REQUEST, "recommendation is not materializable (not rule/skill/agent)"));
    };
    // The write target is the project's repo root.
    let Some(repo_root) = state.pg.project_root_path(&pid).await
        .map_err(|e| merr(StatusCode::INTERNAL_SERVER_ERROR, &e))?
    else {
        return Err(merr(StatusCode::BAD_REQUEST, "project has no repo folder to write the file into"));
    };
    let root = std::path::Path::new(&repo_root);
    let slug = slugify(b.title.as_deref().unwrap_or(&title), kind.as_str());
    // Collision check while still pending → the user can rename before accepting.
    if artifact_path(root, kind, &slug).exists() {
        return Err(merr(StatusCode::CONFLICT, &format!(
            "a {} named '{slug}' already exists in this repo; edit the title", kind.as_str()
        )));
    }

    // Flip to accepted (guarded) + get the prompt seed, then write the file.
    let seed = state.pg.begin_file_materialization(&rec_uuid).await
        .map_err(|e| merr(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let Some((_at, _title, seed_why, prompt)) = seed else {
        return Err(merr(StatusCode::CONFLICT, "recommendation not found or already decided"));
    };
    let description = b.body.as_deref().map(str::trim).filter(|s| !s.is_empty())
        .map(str::to_string).unwrap_or(seed_why.clone());
    // Body: the rec's prompt (a create_agent prompt IS an agent spec) else the why.
    let body_text = prompt.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or(seed_why.trim());
    let content = match kind {
        ArtifactKind::Skill => render_skill_md(&slug, &description, body_text),
        ArtifactKind::Agent => render_agent_md(&slug, &description, body_text),
    };
    let rel = match write_artifact(root, kind, &slug, &content) {
        Ok(p) => p,
        Err(e) => {
            // Rec is already accepted (flip committed); the file write failed. Surface
            // it (never a silent swallow) — the ref stays NULL, recoverable by a repair.
            tracing::error!(error = %e, rec = %rec_uuid, "materialize file failed after accept flip");
            let code = if e.contains("already exists") { StatusCode::CONFLICT } else { StatusCode::INTERNAL_SERVER_ERROR };
            return Err(merr(code, &e));
        }
    };
    let materialized = serde_json::json!({
        "kind": kind.as_str(), "file_path": rel.to_string_lossy(), "repo_root": repo_root, "name": slug,
    });
    if let Err(e) = state.pg.set_recommendation_materialized(&rec_uuid, &materialized).await {
        tracing::error!(error = %e, rec = %rec_uuid, "set materialized_ref failed (file already written)");
    }
    state.task_queue.enqueue(crate::tasks::Task::new(crate::tasks::TaskKind::MeasureVerdicts, "", "")).await;
    Ok(Json(serde_json::json!({ "ok": true, "materialized": materialized })))
}

/// POST /api/projects/{id}/recommendations/{rec_id}/reject
pub(crate) async fn reject_project_recommendation(
    State(state): State<AppState>,
    Path((_project_id, rec_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let rec_uuid = uuid::Uuid::parse_str(&rec_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.reject_recommendation(&rec_uuid).await
        .map_err(|e| {
            if e.contains("not found") || e.contains("already decided") {
                StatusCode::CONFLICT
            } else {
                tracing::error!(error = %e, rec = %rec_uuid, "reject_recommendation failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Doc drift scan (T3 Slice 2.3) ───────────────────────────────────────────

/// POST /api/projects/{id}/drift/scan — run the doc-drift detector on the
/// project. Returns `{ scannedDocs, newBroken, resolved }` so the UI can
/// flash a "found N drift signals" notice after triggering.
pub(crate) async fn scan_project_doc_drift(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let summary = state.pg.scan_project_doc_drift(&uuid).await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "scan_project_doc_drift failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(summary))
}

/// GET /api/projects/{id}/health — the weighted 0–100 health score + per-metric 0–5
/// ratings (radar spokes) + the components map, from the rating views. Honest-empty
/// score (null) when nothing is rated yet.
pub(crate) async fn get_project_health(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let health = state.pg.get_project_health(&uuid).await.map_err(|e| {
        tracing::error!(error = %e, project = %uuid, "get_project_health failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(health))
}

/// GET /api/metrics/correlations — the PORTFOLIO view, across every project.
///
/// Usually the more useful of the two: a correlation needs both metrics on the
/// same day repeatedly, and per project that rarely accumulates. Pooling projects
/// buys sample size, so this describes how the SIGNALS relate rather than how one
/// codebase behaves.
pub(crate) async fn get_portfolio_correlations(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let out = state.pg.get_metric_correlations(None).await.map_err(|e| {
        tracing::error!(error = %e, "get_portfolio_correlations failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(out))
}

/// GET /api/projects/{id}/correlations — which metrics move together.
///
/// Pairs related BY CONSTRUCTION (the registry's `derives_from`) are omitted, not
/// ranked lower: `tokens_in_per_day` vs `tokens_per_day` correlates 1.00 because
/// the second contains the first, which is arithmetic rather than a finding. Each
/// result carries its sample size so a caller can weight the claim.
pub(crate) async fn get_metric_correlations(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let out = state.pg.get_metric_correlations(Some(&uuid)).await.map_err(|e| {
        tracing::error!(error = %e, project = %uuid, "get_metric_correlations failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(out))
}

#[derive(Deserialize)]
pub(crate) struct CoverageBackfillQuery {
    /// How many of the most-recent sampled ISO-week anchors to walk (omit = all).
    weeks: Option<u32>,
}

/// POST /api/projects/{id}/coverage/backfill — the EXPLICIT opt-in coverage backfill
/// (spec: "backfilling can be configured or explicitly requested"). Reconstructs
/// historical coverage by checking out sampled past commits, running the configured
/// `metrics.coverage_command`, and ingesting the produced lcov.
///
/// It RUNS the project's test suite per commit — the longest operation the daemon
/// performs — so this endpoint ENQUEUES `BackfillCoverage` and returns; the queue
/// owns the execution. It used to `tokio::spawn` the work detached, which put the
/// heaviest job in the system outside every guarantee the queue provides: no
/// dedup (two clicks ran two test suites over the same repo at once), no
/// visibility in task status, no retry, and silent loss on daemon restart.
///
/// Deduped per project, so a second request while one is in flight reports the
/// in-flight run instead of stacking another. A no-op when
/// `metrics.coverage_command` is unset (the daemon never runs tests unless configured).
pub(crate) async fn coverage_backfill(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<CoverageBackfillQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let project = uuid.to_string();
    let kind = crate::tasks::TaskKind::BackfillCoverage;
    if state.task_queue.has_pending_kind_path(kind.clone(), &project).await {
        return Ok(Json(
            serde_json::json!({ "queued": false, "running": true, "project": uuid }),
        ));
    }
    // folder_path carries the week bound; the handler parses it back.
    let weeks = q.weeks.map(|w| w.to_string()).unwrap_or_default();
    let task_id = state.task_queue.enqueue(crate::tasks::Task::new(kind, &weeks, &project)).await;
    // The id is the whole point of returning early: the caller follows the work on
    // GET /api/tasks/progress (SSE) or polls GET /api/tasks/status. A request that
    // instead held the connection open for a test-suite-per-commit run would be
    // killed by any client/proxy timeout while the work carried on unobserved.
    Ok(Json(
        serde_json::json!({ "queued": true, "taskId": task_id, "project": uuid, "weeks": q.weeks }),
    ))
}

// ── Service scoping (T2 Slice B) ────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct ServiceScopeBody {
    enabled: bool,
}

/// GET /api/projects/{id}/services — installed services with per-project
/// scope resolved (scoped override wins, then global, then default true).
pub(crate) async fn list_project_services(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let services = state.pg.list_services_with_project_scope(&uuid).await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "list_services_with_project_scope failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({ "services": services })))
}

/// PUT /api/projects/{id}/services/{service_id}/scope — toggle a service's
/// enabled state for this project. Body: `{ enabled: bool }`.
pub(crate) async fn set_project_service_scope(
    State(state): State<AppState>,
    Path((id, service_id)): Path<(String, String)>,
    Json(body): Json<ServiceScopeBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_uuid = crate::api::util::resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    let service_uuid = uuid::Uuid::parse_str(&service_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&project_uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    state.pg
        .set_service_project_scope(&service_uuid, Some(&project_uuid), body.enabled)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, project = %project_uuid, service = %service_uuid, "set_service_project_scope failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Memory share batches ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct BatchListQuery {
    status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct BatchCreateBody {
    memory_ids: Vec<uuid::Uuid>,
    note: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct BatchDecisionBody {
    status: String,
    note: Option<String>,
}

/// GET /api/projects/{id}/memory-batches?status=proposed
pub(crate) async fn list_memory_share_batches(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<BatchListQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let batches = state.pg.list_memory_share_batches(&uuid, q.status.as_deref()).await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "list_memory_share_batches failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({ "batches": batches })))
}

/// POST /api/projects/{id}/memory-batches
pub(crate) async fn create_memory_share_batch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<BatchCreateBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    if body.memory_ids.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let batch_id = state.pg
        .create_memory_share_batch(&uuid, &body.memory_ids, body.note.as_deref())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "create_memory_share_batch failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({ "id": batch_id })))
}

// ── Impact verdicts (manual log) ────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct ImpactListQuery {
    verdict: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ImpactCreateBody {
    title: String,
    note: Option<String>,
    session_id: Option<uuid::Uuid>,
}

#[derive(Deserialize)]
pub(crate) struct ImpactDecisionBody {
    verdict: String,
    note: Option<String>,
}

/// GET /api/projects/{id}/impact-verdicts?verdict=success
pub(crate) async fn list_impact_verdicts(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ImpactListQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let verdicts = state.pg.list_impact_verdicts(&uuid, q.verdict.as_deref()).await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "list_impact_verdicts failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({ "verdicts": verdicts })))
}

/// POST /api/projects/{id}/impact-verdicts
pub(crate) async fn create_impact_verdict(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ImpactCreateBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_existing_project(&state, &id).await?;
    let verdict_id = state.pg
        .create_impact_verdict(&uuid, &body.title, body.note.as_deref(), body.session_id.as_ref())
        .await
        .map_err(|e| {
            if e.contains("title required") {
                StatusCode::BAD_REQUEST
            } else {
                tracing::error!(error = %e, project = %uuid, "create_impact_verdict failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(serde_json::json!({ "id": verdict_id })))
}

/// PUT /api/projects/{id}/impact-verdicts/{verdict_id}
pub(crate) async fn decide_impact_verdict(
    State(state): State<AppState>,
    Path((id, verdict_id)): Path<(String, String)>,
    Json(body): Json<ImpactDecisionBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _project_uuid = crate::api::util::resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    let verdict_uuid = uuid::Uuid::parse_str(&verdict_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg
        .set_impact_verdict_outcome(&verdict_uuid, &body.verdict, body.note.as_deref())
        .await
        .map_err(|e| {
            if e.contains("not found") || e.contains("already decided") {
                StatusCode::CONFLICT
            } else if e.contains("invalid verdict") {
                StatusCode::BAD_REQUEST
            } else {
                tracing::error!(error = %e, verdict = %verdict_uuid, "set_impact_verdict_outcome failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// PUT /api/projects/{id}/memory-batches/{batch_id}
pub(crate) async fn decide_memory_share_batch(
    State(state): State<AppState>,
    Path((id, batch_id)): Path<(String, String)>,
    Json(body): Json<BatchDecisionBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _project_uuid = crate::api::util::resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    let batch_uuid = uuid::Uuid::parse_str(&batch_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg
        .set_memory_share_batch_status(&batch_uuid, &body.status, body.note.as_deref())
        .await
        .map_err(|e| {
            if e.contains("not found") || e.contains("already decided") {
                StatusCode::CONFLICT
            } else if e.contains("invalid status") {
                StatusCode::BAD_REQUEST
            } else {
                tracing::error!(error = %e, batch = %batch_uuid, "set_memory_share_batch_status failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
