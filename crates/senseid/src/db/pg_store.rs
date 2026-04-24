use sqlx_postgres::PgPool;

/// PostgreSQL store — replaces SQLite Store during migration.
/// Schema is managed by `dbd apply`, not by this code.
/// Callers will be wired in as entities migrate (issues #101–#111).
#[allow(dead_code)]
pub struct PgStore {
    pool: PgPool,
}

#[allow(dead_code)]
impl PgStore {
    /// Connect to a PostgreSQL database.
    pub async fn connect(database_url: &str) -> Result<Self, String> {
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| format!("PgStore connect: {}", e))?;
        Ok(Self { pool })
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

    /// Execute a raw SQL statement (for one-off queries like scanned_roots).
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
