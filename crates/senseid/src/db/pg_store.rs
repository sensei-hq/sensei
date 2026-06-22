use std::time::Duration;
use sqlx_postgres::{PgPool, PgPoolOptions};
use sensei_bootstrap::{DB_POOL_MAX_CONNECTIONS, DB_POOL_ACQUIRE_TIMEOUT_SECS, DB_POOL_IDLE_TIMEOUT_SECS};

/// PostgreSQL store.
/// Schema is managed by `dbd apply`, not by this code.
#[derive(Clone)]
pub struct PgStore {
    pool: PgPool,
}

pub struct InsertMemory {
    pub project_id:    Option<uuid::Uuid>,
    pub scope:         String,
    pub scope_filter:  Option<String>,
    pub mtype:         String,    // memory_type enum value
    pub title:         String,
    pub content:       String,
    pub impact:        Option<String>,
    pub tags:          Vec<String>,
    pub triage_signal: Option<String>,
    pub status:        String,    // memory_status enum value
    // Governance plane: where the rule applies (namespace) + its authority.
    pub namespace_id:  Option<uuid::Uuid>,
    pub enforcement:   Option<String>, // enforcement enum value; None → DB default 'recommended'
    pub origin:        Option<String>, // None → DB default 'learned'
    pub source_id:     Option<uuid::Uuid>, // provenance: knowledge_sources.id for origin='federated'
}

pub struct OutcomeRow {
    pub memory_id:  uuid::Uuid,
    pub session_id: Option<uuid::Uuid>,
    pub outcome:    String,
    pub context:    Option<String>,
}

/// Input for registering a federation endpoint.
pub struct NewKnowledgeSource {
    pub kind:           String,
    pub name:           String,
    pub url:            String,
    pub namespace_id:   Option<uuid::Uuid>,
    pub credential_ref: String,
    pub direction:      String, // push | pull | both
}

/// A registered federation endpoint (row of sensei.knowledge_sources).
#[derive(Debug, Clone)]
pub struct KnowledgeSource {
    pub id:             uuid::Uuid,
    pub kind:           String,
    pub name:           String,
    pub url:            String,
    pub namespace_id:   Option<uuid::Uuid>,
    pub credential_ref: String,
    pub direction:      String,
    pub last_seq:       i64,
    pub enabled:        bool,
}

/// A federated_memories ledger row.
#[derive(Debug, Clone)]
pub struct FederatedLink {
    pub memory_id:  Option<uuid::Uuid>,
    pub remote_seq: i64,
}

/// Snapshot needed to publish a memory to a hive (+ namespace identity + origin/scope_key for gating).
#[derive(Debug, Clone)]
pub struct MemoryPushPayload {
    pub title:       String,
    pub content:     String,
    pub impact:      Option<String>,
    pub enforcement: String,
    pub rule_type:   String,
    pub origin:      String,
    pub scope_key:   String,
    pub slug:        String,
    pub name:        String,
}

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
// PgStore API surface — methods wired up incrementally; SQLx tuple return types
// are inherently verbose and adding an extra layer of type aliases would
// not improve readability at the call sites.
impl PgStore {
    /// Connect to a PostgreSQL database using the shared pool defaults from
    /// [`sensei_bootstrap`] (`DB_POOL_MAX_CONNECTIONS`, `DB_POOL_ACQUIRE_TIMEOUT_SECS`,
    /// `DB_POOL_IDLE_TIMEOUT_SECS`).
    pub async fn connect(database_url: &str) -> Result<Self, String> {
        let pool = PgPoolOptions::new()
            .max_connections(DB_POOL_MAX_CONNECTIONS)
            .acquire_timeout(Duration::from_secs(DB_POOL_ACQUIRE_TIMEOUT_SECS))
            .idle_timeout(Duration::from_secs(DB_POOL_IDLE_TIMEOUT_SECS))
            .connect(database_url)
            .await
            .map_err(|e| format!("PgStore connect: {}", e))?;
        Ok(Self { pool })
    }

