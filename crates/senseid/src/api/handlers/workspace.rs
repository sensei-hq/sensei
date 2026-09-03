use crate::api::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Json, Sse, sse::Event},
};
use serde::Deserialize;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

// ── Repos CRUD ──────────────────────────────────────────────────────────────

pub(crate) async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    state.pg.list_repositories().await.map(Json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
pub(crate) struct CreateProjectBody {
    #[serde(rename = "repoId")]
    #[allow(dead_code)]
    repo_id: String,
    name: Option<String>,
    path: String,
}

pub(crate) async fn create_project(
    State(state): State<AppState>,
    Json(body): Json<CreateProjectBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let name = body
        .name
        .unwrap_or_else(|| body.path.split('/').next_back().unwrap_or("unknown").to_string());

    // Look up or create a watch root for the parent directory
    let parent_path = std::path::Path::new(&body.path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| body.path.clone());
    let root_id = state
        .pg
        .add_watch_root(&parent_path, "auto", &serde_json::json!([]))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let folder_id = state
        .pg
        .upsert_repo(&root_id, &name, &body.path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"ok": true, "folderId": folder_id})))
}

pub(crate) async fn update_project(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Look up folder by name (old string repo_id)
    let folder = state
        .pg
        .get_repo_by_name(&repo_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let folder_id = folder["id"]
        .as_str()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build props from the update body
    let mut props = serde_json::Map::new();
    if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
        props.insert("name".into(), serde_json::json!(name));
    }
    if let Some(status) = body.get("status").and_then(|v| v.as_str()) {
        props.insert("status".into(), serde_json::json!(status));
    }

    state
        .pg
        .set_folder_props(&folder_id, &serde_json::Value::Object(props))
        .await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn delete_project(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state
        .pg
        .delete_repo_by_name(&repo_id)
        .await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Folders ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct UpdateFolderBody {
    #[serde(default)]
    role: Option<String>,
}

/// PUT /api/folders/{id} — update mutable folder fields. Currently only
/// `role` is supported; the Projects setup stage uses this to persist the
/// per-folder role dropdown without going through set_folder_project
/// (which would also touch project membership).
pub(crate) async fn update_folder(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateFolderBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let folder_id = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    if let Some(role) = body.role.as_deref() {
        // Accept empty string as "clear the role" — daemon stores it as NULL.
        let role_arg = if role.is_empty() { None } else { Some(role) };
        state
            .pg
            .update_folder_role(&folder_id, role_arg)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

#[derive(Deserialize)]
pub(crate) struct RemapFolderBody {
    pub old: String,
    pub new: String,
}

/// Manual repair for a repo rename/move the auto-detect (git-remote match) couldn't
/// catch — `POST /api/folders/remap { old, new }`, backing `sensei folder remap`.
/// `new` must be an indexed folder (the rename's destination); `old` is the vanished
/// path. If `old` still has a stale folder row it is re-pointed onto `new` (history
/// moved, old path aliased forward, husk dropped); if it's already gone we just
/// record the forward alias. Either way orphaned sessions captured under `old` are
/// re-attached. 404 if `new` isn't indexed (scan it first); 400 on empty/identical.
pub(crate) async fn remap_folder_endpoint(
    State(state): State<AppState>,
    Json(body): Json<RemapFolderBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let old = body.old.trim();
    let new = body.new.trim();
    if old.is_empty() || new.is_empty() || old == new {
        return Err(StatusCode::BAD_REQUEST);
    }
    // `new` must be a real, indexed folder — the destination we attribute history to.
    let new_id = state
        .pg
        .folder_id_by_abs_path(new)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    // `old` may still carry a stale husk row (re-point it) or be already gone (alias only).
    let old_id =
        state.pg.folder_id_by_abs_path(old).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if old_id == Some(new_id) {
        return Err(StatusCode::BAD_REQUEST); // old and new are the same folder
    }
    let remapped = match old_id {
        Some(oid) => {
            state
                .pg
                .remap_folder(&oid, old, &new_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            true
        }
        None => {
            state
                .pg
                .add_folder_path_alias(old, &new_id, "manual")
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            false
        }
    };
    // BOTH repairs, via the shared definition. A remap is precisely the event that
    // makes a previously unresolvable cwd resolvable, so running only the
    // events-based half here left exactly the sessions this endpoint exists to
    // recover unattached. Cheap and bounded (two idempotent statements), so it
    // stays inline — the caller is asking "what did my remap recover?" and a task
    // id would not answer that.
    let sessions_repaired = crate::transcript::repair_sessions(&state.pg).await;
    Ok(Json(serde_json::json!({
        "ok": true,
        "old": old,
        "new": new,
        "remapped": remapped,
        "aliased": true,
        "sessions_repaired": sessions_repaired,
    })))
}

// ── Exclude / Exclusions ────────────────────────────────────────────────────

pub(crate) async fn exclude_project(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Look up folder path before deleting
    let folder =
        state.pg.get_repo_by_name(&repo_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let path = folder.as_ref().and_then(|f| f["abs_path"].as_str()).unwrap_or_default().to_string();

    // Clear indexed nodes before deleting the folder record
    if let Some(folder_id) = folder.as_ref().and_then(|f| crate::api::util::json_uuid(&f["id"]))
        && let Err(e) = state.pg.delete_nodes_by_folder(&folder_id).await
    {
        tracing::warn!(error = %e, %folder_id, "exclude_project: failed to delete nodes for folder");
    }

    // Delete the folder record (exclusions now handled by watcher)
    state
        .pg
        .delete_repo_by_name(&repo_id)
        .await
        .map(|_| Json(serde_json::json!({"ok": true, "excluded": path})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// Exclusions are per watch root (`folders_to_watch.excluded`) — managed via
// `update_watch_root` (PUT /api/scan/roots/{id}), which prunes added subtrees
// and re-scans removed ones. There is no standalone exclusions endpoint.

// ── Project Tags ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct TagBody {
    pub tag: String,
}

pub(crate) async fn add_project_tag(
    State(state): State<AppState>,
    Path(_repo_id): Path<String>,
    Json(body): Json<TagBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // PgStore tags are a controlled vocabulary (tag, category).
    // Register the tag in the vocabulary; per-entity tagging uses folder props.
    state
        .pg
        .add_tag(&body.tag, Some("repo"))
        .await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn remove_project_tag(
    State(state): State<AppState>,
    Path((_repo_id, tag)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // PgStore tags are a controlled vocabulary. Remove from vocabulary.
    state
        .pg
        .remove_tag(&tag)
        .await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── README frontmatter write-back (opt-in) ──────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct FrontmatterSyncBody {
    /// Absolute path of the repo whose README to update.
    pub folder: String,
    pub organization: Option<String>,
    pub client: Option<String>,
    pub project: Option<String>,
    pub team: Option<String>,
    pub role: Option<String>,
    #[serde(default)]
    pub stack: Vec<String>,
    pub summary: Option<String>,
    pub tagline: Option<String>,
    pub icon: Option<String>,
    pub icon_dark: Option<String>,
}

/// POST /api/repos/sync-frontmatter — write sensei's managed identity fields
/// into a repo's README frontmatter (preserving the body + unmanaged keys).
///
/// This is the ONLY path that writes a user's README; scanning stays read-only.
/// It is opt-in: refuses with 409 unless the `sync_readme_frontmatter`
/// preference is enabled (the UI hides the action when it is off). Triggered
/// explicitly from the UI, never by the scanner.
pub(crate) async fn sync_readme_frontmatter(
    State(state): State<AppState>,
    Json(body): Json<FrontmatterSyncBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Opt-in gate.
    let enabled = state
        .pg
        .get_config("sync_readme_frontmatter")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .as_deref()
        == Some("true");
    if !enabled {
        return Err((StatusCode::CONFLICT, "sync_readme_frontmatter is not enabled".to_string()));
    }

    let folder = state
        .pg
        .get_repo_by_path(&body.folder)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "folder not indexed".to_string()))?;
    let folder_id = crate::api::util::json_uuid(&folder["id"])
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "folder has no id".to_string()))?;

    let fm = crate::tasks::processors::metadata::Frontmatter {
        organization: body.organization,
        client: body.client,
        project: body.project,
        team: body.team,
        role: body.role,
        stack: body.stack,
        summary: body.summary,
        tagline: body.tagline,
        icon: body.icon,
        icon_dark: body.icon_dark,
    };

    // Merge into the existing README (or create README.md if none exists).
    let repo = std::path::Path::new(&body.folder);
    let readme = ["README.md", "readme.md", "Readme.md", "README"]
        .iter()
        .map(|n| repo.join(n))
        .find(|p| p.exists())
        .unwrap_or_else(|| repo.join("README.md"));
    let existing = std::fs::read_to_string(&readme).unwrap_or_default();
    let merged = crate::tasks::processors::metadata::merge_frontmatter(&existing, &fm);
    std::fs::write(&readme, &merged).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("write {}: {e}", readme.display()))
    })?;

    // Echo-loop guard: update the stored frontmatter snapshot so the watcher's
    // reconcile_repo_metadata (DB-only — it never writes the README) sees no change
    // and skips the redundant re-reconcile our own write would otherwise trigger.
    let snapshot = serde_json::json!({
        "frontmatter": serde_json::to_value(&fm).unwrap_or(serde_json::Value::Null),
    });
    state
        .pg
        .set_folder_props(&folder_id, &snapshot)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(serde_json::json!({ "ok": true, "readme": readme.display().to_string() })))
}

// ── Scan ────────────────────────────────────────────────────────────────────

fn expand_tilde(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        sensei_bootstrap::home_dir().join(stripped).to_string_lossy().to_string()
    } else {
        path.to_string()
    }
}

#[derive(Deserialize)]
pub(crate) struct AddRootBody {
    pub path: String,
    /// Optional exclusion globs applied to the watcher and stored on the
    /// folders_to_watch row (#41). Missing / null / [] all mean "no
    /// exclusions".
    #[serde(default)]
    pub excluded: Vec<String>,
}

/// One exclusion entry, resolved against its root and checked against disk.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ExclusionCheck {
    pub entry: String,
    pub resolved: String,
    pub exists: bool,
}

/// Resolve each exclusion against `root` and report whether it names a real
/// directory.
///
/// Exists because an exclusion that resolves nowhere is otherwise a SILENT
/// NO-OP: `root_exclusion_prefixes` joins `root + entry`, the join points at
/// nothing, and the scanner indexes the very content the user excluded. Live
/// cost of that silence: 329,087 nodes of a vendored Node/OpenSSL tree — 45% of
/// the graph — indexed from a path that HAD an exclusion entry, which was simply
/// missing one path segment.
///
/// Resolution goes through [`crate::db::pg_store::folders::resolve_exclusion`],
/// the same function the scanner and watcher resolve with, so this check cannot
/// disagree with what will actually be matched.
///
/// A non-existent path is REPORTED, not rejected: excluding a directory before
/// creating it is legitimate, and a hard failure would block it. The point is
/// that the caller can see it.
pub(crate) fn check_exclusions(root: &str, excluded: &[String]) -> Vec<ExclusionCheck> {
    excluded
        .iter()
        .filter(|e| !e.trim().is_empty())
        .map(|entry| {
            let resolved = crate::db::pg_store::folders::resolve_exclusion(root, entry);
            let exists = std::path::Path::new(&resolved).is_dir();
            ExclusionCheck { entry: entry.clone(), resolved, exists }
        })
        .collect()
}

/// Add a watch root to the DB immediately (synchronous) — does not start scanning.
/// The Scan page is responsible for calling POST /api/scan to trigger the actual scan.
///
/// After the row lands, the handler also registers the path with the global
/// RootWatcher and — if the watcher goes live — flips the row's status from the
/// DB default (`scanning`) to `watching`. Without this the UI's Roots list
/// would keep showing the "recursive" fallback badge indefinitely until the
/// next daemon restart when `spawn_root_watchers` finally ran (#6).
pub(crate) async fn add_watch_root(
    State(state): State<AppState>,
    Json(body): Json<AddRootBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let expanded = expand_tilde(&body.path);
    if expanded.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let name = std::path::Path::new(&expanded)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("root")
        .to_string();
    // Resolve every exclusion and check it names a real directory. A miss is
    // NOT fatal (excluding a not-yet-created path is legitimate) but it is
    // logged and returned, because the alternative — silent acceptance — is how
    // an entry missing one path segment let 329,087 vendored nodes into the
    // graph with nothing reporting it.
    let checks = check_exclusions(&expanded, &body.excluded);
    for c in checks.iter().filter(|c| !c.exists) {
        tracing::warn!(
            entry = %c.entry, resolved = %c.resolved,
            "add_watch_root: exclusion resolves to a path that does not exist — it will match \
             NOTHING and the content will be indexed"
        );
    }

    let excluded_json = serde_json::Value::Array(
        body.excluded.iter().map(|s| serde_json::Value::String(s.clone())).collect(),
    );
    let id = state.pg.add_watch_root(&expanded, &name, &excluded_json).await.map_err(|e| {
        tracing::error!("add_watch_root: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Register the new root with the live watcher singleton and (re)start it.
    // The mutex is std, so the lock scope must not span an await.
    let watching = {
        let queue = state.task_queue.clone();
        let w_mutex = crate::watcher::root_watcher::RootWatcher::instance(queue);
        match w_mutex.lock() {
            Ok(mut w) => {
                // RESOLVED prefixes, not the raw entries. `scan.rs` registers
                // the watcher with `root_exclusion_prefixes` output; passing the
                // raw relative form here meant the watcher saw a different shape
                // depending on which path registered it — the reason it had
                // grown its own two-form matcher.
                w.register(
                    std::path::PathBuf::from(&expanded),
                    checks.iter().map(|c| c.resolved.clone()).collect(),
                );
                w.start().is_ok()
                    && *w.status() == crate::watcher::root_watcher::WatcherStatus::Watching
            }
            Err(e) => {
                tracing::warn!(error = %e, "add_watch_root: RootWatcher mutex poisoned; leaving status at default");
                false
            }
        }
    };
    if watching && let Err(e) = state.pg.update_watch_status(&id, "watching").await {
        tracing::warn!(error = %e, %id, "add_watch_root: update_watch_status watching failed");
    }

    Ok(Json(serde_json::json!({
        "ok": true, "id": id, "path": expanded, "excluded": body.excluded,
        // What the exclusions actually RESOLVE to, and whether each names a real
        // directory — so a typo is visible at the moment it is made rather than
        // discovered later as unexpectedly-indexed content.
        "exclusionChecks": checks,
    })))
}

#[derive(Deserialize)]
pub(crate) struct UpdateRootBody {
    /// New name for the root (optional).
    #[serde(default)]
    pub name: Option<String>,
    /// Full replacement exclusion set (optional). Semantics are "set to this
    /// list" rather than "merge with existing" — the UI holds the canonical
    /// list; server-side merging invites drift.
    #[serde(default)]
    pub excluded: Option<Vec<String>>,
}

/// PUT /api/scan/roots/:id — update a watch root's name / exclusions (#41).
///
/// Path is immutable (a rename would need to remove + re-add for correctness).
/// Body fields are all optional; omitted fields leave the DB row alone.
pub(crate) async fn update_watch_root(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateRootBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Snapshot the root's path + current exclusions so we can diff (added prune,
    // removed re-scan) per the DDL semantics.
    let Some((root_path, old_excluded)) = state.pg.get_watch_root(&uuid).await.map_err(|e| {
        tracing::error!(error = %e, %uuid, "update_watch_root: get_watch_root failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    let excluded_json = body.excluded.as_ref().map(|list| {
        serde_json::Value::Array(
            list.iter().map(|s| serde_json::Value::String(s.clone())).collect(),
        )
    });
    state.pg.update_watch_root(&uuid, body.name.as_deref(), excluded_json.as_ref()).await.map_err(
        |e| {
            tracing::error!(error = %e, %uuid, "update_watch_root: DB write failed");
            StatusCode::INTERNAL_SERVER_ERROR
        },
    )?;

    let mut pruned_folders: u64 = 0;
    if let Some(new_list) = body.excluded.as_ref() {
        // Added entries → delete the matching subtree (folders + children).
        for entry in new_list.iter().filter(|e| !old_excluded.contains(*e)) {
            // The SAME resolver the live watcher is registered with, so a stored
            // exclusion cannot gate the watcher while pruning nothing. A second
            // copy of this formula is exactly how that happens.
            let prefix = crate::db::pg_store::folders::resolve_exclusion(&root_path, entry);
            pruned_folders += state.pg.prune_under_prefix(&prefix).await.unwrap_or_else(|e| {
                tracing::warn!(error = %e, entry, "update_watch_root: prune excluded subtree failed"); 0
            });
        }
        if pruned_folders > 0 {
            let _ = state.pg.prune_empty_projects(0).await;
        }
        // Removed entries → re-scan the root so the un-excluded subtree re-indexes.
        if old_excluded.iter().any(|e| !new_list.contains(e)) {
            let task = crate::tasks::Task::new(crate::tasks::TaskKind::ScanRoot, "", &root_path);
            state.task_queue.enqueue(task).await;
        }

        // Push the resolved absolute prefixes into the live watcher so the change
        // takes effect immediately, not on next daemon restart.
        // Fail closed: never register the live watcher with an empty exclusion
        // set on a read error (it would then watch/index the excluded subtree).
        let prefixes = state
            .pg
            .root_exclusion_prefixes(&root_path)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let w_mutex = crate::watcher::root_watcher::RootWatcher::instance(state.task_queue.clone());
        match w_mutex.lock() {
            Ok(mut w) => {
                w.register(std::path::PathBuf::from(&root_path), prefixes);
                let _ = w.start();
            }
            Err(e) => {
                tracing::warn!(error = %e, %uuid, "update_watch_root: RootWatcher mutex poisoned; DB updated, live state stale")
            }
        }
    }

    Ok(Json(serde_json::json!({ "ok": true, "id": uuid, "prunedFolders": pruned_folders })))
}

/// Delete a watch root by ID.
pub(crate) async fn delete_watch_root(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.remove_watch_root(&uuid).await.map_err(|e| {
        tracing::error!("delete_watch_root: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(crate) struct ScanBody {
    pub root: String,
    #[serde(default = "default_depth")]
    pub _max_depth: u32,
}

fn default_depth() -> u32 {
    4
}

pub(crate) async fn scan_folder(
    State(state): State<AppState>,
    Json(body): Json<ScanBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if body.root.is_empty() {
        tracing::warn!("scan_folder: empty root path rejected");
        return Err(StatusCode::BAD_REQUEST);
    }
    let root_path = expand_tilde(&body.root);
    let root = std::path::Path::new(&root_path);
    if !root.exists() {
        return Ok(Json(serde_json::json!({"ok": false, "error": "path not found"})));
    }

    // Enqueue ScanRoot task — runs asynchronously via task workers
    let task = crate::tasks::Task::new(crate::tasks::TaskKind::ScanRoot, "", &root_path);
    let task_id = state.task_queue.enqueue(task).await;

    Ok(Json(serde_json::json!({"ok": true, "scanning": true, "taskId": task_id})))
}

#[derive(Deserialize)]
pub(crate) struct BackfillBody {
    /// Optional repo abs_path to backfill. If omitted, every folder with
    /// nodes missing embeddings is backfilled.
    pub folder: Option<String>,
}

/// Enqueue `EmbedNodes` tasks to backfill embeddings for already-indexed nodes.
/// `EmbedNodes` otherwise only runs during a fresh (re)index, so existing or
/// unchanged folders never get embedded. Idempotent — `EmbedNodes` embeds only
/// nodes whose embedding is still NULL.
pub(crate) async fn backfill_embeddings(
    State(state): State<AppState>,
    Json(body): Json<BackfillBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let paths: Vec<String> = match body.folder {
        Some(f) if !f.trim().is_empty() => vec![expand_tilde(&f)],
        _ => state
            .pg
            .folders_with_pending_embeddings()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    };
    for abs_path in &paths {
        let task = crate::tasks::Task::new(crate::tasks::TaskKind::EmbedNodes, abs_path, "");
        state.task_queue.enqueue(task).await;
    }
    Ok(Json(serde_json::json!({"ok": true, "folders": paths.len()})))
}

/// Return project grouping suggestions from the last scan.
pub(crate) async fn scan_suggestions(State(state): State<AppState>) -> Json<serde_json::Value> {
    let suggestions = match state.pg.get_config("solution_suggestions").await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "scan_suggestions: get_config failed");
            None
        }
    }
    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
    .unwrap_or(serde_json::json!([]));
    Json(suggestions)
}

/// List configured scan roots with their scan status.
pub(crate) async fn scan_roots(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // A DB error is a 500 — never masked as an empty root list (which reads as
    // "no scan roots configured" and hides the failure).
    let mut roots = state.pg.list_watch_roots().await.map_err(|e| {
        tracing::warn!(error = %e, "scan_roots: list_watch_roots failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Enrich with repo count per root
    let repos = state.pg.list_repositories().await.map_err(|e| {
        tracing::warn!(error = %e, "scan_roots: list_repositories failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    for root in &mut roots {
        let root_path = root["path"].as_str().unwrap_or("");
        let count = repos
            .iter()
            .filter(|r| r["abs_path"].as_str().unwrap_or("").starts_with(root_path))
            .count();
        root["repos_found"] = serde_json::json!(count);
        root["scanned"] = serde_json::json!(count > 0);
    }

    Ok(Json(serde_json::json!(roots)))
}

// ── Indexing ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct IndexBody {
    #[serde(rename = "repoId")]
    repo_id: String,
    #[serde(rename = "repoPath")]
    repo_path: String,
    #[serde(default)]
    _force: bool,
}

pub(crate) async fn index_project(
    State(state): State<AppState>,
    Json(body): Json<IndexBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Clear errors for this project (PgStore expects UUID)
    if let Ok(folder_id) = uuid::Uuid::parse_str(&body.repo_id)
        && let Err(e) = state.pg.clear_index_errors(&folder_id).await
    {
        tracing::warn!(error = %e, %folder_id, "index_project: failed to clear index errors");
    }

    let task = crate::tasks::Task::new(
        crate::tasks::TaskKind::ProcessGitFolder,
        &body.repo_id,
        &body.repo_path,
    );
    let task_id = state.task_queue.enqueue(task).await;

    Ok(Json(serde_json::json!({
        "ok": true,
        "queued": true,
        "taskId": task_id,
        "repoId": body.repo_id,
    })))
}

pub(crate) async fn task_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let status = state.task_queue.status().await;
    let progress = state.task_queue.progress().await;
    Json(serde_json::json!({ "queue": status, "repos": progress }))
}

/// Read-only index integrity report (P2 — `GET /api/index/doctor`). Runs the
/// invariant self-audit in READ-ONLY mode and returns per-class drift counts +
/// samples. Never mutates — the periodic repair pass owns fixing. Backs
/// `sensei index doctor`.
pub(crate) async fn index_doctor(State(state): State<AppState>) -> Json<serde_json::Value> {
    let report = crate::tasks::index_audit::run_doctor(&state.pg).await;
    Json(serde_json::to_value(&report).unwrap_or_else(|_| serde_json::json!({})))
}

/// Every task event, for every task — the firehose behind both
/// `/api/tasks/progress` and `/api/index/progress`.
///
/// Live-only: a subscriber sees what happens from the moment it attaches and
/// nothing before, so it cannot answer "what happened to task N". Use
/// `/api/tasks/{id}/events` for that — it opens with a snapshot from the durable
/// log first.
///
/// This was two byte-identical functions serving the two routes. They never
/// diverged, but nothing prevented it — a filter added to one would silently not
/// apply to the other.
pub(crate) async fn task_progress_sse(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.task_queue.sender().subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => {
            let data = serde_json::to_string(&event).unwrap_or_default();
            Some(Ok(Event::default().data(data)))
        }
        Err(_) => None,
    });
    Sse::new(stream)
}

// ── Index Errors ────────────────────────────────────────────────────────────

pub(crate) async fn list_index_errors(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    state.pg.get_index_errors(None).await.map(Json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn list_repo_index_errors(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let folder_id = uuid::Uuid::parse_str(&repo_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state
        .pg
        .get_index_errors(Some(&folder_id))
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Stop ────────────────────────────────────────────────────────────────────

pub(crate) async fn stop() -> Json<serde_json::Value> {
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        std::process::exit(0);
    });
    Json(serde_json::json!({"ok": true}))
}

#[cfg(test)]
mod exclusion_tests {
    use super::*;

    /// An exclusion that resolves to a path which does not exist is a SILENT
    /// NO-OP today: `root_exclusion_prefixes` joins `root + entry`, the join
    /// points nowhere, and the scanner happily indexes the content the user
    /// meant to exclude. That is how 329,087 nodes of a vendored Node/OpenSSL
    /// tree — 45% of the graph — got indexed from a path that HAD an exclusion
    /// entry: the entry read `find-me-board/…` when the real path was
    /// `pre-sales/find-me-board/…`, so the resolved prefix matched nothing and
    /// nothing said so.
    ///
    /// Reporting `resolved` + `exists` back to the caller turns a typo into
    /// something visible at the moment it is made.
    ///
    /// Breaking mutation: make `exists` always true — the typo case stops being
    /// distinguishable from the real one.
    #[test]
    fn an_exclusion_that_resolves_nowhere_is_reported_not_silently_accepted() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        std::fs::create_dir_all(tmp.path().join("pre-sales/vendored")).unwrap();

        let checks =
            check_exclusions(&root, &["pre-sales/vendored".to_string(), "vendored".to_string()]);
        assert_eq!(checks.len(), 2);

        assert_eq!(checks[0].resolved, format!("{root}/pre-sales/vendored"));
        assert!(checks[0].exists, "the correct entry resolves to a real directory");

        assert_eq!(checks[1].resolved, format!("{root}/vendored"));
        assert!(
            !checks[1].exists,
            "the entry missing its `pre-sales/` segment resolves nowhere — exactly the live \
             typo, and it must be REPORTED rather than stored silently"
        );

        // Leading/trailing slashes are normalised by the shared resolver, so a
        // second copy of that rule cannot drift from it.
        let checks = check_exclusions(&root, &["/pre-sales/vendored/".to_string()]);
        assert_eq!(checks[0].resolved, format!("{root}/pre-sales/vendored"));
        assert!(checks[0].exists);
    }
}
