use sqlx_postgres::PgPool;

/// PostgreSQL store.
/// Schema is managed by `dbd apply`, not by this code.
pub struct PgStore {
    pool: PgPool,
}

#[allow(dead_code)] // PgStore API surface — methods wired up incrementally
impl PgStore {
    /// Connect to a PostgreSQL database.
    pub async fn connect(database_url: &str) -> Result<Self, String> {
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| format!("PgStore connect: {}", e))?;
        Ok(Self { pool })
    }

    /// Connect to the test database. Uses TEST_DATABASE_URL or defaults to local sensei.
    pub async fn connect_test() -> Result<Self, String> {
        let url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost:5432/sensei".to_string());
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
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, name, file_path, signature, line_start FROM sensei.nodes
             WHERE folder_id = $1 AND kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
             AND (name ILIKE '%' || $2 || '%' OR signature ILIKE '%' || $2 || '%')
             ORDER BY name LIMIT 50"
        ).bind(folder_id).bind(query).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, fp, sig, line)| {
            serde_json::json!({ "id": id, "name": name, "file_path": fp, "signature": sig, "line_start": line })
        }).collect())
    }

    pub async fn search_types(&self, folder_id: &uuid::Uuid, query: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, name, file_path, line_start FROM sensei.nodes
             WHERE folder_id = $1 AND kind IN ('class'::sensei.node_kind, 'struct'::sensei.node_kind, 'interface'::sensei.node_kind, 'enum'::sensei.node_kind, 'type'::sensei.node_kind)
             AND name ILIKE '%' || $2 || '%'
             ORDER BY name LIMIT 50"
        ).bind(folder_id).bind(query).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, fp, line)| {
            serde_json::json!({ "id": id, "name": name, "file_path": fp, "line_start": line })
        }).collect())
    }

    pub async fn count_nodes_by_kind(&self, folder_id: &uuid::Uuid) -> Result<std::collections::HashMap<String, i64>, String> {
        let rows: Vec<(String, i64)> = sqlx_core::query_as::query_as(
            "SELECT kind::text, COUNT(*) FROM sensei.nodes WHERE folder_id = $1 GROUP BY kind"
        ).bind(folder_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().collect())
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

    /// Get a repo (folder with kind='git'/'subtree') by abs_path.
    pub async fn get_repo_by_path(&self, abs_path: &str) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, String, Option<uuid::Uuid>, serde_json::Value, Vec<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, kind::text, name, abs_path, project_id, props, tags, modified_at FROM sensei.folders WHERE abs_path = $1"
            ).bind(abs_path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id, kind, name, abs, pid, props, tags, modified)| {
            serde_json::json!({
                "id": id, "kind": kind, "name": name, "abs_path": abs,
                "project_id": pid, "props": props, "tags": tags,
                "modified_at": modified.to_rfc3339(),
            })
        }))
    }

    /// Get a repo by name (for backward compat with repo_id lookups).
    pub async fn get_repo_by_name(&self, name: &str) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, Option<uuid::Uuid>, serde_json::Value, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, abs_path, project_id, props, modified_at FROM sensei.folders WHERE name = $1 AND kind IN ('git'::sensei.folder_kind, 'subtree'::sensei.folder_kind) LIMIT 1"
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

    /// Mark a folder as indexed with detected libs.
    pub async fn mark_folder_indexed(&self, folder_id: &uuid::Uuid, libs: &[String]) -> Result<(), String> {
        let props = serde_json::json!({"indexed_at": chrono::Utc::now().to_rfc3339(), "libs": libs});
        self.set_folder_props(folder_id, &props).await
    }

    /// Delete a folder (cascade deletes nodes, edges, scan_state, etc.).
    pub async fn delete_repo_by_name(&self, name: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "DELETE FROM sensei.folders WHERE name = $1 AND kind IN ('git'::sensei.folder_kind, 'subtree'::sensei.folder_kind)"
        ).bind(name).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Nodes ─────────────────────────────────────────────────────────

    pub async fn upsert_node(
        &self, folder_id: &uuid::Uuid, kind: &str, name: &str, file_path: &str,
        parent_id: Option<&uuid::Uuid>, signature: Option<&str>,
        line_start: Option<i32>, line_end: Option<i32>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.nodes(folder_id, kind, name, file_path, parent_id, signature, line_start, line_end)
             VALUES($1, $2::sensei.node_kind, $3, $4, $5, $6, $7, $8)
             ON CONFLICT DO NOTHING RETURNING id"
        ).bind(folder_id).bind(kind).bind(name).bind(file_path)
            .bind(parent_id).bind(signature).bind(line_start).bind(line_end)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn get_nodes_by_folder(&self, folder_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<uuid::Uuid>, Option<i32>, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, file_path, parent_id, line_start, line_end FROM sensei.nodes WHERE folder_id = $1 ORDER BY file_path, line_start"
        ).bind(folder_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, fp, pid, ls, le)| {
            serde_json::json!({ "id": id, "kind": kind, "name": name, "file_path": fp, "parent_id": pid, "line_start": ls, "line_end": le })
        }).collect())
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

    pub async fn get_edges_by_kind(&self, folder_id: &uuid::Uuid, kind: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, Option<uuid::Uuid>, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, source_id, target_id, target_name FROM sensei.edges WHERE folder_id = $1 AND kind = $2::sensei.edge_kind"
        ).bind(folder_id).bind(kind).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, src, tgt, name)| {
            serde_json::json!({ "id": id, "source_id": src, "target_id": tgt, "target_name": name })
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
            "SELECT id, name, abs_path, kind::text FROM sensei.folders WHERE kind IN ('git'::sensei.folder_kind, 'subtree'::sensei.folder_kind) ORDER BY name"
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

    pub async fn delete_library(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.libraries WHERE id = $1")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
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

    // ── Projects ──────────────────────────────────────────────────────

    pub async fn create_project(&self, name: &str, description: Option<&str>, client: Option<&str>) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.projects(name, description, client) VALUES($1, $2, $3) RETURNING id"
        ).bind(name).bind(description).bind(client)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx_core::query_as::query_as;

    fn test_db_url() -> String {
        std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost:5432/sensei".to_string())
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
}
