use crate::api::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Timelike;
use serde::Deserialize;

// Re-use TagBody from workspace
use super::workspace::TagBody;
// One canonical name-or-uuid resolver (was duplicated here — see the #109 audit).
// It returns Err(500) on a DB error and Ok(None) on a genuine miss, so callers
// distinguish an outage from an unknown project instead of masking both as 404.
use crate::api::util::resolve_project_uuid;

// ── Solutions CRUD ──────────────────────────────────────────────────────────

/// Query params for [`list_solutions`]. `under` scopes the list to projects
/// whose folders live under an absolute path (the sensei MCP `find_projects`
/// tool sends it); omitted → every project (unchanged).
#[derive(Deserialize)]
pub(crate) struct ListSolutionsQuery {
    #[serde(default)]
    under: Option<String>,
}

pub(crate) async fn list_solutions(
    State(state): State<AppState>,
    Query(q): Query<ListSolutionsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let under = q.under.as_deref().filter(|s| !s.is_empty());
    let projects =
        state.pg.list_projects_under(under).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Enrich each project with its folder membership so the Projects setup
    // page (and any future project detail view) can render folder names,
    // paths, and roles without an extra round trip per project.
    //
    // Folder-scope matters here: the `?under=` call is the MCP `find_projects`
    // discovery path, which only needs the repo-root folders — attaching the
    // hundreds of nested `kind:'folder'` descendants blew the response past the
    // MCP client's token cap (~72K chars for sensei). So when `under` is set we
    // send only the compact repo roots (still enough for cwd→project
    // resolution). The un-scoped app path (`GET /api/projects`, no `under`)
    // keeps the full folder tree — unchanged.
    let compact = under.is_some();
    let mut enriched = Vec::with_capacity(projects.len());
    for mut project in projects {
        // A uuid read back out of a DB row, NOT the `{id}` of a route — this
        // handler lists projects and takes no path parameter. Named `row_*` to
        // say so: a route `{id}` is name-or-uuid and must go through
        // `resolve_project_uuid`, and the #100 guard keys on that distinction.
        let row_project_id = project["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok());
        if let Some(pid) = row_project_id {
            // Fail closed: a folder-enrichment read error is a 500, not a
            // silently folder-less project (which reads as "this project has no
            // repos"). A genuinely empty result stays an empty array.
            let folders = if compact {
                state
                    .pg
                    .list_root_folders_by_project(&pid)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            } else {
                state
                    .pg
                    .list_folders_by_project(&pid)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            };
            project["folders"] = serde_json::Value::Array(folders);
        } else {
            project["folders"] = serde_json::Value::Array(vec![]);
        }
        enriched.push(project);
    }
    Ok(Json(serde_json::json!(enriched)))
}

#[derive(Deserialize)]
pub(crate) struct CreateSolutionBody {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    client: Option<String>,
    // TODO: category and repos unused until maturity + repo membership is modeled
    #[serde(default = "default_category")]
    #[allow(dead_code)]
    category: String,
    #[serde(default)]
    #[allow(dead_code)]
    repos: Vec<CreateProjectRepo>,
}

#[derive(Deserialize)]
pub(crate) struct CreateProjectRepo {
    repo_id: String,
    #[serde(default = "default_role")]
    role: String,
    label: Option<String>,
}

fn default_category() -> String {
    "active".to_string()
}
fn default_role() -> String {
    "unknown".to_string()
}

pub(crate) async fn create_solution(
    State(state): State<AppState>,
    Json(body): Json<CreateSolutionBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let id = state
        .pg
        .create_project(&body.name, body.description.as_deref(), body.client.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // TODO: repo-to-project membership not yet modeled.

    Ok((StatusCode::CREATED, Json(serde_json::json!({"ok": true, "id": id}))))
}

/// A jsonb project field from the request body: present as an object or array
/// (icon/stack are objects, links an array). A scalar or `null` is treated as
/// "not provided" so a malformed value can't clobber a good column.
fn jsonb_field<'a>(body: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    body.get(key).filter(|v| v.is_object() || v.is_array())
}

/// PUT /api/projects/{id} — update a project's editable identity. Persists the
/// full About-form field set (name/description/maturity/client/goal/
/// preferred_acp + icon/stack/links jsonb); omitted fields are left unchanged
/// (partial-update). An unknown `maturity` is rejected with 400 before the DB
/// write rather than surfacing the Postgres enum cast as a 500.
pub(crate) async fn update_solution(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::db::pg_store::{PROJECT_MATURITIES, ProjectPatch};

    // Name-or-uuid: resolve so `PUT /api/projects/sensei` works, not only a uuid
    // (#100). 404 when no such project.
    let project_id = resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;

    // Validate the only enum-backed field (maturity) up front → 400, not 500.
    // client/goal/preferred_acp are free text; icon/stack/links are jsonb.
    let maturity = body.get("maturity").and_then(|v| v.as_str());
    if let Some(m) = maturity
        && !PROJECT_MATURITIES.contains(&m)
    {
        tracing::warn!(maturity = m, "update_solution rejected unknown maturity");
        return Err(StatusCode::BAD_REQUEST);
    }

    let patch = ProjectPatch {
        name: body.get("name").and_then(|v| v.as_str()),
        description: body.get("description").and_then(|v| v.as_str()),
        maturity,
        client: body.get("client").and_then(|v| v.as_str()),
        goal: body.get("goal").and_then(|v| v.as_str()),
        preferred_acp: body.get("preferred_acp").and_then(|v| v.as_str()),
        icon: jsonb_field(&body, "icon"),
        stack: jsonb_field(&body, "stack"),
        links: jsonb_field(&body, "links"),
    };

    state.pg.update_project(&project_id, &patch).await.map_err(|e| {
        tracing::error!(error = %e, "update_solution failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({"ok": true})))
}

pub(crate) async fn delete_solution(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    state
        .pg
        .delete_project(&project_id)
        .await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(serde::Deserialize)]
pub(crate) struct MergeProjectsBody {
    /// Project(s) whose folders + sessions + memories fold into `target`.
    /// Empty is an error. Deduped by the handler before running the merge.
    pub sources: Vec<String>,
    /// Destination project id. Must not appear in `sources`.
    pub target: String,
}

/// POST /api/projects/merge — merge one or more source projects into a
/// target project (#41). Derived signals (patterns / recommendations /
/// reasoning_traces / impact_verdicts) on the source projects are dropped
/// via ON DELETE CASCADE and regenerated by the analyzer on the next tick
/// over the merged corpus.
///
/// Returns `{ok: true, merged: <count>}` on success; per-source failures
/// short-circuit and roll back that source's transaction only (previously
/// merged sources stay merged).
pub(crate) async fn merge_projects(
    State(state): State<AppState>,
    Json(body): Json<MergeProjectsBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let target = uuid::Uuid::parse_str(&body.target).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "target is not a valid uuid"})))
    })?;

    // Parse + dedupe + reject self-references up front so the DB layer only
    // sees clean input.
    let mut sources: Vec<uuid::Uuid> = Vec::with_capacity(body.sources.len());
    for s in &body.sources {
        let uuid = uuid::Uuid::parse_str(s).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("source '{}' is not a valid uuid", s)})),
            )
        })?;
        if uuid == target {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "target must not appear in sources"})),
            ));
        }
        if !sources.contains(&uuid) {
            sources.push(uuid);
        }
    }
    if sources.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "sources must not be empty"})),
        ));
    }

    let mut merged = 0u32;
    for src in &sources {
        state.pg.merge_projects(src, &target).await.map_err(|e| {
            tracing::error!(error = %e, source = %src, target = %target, "merge_projects failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": e,
                    "merged_before_failure": merged,
                })),
            )
        })?;
        merged += 1;
    }

    Ok(Json(serde_json::json!({ "ok": true, "merged": merged })))
}

