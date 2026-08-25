use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    /// List every installed service, joined with the given project's per-scope
    /// override so the UI can render enabled/disabled state without a second
    /// round-trip. `enabled_for_project` reads from the scoped override when
    /// present, otherwise falls back to the global row's `enabled`, otherwise
    /// defaults to `true` (installed services are on by default).
    pub async fn list_services_with_project_scope(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            uuid::Uuid,     // id
            String,         // name
            String,         // display_name
            Option<String>, // publisher
            String,         // protocol
            String,         // kind
            Option<String>, // summary
            i32,            // tools_count
            bool,           // verified
            bool,           // installed
            Option<bool>,   // scoped_enabled
            Option<bool>,   // global_enabled
        )> = sqlx_core::query_as::query_as(
            "SELECT s.id, s.name, s.display_name, s.publisher,
                    s.protocol::text, s.kind::text, s.summary, s.tools_count,
                    s.verified, s.installed,
                    (SELECT enabled FROM sensei.service_projects sp
                      WHERE sp.service_id = s.id AND sp.project_id = $1) AS scoped_enabled,
                    (SELECT enabled FROM sensei.service_projects sp
                      WHERE sp.service_id = s.id AND sp.project_id IS NULL) AS global_enabled
               FROM sensei.services s
              WHERE s.installed = true
              ORDER BY s.display_name",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    name,
                    display_name,
                    publisher,
                    protocol,
                    kind,
                    summary,
                    tools_count,
                    verified,
                    installed,
                    scoped_enabled,
                    global_enabled,
                )| {
                    // Effective enable: scoped override wins, then global row, then default true.
                    let enabled_for_project = scoped_enabled.or(global_enabled).unwrap_or(true);
                    serde_json::json!({
                        "id":                 id,
                        "name":               name,
                        "displayName":        display_name,
                        "publisher":          publisher,
                        "protocol":           protocol,
                        "kind":               kind,
                        "summary":            summary,
                        "toolsCount":         tools_count,
                        "verified":           verified,
                        "installed":          installed,
                        "enabledForProject":  enabled_for_project,
                        "scopedEnabled":      scoped_enabled,
                        "globalEnabled":      global_enabled,
                    })
                },
            )
            .collect())
    }

    /// Upsert the per-project scope row for a service. `project_id = None`
    /// writes the global scope. Idempotent — repeat calls flip the enabled
    /// flag and bump `modified_at`.
    pub async fn set_service_project_scope(
        &self,
        service_id: &uuid::Uuid,
        project_id: Option<&uuid::Uuid>,
        enabled: bool,
    ) -> Result<(), String> {
        // Partial-unique indexes on (service_id) WHERE project_id IS NULL and
        // (service_id, project_id) WHERE project_id IS NOT NULL guarantee at
        // most one row per scope, so an UPDATE-first fallback is enough
        // without needing INSERT ... ON CONFLICT (which can't target a
        // partial unique index without extra hints in Postgres).
        let updated = if let Some(pid) = project_id {
            sqlx_core::query::query(
                "UPDATE sensei.service_projects
                    SET enabled = $1, modified_at = now()
                  WHERE service_id = $2 AND project_id = $3",
            )
            .bind(enabled)
            .bind(service_id)
            .bind(pid)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            sqlx_core::query::query(
                "UPDATE sensei.service_projects
                    SET enabled = $1, modified_at = now()
                  WHERE service_id = $2 AND project_id IS NULL",
            )
            .bind(enabled)
            .bind(service_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        };

        if updated.rows_affected() == 0 {
            sqlx_core::query::query(
                "INSERT INTO sensei.service_projects (service_id, project_id, enabled)
                 VALUES ($1, $2, $3)",
            )
            .bind(service_id)
            .bind(project_id)
            .bind(enabled)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Per-tool aggregation for a project's Instruments screen.
    /// Joins `session_tool_calls` back to `activity.sessions` on
    /// `client_session_id`, filters by `session.project_id`, and computes
    /// call count, error count, avg duration, and FTR (fraction of sessions
    /// that used the tool AND completed FTR).
    pub async fn get_project_mcp_tool_stats(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, i64, i64, Option<f64>, Option<f64>, Option<chrono::DateTime<chrono::Utc>>)> =
            sqlx_core::query_as::query_as(
                "WITH scoped AS (
                     SELECT stc.tool_name,
                            stc.success,
                            stc.duration_ms,
                            stc.started_at,
                            s.ftr
                       FROM sensei.session_tool_calls stc
                       JOIN activity.sessions s
                         ON s.client_session_id = stc.session_id
                      WHERE s.project_id = $1
                 )
                 SELECT tool_name,
                        count(*)::bigint                                                          AS calls,
                        count(*) FILTER (WHERE success IS FALSE)::bigint                          AS errors,
                        avg(duration_ms)::float8                                                  AS avg_duration_ms,
                        (count(*) FILTER (WHERE ftr IS TRUE)::float8
                            / NULLIF(count(*) FILTER (WHERE ftr IS NOT NULL), 0))                 AS ftr,
                        max(started_at)                                                           AS last_used_at
                   FROM scoped
                  GROUP BY tool_name
                  ORDER BY calls DESC, tool_name ASC"
            )
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(|(tool_name, calls, errors, avg_dur, ftr, last_used_at)| {
                serde_json::json!({
                    "toolName":      tool_name,
                    "calls":         calls,
                    "errors":        errors,
                    "avgDurationMs": avg_dur,
                    "ftr":           ftr,
                    "lastUsedAt":    last_used_at.map(|t| t.to_rfc3339()),
                })
            })
            .collect())
    }

    // ── Manual impact-verdict log (T3 Slice 3) ─────────────────────────────

    pub async fn upsert_service(
        &self,
        name: &str,
        display_name: &str,
        kind: &str,
        protocol: &str,
        config: &serde_json::Value,
    ) -> Result<uuid::Uuid, String> {
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
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Snapshots (activity) ─────────────────────────────────────────

    pub async fn get_tool_usage_stats(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, i64, i64, Option<f64>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT tool_name, call_count, error_count, avg_duration_ms::float8, last_used_at
             FROM sensei.tool_usage_stats ORDER BY call_count DESC LIMIT 50",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(name, calls, errors, dur, last)| {
                serde_json::json!({ "tool_name": name, "call_count": calls, "error_count": errors,
                                "avg_duration_ms": dur, "last_used_at": last.to_rfc3339() })
            })
            .collect())
    }

    /// Read the cached tool manifest for a server. `None` when nothing has
    /// been probed yet.
    pub async fn get_mcp_tool_manifest(
        &self,
        server_id: &uuid::Uuid,
    ) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(
            uuid::Uuid,
            serde_json::Value,
            i32,
            chrono::DateTime<chrono::Utc>,
            i32,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = sqlx_core::query_as::query_as(
            "SELECT id, tools, tool_count, probed_at, ttl_seconds, error,
                        protocol_version, server_name, server_version
                   FROM sensei.mcp_tool_manifests
                  WHERE server_id = $1",
        )
        .bind(server_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(row.map(|(id, tools, tool_count, probed_at, ttl, error, pv, sn, sv)| {
            serde_json::json!({
                "id":                id,
                "server_id":         server_id,
                "tools":             tools,
                "tool_count":        tool_count,
                "probed_at":         probed_at.to_rfc3339(),
                "ttl_seconds":       ttl,
                "error":             error,
                "protocol_version":  pv,
                "server_name":       sn,
                "server_version":    sv,
                "age_seconds":       (chrono::Utc::now() - probed_at).num_seconds(),
            })
        }))
    }

    /// Upsert a probed manifest. Uses `server_id UNIQUE` on the table so a
    /// re-probe overwrites in place.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_mcp_tool_manifest(
        &self,
        server_id: &uuid::Uuid,
        tools: &serde_json::Value,
        tool_count: i32,
        protocol_version: Option<&str>,
        server_name: Option<&str>,
        server_version: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.mcp_tool_manifests
                (server_id, tools, tool_count, probed_at, protocol_version, server_name, server_version, error)
             VALUES ($1, $2, $3, now(), $4, $5, $6, $7)
             ON CONFLICT (server_id) DO UPDATE SET
                tools            = EXCLUDED.tools,
                tool_count       = EXCLUDED.tool_count,
                probed_at        = now(),
                protocol_version = EXCLUDED.protocol_version,
                server_name      = EXCLUDED.server_name,
                server_version   = EXCLUDED.server_version,
                error            = EXCLUDED.error"
        )
        .bind(server_id).bind(tools).bind(tool_count)
        .bind(protocol_version).bind(server_name).bind(server_version).bind(error)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Full server row for the probe orchestrator — command, args, env,
    /// enabled state — keyed by id.
    pub async fn get_mcp_server_by_id(
        &self,
        id: &uuid::Uuid,
    ) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, String, Option<uuid::Uuid>, String, String, serde_json::Value, serde_json::Value, bool, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, acp_family, mcp_key, scope, project_id, config_source, command, args, env, enabled, connection_state
                   FROM sensei.mcp_servers WHERE id = $1"
            )
            .bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(row.map(|(id, family, key, scope, pid, source, cmd, args, env, enabled, state)| {
            serde_json::json!({
                "id": id, "acp_family": family, "mcp_key": key, "scope": scope,
                "project_id": pid, "config_source": source, "command": cmd,
                "args": args, "env": env, "enabled": enabled, "connection_state": state,
            })
        }))
    }

    // ── #84 Track 2 Slice D — Health tab per-tool verdict split ───────────

    /// Upsert a discovered MCP server row (#84). The uniqueness key is
    /// `(acp_family, mcp_key, scope, project_id)`; existing rows have
    /// `command`/`args`/`env`/`config_source`/`last_seen_at` refreshed, but
    /// `enabled` is preserved (a user's manual toggle survives a re-scan).
    ///
    /// Args:
    /// - `acp_family`  — 'claude' | 'zed' | 'cursor' | 'codex' | 'opencode' | 'other'
    /// - `mcp_key`     — key in the ACP config, e.g. 'sensei', 'postgres'
    /// - `project_id`  — Some(uuid) for project-scope, None for user-scope
    /// - `config_source` — absolute path where discovered
    /// - `command`     — the mcp entry's `command`
    /// - `args`        — JSON array of args (from the config)
    /// - `env`         — JSON object of env vars (from the config)
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_mcp_server(
        &self,
        acp_family: &str,
        mcp_key: &str,
        project_id: Option<uuid::Uuid>,
        config_source: &str,
        command: &str,
        args: &serde_json::Value,
        env: &serde_json::Value,
    ) -> Result<uuid::Uuid, String> {
        let scope = if project_id.is_some() { "project" } else { "user" };
        // Partial unique indexes mean the ON CONFLICT target differs for
        // user vs project scope; the cleanest cross-cutting pattern is
        // "try INSERT, on failure UPDATE by lookup". Use a plain lookup +
        // conditional insert inside a transaction so a concurrent scan
        // can't race us into a duplicate.
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        let existing: Option<(uuid::Uuid,)> = if let Some(pid) = project_id {
            sqlx_core::query_as::query_as(
                "SELECT id FROM sensei.mcp_servers
                  WHERE acp_family = $1 AND mcp_key = $2
                    AND scope = 'project' AND project_id = $3",
            )
            .bind(acp_family)
            .bind(mcp_key)
            .bind(pid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?
        } else {
            sqlx_core::query_as::query_as(
                "SELECT id FROM sensei.mcp_servers
                  WHERE acp_family = $1 AND mcp_key = $2
                    AND scope = 'user'",
            )
            .bind(acp_family)
            .bind(mcp_key)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?
        };

        let id = if let Some((existing_id,)) = existing {
            sqlx_core::query::query(
                "UPDATE sensei.mcp_servers
                    SET config_source = $2,
                        command       = $3,
                        args          = $4,
                        env           = $5,
                        last_seen_at  = now()
                  WHERE id = $1",
            )
            .bind(existing_id)
            .bind(config_source)
            .bind(command)
            .bind(args)
            .bind(env)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            existing_id
        } else {
            let (new_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
                "INSERT INTO sensei.mcp_servers
                    (acp_family, mcp_key, scope, project_id, config_source, command, args, env)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 RETURNING id",
            )
            .bind(acp_family)
            .bind(mcp_key)
            .bind(scope)
            .bind(project_id)
            .bind(config_source)
            .bind(command)
            .bind(args)
            .bind(env)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            new_id
        };

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// List MCP servers. `project_id = None` returns user-scope rows; a
    /// concrete project returns the union of user-scope + that project's
    /// project-scope rows (the Instruments Playground shows both). Ordered
    /// by family, then key.
    pub async fn list_mcp_servers(
        &self,
        project_id: Option<uuid::Uuid>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(
            uuid::Uuid,
            String,
            String,
            String,
            Option<uuid::Uuid>,
            String,
            String,
            serde_json::Value,
            serde_json::Value,
            bool,
            String,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        )> = if let Some(pid) = project_id {
            sqlx_core::query_as::query_as(
                    "SELECT id, acp_family, mcp_key, scope, project_id, config_source, command, args, env, enabled, connection_state, last_error, last_seen_at, discovered_at
                       FROM sensei.mcp_servers
                      WHERE scope = 'user' OR project_id = $1
                      ORDER BY acp_family, mcp_key"
                ).bind(pid).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
        } else {
            sqlx_core::query_as::query_as(
                    "SELECT id, acp_family, mcp_key, scope, project_id, config_source, command, args, env, enabled, connection_state, last_error, last_seen_at, discovered_at
                       FROM sensei.mcp_servers
                      WHERE scope = 'user'
                      ORDER BY acp_family, mcp_key"
                ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
        };

        Ok(rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.0, "acp_family": r.1, "mcp_key": r.2, "scope": r.3,
                    "project_id": r.4, "config_source": r.5, "command": r.6,
                    "args": r.7, "env": r.8, "enabled": r.9,
                    "connection_state": r.10, "last_error": r.11,
                    "last_seen_at": r.12.to_rfc3339(),
                    "discovered_at": r.13.to_rfc3339(),
                })
            })
            .collect())
    }

    /// Toggle `enabled` for an MCP server. Returns the new state, or `None`
    /// if the id doesn't exist.
    pub async fn set_mcp_server_enabled(
        &self,
        id: &uuid::Uuid,
        enabled: bool,
    ) -> Result<Option<bool>, String> {
        let row: Option<(bool,)> = sqlx_core::query_as::query_as(
            "UPDATE sensei.mcp_servers
                SET enabled = $2,
                    connection_state = CASE WHEN $2 THEN connection_state ELSE 'disabled' END
              WHERE id = $1
          RETURNING enabled",
        )
        .bind(id)
        .bind(enabled)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(e,)| e))
    }

    /// Delete rows the current scan did NOT touch — servers that no longer
    /// appear in any ACP config. Compares against `not_seen_before` so a
    /// row scanned after the cutoff survives. Returns the number of rows
    /// pruned. Called at the end of `discover_mcp_servers`.
    pub async fn prune_stale_mcp_servers(
        &self,
        not_seen_before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, String> {
        let res = sqlx_core::query::query("DELETE FROM sensei.mcp_servers WHERE last_seen_at < $1")
            .bind(not_seen_before)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    // ── Unified tool inventory (assistant_tools) + Instruments · Health grid ──

    /// Wipe the inventory — the capture repopulates from scratch so tools that
    /// vanished from a source don't linger.
    pub async fn clear_assistant_tools(&self) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.assistant_tools")
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Upsert one registered tool (idempotent on the (family, source_type,
    /// source_key, tool_name) unique index).
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_assistant_tool(
        &self,
        assistant_family: &str,
        source_type: &str,
        source_key: &str,
        tool_name: &str,
        invoked_name: &str,
        description: Option<&str>,
        server_id: Option<uuid::Uuid>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.assistant_tools
                (assistant_family, source_type, source_key, tool_name, invoked_name, description, server_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT (assistant_family, source_type, source_key, tool_name)
             DO UPDATE SET invoked_name = EXCLUDED.invoked_name,
                           description  = EXCLUDED.description,
                           server_id    = EXCLUDED.server_id,
                           updated_at   = now()"
        ).bind(assistant_family).bind(source_type).bind(source_key)
         .bind(tool_name).bind(invoked_name).bind(description).bind(server_id)
         .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Distinct built-in (non-MCP) tool names observed in usage — the harness's
    /// built-in catalog (no canonical list exists, so observed usage IS the
    /// registry: Bash, Read, Edit, Task, Skill, …).
    pub async fn distinct_builtin_tool_names(&self) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT tool_name FROM sensei.tool_usage_stats
              WHERE tool_name NOT LIKE 'mcp__%' ORDER BY tool_name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    /// Usage-observed MCP prefixes → their bare tool names. Powers the bridge
    /// that maps a probed server to its harness usage key.
    pub async fn usage_mcp_prefix_tools(
        &self,
    ) -> Result<std::collections::HashMap<String, std::collections::HashSet<String>>, String> {
        let rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT split_part(tool_name,'__',2) AS prefix,
                    split_part(tool_name,'__',3) AS bare
               FROM sensei.tool_usage_stats
              WHERE tool_name LIKE 'mcp__%'",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let mut map: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        for (p, b) in rows {
            map.entry(p).or_default().insert(b);
        }
        Ok(map)
    }

    /// Set an MCP server's connection state (after a probe attempt).
    pub async fn set_mcp_server_connection_state(
        &self,
        id: &uuid::Uuid,
        state: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.mcp_servers SET connection_state = $2, last_seen_at = now() WHERE id = $1"
        ).bind(id).bind(state).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The Instruments · Health L1 grid — one row per tool source. Unions the
    /// inventory (registered known) with usage-observed MCP sources not yet in
    /// the inventory (registered unknown → null share, never a fabricated bar).
    pub async fn get_tools_health(&self) -> Result<Vec<serde_json::Value>, String> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(String, String, String, Option<i64>, Option<uuid::Uuid>, i64, i64, Option<String>)> =
            sqlx_core::query_as::query_as(
            // `evt` is the real 14-day usage window: one row per tool with its
            // PostToolUse count over the last 14 days. `assistant_events.ts` is
            // epoch MILLIS (bigint), so the cutoff is computed in millis too.
            // tool_usage_stats is an all-time view — never use it for the window.
            "WITH evt AS (
               SELECT h.tool_name AS tool_name, count(*)::bigint AS calls_14d
                 FROM activity.assistant_events h
                WHERE h.event_type = 'PostToolUse' AND h.tool_name IS NOT NULL
                  AND h.ts >= (extract(epoch from now() - interval '14 days') * 1000)::bigint
                GROUP BY h.tool_name ),
             reg AS (
               SELECT assistant_family, source_type, source_key,
                      count(*)::bigint AS registered,
                      (array_agg(server_id) FILTER (WHERE server_id IS NOT NULL))[1] AS server_id
                 FROM sensei.assistant_tools
                GROUP BY assistant_family, source_type, source_key ),
             inv AS (
               SELECT at.assistant_family, at.source_type, at.source_key,
                      count(DISTINCT e.tool_name)::bigint AS invoked_14d,
                      coalesce(sum(e.calls_14d),0)::bigint AS calls_14d
                 FROM sensei.assistant_tools at
                 JOIN evt e ON e.tool_name = at.invoked_name
                GROUP BY at.assistant_family, at.source_type, at.source_key ),
             uncovered AS (
               SELECT 'claude'::text AS assistant_family, 'mcp'::text AS source_type,
                      split_part(e.tool_name,'__',2) AS source_key,
                      count(DISTINCT e.tool_name)::bigint AS invoked_14d,
                      coalesce(sum(e.calls_14d),0)::bigint AS calls_14d
                 FROM evt e
                WHERE e.tool_name LIKE 'mcp__%'
                  AND NOT EXISTS (SELECT 1 FROM sensei.assistant_tools at WHERE at.invoked_name = e.tool_name)
                GROUP BY split_part(e.tool_name,'__',2) )
             SELECT r.assistant_family, r.source_type, r.source_key,
                    r.registered, r.server_id,
                    coalesce(i.invoked_14d,0)::bigint, coalesce(i.calls_14d,0)::bigint,
                    s.connection_state
               FROM reg r
               LEFT JOIN inv i ON i.assistant_family=r.assistant_family
                              AND i.source_type=r.source_type AND i.source_key=r.source_key
               LEFT JOIN sensei.mcp_servers s ON s.id = r.server_id
             UNION ALL
             SELECT assistant_family, source_type, source_key,
                    NULL::bigint, NULL::uuid, invoked_14d, calls_14d, NULL::text
               FROM uncovered
             ORDER BY 7 DESC"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(|(family, stype, skey, registered, server_id, invoked, calls, conn)| {
                let connected = match stype.as_str() {
                    "builtin" => true,
                    _ => conn.as_deref() == Some("connected"),
                };
                let share = registered.filter(|r| *r > 0).map(|r| invoked as f64 / r as f64);
                serde_json::json!({
                    "assistant_family": family,
                    "source_type": stype,
                    "source_key": skey,
                    "name": crate::tool_discovery::pretty_source_name(&stype, &skey),
                    "connected": connected,
                    "connection_state": conn,
                    "server_id": server_id,
                    "tools_registered": registered,
                    "tools_invoked_14d": invoked,
                    "calls_14d": calls,
                    "share_invoked": share,
                })
            })
            .collect())
    }

    /// Return every active chain with its ordered model list. The wizard
    /// reads this to build the per-role picker; the settings page reuses
    /// it for the "which chain serves which role" table.
    pub async fn list_chains_with_models(&self) -> Result<Vec<serde_json::Value>, String> {
        // One round trip: chain metadata + JSON-aggregated members ordered
        // by sequence_order. Sqlx decodes the aggregate directly; the null
        // JSON coalesce keeps chains with no models rendering as `[]`
        // instead of the row disappearing.
        type ChainRow = (
            uuid::Uuid,
            String,
            String,
            Option<String>,
            Option<String>,
            i32,
            bool,
            serde_json::Value,
        );
        let rows: Vec<ChainRow> = sqlx_core::query_as::query_as(
            "SELECT fc.id,
                    fc.name,
                    fc.capability::text,
                    fc.role::text,
                    fc.description,
                    fc.max_fallback_attempts,
                    fc.is_active,
                    COALESCE(
                        (SELECT jsonb_agg(
                                    jsonb_build_object(
                                        'memberId',      fcm.id,
                                        'sequenceOrder', fcm.sequence_order,
                                        'modelName',     m.name,
                                        'routerName',    r.id::text
                                    ) ORDER BY fcm.sequence_order
                                )
                           FROM gateway.fallback_chain_models fcm
                           JOIN gateway.routers r ON r.id = fcm.router_id
                           JOIN gateway.models  m ON m.id = fcm.model_id
                          WHERE fcm.chain_id = fc.id AND fcm.is_active),
                        '[]'::jsonb) AS models
               FROM gateway.fallback_chains fc
              WHERE fc.is_active
              ORDER BY fc.sequence, fc.name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(|(id, name, capability, role, description, max_attempts, is_active, models)| {
                serde_json::json!({
                    "id":                 id,
                    "name":               name,
                    "capability":         capability,
                    "role":               role,
                    "description":        description,
                    "maxFallbackAttempts": max_attempts,
                    "isActive":           is_active,
                    "models":             models,
                })
            })
            .collect())
    }

    /// Assign (or clear) the sensei inference role a chain serves. The
    /// `role` column carries a unique-when-set index — writing a role
    /// that another chain already owns returns a database error the
    /// caller can map to a 409. Pass `None` to unassign.
    pub async fn set_chain_role(
        &self,
        chain_id: &uuid::Uuid,
        role: Option<&str>,
    ) -> Result<(), String> {
        // Cast at bind time so `None` writes SQL NULL, not the empty
        // string. `modified_at` updates so downstream diff-based reads
        // see the change.
        let result = sqlx_core::query::query(
            "UPDATE gateway.fallback_chains
                SET role = $2::sensei.inference_role,
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(chain_id)
        .bind(role)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        if result.rows_affected() == 0 {
            return Err("chain not found".into());
        }
        Ok(())
    }

    // ── Chain member editing (add / remove / move) ───────────────────
    //
    // Members of a chain are rows in `gateway.fallback_chain_models`,
    // ordered by `sequence_order`. The (chain_id, sequence_order) pair
    // is unique — so writes must maintain contiguous ordering, and
    // moves happen through temporary shifts to dodge the constraint.

    /// List the models that a chain COULD use — everything with a
    /// matching capability, in any router, minus the models already
    /// present in the chain. Each row carries the model + its router
    /// so the picker can render provider chips per the mockup.
    pub async fn list_available_models_for_chain(
        &self,
        chain_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, uuid::Uuid, String)> =
            sqlx_core::query_as::query_as(
                "SELECT m.id, m.name, m.full_name, r.id, r.name
               FROM gateway.models m
               JOIN gateway.models_in_router mir ON mir.model_id = m.id
               JOIN gateway.routers r ON r.id = mir.router_id
              WHERE m.capabilities @> ARRAY[(
                  SELECT fc.capability FROM gateway.fallback_chains fc WHERE fc.id = $1
              )]::sensei.model_capability[]
                AND NOT EXISTS (
                    SELECT 1 FROM gateway.fallback_chain_models fcm
                     WHERE fcm.chain_id = $1
                       AND fcm.model_id = m.id
                       AND fcm.router_id = r.id
                )
              ORDER BY r.name, m.full_name",
            )
            .bind(chain_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(mid, name, full, rid, rname)| {
                serde_json::json!({
                    "modelId":    mid,
                    "modelName":  name,
                    "fullName":   full,
                    "routerId":   rid,
                    "routerName": rname,
                })
            })
            .collect())
    }

    /// Append a model to the end of a chain's ordered list. Returns the
    /// new row id and the assigned sequence_order so the caller can
    /// update its optimistic UI. Fails with a helpful message when the
    /// (model_id, router_id) pair isn't reachable via `models_in_router`.
    pub async fn add_chain_model(
        &self,
        chain_id: &uuid::Uuid,
        model_id: &uuid::Uuid,
        router_id: &uuid::Uuid,
    ) -> Result<(uuid::Uuid, i32), String> {
        // Guard: chain must exist.
        let (chain_exists,): (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM gateway.fallback_chains WHERE id = $1)",
        )
        .bind(chain_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        if !chain_exists {
            return Err("chain not found".into());
        }

        // Guard: the (model_id, router_id) pair must be reachable via
        // models_in_router. This is what the FK check would tell us,
        // but a clearer message helps the wizard render a useful error.
        let (reachable,): (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(
                SELECT 1 FROM gateway.models_in_router
                 WHERE model_id = $1 AND router_id = $2
             )",
        )
        .bind(model_id)
        .bind(router_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        if !reachable {
            return Err("model is not reachable via this router".into());
        }

        // Next sequence_order = max + 1 (or 1 for an empty chain). The
        // unique(chain_id, sequence_order) index catches any race; on a
        // conflict we surface as-is.
        let (row_id, seq): (uuid::Uuid, i32) = sqlx_core::query_as::query_as(
            "INSERT INTO gateway.fallback_chain_models (chain_id, router_id, model_id, sequence_order)
             SELECT $1, $2, $3, COALESCE(MAX(sequence_order), 0) + 1
               FROM gateway.fallback_chain_models
              WHERE chain_id = $1
             RETURNING id, sequence_order"
        )
        .bind(chain_id).bind(router_id).bind(model_id)
        .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;

        Ok((row_id, seq))
    }

    /// Remove a chain-model row by id and compact the sequence so the
    /// remaining rows stay contiguous (1, 2, 3, …). Fails if the row
    /// isn't in the given chain — surfaces as 404 upstream.
    pub async fn remove_chain_model(
        &self,
        chain_id: &uuid::Uuid,
        member_id: &uuid::Uuid,
    ) -> Result<(), String> {
        // Two-step in a single transaction so the compaction sees the
        // deletion. The unique(chain_id, sequence_order) constraint
        // enforces the contiguous invariant.
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        let (removed_seq,): (Option<i32>,) = sqlx_core::query_as::query_as(
            "DELETE FROM gateway.fallback_chain_models
              WHERE id = $1 AND chain_id = $2
              RETURNING (sequence_order)::int",
        )
        .bind(member_id)
        .bind(chain_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?
        .map(|(s,)| (Some(s),))
        .unwrap_or((None,));

        let Some(seq) = removed_seq else {
            return Err("chain member not found".into());
        };

        // Compact: shift everyone above the removed slot down by one.
        // The unique index would collide if we did a single-step
        // decrement, so we bump the shifted rows to a negative range
        // first, then normalise.
        sqlx_core::query::query(
            "UPDATE gateway.fallback_chain_models
                SET sequence_order = -sequence_order
              WHERE chain_id = $1 AND sequence_order > $2",
        )
        .bind(chain_id)
        .bind(seq)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        sqlx_core::query::query(
            "UPDATE gateway.fallback_chain_models
                SET sequence_order = -sequence_order - 1
              WHERE chain_id = $1 AND sequence_order < 0",
        )
        .bind(chain_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Swap a chain-model with its neighbour above (direction = -1) or
    /// below (direction = +1). No-op at boundaries. Returns Ok(false)
    /// when no swap happened so the caller can distinguish "hit
    /// boundary" from "wrote".
    pub async fn move_chain_model(
        &self,
        chain_id: &uuid::Uuid,
        member_id: &uuid::Uuid,
        direction: i32,
    ) -> Result<bool, String> {
        if direction != -1 && direction != 1 {
            return Err("direction must be -1 (up) or +1 (down)".into());
        }

        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        // Find the current sequence_order (also confirms membership).
        let cur: Option<(i32,)> = sqlx_core::query_as::query_as(
            "SELECT sequence_order FROM gateway.fallback_chain_models
              WHERE id = $1 AND chain_id = $2",
        )
        .bind(member_id)
        .bind(chain_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        let Some((cur_seq,)) = cur else {
            return Err("chain member not found".into());
        };

        let target_seq = cur_seq + direction;
        if target_seq < 1 {
            return Ok(false); // Already at top.
        }

        // Locate the neighbour to swap with. If none exists at target
        // (member is last row), also a boundary.
        let neighbour: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM gateway.fallback_chain_models
              WHERE chain_id = $1 AND sequence_order = $2",
        )
        .bind(chain_id)
        .bind(target_seq)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        let Some((neighbour_id,)) = neighbour else {
            return Ok(false); // Already at bottom.
        };

        // Three-step swap to dodge the unique(chain_id, sequence_order)
        // index: park the mover at a negative slot, move the neighbour
        // into the mover's old slot, then land the mover.
        sqlx_core::query::query(
            "UPDATE gateway.fallback_chain_models
                SET sequence_order = -$1
              WHERE id = $2",
        )
        .bind(cur_seq)
        .bind(member_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        sqlx_core::query::query(
            "UPDATE gateway.fallback_chain_models
                SET sequence_order = $1
              WHERE id = $2",
        )
        .bind(cur_seq)
        .bind(neighbour_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        sqlx_core::query::query(
            "UPDATE gateway.fallback_chain_models
                SET sequence_order = $1
              WHERE id = $2",
        )
        .bind(target_seq)
        .bind(member_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(true)
    }

    // ── Front-door intake: playbooks / rules / guide / runs ────────────
}
