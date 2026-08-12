use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
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

    /// Documentation pages for a library by name, optionally filtered to a
    /// single component. `component=None` returns every page (the handler
    /// builds the index/overview from these); `Some(c)` returns just that
    /// component's page(s). NULL-component pages (the library overview) sort
    /// first. This is what `get_lib_docs` reads — it must return the page
    /// CONTENT, not just library metadata.
    pub async fn get_library_pages(
        &self, name: &str, component: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT lp.title, lp.component, lp.description, lp.content,
                        COALESCE(lp.url, lp.local_path) AS location, lp.source_type::text
                   FROM sensei.library_pages lp
                   JOIN sensei.libraries l ON l.id = lp.library_id
                  WHERE l.name = $1
                    AND ($2::text IS NULL OR lp.component = $2)
                  ORDER BY (lp.component IS NULL) DESC, lp.component, lp.title"
            )
            .bind(name).bind(component)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(title, component, description, content, location, source_type)| {
            serde_json::json!({
                "title": title, "component": component,
                "description": description, "content": content,
                "location": location, "source": source_type,
            })
        }).collect())
    }

    /// Search library pages by title / component / content (ILIKE). Returns
    /// ranked matches with a short snippet rather than full content, so
    /// `search_lib_docs` is concise. Title/component hits rank above body hits.
    pub async fn search_library_pages(&self, query: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, Option<String>, Option<String>, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT l.name, lp.title, lp.component, lp.description,
                        left(lp.content, 400) AS snippet
                   FROM sensei.library_pages lp
                   JOIN sensei.libraries l ON l.id = lp.library_id
                  WHERE lp.title ILIKE '%' || $1 || '%'
                     OR lp.component ILIKE '%' || $1 || '%'
                     OR lp.content ILIKE '%' || $1 || '%'
                  ORDER BY (lp.title ILIKE '%' || $1 || '%') DESC,
                           (lp.component ILIKE '%' || $1 || '%') DESC,
                           l.name, lp.component
                  LIMIT 30"
            )
            .bind(query)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(library, title, component, description, snippet)| {
            serde_json::json!({
                "library": library, "title": title, "component": component,
                "description": description, "snippet": snippet,
            })
        }).collect())
    }

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

    /// Refresh a library's source pointer (`source_type` + `base_url`) BY id — for
    /// a re-index that resolves the row via its uuid rather than by
    /// `(ecosystem, name)`. NEVER changes `ecosystem`: that is half the
    /// `upsert_library` conflict key and the row's identity, and clobbering it is
    /// exactly the phantom-row bug this avoids. `base_url` is COALESCE'd so a
    /// missing value doesn't wipe the stored one.
    pub async fn update_library_source(
        &self, id: &uuid::Uuid, source_type: &str, base_url: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.libraries
                SET source_type = $2::sensei.library_source_type,
                    base_url = COALESCE($3, base_url),
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(id).bind(source_type).bind(base_url)
        .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Library capabilities (workstream D): skills/agents a library provides ──
    // Two writers coexist in one table, keyed by `source` ('manifest' | 'generated').

    /// Manifest-authoritative replace of a library's `source`-scoped capabilities:
    /// delete this library's rows for `source`, then re-insert — so a skill/agent
    /// REMOVED from a manifest disappears on re-ingest. One transaction. Mirrors
    /// [`Self::replace_folder_commands`]. `version_range` is the manifest's applies-to
    /// range (same for all rows). Only entries with a resolved `body` are persisted
    /// (a path/body-less entry is dropped upstream at ingest — no fabrication).
    /// Returns (skills, agents) written.
    pub async fn replace_library_capabilities(
        &self,
        library_id: &uuid::Uuid,
        source: &str,
        version_range: Option<&str>,
        skills: &[crate::libraries::manifest::ProvidedSkill],
        agents: &[crate::libraries::manifest::ProvidedAgent],
    ) -> Result<(u32, u32), String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx_core::query::query("DELETE FROM sensei.library_skills WHERE library_id = $1 AND source = $2")
            .bind(library_id).bind(source).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        sqlx_core::query::query("DELETE FROM sensei.library_agents WHERE library_id = $1 AND source = $2")
            .bind(library_id).bind(source).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        let mut ns = 0u32;
        for s in skills.iter().filter(|s| s.body.is_some()) {
            sqlx_core::query::query(
                "INSERT INTO sensei.library_skills(library_id, name, focus, body, source, source_path, version_range)
                 VALUES($1,$2,$3,$4,$5,$6,$7)
                 ON CONFLICT(library_id, name) DO UPDATE SET
                   focus=EXCLUDED.focus, body=EXCLUDED.body, source=EXCLUDED.source,
                   source_path=EXCLUDED.source_path, version_range=EXCLUDED.version_range, modified_at=now()"
            ).bind(library_id).bind(&s.name).bind(&s.focus).bind(s.body.as_deref()).bind(source).bind(s.path.as_deref()).bind(version_range)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            ns += 1;
        }
        let mut na = 0u32;
        for a in agents.iter().filter(|a| a.body.is_some()) {
            sqlx_core::query::query(
                "INSERT INTO sensei.library_agents(library_id, name, focus, body, source, source_path, version_range)
                 VALUES($1,$2,$3,$4,$5,$6,$7)
                 ON CONFLICT(library_id, name) DO UPDATE SET
                   focus=EXCLUDED.focus, body=EXCLUDED.body, source=EXCLUDED.source,
                   source_path=EXCLUDED.source_path, version_range=EXCLUDED.version_range, modified_at=now()"
            ).bind(library_id).bind(&a.name).bind(&a.focus).bind(a.body.as_deref()).bind(source).bind(a.path.as_deref()).bind(version_range)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            na += 1;
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok((ns, na))
    }

    /// Skills a library provides, by library NAME. Enum-free; errors propagate.
    pub async fn list_library_skills(&self, library: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, Option<String>, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT s.name, s.focus, s.body, s.source, s.version_range
               FROM sensei.library_skills s JOIN sensei.libraries l ON l.id = s.library_id
              WHERE l.name = $1 ORDER BY s.focus"
        ).bind(library).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, focus, body, source, vr)| {
            serde_json::json!({ "name": name, "focus": focus, "body": body, "source": source, "version_range": vr })
        }).collect())
    }

    /// One skill of a library by `focus`. `focus` is NOT unique (uniqueness is on
    /// name), so this takes the most-recent match via `LIMIT 1` — never a multi-row
    /// error. `None` on a genuine miss (handler → 404), `Err` on failure.
    pub async fn get_library_skill(&self, library: &str, focus: &str) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(String, String, Option<String>, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT s.name, s.focus, s.body, s.source, s.version_range
               FROM sensei.library_skills s JOIN sensei.libraries l ON l.id = s.library_id
              WHERE l.name = $1 AND s.focus = $2 ORDER BY s.modified_at DESC LIMIT 1"
        ).bind(library).bind(focus).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(name, focus, body, source, vr)| {
            serde_json::json!({ "name": name, "focus": focus, "body": body, "source": source, "version_range": vr })
        }))
    }

    /// Review agents a library provides, by library NAME.
    pub async fn list_library_agents(&self, library: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, Option<String>, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT a.name, a.focus, a.body, a.source, a.version_range
               FROM sensei.library_agents a JOIN sensei.libraries l ON l.id = a.library_id
              WHERE l.name = $1 ORDER BY a.focus"
        ).bind(library).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, focus, body, source, vr)| {
            serde_json::json!({ "name": name, "focus": focus, "body": body, "source": source, "version_range": vr })
        }).collect())
    }

    /// The library skills/agents to SUGGEST for a project, from the libraries it
    /// depends on — REUSES `project_libraries_resolved` (the same view
    /// [`Self::get_project_libraries`] reads) joined to the capability tables. Backs
    /// the recommender enrichment. Returns `{suggested_skills, suggested_agents}`.
    pub async fn list_project_library_capabilities(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        let skills: Vec<(String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT pl.name, s.name, s.focus
               FROM sensei.project_libraries_resolved pl
               JOIN sensei.library_skills s ON s.library_id = pl.id
              WHERE (pl.scoped_project_id = $1 OR pl.scoped_project_id IS NULL) AND pl.enabled = true
              ORDER BY pl.name, s.focus"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        let agents: Vec<(String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT pl.name, a.name, a.focus
               FROM sensei.project_libraries_resolved pl
               JOIN sensei.library_agents a ON a.library_id = pl.id
              WHERE (pl.scoped_project_id = $1 OR pl.scoped_project_id IS NULL) AND pl.enabled = true
              ORDER BY pl.name, a.focus"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "suggested_skills": skills.into_iter().map(|(lib, name, focus)| serde_json::json!({ "library": lib, "name": name, "focus": focus })).collect::<Vec<_>>(),
            "suggested_agents": agents.into_iter().map(|(lib, name, focus)| serde_json::json!({ "library": lib, "name": name, "focus": focus })).collect::<Vec<_>>(),
        }))
    }

    // ── Library update detection (workstream F, v0) ────────────────────────────

    /// Library pins per project, for the update scheduler: joins referenced_libraries
    /// (the folder's pinned `version_used`) → folders (project) → libraries. Returns
    /// `(library_id, name, ecosystem, local_path, project_id, version_used, base_url,
    /// source_type)`; only rows with a project and a non-empty pin. `base_url` +
    /// `source_type` let the apply arm rebuild the re-index `task.url` fail-closed.
    pub async fn list_library_project_pins(
        &self,
    ) -> Result<Vec<(uuid::Uuid, String, String, Option<String>, uuid::Uuid, String, Option<String>, Option<String>)>, String> {
        let rows = sqlx_core::query_as::query_as(
            "SELECT l.id, l.name, l.ecosystem::text, l.local_path, f.project_id, rl.version_used, l.base_url, l.source_type::text
               FROM sensei.referenced_libraries rl
               JOIN sensei.libraries l ON l.id = rl.library_id
               JOIN sensei.folders f ON f.id = rl.folder_id
              WHERE f.project_id IS NOT NULL AND rl.version_used IS NOT NULL AND rl.version_used <> ''",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Cache the latest-known version + check time for a library in `libraries.props`
    /// (the TTL guard against re-hitting registries every tick). No schema change.
    pub async fn set_library_latest_cache(&self, library_id: &uuid::Uuid, latest: &str, checked_at_unix: i64) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.libraries
                SET props = coalesce(props, '{}'::jsonb)
                          || jsonb_build_object('latest_version', $2::text, 'latest_checked_at', $3::bigint)
              WHERE id = $1",
        )
        .bind(library_id).bind(latest).bind(checked_at_unix)
        .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The cached `(latest_version, latest_checked_at_unix)` from `libraries.props`,
    /// if both are present.
    pub async fn get_library_latest_cache(&self, library_id: &uuid::Uuid) -> Result<Option<(String, i64)>, String> {
        let row: Option<(Option<String>, Option<i64>)> = sqlx_core::query_as::query_as(
            "SELECT props->>'latest_version', (props->>'latest_checked_at')::bigint FROM sensei.libraries WHERE id = $1",
        )
        .bind(library_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.and_then(|(v, t)| match (v, t) {
            (Some(v), Some(t)) => Some((v, t)),
            _ => None,
        }))
    }

    /// Stamp the "docs applied at version" marker in `libraries.props` after a
    /// CONFIRMED, non-empty re-index (F v1 auto-apply). Mirrors
    /// [`Self::set_library_latest_cache`]'s single-statement jsonb merge — no
    /// schema change. Only ever written on success, so it never fabricates
    /// "applied".
    pub async fn set_library_docs_applied(&self, library_id: &uuid::Uuid, version: &str, applied_at_unix: i64) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.libraries
                SET props = coalesce(props, '{}'::jsonb)
                          || jsonb_build_object('docs_applied_version', $2::text, 'docs_applied_at', $3::bigint)
              WHERE id = $1",
        )
        .bind(library_id).bind(version).bind(applied_at_unix)
        .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The `docs_applied_version` marker from `libraries.props`, if present — the
    /// gate that stops the scheduler re-applying an already-applied version.
    pub async fn get_library_docs_applied(&self, library_id: &uuid::Uuid) -> Result<Option<String>, String> {
        let row: Option<(Option<String>,)> = sqlx_core::query_as::query_as(
            "SELECT props->>'docs_applied_version' FROM sensei.libraries WHERE id = $1",
        )
        .bind(library_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.and_then(|(v,)| v))
    }

    /// True if a recommendation already flags this project's update of `library_id`
    /// to `to_version` at the given security tier. `is_security` discriminates the
    /// tier so a prior/dismissed non-security notify can't suppress a later security
    /// flag (and vice-versa). Mirrors [`Self::recommendation_exists_for_pattern`],
    /// keyed on the library payload in `based_on`.
    pub async fn pending_library_update_exists(&self, project_id: &uuid::Uuid, library_id: &uuid::Uuid, to_version: &str, is_security: bool) -> Result<bool, String> {
        // The is_security discriminator: a row's tier is `based_on.is_security`
        // (absent/false = non-security). COALESCE the missing key to false so a
        // legacy notify (no key) reads as non-security, and only a same-tier row
        // matches — a non-security notify can't dedup-suppress a security flag.
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(
               SELECT 1 FROM inference.recommendations
                WHERE project_id = $1 AND action_type = 'library_update'
                  AND based_on->'library_update' @> jsonb_build_object('library_id', $2::text, 'to_version', $3::text)
                  AND COALESCE((based_on->'library_update'->>'is_security')::boolean, false) = $4)",
        )
        .bind(project_id).bind(library_id.to_string()).bind(to_version).bind(is_security)
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
        local_path: Option<&str>, description: Option<&str>, content: Option<&str>,
        source_type: &str, component: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.library_pages(library_id, title, url, local_path, description, content, source_type, component, fetched_at)
             VALUES($1, $2, $3, $4, $5, $6, $7::sensei.library_source_type, $8, now())
             ON CONFLICT(library_id, title) DO UPDATE SET
               url = COALESCE(EXCLUDED.url, library_pages.url),
               local_path = COALESCE(EXCLUDED.local_path, library_pages.local_path),
               description = COALESCE(EXCLUDED.description, library_pages.description),
               content = COALESCE(EXCLUDED.content, library_pages.content),
               component = COALESCE(EXCLUDED.component, library_pages.component),
               fetched_at = now(), modified_at = now()
             RETURNING id"
        ).bind(library_id).bind(title).bind(url).bind(local_path).bind(description).bind(content).bind(source_type).bind(component)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn update_library_page_count(&self, library_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.libraries SET page_count = (SELECT count(*) FROM sensei.library_pages WHERE library_id = $1), modified_at = now() WHERE id = $1"
        ).bind(library_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Upsert a folder → library edge with optional `version_used` and `props`.
    ///
    /// `props` is merged (`||`) with any existing row's props, so callers can
    /// stack tags across passes without clobbering earlier metadata. Pass
    /// `None` for a props-free upsert.
    ///
    /// Typical `props` shape: `{"local_source": "../actions", "protocol": "link"}`
    /// for a dep declared via `link:` / `workspace:` / `file:` / Cargo `path=`.
    pub async fn upsert_referenced_library(
        &self,
        folder_id: &uuid::Uuid,
        library_id: &uuid::Uuid,
        version: Option<&str>,
        props: Option<serde_json::Value>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.referenced_libraries(folder_id, library_id, version_used, props)
             VALUES($1, $2, $3, COALESCE($4, '{}'::jsonb))
             ON CONFLICT(folder_id, library_id) DO UPDATE SET
               version_used = COALESCE(EXCLUDED.version_used, referenced_libraries.version_used),
               props = referenced_libraries.props || EXCLUDED.props,
               modified_at = now()"
        ).bind(folder_id).bind(library_id).bind(version).bind(props)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Upsert a project → project edge into `sensei.project_dependencies`.
    ///
    /// Called from `extract_deps` when a `link:` / `workspace:` / `file:` /
    /// `path=` dep resolves to a sibling folder that belongs to a DIFFERENT
    /// project than the declaring folder. Idempotent on the composite PK
    /// `(from_project_id, to_project_id, from_folder_id, source_manifest)`.
    pub async fn upsert_project_dependency(
        &self,
        from_project_id: &uuid::Uuid,
        to_project_id: &uuid::Uuid,
        from_folder_id: &uuid::Uuid,
        source_protocol: &str,
        source_manifest: &str,
        resolved_target: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.project_dependencies
                (from_project_id, to_project_id, from_folder_id, source_protocol, source_manifest, resolved_target)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (from_project_id, to_project_id, from_folder_id, source_manifest) DO UPDATE SET
               source_protocol = EXCLUDED.source_protocol,
               resolved_target = EXCLUDED.resolved_target,
               modified_at = now()"
        )
            .bind(from_project_id)
            .bind(to_project_id)
            .bind(from_folder_id)
            .bind(source_protocol)
            .bind(source_manifest)
            .bind(resolved_target)
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

    pub async fn get_project_libraries(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        // Query the resolved view directly — it already joins libraries internally.
        // Extended for T3 Slice 1.5: pull `page_count` (indexed docs marker) and
        // `local_path` (workspace / local-source marker) so the Libraries page
        // can render "wrapped by sensei" and "local source" badges without a
        // second round-trip.
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            uuid::Uuid, String, String, Option<String>, bool,
            serde_json::Value, String, i32, Option<String>,
        )> = sqlx_core::query_as::query_as(
                "SELECT id, name, ecosystem::text, description, enabled,
                        project_props, scope, page_count, local_path
                 FROM sensei.project_libraries_resolved
                 WHERE (scoped_project_id = $1 OR scoped_project_id IS NULL)
                   AND enabled = true
                 ORDER BY scope DESC, name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, ecosystem, desc, enabled, props, scope, page_count, local_path)| {
            serde_json::json!({
                "id":            id,
                "name":          name,
                "ecosystem":     ecosystem,
                "description":   desc,
                "enabled":       enabled,
                "project_props": props,
                "scope":         scope,
                "hasDocs":       page_count > 0,
                "pageCount":     page_count,
                "localSource":   local_path,
            })
        }).collect())
    }

    /// List libraries pinned to different versions across folders of a project.
    ///
    /// Reads `sensei.project_library_version_conflicts` — excludes local-
    /// protocol deps so only registry-version drift surfaces. Returns one row
    /// per conflicting (project, library) pair with the distinct versions and
    /// the folders where each version was seen.
    pub async fn list_project_library_version_conflicts(
        &self, project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Vec<String>, Vec<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT library_id, library_name, ecosystem, versions, folders
                   FROM sensei.project_library_version_conflicts
                  WHERE project_id = $1
                  ORDER BY library_name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(lib_id, name, ecosystem, versions, folders)| {
            serde_json::json!({
                "library_id": lib_id,
                "library_name": name,
                "ecosystem": ecosystem,
                "versions": versions,
                "folders": folders,
            })
        }).collect())
    }

    /// List outgoing project → project edges for a project.
    ///
    /// Returns one row per edge with the target project's name joined in.
    /// Sorted by target project name for stable UI ordering.
    pub async fn list_project_dependencies(
        &self, project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, uuid::Uuid, String, String, Option<String>, String)> =
            sqlx_core::query_as::query_as(
                "SELECT to_p.id, to_p.name, pd.from_folder_id, pd.source_protocol,
                        pd.source_manifest, pd.resolved_target, from_f.name
                   FROM sensei.project_dependencies pd
                   JOIN sensei.projects to_p   ON to_p.id   = pd.to_project_id
                   JOIN sensei.folders  from_f ON from_f.id = pd.from_folder_id
                  WHERE pd.from_project_id = $1
                  ORDER BY to_p.name, from_f.name, pd.source_manifest"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(to_id, to_name, from_folder_id, protocol, manifest, target, from_folder_name)| {
            serde_json::json!({
                "to_project_id": to_id,
                "to_project_name": to_name,
                "from_folder_id": from_folder_id,
                "from_folder": from_folder_name,
                "source_protocol": protocol,
                "source_manifest": manifest,
                "resolved_target": target,
            })
        }).collect())
    }

}