pub(crate) async fn add_solution_repo(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<CreateProjectRepo>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;

    // Look up the folder by name (old string repo_id)
    let folder = state
        .pg
        .get_repo_by_name(&body.repo_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let folder_id =
        crate::api::util::json_uuid(&folder["id"]).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    state
        .pg
        .set_folder_project(&folder_id, &project_id, &body.role, body.label.as_deref())
        .await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn remove_solution_repo(
    State(state): State<AppState>,
    Path((_id, repo_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Clear folder-project association by setting props to remove project link
    let folder = state
        .pg
        .get_repo_by_name(&repo_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let folder_id =
        crate::api::util::json_uuid(&folder["id"]).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Clear the project association by setting project_id to null via props
    state
        .pg
        .set_folder_props(&folder_id, &serde_json::json!({"project_id": null}))
        .await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn add_solution_tag(
    State(state): State<AppState>,
    Path(_id): Path<String>,
    Json(body): Json<TagBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // PgStore tags are a controlled vocabulary. Register in vocabulary.
    state
        .pg
        .add_tag(&body.tag, Some("solution"))
        .await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn remove_solution_tag(
    State(state): State<AppState>,
    Path((_id, tag)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // PgStore tags are a controlled vocabulary. Remove from vocabulary.
    state
        .pg
        .remove_tag(&tag)
        .await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Solution Analysis ───────────────────────────────────────────────────────

pub(crate) async fn analyze_solution(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    let project = state
        .pg
        .get_project(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // On-demand analyzer trigger (#67): enqueue session enrichment / analysis
    // for this project (the scheduler does this periodically). Task::new(kind,
    // folder_path, path) — the handler reads the project id from `path`.
    state
        .task_queue
        .enqueue(crate::tasks::Task::new(crate::tasks::TaskKind::AnalyzeProject, "", &id))
        .await;

    // TODO: implement full cross-repo analysis
    Ok(Json(serde_json::json!({
        "project": project,
        "links": [],
        "shared_libs": [],
    })))
}

/// `POST /api/projects/{id}/process/analyze` — on-demand LLM process-quality pass
/// (spec 2026-08-20): enqueue `AnalyzeSessionProcess` for this project so its
/// un-scored sessions get spec-depth/deviation + refuted-findings +
/// incomplete-analysis judgments (the scheduler also runs this daily). The task is
/// watermark-gated + batch-capped, so repeated calls drain the backlog
/// incrementally. Fail-closed: 404 for an unknown project. Returns `{ queued }`.
pub(crate) async fn analyze_process(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    state
        .task_queue
        .enqueue(crate::tasks::Task::new(
            crate::tasks::TaskKind::AnalyzeSessionProcess,
            "",
            &project_id.to_string(),
        ))
        .await;
    Ok(Json(serde_json::json!({ "ok": true, "queued": true })))
}

/// `POST /api/projects/{id}/backfill` — RE-DERIVE this project's session signals
/// from scratch: clear `analyzed_at` (so every captured session re-enriches with
/// the CURRENT transcript-ground-truth logic) and enqueue `AnalyzeProject` (which
/// re-enriches them and chains a metrics recompute). Distinct from
/// [`backfill_metrics`] (day-history recovery) — this refreshes the SESSION
/// derivations, so it is the button to press after a metric-definition change.
/// Returns `{ reset, queued }`. Fail-closed: 404 for an unknown project, 500 on a
/// read/write error.
pub(crate) async fn backfill_project_sessions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    state
        .pg
        .get_project(&uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let reset = state
        .pg
        .reset_project_sessions_for_reenrichment(&uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .task_queue
        .enqueue(crate::tasks::Task::for_project(crate::tasks::TaskKind::AnalyzeProject, &uuid))
        .await;
    Ok(Json(serde_json::json!({ "reset": reset, "queued": true })))
}

/// Optional lower bound for a capture ingest.
#[derive(Deserialize)]
pub(crate) struct CaptureIngestQuery {
    /// Only ingest units changed on or after this day. Omitted = everything.
    from: Option<chrono::NaiveDate>,
}

/// Enqueue a transcript backfill (#73) — ingest assistant/user prose from the
/// agent transcript caches into activity.transcript_turns. Resumable, so this
/// is safe to call repeatedly; only changed transcripts do work.
///
/// ENQUEUES rather than running the backfill inline. The dispatcher walks every
/// transcript the adapters can see — ~2,700 files on this machine — and running
/// that on the request thread meant the caller held a connection open across a
/// filesystem sweep it could not observe, and any client timeout abandoned a
/// sweep that kept running. Follow the work on `GET /api/tasks/progress` (SSE)
/// or `GET /api/tasks/status`.
///
/// Deduped: a second request while one is in flight reports the in-flight run.
pub(crate) async fn ingest_captures(
    State(state): State<AppState>,
    Query(q): Query<CaptureIngestQuery>,
) -> Json<serde_json::Value> {
    let kind = crate::tasks::TaskKind::IngestCaptures;
    if state.task_queue.has_pending_kind(kind.clone()).await {
        return Json(serde_json::json!({ "ok": true, "queued": false, "running": true }));
    }
    // `?from=YYYY-MM-DD` bounds the walk; omitted ingests everything. Same kind,
    // same handler — the range is a parameter, which is why there is no separate
    // "backfill" task to keep in sync with the normal one.
    let mut task = crate::tasks::Task::new(kind, "", "");
    task.as_of = q.from;
    let task_id = state.task_queue.enqueue(task).await;
    Json(serde_json::json!({ "ok": true, "queued": true, "taskId": task_id, "from": q.from }))
}

/// Enqueue a metrics backfill (Phase 5 — history recovery): one `ComputeProjectMetrics` per
/// project. The planner then backfills every data day its sources reach and recomputes
/// today, so the metric charts render months of history. Overlap-guarded
/// (`has_pending_kind`), so this is safe to call repeatedly; a re-plan is idempotent
/// (per-day upserts). Mirrors [`ingest_captures`]. A project-list read failure is
/// a 500 — never masked into a fake `enqueued: 0` (which would read as "no projects").
pub(crate) async fn backfill_metrics(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let enqueued =
        crate::tasks::metrics_scheduler::enqueue_backfill_all(&state.task_queue, &state.pg)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "ok": true, "enqueued": enqueued })))
}

// ── Per-Repo Summary ────────────────────────────────────────────────────────

pub(crate) async fn project_summary(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Resolve to project-scoped folder ids; fall back to NOT_FOUND only if
    // neither a project name/UUID nor a repo name matches.
    let ids =
        state.pg.scope_folder_ids(&repo_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if ids.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Derive counts across all folders in scope. A DB error is a 500 — never
    // masked as 0 counts (which would read as "this project has no code").
    let counts = state
        .pg
        .count_nodes_by_kind_scoped(&ids)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let fn_count =
        counts.get("function").copied().unwrap_or(0) + counts.get("method").copied().unwrap_or(0);
    let type_count = counts.get("class").copied().unwrap_or(0)
        + counts.get("struct").copied().unwrap_or(0)
        + counts.get("interface").copied().unwrap_or(0)
        + counts.get("enum").copied().unwrap_or(0)
        + counts.get("type").copied().unwrap_or(0);
    let edge_count =
        state.pg.count_edges_scoped(&ids).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let pkg_count = counts.get("package").copied().unwrap_or(0);
    let mod_count = counts.get("module").copied().unwrap_or(0);

    // Resolve name/path: prefer project row if repo_id is a project name/UUID,
    // else fall back to the first (root) folder row.
    // Resolve the REAL project/folder row. `repo_id` may be a project name, a
    // project UUID, or a repo name. When it's a UUID the two name lookups miss, so
    // look the project up by id. NEVER fabricate: no returning the UUID as the
    // name, no hardcoded status — if nothing resolves (the scope came from an
    // orphaned folder set), 404.
    let project = match state.pg.get_project_by_name(&repo_id).await {
        Ok(Some(p)) => Some(p),
        Ok(None) => match uuid::Uuid::parse_str(&repo_id) {
            Ok(uuid) => {
                state.pg.get_project(&uuid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            }
            Err(_) => None,
        },
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    let (name, path, stack, libs, tags, status, indexed_at) = if let Some(proj) = project {
        (
            proj["name"].clone(),
            proj.get("path").cloned().unwrap_or(serde_json::Value::Null),
            proj.get("stack").cloned().unwrap_or(serde_json::json!([])),
            serde_json::json!([]),
            proj.get("tags").cloned().unwrap_or(serde_json::json!([])),
            // Real lifecycle (project_maturity), not a hardcoded "active".
            proj.get("maturity").cloned().unwrap_or(serde_json::Value::Null),
            serde_json::Value::Null,
        )
    } else if let Ok(Some(folder)) = state.pg.get_repo_by_name(&repo_id).await {
        (
            folder["name"].clone(),
            folder["abs_path"].clone(),
            folder.get("stack").cloned().unwrap_or(serde_json::json!([])),
            folder.get("libs").cloned().unwrap_or(serde_json::json!([])),
            folder.get("tags").cloned().unwrap_or(serde_json::json!([])),
            // Real folder status, honest-null if unset — not a fabricated "active".
            folder.get("status").cloned().unwrap_or(serde_json::Value::Null),
            folder.get("indexed_at").cloned().unwrap_or(serde_json::Value::Null),
        )
    } else {
        return Err(StatusCode::NOT_FOUND);
    };

    Ok(Json(serde_json::json!({
        "repoId": name,
        "name": name,
        "path": path,
        "stack": stack,
        "libs": libs,
        "tags": tags,
        "status": status,
        "indexedAt": indexed_at,
        "functions": fn_count,
        "types": type_count,
        "packages": pkg_count,
        "modules": mod_count,
        "edges": edge_count,
        "solutions": [],
    })))
}

// ── Solution Graph & Roles ──────────────────────────────────────────────────

pub(crate) async fn solution_graph(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    let project = state
        .pg
        .get_project(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let project_name = project["name"].as_str().unwrap_or("unknown");

    // Get all repos (folders) and filter those belonging to this project.
    // Fail closed on a read error (500) — an empty repo list here would render
    // as an empty solution graph, indistinguishable from a real single-node one.
    let all_repos =
        state.pg.list_repositories().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let project_repos: Vec<&serde_json::Value> =
        all_repos.iter().filter(|r| r["project_id"].as_str() == Some(&id)).collect();

    let mut all_nodes = Vec::new();
    let mut all_edges = Vec::new();

    // Collect folder ids for all member repos, preserving per-repo metadata.
    let mut folder_ids: Vec<uuid::Uuid> = Vec::new();
    let mut folder_id_to_repo: std::collections::HashMap<uuid::Uuid, (&serde_json::Value, String)> =
        std::collections::HashMap::new();
    let mut seen_repo_ids = std::collections::HashSet::new();
    for repo in &project_repos {
        let repo_name = repo["name"].as_str().unwrap_or("").to_string();
        if !seen_repo_ids.insert(repo_name.clone()) {
            continue;
        }
        if let Some(fid) = crate::api::util::json_uuid(&repo["id"]) {
            folder_ids.push(fid);
            folder_id_to_repo.insert(fid, (repo, repo_name));
        }
    }

    // Fetch nodes and edges in one scoped call each. Fail closed on a read
    // error (500) — a swallowed error would silently drop nodes/edges and render
    // a truncated graph as if it were complete.
    let scoped_nodes = state
        .pg
        .get_nodes_scoped(&folder_ids)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let scoped_edges = state
        .pg
        .get_edges_scoped(&folder_ids, "calls")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Re-annotate nodes with repoId/role from folder metadata.
    // get_nodes_scoped returns folder_id — use it to look up the repo.
    for node in scoped_nodes {
        let fid_opt = node
            .get("folder_id")
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok());
        let (repo_name, role) = fid_opt
            .and_then(|fid| folder_id_to_repo.get(&fid))
            .map(|(r, rn)| (rn.as_str(), r["role"].as_str().unwrap_or("unknown")))
            .unwrap_or(("", "unknown"));
        all_nodes.push(serde_json::json!({
            "id": node["id"], "name": node["name"], "kind": node["kind"],
            "file": node["file_path"], "line": node["line_start"],
            "complexity": node.get("complexity"),
            "doc_type": node.get("doc_type"), "level": node.get("level"),
            "parent_id": node["parent_id"],
            "repoId": repo_name, "role": role,
        }));
    }
    for edge in scoped_edges {
        // edges don't carry folder_id in the scoped result shape; repoId is best-effort
        all_edges.push(serde_json::json!({
            "source": edge["source_id"], "target": edge["target_id"],
            "type": "calls",
        }));
    }

    // Inject project-level hierarchy: soln -> repo nodes for each member
    let soln_node_id = format!("soln:{}", id);
    all_nodes.push(serde_json::json!({
        "id": &soln_node_id, "name": project_name, "kind": "solution",
        "file": "", "line": 0, "complexity": null,
    }));
    for repo in &project_repos {
        let repo_name = repo["name"].as_str().unwrap_or("");
        let repo_node_id = format!("repo:{}", repo_name);
        if !all_nodes.iter().any(|n| n.get("id").and_then(|v| v.as_str()) == Some(&repo_node_id)) {
            let label = repo["label"].as_str().unwrap_or(repo_name);
            let abs_path = repo["abs_path"].as_str().unwrap_or("");
            let role = repo["role"].as_str().unwrap_or("unknown");
            all_nodes.push(serde_json::json!({
                "id": &repo_node_id, "name": label, "kind": "repo",
                "file": abs_path, "line": 0, "complexity": null,
                "role": role,
            }));
        }
        all_edges.push(serde_json::json!({
            "source": &soln_node_id, "target": &repo_node_id, "type": "CONTAINS_REPO",
        }));
    }

    Ok(Json(serde_json::json!({
        "solutionId": id,
        "name": project_name,
        "nodes": all_nodes.len(),
        "edges": all_edges.len(),
        "repos": project_repos.len(),
        "graph": {"nodes": all_nodes, "edges": all_edges},
    })))
}

pub(crate) async fn solution_roles(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    // Verify project exists
    state
        .pg
        .get_project(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get repos belonging to this project. Fail closed on a read error (500) —
    // an empty list would render as "this solution has no repos / roles".
    let all_repos =
        state.pg.list_repositories().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let project_repos: Vec<serde_json::Value> =
        all_repos.into_iter().filter(|r| r["project_id"].as_str() == Some(&id)).collect();

    // Build simple role list from folder data
    let roles: Vec<serde_json::Value> = project_repos
        .iter()
        .map(|r| {
            serde_json::json!({
                "repoId": r["name"],
                "role": r.get("role").and_then(|v| v.as_str()).unwrap_or("unknown"),
                "label": r.get("label").and_then(|v| v.as_str()),
            })
        })
        .collect();

    Ok(Json(serde_json::json!(roles)))
}

// ── Metrics ─────────────────────────────────────────────────────────────────

pub(crate) async fn get_metrics(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Metrics are computed from session data in PgStore. Look up the folder to
    // get its UUID, then query sessions. Fail closed: a lookup error is a 500
    // (never masked as a miss); a genuine miss is a 404 — NOT a 200 carrying a
    // fabricated {"error":"project not found"} body a caller can't distinguish
    // from real metrics.
    let folder = state
        .pg
        .get_repo_by_name(&project)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let folder_id =
        crate::api::util::json_uuid(&folder["id"]).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let sessions = state.pg.list_sessions_by_folder(&folder_id, 100).await
        .map_err(|e| { tracing::warn!(error = %e, project = %project, "get_metrics: list_sessions_by_folder failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    let session_count = sessions.len();
    let completed = sessions.iter().filter(|s| s["outcome"].as_str() == Some("completed")).count();
    // FTR is store-backed (project_metrics, metric='ftr') — the SAME number the
    // Phase-7 endpoints serve. Honest-absent (null) when the folder isn't
    // attached to a project or the project has no ftr rows in the window; NEVER a
    // fabricated 0. A read error is a 500, never masked.
    let ftr: Option<f64> = match crate::api::util::json_uuid(&folder["project_id"]) {
        Some(pid) => state.pg.get_project_ftr_rate(&pid).await
            .map_err(|e| { tracing::warn!(error = %e, project = %project, "get_metrics: get_project_ftr_rate failed"); StatusCode::INTERNAL_SERVER_ERROR })?,
        None => None,
    };
    Ok(Json(serde_json::json!({
        "project": project,
        "sessions": session_count,
        "completed": completed,
        "ftr": ftr,
    })))
}

// ── Observatory Chart Data ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct DaysQuery {
    #[serde(default = "default_days")]
    days: i32,
}
fn default_days() -> i32 {
    14
}

#[derive(Deserialize)]
pub(crate) struct LimitQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}
fn default_limit() -> i64 {
    10
}

/// GET /api/observatory/ftr-daily?days=14 — holistic FTR sparkline (all projects)
pub(crate) async fn holistic_ftr_daily(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<DaysQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let data = state.pg.get_ftr_daily(None, q.days).await.map_err(|e| {
        tracing::error!("ftr_daily error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({ "ftr_daily": data })))
}

/// GET /api/projects/{id}/ftr-daily?days=14 — per-project FTR sparkline
pub(crate) async fn project_ftr_daily(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<DaysQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    let data = state
        .pg
        .get_ftr_daily(Some(&project_id), q.days)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "ftr_daily": data })))
}

/// GET /api/projects/{id}/hotspots?days=7 — files with highest rework
pub(crate) async fn project_hotspots(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<DaysQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    let data = state
        .pg
        .get_hotspots(&project_id, q.days)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "hotspots": data })))
}

/// GET /api/projects/{id}/quality-signals — 4 quality indicators
pub(crate) async fn project_quality_signals(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    let data = state
        .pg
        .get_quality_signals(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

/// GET /api/projects/{id}/maturity — the derived early/mature signal (#71)
/// plus watched/target progress. Not the stored `projects.maturity` lifecycle
/// enum; computed on read from enriched-session count + insight presence.
pub(crate) async fn project_maturity(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    let (watched, has_insights) = state
        .pg
        .get_project_maturity_inputs(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let sig =
        crate::maturity::maturity_signal(watched, has_insights, crate::maturity::MATURITY_TARGET);
    Ok(Json(serde_json::json!({
        "stage": sig.stage,
        "watched": sig.watched,
        "target": sig.target,
        "hasInsights": sig.has_insights,
    })))
}

/// GET /api/observatory/tool-usage — tool usage across all sessions
pub(crate) async fn tool_usage(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let data = state.pg.get_tool_usage_stats().await.map_err(|e| {
        tracing::error!("tool_usage error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({ "tools": data })))
}

/// GET /api/observatory/tool-signals — curated insight cards for the
/// Health tab's Insights strip.
///
/// Derives on the fly from `sensei.tool_usage_stats` (small dataset —
/// no cache round-trip needed) and applies [`ts::curate_insights`] so a
/// registry with dozens of dormant tools collapses to a single summary
/// card rather than one per tool. Per-tool detail rows still come from
/// `/api/observatory/tool-insights`, which reads the cached snapshot.
pub(crate) async fn tool_signals(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::analysis::insight_copy::{CopyLimits, copy_or_warm};
    use crate::api::handlers::tool_signals as ts;

    let rows = state.pg.get_tool_usage_stats().await.map_err(|e| {
        tracing::error!(error = %e, "tool_signals: get_tool_usage_stats failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let stats: Vec<ts::ToolUsageRow> =
        rows.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect();
    let raw = ts::derive_signals(&stats, chrono::Utc::now(), &ts::SignalThresholds::default());
    let mut signals = ts::curate_insights(raw);

    // ── Mentor-voice copy (insight-copy) ─────────────────────────────────────
    // Route each curated card's title + detail through the insight-copy pipeline.
    // `copy_or_warm` is a wire-path cache read (+ a detached background warm on a
    // miss), never a blocking model call — variant / action / tool_name stay
    // code-owned; only the sentence changes. The eager warm in
    // `tasks::tool_insights::aggregate_tool_insights` primes the per-tool cards so
    // this read is a pure cache hit in steady state; summary cards warm on first
    // miss. Cap at COPY_CAP as a belt-and-braces guard against a warm storm —
    // curation already collapses the list, so this rarely bites (same idiom as
    // `get_insights`).
    const COPY_CAP: usize = 8;
    for s in signals.iter_mut().take(COPY_CAP) {
        let (kind, facts, fb) = ts::signal_copy_inputs(s);
        let c =
            copy_or_warm(&state.pg, &state.gateway, kind, &facts, CopyLimits::default(), fb).await;
        s.title = c.title;
        s.detail = c.detail;
    }

    Ok(Json(serde_json::json!({ "signals": signals, "source": "derived" })))
}

/// GET /api/observatory/tool-insights — full cached snapshot per tool,
/// including metrics + signal (if any) + computed_at. Powers the Health
/// tab's per-tool detail pane and future trend charts.
pub(crate) async fn tool_insights(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let rows = state.pg.get_latest_tool_insights().await.map_err(|e| {
        tracing::error!(error = %e, "tool_insights: get_latest_tool_insights failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({ "insights": rows })))
}

/// GET /api/observatory/model-effectiveness — FTR / corrections / volume per
/// (provider, model) across the multi-model corpus (Zed + Claude). Powers the
/// "which models work best here" view.
pub(crate) async fn model_effectiveness(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let data = state.pg.get_model_effectiveness().await.map_err(|e| {
        tracing::error!("model_effectiveness error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({ "models": data })))
}

/// GET /api/libraries/{id}/usage — per-library usage across folders
pub(crate) async fn library_usage(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let library_id = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let data = state
        .pg
        .get_library_usage(&library_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "usage": data })))
}

/// GET /api/projects/{id}/teachings?limit=10 — adopted rules
pub(crate) async fn project_teachings(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<LimitQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    let data = state
        .pg
        .get_adopted_teachings(&project_id, q.limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "teachings": data })))
}

// ── Observatory · Today (Slot 1) ────────────────────────────────────────────

/// Compact relative-time label ("2d ago", "3h ago", "just now").
fn relative_when(
    then: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> String {
    let secs = (now - then).num_seconds().max(0);
    match secs {
        0..=59 => "just now".to_string(),
        60..=3_599 => format!("{}m ago", secs / 60),
        3_600..=86_399 => format!("{}h ago", secs / 3_600),
        86_400..=604_799 => format!("{}d ago", secs / 86_400),
        _ => format!("{}w ago", secs / 604_800),
    }
}

/// GET /api/observatory/today — the assembled Today payload. The daemon owns
/// every decision this screen renders (maturity, hero koan, insights, adopted
/// lane); the pure assembly lives in [`crate::observatory_home`]. The UI is a
/// dumb renderer of this payload plus `/api/observatory/ftr`.
pub(crate) async fn observatory_today(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::analysis::insight_copy::{CopyLimits, FallbackCopy, InsightKind, copy_or_warm};
    use crate::observatory_home as home;

    let now_local = chrono::Local::now();
    let greeting = home::greeting(now_local.hour());
    let today = now_local.format("%a · %-d %b").to_string();

    // Maturity is a daemon decision (aggregate across active projects), reusing
    // the shared, already-tested maturity gate.
    let (watched, has_insights) = state.pg.get_global_maturity_inputs().await.map_err(|e| {
        tracing::error!(error = %e, "observatory_today: maturity inputs failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let sig =
        crate::maturity::maturity_signal(watched, has_insights, crate::maturity::MATURITY_TARGET);

    // Recent sessions — reuse the raw shape `/api/sessions` returns so the app's
    // existing RecentSessions component + toRecentSessions() render them without
    // a second shaping path. Drop content-less ghost rows (#61), cap at 5.
    let all = state.pg.list_all_sessions(20, None, None).await.map_err(|e| {
        tracing::error!(error = %e, "observatory_today: sessions failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let recent: Vec<serde_json::Value> = all
        .into_iter()
        .filter(|s| {
            let has = |k: &str| s[k].as_str().map(|v| !v.trim().is_empty()).unwrap_or(false);
            has("project") || has("task") || has("summary")
        })
        .take(5)
        .collect();
    let recent_ids: Vec<String> =
        recent.iter().filter_map(|s| s["id"].as_str().map(str::to_string)).take(4).collect();

    let now = chrono::Utc::now();
    let (hero, insights, adopted) = if sig.stage == "mature" {
        let recs = state.pg.get_pending_recommendations_global(4).await
            .map_err(|e| { tracing::warn!(error = %e, "observatory_today: get_pending_recommendations_global failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
        let rec_lite: Vec<home::RecLite> = recs
            .iter()
            .map(|r| home::RecLite {
                urgency: r["urgency"].as_str().unwrap_or("low").to_string(),
                title: r["title"].as_str().unwrap_or("").to_string(),
                why: r["why"].as_str().unwrap_or("").to_string(),
                impact: r["impact"].as_str().map(str::to_string),
            })
            .collect();

        if let Some((top, rest)) = rec_lite.split_first() {
            // Honest provenance: session ids drawn from the top rec's own
            // evidence, not arbitrary recent sessions. Empty when it has none.
            let sources = recs
                .first()
                .map(|r| home::session_ids_from_evidence(&r["evidence"], 3))
                .unwrap_or_default();
            // Mature hero — route the koan title + body through insight-copy
            // (mature-only by design; see the early-branch note below). The
            // model owns the sentence; the code owns the action/impact/source/
            // noticed fields, which stay exactly as mature_hero set them.
            // `copy_or_warm` reads the persisted row on the wire and warms in
            // the background on a miss — this await never blocks on inference.
            let mut hero = home::mature_hero(top, &sources, "");
            let hero_facts = serde_json::json!({
                "title": top.title,
                "why": top.why,
                "impact": top.impact,
                "urgency": top.urgency,
                "sources": sources,
            });
            let hero_fallback = FallbackCopy {
                title: hero["koan"].as_str().unwrap_or_default().to_string(),
                detail: hero["body"].as_str().unwrap_or_default().to_string(),
            };
            let hero_copy = copy_or_warm(
                &state.pg,
                &state.gateway,
                InsightKind::HeroKoanMature,
                &hero_facts,
                CopyLimits::default(),
                hero_fallback,
            )
            .await;
            hero["koan"] = hero_copy.title.into();
            hero["body"] = hero_copy.detail.into();

            // Supporting insight cards — route each card's one-liner through
            // insight-copy. Each await is a cache read (+ a background warm on a
            // miss), not a model call, so the ≤4 per-screen loop stays cheap.
            let mut insights: Vec<serde_json::Value> = Vec::new();
            for rec in rest.iter().take(3) {
                let mut card = home::insight_card(rec);
                let card_facts = serde_json::json!({
                    "title": rec.title,
                    "why": rec.why,
                    "impact": rec.impact,
                });
                let card_fallback = FallbackCopy {
                    title: card["label"].as_str().unwrap_or_default().to_string(),
                    detail: card["text"].as_str().unwrap_or_default().to_string(),
                };
                let card_copy = copy_or_warm(
                    &state.pg,
                    &state.gateway,
                    InsightKind::InsightRecurringPattern,
                    &card_facts,
                    CopyLimits::default(),
                    card_fallback,
                )
                .await;
                card["text"] = card_copy.detail.into();
                insights.push(card);
            }

            let mems = state.pg.list_active_memories_global(5).await.map_err(|e| {
                tracing::warn!(error = %e, "observatory_today: list_active_memories_global failed");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            let adopted: Vec<serde_json::Value> = mems
                .iter()
                .map(|m| {
                    let when = m["modified_at"]
                        .as_str()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|t| relative_when(t.with_timezone(&chrono::Utc), now))
                        .unwrap_or_default();
                    let scope = m["scope"].as_str().unwrap_or("project");
                    let source = m["impact"]
                        .as_str()
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or("adopted by sensei");
                    home::adopted_row(&when, m["title"].as_str().unwrap_or(""), scope, source)
                })
                .collect();
            (hero, insights, adopted)
        } else {
            (home::steady_hero(recent.len()), Vec::new(), Vec::new())
        }
    } else {
        // Early (and the steady-hero branch above) stay static by design:
        // insight-copy is mature-only. Early copy is purpose-built calibration
        // text; routing it through the model risks inventing a teaching where
        // there is no signal (spec wrong-gate: "Koan is generic → early state").
        (
            home::early_hero(watched, crate::maturity::MATURITY_TARGET, &recent_ids),
            home::early_insights(),
            Vec::new(),
        )
    };

    Ok(Json(serde_json::json!({
        "greeting": greeting,
        "today": today,
        "dataMaturity": sig.stage,
        "hero": hero,
        "insights": insights,
        "adopted": adopted,
        "recentSessions": recent,
    })))
}

/// GET /api/observatory/ftr — holistic First-Try-Right rollup for the Today
/// header: `{ ftr14d, ftr14dPrev, ftrTrend[14], sessions7d }`.
pub(crate) async fn observatory_ftr(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let data = state.pg.get_holistic_ftr().await.map_err(|e| {
        tracing::error!(error = %e, "observatory_ftr failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(data))
}

#[derive(Deserialize)]
pub(crate) struct InsightsQuery {
    #[serde(default)]
    project: Option<String>,
}

/// GET /api/insights?project=<name|uuid>? — the Learnings Triage aggregator.
/// Bundles pending recommendations, in-force/violated memories, suggested/rule
/// patterns, and top recurring corrections into Now / Soon / Settled columns
/// (each item carries a `column` label the UI trusts). Cross-project by default;
/// `?project=` scopes every column to one project. Read-only — the Apply/Dismiss
/// actions reuse the per-project accept/reject endpoints.
pub(crate) async fn get_insights(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<InsightsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::analysis::insight_copy::{CopyLimits, FallbackCopy, InsightKind, copy_or_warm};
    use crate::insights as ins;

    let project: Option<uuid::Uuid> = match q.project.as_deref() {
        Some(p) if !p.trim().is_empty() => {
            Some(resolve_project_uuid(&state, p).await?.ok_or(StatusCode::NOT_FOUND)?)
        }
        _ => None,
    };
    let pref = project.as_ref();

    let err = |label: &'static str| {
        move |e: String| {
            tracing::error!(error = %e, "get_insights: {label} failed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };

    // Recommendations — always shown (bucketed by urgency), tagged with a column.
    let mut recs =
        state.pg.get_insights_recommendations(pref).await.map_err(err("recommendations"))?;
    for r in recs.iter_mut() {
        let col = ins::rec_column(r["urgency"].as_str().unwrap_or("low"));
        r["column"] = serde_json::json!(col);
    }

    // Memories — filtered + tagged; excluded when the bucket rule returns None.
    let mut memories: Vec<serde_json::Value> = state
        .pg
        .get_insights_memories(pref)
        .await
        .map_err(err("memories"))?
        .into_iter()
        .filter_map(|mut m| {
            let vc = m["violated_count"].as_i64().unwrap_or(0);
            ins::memory_column(m["status"].as_str().unwrap_or(""), vc).map(|col| {
                m["column"] = serde_json::json!(col);
                m
            })
        })
        .collect();

    // Patterns — suggested/rule only; tagged.
    let patterns: Vec<serde_json::Value> = state
        .pg
        .get_insights_patterns(pref)
        .await
        .map_err(err("patterns"))?
        .into_iter()
        .filter_map(|mut p| {
            ins::pattern_column(p["lifecycle"].as_str().unwrap_or("")).map(|col| {
                p["column"] = serde_json::json!(col);
                p
            })
        })
        .collect();

    // Corrections — top 3, always Now.
    let mut corrections =
        state.pg.get_insights_corrections(pref, 3).await.map_err(err("corrections"))?;
    for c in corrections.iter_mut() {
        c["column"] = serde_json::json!(ins::CORRECTION_COLUMN);
    }

    // ── Mentor-voice copy (insight-copy) ─────────────────────────────────────
    // Route each card's user-facing sentence through the insight-copy pipeline
    // in triage order (Now → Soon → Settled). `copy_or_warm` is a wire-path
    // cache read (+ a detached background warm on a miss), never a blocking model
    // call — column / tone / action stay code-owned; only the sentence changes.
    // Cap the model-routed items to the top 8 across the whole screen to avoid a
    // warm storm on first load: the spec ([[pipeline/insight-copy]]) budgets
    // "5 calls max per screen"; 8 is a safe ceiling here because warms are
    // backgrounded and deduped by facts_hash. Cards beyond the cap render their
    // static text (which is what they are until a warm caches them anyway).
    // Corrections stay static — short, count-driven labels, not a mentor sentence.
    // Corrections and patterns stay static: corrections are short, count-driven
    // labels; a pattern card's only free-text field (`name`) renders as a mono,
    // truncated identifier — a mentor sentence does not belong there (route
    // patterns only once the card grows a prose body — a frontend change).
    const COPY_CAP: usize = 8;
    let routable = recs.len() + memories.len();
    let mut budget = COPY_CAP;
    'cap: for col in [ins::NOW, ins::SOON, ins::SETTLED] {
        // Recommendations — title (headline) + why (impact sentence). Facts +
        // fallback come from the shared `ins::rec_copy_inputs` so this board and
        // the per-project recommendations endpoint hash to the SAME cache key
        // (one warm serves both screens).
        for r in recs.iter_mut().filter(|r| r["column"].as_str() == Some(col)) {
            if budget == 0 {
                break 'cap;
            }
            let (kind, facts, fallback) = ins::rec_copy_inputs(r);
            let copy = copy_or_warm(
                &state.pg,
                &state.gateway,
                kind,
                &facts,
                CopyLimits::default(),
                fallback,
            )
            .await;
            ins::apply_rec_copy(r, copy);
            budget -= 1;
        }
        // Memories — title + content. Adopt-worthy when in-force and unviolated;
        // otherwise it needs a human look (proposed, or violated regardless of
        // status — a live violation is never "adopt as-is").
        for m in memories.iter_mut().filter(|m| m["column"].as_str() == Some(col)) {
            if budget == 0 {
                break 'cap;
            }
            let status = m["status"].as_str().unwrap_or("");
            let violated = m["violated_count"].as_i64().unwrap_or(0) > 0;
            let kind = if !violated && matches!(status, "active" | "reinforced" | "battle_tested") {
                InsightKind::MemoryProposedAdopt
            } else {
                InsightKind::MemoryProposedReview
            };
            let facts =
                serde_json::json!({ "title": m["title"], "status": status, "what": m["content"] });
            let fallback = FallbackCopy {
                title: m["title"].as_str().unwrap_or_default().to_string(),
                detail: m["content"].as_str().unwrap_or_default().to_string(),
            };
            let copy = copy_or_warm(
                &state.pg,
                &state.gateway,
                kind,
                &facts,
                CopyLimits::default(),
                fallback,
            )
            .await;
            m["title"] = copy.title.into();
            m["content"] = copy.detail.into();
            budget -= 1;
        }
        // Patterns intentionally NOT routed — see the note above (mono/truncated
        // `name` field). They render their static identifier until the card gains
        // a prose body.
    }
    if routable > COPY_CAP {
        tracing::debug!(
            routable,
            cap = COPY_CAP,
            "get_insights: capped insight-copy routing to the top cards; remainder render static text"
        );
    }

    // Per-column totals across every source type.
    let count_in = |col: &str| -> i64 {
        let f = |v: &&serde_json::Value| v["column"].as_str() == Some(col);
        (recs.iter().filter(f).count()
            + memories.iter().filter(f).count()
            + patterns.iter().filter(f).count()
            + corrections.iter().filter(f).count()) as i64
    };
    let counts = serde_json::json!({
        "now": count_in(ins::NOW), "soon": count_in(ins::SOON), "settled": count_in(ins::SETTLED),
    });

    // Project filter chips — the distinct projects that actually appear in the
    // results, with their kanji from the project icon.
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for v in recs.iter().chain(memories.iter()).chain(patterns.iter()) {
        if let Some(pid) = v["project_id"].as_str() {
            seen.insert(pid.to_string());
        }
    }
    let all_projects = state.pg.list_projects().await.map_err(err("projects"))?;
    let projects: Vec<serde_json::Value> = all_projects
        .iter()
        .filter_map(|p| {
            let pid = p["id"].as_str()?;
            if !seen.contains(pid) {
                return None;
            }
            let kanji = if p["icon"].get("kind").and_then(|k| k.as_str()) == Some("kanji") {
                p["icon"].get("value").and_then(|v| v.as_str()).unwrap_or("場")
            } else {
                "場"
            };
            // last_session_at lets the Insights project filter show the 3 MOST-RECENT
            // projects as chips (the rest reachable by search) instead of one chip per
            // project. Already computed by list_projects; pass it through.
            Some(serde_json::json!({
                "id": pid, "name": p["name"], "kanji": kanji,
                "last_session_at": p["last_session_at"],
            }))
        })
        .collect();

    Ok(Json(serde_json::json!({
        "counts": counts,
        "projects": projects,
        "memories": memories,
        "recommendations": recs,
        "patterns": patterns,
        "corrections": corrections,
    })))
}
