use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    pub async fn create_extension(
        &self,
        kind: &str,
        name: &str,
        description: Option<&str>,
        content: Option<&str>,
        scope: &str,
        source: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.extensions(kind, name, description, content, scope, source)
             VALUES($1::sensei.extension_kind, $2, $3, $4, $5::sensei.extension_scope, $6::sensei.extension_source) RETURNING id"
        ).bind(kind).bind(name).bind(description).bind(content).bind(scope).bind(source)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn update_extension(
        &self,
        id: &uuid::Uuid,
        description: Option<&str>,
        content: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.extensions SET description = COALESCE($2, description), content = COALESCE($3, content) WHERE id = $1"
        ).bind(id).bind(description).bind(content)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn list_extensions_by_kind(
        &self,
        kind: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, String, String, bool)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, description, scope::text, source::text, enabled FROM sensei.extensions WHERE kind = $1::sensei.extension_kind ORDER BY name"
        ).bind(kind).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, desc, scope, source, enabled)| {
            serde_json::json!({ "id": id, "kind": kind, "name": name, "description": desc, "scope": scope, "source": source, "enabled": enabled })
        }).collect())
    }

    pub async fn get_extension_history(
        &self,
        extension_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, i32, String, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT id, operation::text, revision, name, changed_at FROM history.past_extensions WHERE extension_id = $1 ORDER BY changed_at DESC"
        ).bind(extension_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, op, rev, name, ts)| {
            serde_json::json!({ "id": id, "operation": op, "revision": rev, "name": name, "changed_at": ts.to_rfc3339() })
        }).collect())
    }

    pub async fn delete_extension(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.extensions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Folders ──────────────────────────────────────────────────────

    pub async fn get_project_extensions(
        &self,
        project_id: &uuid::Uuid,
        kind_filter: Option<&[&str]>,
    ) -> Result<Vec<serde_json::Value>, String> {
        // Query the resolved view directly — it already joins extensions internally
        let rows: Vec<(uuid::Uuid, String, String, bool, serde_json::Value, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, kind::text, enabled, project_props, scope
                 FROM sensei.project_extensions_resolved
                 WHERE (scoped_project_id = $1 OR scoped_project_id IS NULL)
                   AND enabled = true
                 ORDER BY scope DESC, name",
            )
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .filter(|(_, _, kind, _, _, _)| kind_filter.is_none_or(|f| f.contains(&kind.as_str())))
            .map(|(id, name, kind, enabled, props, scope)| {
                serde_json::json!({
                    "id": id, "name": name, "kind": kind,
                    "enabled": enabled, "project_props": props, "scope": scope,
                })
            })
            .collect())
    }
}
