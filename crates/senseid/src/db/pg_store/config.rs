use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
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

    // ── Scan exclusions (per watch root) ──────────────────────────────
    // Exclusions live in `folders_to_watch.excluded` — a jsonb array of relative
    // folder names/paths per root (the DDL design). `~/Developer` with
    // `excluded=["Code"]` excludes `~/Developer/Code`. An entry that is a bare
    // name matches that segment anywhere under the root; the absolute-prefix form
    // (root/entry) is precise. Adding an entry prunes the matching subtree;
    // removing one triggers a re-scan (see `update_watch_root` handler).

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

    // ── Collective sharing preferences (single row) ───────────────────
    //
    // One logical setting for the one local user: the table holds exactly one row
    // guarded by the `singleton` boolean PK (see collective_preferences.ddl).
    // Enum validation lives in `crate::collective::preferences` — these methods
    // only read/upsert.

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

    // ── Runs (relay engine run-state — activity.runs / activity.run_events) ─
    //
    // Durable state of an autonomous multi-phase run + its append-only cadence
    // log. `status` is `sensei.run_status` (bound via
    // `RelayRunStatus::as_db_str` cast `$N::sensei.run_status`, mirroring the
    // `$N::sensei.assistant_family` cast on `insert_assistant_event`). `kind` is
    // `sensei.run_event_kind`, bound the same way. Timestamps come back as
    // RFC-3339 `::text` (like `DojoMembership.last_heartbeat_at`). See
    // `crate::runs` for the row types.

}