    /// Connect to the test database. Uses TEST_DATABASE_URL or defaults to sensei_test.
    pub async fn connect_test() -> Result<Self, String> {
        let url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| format!("postgresql://localhost:{}/sensei_test", sensei_bootstrap::POSTGRES_PORT));
        Self::connect(&url).await
    }

    /// Get a reference to the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ── Config ────────────────────────────────────────────────────────

    pub async fn get_config(&self, key: &str) -> Result<Option<String>, String> {
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT value FROM sensei.config WHERE key = $1"
        )
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    pub async fn set_config(&self, key: &str, value: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.config(key, value) VALUES($1, $2) ON CONFLICT(key) DO UPDATE SET value = EXCLUDED.value"
        )
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn delete_config(&self, key: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.config WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_all_config(&self) -> Result<std::collections::HashMap<String, String>, String> {
        let rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT key, value FROM sensei.config"
        )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().collect())
    }

    // ── Tags (controlled vocabulary) ──────────────────────────────────

    pub async fn add_tag(&self, tag: &str, category: Option<&str>) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.tags(tag, category) VALUES($1, $2) ON CONFLICT(tag) DO UPDATE SET category = EXCLUDED.category, modified_at = now()"
        )
            .bind(tag)
            .bind(category)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn remove_tag(&self, tag: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.tags WHERE tag = $1")
            .bind(tag)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn list_tags(&self) -> Result<Vec<(String, Option<String>)>, String> {
        sqlx_core::query_as::query_as("SELECT tag, category FROM sensei.tags ORDER BY tag")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn list_tags_by_category(&self, category: &str) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT tag FROM sensei.tags WHERE category = $1 ORDER BY tag"
        )
            .bind(category)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    // ── Workflow State ────────────────────────────────────────────────

    pub async fn upsert_workflow_state(
        &self, project: &str, phase: Option<&str>, plan: Option<&str>,
        task: Option<&str>, issue: Option<i64>, checkpoint: Option<&str>,
        rules_hash: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.workflow_state(project, active_phase, active_plan, active_task, active_issue, last_checkpoint, rules_hash, updated_at)
             VALUES($1, $2, $3, $4, $5, $6, $7, now())
             ON CONFLICT(project) DO UPDATE SET
               active_phase = COALESCE($2, workflow_state.active_phase),
               active_plan = COALESCE($3, workflow_state.active_plan),
               active_task = COALESCE($4, workflow_state.active_task),
               active_issue = COALESCE($5, workflow_state.active_issue),
               last_checkpoint = COALESCE($6, workflow_state.last_checkpoint),
               rules_hash = COALESCE($7, workflow_state.rules_hash),
               updated_at = now()"
        )
            .bind(project).bind(phase).bind(plan).bind(task)
            .bind(issue).bind(checkpoint).bind(rules_hash)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_workflow_state(&self, project: &str) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(
            Option<String>, Option<String>, Option<String>,
            Option<i32>, Option<String>, Option<String>, chrono::DateTime<chrono::Utc>,
        )> = sqlx_core::query_as::query_as(
            "SELECT active_phase, active_plan, active_task, active_issue, last_checkpoint, rules_hash, updated_at
             FROM sensei.workflow_state WHERE project = $1"
        )
            .bind(project)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(row.map(|(phase, plan, task, issue, checkpoint, hash, updated)| {
            serde_json::json!({
                "project": project,
                "active_phase": phase,
                "active_plan": plan,
                "active_task": task,
                "active_issue": issue,
                "last_checkpoint": checkpoint,
                "rules_hash": hash,
                "updated_at": updated.to_rfc3339(),
            })
        }))
    }

    pub async fn delete_workflow_state(&self, project: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.workflow_state WHERE project = $1")
            .bind(project)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── PG Function Wrappers ───────────────────────────────────────────

    /// BM25-style keyword ranking: matches nodes by name/signature/docstring.
    pub async fn rank_bm25(&self, folder_id: &uuid::Uuid, query: &str) -> Result<Vec<(String, f64)>, String> {
        let rows: Vec<(String, f64)> = sqlx_core::query_as::query_as(
            "SELECT file_path, score FROM sensei.rank_bm25($1, $2)"
        ).bind(folder_id).bind(query)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    // ── Graph (typed wrappers) ─────────────────────────────────────────

    pub async fn merge_function(
        &self, folder_id: &uuid::Uuid, name: &str, file_path: &str,
        signature: Option<&str>, line_start: Option<i32>, line_end: Option<i32>,
        parent_id: Option<&uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node(folder_id, "function", name, file_path, parent_id, signature, line_start, line_end).await
    }

    pub async fn merge_file(
        &self, folder_id: &uuid::Uuid, name: &str, file_path: &str,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node(folder_id, "file", name, file_path, None, None, None, None).await
    }

    pub async fn merge_type(
        &self, folder_id: &uuid::Uuid, name: &str, file_path: &str,
        kind: &str, line_start: Option<i32>,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node(folder_id, kind, name, file_path, None, None, line_start, None).await
    }

    pub async fn merge_doc(
        &self, folder_id: &uuid::Uuid, name: &str, file_path: &str,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node(folder_id, "doc", name, file_path, None, None, None, None).await
    }

    pub async fn project_exists(&self, folder_id: &uuid::Uuid) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM sensei.nodes WHERE folder_id = $1)"
        ).bind(folder_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn search_functions(&self, folder_id: &uuid::Uuid, query: &str) -> Result<Vec<serde_json::Value>, String> {
        self.search_functions_scoped(std::slice::from_ref(folder_id), query).await
    }

    pub async fn search_types(&self, folder_id: &uuid::Uuid, query: &str) -> Result<Vec<serde_json::Value>, String> {
        self.search_types_scoped(std::slice::from_ref(folder_id), query).await
    }

    pub async fn count_nodes_by_kind(&self, folder_id: &uuid::Uuid) -> Result<std::collections::HashMap<String, i64>, String> {
        self.count_nodes_by_kind_scoped(std::slice::from_ref(folder_id)).await
    }

    pub async fn delete_node(&self, node_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE id = $1")
            .bind(node_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn delete_nodes_by_file(&self, folder_id: &uuid::Uuid, file_path: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE folder_id = $1 AND file_path = $2")
            .bind(folder_id).bind(file_path).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn clear_all_nodes(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        self.delete_nodes_by_folder(folder_id).await
    }

    // ── Repo (folders with kind='git'/'subtree') ──────────────────────

    /// Register a git repo as a folder. Equivalent to old upsert_repo_basic.
    pub async fn upsert_repo(&self, root_id: &uuid::Uuid, name: &str, abs_path: &str) -> Result<uuid::Uuid, String> {
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
    pub async fn upsert_repo_kind(&self, root_id: &uuid::Uuid, kind: &str, name: &str, abs_path: &str) -> Result<uuid::Uuid, String> {
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
    /// to its parent folder. Status is terminal (`indexed`) — these rows model
    /// the filesystem tree, not scan progress. On conflict the kind is preserved
    /// so a path that is actually a (nested) project root is never reclassified.
    pub async fn upsert_subfolder(
        &self, root_id: &uuid::Uuid, name: &str, path: &str, abs_path: &str,
        parent_id: Option<&uuid::Uuid>, project_id: Option<&uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders(root_id, kind, status, name, path, abs_path, parent_id, project_id)
             VALUES($1, 'folder'::sensei.folder_kind, 'indexed'::sensei.folder_status, $2, $3, $4, $5, $6)
             ON CONFLICT(abs_path) DO UPDATE SET
                name = EXCLUDED.name,
                parent_id = COALESCE(EXCLUDED.parent_id, folders.parent_id),
                project_id = COALESCE(EXCLUDED.project_id, folders.project_id),
                modified_at = now()
             RETURNING id"
        )
            .bind(root_id).bind(name).bind(path).bind(abs_path).bind(parent_id).bind(project_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Get a repo (folder with kind='git'/'subtree') by abs_path.
    pub async fn get_repo_by_path(&self, abs_path: &str) -> Result<Option<serde_json::Value>, String> {
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

    /// Set folder props (metadata like stack, libs, indexed_at, etc.).
    pub async fn set_folder_props(&self, folder_id: &uuid::Uuid, props: &serde_json::Value) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET props = props || $2, modified_at = now() WHERE id = $1"
        ).bind(folder_id).bind(props).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Assign a folder to a project with role/label.
    pub async fn set_folder_project(&self, folder_id: &uuid::Uuid, project_id: &uuid::Uuid, role: &str, label: Option<&str>) -> Result<(), String> {
        let props = serde_json::json!({"role": role, "label": label});
        sqlx_core::query::query(
            "UPDATE sensei.folders SET project_id = $2, props = props || $3, modified_at = now() WHERE id = $1"
        ).bind(folder_id).bind(project_id).bind(props).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update only the `role` column on a folder. Used by the Projects
    /// setup stage when the user picks a role from the dropdown — distinct
    /// from set_folder_project (which also reassigns project membership).
    pub async fn update_folder_role(&self, folder_id: &uuid::Uuid, role: Option<&str>) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET role = $2::sensei.folder_role, modified_at = now() WHERE id = $1"
        ).bind(folder_id).bind(role).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// All folders belonging to a project, ordered by path. Used to enrich
    /// /api/projects responses with folder membership so the Projects setup
    /// page can render per-folder details.
    pub async fn list_folders_by_project(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, path, abs_path, role::text
             FROM sensei.folders
             WHERE project_id = $1
             ORDER BY path"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, path, abs, role)| {
            serde_json::json!({
                "id": id, "kind": kind, "name": name,
                "path": path, "abs_path": abs, "role": role,
            })
        }).collect())
    }

    /// Mark a folder as indexed with detected libs.
    pub async fn mark_folder_indexed(&self, folder_id: &uuid::Uuid, libs: &[String]) -> Result<(), String> {
        let props = serde_json::json!({"indexed_at": chrono::Utc::now().to_rfc3339(), "libs": libs});
        sqlx_core::query::query(
            "UPDATE sensei.folders SET status = 'indexed'::sensei.folder_status, props = props || $2, modified_at = now() WHERE id = $1"
        ).bind(folder_id).bind(&props).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
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

    pub async fn upsert_node(
        &self, folder_id: &uuid::Uuid, kind: &str, name: &str, file_path: &str,
        parent_id: Option<&uuid::Uuid>, signature: Option<&str>,
        line_start: Option<i32>, line_end: Option<i32>,
    ) -> Result<uuid::Uuid, String> {
        // ON CONFLICT targets nodes_unique_identity (folder_id, file_path, kind, name,
        // parent_id, line_start NULLS NOT DISTINCT).  DO UPDATE keeps the row stable
        // on re-scans — same UUID returned whether the row was just inserted or already
        // existed — and refreshes mutable fields (signature, line_end, modified_at).
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.nodes(folder_id, kind, name, file_path, parent_id, signature, line_start, line_end)
             VALUES($1, $2::sensei.node_kind, $3, $4, $5, $6, $7, $8)
             ON CONFLICT ON CONSTRAINT nodes_unique_identity DO UPDATE
               SET signature   = EXCLUDED.signature,
                   line_end    = EXCLUDED.line_end,
                   modified_at = now()
             RETURNING id"
        ).bind(folder_id).bind(kind).bind(name).bind(file_path)
            .bind(parent_id).bind(signature).bind(line_start).bind(line_end)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn get_nodes_by_folder(&self, folder_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        self.get_nodes_scoped(std::slice::from_ref(folder_id)).await
    }

    /// Nodes in a folder that still need an embedding, restricted to the kinds
    /// worth embedding (code symbols + files + doc sections). Returns
    /// `(id, kind, name, signature, file_path)` — the fields needed to build the
    /// embedding text. Used by the `EmbedNodes` task.
    pub async fn nodes_without_embeddings(
        &self,
        folder_id: &uuid::Uuid,
        limit: i64,
    ) -> Result<Vec<(uuid::Uuid, String, String, Option<String>, String)>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, kind::text, name, signature, file_path
                   FROM sensei.nodes
                  WHERE folder_id = $1
                    AND embedding IS NULL
                    AND kind IN ('file','function','method','class','interface',
                                 'type','const','enum','enum_variant','section',
                                 'struct','component','hook','doc','extension')
                  ORDER BY file_path, line_start
                  LIMIT $2",
            )
            .bind(folder_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Store a node's vector embedding. The slice is rendered to pgvector's text
    /// form (`[v1,v2,...]`) and cast to `vector`, so no pgvector crate is needed.
    pub async fn set_node_embedding(
        &self,
        node_id: &uuid::Uuid,
        embedding: &[f32],
    ) -> Result<(), String> {
        use std::fmt::Write as _;
        let mut buf = String::with_capacity(embedding.len() * 8 + 2);
        buf.push('[');
        for (i, v) in embedding.iter().enumerate() {
            if i > 0 {
                buf.push(',');
            }
            let _ = write!(buf, "{v}");
        }
        buf.push(']');
        sqlx_core::query::query("UPDATE sensei.nodes SET embedding = $1::vector WHERE id = $2")
            .bind(buf)
            .bind(node_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Find near-duplicate function/method pairs within a folder by cosine
    /// similarity on their code embeddings (HNSW `<=>` cosine distance). Each
    /// pair is returned once (`a.id < b.id`) at or above `min_similarity`,
    /// strongest first. Trivial functions (< 4 lines) are skipped — they bound
    /// the O(n²) self-join and avoid false positives from boilerplate. On-demand
    /// review query, not a hot path.
    pub async fn find_duplicates(&self, folder_id: &uuid::Uuid, min_similarity: f64, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let max_distance = 1.0 - min_similarity;
        let rows: Vec<(String, String, Option<i32>, String, String, Option<i32>, f64)> =
            sqlx_core::query_as::query_as(
                "SELECT a.name, a.file_path, a.line_start,
                        b.name, b.file_path, b.line_start,
                        1 - (a.embedding <=> b.embedding) AS similarity
                   FROM sensei.nodes a
                   JOIN sensei.nodes b
                     ON b.folder_id = a.folder_id
                    AND a.id < b.id
                    AND b.kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
                    AND b.embedding IS NOT NULL
                    AND (b.line_end - b.line_start) >= 3
                  WHERE a.folder_id = $1
                    AND a.kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
                    AND a.embedding IS NOT NULL
                    AND (a.line_end - a.line_start) >= 3
                    AND (a.embedding <=> b.embedding) <= $2
                  ORDER BY similarity DESC
                  LIMIT $3",
            )
            .bind(folder_id)
            .bind(max_distance)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(na, fa, la, nb, fb, lb, sim)| {
            serde_json::json!({
                "a": { "name": na, "file": fa, "line": la },
                "b": { "name": nb, "file": fb, "line": lb },
                "similarity": (sim * 10000.0).round() / 10000.0,
            })
        }).collect())
    }

    /// Abs paths of folders that still have embeddable nodes without an
    /// embedding. Used by the backfill endpoint to enqueue `EmbedNodes` for
    /// already-indexed folders (which a normal incremental scan won't revisit).
    pub async fn folders_with_pending_embeddings(&self) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT f.abs_path
               FROM sensei.nodes n
               JOIN sensei.folders f ON f.id = n.folder_id
              WHERE n.embedding IS NULL
                AND n.kind IN ('file','function','method','class','interface',
                               'type','const','enum','enum_variant','section',
                               'struct','component','hook','doc','extension')",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(p,)| p).collect())
    }

    pub async fn get_nodes_by_file(&self, folder_id: &uuid::Uuid, file_path: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<uuid::Uuid>, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, parent_id, line_start FROM sensei.nodes WHERE folder_id = $1 AND file_path = $2 ORDER BY line_start"
        ).bind(folder_id).bind(file_path).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, pid, ls)| {
            serde_json::json!({ "id": id, "kind": kind, "name": name, "parent_id": pid, "line_start": ls })
        }).collect())
    }

    pub async fn delete_nodes_by_folder(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE folder_id = $1")
            .bind(folder_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn update_node_community(&self, node_id: &uuid::Uuid, community_id: i32) -> Result<(), String> {
        sqlx_core::query::query("UPDATE sensei.nodes SET community_id = $2, modified_at = now() WHERE id = $1")
            .bind(node_id).bind(community_id)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Edges ────────────────────────────────────────────────────────

    pub async fn insert_edge(
        &self, folder_id: &uuid::Uuid, source_id: &uuid::Uuid,
        target_id: Option<&uuid::Uuid>, target_name: Option<&str>,
        kind: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.edges(folder_id, source_id, target_id, target_name, kind) VALUES($1, $2, $3, $4, $5::sensei.edge_kind) RETURNING id"
        ).bind(folder_id).bind(source_id).bind(target_id).bind(target_name).bind(kind)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn get_callers(&self, node_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT e.id, e.source_id, e.kind::text FROM sensei.edges e WHERE e.target_id = $1 AND e.kind = 'calls'::sensei.edge_kind"
        ).bind(node_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, src, kind)| {
            serde_json::json!({ "edge_id": id, "caller_id": src, "kind": kind })
        }).collect())
    }

    pub async fn get_callees(&self, node_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, Option<uuid::Uuid>, Option<String>, String)> = sqlx_core::query_as::query_as(
            "SELECT e.id, e.target_id, e.target_name, e.kind::text FROM sensei.edges e WHERE e.source_id = $1 AND e.kind = 'calls'::sensei.edge_kind"
        ).bind(node_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, tgt, name, kind)| {
            serde_json::json!({ "edge_id": id, "callee_id": tgt, "callee_name": name, "kind": kind })
        }).collect())
    }

    /// Update an unresolved edge with a resolved target_id.
    pub async fn resolve_edge(&self, edge_id: &uuid::Uuid, target_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("UPDATE sensei.edges SET target_id = $2, modified_at = now() WHERE id = $1")
            .bind(edge_id).bind(target_id)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Un-resolve edges that point INTO a file's nodes: clear `target_id` while
    /// keeping `target_name`. Called before re-indexing a changed file so the
    /// inbound cross-file edges survive (they'd otherwise be cascade-deleted when
    /// the target nodes are dropped) and are re-pointed by `resolve_edges` once
    /// the file's new nodes exist. Returns the number of edges un-resolved.
    pub async fn unresolve_edges_to_file(&self, folder_id: &uuid::Uuid, file_path: &str) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.edges SET target_id = NULL, modified_at = now()
              WHERE folder_id = $1
                AND target_id IN (SELECT id FROM sensei.nodes WHERE folder_id = $1 AND file_path = $2)
                AND target_name IS NOT NULL"
        ).bind(folder_id).bind(file_path).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    pub async fn get_edges_by_kind(&self, folder_id: &uuid::Uuid, kind: &str) -> Result<Vec<serde_json::Value>, String> {
        self.get_edges_scoped(std::slice::from_ref(folder_id), kind).await
    }

    // ── View-based graph queries ────────────────────────────────────

    /// Find callers of a function by name via the call_graph view.
    /// `scope` is resolved via [`scope_folder_ids`]: a project name/UUID expands
    /// to all of that project's folders; a bare folder name falls back to just
    /// that folder.
    pub async fn get_callers_by_name(&self, scope: &str, target: &str) -> Result<Vec<serde_json::Value>, String> {
        let folder_ids = self.scope_folder_ids(scope).await?;
        if folder_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(String, String, String, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT source_name, source_kind::text, source_file, source_line
               FROM sensei.call_graph
              WHERE folder_id = ANY($1) AND target_name = $2 AND edge_kind = 'calls'
              ORDER BY source_file, source_line LIMIT 100"
        ).bind(&folder_ids[..]).bind(target).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, kind, file, line)| {
            serde_json::json!({ "name": name, "kind": kind, "file_path": file, "line_start": line })
        }).collect())
    }

    /// Find callees of a function by name via the call_graph view.
    /// `scope` is resolved via [`scope_folder_ids`]: a project name/UUID expands
    /// to all of that project's folders; a bare folder name falls back to just
    /// that folder.
    pub async fn get_callees_by_name(&self, scope: &str, source: &str) -> Result<Vec<serde_json::Value>, String> {
        let folder_ids = self.scope_folder_ids(scope).await?;
        if folder_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(Option<String>, Option<String>, Option<String>, Option<i32>, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT target_name, target_kind::text, target_file, target_line, unresolved_target
               FROM sensei.call_graph
              WHERE folder_id = ANY($1) AND source_name = $2 AND edge_kind = 'calls'
              ORDER BY target_file, target_line LIMIT 100"
        ).bind(&folder_ids[..]).bind(source).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, kind, file, line, unresolved)| {
            let display_name = name.or(unresolved).unwrap_or_default();
            serde_json::json!({ "name": display_name, "kind": kind, "file_path": file, "line_start": line })
        }).collect())
    }

    /// Get files matching a tag via the file_tags view.
    pub async fn get_files_by_tag(&self, folder_name: &str, tag: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Vec<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, file_path, tags FROM sensei.file_tags
              WHERE folder = $1 AND $2 = ANY(tags)
              ORDER BY file_path LIMIT 200"
        ).bind(folder_name).bind(tag).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, fp, tags)| {
            serde_json::json!({ "id": id, "file_path": fp, "tags": tags })
        }).collect())
    }

    /// Get doc coverage with drift detection via the doc_coverage view.
    pub async fn get_doc_drift(&self, folder_name: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, String, String, bool)> = sqlx_core::query_as::query_as(
            "SELECT doc_name, doc_file, code_name, code_file, drifted
               FROM sensei.doc_coverage
              WHERE folder = $1
              ORDER BY drifted DESC, doc_file LIMIT 200"
        ).bind(folder_name).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(doc_name, doc_file, code_name, code_file, drifted)| {
            serde_json::json!({ "doc": doc_name, "docFile": doc_file, "code": code_name, "codeFile": code_file, "drifted": drifted })
        }).collect())
    }

    /// Count all edges across multiple folders (project-scoped variant).
    pub async fn count_edges_scoped(&self, folder_ids: &[uuid::Uuid]) -> Result<i64, String> {
        let row: (i64,) = sqlx_core::query_as::query_as(
            "SELECT COUNT(*) FROM sensei.edges WHERE folder_id = ANY($1)"
        ).bind(folder_ids).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Count all edges for a folder.
    pub async fn count_edges(&self, folder_id: &uuid::Uuid) -> Result<i64, String> {
        self.count_edges_scoped(&[*folder_id]).await
    }

    /// Delete nodes whose file_path starts with a given prefix (for folder deletion).
    pub async fn delete_nodes_by_path_prefix(&self, folder_id: &uuid::Uuid, prefix: &str) -> Result<u64, String> {
        let result = sqlx_core::query::query(
            "DELETE FROM sensei.nodes WHERE folder_id = $1 AND file_path LIKE $2 || '%'"
        ).bind(folder_id).bind(prefix).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(result.rows_affected())
    }

    /// Search libraries by name (ILIKE).
    pub async fn search_libraries(&self, query: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, name, ecosystem::text, description FROM sensei.libraries
             WHERE name ILIKE '%' || $1 || '%'
             ORDER BY name LIMIT 50"
        ).bind(query).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, eco, desc)| {
            serde_json::json!({ "id": id, "name": name, "ecosystem": eco, "description": desc })
        }).collect())
    }

    /// Get a single library by exact name.
    pub async fn get_library_by_name(&self, name: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, name, ecosystem::text, description FROM sensei.libraries
             WHERE name = $1
             ORDER BY name"
        ).bind(name).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, eco, desc)| {
            serde_json::json!({ "id": id, "name": name, "ecosystem": eco, "description": desc })
        }).collect())
    }

    /// List all sessions across all folders.
    pub async fn list_all_sessions(&self, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        // Join the project name so each session can be labelled, and return the
        // timestamps in the camelCase shape the SessionData wire type and the
        // observatory components actually read (startedAt / completedAt). The
        // old shape returned folder_id (a bare uuid, never a project name) and
        // snake_case `started_at` with no `completed_at`, so every displayed
        // column — project, task time, duration — came back blank (#61).
        type SessionRow = (
            uuid::Uuid, Option<String>, String, Option<String>, Option<String>,
            Option<bool>, i32, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>,
        );
        let rows: Vec<SessionRow> = sqlx_core::query_as::query_as(
            "SELECT s.id, p.name, s.task, s.summary, s.outcome::text, s.ftr, s.turns,
                    s.started_at, s.completed_at
             FROM activity.sessions s
             LEFT JOIN sensei.projects p ON p.id = s.project_id
             ORDER BY s.started_at DESC LIMIT $1"
        ).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, project, task, summary, outcome, ftr, turns, started, completed)| {
            serde_json::json!({
                "id": id,
                "project": project,
                "task": task,
                "summary": summary,
                "outcome": outcome,
                "ftr": ftr,
                "turns": turns,
                "startedAt": started.to_rfc3339(),
                "completedAt": completed.map(|c| c.to_rfc3339()),
            })
        }).collect())
    }

    // ── Extensions ────────────────────────────────────────────────────

    pub async fn create_extension(
        &self, kind: &str, name: &str, description: Option<&str>, content: Option<&str>,
        scope: &str, source: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.extensions(kind, name, description, content, scope, source)
             VALUES($1::sensei.extension_kind, $2, $3, $4, $5::sensei.extension_scope, $6::sensei.extension_source) RETURNING id"
        ).bind(kind).bind(name).bind(description).bind(content).bind(scope).bind(source)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn update_extension(&self, id: &uuid::Uuid, description: Option<&str>, content: Option<&str>) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.extensions SET description = COALESCE($2, description), content = COALESCE($3, content) WHERE id = $1"
        ).bind(id).bind(description).bind(content)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn list_extensions_by_kind(&self, kind: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, String, String, bool)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, description, scope::text, source::text, enabled FROM sensei.extensions WHERE kind = $1::sensei.extension_kind ORDER BY name"
        ).bind(kind).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, desc, scope, source, enabled)| {
            serde_json::json!({ "id": id, "kind": kind, "name": name, "description": desc, "scope": scope, "source": source, "enabled": enabled })
        }).collect())
    }

    pub async fn get_extension_history(&self, extension_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, i32, String, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT id, operation::text, revision, name, changed_at FROM history.past_extensions WHERE extension_id = $1 ORDER BY changed_at DESC"
        ).bind(extension_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, op, rev, name, ts)| {
            serde_json::json!({ "id": id, "operation": op, "revision": rev, "name": name, "changed_at": ts.to_rfc3339() })
        }).collect())
    }

    pub async fn delete_extension(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.extensions WHERE id = $1")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Folders ──────────────────────────────────────────────────────

    pub async fn upsert_folder(
        &self, root_id: &uuid::Uuid, kind: &str, name: &str, path: &str, abs_path: &str,
        parent_id: Option<&uuid::Uuid>, project_id: Option<&uuid::Uuid>,
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

    pub async fn list_folders_by_root(&self, root_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, Option<uuid::Uuid>)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, path, abs_path, project_id FROM sensei.folders WHERE root_id = $1 ORDER BY path"
        ).bind(root_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, path, abs, pid)| {
            serde_json::json!({ "id": id, "kind": kind, "name": name, "path": path, "abs_path": abs, "project_id": pid })
        }).collect())
    }

    pub async fn delete_folder_tree(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        // CASCADE will handle children via parent_id FK
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
            .bind(folder_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// List folders that were registered by a scan but never finished
    /// indexing — i.e. status is `discovered` (scan ran, ProcessGitFolder
    /// hadn't started) or `queued` (mid-flight when the daemon stopped).
    /// `indexing`, `indexed`, `failed`, and `deferred` are excluded.
    ///
    /// Called once at daemon startup to rebuild the in-memory queue, which
    /// otherwise loses every task on restart.
    pub async fn list_pending_folders(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, String, String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT id, root_id, kind::text, name, abs_path, status::text \
             FROM sensei.folders \
             WHERE status IN ('discovered'::sensei.folder_status, 'queued'::sensei.folder_status) \
             ORDER BY abs_path"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, root_id, kind, name, abs_path, status)| {
            serde_json::json!({
                "id": id,
                "root_id": root_id,
                "kind": kind,
                "name": name,
                "abs_path": abs_path,
                "status": status,
            })
        }).collect())
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

    pub async fn create_benchmark_report(
        &self, folder_id: Option<&uuid::Uuid>, run_name: &str, strategy: &str,
        score: Option<f64>, tokens: Option<i32>, elapsed_ms: Option<i32>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.benchmark_reports(folder_id, run_name, strategy, score, tokens, elapsed_ms) VALUES($1, $2, $3, $4, $5, $6) RETURNING id"
        ).bind(folder_id).bind(run_name).bind(strategy).bind(score).bind(tokens).bind(elapsed_ms)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_benchmark_reports(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<f64>, Option<i32>, bool, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT id, run_name, strategy, score::float8, tokens, promoted, modified_at FROM sensei.benchmark_reports ORDER BY modified_at DESC"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, strategy, score, tokens, promoted, modified)| {
            serde_json::json!({ "id": id, "run_name": name, "strategy": strategy, "score": score, "tokens": tokens, "promoted": promoted, "modified_at": modified.to_rfc3339() })
        }).collect())
    }

    // ── Views (read-only) ────────────────────────────────────────────

    pub async fn list_repositories(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT id, name, abs_path, kind::text FROM sensei.folders WHERE kind IN ('git'::sensei.folder_kind, 'subtree'::sensei.folder_kind, 'standalone'::sensei.folder_kind) ORDER BY name"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, abs_path, kind)| {
            serde_json::json!({ "id": id, "name": name, "abs_path": abs_path, "kind": kind })
        }).collect())
    }

    // ── Memories ──────────────────────────────────────────────────────

    pub async fn create_memory(
        &self, project_id: Option<&uuid::Uuid>, scope: &str, scope_filter: Option<&str>,
        mem_type: &str, title: &str, content: &str, impact: Option<&str>,
        session_id: Option<&uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memories(project_id, scope, scope_filter, type, title, content, impact, session_id)
             VALUES($1, $2::sensei.memory_scope, $3, $4::sensei.memory_type, $5, $6, $7, $8) RETURNING id"
        ).bind(project_id).bind(scope).bind(scope_filter).bind(mem_type)
            .bind(title).bind(content).bind(impact).bind(session_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn reinforce_memory(&self, id: &uuid::Uuid, amount: f64) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.memories SET strength = LEAST(strength + $2, 5.0), modified_at = now() WHERE id = $1"
        ).bind(id).bind(amount).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn archive_memory(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.memories SET status = 'archived'::sensei.memory_status, modified_at = now() WHERE id = $1"
        ).bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_memory(&self, id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, String, String, Option<String>, f64, String, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, project_id, scope::text, scope_filter, type::text, title, content, impact, strength::float8, status::text, modified_at FROM sensei.memories WHERE id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id, pid, scope, filter, mtype, title, content, impact, strength, status, modified)| {
            serde_json::json!({
                "id": id, "project_id": pid, "scope": scope, "scope_filter": filter,
                "type": mtype, "title": title, "content": content, "impact": impact,
                "strength": strength, "status": status, "modified_at": modified.to_rfc3339(),
            })
        }))
    }

    pub async fn list_active_memories(&self, project_id: Option<&uuid::Uuid>, scope: Option<&str>) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, String, String, String, Option<String>, f64)> = match (project_id, scope) {
            (Some(pid), Some(s)) => sqlx_core::query_as::query_as(
                "SELECT id, scope::text, scope_filter, type::text, title, content, impact, strength::float8
                 FROM sensei.memories WHERE status = 'active' AND strength >= 1.0 AND (project_id = $1 OR project_id IS NULL) AND scope = $2::sensei.memory_scope
                 ORDER BY strength DESC"
            ).bind(pid).bind(s).fetch_all(&self.pool).await,
            (Some(pid), None) => sqlx_core::query_as::query_as(
                "SELECT id, scope::text, scope_filter, type::text, title, content, impact, strength::float8
                 FROM sensei.memories WHERE status = 'active' AND strength >= 1.0 AND (project_id = $1 OR project_id IS NULL)
                 ORDER BY strength DESC"
            ).bind(pid).fetch_all(&self.pool).await,
            _ => sqlx_core::query_as::query_as(
                "SELECT id, scope::text, scope_filter, type::text, title, content, impact, strength::float8
                 FROM sensei.memories WHERE status = 'active' AND strength >= 1.0 AND project_id IS NULL
                 ORDER BY strength DESC"
            ).fetch_all(&self.pool).await,
        }.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, scope, filter, mtype, title, content, impact, strength)| {
            serde_json::json!({ "id": id, "scope": scope, "scope_filter": filter, "type": mtype, "title": title, "content": content, "impact": impact, "strength": strength })
        }).collect())
    }

    // ── Memory Examples ──────────────────────────────────────────────

    pub async fn add_memory_example(&self, memory_id: &uuid::Uuid, node_id: &str, is_good: bool, note: Option<&str>) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memory_examples(memory_id, node_id, is_good, note) VALUES($1, $2, $3, $4) RETURNING id"
        ).bind(memory_id).bind(node_id).bind(is_good).bind(note)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_memory_examples(&self, memory_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, bool, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, node_id, is_good, note FROM sensei.memory_examples WHERE memory_id = $1"
        ).bind(memory_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, nid, good, note)| {
            serde_json::json!({ "id": id, "node_id": nid, "is_good": good, "note": note })
        }).collect())
    }

    // ── Memory Evidence ──────────────────────────────────────────────

    pub async fn add_memory_evidence(&self, memory_id: &uuid::Uuid, session_id: &uuid::Uuid, note: Option<&str>) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memory_evidence(memory_id, session_id, note) VALUES($1, $2, $3) RETURNING id"
        ).bind(memory_id).bind(session_id).bind(note)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_memory_evidence(&self, memory_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, Option<String>, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT id, session_id, note, modified_at FROM sensei.memory_evidence WHERE memory_id = $1"
        ).bind(memory_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, sid, note, modified)| {
            serde_json::json!({ "id": id, "session_id": sid, "note": note, "modified_at": modified.to_rfc3339() })
        }).collect())
    }

    // ── Memory Links ─────────────────────────────────────────────────

    pub async fn link_memories(&self, parent_id: &uuid::Uuid, child_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.memory_links(parent_id, child_id) VALUES($1, $2) ON CONFLICT DO NOTHING"
        ).bind(parent_id).bind(child_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_memory_children(&self, parent_id: &uuid::Uuid) -> Result<Vec<uuid::Uuid>, String> {
        let rows: Vec<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT child_id FROM sensei.memory_links WHERE parent_id = $1"
        ).bind(parent_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn get_memory_parent(&self, child_id: &uuid::Uuid) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT parent_id FROM sensei.memory_links WHERE child_id = $1"
        ).bind(child_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    // ── Recommendations (inference) ──────────────────────────────────

    pub async fn create_recommendation(
        &self, project_id: &uuid::Uuid, title: &str, why: &str, action_type: &str, urgency: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.recommendations(project_id, title, why, action_type, urgency)
             VALUES($1, $2, $3, $4, $5::sensei.recommendation_urgency) RETURNING id"
        ).bind(project_id).bind(title).bind(why).bind(action_type).bind(urgency)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn accept_recommendation(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE inference.recommendations SET status = 'accepted'::sensei.recommendation_status, acted_at = now() WHERE id = $1"
        ).bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn measure_recommendation(&self, id: &uuid::Uuid, verdict: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE inference.recommendations SET verdict = $2::sensei.recommendation_verdict, measured_at = now() WHERE id = $1"
        ).bind(id).bind(verdict).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn list_recommendations(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT id, title, why, urgency::text, status::text, verdict::text FROM inference.recommendations WHERE project_id = $1 ORDER BY urgency::text"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, why, urg, status, verdict)| {
            serde_json::json!({ "id": id, "title": title, "why": why, "urgency": urg, "status": status, "verdict": verdict })
        }).collect())
    }

    // ── Communities (inference) ───────────────────────────────────────

    pub async fn upsert_community(&self, folder_id: &uuid::Uuid, community_id: i32, label: &str, node_count: i32) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.communities(folder_id, community_id, label, node_count)
             VALUES($1, $2, $3, $4)
             ON CONFLICT(folder_id, community_id) DO UPDATE SET label = EXCLUDED.label, node_count = EXCLUDED.node_count, modified_at = now()
             RETURNING id"
        ).bind(folder_id).bind(community_id).bind(label).bind(node_count)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_communities(&self, folder_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, i32)> = sqlx_core::query_as::query_as(
            "SELECT id, label, node_count FROM inference.communities WHERE folder_id = $1 ORDER BY node_count DESC"
        ).bind(folder_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, label, count)| {
            serde_json::json!({ "id": id, "label": label, "node_count": count })
        }).collect())
    }

    // ── Reasoning Traces (inference) ─────────────────────────────────

    pub async fn insert_reasoning_trace(
        &self, project_id: Option<&uuid::Uuid>, trigger_event: &str,
        models_used: &[String], exchanges: &serde_json::Value, consensus: &serde_json::Value,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.reasoning_traces(project_id, trigger_event, models_used, exchanges, consensus) VALUES($1, $2, $3, $4, $5) RETURNING id"
        ).bind(project_id).bind(trigger_event).bind(models_used).bind(exchanges).bind(consensus)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn get_reasoning_traces_by_project(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Vec<String>, serde_json::Value, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT id, trigger_event, models_used, exchanges, consensus FROM inference.reasoning_traces WHERE project_id = $1"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, trigger, models, exchanges, consensus)| {
            serde_json::json!({ "id": id, "trigger_event": trigger, "models_used": models, "exchanges": exchanges, "consensus": consensus })
        }).collect())
    }

    // ── Folders to Watch ───────────────────────────────────────────────

    pub async fn add_watch_root(&self, path: &str, name: &str, excluded: &serde_json::Value) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders_to_watch(path, name, excluded) VALUES($1, $2, $3)
             ON CONFLICT(path) DO UPDATE SET name = EXCLUDED.name, excluded = EXCLUDED.excluded, modified_at = now()
             RETURNING id"
        ).bind(path).bind(name).bind(excluded)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
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

    pub async fn update_watch_status(&self, id: &uuid::Uuid, status: &str) -> Result<(), String> {
        sqlx_core::query::query("UPDATE sensei.folders_to_watch SET status = $2::sensei.watch_status, modified_at = now() WHERE id = $1")
            .bind(id).bind(status).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn remove_watch_root(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Scan State ───────────────────────────────────────────────────

    pub async fn upsert_scan_state(&self, folder_id: &uuid::Uuid, file_path: &str, mtime: i64, content_hash: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.scan_state(folder_id, file_path, mtime, content_hash) VALUES($1, $2, $3, $4)
             ON CONFLICT(folder_id, file_path) DO UPDATE SET mtime = EXCLUDED.mtime, content_hash = EXCLUDED.content_hash, indexed_at = now(), modified_at = now()"
        ).bind(folder_id).bind(file_path).bind(mtime).bind(content_hash)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_stale_files(&self, folder_id: &uuid::Uuid, current_files: &[(String, i64)]) -> Result<Vec<String>, String> {
        // Return files where mtime has changed
        let mut stale = Vec::new();
        for (path, mtime) in current_files {
            let row: Option<(i64,)> = sqlx_core::query_as::query_as(
                "SELECT mtime FROM sensei.scan_state WHERE folder_id = $1 AND file_path = $2"
            ).bind(folder_id).bind(path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
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
            .bind(folder_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// All scan-state fingerprints for a folder as `(file_path, mtime)`. Loaded
    /// once per scan so the indexer can diff the working tree against the last
    /// index in memory (skip unchanged files, re-index changed, drop removed)
    /// instead of N per-file queries.
    pub async fn list_scan_state(&self, folder_id: &uuid::Uuid) -> Result<Vec<(String, i64)>, String> {
        let rows: Vec<(String, i64)> = sqlx_core::query_as::query_as(
            "SELECT file_path, mtime FROM sensei.scan_state WHERE folder_id = $1"
        ).bind(folder_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Drop a single file's scan-state row (used when a file no longer exists on
    /// disk, e.g. it was deleted or removed by a branch switch).
    pub async fn delete_scan_state_file(&self, folder_id: &uuid::Uuid, file_path: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.scan_state WHERE folder_id = $1 AND file_path = $2")
            .bind(folder_id).bind(file_path).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Services ─────────────────────────────────────────────────────

    pub async fn upsert_service(&self, name: &str, display_name: &str, kind: &str, protocol: &str, config: &serde_json::Value) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.services(name, display_name, kind, protocol, config) VALUES($1, $2, $3::sensei.service_kind, $4::sensei.service_protocol, $5)
             ON CONFLICT(name) DO UPDATE SET display_name = EXCLUDED.display_name, config = EXCLUDED.config, modified_at = now()
             RETURNING id"
        ).bind(name).bind(display_name).bind(kind).bind(protocol).bind(config)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_services(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, bool, serde_json::Value)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, display_name, kind::text, protocol::text, installed, config FROM sensei.services ORDER BY name"
            ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, dn, kind, proto, inst, config)| {
            serde_json::json!({ "id": id, "name": name, "display_name": dn, "kind": kind, "protocol": proto, "installed": inst, "config": config })
        }).collect())
    }

    pub async fn delete_service(&self, name: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.services WHERE name = $1")
            .bind(name).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Snapshots (activity) ─────────────────────────────────────────

    pub async fn create_snapshot(
        &self, session_id: &uuid::Uuid, folder_id: &uuid::Uuid, kind: &str,
        progress: &str, next_step: Option<&str>, completed_steps: &[String],
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.snapshots(session_id, folder_id, kind, progress_summary, next_step_hint, completed_steps) VALUES($1, $2, $3::sensei.snapshot_kind, $4, $5, $6) RETURNING id"
        ).bind(session_id).bind(folder_id).bind(kind).bind(progress).bind(next_step).bind(completed_steps)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn get_latest_snapshot(&self, session_id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, Option<String>, Vec<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, kind::text, progress_summary, next_step_hint, completed_steps, created_at FROM activity.snapshots WHERE session_id = $1 ORDER BY created_at DESC LIMIT 1"
            ).bind(session_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id, kind, progress, next, steps, ts)| {
            serde_json::json!({ "id": id, "kind": kind, "progress_summary": progress, "next_step_hint": next, "completed_steps": steps, "created_at": ts.to_rfc3339() })
        }))
    }

    // ── Detected Patterns (inference) ──────────────────────────────────

    pub async fn upsert_pattern(
        &self, folder_id: &uuid::Uuid, name: &str, is_anti: bool,
        confidence: Option<f64>, instances: &serde_json::Value,
    ) -> Result<uuid::Uuid, String> {
        let count = instances.as_array().map(|a| a.len() as i32).unwrap_or(0);
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.detected_patterns(folder_id, name, is_anti_pattern, confidence, instance_count, instances)
             VALUES($1, $2, $3, $4, $5, $6)
             ON CONFLICT(folder_id, name, is_anti_pattern) DO UPDATE SET
               confidence = COALESCE(EXCLUDED.confidence, detected_patterns.confidence),
               instance_count = EXCLUDED.instance_count,
               instances = EXCLUDED.instances,
               modified_at = now()
             RETURNING id"
        ).bind(folder_id).bind(name).bind(is_anti).bind(confidence).bind(count).bind(instances)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn promote_pattern(&self, id: &uuid::Uuid, lifecycle: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE inference.detected_patterns SET lifecycle = $2::sensei.pattern_lifecycle, modified_at = now() WHERE id = $1"
        ).bind(id).bind(lifecycle)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn list_patterns_by_folder(&self, folder_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, String, bool, Option<f64>, i32, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, family, lifecycle::text, is_anti_pattern, confidence::float8, instance_count, modified_at
                 FROM inference.detected_patterns WHERE folder_id = $1 ORDER BY instance_count DESC"
            ).bind(folder_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, family, lc, anti, conf, count, modified)| {
            serde_json::json!({
                "id": id, "name": name, "family": family, "lifecycle": lc,
                "is_anti_pattern": anti, "confidence": conf, "instance_count": count,
                "modified_at": modified.to_rfc3339(),
            })
        }).collect())
    }

    // ── Libraries ────────────────────────────────────────────────────

    pub async fn upsert_library(
        &self, name: &str, ecosystem: &str, version: Option<&str>,
        description: Option<&str>, source_type: Option<&str>, base_url: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.libraries(name, ecosystem, version, description, source_type, base_url)
             VALUES($1, $2::sensei.library_ecosystem, $3, $4, $5::sensei.library_source_type, $6)
             ON CONFLICT(ecosystem, name) DO UPDATE SET
               version = COALESCE(EXCLUDED.version, libraries.version),
               description = COALESCE(EXCLUDED.description, libraries.description),
               source_type = COALESCE(EXCLUDED.source_type, libraries.source_type),
               base_url = COALESCE(EXCLUDED.base_url, libraries.base_url),
               modified_at = now()
             RETURNING id"
        ).bind(name).bind(ecosystem).bind(version).bind(description).bind(source_type).bind(base_url)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn get_library(&self, id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, Option<String>, Option<String>, i32, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, ecosystem::text, version, description, page_count, modified_at FROM sensei.libraries WHERE id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(row.map(|(id, name, eco, ver, desc, pages, modified)| {
            serde_json::json!({
                "id": id, "name": name, "ecosystem": eco, "version": ver,
                "description": desc, "page_count": pages, "modified_at": modified.to_rfc3339(),
            })
        }))
    }

    pub async fn list_libraries(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, i32)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, ecosystem::text, version, page_count FROM sensei.libraries ORDER BY name"
            ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, eco, ver, pages)| {
            serde_json::json!({ "id": id, "name": name, "ecosystem": eco, "version": ver, "page_count": pages })
        }).collect())
    }

    /// List libraries joined with their folder usage. Returns one row per
    /// library with `repos` (folder names that reference it) and `repoCount`.
    /// Drives `GET /api/libs` for the setup wizard so the Libraries page can
    /// render ecosystem + version + usage without a second round-trip.
    pub async fn list_libraries_with_usage(
        &self,
        scope_folder_name: Option<&str>,
        scope_project_id: Option<&uuid::Uuid>,
        min_repos: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        // Aggregate by library, joining via referenced_libraries to count and
        // list distinct folder names. The optional scopes filter the *folders*
        // counted (not the library), so a lib appears only if some in-scope
        // folder references it.
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<String>, i32, i64, Vec<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT l.id, l.name, l.ecosystem::text, l.version, l.description, l.page_count,
                        COUNT(DISTINCT rl.folder_id)::bigint AS repo_count,
                        COALESCE(array_agg(DISTINCT f.name ORDER BY f.name), ARRAY[]::text[]) AS repos
                   FROM sensei.libraries l
                   JOIN sensei.referenced_libraries rl ON rl.library_id = l.id
                   JOIN sensei.folders f ON f.id = rl.folder_id
                  WHERE l.kind = 'detected'::sensei.library_kind
                    AND ($1::text     IS NULL OR f.name = $1)
                    AND ($2::uuid     IS NULL OR f.project_id = $2)
                  GROUP BY l.id, l.name, l.ecosystem, l.version, l.description, l.page_count
                 HAVING COUNT(DISTINCT rl.folder_id) >= $3
                  ORDER BY repo_count DESC, l.name"
            )
            .bind(scope_folder_name)
            .bind(scope_project_id)
            .bind(min_repos)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, eco, ver, desc, pages, repo_count, repos)| {
            serde_json::json!({
                "id": id, "name": name, "ecosystem": eco, "version": ver,
                "description": desc, "pageCount": pages,
                "repoCount": repo_count, "repos": repos,
            })
        }).collect())
    }

    pub async fn delete_library(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.libraries WHERE id = $1")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn upsert_library_page(
        &self, library_id: &uuid::Uuid, title: &str, url: Option<&str>,
        description: Option<&str>, content: Option<&str>, source_type: &str,
        component: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.library_pages(library_id, title, url, description, content, source_type, component, fetched_at)
             VALUES($1, $2, $3, $4, $5, $6::sensei.library_source_type, $7, now())
             ON CONFLICT(library_id, title) DO UPDATE SET
               url = COALESCE(EXCLUDED.url, library_pages.url),
               description = COALESCE(EXCLUDED.description, library_pages.description),
               content = COALESCE(EXCLUDED.content, library_pages.content),
               component = COALESCE(EXCLUDED.component, library_pages.component),
               fetched_at = now(), modified_at = now()
             RETURNING id"
        ).bind(library_id).bind(title).bind(url).bind(description).bind(content).bind(source_type).bind(component)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn update_library_page_count(&self, library_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.libraries SET page_count = (SELECT count(*) FROM sensei.library_pages WHERE library_id = $1), modified_at = now() WHERE id = $1"
        ).bind(library_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn upsert_referenced_library(
        &self, folder_id: &uuid::Uuid, library_id: &uuid::Uuid, version: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.referenced_libraries(folder_id, library_id, version_used)
             VALUES($1, $2, $3)
             ON CONFLICT(folder_id, library_id) DO UPDATE SET
               version_used = COALESCE(EXCLUDED.version_used, referenced_libraries.version_used),
               modified_at = now()"
        ).bind(folder_id).bind(library_id).bind(version)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Roll a folder-level library reference up to a project-level association
    /// (sensei.project_libraries), scoped to `project_id`. `referenced_libraries`
    /// is folder-grained; `project_libraries` is the project↔library M2M the
    /// indexer owns and which `project_libraries_resolved` (the Projects screen)
    /// reads. Idempotent and non-destructive: `ON CONFLICT DO NOTHING` preserves
    /// any user edits to `enabled`/`props` on re-scan.
    pub async fn upsert_project_library(
        &self, library_id: &uuid::Uuid, project_id: &uuid::Uuid,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.project_libraries(library_id, project_id)
             VALUES($1, $2)
             ON CONFLICT (library_id, project_id) WHERE project_id IS NOT NULL DO NOTHING"
        ).bind(library_id).bind(project_id)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Verdict measurement ────────────────────────────────────────────

    /// Recompute FTR deltas for accepted recommendations with pending verdict.
    /// Compares current 14-day FTR against baseline_ftr snapshot at time of acceptance.
    /// Returns number of recommendations updated.
    pub async fn measure_pending_verdicts(&self) -> Result<i64, String> {
        // Update current_ftr and verdict for accepted recommendations that have been
        // acted on at least 3 days ago (enough data for a meaningful comparison).
        let result = sqlx_core::query::query(
            "WITH current AS (
               SELECT r.id AS rec_id,
                      AVG(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END) AS current_ftr
                 FROM inference.recommendations r
                 JOIN activity.sessions s ON s.project_id = r.project_id
                                         AND s.started_at > r.acted_at
                WHERE r.status = 'accepted'
                  AND r.verdict = 'pending'
                  AND r.acted_at < now() - interval '3 days'
                  AND s.outcome IS NOT NULL
                GROUP BY r.id
                HAVING COUNT(*) >= 3
             )
             UPDATE inference.recommendations r
                SET current_ftr = c.current_ftr,
                    verdict = CASE
                      WHEN c.current_ftr > r.baseline_ftr + 0.05 THEN 'positive'
                      WHEN c.current_ftr < r.baseline_ftr - 0.05 THEN 'negative'
                      ELSE 'neutral'
                    END::sensei.recommendation_verdict,
                    measured_at = now()
               FROM current c
              WHERE c.rec_id = r.id"
        ).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(result.rows_affected() as i64)
    }

    // ── Observatory views ──────────────────────────────────────────────

    pub async fn get_ftr_daily(&self, project_id: Option<&uuid::Uuid>, days: i32) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(Option<uuid::Uuid>, chrono::NaiveDate, Option<f64>, Option<i64>)> = if let Some(pid) = project_id {
            sqlx_core::query_as::query_as(
                "SELECT project_id, day, ftr_rate::float8, session_count::bigint FROM sensei.ftr_daily
                 WHERE project_id = $1 AND day >= (current_date - $2::int)
                 ORDER BY day"
            ).bind(pid).bind(days).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
        } else {
            sqlx_core::query_as::query_as(
                "SELECT NULL::uuid, day, AVG(ftr_rate)::float8 as ftr_rate, SUM(session_count)::bigint as session_count
                 FROM sensei.ftr_daily WHERE day >= (current_date - $1::int)
                 GROUP BY day ORDER BY day"
            ).bind(days).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
        };
        Ok(rows.into_iter().map(|(_, day, ftr, count)| {
            serde_json::json!({ "day": day.to_string(), "ftr_rate": ftr.unwrap_or(0.0), "session_count": count.unwrap_or(0) })
        }).collect())
    }

    pub async fn get_hotspots(&self, project_id: &uuid::Uuid, days: i32) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, i64, i64)> = sqlx_core::query_as::query_as(
            "SELECT folder, file_path, edit_count, correction_count
             FROM sensei.project_hotspots
             WHERE project_id = $1 AND last_event_at >= (now() - ($2::int || ' days')::interval)
             ORDER BY (edit_count + correction_count) DESC LIMIT 20"
        ).bind(project_id).bind(days).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(folder, path, edits, corrections)| {
            serde_json::json!({ "folder": folder, "file_path": path, "edit_count": edits, "correction_count": corrections })
        }).collect())
    }

    pub async fn get_quality_signals(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        let row: Option<(f64, Option<f64>, i64, Option<f64>)> = sqlx_core::query_as::query_as(
            "SELECT ftr_7d, pattern_compliance, open_drift_count, test_pass_rate
             FROM sensei.project_quality_signals WHERE project_id = $1"
        ).bind(project_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(match row {
            Some((ftr, compliance, drift, tests)) => serde_json::json!({
                "ftr_7d": ftr, "pattern_compliance": compliance,
                "open_drift_count": drift, "test_pass_rate": tests
            }),
            None => serde_json::json!({
                "ftr_7d": 0, "pattern_compliance": null, "open_drift_count": 0, "test_pass_rate": null
            }),
        })
    }

    pub async fn get_tool_usage_stats(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, i64, i64, Option<f64>, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT tool_name, call_count, error_count, avg_duration_ms, last_used_at
             FROM sensei.tool_usage_stats ORDER BY call_count DESC LIMIT 50"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, calls, errors, dur, last)| {
            serde_json::json!({ "tool_name": name, "call_count": calls, "error_count": errors,
                                "avg_duration_ms": dur, "last_used_at": last.to_rfc3339() })
        }).collect())
    }

    pub async fn get_library_usage(&self, library_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, Option<uuid::Uuid>, Option<String>, i64)> = sqlx_core::query_as::query_as(
            "SELECT library_name, folder, project_id, version_used, unresolved_import_count
             FROM sensei.library_usage WHERE library_id = $1 ORDER BY folder"
        ).bind(library_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, folder, pid, ver, imports)| {
            serde_json::json!({ "library_name": name, "folder": folder, "project_id": pid,
                                "version_used": ver, "import_count": imports })
        }).collect())
    }

    pub async fn get_pending_recommendations(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<String>, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT id, urgency::text, title, why, impact, evidence
             FROM inference.recommendations
             WHERE project_id = $1 AND status = 'pending'
             ORDER BY CASE urgency WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, id
             LIMIT 10"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, urgency, title, why, impact, evidence)| {
            serde_json::json!({ "id": id, "urgency": urgency, "title": title,
                                "why": why, "impact": impact, "evidence": evidence })
        }).collect())
    }

    pub async fn get_adopted_teachings(&self, project_id: &uuid::Uuid, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, i32, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT dp.id, dp.name, dp.family, dp.instance_count, dp.modified_at
             FROM inference.detected_patterns dp
             JOIN sensei.folders f ON f.id = dp.folder_id
             WHERE f.project_id = $1 AND dp.lifecycle = 'rule' AND NOT dp.is_anti_pattern
             ORDER BY dp.modified_at DESC LIMIT $2"
        ).bind(project_id).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, family, count, modified)| {
            serde_json::json!({ "id": id, "name": name, "family": family,
                                "instance_count": count, "modified_at": modified.to_rfc3339() })
        }).collect())
    }

    // ── Sessions (activity) ────────────────────────────────────────────

    pub async fn create_session(&self, folder_id: &uuid::Uuid, task: &str, acp_id: Option<&str>) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.sessions(folder_id, task, acp_id) VALUES($1, $2, $3) RETURNING id"
        ).bind(folder_id).bind(task).bind(acp_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn complete_session(
        &self, id: &uuid::Uuid, outcome: &str, ftr: bool,
        turns: i32, corrections: i32,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.sessions SET outcome = $2::sensei.session_outcome, ftr = $3, turns = $4, corrections = $5, completed_at = now() WHERE id = $1"
        ).bind(id).bind(outcome).bind(ftr).bind(turns).bind(corrections)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Nearest-ancestor folder for an absolute path: the folder whose `abs_path`
    /// is the path itself or its closest parent. Attributes a hook event (which
    /// carries a `cwd`) to the indexed folder it ran in. `None` when uncovered.
    pub async fn find_folder_for_path(
        &self, path: &str,
    ) -> Result<Option<(uuid::Uuid, Option<uuid::Uuid>)>, String> {
        let row: Option<(uuid::Uuid, Option<uuid::Uuid>)> = sqlx_core::query_as::query_as(
            "SELECT id, project_id FROM sensei.folders
             WHERE $1 = abs_path OR $1 LIKE abs_path || '/%'
             ORDER BY length(abs_path) DESC
             LIMIT 1"
        ).bind(path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Find-or-create the `activity.sessions` row for an assistant
    /// `client_session_id`, attributing it to `folder_id`/`project_id`. Marks it
    /// completed when `is_end` (Stop / SessionEnd). Idempotent per
    /// client_session_id so every hook event of a session folds into one row (#31).
    pub async fn record_session_event(
        &self, client_session_id: &str, folder_id: &uuid::Uuid,
        project_id: Option<&uuid::Uuid>, family: &str, is_end: bool,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.sessions (client_session_id, folder_id, project_id, acp_id, completed_at)
             VALUES ($1, $2, $3, $4, CASE WHEN $5 THEN now() ELSE NULL END)
             ON CONFLICT (client_session_id) WHERE client_session_id IS NOT NULL
             DO UPDATE SET
               completed_at = CASE WHEN $5 THEN now() ELSE activity.sessions.completed_at END,
               project_id   = COALESCE(activity.sessions.project_id, EXCLUDED.project_id)
             RETURNING id"
        ).bind(client_session_id).bind(folder_id).bind(project_id).bind(family).bind(is_end)
         .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn get_session(&self, id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, uuid::Uuid, String, Option<String>, Option<String>, Option<bool>, i32, i32, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, folder_id, task, acp_id, outcome::text, ftr, turns, corrections, started_at, completed_at FROM activity.sessions WHERE id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(row.map(|(id, fid, task, acp, outcome, ftr, turns, corr, started, completed)| {
            serde_json::json!({
                "id": id, "folder_id": fid, "task": task, "acp_id": acp,
                "outcome": outcome, "ftr": ftr, "turns": turns, "corrections": corr,
                "started_at": started.to_rfc3339(),
                "completed_at": completed.map(|t| t.to_rfc3339()),
            })
        }))
    }

    pub async fn list_sessions_by_folder(&self, folder_id: &uuid::Uuid, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, Option<bool>, i32, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, task, outcome::text, ftr, corrections, started_at FROM activity.sessions WHERE folder_id = $1 ORDER BY started_at DESC LIMIT $2"
            ).bind(folder_id).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, task, outcome, ftr, corr, started)| {
            serde_json::json!({ "id": id, "task": task, "outcome": outcome, "ftr": ftr, "corrections": corr, "started_at": started.to_rfc3339() })
        }).collect())
    }

    // ── Events (activity) ────────────────────────────────────────────

    pub async fn insert_event(
        &self, session_id: &uuid::Uuid, folder_id: &uuid::Uuid,
        event_type: &str, turn_number: Option<i32>, data: &serde_json::Value,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.events(session_id, folder_id, event_type, turn_number, data) VALUES($1, $2, $3::sensei.event_type, $4, $5) RETURNING id"
        ).bind(session_id).bind(folder_id).bind(event_type).bind(turn_number).bind(data)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn get_events_by_session(&self, session_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<i32>, serde_json::Value, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, event_type::text, turn_number, data, created_at FROM activity.events WHERE session_id = $1 ORDER BY created_at"
            ).bind(session_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, etype, turn, data, ts)| {
            serde_json::json!({ "id": id, "event_type": etype, "turn_number": turn, "data": data, "created_at": ts.to_rfc3339() })
        }).collect())
    }

    pub async fn get_events_by_type(&self, folder_id: &uuid::Uuid, event_type: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, serde_json::Value, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, session_id, data, created_at FROM activity.events WHERE folder_id = $1 AND event_type = $2::sensei.event_type ORDER BY created_at DESC"
            ).bind(folder_id).bind(event_type).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, sid, data, ts)| {
            serde_json::json!({ "id": id, "session_id": sid, "data": data, "created_at": ts.to_rfc3339() })
        }).collect())
    }

    // ── Hook events ───────────────────────────────────────────────────

    /// Insert a hook event payload into activity.hook_events.
    /// session_id is the assistant's string session ID (not a DB UUID).
    /// assistant_family identifies the source (claude, cursor, zed, …); defaults to 'claude'.
    pub async fn insert_hook_event(
        &self,
        session_id: &str,
        assistant_family: &str,
        event_type: &str,
        tool_name: Option<&str>,
        cwd: Option<&str>,
        ts: i64,
        success: Option<bool>,
        payload: &serde_json::Value,
    ) -> Result<i64, String> {
        let row: (i64,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.hook_events \
             (session_id, assistant_family, event_type, tool_name, cwd, ts, success, payload) \
             VALUES($1, $2::sensei.assistant_family, $3, $4, $5, $6, $7, $8) RETURNING id"
        )
        .bind(session_id)
        .bind(assistant_family)
        .bind(event_type)
        .bind(tool_name)
        .bind(cwd)
        .bind(ts)
        .bind(success)
        .bind(payload)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Newest hook_event timestamp (epoch ms) for an assistant family, or None
    /// when the daemon has never recorded one for it. `assistant_family` is a
    /// Postgres enum, so bind with the explicit cast.
    pub async fn latest_hook_event_ts(&self, family: &str) -> Result<Option<i64>, String> {
        let row: (Option<i64>,) = sqlx_core::query_as::query_as(
            "SELECT max(ts) FROM activity.hook_events WHERE assistant_family = $1::sensei.assistant_family"
        )
        .bind(family)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// All hook events for one assistant session (by its string `session_id`),
    /// oldest-first, projected to the fields session enrichment reads (#66).
    pub async fn get_hook_events_for_session(&self, client_session_id: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, Option<String>, i64, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT event_type, tool_name, ts, payload FROM activity.hook_events
             WHERE session_id = $1 ORDER BY ts"
        ).bind(client_session_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(event_type, tool_name, ts, payload)| {
            serde_json::json!({ "event_type": event_type, "tool_name": tool_name, "ts": ts, "payload": payload })
        }).collect())
    }

    /// `(session uuid, client_session_id)` for every attributed session of a
    /// project that can be enriched from the hook stream (#66).
    pub async fn get_project_session_ids(&self, project_id: &uuid::Uuid) -> Result<Vec<(uuid::Uuid, String)>, String> {
        let rows: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT id, client_session_id FROM activity.sessions
             WHERE project_id = $1 AND client_session_id IS NOT NULL"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Write enrichment metrics onto a session (#66). Sets the derived fields
    /// and merges `tool_usage` into `props` — deliberately does NOT touch
    /// `completed_at` (owned by the hook-stream session derivation, #31).
    pub async fn update_session_metrics(
        &self, session_id: &uuid::Uuid, turns: i32, corrections: i32, outcome: &str,
        ftr: bool, duration_ms: i64, module: Option<&str>, tool_usage: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.sessions
                SET outcome = $2::sensei.session_outcome, ftr = $3, turns = $4,
                    corrections = $5, duration_ms = $6, module = $7,
                    props = props || jsonb_build_object('tool_usage', $8::jsonb)
              WHERE id = $1"
        ).bind(session_id).bind(outcome).bind(ftr).bind(turns).bind(corrections)
            .bind(duration_ms as i32).bind(module).bind(tool_usage)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Projects ──────────────────────────────────────────────────────

    pub async fn create_project(&self, name: &str, description: Option<&str>, client: Option<&str>) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.projects(name, description, client) VALUES($1, $2, $3) RETURNING id"
        ).bind(name).bind(description).bind(client)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Update a project's derived identity props (from README frontmatter or
    /// best-guess). Only overwrites description/client when provided; replaces
    /// stack only when a non-empty stack is given; unions tags.
    pub async fn set_project_identity(
        &self,
        id: &uuid::Uuid,
        description: Option<&str>,
        client: Option<&str>,
        stack: &[String],
        tags: &[String],
    ) -> Result<(), String> {
        let stack_json = serde_json::json!(stack);
        let tags_vec: Vec<String> = tags.to_vec();
        sqlx_core::query::query(
            "UPDATE sensei.projects
                SET description = COALESCE($2, description),
                    client      = COALESCE($3, client),
                    stack       = CASE WHEN jsonb_array_length($4) > 0 THEN $4 ELSE stack END,
                    tags        = array(SELECT DISTINCT unnest(tags || $5)),
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(id)
        .bind(description)
        .bind(client)
        .bind(&stack_json)
        .bind(&tags_vec)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get-or-create a namespace instance by (scope_key, slug). Returns its id.
    pub async fn upsert_namespace(
        &self,
        scope_key: &str,
        name: &str,
        slug: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.namespaces(scope_key, name, slug)
             VALUES($1, $2, $3)
             ON CONFLICT (scope_key, slug) DO UPDATE SET name = EXCLUDED.name, modified_at = now()
             RETURNING id",
        )
        .bind(scope_key)
        .bind(name)
        .bind(slug)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
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

    /// Self-healing reconcile: tag discovery projects with no member folders as
    /// `orphaned` (for the user to resolve), and clear the tag from any that
    /// regained folders. Never deletes. Returns rows changed.
    pub async fn mark_orphaned_projects(&self) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.projects p
                SET tags = CASE
                      WHEN NOT EXISTS (SELECT 1 FROM sensei.folders f WHERE f.project_id = p.id)
                        THEN array(SELECT DISTINCT unnest(p.tags || ARRAY['orphaned']))
                      ELSE array_remove(p.tags, 'orphaned')
                    END,
                    modified_at = now()
              WHERE p.maturity = 'discovery'
                AND ((NOT EXISTS (SELECT 1 FROM sensei.folders f WHERE f.project_id = p.id))
                     <> ('orphaned' = ANY(p.tags)))",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    pub async fn get_project(&self, id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, Option<String>, Option<String>, String, Option<String>, serde_json::Value, serde_json::Value, Vec<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, description, client, maturity::text, goal, stack, links, tags, modified_at FROM sensei.projects WHERE id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(row.map(|(id, name, desc, client, maturity, goal, stack, links, tags, modified)| {
            serde_json::json!({
                "id": id, "name": name, "description": desc, "client": client,
                "maturity": maturity, "goal": goal, "stack": stack, "links": links,
                "tags": tags, "modified_at": modified.to_rfc3339(),
            })
        }))
    }

    pub async fn get_project_by_name(&self, name: &str) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, Option<String>, Option<String>, String, Option<String>, serde_json::Value, serde_json::Value, Vec<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, description, client, maturity::text, goal, stack, links, tags, modified_at FROM sensei.projects WHERE name = $1"
            ).bind(name).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(row.map(|(id, name, desc, client, maturity, goal, stack, links, tags, modified)| {
            serde_json::json!({
                "id": id, "name": name, "description": desc, "client": client,
                "maturity": maturity, "goal": goal, "stack": stack, "links": links,
                "tags": tags, "modified_at": modified.to_rfc3339(),
            })
        }))
    }

    pub async fn list_projects(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, Option<String>, String, Vec<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, description, client, maturity::text, tags, modified_at FROM sensei.projects ORDER BY name"
            ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, desc, client, maturity, tags, modified)| {
            serde_json::json!({
                "id": id, "name": name, "description": desc, "client": client,
                "maturity": maturity, "tags": tags, "modified_at": modified.to_rfc3339(),
            })
        }).collect())
    }

    pub async fn update_project(&self, id: &uuid::Uuid, name: Option<&str>, description: Option<&str>, maturity: Option<&str>) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.projects SET name = COALESCE($2, name), description = COALESCE($3, description), maturity = COALESCE($4::sensei.project_maturity, maturity), modified_at = now() WHERE id = $1"
        ).bind(id).bind(name).bind(description).bind(maturity)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn delete_project(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_project_libraries(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        // Query the resolved view directly — it already joins libraries internally
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, bool, serde_json::Value, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, ecosystem::text, description, enabled, project_props, scope
                 FROM sensei.project_libraries_resolved
                 WHERE (scoped_project_id = $1 OR scoped_project_id IS NULL)
                   AND enabled = true
                 ORDER BY scope DESC, name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, ecosystem, desc, enabled, props, scope)| {
            serde_json::json!({
                "id": id, "name": name, "ecosystem": ecosystem,
                "description": desc, "enabled": enabled,
                "project_props": props, "scope": scope,
            })
        }).collect())
    }

    pub async fn get_project_extensions(&self, project_id: &uuid::Uuid, kind_filter: Option<&[&str]>) -> Result<Vec<serde_json::Value>, String> {
        // Query the resolved view directly — it already joins extensions internally
        let rows: Vec<(uuid::Uuid, String, String, bool, serde_json::Value, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, kind::text, enabled, project_props, scope
                 FROM sensei.project_extensions_resolved
                 WHERE (scoped_project_id = $1 OR scoped_project_id IS NULL)
                   AND enabled = true
                 ORDER BY scope DESC, name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter()
            .filter(|(_, _, kind, _, _, _)| {
                kind_filter.is_none_or(|f| f.contains(&kind.as_str()))
            })
            .map(|(id, name, kind, enabled, props, scope)| {
                serde_json::json!({
                    "id": id, "name": name, "kind": kind,
                    "enabled": enabled, "project_props": props, "scope": scope,
                })
            }).collect())
    }

    pub async fn get_project_ftr(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        let row: Option<(Option<f64>, Option<f64>, i64)> =
            sqlx_core::query_as::query_as(
                "SELECT ftr_14d, ftr_14d_prev, sessions_7d
                 FROM sensei.project_ftr_metrics WHERE project_id = $1"
            ).bind(project_id)
            .fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        let (ftr_14d, ftr_14d_prev, sessions_7d) = row.unwrap_or((None, None, 0));

        // 14-day daily trend array
        let daily: Vec<(chrono::NaiveDate, Option<f64>)> =
            sqlx_core::query_as::query_as(
                "SELECT date_trunc('day', started_at)::date AS day,
                        AVG(CASE WHEN ftr THEN 1.0 ELSE 0.0 END) AS daily_ftr
                 FROM activity.sessions
                 WHERE project_id = $1 AND started_at > now() - interval '14d'
                 GROUP BY day ORDER BY day"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let trend: Vec<f64> = daily.into_iter().map(|(_, v)| v.unwrap_or(0.0)).collect();

        Ok(serde_json::json!({
            "ftr14d": ftr_14d.unwrap_or(0.0),
            "ftr14dPrev": ftr_14d_prev.unwrap_or(0.0),
            "ftrTrend": trend,
            "sessions7d": sessions_7d,
        }))
    }

    pub async fn get_project_drift(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, status::text, detail, detected_at
                 FROM sensei.project_drift WHERE project_id = $1
                 ORDER BY detected_at DESC LIMIT 200"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let total = rows.len();
        let drifted = rows.iter().filter(|r| r.1 == "drifted").count();
        let broken = rows.iter().filter(|r| r.1 == "broken").count();
        let items: Vec<_> = rows.into_iter().map(|(id, status, detail, detected_at)| {
            serde_json::json!({ "id": id, "status": status, "detail": detail, "detectedAt": detected_at.to_rfc3339() })
        }).collect();

        Ok(serde_json::json!({ "items": items, "total": total, "drifted": drifted, "broken": broken }))
    }

    pub async fn get_project_patterns(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, bool, String, f64, i64)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, family, is_anti_pattern, lifecycle::text, confidence, instance_count
                 FROM sensei.project_patterns WHERE project_id = $1
                 ORDER BY is_anti_pattern, name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let (followed, anti): (Vec<_>, Vec<_>) = rows.into_iter().partition(|r| !r.3);
        let map_row = |(id, name, family, is_anti, lifecycle, confidence, count): (uuid::Uuid, String, Option<String>, bool, String, f64, i64)| {
            serde_json::json!({ "id": id, "name": name, "family": family, "isAntiPattern": is_anti, "lifecycle": lifecycle, "confidence": confidence, "instanceCount": count })
        };
        Ok(serde_json::json!({
            "followed": followed.into_iter().map(map_row).collect::<Vec<_>>(),
            "antiPatterns": anti.into_iter().map(map_row).collect::<Vec<_>>(),
        }))
    }

    pub async fn get_project_memories(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, f64, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, title, type::text, status::text, strength, last_relevant_at
                 FROM sensei.memories WHERE project_id = $1
                 ORDER BY last_relevant_at DESC LIMIT 100"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let pending_share = rows.iter().filter(|r| r.3 == "pending_share").count();
        let total = rows.len();
        let active: Vec<_> = rows.into_iter()
            .filter(|r| r.3 == "active")
            .map(|(id, title, typ, status, strength, last)| {
                serde_json::json!({ "id": id, "title": title, "type": typ, "status": status, "strength": strength, "lastRelevantAt": last.to_rfc3339() })
            }).collect();

        Ok(serde_json::json!({ "active": active, "total": total, "pendingShare": pending_share }))
    }

    pub async fn ensure_test_project(&self, name: &str) -> Result<uuid::Uuid, String> {
        // Namespace fixtures under `_test:` so leaked rows are identifiable
        // (and filterable by the Projects screen) and never masquerade as real
        // projects. Find-or-create by name so repeated test runs reuse one row
        // instead of minting a fresh UUID each call (#34). Each fixture name is
        // owned by a single test, so the SELECT-then-INSERT is race-free here.
        let name = format!("_test:{name}");
        if let Some(row) = sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM sensei.projects WHERE name = $1"
        ).bind(&name).fetch_optional(&self.pool).await.map_err(|e| e.to_string())? {
            return Ok(row.0);
        }
        let id = uuid::Uuid::new_v4();
        sqlx_core::query::query(
            "INSERT INTO sensei.projects (id, name) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING"
        ).bind(id).bind(&name)
         .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub async fn insert_memory(&self, m: &InsertMemory) -> Result<uuid::Uuid, String> {
        let id: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memories
                (project_id, scope, scope_filter, type, title, content, impact,
                 tags, triage_signal, status, namespace_id, enforcement, origin, source_id)
             VALUES ($1, $2::sensei.memory_scope, $3, $4::sensei.memory_type, $5, $6, $7,
                     $8, $9, $10::sensei.memory_status, $11,
                     COALESCE($12::sensei.enforcement, 'recommended'::sensei.enforcement),
                     COALESCE($13, 'learned'), $14)
             RETURNING id"
        )
            .bind(m.project_id)
            .bind(&m.scope).bind(&m.scope_filter)
            .bind(&m.mtype).bind(&m.title).bind(&m.content).bind(&m.impact)
            .bind(&m.tags).bind(&m.triage_signal).bind(&m.status)
            .bind(m.namespace_id).bind(&m.enforcement).bind(&m.origin).bind(m.source_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(id.0)
    }

    /// Promote a proven memory to a higher (broader) scope: copy it as a
    /// `proposed` memory on `target_namespace_id` with `origin='promoted'` and
    /// `source_id` pointing back at the original. The copy lands in the triage
    /// queue — accepting it (set_memory_status proposed→active) is the approval
    /// gate, so a promotion never auto-applies at the new scope. Only an
    /// established source (active/reinforced/battle_tested) is promotable;
    /// returns Ok(None) otherwise. `enforcement` overrides the source's when set.
    pub async fn promote_memory(
        &self,
        source_id: uuid::Uuid,
        target_namespace_id: Option<uuid::Uuid>,
        enforcement: Option<&str>,
    ) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memories
                (project_id, scope, scope_filter, type, title, content, impact, tags,
                 status, namespace_id, enforcement, origin, source_id)
             SELECT project_id, scope, scope_filter, type, title, content, impact, tags,
                    'proposed'::sensei.memory_status,
                    $2,
                    COALESCE($3::sensei.enforcement, enforcement),
                    'promoted', $1
               FROM sensei.memories
              WHERE id = $1
                AND status IN ('active'::sensei.memory_status,
                               'reinforced'::sensei.memory_status,
                               'battle_tested'::sensei.memory_status)
             RETURNING id"
        )
            .bind(source_id)
            .bind(target_namespace_id)
            .bind(enforcement)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// Memories that have proven themselves (`battle_tested`) and have not
    /// already been promoted — the candidates a UI surfaces for "promote to a
    /// broader scope".
    pub async fn list_promotion_candidates(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<uuid::Uuid>, String)> =
            sqlx_core::query_as::query_as(
                "SELECT m.id, m.title, m.content, m.namespace_id, m.enforcement::text
                   FROM sensei.memories m
                  WHERE m.status = 'battle_tested'::sensei.memory_status
                    AND NOT EXISTS (
                          SELECT 1 FROM sensei.memories c WHERE c.source_id = m.id
                    )
                  ORDER BY m.strength DESC, m.modified_at DESC",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, content, ns, enforcement)| {
            serde_json::json!({ "id": id, "title": title, "content": content,
                "namespace_id": ns, "enforcement": enforcement })
        }).collect())
    }

    pub async fn list_memories(
        &self,
        project_id: Option<uuid::Uuid>,
        status:     Option<&str>,
        scope:      Option<&str>,
        limit:      i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, String, String,
                       Option<String>, f64, String, i32, i32,
                       Option<chrono::DateTime<chrono::Utc>>, Vec<String>, Option<String>,
                       chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, project_id, scope::text, scope_filter, type::text, title, content,
                        impact, strength::float8, status::text, reinforced_count, violated_count,
                        last_relevant_at, tags, triage_signal, modified_at
                   FROM sensei.memories
                  WHERE ($1::uuid IS NULL OR project_id = $1)
                    AND ($2::text IS NULL OR status::text = $2)
                    AND ($3::text IS NULL OR scope::text = $3)
                  ORDER BY strength DESC, last_relevant_at DESC NULLS LAST, modified_at DESC
                  LIMIT $4"
            )
            .bind(project_id).bind(status).bind(scope).bind(limit)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|r| serde_json::json!({
            "id":               r.0,
            "project_id":       r.1,
            "scope":            r.2,
            "scope_filter":     r.3,
            "type":             r.4,
            "title":            r.5,
            "content":          r.6,
            "impact":           r.7,
            "strength":         r.8,
            "status":           r.9,
            "applied_count":    r.10,
            "violated_count":   r.11,
            "last_relevant_at": r.12.map(|t| t.to_rfc3339()),
            "tags":             r.13,
            "triage_signal":    r.14,
            "modified_at":      r.15.to_rfc3339(),
        })).collect())
    }

    /// Transition a memory's status, only when its current status is in `from_states`.
    /// Returns the new status if the transition happened, None if no row matched.
    pub async fn set_memory_status(
        &self,
        memory_id: uuid::Uuid,
        to_status: &str,
        from_states: &[&str],
    ) -> Result<Option<String>, String> {
        let from_owned: Vec<String> = from_states.iter().map(|s| s.to_string()).collect();
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "UPDATE sensei.memories
                SET status      = $1::sensei.memory_status
                  , modified_at = now()
              WHERE id = $2
                AND status::text = ANY($3)
              RETURNING status::text"
        )
            .bind(to_status).bind(memory_id).bind(&from_owned)
            .fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    /// Full memory detail bundle: row + evidence + examples + recent outcomes.
    pub async fn get_memory_detail(&self, id: uuid::Uuid) -> Result<serde_json::Value, String> {
        let row: Option<(uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, String, String,
                         Option<String>, f64, String, i32, i32,
                         Option<chrono::DateTime<chrono::Utc>>, Vec<String>, Option<String>,
                         chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, project_id, scope::text, scope_filter, type::text, title, content,
                        impact, strength::float8, status::text, reinforced_count, violated_count,
                        last_relevant_at, tags, triage_signal, modified_at
                   FROM sensei.memories WHERE id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        let r = row.ok_or_else(|| format!("memory {id} not found"))?;
        let memory = serde_json::json!({
            "id":               r.0,
            "project_id":       r.1,
            "scope":            r.2,
            "scope_filter":     r.3,
            "type":             r.4,
            "title":            r.5,
            "content":          r.6,
            "impact":           r.7,
            "strength":         r.8,
            "status":           r.9,
            "applied_count":    r.10,
            "violated_count":   r.11,
            "last_relevant_at": r.12.map(|t| t.to_rfc3339()),
            "tags":             r.13,
            "triage_signal":    r.14,
            "modified_at":      r.15.to_rfc3339(),
        });

        // Evidence — table has: session_id, note, modified_at (no url column).
        let evidence: Vec<(Option<uuid::Uuid>, Option<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT session_id, note, modified_at
                   FROM sensei.memory_evidence
                  WHERE memory_id = $1
                  ORDER BY modified_at DESC"
            ).bind(id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        // Examples — table has: node_id, is_good (non-nullable), note. No is_bad column.
        let examples: Vec<(Option<String>, bool, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT node_id, is_good, note
                   FROM sensei.memory_examples
                  WHERE memory_id = $1"
            ).bind(id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        // Last 20 outcomes
        let outcomes: Vec<(String, Option<uuid::Uuid>, Option<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT outcome::text, session_id, context, recorded_at
                   FROM sensei.memory_outcomes
                  WHERE memory_id = $1
                  ORDER BY recorded_at DESC
                  LIMIT 20"
            ).bind(id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "memory":   memory,
            "evidence": evidence.into_iter().map(|(session_id, note, ts)|
                serde_json::json!({ "session_id": session_id, "note": note, "recorded_at": ts.to_rfc3339() })
            ).collect::<Vec<_>>(),
            "examples": examples.into_iter().map(|(node, is_good, note)|
                serde_json::json!({ "node_id": node, "is_good": is_good, "note": note })
            ).collect::<Vec<_>>(),
            "outcomes": outcomes.into_iter().map(|(outcome, sess, ctx, ts)|
                serde_json::json!({ "outcome": outcome, "session_id": sess, "context": ctx, "recorded_at": ts.to_rfc3339() })
            ).collect::<Vec<_>>(),
        }))
    }

    /// Assemble a blended context blob: project-scoped + stack-scoped + global memories.
    /// Only active/reinforced/battle_tested/challenged memories are included.
    /// Governance Tier-1 resolution: the active rules that apply to a repo,
    /// ordered strongest-first. A rule applies when it sits on one of the repo's
    /// member namespaces (`folder_namespaces`), on an always-on `general`/`user`
    /// scope, or is unscoped (`namespace_id IS NULL`). Ordering is the two-axis
    /// precedence — enforcement desc (mandatory first), then scope level desc
    /// (most-specific first), then strength. Structuring (dedup + mandatory-lock)
    /// is done by `crate::governance::structure_ruleset` so it stays pure.
    pub async fn resolve_rules_raw(&self, folder_id: &uuid::Uuid) -> Result<Vec<crate::governance::RawRule>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT m.id, m.title, m.content, m.impact, m.enforcement::text,
                        COALESCE(n.scope_key, 'general') AS scope,
                        n.name AS namespace
                   FROM sensei.memories m
                   LEFT JOIN sensei.namespaces n ON n.id = m.namespace_id
                   LEFT JOIN sensei.scopes s ON s.key = n.scope_key
                  WHERE m.status IN ('active'::sensei.memory_status,
                                     'reinforced'::sensei.memory_status,
                                     'battle_tested'::sensei.memory_status)
                    AND ( m.namespace_id IS NULL
                          OR n.scope_key IN ('general', 'user')
                          OR m.namespace_id IN (
                                SELECT namespace_id FROM sensei.folder_namespaces WHERE folder_id = $1
                          ) )
                  ORDER BY m.enforcement DESC,
                           COALESCE(n.level, s.level, 0) DESC,
                           m.strength DESC",
            )
            .bind(folder_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, content, impact, enforcement, scope, namespace)| {
            crate::governance::RawRule {
                id: id.to_string(), title, content, impact, enforcement, scope, namespace,
            }
        }).collect())
    }

    /// Resolve a repo's namespace at a governance scope — e.g. "this repo's
    /// `project` namespace" or "its `organization` namespace". Used when
    /// authoring a rule so the caller can say "scope this to the project" and we
    /// attach the right namespace_id from the repo's memberships. Returns None
    /// for always-on scopes (`general`/`user`) or when the repo has no namespace
    /// at that scope.
    pub async fn namespace_for_folder_scope(&self, folder_id: &uuid::Uuid, scope_key: &str) -> Result<Option<uuid::Uuid>, String> {
        if matches!(scope_key, "general" | "user") {
            return Ok(None); // always-on scopes are unscoped (namespace_id NULL)
        }
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT n.id
               FROM sensei.folder_namespaces fn
               JOIN sensei.namespaces n ON n.id = fn.namespace_id
              WHERE fn.folder_id = $1 AND n.scope_key = $2
              LIMIT 1",
        )
        .bind(folder_id)
        .bind(scope_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// The global, repo-independent ruleset: rules at the always-on `general`
    /// and `user` scopes (plus unscoped). These apply everywhere and are what
    /// the daemon materializes into `~/.sensei/rules.md`. Same ordering as
    /// `resolve_rules_raw` but with no folder dimension.
    pub async fn resolve_global_rules(&self) -> Result<Vec<crate::governance::RawRule>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT m.id, m.title, m.content, m.impact, m.enforcement::text,
                        COALESCE(n.scope_key, 'general') AS scope,
                        n.name AS namespace
                   FROM sensei.memories m
                   LEFT JOIN sensei.namespaces n ON n.id = m.namespace_id
                   LEFT JOIN sensei.scopes s ON s.key = n.scope_key
                  WHERE m.status IN ('active'::sensei.memory_status,
                                     'reinforced'::sensei.memory_status,
                                     'battle_tested'::sensei.memory_status)
                    AND ( m.namespace_id IS NULL OR n.scope_key IN ('general', 'user') )
                  ORDER BY m.enforcement DESC,
                           COALESCE(n.level, s.level, 0) DESC,
                           m.strength DESC",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, content, impact, enforcement, scope, namespace)| {
            crate::governance::RawRule {
                id: id.to_string(), title, content, impact, enforcement, scope, namespace,
            }
        }).collect())
    }

    // ── Governance Tier-2: consolidated (LLM-merged, approved) rulesets ──

    /// Next version number for a scope's consolidated ruleset (max+1, or 1).
    pub async fn next_ruleset_version(&self, scope: &str) -> Result<i32, String> {
        let row: (Option<i32>,) = sqlx_core::query_as::query_as(
            "SELECT max(version) FROM sensei.consolidated_rulesets WHERE scope = $1",
        ).bind(scope).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0.unwrap_or(0) + 1)
    }

    /// The source_hash of a scope's most recent consolidation (any status), so a
    /// re-merge can be skipped when the Tier-1 input is unchanged.
    pub async fn latest_ruleset_source_hash(&self, scope: &str) -> Result<Option<String>, String> {
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT source_hash FROM sensei.consolidated_rulesets WHERE scope = $1 ORDER BY version DESC LIMIT 1",
        ).bind(scope).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(h,)| h))
    }

    /// Insert a new consolidated ruleset version.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_consolidated_ruleset(
        &self, scope: &str, version: i32, content: &str, conflicts: &serde_json::Value,
        model: Option<&str>, source_hash: &str, status: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.consolidated_rulesets
                (scope, version, content, conflicts, model, source_hash, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        )
            .bind(scope).bind(version).bind(content).bind(conflicts)
            .bind(model).bind(source_hash).bind(status)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Fetch a scope's consolidated ruleset: the row with `status` when given
    /// (e.g. "approved"), else the latest version.
    pub async fn get_consolidated_ruleset(&self, scope: &str, status: Option<&str>) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, i32, String, serde_json::Value, Option<String>, String)> = match status {
            Some(s) => sqlx_core::query_as::query_as(
                "SELECT id, version, content, conflicts, model, status FROM sensei.consolidated_rulesets
                  WHERE scope = $1 AND status = $2 ORDER BY version DESC LIMIT 1",
            ).bind(scope).bind(s).fetch_optional(&self.pool).await,
            None => sqlx_core::query_as::query_as(
                "SELECT id, version, content, conflicts, model, status FROM sensei.consolidated_rulesets
                  WHERE scope = $1 ORDER BY version DESC LIMIT 1",
            ).bind(scope).fetch_optional(&self.pool).await,
        }.map_err(|e| e.to_string())?;
        Ok(row.map(|(id, version, content, conflicts, model, status)| serde_json::json!({
            "id": id, "version": version, "content": content,
            "conflicts": conflicts, "model": model, "status": status,
        })))
    }

    /// Approve a consolidated ruleset: supersede the scope's prior approved
    /// version, then mark this one approved. Returns (scope, content).
    pub async fn approve_consolidated_ruleset(&self, id: &uuid::Uuid) -> Result<Option<(String, String)>, String> {
        sqlx_core::query::query(
            "UPDATE sensei.consolidated_rulesets SET status = 'superseded'
              WHERE status = 'approved'
                AND scope = (SELECT scope FROM sensei.consolidated_rulesets WHERE id = $1)
                AND id <> $1",
        ).bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        let row: Option<(String, String)> = sqlx_core::query_as::query_as(
            "UPDATE sensei.consolidated_rulesets SET status = 'approved' WHERE id = $1 RETURNING scope, content",
        ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    pub async fn assemble_context(
        &self,
        project_id: uuid::Uuid,
        stack_ids:  &[String],
        tags:       Option<&[String]>,
        limit:      i64,
    ) -> Result<serde_json::Value, String> {
        let allowed = ["active", "reinforced", "battle_tested", "challenged"];
        let allowed_owned: Vec<String> = allowed.iter().map(|s| s.to_string()).collect();
        let stack_owned: Vec<String> = stack_ids.to_vec();
        let tags_owned: Option<Vec<String>> = tags.map(|t| t.to_vec());

        let rows: Vec<(uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, String, String,
                       Option<String>, f64, String, i32, i32,
                       Option<chrono::DateTime<chrono::Utc>>, Vec<String>, Option<String>,
                       chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, project_id, scope::text, scope_filter, type::text, title, content,
                        impact, strength::float8, status::text, reinforced_count, violated_count,
                        last_relevant_at, tags, triage_signal, modified_at
                   FROM sensei.memories
                  WHERE status::text = ANY($1)
                    AND (
                           project_id = $2
                        OR (scope = 'stack'  AND scope_filter = ANY($3))
                        OR  scope = 'global'
                    )
                    AND ($4::text[] IS NULL OR tags && $4)
                  ORDER BY strength DESC, last_relevant_at DESC NULLS LAST, modified_at DESC
                  LIMIT $5"
            )
            .bind(&allowed_owned).bind(project_id).bind(&stack_owned)
            .bind(&tags_owned).bind(limit)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let memories: Vec<serde_json::Value> = rows.into_iter().map(|r| serde_json::json!({
            "id":               r.0,
            "scope":            r.2,
            "scope_filter":     r.3,
            "type":             r.4,
            "title":            r.5,
            "content":          r.6,
            "impact":           r.7,
            "strength":         r.8,
            "applied_count":    r.10,
            "violated_count":   r.11,
            "last_relevant_at": r.12.map(|t| t.to_rfc3339()),
            "tags":             r.13,
            "updated_at":       r.15.to_rfc3339(),
        })).collect();

        // Version = max modified_at across the set (stable identifier for cache validation).
        let version = memories.iter()
            .filter_map(|m| m["updated_at"].as_str().map(|s| s.to_string()))
            .max()
            .unwrap_or_default();
        let cache_until = (chrono::Utc::now() + chrono::Duration::minutes(5)).to_rfc3339();

        Ok(serde_json::json!({
            "version":     version,
            "memories":    memories,
            "cache_until": cache_until,
        }))
    }

    /// Insert a batch of outcomes. Skips rows whose target memory is archived or rejected.
    pub async fn record_outcomes_batch(
        &self,
        rows: &[OutcomeRow],
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut skipped: Vec<serde_json::Value> = Vec::new();
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        for r in rows {
            // Check current status first.
            let status: Option<(String,)> = sqlx_core::query_as::query_as(
                "SELECT status::text FROM sensei.memories WHERE id = $1"
            ).bind(r.memory_id).fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
            let Some((s,)) = status else {
                skipped.push(serde_json::json!({"memory_id": r.memory_id, "reason": "not_found"}));
                continue;
            };
            if s == "archived" || s == "rejected" {
                skipped.push(serde_json::json!({"memory_id": r.memory_id, "reason": format!("status_{s}")}));
                continue;
            }
            sqlx_core::query::query(
                "INSERT INTO sensei.memory_outcomes (memory_id, session_id, outcome, context)
                 VALUES ($1, $2, $3::sensei.memory_outcome, $4)"
            )
                .bind(r.memory_id).bind(r.session_id).bind(&r.outcome).bind(&r.context)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(skipped)
    }

    /// Return the list of stack identifiers for a project.
    /// The `sensei.projects.stack` column is JSONB and may be an array of strings,
    /// an object with a recognisable array key, or absent — all cases return `[]`.
    pub async fn get_project_stack_ids(&self, project_id: &uuid::Uuid) -> Result<Vec<String>, String> {
        let row: Option<(serde_json::Value,)> = sqlx_core::query_as::query_as(
            "SELECT stack FROM sensei.projects WHERE id = $1"
        ).bind(project_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        let Some((stack_json,)) = row else { return Ok(vec![]); };

        // The stack jsonb may be an array of strings, an object with a "languages" key,
        // or empty. Be permissive: accept array-of-strings OR object-with-arrays, return [].
        match &stack_json {
            serde_json::Value::Array(arr) => {
                Ok(arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            }
            serde_json::Value::Object(obj) => {
                // Try common keys: languages, ids, items.
                for key in &["languages", "ids", "items"] {
                    if let Some(serde_json::Value::Array(arr)) = obj.get(*key) {
                        return Ok(arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());
                    }
                }
                // No recognizable shape — return empty (no stack blending).
                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }

    pub async fn get_project_repos(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        // Only project ROOTS are repos. `kind='folder'` rows are the navigable
        // subfolder tree (materialized by process_git_folder) and must NOT be
        // listed as repos — otherwise a single-repo project with N subfolders
        // renders as an N+1-repo "multi-repo" project (#62). The data is correct;
        // this read path was projecting the subfolder tree as repos.
        let rows: Vec<(uuid::Uuid, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, abs_path, kind::text FROM sensei.folders
                 WHERE project_id = $1 AND kind::text <> 'folder' ORDER BY name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, path, kind)| {
            serde_json::json!({ "id": id, "name": name, "path": path, "kind": kind })
        }).collect())
    }

    pub async fn list_sessions_by_project(&self, project_id: &uuid::Uuid, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<bool>, String, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, task, ftr, outcome::text, started_at
                 FROM activity.sessions WHERE project_id = $1
                 ORDER BY started_at DESC LIMIT $2"
            ).bind(project_id).bind(limit)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, task, ftr, outcome, started)| {
            serde_json::json!({ "id": id, "task": task, "ftr": ftr, "outcome": outcome, "startedAt": started.to_rfc3339() })
        }).collect())
    }

    pub async fn get_project_recommendations(&self, project_id: &uuid::Uuid, status: Option<&str>) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<String>, String, Option<String>,
                        Option<f64>, Option<f64>, Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, title, urgency::text, status::text, verdict::text, why, impact,
                        baseline_ftr::float8, current_ftr::float8, acted_at, measured_at
                 FROM inference.recommendations WHERE project_id = $1
                   AND ($2::text IS NULL OR status::text = $2)
                 ORDER BY CASE urgency WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, id
                 LIMIT 50"
            ).bind(project_id).bind(status)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, title, urgency, status, verdict, why, impact, baseline, current, acted, measured)| {
            serde_json::json!({
                "id": id, "title": title, "urgency": urgency, "status": status, "verdict": verdict,
                "why": why, "impact": impact,
                "baseline_ftr": baseline, "current_ftr": current,
                "acted_at": acted.map(|t| t.to_rfc3339()), "measured_at": measured.map(|t| t.to_rfc3339()),
            })
        }).collect())
    }

    // ── Index Errors ──────────────────────────────────────────────────

    pub async fn log_index_error(
        &self, folder_id: &uuid::Uuid, file_path: &str, error: &str,
        adapter: Option<&str>, phase: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.index_errors(folder_id, file_path, error, adapter, phase) VALUES($1, $2, $3, $4, $5)"
        )
            .bind(folder_id).bind(file_path).bind(error).bind(adapter).bind(phase)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_index_errors(&self, folder_id: Option<&uuid::Uuid>) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<String>, chrono::DateTime<chrono::Utc>)> = match folder_id {
            Some(fid) => sqlx_core::query_as::query_as(
                "SELECT folder_id, file_path, error, adapter, phase, created_at FROM sensei.index_errors WHERE folder_id = $1 ORDER BY created_at DESC"
            ).bind(fid).fetch_all(&self.pool).await,
            None => sqlx_core::query_as::query_as(
                "SELECT folder_id, file_path, error, adapter, phase, created_at FROM sensei.index_errors ORDER BY created_at DESC LIMIT 200"
            ).fetch_all(&self.pool).await,
        }.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(fid, fp, err, adapter, phase, ts)| {
            serde_json::json!({
                "folder_id": fid, "file_path": fp, "error": err,
                "adapter": adapter, "phase": phase, "created_at": ts.to_rfc3339(),
            })
        }).collect())
    }

    pub async fn clear_index_errors(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.index_errors WHERE folder_id = $1")
            .bind(folder_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete `public.logs` rows older than `days` days. The task logger writes
    /// two rows per task, so large scans add hundreds of thousands of rows;
    /// this enforces a retention window. Returns the number of rows removed.
    pub async fn prune_logs(&self, days: i32) -> Result<u64, String> {
        let r = sqlx_core::query::query(
            "DELETE FROM public.logs WHERE logged_at < now() - (interval '1 day' * $1)"
        )
            .bind(days)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(r.rows_affected())
    }

    // ── Raw ──────────────────────────────────────────────────────────

    /// Execute a parameterized query returning unresolved edges.
    pub async fn execute_raw_query(&self, sql: &str, folder_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, Option<String>, String)> = sqlx_core::query_as::query_as(sql)
            .bind(folder_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, src, tgt_name, kind)| {
            serde_json::json!({ "id": id, "source_id": src, "target_name": tgt_name, "kind": kind })
        }).collect())
    }

    /// Execute a raw SQL statement.
    pub async fn execute_raw(&self, sql: &str) -> Result<(), String> {
        sqlx_core::query::query(sql)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("PgStore execute_raw: {}", e))?;
        Ok(())
    }

    // ── Logging (public.logs) ───────────────────────────────────────

    /// Insert a structured log entry into public.logs (kavach pattern).
    pub async fn insert_log(
        &self,
        level: &str,
        running_on: &str,
        logged_at: &str,
        message: &str,
        context: &serde_json::Value,
        data: &Option<serde_json::Value>,
        error: &Option<serde_json::Value>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO public.logs(level, running_on, logged_at, message, context, data, error)
             VALUES($1, $2, $3::timestamptz, $4, $5, $6, $7)"
        )
        .bind(level)
        .bind(running_on)
        .bind(logged_at)
        .bind(message)
        .bind(context)
        .bind(data)
        .bind(error)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("insert_log: {}", e))?;
        Ok(())
    }

    // ── Task Executions (activity.task_executions) ──────────────────

    /// Insert a running task execution record. Returns the row UUID.
    pub async fn start_task_execution(
        &self,
        task_id: i64,
        parent_task_id: Option<i64>,
        task_kind: &str,
        folder_path: &str,
        path: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.task_executions(task_id, parent_task_id, task_kind, folder_path, path, status)
             VALUES($1, $2, $3, $4, $5, 'running') RETURNING id"
        )
        .bind(task_id)
        .bind(parent_task_id)
        .bind(task_kind)
        .bind(folder_path)
        .bind(path)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("start_task_execution: {}", e))?;
        Ok(row.0)
    }

    /// Mark a task execution as completed.
    pub async fn complete_task_execution(
        &self,
        id: &uuid::Uuid,
        items_processed: i32,
        duration_ms: i32,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.task_executions
                SET status = 'completed', items_processed = $2, duration_ms = $3, completed_at = now()
              WHERE id = $1"
        )
        .bind(id)
        .bind(items_processed)
        .bind(duration_ms)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("complete_task_execution: {}", e))?;
        Ok(())
    }

    /// Mark a task execution as failed.
    pub async fn fail_task_execution(
        &self,
        id: &uuid::Uuid,
        duration_ms: i32,
        error_message: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.task_executions
                SET status = 'failed', duration_ms = $2, error_message = $3, completed_at = now()
              WHERE id = $1"
        )
        .bind(id)
        .bind(duration_ms)
        .bind(error_message)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("fail_task_execution: {}", e))?;
        Ok(())
    }

    // ── Knowledge Sources (federation endpoints) ──────────────────────

    pub async fn create_knowledge_source(&self, s: &NewKnowledgeSource) -> Result<uuid::Uuid, String> {
        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.knowledge_sources(kind, name, url, namespace_id, credential_ref, direction)
             VALUES($1,$2,$3,$4,$5,$6) RETURNING id")
            .bind(&s.kind).bind(&s.name).bind(&s.url).bind(s.namespace_id).bind(&s.credential_ref).bind(&s.direction)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub async fn list_knowledge_sources(&self) -> Result<Vec<KnowledgeSource>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<uuid::Uuid>, String, String, i64, bool)> =
            sqlx_core::query_as::query_as(
                "SELECT id, kind, name, url, namespace_id, credential_ref, direction, last_seq, enabled
                   FROM sensei.knowledge_sources ORDER BY created_at")
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id,kind,name,url,namespace_id,credential_ref,direction,last_seq,enabled)|
            KnowledgeSource { id,kind,name,url,namespace_id,credential_ref,direction,last_seq,enabled }).collect())
    }

    pub async fn get_knowledge_source(&self, id: &uuid::Uuid) -> Result<Option<KnowledgeSource>, String> {
        let row: Option<(uuid::Uuid, String, String, String, Option<uuid::Uuid>, String, String, i64, bool)> =
            sqlx_core::query_as::query_as(
                "SELECT id, kind, name, url, namespace_id, credential_ref, direction, last_seq, enabled
                   FROM sensei.knowledge_sources WHERE id = $1")
            .bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id,kind,name,url,namespace_id,credential_ref,direction,last_seq,enabled)|
            KnowledgeSource { id,kind,name,url,namespace_id,credential_ref,direction,last_seq,enabled }))
    }

    pub async fn set_source_cursor(&self, id: &uuid::Uuid, last_seq: i64) -> Result<(), String> {
        sqlx_core::query::query("UPDATE sensei.knowledge_sources SET last_seq = $2 WHERE id = $1")
            .bind(id).bind(last_seq).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn delete_knowledge_source(&self, id: &uuid::Uuid) -> Result<bool, String> {
        let res = sqlx_core::query::query("DELETE FROM sensei.knowledge_sources WHERE id = $1")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    // ── Federation ledger ─────────────────────────────────────────────

    pub async fn namespace_is_shareable(&self, namespace_id: &uuid::Uuid) -> Result<bool, String> {
        let row: Option<(bool,)> = sqlx_core::query_as::query_as(
            "SELECT s.shareable FROM sensei.namespaces n JOIN sensei.scopes s ON s.key = n.scope_key
              WHERE n.id = $1")
            .bind(namespace_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(b,)| b).unwrap_or(false))
    }

    pub async fn upsert_federated_memory(
        &self, source_id: &uuid::Uuid, remote_rule_id: &uuid::Uuid,
        content_hash: &str, memory_id: Option<&uuid::Uuid>, remote_seq: i64,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.federated_memories(knowledge_source_id, remote_rule_id, content_hash, memory_id, remote_seq)
             VALUES($1,$2,$3,$4,$5)
             ON CONFLICT(knowledge_source_id, remote_rule_id) DO UPDATE SET
               content_hash = EXCLUDED.content_hash,
               memory_id = COALESCE(EXCLUDED.memory_id, sensei.federated_memories.memory_id),
               remote_seq = EXCLUDED.remote_seq, synced_at = now()")
            .bind(source_id).bind(remote_rule_id).bind(content_hash).bind(memory_id).bind(remote_seq)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn find_federated_memory(
        &self, source_id: &uuid::Uuid, remote_rule_id: &uuid::Uuid,
    ) -> Result<Option<FederatedLink>, String> {
        let row: Option<(Option<uuid::Uuid>, i64)> = sqlx_core::query_as::query_as(
            "SELECT memory_id, remote_seq FROM sensei.federated_memories
              WHERE knowledge_source_id = $1 AND remote_rule_id = $2")
            .bind(source_id).bind(remote_rule_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(memory_id, remote_seq)| FederatedLink { memory_id, remote_seq }))
    }

    /// Retire a federated memory (tombstone pulled from upstream). Only archives
    /// federated-origin rows, so a locally-authored/promoted memory is never force-archived.
    pub async fn archive_federated_memory(&self, memory_id: &uuid::Uuid) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.memories SET status = 'archived'::sensei.memory_status
              WHERE id = $1 AND origin = 'federated'")
            .bind(memory_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// Fields to build a PublishedRule for a memory + its namespace identity.
    /// None if the memory has no namespace (unscoped).
    pub async fn memory_push_payload(&self, memory_id: &uuid::Uuid)
        -> Result<Option<MemoryPushPayload>, String> {
        let row: Option<(String, String, Option<String>, String, String, String, String, String, String)> =
            sqlx_core::query_as::query_as(
            "SELECT m.title, m.content, m.impact, m.enforcement::text, m.type::text, m.origin,
                    n.scope_key, n.slug, n.name
               FROM sensei.memories m JOIN sensei.namespaces n ON n.id = m.namespace_id
              WHERE m.id = $1")
            .bind(memory_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(title, content, impact, enforcement, rule_type, origin, scope_key, slug, name)|
            MemoryPushPayload { title, content, impact, enforcement, rule_type, origin, scope_key, slug, name }))
    }

    // ── Scoped query helpers (#60) ─────────────────────────────────────

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
            let pid = crate::api::util::json_uuid(&proj["id"])
                .ok_or_else(|| format!("scope_folder_ids: project row missing id for '{}'", ident))?;
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
            let fid = crate::api::util::json_uuid(&folder["id"])
                .ok_or_else(|| format!("scope_folder_ids: folder row missing id for '{}'", ident))?;
            if let Some(pid) = crate::api::util::json_uuid(&folder["project_id"]) {
                return self.folder_ids_for_project(&pid).await;
            }
            return Ok(vec![fid]);
        }

        // (4) No match.
        Ok(vec![])
    }

    /// Collect all folder ids belonging to a project, deduped.
    async fn folder_ids_for_project(&self, project_id: &uuid::Uuid) -> Result<Vec<uuid::Uuid>, String> {
        let folders = self.list_folders_by_project(project_id).await?;
        let mut ids: Vec<uuid::Uuid> = folders
            .iter()
            .filter_map(|f| crate::api::util::json_uuid(&f["id"]))
            .collect();
        // folders.id is the PK so dupes can't occur today, but sort+dedup keeps
        // this robust if list_folders_by_project ever grows a join.
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }

    // ── Project-scoped query variants (#60) ───────────────────────────

    /// Search functions across multiple folders (project-scoped variant).
    pub async fn search_functions_scoped(&self, folder_ids: &[uuid::Uuid], query: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, name, file_path, signature, line_start FROM sensei.nodes
             WHERE folder_id = ANY($1) AND kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
             AND (name ILIKE '%' || $2 || '%' OR signature ILIKE '%' || $2 || '%')
             ORDER BY name LIMIT 50"
        ).bind(folder_ids).bind(query).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, fp, sig, line)| {
            serde_json::json!({ "id": id, "name": name, "file_path": fp, "signature": sig, "line_start": line })
        }).collect())
    }

    /// Search types across multiple folders (project-scoped variant).
    pub async fn search_types_scoped(&self, folder_ids: &[uuid::Uuid], query: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, name, file_path, line_start FROM sensei.nodes
             WHERE folder_id = ANY($1) AND kind IN ('class'::sensei.node_kind, 'struct'::sensei.node_kind, 'interface'::sensei.node_kind, 'enum'::sensei.node_kind, 'type'::sensei.node_kind)
             AND name ILIKE '%' || $2 || '%'
             ORDER BY name LIMIT 50"
        ).bind(folder_ids).bind(query).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, fp, line)| {
            serde_json::json!({ "id": id, "name": name, "file_path": fp, "line_start": line })
        }).collect())
    }

    /// Count nodes by kind across multiple folders (project-scoped variant).
    pub async fn count_nodes_by_kind_scoped(&self, folder_ids: &[uuid::Uuid]) -> Result<std::collections::HashMap<String, i64>, String> {
        let rows: Vec<(String, i64)> = sqlx_core::query_as::query_as(
            "SELECT kind::text, COUNT(*) FROM sensei.nodes WHERE folder_id = ANY($1) GROUP BY kind"
        ).bind(folder_ids).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().collect())
    }

    /// Get all nodes across multiple folders (project-scoped variant).
    pub async fn get_nodes_scoped(&self, folder_ids: &[uuid::Uuid]) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<uuid::Uuid>, Option<i32>, Option<i32>, uuid::Uuid)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, file_path, parent_id, line_start, line_end, folder_id FROM sensei.nodes WHERE folder_id = ANY($1) ORDER BY file_path, line_start"
        ).bind(folder_ids).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, fp, pid, ls, le, folder_id)| {
            serde_json::json!({ "id": id, "kind": kind, "name": name, "file_path": fp, "parent_id": pid, "line_start": ls, "line_end": le, "folder_id": folder_id })
        }).collect())
    }

    /// Get edges by kind across multiple folders (project-scoped variant).
    pub async fn get_edges_scoped(&self, folder_ids: &[uuid::Uuid], kind: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, Option<uuid::Uuid>, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, source_id, target_id, target_name FROM sensei.edges WHERE folder_id = ANY($1) AND kind = $2::sensei.edge_kind"
        ).bind(folder_ids).bind(kind).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, src, tgt, name)| {
            serde_json::json!({ "id": id, "source_id": src, "target_id": tgt, "target_name": name })
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx_core::query_as::query_as;

    /// Test DB URL. Defaults to `sensei_test` — the throwaway DB the
    /// monorepo convention reserves for `cargo test` and CI. NEVER default
    /// to `sensei`: every test that inserts (e.g. `create_test_folder`)
    /// would leak into the user's production data, and the `/_test` row
    /// from earlier runs is a real example of how that surfaces in the UI.
    /// Override with `TEST_DATABASE_URL` for ad-hoc targets (e.g. a forked
    /// snapshot for debugging).
    fn test_db_url() -> String {
        std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| format!("postgresql://localhost:{}/sensei_test", sensei_bootstrap::POSTGRES_PORT))
    }

    #[tokio::test]
    async fn connect_to_pg() {
        let store = PgStore::connect(&test_db_url()).await.unwrap();
        let row: (i32,) = query_as("SELECT 1")
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(row.0, 1);
    }

    #[tokio::test]
    async fn execute_raw_works() {
        let store = PgStore::connect(&test_db_url()).await.unwrap();
        store.execute_raw("SELECT 1").await.unwrap();
    }

    #[tokio::test]
    async fn schema_exists() {
        let store = PgStore::connect(&test_db_url()).await.unwrap();
        let row: (bool,) = query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'sensei')"
        )
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert!(row.0, "sensei schema must exist — run `dbd apply` first");
    }

    // ── Config tests ───────────────────────────────────────────────

    async fn pg_store() -> PgStore {
        PgStore::connect(&test_db_url()).await.unwrap()
    }

    /// Generate a unique key prefix for test isolation.
    fn tkey(test: &str, key: &str) -> String {
        format!("_test:{}:{}", test, key)
    }

    #[tokio::test]
    async fn config_set_and_get() {
        let s = pg_store().await;
        let k = tkey("set_get", "theme");
        s.set_config(&k, "dark").await.unwrap();
        assert_eq!(s.get_config(&k).await.unwrap(), Some("dark".into()));
        s.delete_config(&k).await.unwrap(); // cleanup
    }

    #[tokio::test]
    async fn config_get_missing_returns_none() {
        let s = pg_store().await;
        assert_eq!(s.get_config("_test:missing:nonexistent").await.unwrap(), None);
    }

    #[tokio::test]
    async fn config_set_overwrites() {
        let s = pg_store().await;
        let k = tkey("overwrite", "k");
        s.set_config(&k, "v1").await.unwrap();
        s.set_config(&k, "v2").await.unwrap();
        assert_eq!(s.get_config(&k).await.unwrap(), Some("v2".into()));
        s.delete_config(&k).await.unwrap();
    }

    #[tokio::test]
    async fn config_delete() {
        let s = pg_store().await;
        let k = tkey("delete", "k");
        s.set_config(&k, "v").await.unwrap();
        s.delete_config(&k).await.unwrap();
        assert_eq!(s.get_config(&k).await.unwrap(), None);
    }

    #[tokio::test]
    async fn config_delete_nonexistent_is_noop() {
        let s = pg_store().await;
        s.delete_config("_test:noop:nope").await.unwrap();
    }

    #[tokio::test]
    async fn config_get_all() {
        let s = pg_store().await;
        let k1 = tkey("getall", "a");
        let k2 = tkey("getall", "b");
        s.set_config(&k1, "1").await.unwrap();
        s.set_config(&k2, "2").await.unwrap();
        let all = s.get_all_config().await.unwrap();
        assert_eq!(all[&k1], "1");
        assert_eq!(all[&k2], "2");
        s.delete_config(&k1).await.unwrap();
        s.delete_config(&k2).await.unwrap();
    }

    /// Create a unique test folder for FK tests. Uses suffix for isolation.
    async fn create_test_folder(s: &PgStore, suffix: &str) -> uuid::Uuid {
        use sqlx_core::query_as::query_as;
        s.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) ON CONFLICT DO NOTHING"
        ).await.unwrap();
        let abs_path = format!("/_test/{}", suffix);
        let row: (uuid::Uuid,) = query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path) VALUES('00000000-0000-0000-0000-000000000001', 'git'::sensei.folder_kind, $1, $1, $2) ON CONFLICT(abs_path) DO UPDATE SET name = EXCLUDED.name RETURNING id"
        ).bind(suffix).bind(&abs_path).fetch_one(s.pool()).await.unwrap();
        row.0
    }

    // ── PG Function tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn rank_bm25_returns_results() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("bm25_{}", uuid::Uuid::new_v4())).await;
        s.upsert_node(&fid, "function", "authenticate_user", "src/auth.rs", None, Some("fn authenticate_user(token: &str)"), Some(1), Some(20)).await.unwrap();
        s.upsert_node(&fid, "function", "validate_email", "src/validation.rs", None, Some("fn validate_email(email: &str)"), Some(1), Some(10)).await.unwrap();
        let results = s.rank_bm25(&fid, "authenticate").await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "src/auth.rs");
        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn rank_bm25_empty_folder() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("bm25_empty_{}", uuid::Uuid::new_v4())).await;
        let results = s.rank_bm25(&fid, "anything").await.unwrap();
        assert!(results.is_empty());
    }

    // ── Nodes + Edges tests ────────────────────────────────────────────

    #[tokio::test]
    async fn node_upsert_and_query() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("node_{}", uuid::Uuid::new_v4())).await;
        let file_id = s.upsert_node(&fid, "file", "main.rs", "src/main.rs", None, None, None, None).await.unwrap();
        let fn_id = s.upsert_node(&fid, "function", "main", "src/main.rs", Some(&file_id), Some("fn main()"), Some(1), Some(10)).await.unwrap();
        let nodes = s.get_nodes_by_folder(&fid).await.unwrap();
        assert_eq!(nodes.len(), 2);
        let by_file = s.get_nodes_by_file(&fid, "src/main.rs").await.unwrap();
        assert_eq!(by_file.len(), 2);
        s.delete_nodes_by_folder(&fid).await.unwrap();
        assert_eq!(s.get_nodes_by_folder(&fid).await.unwrap().len(), 0);
        let _ = (file_id, fn_id);
    }

    #[tokio::test]
    async fn upsert_persists_doc_and_symbol_kinds() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("kinds_{}", uuid::Uuid::new_v4())).await;
        // Each of these failed the enum cast before the fix and was dropped.
        for (kind, name, path) in [
            ("doc", "README", "README.md"),
            ("struct", "Point", "src/geo.rs"),
            ("component", "Button", "src/Button.svelte"),
            ("hook", "useState", "src/Button.svelte"),
            ("extension", "review", "marketplace/commands/review.md"),
        ] {
            s.upsert_node(&fid, kind, name, path, None, None, Some(1), Some(2))
                .await
                .unwrap_or_else(|e| panic!("upsert {kind} failed: {e}"));
        }
        let kinds = s.count_nodes_by_kind(&fid).await.unwrap();
        for kind in ["doc", "struct", "component", "hook", "extension"] {
            assert_eq!(kinds.get(kind), Some(&1), "missing {kind} node");
        }
        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn doc_nodes_are_embeddable() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("embed_{}", uuid::Uuid::new_v4())).await;
        s.upsert_node(&fid, "doc", "README", "README.md", None, None, Some(1), Some(2))
            .await.unwrap();
        let pending = s.nodes_without_embeddings(&fid, 100).await.unwrap();
        assert!(
            pending.iter().any(|(_, kind, name, _, _)| kind == "doc" && name == "README"),
            "doc node not returned by nodes_without_embeddings"
        );
        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn edge_insert_and_query() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("edge_{}", uuid::Uuid::new_v4())).await;
        let fn_a = s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();
        let fn_b = s.upsert_node(&fid, "function", "b", "b.rs", None, None, Some(1), Some(5)).await.unwrap();
        s.insert_edge(&fid, &fn_a, Some(&fn_b), None, "calls").await.unwrap();
        let callers = s.get_callers(&fn_b).await.unwrap();
        assert_eq!(callers.len(), 1);
        assert_eq!(callers[0]["caller_id"], fn_a.to_string());
        let callees = s.get_callees(&fn_a).await.unwrap();
        assert_eq!(callees.len(), 1);
        let by_kind = s.get_edges_by_kind(&fid, "calls").await.unwrap();
        assert_eq!(by_kind.len(), 1);
        s.delete_nodes_by_folder(&fid).await.unwrap(); // cascades edges
    }

    // ── Extensions tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn extension_create_and_list() {
        let s = pg_store().await;
        let name = format!("_test:ext_{}", uuid::Uuid::new_v4());
        let id = s.create_extension("skill", &name, Some("test skill"), Some("# content"), "global", "local").await.unwrap();
        let skills = s.list_extensions_by_kind("skill").await.unwrap();
        assert!(skills.iter().any(|e| e["name"] == name));
        s.delete_extension(&id).await.unwrap();
    }

    #[tokio::test]
    async fn extension_historize_trigger() {
        let s = pg_store().await;
        let name = format!("_test:ext_hist_{}", uuid::Uuid::new_v4());
        let id = s.create_extension("skill", &name, Some("v1"), None, "global", "local").await.unwrap();
        s.update_extension(&id, Some("v2"), None).await.unwrap();
        let history = s.get_extension_history(&id).await.unwrap();
        assert!(history.len() >= 2, "historize trigger should create INSERT + UPDATE entries");
        s.delete_extension(&id).await.unwrap();
    }

    // ── Folders tests ────────────────────────────────────────────────

    #[tokio::test]
    async fn folder_upsert_and_list() {
        let s = pg_store().await;
        let path = format!("/_test/folder_root_{}", uuid::Uuid::new_v4());
        let rid = s.add_watch_root(&path, "test_root", &serde_json::json!([])).await.unwrap();
        let fid = s.upsert_folder(&rid, "git", "myrepo", "myrepo", &format!("{}/myrepo", path), None, None).await.unwrap();
        let folders = s.list_folders_by_root(&rid).await.unwrap();
        assert!(folders.iter().any(|f| f["name"] == "myrepo"));
        s.delete_folder_tree(&fid).await.unwrap();
        s.remove_watch_root(&rid).await.unwrap();
    }

    #[tokio::test]
    async fn list_pending_folders_returns_only_non_terminal_status() {
        let s = pg_store().await;
        let root_path = format!("/_test/pending_resume_{}", uuid::Uuid::new_v4().simple());
        let rid = s.add_watch_root(&root_path, "pending_root", &serde_json::json!([])).await.unwrap();

        // Seed one folder per status. Default is 'discovered'; the rest are
        // forced with an explicit UPDATE because upsert_folder has no status
        // parameter and `mark_folder_indexed` is the only writer of `indexed`.
        for (status, suffix) in [
            ("discovered", "a"),
            ("queued",     "b"),
            ("indexing",   "c"),
            ("indexed",    "d"),
            ("failed",     "e"),
            ("deferred",   "f"),
        ] {
            let name = format!("repo_{}", suffix);
            let abs_path = format!("{}/{}", root_path, name);
            let fid = s.upsert_folder(&rid, "git", &name, &name, &abs_path, None, None).await.unwrap();
            sqlx_core::query::query(
                "UPDATE sensei.folders SET status = $2::sensei.folder_status WHERE id = $1"
            ).bind(fid).bind(status).execute(s.pool()).await.unwrap();
        }

        let rows = s.list_pending_folders().await.unwrap();
        let ours: Vec<_> = rows.iter()
            .filter(|r| r["abs_path"].as_str().unwrap_or("").starts_with(&root_path))
            .collect();

        // Only `discovered` and `queued` are non-terminal in the resume sense.
        // `indexing` would mean a worker is still running, which can't be true
        // at startup since the in-memory queue was just created.
        let statuses: std::collections::BTreeSet<&str> = ours.iter()
            .map(|r| r["status"].as_str().unwrap())
            .collect();
        assert_eq!(
            statuses,
            std::collections::BTreeSet::from(["discovered", "queued"]),
            "expected only discovered+queued, got {:?}", statuses
        );

        // Resume needs enough info to enqueue ProcessGitFolder: id, kind, abs_path.
        for r in &ours {
            assert!(r["id"].is_string(),       "row missing id: {:?}", r);
            assert!(r["kind"].is_string(),     "row missing kind: {:?}", r);
            assert!(r["abs_path"].is_string(), "row missing abs_path: {:?}", r);
        }

        // cleanup — removing the watch root cascades to folders.
        s.remove_watch_root(&rid).await.unwrap();
    }

    // ── Benchmark Reports tests ──────────────────────────────────────

    #[tokio::test]
    async fn benchmark_create_and_list() {
        let s = pg_store().await;
        let id = s.create_benchmark_report(None, "_test:bench", "strategy_a", Some(95.5), Some(1000), Some(5000)).await.unwrap();
        let reports = s.list_benchmark_reports().await.unwrap();
        assert!(reports.iter().any(|r| r["run_name"] == "_test:bench"));
        sqlx_core::query::query("DELETE FROM sensei.benchmark_reports WHERE id = $1").bind(id).execute(s.pool()).await.unwrap();
    }

    // ── Views tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn repositories_view() {
        let s = pg_store().await;
        // list_repositories returns git+subtree folders
        let repos = s.list_repositories().await.unwrap();
        // Just verify it doesn't error — content depends on seeded data
        // Just verify the query succeeds — content depends on seeded data
        let _ = repos;
    }

    // ── Memories tests ─────────────────────────────────────────────────

    #[tokio::test]
    async fn memory_create_and_get() {
        let s = pg_store().await;
        let id = s.create_memory(None, "global", None, "decision", "_test:mem_create", "Always use TDD", Some("Bugs ship to prod"), None).await.unwrap();
        let m = s.get_memory(&id).await.unwrap().unwrap();
        assert_eq!(m["title"], "_test:mem_create");
        assert_eq!(m["scope"], "global");
        assert_eq!(m["strength"], 1.0);
        assert_eq!(m["status"], "active");
        // cleanup via historize trigger test
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(id).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn memory_reinforce() {
        let s = pg_store().await;
        let id = s.create_memory(None, "global", None, "pattern", "_test:mem_reinforce", "rule", None, None).await.unwrap();
        s.reinforce_memory(&id, 1.0).await.unwrap();
        s.reinforce_memory(&id, 1.0).await.unwrap();
        let m = s.get_memory(&id).await.unwrap().unwrap();
        assert_eq!(m["strength"], 3.0); // 1.0 + 1.0 + 1.0
        // Cap at 5.0
        s.reinforce_memory(&id, 10.0).await.unwrap();
        let m = s.get_memory(&id).await.unwrap().unwrap();
        assert_eq!(m["strength"], 5.0);
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(id).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn memory_archive() {
        let s = pg_store().await;
        let id = s.create_memory(None, "global", None, "question", "_test:mem_archive", "open q", None, None).await.unwrap();
        s.archive_memory(&id).await.unwrap();
        let m = s.get_memory(&id).await.unwrap().unwrap();
        assert_eq!(m["status"], "archived");
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(id).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn memory_list_active() {
        let s = pg_store().await;
        let id1 = s.create_memory(None, "global", None, "decision", "_test:mem_list_a", "rule a", None, None).await.unwrap();
        let id2 = s.create_memory(None, "global", None, "decision", "_test:mem_list_b", "rule b", None, None).await.unwrap();
        let active = s.list_active_memories(None, Some("global")).await.unwrap();
        assert!(active.iter().any(|m| m["title"] == "_test:mem_list_a"));
        assert!(active.iter().any(|m| m["title"] == "_test:mem_list_b"));
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)").bind(&[id1, id2][..]).execute(s.pool()).await.unwrap();
    }

    // ── Memory Examples tests ────────────────────────────────────────

    #[tokio::test]
    async fn memory_example_add_and_list() {
        let s = pg_store().await;
        let mid = s.create_memory(None, "global", None, "pattern", "_test:mem_ex", "rule", None, None).await.unwrap();
        s.add_memory_example(&mid, "fn:auth_handler", true, Some("canonical auth")).await.unwrap();
        s.add_memory_example(&mid, "fn:inline_auth", false, Some("avoid inline")).await.unwrap();
        let examples = s.list_memory_examples(&mid).await.unwrap();
        assert_eq!(examples.len(), 2);
        assert!(examples.iter().any(|e| e["is_good"] == true));
        assert!(examples.iter().any(|e| e["is_good"] == false));
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(mid).execute(s.pool()).await.unwrap();
    }

    // ── Memory Evidence tests ────────────────────────────────────────

    #[tokio::test]
    async fn memory_evidence_add_and_list() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("mem_ev_{}", uuid::Uuid::new_v4())).await;
        let sid = s.create_session(&fid, "test", None).await.unwrap();
        let mid = s.create_memory(None, "global", None, "decision", "_test:mem_ev", "rule", None, None).await.unwrap();
        s.add_memory_evidence(&mid, &sid, Some("user corrected twice")).await.unwrap();
        let evidence = s.list_memory_evidence(&mid).await.unwrap();
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0]["note"], "user corrected twice");
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(mid).execute(s.pool()).await.unwrap();
    }

    // ── Memory Links tests ───────────────────────────────────────────

    #[tokio::test]
    async fn memory_links_parent_child() {
        let s = pg_store().await;
        let parent = s.create_memory(None, "global", None, "decision", "_test:mem_parent", "combined", None, None).await.unwrap();
        let child1 = s.create_memory(None, "global", None, "decision", "_test:mem_child1", "original 1", None, None).await.unwrap();
        let child2 = s.create_memory(None, "global", None, "decision", "_test:mem_child2", "original 2", None, None).await.unwrap();
        s.link_memories(&parent, &child1).await.unwrap();
        s.link_memories(&parent, &child2).await.unwrap();
        let children = s.get_memory_children(&parent).await.unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(s.get_memory_parent(&child1).await.unwrap(), Some(parent));
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
            .bind(&[parent, child1, child2][..]).execute(s.pool()).await.unwrap();
    }

    // ── Recommendations tests ────────────────────────────────────────

    #[tokio::test]
    async fn recommendation_lifecycle() {
        let s = pg_store().await;
        let pid = s.create_project("_test:rec_proj", None, None).await.unwrap();
        let rid = s.create_recommendation(&pid, "_test:rec", "reduces corrections", "promote_pattern", "high").await.unwrap();
        s.accept_recommendation(&rid).await.unwrap();
        s.measure_recommendation(&rid, "positive").await.unwrap();
        let recs = s.list_recommendations(&pid).await.unwrap();
        let r = recs.iter().find(|r| r["title"] == "_test:rec").unwrap();
        assert_eq!(r["status"], "accepted");
        assert_eq!(r["verdict"], "positive");
        sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id = $1").bind(rid).execute(s.pool()).await.unwrap();
        s.delete_project(&pid).await.unwrap();
    }

    // ── Communities tests ────────────────────────────────────────────

    #[tokio::test]
    async fn community_upsert_and_list() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("comm_{}", uuid::Uuid::new_v4())).await;
        let cid = s.upsert_community(&fid, 1, "_test:auth_cluster", 3).await.unwrap();
        let comms = s.list_communities(&fid).await.unwrap();
        assert!(comms.iter().any(|c| c["label"] == "_test:auth_cluster" && c["node_count"] == 3));
        sqlx_core::query::query("DELETE FROM inference.communities WHERE id = $1").bind(cid).execute(s.pool()).await.unwrap();
    }

    // ── Reasoning Traces tests ───────────────────────────────────────

    #[tokio::test]
    async fn reasoning_trace_insert_and_get() {
        let s = pg_store().await;
        let pid = s.create_project("_test:rt_proj", None, None).await.unwrap();
        let tid = s.insert_reasoning_trace(
            Some(&pid), "pattern_emerging", &["gemma4:27b".into()],
            &serde_json::json!([{"model":"gemma4","role":"proposer","content":"analyze"}]),
            &serde_json::json!({"conclusion":"adopt adapter pattern","confidence":0.9}),
        ).await.unwrap();
        let traces = s.get_reasoning_traces_by_project(&pid).await.unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0]["consensus"]["confidence"], 0.9);
        assert_eq!(traces[0]["trigger_event"], "pattern_emerging");
        sqlx_core::query::query("DELETE FROM inference.reasoning_traces WHERE id = $1").bind(tid).execute(s.pool()).await.unwrap();
        s.delete_project(&pid).await.unwrap();
    }

    // ── Folders to Watch tests ─────────────────────────────────────────

    #[tokio::test]
    async fn watch_root_add_and_list() {
        let s = pg_store().await;
        let path = format!("/_test/watch_{}", uuid::Uuid::new_v4());
        let id = s.add_watch_root(&path, "test_root", &serde_json::json!(["node_modules"])).await.unwrap();
        let roots = s.list_watch_roots().await.unwrap();
        assert!(roots.iter().any(|r| r["path"] == path));
        s.remove_watch_root(&id).await.unwrap();
    }

    #[tokio::test]
    async fn watch_root_update_status() {
        let s = pg_store().await;
        let path = format!("/_test/watch_status_{}", uuid::Uuid::new_v4());
        let id = s.add_watch_root(&path, "test", &serde_json::json!([])).await.unwrap();
        s.update_watch_status(&id, "watching").await.unwrap();
        let roots = s.list_watch_roots().await.unwrap();
        let r = roots.iter().find(|r| r["path"] == path).unwrap();
        assert_eq!(r["status"], "watching");
        s.remove_watch_root(&id).await.unwrap();
    }

    // ── Scan State tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn scan_state_upsert_and_stale() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("scan_{}", uuid::Uuid::new_v4())).await;
        s.upsert_scan_state(&fid, "src/main.rs", 1000, "hash1").await.unwrap();
        // Same mtime = not stale
        let stale = s.get_stale_files(&fid, &[("src/main.rs".into(), 1000)]).await.unwrap();
        assert!(stale.is_empty());
        // Changed mtime = stale
        let stale = s.get_stale_files(&fid, &[("src/main.rs".into(), 2000)]).await.unwrap();
        assert_eq!(stale, vec!["src/main.rs"]);
        // New file = stale
        let stale = s.get_stale_files(&fid, &[("src/new.rs".into(), 1000)]).await.unwrap();
        assert_eq!(stale, vec!["src/new.rs"]);
        s.delete_scan_state(&fid).await.unwrap();
    }

    // ── Services tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn service_upsert_and_list() {
        let s = pg_store().await;
        let name = format!("_test:svc_{}", uuid::Uuid::new_v4());
        let id = s.upsert_service(&name, "Test MCP", "data", "mcp", &serde_json::json!({"url":"http://localhost"})).await.unwrap();
        let svcs = s.list_services().await.unwrap();
        assert!(svcs.iter().any(|sv| sv["name"] == name));
        s.delete_service(&name).await.unwrap();
        let _ = id;
    }

    // ── Snapshots tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn snapshot_create_and_get_latest() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("snap_{}", uuid::Uuid::new_v4())).await;
        let sid = s.create_session(&fid, "snapshot test", None).await.unwrap();
        s.create_snapshot(&sid, &fid, "manual", "Step 1 done", Some("Do step 2"), &["Step 1".into()]).await.unwrap();
        s.create_snapshot(&sid, &fid, "checkpoint", "Step 2 done", None, &["Step 1".into(), "Step 2".into()]).await.unwrap();
        let latest = s.get_latest_snapshot(&sid).await.unwrap().unwrap();
        assert_eq!(latest["progress_summary"], "Step 2 done");
        assert_eq!(latest["kind"], "checkpoint");
        assert_eq!(latest["completed_steps"].as_array().unwrap().len(), 2);
    }

    // ── Detected Patterns tests ────────────────────────────────────────

    #[tokio::test]
    async fn pattern_upsert_and_list() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "pat_upsert").await;
        let instances = serde_json::json!([{"file":"src/lib.rs","line":10},{"file":"src/main.rs","line":20}]);
        let pid = s.upsert_pattern(&fid, "_test:Adapter", false, Some(0.85), &instances).await.unwrap();
        let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
        assert!(patterns.iter().any(|p| p["name"] == "_test:Adapter" && p["instance_count"] == 2));
        // cleanup
        sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
            .bind(pid).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn pattern_promote() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "pat_promote").await;
        let pid = s.upsert_pattern(&fid, "_test:Factory", false, None, &serde_json::json!([])).await.unwrap();
        s.promote_pattern(&pid, "rule").await.unwrap();
        let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
        let p = patterns.iter().find(|p| p["id"] == pid.to_string()).unwrap();
        assert_eq!(p["lifecycle"], "rule");
        sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
            .bind(pid).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn pattern_upsert_updates_existing() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "pat_dup").await;
        let id1 = s.upsert_pattern(&fid, "_test:Singleton", false, Some(0.5), &serde_json::json!([{"file":"a.rs"}])).await.unwrap();
        let id2 = s.upsert_pattern(&fid, "_test:Singleton", false, Some(0.9), &serde_json::json!([{"file":"a.rs"},{"file":"b.rs"}])).await.unwrap();
        assert_eq!(id1, id2); // same row updated
        let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
        let p = patterns.iter().find(|p| p["name"] == "_test:Singleton").unwrap();
        assert_eq!(p["instance_count"], 2);
        sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
            .bind(id1).execute(s.pool()).await.unwrap();
    }

    // ── Libraries tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn library_upsert_and_get() {
        let s = pg_store().await;
        let id = s.upsert_library("_test:tokio", "cargo", Some("1.0"), Some("async runtime"), None, None).await.unwrap();
        let lib = s.get_library(&id).await.unwrap().unwrap();
        assert_eq!(lib["name"], "_test:tokio");
        assert_eq!(lib["ecosystem"], "cargo");
        assert_eq!(lib["version"], "1.0");
        s.delete_library(&id).await.unwrap();
    }

    #[tokio::test]
    async fn project_library_promotion_shows_in_resolved_and_is_idempotent() {
        // #30: referenced_libraries (folder-grained) must roll up to
        // project_libraries so detected libs — incl. scoped @rokkit/* — show in
        // project_libraries_resolved (the Projects screen). Was never populated.
        let s = pg_store().await;
        let pid = s.ensure_test_project("proj-lib-promo").await.unwrap();
        let lib = s.upsert_library("_test:@rokkit/core", "npm", Some("1.2"), None, None, None).await.unwrap();
        // Promote twice — must be idempotent (no error, no duplicate row).
        s.upsert_project_library(&lib, &pid).await.unwrap();
        s.upsert_project_library(&lib, &pid).await.unwrap();
        let libs = s.get_project_libraries(&pid).await.unwrap();
        let hits = libs.iter().filter(|l| l["name"] == "_test:@rokkit/core").count();
        assert_eq!(hits, 1, "promoted scoped lib should appear exactly once in resolved view; got {libs:?}");
        s.delete_library(&lib).await.unwrap(); // FK CASCADE removes the project_libraries row
    }

    #[tokio::test]
    async fn ensure_test_project_is_namespaced_and_idempotent() {
        // #34: test fixtures must not accrete a new row per run, nor look like
        // real projects. Reuse one `_test:`-namespaced row per name.
        let s = pg_store().await;
        let a = s.ensure_test_project("dup-check").await.unwrap();
        let b = s.ensure_test_project("dup-check").await.unwrap();
        assert_eq!(a, b, "repeated ensure_test_project must reuse one row, not create a new one");
        let proj = s.get_project(&a).await.unwrap().unwrap();
        assert_eq!(proj["name"], "_test:dup-check", "test projects must be _test:-namespaced");
        s.delete_project(&a).await.ok();
    }

    #[tokio::test]
    async fn find_folder_for_path_returns_nearest_ancestor() {
        // #31: a hook's cwd (often a subdir) must resolve to its indexed folder.
        let s = pg_store().await;
        let fid = create_test_folder(&s, "sess-nearest").await; // abs_path /_test/sess-nearest
        assert_eq!(s.find_folder_for_path("/_test/sess-nearest/src/auth").await.unwrap()
            .map(|(id, _)| id), Some(fid), "subdir cwd resolves to ancestor folder");
        assert_eq!(s.find_folder_for_path("/_test/sess-nearest").await.unwrap()
            .map(|(id, _)| id), Some(fid), "exact path resolves too");
        assert_eq!(s.find_folder_for_path("/_test/nonexistent-xyz/deep").await.unwrap(), None,
            "uncovered path resolves to nothing");
    }

    #[tokio::test]
    async fn record_session_event_folds_into_one_row_and_completes() {
        // #31: every hook event of a session folds into one row keyed by the
        // assistant session id; Stop/SessionEnd marks it completed.
        let s = pg_store().await;
        let fid = create_test_folder(&s, "sess-record").await;
        let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
        let id1 = s.record_session_event(&sid, &fid, None, "claude", false).await.unwrap();
        let id2 = s.record_session_event(&sid, &fid, None, "claude", false).await.unwrap();
        assert_eq!(id1, id2, "same client_session_id must fold into one session row");
        assert!(s.get_session(&id1).await.unwrap().unwrap()["completed_at"].is_null(),
            "not completed before an end event");
        let id3 = s.record_session_event(&sid, &fid, None, "claude", true).await.unwrap();
        assert_eq!(id3, id1, "end event updates the same row");
        assert!(!s.get_session(&id1).await.unwrap().unwrap()["completed_at"].is_null(),
            "Stop/SessionEnd sets completed_at");
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
            .bind(id1).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn list_all_sessions_joins_project_and_uses_camelcase_times() {
        // #61: the observatory reads project name + startedAt/completedAt. The
        // returned row must carry the joined project NAME (not a bare folder
        // uuid) under camelCase timestamp keys, with completedAt set once the
        // session ends — otherwise every displayed column renders blank.
        let s = pg_store().await;
        let proj_name = format!("_test:obs-{}", uuid::Uuid::new_v4());
        let pid = s.create_project(&proj_name, None, None).await.unwrap();
        let fid = create_test_folder(&s, "obs-sess").await;
        let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
        let session_id = s.record_session_event(&sid, &fid, Some(&pid), "claude", false).await.unwrap();
        s.record_session_event(&sid, &fid, Some(&pid), "claude", true).await.unwrap();

        let all = s.list_all_sessions(500).await.unwrap();
        let row = all.iter()
            .find(|r| r["id"].as_str() == Some(session_id.to_string().as_str()))
            .expect("our session is listed");

        assert_eq!(row["project"], serde_json::json!(proj_name), "project name is joined, not a folder uuid");
        assert!(row["startedAt"].as_str().is_some(), "startedAt present (camelCase)");
        assert!(row.get("started_at").is_none(), "no stale snake_case started_at key");
        assert!(row["completedAt"].as_str().is_some(), "completedAt set after the end event");
        assert!(row.get("folder_id").is_none(), "folder_id no longer leaks in place of the project");

        sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
            .bind(session_id).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(pid).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn get_project_repos_excludes_subfolder_tree() {
        // #62: a single-repo project with subfolders must list only its repo
        // root(s), never the kind='folder' subfolder tree — else the UI shows it
        // as a multi-repo project with every folder as a repo.
        let s = pg_store().await;
        let pid = s.create_project(&format!("_test:repos-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        s.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) ON CONFLICT DO NOTHING"
        ).await.unwrap();
        let git_abs = format!("/_test/repos-git-{}", uuid::Uuid::new_v4());
        let sub_abs = format!("/_test/repos-sub-{}", uuid::Uuid::new_v4());
        sqlx_core::query::query(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path, project_id) VALUES
               ('00000000-0000-0000-0000-000000000001','git'::sensei.folder_kind,'the-repo','the-repo',$1,$3),
               ('00000000-0000-0000-0000-000000000001','folder'::sensei.folder_kind,'subdir','subdir',$2,$3)"
        ).bind(&git_abs).bind(&sub_abs).bind(pid).execute(s.pool()).await.unwrap();

        let repos = s.get_project_repos(&pid).await.unwrap();
        let kinds: Vec<String> = repos.iter().filter_map(|r| r["kind"].as_str().map(str::to_string)).collect();
        assert!(kinds.iter().any(|k| k == "git"), "the repo root is listed: {kinds:?}");
        assert!(!kinds.iter().any(|k| k == "folder"), "kind=folder subfolders excluded: {kinds:?}");

        sqlx_core::query::query("DELETE FROM sensei.folders WHERE project_id = $1").bind(pid).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn library_upsert_updates() {
        let s = pg_store().await;
        let id1 = s.upsert_library("_test:react", "npm", Some("18"), None, None, None).await.unwrap();
        let id2 = s.upsert_library("_test:react", "npm", Some("19"), Some("UI library"), None, None).await.unwrap();
        assert_eq!(id1, id2);
        let lib = s.get_library(&id1).await.unwrap().unwrap();
        assert_eq!(lib["version"], "19");
        assert_eq!(lib["description"], "UI library");
        s.delete_library(&id1).await.unwrap();
    }

    #[tokio::test]
    async fn library_list() {
        let s = pg_store().await;
        let id1 = s.upsert_library("_test:lib_a", "npm", None, None, None, None).await.unwrap();
        let id2 = s.upsert_library("_test:lib_b", "cargo", None, None, None, None).await.unwrap();
        let all = s.list_libraries().await.unwrap();
        assert!(all.iter().any(|l| l["name"] == "_test:lib_a"));
        assert!(all.iter().any(|l| l["name"] == "_test:lib_b"));
        s.delete_library(&id1).await.unwrap();
        s.delete_library(&id2).await.unwrap();
    }

    #[tokio::test]
    async fn library_delete() {
        let s = pg_store().await;
        let id = s.upsert_library("_test:deleteme", "npm", None, None, None, None).await.unwrap();
        s.delete_library(&id).await.unwrap();
        assert!(s.get_library(&id).await.unwrap().is_none());
    }

    // ── Sessions + Events tests ────────────────────────────────────────

    #[tokio::test]
    async fn session_create_and_get() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "sess_create").await;
        let sid = s.create_session(&fid, "fix bug #42", Some("claude-code")).await.unwrap();
        let sess = s.get_session(&sid).await.unwrap().unwrap();
        assert_eq!(sess["task"], "fix bug #42");
        assert_eq!(sess["acp_id"], "claude-code");
        assert!(sess["outcome"].is_null());
        assert_eq!(sess["turns"], 0);
    }

    #[tokio::test]
    async fn session_complete() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "sess_complete").await;
        let sid = s.create_session(&fid, "add feature", None).await.unwrap();
        s.complete_session(&sid, "completed", true, 5, 0).await.unwrap();
        let sess = s.get_session(&sid).await.unwrap().unwrap();
        assert_eq!(sess["outcome"], "completed");
        assert_eq!(sess["ftr"], true);
        assert_eq!(sess["turns"], 5);
        assert!(sess["completed_at"].as_str().is_some());
    }

    #[tokio::test]
    async fn session_list_by_folder() {
        let s = pg_store().await;
        let suffix = format!("sess_list_{}", uuid::Uuid::new_v4());
        let fid = create_test_folder(&s, &suffix).await;
        s.create_session(&fid, "task 1", None).await.unwrap();
        s.create_session(&fid, "task 2", None).await.unwrap();
        let sessions = s.list_sessions_by_folder(&fid, 10).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn event_insert_and_get() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "evt_insert").await;
        let sid = s.create_session(&fid, "test", None).await.unwrap();
        let data = serde_json::json!({"tool_name": "search", "duration_ms": 42});
        s.insert_event(&sid, &fid, "tool_call", Some(1), &data).await.unwrap();
        let events = s.get_events_by_session(&sid).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["event_type"], "tool_call");
        assert_eq!(events[0]["data"]["tool_name"], "search");
    }

    #[tokio::test]
    async fn event_get_by_type() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("evt_type_{}", uuid::Uuid::new_v4())).await;
        let sid = s.create_session(&fid, "test", None).await.unwrap();
        s.insert_event(&sid, &fid, "correction", None, &serde_json::json!({"description": "wrong indent"})).await.unwrap();
        s.insert_event(&sid, &fid, "tool_call", Some(1), &serde_json::json!({"tool_name": "grep"})).await.unwrap();
        let corrections = s.get_events_by_type(&fid, "correction").await.unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0]["data"]["description"], "wrong indent");
    }

    #[tokio::test]
    async fn session_get_nonexistent() {
        let s = pg_store().await;
        assert!(s.get_session(&uuid::Uuid::new_v4()).await.unwrap().is_none());
    }

    // ── Hook events tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn hook_event_insert_and_query() {
        let s = pg_store().await;
        let session_id = format!("test-session-{}", uuid::Uuid::new_v4());
        let payload = serde_json::json!({
            "session_id": session_id,
            "hook_event_name": "PreToolUse",
            "assistant_family": "claude",
            "tool_name": "Read",
            "cwd": "/tmp/test",
        });
        let id = s.insert_hook_event(
            &session_id, "claude", "PreToolUse", Some("Read"), Some("/tmp/test"),
            chrono::Utc::now().timestamp_millis(), None, &payload,
        ).await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn hook_event_post_tool_use_success() {
        let s = pg_store().await;
        let session_id = format!("test-session-{}", uuid::Uuid::new_v4());
        let payload = serde_json::json!({"hook_event_name": "PostToolUse", "assistant_family": "claude", "tool_name": "Bash"});
        let id = s.insert_hook_event(
            &session_id, "claude", "PostToolUse", Some("Bash"), None,
            chrono::Utc::now().timestamp_millis(), Some(true), &payload,
        ).await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn hook_event_no_tool_name() {
        let s = pg_store().await;
        let session_id = format!("test-session-{}", uuid::Uuid::new_v4());
        let payload = serde_json::json!({"hook_event_name": "SessionStart", "assistant_family": "claude", "model": "claude-sonnet-4"});
        let id = s.insert_hook_event(
            &session_id, "claude", "SessionStart", None, Some("/home/user/project"),
            chrono::Utc::now().timestamp_millis(), None, &payload,
        ).await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn hook_event_cursor_family() {
        let s = pg_store().await;
        let session_id = format!("cursor-session-{}", uuid::Uuid::new_v4());
        let payload = serde_json::json!({"hook_event_name": "SessionStart", "assistant_family": "cursor"});
        let id = s.insert_hook_event(
            &session_id, "cursor", "SessionStart", None, Some("/home/user/project"),
            chrono::Utc::now().timestamp_millis(), None, &payload,
        ).await.unwrap();
        assert!(id > 0);
    }

    // ── Projects tests ────────────────────────────────────────────────

    #[tokio::test]
    async fn project_create_and_get() {
        let s = pg_store().await;
        let id = s.create_project("_test:proj:create", Some("desc"), Some("client")).await.unwrap();
        let p = s.get_project(&id).await.unwrap().unwrap();
        assert_eq!(p["name"], "_test:proj:create");
        assert_eq!(p["description"], "desc");
        assert_eq!(p["client"], "client");
        assert_eq!(p["maturity"], "discovery"); // default
        s.delete_project(&id).await.unwrap();
    }

    #[tokio::test]
    async fn project_list() {
        let s = pg_store().await;
        let id1 = s.create_project("_test:proj:list_a", None, None).await.unwrap();
        let id2 = s.create_project("_test:proj:list_b", None, None).await.unwrap();
        let all = s.list_projects().await.unwrap();
        let names: Vec<&str> = all.iter().filter_map(|p| p["name"].as_str()).collect();
        assert!(names.contains(&"_test:proj:list_a"));
        assert!(names.contains(&"_test:proj:list_b"));
        s.delete_project(&id1).await.unwrap();
        s.delete_project(&id2).await.unwrap();
    }

    #[tokio::test]
    async fn project_update() {
        let s = pg_store().await;
        let id = s.create_project("_test:proj:update", None, None).await.unwrap();
        s.update_project(&id, Some("renamed"), None, Some("active")).await.unwrap();
        let p = s.get_project(&id).await.unwrap().unwrap();
        assert_eq!(p["name"], "renamed");
        assert_eq!(p["maturity"], "active");
        s.delete_project(&id).await.unwrap();
    }

    #[tokio::test]
    async fn project_delete() {
        let s = pg_store().await;
        let id = s.create_project("_test:proj:delete", None, None).await.unwrap();
        s.delete_project(&id).await.unwrap();
        assert!(s.get_project(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn project_get_nonexistent() {
        let s = pg_store().await;
        let fake = uuid::Uuid::new_v4();
        assert!(s.get_project(&fake).await.unwrap().is_none());
    }

    // ── Index Errors tests ───────────────────────────────────────────

    #[tokio::test]
    async fn idx_err_log_and_get() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "idx_err_log").await;
        s.clear_index_errors(&fid).await.unwrap(); // ensure clean
        s.log_index_error(&fid, "src/bad.ts", "SyntaxError", Some("typescript"), None).await.unwrap();
        s.log_index_error(&fid, "src/x.py", "IndentError", Some("python"), Some("parse")).await.unwrap();
        let errors = s.get_index_errors(Some(&fid)).await.unwrap();
        assert_eq!(errors.len(), 2);
        s.clear_index_errors(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn idx_err_clear() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "idx_err_clear").await;
        s.clear_index_errors(&fid).await.unwrap();
        s.log_index_error(&fid, "a.rs", "err", Some("rust"), None).await.unwrap();
        s.clear_index_errors(&fid).await.unwrap();
        assert_eq!(s.get_index_errors(Some(&fid)).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn idx_err_empty() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "idx_err_empty").await;
        s.clear_index_errors(&fid).await.unwrap();
        assert_eq!(s.get_index_errors(Some(&fid)).await.unwrap().len(), 0);
    }

    // ── Workflow State tests ────────────────────────────────────────────

    #[tokio::test]
    async fn wf_upsert_and_get() {
        let s = pg_store().await;
        let p = "_test:wf:upsert";
        s.delete_workflow_state(p).await.unwrap();
        assert!(s.get_workflow_state(p).await.unwrap().is_none());
        s.upsert_workflow_state(p, Some("ideate"), None, None, None, None, None).await.unwrap();
        let state = s.get_workflow_state(p).await.unwrap().unwrap();
        assert_eq!(state["active_phase"], "ideate");
        assert!(state["active_task"].is_null());
        s.delete_workflow_state(p).await.unwrap();
    }

    #[tokio::test]
    async fn wf_partial_update_preserves() {
        let s = pg_store().await;
        let p = "_test:wf:partial";
        s.delete_workflow_state(p).await.unwrap();
        s.upsert_workflow_state(p, Some("build"), Some("plan.md"), Some("task 1"), Some(42), None, Some("hash123")).await.unwrap();
        s.upsert_workflow_state(p, Some("validate"), None, None, None, None, None).await.unwrap();
        let state = s.get_workflow_state(p).await.unwrap().unwrap();
        assert_eq!(state["active_phase"], "validate");
        assert_eq!(state["active_plan"], "plan.md");
        assert_eq!(state["active_task"], "task 1");
        assert_eq!(state["active_issue"], 42);
        s.delete_workflow_state(p).await.unwrap();
    }

    #[tokio::test]
    async fn wf_nonexistent_returns_none() {
        let s = pg_store().await;
        assert!(s.get_workflow_state("_test:wf:none").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn wf_delete() {
        let s = pg_store().await;
        let p = "_test:wf:delete";
        s.upsert_workflow_state(p, Some("ideate"), None, None, None, None, None).await.unwrap();
        s.delete_workflow_state(p).await.unwrap();
        assert!(s.get_workflow_state(p).await.unwrap().is_none());
    }

    // ── Tags tests ────────────────────────────────────────────────────

    #[tokio::test]
    async fn tag_add_and_list() {
        let s = pg_store().await;
        let tag = "_test:tag_add:rust";
        s.add_tag(tag, Some("stack")).await.unwrap();
        let tags = s.list_tags().await.unwrap();
        assert!(tags.iter().any(|(t, c)| t == tag && c.as_deref() == Some("stack")));
        s.remove_tag(tag).await.unwrap();
    }

    #[tokio::test]
    async fn tag_add_without_category() {
        let s = pg_store().await;
        let tag = "_test:tag_nocat:misc";
        s.add_tag(tag, None).await.unwrap();
        let tags = s.list_tags().await.unwrap();
        assert!(tags.iter().any(|(t, c)| t == tag && c.is_none()));
        s.remove_tag(tag).await.unwrap();
    }

    #[tokio::test]
    async fn tag_add_duplicate_is_upsert() {
        let s = pg_store().await;
        let tag = "_test:tag_dup:ts";
        s.add_tag(tag, Some("stack")).await.unwrap();
        s.add_tag(tag, Some("language")).await.unwrap(); // update category
        let tags = s.list_tags().await.unwrap();
        let found: Vec<_> = tags.iter().filter(|(t, _)| t == tag).collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1.as_deref(), Some("language"));
        s.remove_tag(tag).await.unwrap();
    }

    #[tokio::test]
    async fn tag_remove() {
        let s = pg_store().await;
        let tag = "_test:tag_rm:go";
        s.add_tag(tag, Some("stack")).await.unwrap();
        s.remove_tag(tag).await.unwrap();
        let tags = s.list_tags().await.unwrap();
        assert!(!tags.iter().any(|(t, _)| t == tag));
    }

    #[tokio::test]
    async fn tag_remove_nonexistent_is_noop() {
        let s = pg_store().await;
        s.remove_tag("_test:tag_rm_noop:xyz").await.unwrap();
    }

    #[tokio::test]
    async fn tag_list_by_category() {
        let s = pg_store().await;
        let t1 = "_test:tag_cat:rust";
        let t2 = "_test:tag_cat:ts";
        let t3 = "_test:tag_cat:active";
        s.add_tag(t1, Some("stack")).await.unwrap();
        s.add_tag(t2, Some("stack")).await.unwrap();
        s.add_tag(t3, Some("status")).await.unwrap();
        let stack_tags = s.list_tags_by_category("stack").await.unwrap();
        assert!(stack_tags.contains(&t1.to_string()));
        assert!(stack_tags.contains(&t2.to_string()));
        assert!(!stack_tags.contains(&t3.to_string()));
        s.remove_tag(t1).await.unwrap();
        s.remove_tag(t2).await.unwrap();
        s.remove_tag(t3).await.unwrap();
    }

    // ── Schema tests ─────────────────────────────────────────────────

    #[tokio::test]
    async fn memories_table_exists() {
        let store = PgStore::connect(&test_db_url()).await.unwrap();
        let row: (bool,) = query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = 'sensei' AND table_name = 'memories')"
        )
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert!(row.0, "sensei.memories table must exist — run `dbd apply` first");
    }

    // ── Knowledge Sources tests ───────────────────────────────────────

    #[tokio::test]
    async fn knowledge_source_crud_roundtrip() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let id = pg.create_knowledge_source(&NewKnowledgeSource {
            kind: "hive_mind".into(), name: "Org Hive".into(), url: "https://hive.example".into(),
            namespace_id: None, credential_ref: "hive-test".into(), direction: "both".into(),
        }).await.unwrap();

        let all = pg.list_knowledge_sources().await.unwrap();
        assert!(all.iter().any(|s| s.id == id && s.last_seq == 0 && s.enabled));

        pg.set_source_cursor(&id, 42).await.unwrap();
        let one = pg.get_knowledge_source(&id).await.unwrap().unwrap();
        assert_eq!(one.last_seq, 42);
        assert_eq!(one.direction, "both");

        assert!(pg.delete_knowledge_source(&id).await.unwrap());
        assert!(pg.get_knowledge_source(&id).await.unwrap().is_none());
    }

    // ── scope_folder_ids tests (#60) ─────────────────────────────────

    /// Build an isolated project + root folder + child subfolder for scope tests.
    async fn setup_scope_test(s: &PgStore, suffix: &str) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid) {
        let proj_name = format!("_test:scope:{}", suffix);
        let proj_id = s.create_project(&proj_name, None, None).await.unwrap();

        // Root folder: upsert into folders_to_watch first (foreign-key for root_id).
        let watch_path = format!("/_test/scope_{}", suffix);
        let watch_id = s.add_watch_root(&watch_path, &format!("scope_root_{}", suffix), &serde_json::json!([])).await.unwrap();

        // Root repo folder (kind='git', owns root_id = watch_id).
        let root_abs = format!("/_test/scope_{}/root", suffix);
        let root_name = format!("scope_root_{}", suffix);
        let root_id = s.upsert_repo(&watch_id, &root_name, &root_abs).await.unwrap();
        s.set_folder_project(&root_id, &proj_id, "main", None).await.unwrap();

        // Child subfolder (kind='folder', parent = root, project = proj_id).
        let child_abs = format!("/_test/scope_{}/root/child", suffix);
        let child_name = format!("scope_child_{}", suffix);
        let child_id = s.upsert_subfolder(&watch_id, &child_name, &child_name, &child_abs, Some(&root_id), Some(&proj_id)).await.unwrap();

        (proj_id, root_id, child_id)
    }

    #[tokio::test]
    async fn scope_folder_ids_by_project_name_returns_all_folders() {
        let s = pg_store().await;
        let uid = uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
        let (proj_id, root_id, child_id) = setup_scope_test(&s, &uid).await;
        let proj_name = format!("_test:scope:{}", uid);

        let ids = s.scope_folder_ids(&proj_name).await.unwrap();
        assert!(ids.contains(&root_id), "root folder must be in scope ids; got {:?}", ids);
        assert!(ids.contains(&child_id), "child folder must be in scope ids; got {:?}", ids);

        // Also test by UUID string.
        let by_uuid = s.scope_folder_ids(&proj_id.to_string()).await.unwrap();
        assert!(by_uuid.contains(&child_id), "UUID lookup must find child; got {:?}", by_uuid);

        // Nonexistent ident returns empty.
        let empty = s.scope_folder_ids("nonexistent-xyz-scope-test-noop").await.unwrap();
        assert!(empty.is_empty(), "nonexistent must be empty; got {:?}", empty);

        // Cleanup.
        s.delete_nodes_by_folder(&root_id).await.unwrap();
        s.delete_nodes_by_folder(&child_id).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = ANY($1::uuid[])")
            .bind(vec![child_id, root_id]).execute(s.pool()).await.unwrap();
        s.delete_project(&proj_id).await.unwrap();
    }

    // ── project-scoped query variants tests (#60) ─────────────────────

    #[tokio::test]
    async fn scoped_search_and_count_across_child_folder() {
        let s = pg_store().await;
        let uid = uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
        let (proj_id, root_id, child_id) = setup_scope_test(&s, &uid).await;
        let proj_name = format!("_test:scope:{}", uid);

        // Insert a function node in the CHILD folder.
        let fn_id = s.upsert_node(&child_id, "function", "widget_builder", "src/widget.rs", None, Some("fn widget_builder()"), Some(1), Some(10)).await.unwrap();
        // Insert a callee node (target) in child folder.
        let tgt_id = s.upsert_node(&child_id, "function", "render_widget", "src/widget.rs", None, Some("fn render_widget()"), Some(12), Some(20)).await.unwrap();
        // Insert resolved edge: widget_builder calls render_widget.
        s.insert_edge(&child_id, &fn_id, Some(&tgt_id), Some("render_widget"), "calls").await.unwrap();

        // Resolve scope.
        let ids = s.scope_folder_ids(&proj_name).await.unwrap();
        assert!(!ids.is_empty());

        // search_functions_scoped must find widget_builder.
        let fns = s.search_functions_scoped(&ids, "widget_builder").await.unwrap();
        assert!(
            fns.iter().any(|f| f["name"] == "widget_builder"),
            "expected widget_builder in {:?}", fns
        );

        // count_nodes_by_kind_scoped must report at least 2 functions.
        let counts = s.count_nodes_by_kind_scoped(&ids).await.unwrap();
        let fn_count = counts.get("function").copied().unwrap_or(0);
        assert!(fn_count >= 2, "expected >=2 function nodes, got {:?}", counts);

        // get_nodes_scoped must include child nodes.
        let nodes = s.get_nodes_scoped(&ids).await.unwrap();
        assert!(nodes.iter().any(|n| n["name"] == "widget_builder"), "nodes_scoped missing widget_builder");

        // get_edges_scoped must return the calls edge.
        let edges = s.get_edges_scoped(&ids, "calls").await.unwrap();
        assert!(!edges.is_empty(), "expected >=1 calls edge in scoped result");

        // get_callers_by_name with project name: render_widget is called by widget_builder.
        let callers = s.get_callers_by_name(&proj_name, "render_widget").await.unwrap();
        assert!(
            callers.iter().any(|c| c["name"] == "widget_builder"),
            "expected widget_builder as caller of render_widget; got {:?}", callers
        );

        // get_callees_by_name with project name: widget_builder calls render_widget.
        let callees = s.get_callees_by_name(&proj_name, "widget_builder").await.unwrap();
        assert!(
            callees.iter().any(|c| c["name"] == "render_widget"),
            "expected render_widget as callee of widget_builder; got {:?}", callees
        );

        // Cleanup.
        s.delete_nodes_by_folder(&child_id).await.unwrap(); // cascades edges
        s.delete_nodes_by_folder(&root_id).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = ANY($1::uuid[])")
            .bind(vec![child_id, root_id]).execute(s.pool()).await.unwrap();
        s.delete_project(&proj_id).await.unwrap();
        let _ = (fn_id, tgt_id);
    }
}

