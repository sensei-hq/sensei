use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    /// Replace the set of discovered commands for a folder. Delete + insert
    /// in one transaction so a fresh scan atomically supersedes whatever
    /// was there before. Returns the number of rows inserted.
    pub async fn replace_folder_commands(
        &self,
        folder_id: &uuid::Uuid,
        ecosystem: &str,
        source_file: Option<&str>,
        commands: &[(String, String, Option<&str>)],
    ) -> Result<usize, String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        sqlx_core::query::query(
            "DELETE FROM sensei.project_commands WHERE folder_id = $1 AND ecosystem = $2"
        ).bind(folder_id).bind(ecosystem).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        for (raw_name, command_line, category) in commands {
            sqlx_core::query::query(
                "INSERT INTO sensei.project_commands
                    (folder_id, raw_name, command_line, category, ecosystem, source_file)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (folder_id, raw_name) DO UPDATE SET
                    command_line  = EXCLUDED.command_line,
                    category      = EXCLUDED.category,
                    ecosystem     = EXCLUDED.ecosystem,
                    source_file   = EXCLUDED.source_file,
                    discovered_at = now()"
            )
            .bind(folder_id).bind(raw_name).bind(command_line).bind(category).bind(ecosystem).bind(source_file)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(commands.len())
    }

    /// All commands for a project — union across its folders. `category`
    /// filter is applied server-side so callers can ask for just `test` or
    /// `build` without pulling everything. Ordered by category (nulls last)
    /// then raw_name for stable UI display.
    pub async fn get_project_commands(
        &self,
        project_id: &uuid::Uuid,
        category: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(i64, uuid::Uuid, String, String, String, Option<String>, String, Option<String>, chrono::DateTime<chrono::Utc>)> =
            if let Some(cat) = category {
                sqlx_core::query_as::query_as(
                    "SELECT c.id, c.folder_id, f.name, c.raw_name, c.command_line, c.category, c.ecosystem, c.source_file, c.discovered_at
                       FROM sensei.project_commands c
                       JOIN sensei.folders f ON f.id = c.folder_id
                      WHERE f.project_id = $1 AND c.category = $2
                      ORDER BY c.category NULLS LAST, c.raw_name"
                ).bind(project_id).bind(cat).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
            } else {
                sqlx_core::query_as::query_as(
                    "SELECT c.id, c.folder_id, f.name, c.raw_name, c.command_line, c.category, c.ecosystem, c.source_file, c.discovered_at
                       FROM sensei.project_commands c
                       JOIN sensei.folders f ON f.id = c.folder_id
                      WHERE f.project_id = $1
                      ORDER BY c.category NULLS LAST, c.raw_name"
                ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
            };

        // G10 command bias: mark the user's preferred tool per capability.
        let prefs = self.command_preferences("user").await?;
        let mut out = rows.into_iter().map(|(id, folder_id, folder_name, raw_name, command_line, category, ecosystem, source_file, discovered_at)| {
            serde_json::json!({
                "id":            id,
                "folder_id":     folder_id,
                "folder_name":   folder_name,
                "raw_name":      raw_name,
                "command_line":  command_line,
                "category":      category,
                "ecosystem":     ecosystem,
                "source_file":   source_file,
                "discovered_at": discovered_at.to_rfc3339(),
                "preferred":     crate::adapters::manifest::command_matches_preference(
                                     category.as_deref(), &raw_name, &command_line, &prefs),
            })
        }).collect::<Vec<_>>();

        // G10: rank the preferred tool first within each category (stable), so a
        // caller that takes "the test command" gets the biased one. NULL category
        // sorts last (matching the SQL `NULLS LAST`).
        out.sort_by(|a, b| {
            let key = |v: &serde_json::Value| {
                let c = v["category"].as_str();
                (c.is_none(), c.unwrap_or("").to_string())
            };
            key(a).cmp(&key(b))
                .then_with(|| b["preferred"].as_bool().unwrap_or(false)
                              .cmp(&a["preferred"].as_bool().unwrap_or(false)))
                .then_with(|| a["raw_name"].as_str().unwrap_or("")
                              .cmp(b["raw_name"].as_str().unwrap_or("")))
        });
        Ok(out)
    }

    /// User/dojo capability→preferred-tool preferences for a scope, as a
    /// capability→token map. Backs the `get_commands` bias (G10). Fail-open: an
    /// error yields an empty map (no bias) rather than failing the command read.
    pub async fn command_preferences(&self, scope: &str) -> Result<std::collections::HashMap<String, String>, String> {
        // Fail closed: a DB error must not read as an empty preference map — that
        // would silently ignore the user's real tool bias and fall back to
        // defaults (a governance fail-open). See the #109 audit.
        let rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT capability, preferred FROM sensei.dojo_preferences WHERE scope = $1",
        )
        .bind(scope)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("command_preferences: {e}"))?;
        Ok(rows.into_iter().collect())
    }

    /// Upsert a capability→preferred-tool bias for a scope (`user` today; a Dōjō
    /// can later set org/team scopes that override it). One row per (scope,
    /// capability).
    pub async fn upsert_command_preference(
        &self, scope: &str, capability: &str, preferred: &str, note: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.dojo_preferences (scope, capability, preferred, note, updated_at)
             VALUES ($1, $2, $3, $4, now())
             ON CONFLICT (scope, capability) DO UPDATE
               SET preferred = EXCLUDED.preferred, note = EXCLUDED.note, updated_at = now()",
        )
        .bind(scope).bind(capability).bind(preferred).bind(note)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("upsert_command_preference: {e}"))?;
        Ok(())
    }

    // ── #84 Track 2 Slice C — Replay tab session timeline ─────────────────

    /// The command line a repo runs for a canonical command verb (`lint` | `test`
    /// | `build` | …), from the manifest-discovered `project_commands`. `None`
    /// when the repo has no command in that category. Used to map a checker rule's
    /// `checker_ref` to a runnable command.
    pub async fn project_command_for(
        &self,
        folder_id: &uuid::Uuid,
        category: &str,
    ) -> Result<Option<String>, String> {
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT command_line FROM sensei.project_commands
              WHERE folder_id = $1 AND category = $2
              ORDER BY discovered_at DESC LIMIT 1",
        )
        .bind(folder_id)
        .bind(category)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(c,)| c))
    }

}
