use super::*;

/// A resolved repo anchor for an absolute path (spec 2026-08-18): the durable repo a
/// session/transcript attaches to — the nearest repo-kind ancestor (git/subtree/
/// standalone-project-root), alias-aware, deepest+live wins.
#[derive(Debug, Clone)]
pub struct RepoAnchor {
    pub repo_folder_id: uuid::Uuid,
    pub project_id: Option<uuid::Uuid>,
    pub repo_abs_path: String,
    /// "live" (a current abs_path) or "alias" (a former path, after a move/rename).
    pub matched_via: String,
}

/// Resolve ONE stored exclusion entry against its root.
///
/// Stored entries are RELATIVE to the root (`"Code"`, `"archive/old"`) — see
/// `root_exclusion_prefixes_resolves_relative_entries_against_root`. This is the
/// single place that turns one into the absolute prefix the watcher compares and
/// the pruner deletes under.
///
/// It exists because there were briefly two copies: `root_exclusion_prefixes`
/// (feeding the live watcher) and a closure in `update_watch_root` (feeding the
/// prune). Two copies of a path formula is how a stored exclusion comes to gate
/// the watcher while pruning nothing.
pub(crate) fn resolve_exclusion(root_path: &str, entry: &str) -> String {
    let root = root_path.trim_end_matches('/');
    format!("{root}/{}", entry.trim_start_matches('/').trim_end_matches('/'))
}

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    /// The absolute-path exclusion prefixes for a watch root — each `excluded`
    /// entry resolved against `root_path` (`root/entry`). Consumed by the scan
    /// (to skip classification) and the watcher (to ignore events).
    pub async fn root_exclusion_prefixes(&self, root_path: &str) -> Result<Vec<String>, String> {
        // Fail closed: a DB error must NOT read as "no exclusions" — that would
        // let the scanner/watcher/grep process folders the user explicitly
        // excluded (indexing/leaking excluded content). Propagate instead.
        let row: Option<(serde_json::Value,)> = sqlx_core::query_as::query_as(
            "SELECT excluded FROM sensei.folders_to_watch WHERE path = $1",
        )
        .bind(root_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let root = root_path.trim_end_matches('/');
        Ok(row
            .and_then(|(v,)| v.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|e| e.as_str().map(str::to_string))
            .filter(|e| !e.is_empty())
            .map(|e| resolve_exclusion(root, &e))
            .collect())
    }

    /// Watch root's path + its raw (relative) `excluded` list, by id — for the
    /// update handler to diff old-vs-new and prune added / re-scan removed.
    pub async fn get_watch_root(
        &self,
        id: &uuid::Uuid,
    ) -> Result<Option<(String, Vec<String>)>, String> {
        let row: Option<(String, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT path, excluded FROM sensei.folders_to_watch WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(path, ex)| {
            let list = ex
                .as_array()
                .map(|a| a.iter().filter_map(|e| e.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            (path, list)
        }))
    }

    /// Delete every folder at or under `prefix` (cascade nodes/edges/scan_state).
    /// Used when an exclusion is added; the emptied projects are then removed by
    /// [`Self::prune_empty_projects`]. `starts_with` is exact-prefix (no LIKE
    /// wildcard hazard). Returns folders deleted.
    pub async fn prune_under_prefix(&self, prefix: &str) -> Result<u64, String> {
        let p = prefix.trim_end_matches('/');
        let res = sqlx_core::query::query(
            "DELETE FROM sensei.folders f
              WHERE f.abs_path = $1 OR starts_with(f.abs_path, $1 || '/')",
        )
        .bind(p)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    /// Register a git repo as a folder. Equivalent to old upsert_repo_basic.
    pub async fn upsert_repo(
        &self,
        root_id: &uuid::Uuid,
        name: &str,
        abs_path: &str,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_folder(root_id, "git", name, name, abs_path, None, None).await
    }

    /// Register a project root with an explicit folder kind — `git` for real
    /// repos, `standalone` for quasi-repos (non-git project roots).
    ///
    /// Unlike [`upsert_folder`]'s sticky-kind upsert, a root's git↔standalone
    /// classification is **authoritative on every scan**: a repo that lost its
    /// `.git` (now a quasi-repo) is relabelled `standalone`, and one that gained
    /// a `.git` flips back to `git`. `subtree`/`folder` kinds are never clobbered
    /// here — those are owned by subtree detection and tree materialisation.
    pub async fn upsert_repo_kind(
        &self,
        root_id: &uuid::Uuid,
        kind: &str,
        name: &str,
        abs_path: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path)
             VALUES($1, $2::sensei.folder_kind, $3, $3, $4)
             ON CONFLICT(abs_path) DO UPDATE SET
                kind = CASE WHEN folders.kind IN ('git'::sensei.folder_kind, 'standalone'::sensei.folder_kind)
                            THEN EXCLUDED.kind ELSE folders.kind END,
                name = EXCLUDED.name,
                modified_at = now()
             RETURNING id"
        )
            .bind(root_id).bind(kind).bind(name).bind(abs_path)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Upsert a structural subfolder (`kind='folder'`) within a project, linked
    /// to its parent folder. Thin wrapper over [`Self::upsert_subfolder_kind`].
    pub async fn upsert_subfolder(
        &self,
        root_id: &uuid::Uuid,
        name: &str,
        path: &str,
        abs_path: &str,
        parent_id: Option<&uuid::Uuid>,
        project_id: Option<&uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_subfolder_kind(root_id, "folder", name, path, abs_path, parent_id, project_id)
            .await
    }

    /// Upsert a structural subfolder with an explicit `kind` — `folder` (the
    /// navigable filesystem-tree row) or `workspace_member` (a monorepo member,
    /// D5a). Status is terminal (`indexed`) — these rows model the tree, not scan
    /// progress. On conflict the kind is relabelled ONLY between the two
    /// structural kinds (`folder`↔`workspace_member`); a path that is actually a
    /// (nested) project ROOT (`git`/`standalone`/`subtree`) is never reclassified.
    pub async fn upsert_subfolder_kind(
        &self,
        root_id: &uuid::Uuid,
        kind: &str,
        name: &str,
        path: &str,
        abs_path: &str,
        parent_id: Option<&uuid::Uuid>,
        project_id: Option<&uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders(root_id, kind, status, name, path, abs_path, parent_id, project_id)
             VALUES($1, $2::sensei.folder_kind, 'indexed'::sensei.folder_status, $3, $4, $5, $6, $7)
             ON CONFLICT(abs_path) DO UPDATE SET
                kind = CASE WHEN folders.kind IN ('folder'::sensei.folder_kind, 'workspace_member'::sensei.folder_kind)
                            THEN EXCLUDED.kind ELSE folders.kind END,
                name = EXCLUDED.name,
                parent_id = COALESCE(EXCLUDED.parent_id, folders.parent_id),
                project_id = COALESCE(EXCLUDED.project_id, folders.project_id),
                modified_at = now()
             RETURNING id"
        )
            .bind(root_id).bind(kind).bind(name).bind(path).bind(abs_path).bind(parent_id).bind(project_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Get a repo (folder with kind='git'/'subtree') by abs_path.
    pub async fn get_repo_by_path(
        &self,
        abs_path: &str,
    ) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, uuid::Uuid, String, String, String, Option<uuid::Uuid>, serde_json::Value, Vec<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, root_id, kind::text, name, abs_path, project_id, props, tags, modified_at FROM sensei.folders WHERE abs_path = $1"
            ).bind(abs_path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id, root_id, kind, name, abs, pid, props, tags, modified)| {
            serde_json::json!({
                "id": id, "root_id": root_id, "kind": kind, "name": name, "abs_path": abs,
                "project_id": pid, "props": props, "tags": tags,
                "modified_at": modified.to_rfc3339(),
            })
        }))
    }

    /// Get a repo by name (for backward compat with repo_id lookups).
    pub async fn get_repo_by_name(&self, name: &str) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, Option<uuid::Uuid>, serde_json::Value, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, abs_path, project_id, props, modified_at FROM sensei.folders WHERE name = $1 AND kind IN ('git'::sensei.folder_kind, 'subtree'::sensei.folder_kind, 'standalone'::sensei.folder_kind) LIMIT 1"
            ).bind(name).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id, name, abs, pid, props, modified)| {
            serde_json::json!({ "id": id, "name": name, "abs_path": abs, "project_id": pid, "props": props, "modified_at": modified.to_rfc3339() })
        }))
    }

    /// Record the checked-out branch in the TYPED column.
    ///
    /// `folders.branch` was declared and never written — NULL in all 7,772 rows —
    /// while `process_git_folder` put the value in `props->>'branch'`. Two stores
    /// for one fact, and the dead one was the typed, indexable, queryable one.
    ///
    /// It is a genuine partition key rather than a label, because the design is one
    /// folder per checkout ("develop vs main = two folders, one repository") and
    /// that is already live: fitness, strategos and website each have two folder
    /// rows on two branches, so their graphs are already separate. Filtering
    /// `sensei.graph_nodes` by branch therefore separates real graphs — no
    /// branch-in-node-identity needed, which would have cost ~2.1 GB per extra
    /// branch (nodes is 2,150 MB, dominated by embeddings).
    ///
    /// Guarded so a re-index to the same branch changes 0 rows.
    pub async fn set_folder_branch(
        &self,
        folder_id: &uuid::Uuid,
        branch: &str,
    ) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.folders SET branch = $2, modified_at = now()
              WHERE id = $1 AND branch IS DISTINCT FROM $2",
        )
        .bind(folder_id)
        .bind(branch)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("set_folder_branch: {e}"))?;
        Ok(res.rows_affected())
    }

    /// Set folder props (metadata like stack, libs, indexed_at, etc.).
    pub async fn set_folder_props(
        &self,
        folder_id: &uuid::Uuid,
        props: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET props = props || $2, modified_at = now() WHERE id = $1",
        )
        .bind(folder_id)
        .bind(props)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Assign a folder to a project with role/label.
    pub async fn set_folder_project(
        &self,
        folder_id: &uuid::Uuid,
        project_id: &uuid::Uuid,
        role: &str,
        label: Option<&str>,
    ) -> Result<(), String> {
        let props = serde_json::json!({"role": role, "label": label});
        sqlx_core::query::query(
            "UPDATE sensei.folders SET project_id = $2, props = props || $3, modified_at = now() WHERE id = $1"
        ).bind(folder_id).bind(project_id).bind(props).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update only the `role` column on a folder. Used by the Projects
    /// setup stage when the user picks a role from the dropdown — distinct
    /// from set_folder_project (which also reassigns project membership).
    pub async fn update_folder_role(
        &self,
        folder_id: &uuid::Uuid,
        role: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET role = $2::sensei.folder_role, modified_at = now() WHERE id = $1"
        ).bind(folder_id).bind(role).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// All folders belonging to a project, ordered by path. Used to enrich
    /// /api/projects responses with folder membership so the Projects setup
    /// page can render per-folder details.
    pub async fn list_folders_by_project(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.query_folders_by_project(project_id, false).await
    }

    /// Repo-root folders only (`kind IN ('git','standalone')`) for a project —
    /// the compact folder set the folder-scoped `find_projects` view (`GET
    /// /api/projects?under=…`) needs. Drops the hundreds of nested
    /// `kind:'folder'` descendant rows that pushed that response past the MCP
    /// client's token cap (~72K chars for sensei). The repo roots are kept so
    /// the MCP proxy's `resolve_from_cwd_in` longest-prefix cwd→project match
    /// still resolves any deep working directory (a deep cwd still
    /// `starts_with` the repo root).
    pub async fn list_root_folders_by_project(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.query_folders_by_project(project_id, true).await
    }

    /// Return the absolute paths of a project's indexed folders — the input the
    /// analyzer scheduler needs to enqueue `DetectCommunities` per folder. Only
    /// `indexed` folders are worth running community detection on (others have
    /// no code nodes yet).
    pub async fn get_indexed_folder_paths_for_project(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT abs_path
             FROM sensei.folders
             WHERE project_id = $1 AND status = 'indexed'::sensei.folder_status
             ORDER BY abs_path",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(p,)| p).collect())
    }

    /// Resolve a project's canonical working-directory path — the `abs_path` of
    /// its shallowest repo-root folder (`kind IN ('git','standalone')`). Used by
    /// the relay run driver (P3.3b) to pick the cwd it spawns the agent in.
    ///
    /// Shortest `abs_path` wins so a monorepo project resolves to the repo root
    /// rather than a nested sub-package. `None` when the project has no
    /// repo-root folder (e.g. a project deleted out from under a run). The
    /// caller must still confirm the path exists on disk before spawning.
    pub async fn project_root_path(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Option<String>, String> {
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT abs_path
             FROM sensei.folders
             WHERE project_id = $1 AND kind::text IN ('git','standalone')
             ORDER BY length(abs_path), abs_path
             LIMIT 1",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(p,)| p))
    }

    /// Flip a folder to `indexed` and stamp `props.indexed_at`. The dedicated
    /// writer of the `indexed` status, called at the terminal community barrier
    /// (D4.1). Detected libs are folder metadata stamped separately via
    /// `set_folder_props` by the resolve/build barriers, so this need not carry
    /// them — keeping "communities computed → indexed" as the single meaning of
    /// this write.
    pub async fn mark_folder_indexed(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        let props = serde_json::json!({"indexed_at": chrono::Utc::now().to_rfc3339()});
        sqlx_core::query::query(
            "UPDATE sensei.folders SET status = 'indexed'::sensei.folder_status, props = props || $2, modified_at = now() WHERE id = $1"
        ).bind(folder_id).bind(&props).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Set a folder's lifecycle status (D6a). The general setter behind the
    /// `discovered → queued → indexing → indexed | failed` lifecycle;
    /// `mark_folder_indexed` remains the dedicated writer of `indexed` (it also
    /// stamps `props.indexed_at`). A scan marks `indexing` at start so a
    /// crash leaves a recoverable state (resume re-enqueues non-terminal folders).
    pub async fn update_folder_status(
        &self,
        folder_id: &uuid::Uuid,
        status: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET status = $2::sensei.folder_status, modified_at = now() WHERE id = $1"
        ).bind(folder_id).bind(status).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Read a folder's index status (`sensei.folders.status`). `None` when no
    /// such folder row exists — an honest miss, never a fabricated status. Used
    /// by the fail-closed barrier (D6d) to leave a folder with a recorded fatal
    /// failure `failed` rather than advancing it to `indexed`.
    pub async fn get_folder_status(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Option<String>, String> {
        let row: Option<(String,)> =
            sqlx_core::query_as::query_as("SELECT status::text FROM sensei.folders WHERE id = $1")
                .bind(folder_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    /// Append a tag to a folder's `tags` array, idempotently (no duplicates).
    /// Used by the scan reconcile to flag a former project root that still has
    /// on-disk content but no live owner (`stale`) for the user to triage,
    /// rather than deleting content the scan can't account for.
    pub async fn tag_folder(&self, folder_id: &uuid::Uuid, tag: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders
                SET tags = array(SELECT DISTINCT unnest(tags || ARRAY[$2])),
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(folder_id)
        .bind(tag)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Remove a tag from a folder's `tags` array (no-op if absent). Pairs with
    /// [`tag_folder`] so the scan can keep a derived flag (e.g. `needs-review`)
    /// in sync — clearing it when a folder no longer qualifies.
    pub async fn untag_folder(&self, folder_id: &uuid::Uuid, tag: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET tags = array_remove(tags, $2), modified_at = now() WHERE id = $1",
        )
        .bind(folder_id)
        .bind(tag)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete a folder (cascade deletes nodes, edges, scan_state, etc.).
    pub async fn delete_repo_by_name(&self, name: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "DELETE FROM sensei.folders WHERE name = $1 AND kind IN ('git'::sensei.folder_kind, 'subtree'::sensei.folder_kind, 'standalone'::sensei.folder_kind)"
        ).bind(name).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Nodes ─────────────────────────────────────────────────────────

    pub async fn upsert_folder(
        &self,
        root_id: &uuid::Uuid,
        kind: &str,
        name: &str,
        path: &str,
        abs_path: &str,
        parent_id: Option<&uuid::Uuid>,
        project_id: Option<&uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path, parent_id, project_id)
             VALUES($1, $2::sensei.folder_kind, $3, $4, $5, $6, $7)
             ON CONFLICT(abs_path) DO UPDATE SET name = EXCLUDED.name, project_id = COALESCE(EXCLUDED.project_id, folders.project_id), modified_at = now()
             RETURNING id"
        ).bind(root_id).bind(kind).bind(name).bind(path).bind(abs_path).bind(parent_id).bind(project_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_folders_by_root(
        &self,
        root_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, Option<uuid::Uuid>, serde_json::Value, String)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, path, abs_path, project_id, remote_urls, status::text FROM sensei.folders WHERE root_id = $1 ORDER BY path"
        ).bind(root_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, path, abs, pid, remotes, status)| {
            serde_json::json!({ "id": id, "kind": kind, "name": name, "path": path, "abs_path": abs, "project_id": pid, "remote_urls": remotes, "status": status })
        }).collect())
    }

    pub async fn delete_folder_tree(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        // CASCADE will handle children via parent_id FK
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
            .bind(folder_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// A live project root that shares a git remote URL with `remotes` — the signal
    /// that a since-vanished root was RENAMED/MOVED rather than deleted. Restricted
    /// to project-root kinds whose `abs_path` is in `live_abs` (the paths this scan
    /// just discovered) so we only ever remap onto a freshly-confirmed folder.
    /// Returns the first match's id, or `None`. Empty `remotes` → `None` (a folder
    /// with no remote can't be remote-matched). DB-only; pure lookup.
    pub async fn find_live_root_by_remote(
        &self,
        remotes: &[String],
        live_abs: &[String],
    ) -> Result<Option<uuid::Uuid>, String> {
        if remotes.is_empty() || live_abs.is_empty() {
            return Ok(None);
        }
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT f.id FROM sensei.folders f \
             WHERE f.abs_path = ANY($2) \
               AND f.kind IN ('git'::sensei.folder_kind, 'standalone'::sensei.folder_kind, 'subtree'::sensei.folder_kind) \
               AND EXISTS (SELECT 1 FROM jsonb_array_elements(f.remote_urls) e WHERE e->>'url' = ANY($1)) \
             LIMIT 1",
        )
        .bind(remotes)
        .bind(live_abs)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// Whether a folder carries history worth keeping — i.e. any `activity.sessions`
    /// row is attached to it. Drives archive-not-delete on a vanished root.
    pub async fn folder_has_sessions(&self, folder_id: &uuid::Uuid) -> Result<bool, String> {
        // Check BOTH the durable repo anchor and the raw cwd folder (spec 2026-08-18):
        // post-P1 a repo's sessions attach via repo_folder_id, so keying on folder_id
        // alone would report a history-bearing repo as empty → the reconcile would
        // hard-delete it instead of archiving it (I3). This gates archive-not-delete.
        let row: Option<(i32,)> = sqlx_core::query_as::query_as(
            "SELECT 1 FROM activity.sessions WHERE repo_folder_id = $1 OR folder_id = $1 LIMIT 1",
        )
        .bind(folder_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.is_some())
    }

    /// Re-point a renamed/moved root's history onto its new folder, then drop the
    /// now-empty old row. Order matters: (1) record the old path as an alias of the
    /// new folder so future events under old paths resolve forward; (2) move the
    /// old folder's sessions to the new folder BEFORE deleting (delete_folder_tree
    /// cascades sessions, so this must precede it); (3) delete the old husk. The
    /// alias is the durable mapping — even if a session slips through, the orphan
    /// repair re-attaches it via the alias.
    pub async fn remap_folder(
        &self,
        old_folder_id: &uuid::Uuid,
        old_abs_path: &str,
        new_folder_id: &uuid::Uuid,
    ) -> Result<(), String> {
        self.add_folder_path_alias(old_abs_path, new_folder_id, "rename").await?;
        // Repoint BOTH the durable anchor and the raw folder to the moved repo BEFORE the
        // old row is deleted — otherwise delete_folder_tree would SET NULL repo_folder_id
        // and orphan the session from its repo (spec 2026-08-18, move/rename edge E3).
        sqlx_core::query::query(
            "UPDATE activity.sessions
                SET folder_id      = CASE WHEN folder_id      = $1 THEN $2 ELSE folder_id END,
                    repo_folder_id = CASE WHEN repo_folder_id = $1 THEN $2 ELSE repo_folder_id END
              WHERE folder_id = $1 OR repo_folder_id = $1",
        )
        .bind(old_folder_id)
        .bind(new_folder_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        self.delete_folder_tree(old_folder_id).await
    }

    /// Retain a vanished, history-bearing root as `archived` instead of deleting it:
    /// its directory is gone but its sessions/transcripts stay attached. The vanish
    /// prune and reconcile skip `archived` folders thereafter.
    pub async fn archive_folder(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET status = 'archived'::sensei.folder_status WHERE id = $1",
        )
        .bind(folder_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The folder registered at EXACTLY this absolute path (no alias resolution),
    /// or `None`. Distinct from [`Self::get_folder_ids_by_path`], which also follows
    /// aliases — the manual `remap` needs to know whether `old` is itself a real
    /// folder row (to re-point) versus already gone (alias-only).
    pub async fn folder_id_by_abs_path(
        &self,
        abs_path: &str,
    ) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.folders WHERE abs_path = $1 LIMIT 1",
        )
        .bind(abs_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// Write a git root's remotes (`[{name,url}]`) into `folders.remote_urls` — the
    /// producing half of git-remote rename detection, called during scan. Without
    /// this the column stays `'[]'` and [`Self::find_live_root_by_remote`] can never
    /// match, so auto-remap is inert (that was the pre-existing prod state).
    pub async fn update_folder_remotes(
        &self,
        folder_id: &uuid::Uuid,
        remotes: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query("UPDATE sensei.folders SET remote_urls = $2 WHERE id = $1")
            .bind(folder_id)
            .bind(remotes)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Populate the canonical `sensei.repositories` registry + `folders.repository_id`
    /// for the git/subtree CHECKOUT-ROOT folders under `root_id` (D10, the repo-grain
    /// metric grain). For each such folder that has a git remote, derive the canonical
    /// key via [`super::normalize_repo_key`] from its first `remote_urls` entry, upsert
    /// the GLOBAL `sensei.repositories` row (keyed on `repo_key`), and point
    /// `folders.repository_id` at it — so two clones / a rename / a re-checkout of one
    /// remote all resolve to ONE repository and metric history survives a folder move.
    /// A folder with no parseable remote is left `repository_id = NULL` (a local-only
    /// repo has no canonical identity and is never federated — never an abs_path
    /// fallback). Only git/subtree roots carry `repository_id` (I16); a subfolder
    /// resolves via its nearest ancestor. Idempotent: re-running upserts the same
    /// repository row and only re-points folders whose `repository_id` actually
    /// changed. Returns the number of folders (re)pointed this pass.
    pub async fn assign_repositories(&self, root_id: &uuid::Uuid) -> Result<u64, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, name, remote_urls->0->>'url' \
               FROM sensei.folders \
              WHERE root_id = $1 AND kind IN ('git', 'subtree')",
        )
        .bind(root_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let mut assigned = 0u64;
        for (folder_id, name, url) in rows {
            // No remote / unparseable → local-only: leave repository_id NULL.
            let Some(key) = url.as_deref().and_then(super::normalize_repo_key) else { continue };
            let repo_id: (uuid::Uuid,) = sqlx_core::query_as::query_as(
                "INSERT INTO sensei.repositories (repo_key, remote_url, name) VALUES ($1, $2, $3) \
                 ON CONFLICT (repo_key) DO UPDATE SET remote_url = EXCLUDED.remote_url, modified_at = now() \
                 RETURNING id"
            ).bind(&key).bind(url.as_deref()).bind(&name)
             .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
            // Re-point only when it changes, so a no-op re-run stays cheap.
            let res = sqlx_core::query::query(
                "UPDATE sensei.folders SET repository_id = $2 \
                  WHERE id = $1 AND repository_id IS DISTINCT FROM $2",
            )
            .bind(folder_id)
            .bind(repo_id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
            assigned += res.rows_affected();
        }
        Ok(assigned)
    }

    /// The repositories a project spans — the distinct `repository_id`s on its
    /// git/subtree checkout folders (D2: a project is a GROUP of repositories). The
    /// repo-grain compute iterates THIS instead of the single `project_root_path`
    /// (killing the multi-repo blind spot). Ordered by folder path so the "primary"
    /// (shallowest) repository is first and iteration is deterministic. Honest-empty
    /// when the project has no repository-linked folder.
    pub async fn repositories_for_project(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<uuid::Uuid>, String> {
        let rows: Vec<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT f.repository_id \
               FROM sensei.folders f \
              WHERE f.project_id = $1 AND f.repository_id IS NOT NULL \
              GROUP BY f.repository_id \
              ORDER BY min(length(f.abs_path))",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(r,)| r).collect())
    }

    /// The project's CANONICAL (primary) repository — the repository of its
    /// shallowest checkout folder (the root repo). Project-level metrics that have no
    /// natural per-repo grain (`knowledge`, `tool`) are attributed here so they fit
    /// the repo-grain identity. `None` when the project has no repository-linked
    /// folder (a repo-less quasi-repo) — those metrics are then honest-empty.
    pub async fn primary_repository_for_project(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT f.repository_id \
               FROM sensei.folders f \
              WHERE f.project_id = $1 AND f.repository_id IS NOT NULL \
              ORDER BY length(f.abs_path) ASC \
              LIMIT 1",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(r,)| r))
    }

    /// The repository ROOTS a project spans — one `(repository_id, abs_path)` per
    /// distinct `repository_id`, where `abs_path` is the SHALLOWEST (shortest-path)
    /// checkout folder for that repository (its root working tree). The repo-grain
    /// churn/quality compute iterates THIS to run the git-log / qlty walk once per
    /// repository in the project's actual checkout on disk (D2: a project is a GROUP
    /// of repositories). Ordered by path length so the primary (shallowest) repository
    /// is first and iteration is deterministic. Honest-empty when the project has no
    /// repository-linked folder — those repositories are then skipped, never faked (I-E).
    pub async fn repository_roots_for_project(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<(uuid::Uuid, String)>, String> {
        let rows: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT roots.repository_id, roots.abs_path FROM ( \
               SELECT DISTINCT ON (f.repository_id) f.repository_id, f.abs_path \
                 FROM sensei.folders f \
                WHERE f.project_id = $1 AND f.repository_id IS NOT NULL \
                ORDER BY f.repository_id, length(f.abs_path) ASC \
             ) roots \
             ORDER BY length(roots.abs_path) ASC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// The repository's FIRST AI-transcript day — the earliest captured activity
    /// anchored to it: `least(min(sessions.started_at), min(assistant_events.ts))`,
    /// both resolved to the repository via a session's `repo_folder_id →
    /// folders.repository_id`. This is the floor the git-cadence metrics
    /// (`churn`/`quality`) start at by DEFAULT (spec D17: measure the user+AI
    /// interaction, not the repo's whole pre-AI git history). `None` when the
    /// repository has NO captured AI activity — those metrics are then honest-empty
    /// for it (nothing to measure), unless a pre-AI baseline is opted in. Propagates
    /// the read error; never masks it.
    pub async fn repo_ai_start(
        &self,
        repository_id: &uuid::Uuid,
    ) -> Result<Option<chrono::NaiveDate>, String> {
        let row: (Option<chrono::NaiveDate>,) = sqlx_core::query_as::query_as(
            "SELECT least( \
               (SELECT min(date_trunc('day', s.started_at)::date) \
                  FROM activity.sessions s \
                  JOIN sensei.folders    f ON f.id = s.repo_folder_id \
                 WHERE f.repository_id = $1), \
               (SELECT min(date_trunc('day', to_timestamp(ae.ts / 1000.0))::date) \
                  FROM activity.assistant_events ae \
                  JOIN activity.sessions        s ON s.client_session_id = ae.session_id \
                  JOIN sensei.folders           f ON f.id = s.repo_folder_id \
                 WHERE f.repository_id = $1))",
        )
        .bind(repository_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// List folders in a non-terminal (recoverable) index state, for startup
    /// resume: `discovered` (scan ran, ProcessGitFolder hadn't started),
    /// `queued` (enqueued, not started), `indexing` (a scan was in-flight when
    /// the daemon stopped — its in-memory task was lost, D6a), and `failed`
    /// (errored, should retry). `indexed`, `deferred` (intentionally not indexed
    /// — sibling/standalone), and `archived` (directory gone) are terminal and
    /// excluded.
    ///
    /// Called once at daemon startup to rebuild the in-memory queue, which
    /// otherwise loses every task on restart. Re-enqueuing an already-running
    /// folder is deduped by the single-writer guard (`enqueue_unique`).
    pub async fn list_pending_folders(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, String, String, String, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, root_id, kind::text, name, abs_path, status::text \
             FROM sensei.folders \
             WHERE status IN ('discovered'::sensei.folder_status, 'queued'::sensei.folder_status, \
                              'indexing'::sensei.folder_status, 'failed'::sensei.folder_status) \
             ORDER BY abs_path",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(id, root_id, kind, name, abs_path, status)| {
                serde_json::json!({
                    "id": id,
                    "root_id": root_id,
                    "kind": kind,
                    "name": name,
                    "abs_path": abs_path,
                    "status": status,
                })
            })
            .collect())
    }

    /// Count folders belonging to a project that have not yet reached a terminal
    /// index state. Returns 0 when all folders are `indexed` or `failed`.
    pub async fn count_unindexed_folders(&self, project_id: uuid::Uuid) -> Result<i64, String> {
        let row: (i64,) = sqlx_core::query_as::query_as(
            "SELECT COUNT(*) FROM sensei.folders
              WHERE project_id = $1
                AND status NOT IN ('indexed'::sensei.folder_status, 'failed'::sensei.folder_status)"
        ).bind(project_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    // ── Benchmark Reports ────────────────────────────────────────────

    pub async fn list_repositories(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT id, name, abs_path, kind::text FROM sensei.folders WHERE kind IN ('git'::sensei.folder_kind, 'subtree'::sensei.folder_kind, 'standalone'::sensei.folder_kind) ORDER BY name"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, abs_path, kind)| {
            serde_json::json!({ "id": id, "name": name, "abs_path": abs_path, "kind": kind })
        }).collect())
    }

    // ── Memories ──────────────────────────────────────────────────────

    pub async fn add_watch_root(
        &self,
        path: &str,
        name: &str,
        excluded: &serde_json::Value,
    ) -> Result<uuid::Uuid, String> {
        // On conflict, PRESERVE the existing `excluded` — exclusions are managed
        // by `update_watch_root` (the roots API), and a re-scan passes `[]`, which
        // must never wipe the user's exclusions. `excluded` here is the seed for
        // the first insert only.
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders_to_watch(path, name, excluded) VALUES($1, $2, $3)
             ON CONFLICT(path) DO UPDATE SET name = EXCLUDED.name, modified_at = now()
             RETURNING id",
        )
        .bind(path)
        .bind(name)
        .bind(excluded)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_watch_roots(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, serde_json::Value, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, path, name, status::text, excluded, modified_at FROM sensei.folders_to_watch ORDER BY path"
            ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, path, name, status, excluded, modified)| {
            serde_json::json!({ "id": id, "path": path, "name": name, "status": status, "excluded": excluded, "modified_at": modified.to_rfc3339() })
        }).collect())
    }

    /// The watch root that contains `path` — the exact root, or the nearest
    /// ancestor root when `path` sits inside one. Lets `scan_root` reuse an
    /// existing top-level root instead of registering a redundant sub-root (watch
    /// roots stay top-level; a change resolves to its repo via
    /// [`Self::repo_root_for_path`]). `None` when `path` is under no watch root.
    pub async fn enclosing_watch_root(
        &self,
        path: &str,
    ) -> Result<Option<(uuid::Uuid, String)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT id, path FROM sensei.folders_to_watch
              WHERE $1 = path OR $1 LIKE path || '/%'
              ORDER BY length(path) DESC LIMIT 1",
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    pub async fn update_watch_status(&self, id: &uuid::Uuid, status: &str) -> Result<(), String> {
        sqlx_core::query::query("UPDATE sensei.folders_to_watch SET status = $2::sensei.watch_status, modified_at = now() WHERE id = $1")
            .bind(id).bind(status).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update a watch root's name and/or exclusions. Passing `None` for a
    /// field leaves it unchanged; passing `Some(x)` replaces the current
    /// value with `x`. Path is deliberately not mutable — a rename would
    /// need to remove + re-add (folders_to_watch.path is UNIQUE and the
    /// materialised sensei.folders subtree references it). #41.
    pub async fn update_watch_root(
        &self,
        id: &uuid::Uuid,
        name: Option<&str>,
        excluded: Option<&serde_json::Value>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders_to_watch
                SET name     = COALESCE($2, name),
                    excluded = COALESCE($3, excluded),
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(id)
        .bind(name)
        .bind(excluded)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn remove_watch_root(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Scan State ───────────────────────────────────────────────────

    /// Record that a file was indexed at this fingerprint (clears any prior
    /// skip reason — a file that used to be unscannable and now parses is
    /// indexed normally).
    pub async fn upsert_scan_state(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
        mtime: i64,
        content_hash: &str,
    ) -> Result<(), String> {
        self.write_scan_state(folder_id, file_path, mtime, content_hash, None).await
    }

    /// Record that a file was EXAMINED at this fingerprint but deliberately not
    /// indexed, and why.
    ///
    /// Writing the fingerprint is the point: `plan_reindex` treats a file with
    /// no prior row as changed, so a skip that isn't fingerprinted is re-enqueued
    /// on every reconcile forever. Keying the skip to the fingerprint also makes
    /// it self-healing — fix the file and the mtime/hash change re-triggers it.
    pub async fn upsert_scan_state_skipped(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
        mtime: i64,
        content_hash: &str,
        reason: crate::classifiers::ScanSkipReason,
    ) -> Result<(), String> {
        self.write_scan_state(folder_id, file_path, mtime, content_hash, Some(reason)).await
    }

    /// Single writer behind [`upsert_scan_state`] / [`upsert_scan_state_skipped`]
    /// so the statement lives in exactly one place. `skip_reason` is always
    /// assigned on conflict (never left stale) so a file can move between
    /// indexed and skipped in either direction.
    async fn write_scan_state(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
        mtime: i64,
        content_hash: &str,
        reason: Option<crate::classifiers::ScanSkipReason>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.scan_state(folder_id, file_path, mtime, content_hash, skip_reason)
             VALUES($1, $2, $3, $4, $5::sensei.scan_skip_reason)
             ON CONFLICT(folder_id, file_path) DO UPDATE SET mtime = EXCLUDED.mtime, content_hash = EXCLUDED.content_hash, skip_reason = EXCLUDED.skip_reason, indexed_at = now(), modified_at = now()"
        ).bind(folder_id).bind(file_path).bind(mtime).bind(content_hash)
            .bind(reason.map(|r| r.as_db()))
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_stale_files(
        &self,
        folder_id: &uuid::Uuid,
        current_files: &[(String, i64)],
    ) -> Result<Vec<String>, String> {
        // Return files where mtime has changed
        let mut stale = Vec::new();
        for (path, mtime) in current_files {
            let row: Option<(i64,)> = sqlx_core::query_as::query_as(
                "SELECT mtime FROM sensei.scan_state WHERE folder_id = $1 AND file_path = $2",
            )
            .bind(folder_id)
            .bind(path)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
            match row {
                None => stale.push(path.clone()), // new file
                Some((old_mtime,)) if old_mtime != *mtime => stale.push(path.clone()),
                _ => {}
            }
        }
        Ok(stale)
    }

    pub async fn delete_scan_state(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.scan_state WHERE folder_id = $1")
            .bind(folder_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// All scan-state fingerprints for a folder as `(file_path, mtime,
    /// content_hash)`. Loaded once per scan so the indexer can run the two-tier
    /// change-detection (cheap mtime gate → content-hash gate) entirely in
    /// memory instead of N per-file queries. The `content_hash` lets a re-scan
    /// short-circuit a *touched-but-identical* file (mtime drifted, bytes same)
    /// without reindexing it. See [`crate::tasks::handlers::scan_logic::plan_reindex`].
    pub async fn list_scan_state_full(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<(String, i64, String)>, String> {
        let rows: Vec<(String, i64, String)> = sqlx_core::query_as::query_as(
            "SELECT file_path, mtime, content_hash FROM sensei.scan_state WHERE folder_id = $1",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// All scan-state fingerprints for a folder as `(file_path, mtime)`. A thin
    /// projection over [`Self::list_scan_state_full`] for callers that only need
    /// the mtime (removed-file diff, tests).
    pub async fn list_scan_state(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<(String, i64)>, String> {
        Ok(self
            .list_scan_state_full(folder_id)
            .await?
            .into_iter()
            .map(|(p, m, _)| (p, m))
            .collect())
    }

    /// Drop a single file's scan-state row (used when a file no longer exists on
    /// disk, e.g. it was deleted or removed by a branch switch).
    pub async fn delete_scan_state_file(
        &self,
        folder_id: &uuid::Uuid,
        file_path: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "DELETE FROM sensei.scan_state WHERE folder_id = $1 AND file_path = $2",
        )
        .bind(folder_id)
        .bind(file_path)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Services ─────────────────────────────────────────────────────

    /// Nearest-ancestor folder for an absolute path: the folder whose `abs_path`
    /// is the path itself or its closest parent. Attributes a hook event (which
    /// carries a `cwd`) to the indexed folder it ran in. `None` when uncovered.
    pub async fn find_folder_for_path(
        &self,
        path: &str,
    ) -> Result<Option<(uuid::Uuid, Option<uuid::Uuid>)>, String> {
        // Nearest ancestor over current abs_paths AND former paths (aliases), so a
        // hook cwd recorded under an old path (pre-rename) still attributes to the
        // current folder. A live abs_path and an alias of equal length tie-break to
        // the live folder (abs_path row sorts first).
        let row: Option<(uuid::Uuid, Option<uuid::Uuid>)> = sqlx_core::query_as::query_as(
            "SELECT id, project_id FROM (
                 SELECT id, project_id, abs_path AS p, 1 AS live FROM sensei.folders
                 UNION ALL
                 SELECT f.id, f.project_id, a.alias_abs_path AS p, 0 AS live
                   FROM sensei.folder_path_aliases a
                   JOIN sensei.folders f ON f.id = a.folder_id
             ) c
             WHERE $1 = c.p OR $1 LIKE c.p || '/%'
             ORDER BY length(c.p) DESC, c.live DESC
             LIMIT 1",
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Resolve a filesystem path to its owning INDEXED REPO ROOT — the nearest
    /// `git`/`standalone`/`subtree` ancestor folder — returning `(abs_path,
    /// project_id)`. The file watcher uses this to enqueue incremental tasks
    /// against the correct repo (a change in `~/Dev/kavach/src/x.ts` resolves to
    /// the kavach repo root), exactly as the full scan shapes ProcessFile.
    /// Structural subdirs (workspace_member / folder) are skipped so the one-owner
    /// repo root wins. `None` when the path is under no indexed repo.
    pub async fn repo_root_for_path(
        &self,
        path: &str,
    ) -> Result<Option<(String, Option<uuid::Uuid>)>, String> {
        let row: Option<(String, Option<uuid::Uuid>)> = sqlx_core::query_as::query_as(
            "SELECT abs_path, project_id FROM sensei.folders
              WHERE kind IN ('git','standalone','subtree')
                AND ($1 = abs_path OR $1 LIKE abs_path || '/%')
              ORDER BY length(abs_path) DESC
              LIMIT 1",
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Resolve `(folder_id, project_id)` for a repo path — the importer's
    /// project mapping from a transcript's cwd. Matches the folder whose current
    /// `abs_path` is the path, OR (fallback) whose `folder_path_aliases` includes it
    /// — so a transcript recorded under an OLD path (before a rename/move) still
    /// resolves to the current folder + project. A live abs_path match wins over an
    /// alias. None if the path isn't a tracked folder or a known former path.
    pub async fn get_folder_ids_by_path(
        &self,
        abs_path: &str,
    ) -> Result<Option<(uuid::Uuid, Option<uuid::Uuid>)>, String> {
        let row: Option<(uuid::Uuid, Option<uuid::Uuid>)> = sqlx_core::query_as::query_as(
            "SELECT f.id, f.project_id FROM sensei.folders f
             WHERE f.abs_path = $1
                OR f.id = (SELECT folder_id FROM sensei.folder_path_aliases WHERE alias_abs_path = $1)
             ORDER BY (f.abs_path = $1) DESC
             LIMIT 1"
        ).bind(abs_path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// THE shared repo-anchor mapper (spec 2026-08-18): resolve an absolute path to its
    /// owning repo via the canonical `sensei.repo_anchor_for` SQL function — the ONE
    /// implementation, shared by hook attribution, transcript import, cold-start
    /// synthesis, orphan repair, and the file watcher, so per-event attribution and
    /// batch backfills can never diverge. Nearest repo-kind ancestor (git/subtree/
    /// standalone-project-root; monorepo members roll up to the git root), alias-aware,
    /// deepest+live wins. `None` for a path under no tracked repo (a container dir, an
    /// untracked repo, or a foreign cwd) — the caller decides the unattached policy; it
    /// never fabricates a repo. Retires find_folder_for_path / repo_root_for_path /
    /// get_folder_ids_by_path for attribution.
    pub async fn resolve_repo_anchor(&self, abs_path: &str) -> Result<Option<RepoAnchor>, String> {
        let row: Option<(uuid::Uuid, Option<uuid::Uuid>, String, String)> =
            sqlx_core::query_as::query_as(
                "SELECT repo_folder_id, project_id, repo_abs_path, matched_via \
                 FROM sensei.repo_anchor_for($1)",
            )
            .bind(abs_path)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(|(repo_folder_id, project_id, repo_abs_path, matched_via)| RepoAnchor {
            repo_folder_id,
            project_id,
            repo_abs_path,
            matched_via,
        }))
    }

    /// Register a former absolute path for a folder (`folder_path_aliases`), so a
    /// transcript/hook cwd recorded at the old path still resolves to this folder +
    /// its project after a rename/move. Idempotent (re-registering an alias updates
    /// its reason). `reason` is `rename` (explicit) or `detected` (git-remote match).
    pub async fn add_folder_path_alias(
        &self,
        alias_abs_path: &str,
        folder_id: &uuid::Uuid,
        reason: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.folder_path_aliases (alias_abs_path, folder_id, reason)
             VALUES ($1, $2, $3)
             ON CONFLICT (alias_abs_path) DO UPDATE SET folder_id = EXCLUDED.folder_id, reason = EXCLUDED.reason",
        )
        .bind(alias_abs_path)
        .bind(folder_id)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Metrics: value store + active registry (Phase 3) ──────────────────

    /// Resolve an absolute folder path to its `(folder_id, project_id)` for
    /// project-scoped metric attribution. Thin wrapper over
    /// [`Self::get_folder_ids_by_path`] (matches `folders.abs_path`, then falls
    /// back to a `folder_path_aliases.alias_abs_path`) that additionally REQUIRES a
    /// project: a folder not yet attached to a project can't own a project-scoped
    /// metric, so it resolves to `None` — an honest miss, never a fabricated
    /// project id. Reuses the shared resolver rather than duplicating the
    /// folders/alias SQL.
    pub async fn resolve_folder_by_path(
        &self,
        folder_path: &str,
    ) -> Result<Option<(uuid::Uuid, uuid::Uuid)>, String> {
        Ok(self
            .get_folder_ids_by_path(folder_path)
            .await?
            .and_then(|(folder_id, project_id)| project_id.map(|p| (folder_id, p))))
    }

    /// Link a folder (repo) to a namespace it belongs to. Idempotent.
    pub async fn link_folder_namespace(
        &self,
        folder_id: &uuid::Uuid,
        namespace_id: &uuid::Uuid,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.folder_namespaces(folder_id, namespace_id)
             VALUES($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(folder_id)
        .bind(namespace_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Merge icon metadata onto a folder (icons column is jsonb {emoji,devicon,custom}).
    pub async fn set_folder_icons(
        &self,
        folder_id: &uuid::Uuid,
        icons: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET icons = icons || $2, modified_at = now() WHERE id = $1",
        )
        .bind(folder_id)
        .bind(icons)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The project a folder belongs to, or `None` (unattributed folder).
    pub async fn folder_project_id(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(Option<uuid::Uuid>,)> =
            sqlx_core::query_as::query_as("SELECT project_id FROM sensei.folders WHERE id = $1")
                .bind(folder_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        Ok(row.and_then(|(pid,)| pid))
    }

    /// Resolve a scope identifier (project name, project UUID, or folder name)
    /// to the set of folder ids to query.  A project expands to ALL its folders
    /// (children included).  A bare folder name that has a project expands to
    /// that project's folders; a folder with no project falls back to just
    /// itself.  Returns an empty Vec if nothing matches.
    ///
    /// Resolution order:
    ///   1. `ident` matches a project by name → all that project's folder ids.
    ///   2. `ident` is a valid UUID + project with that id exists → its folders.
    ///   3. `ident` matches a repo/folder by name → if folder has a project_id,
    ///      return that project's folders; else return `[folder.id]`.
    ///   4. No match → empty Vec.
    ///
    /// Note: a bare child-folder name (kind='folder') is not resolvable here —
    /// `get_repo_by_name` only matches git/subtree/standalone roots — so it falls
    /// through to the empty Vec. Callers pass a project name/UUID or a repo name.
    pub async fn scope_folder_ids(&self, ident: &str) -> Result<Vec<uuid::Uuid>, String> {
        // (1) Try project name lookup first.
        if let Some(proj) = self.get_project_by_name(ident).await? {
            let pid = crate::api::util::json_uuid(&proj["id"]).ok_or_else(|| {
                format!("scope_folder_ids: project row missing id for '{}'", ident)
            })?;
            return self.folder_ids_for_project(&pid).await;
        }

        // (2) Try parsing ident as a UUID and look up the project directly.
        if let Ok(uid) = uuid::Uuid::parse_str(ident)
            && self.get_project(&uid).await?.is_some()
        {
            return self.folder_ids_for_project(&uid).await;
        }

        // (3) Try folder/repo lookup by name.
        if let Some(folder) = self.get_repo_by_name(ident).await? {
            let fid = crate::api::util::json_uuid(&folder["id"]).ok_or_else(|| {
                format!("scope_folder_ids: folder row missing id for '{}'", ident)
            })?;
            if let Some(pid) = crate::api::util::json_uuid(&folder["project_id"]) {
                return self.folder_ids_for_project(&pid).await;
            }
            return Ok(vec![fid]);
        }

        // (4) No match.
        Ok(vec![])
    }

    /// Repo-root abs_paths (git / subtree / standalone folders) among the given
    /// scope folder ids — the directories a content grep should walk. Distinct
    /// and path-ordered for determinism. Structural (workspace_member / doc /
    /// component / hook) folders are excluded so we walk repo roots, not subdirs.
    pub async fn scope_repo_roots(&self, ids: &[uuid::Uuid]) -> Result<Vec<String>, String> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT abs_path FROM sensei.folders
             WHERE id = ANY($1)
               AND kind IN ('git'::sensei.folder_kind, 'subtree'::sensei.folder_kind, 'standalone'::sensei.folder_kind)
             ORDER BY abs_path",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(p,)| p).collect())
    }

    // ── Project-scoped query variants (#60) ───────────────────────────

    /// Folder rows for a set of folder ids (7.2) — the structural skeleton of the
    /// `/tree` endpoint: `kind`/`role`/`parent_id` drive the folder hierarchy
    /// (repo root → sub-projects/subtrees → subfolders) that the node subtrees
    /// hang off.
    pub async fn get_folders_scoped(
        &self,
        folder_ids: &[uuid::Uuid],
    ) -> Result<Vec<serde_json::Value>, String> {
        if folder_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(uuid::Uuid, String, Option<String>, String, String, Option<uuid::Uuid>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, kind::text, role::text, name, abs_path, parent_id FROM sensei.folders
              WHERE id = ANY($1) ORDER BY abs_path",
            )
            .bind(folder_ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, role, name, abs_path, parent_id)| {
            serde_json::json!({ "id": id, "kind": kind, "role": role, "name": name, "abs_path": abs_path, "parent_id": parent_id })
        }).collect())
    }
}