#[cfg(test)]
mod knowledge_tests {
    use super::*;

    fn ddl_test_skip() -> bool {
        // Tests require a running sensei_dev DB. Skip if env var not set.
        std::env::var("SENSEI_TEST_DB_URL").is_err()
    }

    #[tokio::test]
    async fn list_memories_filters_by_status() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let project_id = pg.ensure_test_project("list-status").await.unwrap();
        let m1 = pg.insert_memory(&InsertMemory {
            project_id: Some(project_id), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "t1".into(), content: "c1".into(),
            impact: None, tags: vec![], triage_signal: None, status: "proposed".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
        }).await.unwrap();
        let _m2 = pg.insert_memory(&InsertMemory {
            project_id: Some(project_id), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "t2".into(), content: "c2".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
        }).await.unwrap();

        let proposed = pg.list_memories(Some(project_id), Some("proposed"), None, 50).await.unwrap();
        assert_eq!(proposed.len(), 1);
        assert_eq!(proposed[0]["id"].as_str().unwrap(), m1.to_string());
    }

    #[tokio::test]
    async fn set_memory_status_accept_proposal() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let pid = pg.ensure_test_project("accept-prop").await.unwrap();
        let mid = pg.insert_memory(&InsertMemory {
            project_id: Some(pid), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "t".into(), content: "c".into(),
            impact: None, tags: vec![], triage_signal: Some("revert".into()),
            status: "proposed".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
        }).await.unwrap();

        let new_status = pg.set_memory_status(mid, "active", &["proposed"]).await.unwrap();
        assert_eq!(new_status.as_deref(), Some("active"));

        // Trying to accept a now-active memory fails.
        let err = pg.set_memory_status(mid, "active", &["proposed"]).await;
        assert!(err.is_err() || err.unwrap().is_none(), "second accept should not match WHERE clause");
    }

    #[tokio::test]
    async fn get_memory_detail_includes_outcomes() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let pid = pg.ensure_test_project("detail").await.unwrap();
        let mid = pg.insert_memory(&InsertMemory {
            project_id: Some(pid), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "t".into(), content: "c".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
        }).await.unwrap();
        let skipped = pg.record_outcomes_batch(&[
            OutcomeRow { memory_id: mid, session_id: None, outcome: "applied".into(), context: None }
        ]).await.unwrap();
        assert_eq!(skipped.len(), 0);

        let detail = pg.get_memory_detail(mid).await.unwrap();
        assert!(detail["memory"]["id"].as_str().unwrap() == mid.to_string());
        assert_eq!(detail["outcomes"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn assemble_context_blends_three_scopes() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let pid = pg.ensure_test_project("blend").await.unwrap();

        pg.insert_memory(&InsertMemory {
            project_id: Some(pid), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "P".into(), content: "p".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
        }).await.unwrap();
        pg.insert_memory(&InsertMemory {
            project_id: None, scope: "stack".into(), scope_filter: Some("rust".into()),
            mtype: "convention".into(), title: "S".into(), content: "s".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
        }).await.unwrap();
        pg.insert_memory(&InsertMemory {
            project_id: None, scope: "global".into(), scope_filter: None,
            mtype: "convention".into(), title: "G".into(), content: "g".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
        }).await.unwrap();

        let blob = pg.assemble_context(pid, &["rust".into()], None, 50).await.unwrap();
        let titles: Vec<String> = blob["memories"].as_array().unwrap().iter()
            .map(|m| m["title"].as_str().unwrap().to_string()).collect();
        assert!(titles.contains(&"P".to_string()));
        assert!(titles.contains(&"S".to_string()));
        assert!(titles.contains(&"G".to_string()));

        // Proposed memories must not appear.
        let m_prop = pg.insert_memory(&InsertMemory {
            project_id: Some(pid), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "PROP".into(), content: "x".into(),
            impact: None, tags: vec![], triage_signal: Some("revert".into()),
            status: "proposed".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
        }).await.unwrap();
        let blob2 = pg.assemble_context(pid, &["rust".into()], None, 50).await.unwrap();
        let titles2: Vec<String> = blob2["memories"].as_array().unwrap().iter()
            .map(|m| m["title"].as_str().unwrap().to_string()).collect();
        assert!(!titles2.contains(&"PROP".to_string()));
        let _ = m_prop;
    }

    #[tokio::test]
    async fn insert_memory_persists_source_id() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let src = uuid::Uuid::new_v4();
        let id = pg.insert_memory(&InsertMemory {
            project_id: None, scope: "global".into(), scope_filter: None,
            mtype: "convention".into(), title: "fed".into(), content: "federated content".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: Some("recommended".into()),
            origin: Some("federated".into()), source_id: Some(src),
        }).await.unwrap();
        let got: (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT source_id FROM sensei.memories WHERE id = $1")
            .bind(id).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(got.0, Some(src));
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(id).execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn federated_ledger_and_shareability() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        // Seed the scopes used by the test (sensei_test is empty; production data
        // is seeded via staging.import_scopes — we replicate the two rows we need).
        sqlx_core::query::query(
            "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('organization', 'Organization', 20, true),
                    ('technology',   'Technology',   40, false)
             ON CONFLICT (key) DO UPDATE SET shareable = EXCLUDED.shareable")
            .execute(pg.pool()).await.unwrap();

        // organization is shareable; technology is not (seeded scopes ladder).
        let org_ns = pg.upsert_namespace("organization", "Test Org", "test-org-fed").await.unwrap();
        let tech_ns = pg.upsert_namespace("technology", "Rust", "rust-fed").await.unwrap();
        assert!(pg.namespace_is_shareable(&org_ns).await.unwrap());
        assert!(!pg.namespace_is_shareable(&tech_ns).await.unwrap());

        let src = pg.create_knowledge_source(&NewKnowledgeSource {
            kind: "hive_mind".into(), name: "H".into(), url: "u".into(), namespace_id: None,
            credential_ref: "c".into(), direction: "both".into() }).await.unwrap();
        let remote = uuid::Uuid::new_v4();
        let mem = pg.insert_memory(&InsertMemory {
            project_id: None, scope: "global".into(), scope_filter: None, mtype: "convention".into(),
            title: "t".into(), content: "c".into(), impact: None, tags: vec![], triage_signal: None,
            status: "active".into(), namespace_id: Some(org_ns), enforcement: Some("recommended".into()),
            origin: Some("federated".into()), source_id: Some(src) }).await.unwrap();
        pg.upsert_federated_memory(&src, &remote, "hash1", Some(&mem), 5).await.unwrap();
        pg.upsert_federated_memory(&src, &remote, "hash1", Some(&mem), 9).await.unwrap(); // idempotent
        let link = pg.find_federated_memory(&src, &remote).await.unwrap().unwrap();
        assert_eq!(link.memory_id, Some(mem));
        assert_eq!(link.remote_seq, 9);

        // push payload: returns snapshot + namespace identity (incl. name) + origin/scope_key
        let payload = pg.memory_push_payload(&mem).await.unwrap().unwrap();
        assert_eq!(payload.scope_key, "organization");
        assert_eq!(payload.slug, "test-org-fed");
        assert_eq!(payload.name, "Test Org");
        assert_eq!(payload.origin, "federated");

        // archive retires a federated memory (drops out of resolution)
        assert!(pg.archive_federated_memory(&mem).await.unwrap());
        let (status,): (String,) = sqlx_core::query_as::query_as("SELECT status::text FROM sensei.memories WHERE id=$1")
            .bind(mem).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(status, "archived");

        pg.delete_knowledge_source(&src).await.unwrap(); // cascades the ledger row
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id=$1").bind(mem).execute(pg.pool()).await.unwrap();
        // clean up namespaces and seeded scopes
        sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE id = ANY($1::uuid[])")
            .bind(vec![org_ns, tech_ns]).execute(pg.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.scopes WHERE key IN ('organization','technology')")
            .execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn latest_hook_event_ts_returns_max_for_family() {
        let pg = PgStore::connect_test().await.unwrap();
        let base = 1_900_000_000_000_i64; // far-future, won't collide with seeded data
        for (i, off) in [0_i64, 5000, 2000].iter().enumerate() {
            pg.insert_hook_event(
                &format!("sess-test-{i}"), "claude", "PreToolUse", Some("Bash"),
                Some("/tmp"), base + off, Some(true), &serde_json::json!({"t": i}),
            ).await.unwrap();
        }
        let max = pg.latest_hook_event_ts("claude").await.unwrap().unwrap();
        assert!(max >= base + 5000, "expected >= {} got {max}", base + 5000);
    }
}
