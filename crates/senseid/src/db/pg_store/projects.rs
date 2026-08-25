use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    pub async fn create_project(
        &self,
        name: &str,
        description: Option<&str>,
        client: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.projects(name, description, client) VALUES($1, $2, $3) RETURNING id"
        ).bind(name).bind(description).bind(client)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Race-safe get-or-create of a project by name — the scan-time assignment
    /// path ([`crate::tasks::handlers::process_git_folder`]) calls this instead
    /// of a bare SELECT-then-INSERT. That earlier pattern raced across the
    /// concurrent scan workers: two folders resolving to the same project name
    /// both saw "no such project" and both called [`Self::create_project`],
    /// minting a second same-name row — the 0-folder "phantom" project that then
    /// made name resolution ambiguous.
    ///
    /// A transaction-scoped advisory lock keyed on the name serializes only
    /// concurrent creators of the SAME name (distinct names hash to distinct
    /// keys and never contend), closing the select-then-insert window WITHOUT a
    /// `UNIQUE(name)` constraint — which would be wrong, since two DIFFERENT
    /// repos may legitimately share a name (a project's identity is its folder
    /// path, not its name).
    ///
    /// When the name already has rows, the folder-bearing one is preferred, so a
    /// pre-existing phantom is never adopted over the real project (the phantom
    /// is pruned separately by [`Self::heal_duplicate_name_projects`]). Returns
    /// `(id, created)`; `created` is true only when a new row was minted, letting
    /// the caller emit its `project_add` event exactly once.
    pub async fn get_or_create_project_by_name(
        &self,
        name: &str,
    ) -> Result<(uuid::Uuid, bool), String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        // Serialize concurrent creators of this exact name. The lock is
        // transaction-scoped (auto-released on commit/rollback); hashtext maps
        // the name into the advisory key space.
        sqlx_core::query::query("SELECT pg_advisory_xact_lock(hashtext($1)::int8)")
            .bind(name)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        // Prefer the folder-bearing row so a not-yet-healed phantom is never
        // adopted over the real project; `id` is the stable tiebreak.
        let existing: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT p.id FROM sensei.projects p
              WHERE p.name = $1
              ORDER BY (SELECT count(*) FROM sensei.folders f WHERE f.project_id = p.id) DESC, p.id
              LIMIT 1",
        )
        .bind(name)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        if let Some((id,)) = existing {
            tx.commit().await.map_err(|e| e.to_string())?;
            return Ok((id, false));
        }

        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.projects(name) VALUES($1) RETURNING id",
        )
        .bind(name)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok((id, true))
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

    /// Overwrite a project's `icon` jsonb with a deterministically inferred icon
    /// ([[pipeline/project-icon]]). The caller guards against clobbering an
    /// author choice; this setter just persists the value.
    pub async fn set_project_icon(
        &self,
        id: &uuid::Uuid,
        icon: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.projects SET icon = $2, modified_at = now() WHERE id = $1",
        )
        .bind(id)
        .bind(icon)
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

    /// Self-healing reconcile: DELETE `discovery` projects that hold nothing — no
    /// folders, no sessions, no learned artifacts (recommendations / memories).
    /// These are phantom rows left when a promoted crate/subfolder was later
    /// reconciled away (pre-#101 residue: names like `logger`, `senseid`,
    /// `gateway-embedded`). `mark_orphaned_projects` only tags them; this removes
    /// the provably-empty ones so they never reach the UI. A project mid-scan has
    /// its git/standalone folder already, so it never matches.
    ///
    /// `grace_secs` guards `modified_at`: scan reconcile passes 60 so a project
    /// just created but whose folder is still being attached in a concurrent step
    /// isn't deleted mid-population (also fixes a shared-test-DB FK race). A
    /// *deliberate* caller — the exclusion handler, which already deleted the
    /// subtree's folders — passes 0: those projects are provably orphaned, not
    /// in-flight, and a boot re-scan may have freshly bumped their `modified_at`.
    /// Returns rows deleted.
    pub async fn prune_empty_projects(&self, grace_secs: i32) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "DELETE FROM sensei.projects p
              WHERE p.maturity = 'discovery'
                AND p.modified_at < now() - make_interval(secs => $1)
                AND NOT EXISTS (SELECT 1 FROM sensei.folders f        WHERE f.project_id = p.id)
                AND NOT EXISTS (SELECT 1 FROM activity.sessions s     WHERE s.project_id = p.id)
                AND NOT EXISTS (SELECT 1 FROM inference.recommendations r WHERE r.project_id = p.id)
                AND NOT EXISTS (SELECT 1 FROM sensei.memories m       WHERE m.project_id = p.id)",
        )
        .bind(grace_secs)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    pub async fn get_project(&self, id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        #[allow(clippy::type_complexity)]
        let row: Option<(uuid::Uuid, String, Option<String>, Option<String>, String, Option<String>, serde_json::Value, serde_json::Value, serde_json::Value, Vec<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, description, client, maturity::text, goal, icon, stack, links, tags, modified_at FROM sensei.projects WHERE id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(row.map(|(id, name, desc, client, maturity, goal, icon, stack, links, tags, modified)| {
            serde_json::json!({
                "id": id, "name": name, "description": desc, "client": client,
                "maturity": maturity, "goal": goal, "icon": icon, "stack": stack, "links": links,
                "tags": tags, "modified_at": modified.to_rfc3339(),
            })
        }))
    }

    /// Overview stat scalars for a project in one round trip: active (non-
    /// archived) memory count, 7-day session + corrected counts, and open
    /// doc-drift + distinct-referenced-doc counts.
    ///
    /// `readyToShare` / `toMerge` are DERIVED from existing columns (no invented
    /// status — [[pipeline/memory]] defines a scope *ladder*, not new statuses):
    /// - `readyToShare` = established memories (status active/reinforced/
    ///   battle_tested) whose `scope` is narrower than the widest rung
    ///   (`global`) — i.e. promotable up the ladder (project→…→global).
    /// - `toMerge` = memories that share a normalized `title` with at least one
    ///   other memory in the project (dedup candidates). There is no signature
    ///   column, so a case/whitespace-folded title is the merge-candidate proxy.
    pub async fn get_project_overview_stats(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<serde_json::Value, String> {
        let (mem_total, sessions_7d, sessions_7d_corrected, drift_open, referenced_docs, ready_to_share, to_merge):
            (i64, i64, i64, i64, i64, i64, i64) = sqlx_core::query_as::query_as(
            "SELECT
               (SELECT count(*) FROM sensei.memories
                  WHERE project_id = $1 AND status != 'archived'),
               (SELECT count(*) FROM activity.sessions
                  WHERE project_id = $1 AND started_at > now() - interval '7 days'),
               (SELECT count(*) FROM activity.sessions
                  WHERE project_id = $1 AND started_at > now() - interval '7 days' AND corrections > 0),
               (SELECT count(*) FROM sensei.project_drift
                  WHERE project_id = $1 AND status::text IN ('drifted','broken')),
               (SELECT count(DISTINCT di.doc_node_id) FROM inference.drift_items di
                  JOIN sensei.folders f ON f.id = di.folder_id WHERE f.project_id = $1),
               (SELECT count(*) FROM sensei.memories
                  WHERE project_id = $1
                    AND status::text IN ('active','reinforced','battle_tested')
                    AND scope::text <> 'global'),
               (SELECT coalesce(sum(c), 0)::bigint FROM (
                    SELECT count(*) AS c FROM sensei.memories
                      WHERE project_id = $1 AND status != 'archived'
                      GROUP BY lower(btrim(title)) HAVING count(*) > 1) g)"
        ).bind(project_id).fetch_one(&self.pool).await
            .map_err(|e| { tracing::error!(error = %e, "get_project_overview_stats failed"); e.to_string() })?;
        Ok(serde_json::json!({
            "sessions7d": sessions_7d,
            "sessions7dCorrected": sessions_7d_corrected,
            "memories": { "total": mem_total, "readyToShare": ready_to_share, "toMerge": to_merge },
            "docDrift": { "open": drift_open, "referencedDocs": referenced_docs },
        }))
    }

    pub async fn get_project_by_name(
        &self,
        name: &str,
    ) -> Result<Option<serde_json::Value>, String> {
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
        self.list_projects_under(None).await
    }

    /// Like [`list_projects`], optionally scoped to a folder. When `under` is
    /// `Some(path)`, only projects that own at least one folder whose `abs_path`
    /// is `path` itself OR lives beneath `path` are returned — path-boundary-safe,
    /// so a sibling `path-other` is NOT matched (the boundary test is
    /// `left(abs_path, len(path)+1) = path || '/'`, never a raw `LIKE` prefix
    /// that would let `_`/`%` in the path act as wildcards). `None` returns every
    /// project (unchanged behavior). The path is a bound parameter — never
    /// interpolated.
    pub async fn list_projects_under(
        &self,
        under: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        // Additive: also returns icon/stack/vision(goal) plus repos_count /
        // libs_count / last_session_at / sessions7d so the Projects index can
        // render its card + list layouts without a per-project fanout. Existing
        // consumers (e.g. the Today loader) keep working — nothing is removed.
        // repos_count counts only real repos (folders.kind git|standalone), NOT
        // the ~10k nested `folder` rows.
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            uuid::Uuid, String, Option<String>, Option<String>, String, Vec<String>,
            chrono::DateTime<chrono::Utc>, Option<serde_json::Value>, Option<serde_json::Value>,
            Option<String>, Option<uuid::Uuid>, i64, i64, Option<chrono::DateTime<chrono::Utc>>, i64,
        )> = sqlx_core::query_as::query_as(
                "SELECT p.id, p.name, p.description, p.client, p.maturity::text, p.tags, p.modified_at,
                        p.icon, p.stack, p.goal, p.dojo_id,
                        (SELECT count(*) FROM sensei.folders f
                          WHERE f.project_id = p.id AND f.kind::text IN ('git','standalone'))::bigint AS repos_count,
                        (SELECT count(*) FROM sensei.project_libraries pl
                          WHERE pl.project_id = p.id)::bigint AS libs_count,
                        (SELECT max(s.started_at) FROM activity.sessions s WHERE s.project_id = p.id) AS last_session_at,
                        (SELECT count(*) FROM activity.sessions s
                          WHERE s.project_id = p.id AND s.started_at > now() - interval '7 days')::bigint AS sessions7d
                 FROM sensei.projects p
                 WHERE $1::text IS NULL OR EXISTS (
                          SELECT 1 FROM sensei.folders f
                           WHERE f.project_id = p.id
                             AND (f.abs_path = $1::text
                               OR left(f.abs_path, length($1::text) + 1) = $1::text || '/'))
                 ORDER BY p.name"
            ).bind(under).fetch_all(&self.pool).await
            .map_err(|e| { tracing::error!(error = %e, "list_projects failed"); e.to_string() })?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    name,
                    desc,
                    client,
                    maturity,
                    tags,
                    modified,
                    icon,
                    stack,
                    vision,
                    dojo_id,
                    repos_count,
                    libs_count,
                    last_session_at,
                    sessions7d,
                )| {
                    serde_json::json!({
                        "id": id, "name": name, "description": desc, "client": client,
                        "maturity": maturity, "tags": tags, "modified_at": modified.to_rfc3339(),
                        "icon": icon, "stack": stack, "vision": vision,
                        "dojo_id": dojo_id,
                        "repos_count": repos_count, "libs_count": libs_count,
                        "last_session_at": last_session_at.map(|t| t.to_rfc3339()),
                        "sessions7d": sessions7d,
                    })
                },
            )
            .collect())
    }

    /// Partial-update a project's editable identity fields. Omitted (`None`)
    /// fields are left untouched via COALESCE, so a lossless patch from the
    /// About form only overwrites the columns the user actually edited. An
    /// unknown `maturity` is rejected up front (before the DB round trip)
    /// rather than allowed to fail as a raw Postgres enum-cast error.
    pub async fn update_project(
        &self,
        id: &uuid::Uuid,
        patch: &ProjectPatch<'_>,
    ) -> Result<(), String> {
        if let Some(m) = patch.maturity
            && !PROJECT_MATURITIES.contains(&m)
        {
            return Err(format!("invalid maturity '{m}': expected one of {PROJECT_MATURITIES:?}"));
        }
        sqlx_core::query::query(
            "UPDATE sensei.projects SET
                 name          = COALESCE($2, name),
                 description   = COALESCE($3, description),
                 maturity      = COALESCE($4::sensei.project_maturity, maturity),
                 client        = COALESCE($5, client),
                 goal          = COALESCE($6, goal),
                 preferred_acp = COALESCE($7, preferred_acp),
                 icon          = COALESCE($8, icon),
                 stack         = COALESCE($9, stack),
                 links         = COALESCE($10, links),
                 modified_at   = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(patch.name)
        .bind(patch.description)
        .bind(patch.maturity)
        .bind(patch.client)
        .bind(patch.goal)
        .bind(patch.preferred_acp)
        .bind(patch.icon)
        .bind(patch.stack)
        .bind(patch.links)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "update_project failed");
            e.to_string()
        })?;
        Ok(())
    }

    pub async fn delete_project(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Merge one project into another (#41). All source-project folders +
    /// sessions + memories are reassigned to `target`, then the source
    /// project row is deleted; ON DELETE CASCADE cleans up the derived rows
    /// (detected_patterns, recommendations, reasoning_traces,
    /// impact_verdicts, memory_share_batches, service_projects,
    /// project_dependencies edges terminating at the source).
    ///
    /// Derived signals (patterns/recommendations) are dropped and
    /// regenerated by the analyzer on the next tick over the merged corpus
    /// — that's why we don't try to hand-merge them here (the unique keys
    /// on those tables would need collision handling that isn't worth the
    /// code for a delete-and-rederive path). User-authored memories DO
    /// survive because `memories.project_id` is nullable + non-unique;
    /// they simply move under the target.
    ///
    /// Runs inside a transaction so a mid-way failure doesn't leave the
    /// merge half-done. Refuses `source == target` (no-op guarded up front)
    /// and errors if either project id doesn't exist.
    pub async fn merge_projects(
        &self,
        source: &uuid::Uuid,
        target: &uuid::Uuid,
    ) -> Result<(), String> {
        if source == target {
            return Err("merge_projects: source and target must differ".into());
        }
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        // Verify both projects exist.
        let (exists,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT COUNT(*) FROM sensei.projects WHERE id = ANY($1::uuid[])",
        )
        .bind([*source, *target].as_slice())
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        if exists != 2 {
            return Err(format!(
                "merge_projects: expected source + target to exist, found {exists} of 2"
            ));
        }

        // Reassign the data-source rows. Order: folders first (they define
        // the corpus), then sessions, then memories (user-authored — must
        // survive the merge). Derived tables are left for CASCADE to trim.
        for stmt in [
            "UPDATE sensei.folders    SET project_id = $2 WHERE project_id = $1",
            "UPDATE activity.sessions SET project_id = $2 WHERE project_id = $1",
            "UPDATE sensei.memories   SET project_id = $2 WHERE project_id = $1",
        ] {
            sqlx_core::query::query(stmt)
                .bind(source)
                .bind(target)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        }

        // CASCADE deletes derived rows (detected_patterns / recommendations /
        // reasoning_traces / impact_verdicts / memory_share_batches /
        // service_projects / project_dependencies edges at either end).
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(source)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Deterministic self-heal for name-duplicate phantom projects: when a name
    /// is shared by EXACTLY ONE folder-bearing project (the survivor) and one or
    /// more 0-folder `discovery` projects (phantoms — an earlier
    /// select-then-insert race minted them; now prevented by
    /// [`Self::get_or_create_project_by_name`]), each phantom is merged into the
    /// survivor via [`Self::merge_projects`] (folders/sessions/memories
    /// reassigned, derived rows CASCADE-trimmed — no FK row is left orphaned).
    /// Idempotent: once the phantoms are gone the candidate query returns
    /// nothing, so a re-run is a no-op.
    ///
    /// Deliberately conservative — a name shared by TWO folder-bearing projects
    /// (two different repos at different paths that happen to share a name) is
    /// LEFT ALONE: those are legitimately distinct projects (identity = path,
    /// not name) and must never be merged. All-empty same-name groups are also
    /// left untouched for [`Self::mark_orphaned_projects`] to tag. Returns the
    /// number of phantoms merged away.
    pub async fn heal_duplicate_name_projects(&self) -> Result<u64, String> {
        let pairs = self.duplicate_name_phantom_pairs().await?;

        let mut healed = 0u64;
        for (phantom, survivor) in pairs {
            match self.merge_projects(&phantom, &survivor).await {
                Ok(()) => {
                    healed += 1;
                    tracing::info!(phantom = %phantom, survivor = %survivor,
                        "heal_duplicate_name_projects: merged 0-folder phantom into folder-bearing survivor");
                }
                Err(e) => tracing::warn!(phantom = %phantom, survivor = %survivor, error = %e,
                    "heal_duplicate_name_projects: merge failed"),
            }
        }
        Ok(healed)
    }

    /// Read-only detection counterpart to [`Self::heal_duplicate_name_projects`]:
    /// the phantom project ids that WOULD be merged into their folder-bearing
    /// survivor. Shares the candidate query; performs no mutation. Used by the
    /// index integrity audit's read-only (`doctor`) pass.
    pub async fn detect_duplicate_name_phantoms(&self) -> Result<Vec<uuid::Uuid>, String> {
        Ok(self
            .duplicate_name_phantom_pairs()
            .await?
            .into_iter()
            .map(|(phantom, _)| phantom)
            .collect())
    }

    /// Self-heal Bug 3: re-absorb a `standalone` project root that was
    /// mis-scoped INSIDE an existing git repo (e.g. a moved `crates/*` sub-crate
    /// registered as its own project instead of a folder of the monorepo). For
    /// each standalone folder nested under a git-repo folder that belongs to a
    /// DIFFERENT project:
    ///
    /// 1. Its own (repo-relative-to-itself) nodes are dropped — the enclosing
    ///    repo re-indexes the subtree with repo-relative paths on its next
    ///    `ProcessGitFolder`, so no duplicate nodes survive. (Node deletion
    ///    cascades edges; it does NOT touch `activity.sessions`, which key on
    ///    `folder_id` — those are preserved.)
    /// 2. The folder row is re-classified `kind='folder'`, re-parented under the
    ///    repo, and re-pointed at the repo's project — so its code attributes to
    ///    the repo's project, exactly like `crates/hive-mind` used to.
    /// 3. When the mis-scoped project then lives ENTIRELY inside the repo, it is
    ///    folded into the repo's project via [`Self::merge_projects`] (moving any
    ///    sessions/memories, CASCADE-trimming derived rows, deleting the phantom).
    ///    A phantom that also owns unrelated folders elsewhere is left for
    ///    [`Self::mark_orphaned_projects`] rather than dragged in.
    ///
    /// Idempotent — once re-absorbed the candidate query returns nothing.
    /// Returns the number of roots re-absorbed.
    pub async fn heal_nested_standalone_roots(&self) -> Result<u64, String> {
        let pairs = self.nested_standalone_candidates().await?;

        let mut healed = 0u64;
        for (s_id, s_pid, g_id, g_pid, g_root, g_abs) in pairs {
            // 1. Drop the mis-scoped root's own nodes (repo re-indexes the subtree).
            if let Err(e) = self.delete_nodes_by_folder(&s_id).await {
                tracing::warn!(folder = %s_id, error = %e, "heal_nested_standalone_roots: delete_nodes_by_folder failed");
                continue;
            }
            // 2. Re-classify as a folder of the enclosing repo's project, under
            //    the repo's watch root (it may have been registered under another).
            if let Err(e) = sqlx_core::query::query(
                "UPDATE sensei.folders
                    SET kind = 'folder'::sensei.folder_kind,
                        parent_id = $2, project_id = $3, root_id = $4, modified_at = now()
                  WHERE id = $1",
            )
            .bind(s_id)
            .bind(g_id)
            .bind(g_pid)
            .bind(g_root)
            .execute(&self.pool)
            .await
            {
                tracing::warn!(folder = %s_id, error = %e, "heal_nested_standalone_roots: re-attribute failed");
                continue;
            }
            // 3. Fold the phantom project into the repo's project when it lives
            //    entirely inside the repo (the folder above was already re-pointed
            //    to g_pid, so it no longer counts against s_pid).
            if let Some(s_pid) = s_pid.filter(|p| *p != g_pid) {
                let outside: (i64,) = sqlx_core::query_as::query_as(
                    "SELECT count(*) FROM sensei.folders
                      WHERE project_id = $1 AND NOT starts_with(abs_path, $2 || '/')",
                )
                .bind(s_pid)
                .bind(&g_abs)
                .fetch_one(&self.pool)
                .await
                .unwrap_or((1,));
                if outside.0 == 0 {
                    match self.merge_projects(&s_pid, &g_pid).await {
                        Ok(()) => tracing::info!(phantom = %s_pid, survivor = %g_pid,
                            "heal_nested_standalone_roots: merged phantom project into enclosing repo's project"),
                        Err(e) => tracing::warn!(phantom = %s_pid, survivor = %g_pid, error = %e,
                            "heal_nested_standalone_roots: merge_projects failed"),
                    }
                } else {
                    tracing::info!(phantom = %s_pid, outside = outside.0,
                        "heal_nested_standalone_roots: phantom has folders outside the repo — left for orphan-tagging");
                }
            }
            healed += 1;
            tracing::info!(folder = %s_id, project = %g_pid, "heal_nested_standalone_roots: re-absorbed nested standalone root");
        }
        Ok(healed)
    }

    /// Read-only detection counterpart to [`Self::heal_nested_standalone_roots`]:
    /// the abs_paths of standalone roots currently mis-scoped inside a git repo
    /// (what the heal WOULD re-absorb). Shares the candidate query; performs no
    /// mutation. Used by the index integrity audit's read-only (`doctor`) pass.
    pub async fn detect_nested_standalone_roots(&self) -> Result<Vec<String>, String> {
        Ok(self.nested_standalone_candidates().await?.into_iter().map(|c| c.5).collect())
    }

    pub async fn get_project_drift(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<serde_json::Value, String> {
        // `expected_signature` and `actual_signature` power the Traceability
        // detail drawer's Expected-vs-Actual diff. Both are nullable — `broken`
        // rows carry no `actual`, `drifted` carries both, `current` may carry
        // neither depending on how the detector wrote the row.
        type DriftRow = (
            uuid::Uuid,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
        );
        let rows: Vec<DriftRow> = sqlx_core::query_as::query_as(
            "SELECT id, status::text, detail, expected_signature, actual_signature, detected_at
                 FROM sensei.project_drift WHERE project_id = $1
                 ORDER BY detected_at DESC LIMIT 200",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let total = rows.len();
        let drifted = rows.iter().filter(|r| r.1 == "drifted").count();
        let broken = rows.iter().filter(|r| r.1 == "broken").count();
        let items: Vec<_> = rows
            .into_iter()
            .map(|(id, status, detail, expected, actual, detected_at)| {
                serde_json::json!({
                    "id": id, "status": status, "detail": detail,
                    "expectedSignature": expected,
                    "actualSignature":   actual,
                    "detectedAt": detected_at.to_rfc3339(),
                })
            })
            .collect();

        Ok(
            serde_json::json!({ "items": items, "total": total, "drifted": drifted, "broken": broken }),
        )
    }

    /// Resolve a local project by name → its id (scope-match for a project-scoped
    /// artifact). `None` = no such project on this install.
    pub async fn resolve_project_by_name(
        &self,
        name: String,
    ) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> =
            sqlx_core::query_as::query_as("SELECT id FROM sensei.projects WHERE name = $1 LIMIT 1")
                .bind(&name)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn ensure_test_project(&self, name: &str) -> Result<uuid::Uuid, String> {
        // Namespace fixtures under `_test:` so leaked rows are identifiable
        // (and filterable by the Projects screen) and never masquerade as real
        // projects. Find-or-create by name so repeated test runs reuse one row
        // instead of minting a fresh UUID each call (#34). Each fixture name is
        // owned by a single test, so the SELECT-then-INSERT is race-free here.
        let name = format!("_test:{name}");
        if let Some(row) = sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM sensei.projects WHERE name = $1",
        )
        .bind(&name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?
        {
            return Ok(row.0);
        }
        let id = uuid::Uuid::new_v4();
        sqlx_core::query::query(
            "INSERT INTO sensei.projects (id, name) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind(&name)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// Inputs for the L3 maturity signal (#71): `(enriched_session_count,
    /// has_insights)`. `has_insights` is true once the analyzer has produced any
    /// recommendation or learned memory for the project.
    pub async fn get_project_maturity_inputs(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<(i64, bool), String> {
        let row: (i64, bool) = sqlx_core::query_as::query_as(
            "SELECT
               (SELECT count(*) FROM activity.sessions WHERE project_id = $1 AND analyzed_at IS NOT NULL),
               (EXISTS(SELECT 1 FROM inference.recommendations WHERE project_id = $1)
                OR EXISTS(SELECT 1 FROM sensei.memories WHERE project_id = $1 AND origin = 'learned'))"
        ).bind(project_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Aggregate maturity inputs across all sessions/projects — powers the
    /// Observatory · Today maturity gate. Mirrors
    /// [`Self::get_project_maturity_inputs`] without the project filter.
    pub async fn get_global_maturity_inputs(&self) -> Result<(i64, bool), String> {
        let row: (i64, bool) = sqlx_core::query_as::query_as(
            "SELECT
               (SELECT count(*) FROM activity.sessions WHERE analyzed_at IS NOT NULL),
               (EXISTS(SELECT 1 FROM inference.recommendations)
                OR EXISTS(SELECT 1 FROM sensei.memories WHERE origin = 'learned'))",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Resolve a repo's namespace at a governance scope — e.g. "this repo's
    /// `project` namespace" or "its `organization` namespace". Used when
    /// authoring a rule so the caller can say "scope this to the project" and we
    /// attach the right namespace_id from the repo's memberships. Returns None
    /// for always-on scopes (`general`/`user`) or when the repo has no namespace
    /// at that scope.
    /// A folder's namespace memberships as `(scope_key, slug)` pairs — the stable
    /// cross-DB identity the Dōjō `rules/resolved` endpoint matches on (the daemon
    /// and Dōjō have separate namespace uuids). Excludes the always-on
    /// general/user scopes (no namespace row). Used to fold adopted-pack rules
    /// into `get_rules`.
    pub async fn folder_namespace_pairs(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<(String, String)>, String> {
        let rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT n.scope_key, n.slug
               FROM sensei.folder_namespaces fn
               JOIN sensei.namespaces n ON n.id = fn.namespace_id
              WHERE fn.folder_id = $1",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    pub async fn namespace_for_folder_scope(
        &self,
        folder_id: &uuid::Uuid,
        scope_key: &str,
    ) -> Result<Option<uuid::Uuid>, String> {
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

    /// Return the list of stack identifiers for a project.
    /// The `sensei.projects.stack` column is JSONB and may be an array of strings,
    /// an object with a recognisable array key, or absent — all cases return `[]`.
    pub async fn get_project_stack_ids(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<String>, String> {
        let row: Option<(serde_json::Value,)> =
            sqlx_core::query_as::query_as("SELECT stack FROM sensei.projects WHERE id = $1")
                .bind(project_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

        let Some((stack_json,)) = row else {
            return Ok(vec![]);
        };

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
                        return Ok(arr
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect());
                    }
                }
                // No recognizable shape — return empty (no stack blending).
                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }

    pub async fn get_project_repos(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        // Only project ROOTS are repos. `kind='folder'` (navigable subfolder tree)
        // AND `kind='workspace_member'` (monorepo members, D5a) are the structural
        // tree, NOT separate repos — listing them makes a single-repo monorepo with
        // N members render as an N+1-repo "multi-repo" project (#62). The data is
        // correct; this read path was projecting the subfolder tree as repos.
        let rows: Vec<(uuid::Uuid, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, abs_path, kind::text FROM sensei.folders
                 WHERE project_id = $1 AND kind::text NOT IN ('folder', 'workspace_member') ORDER BY name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, path, kind)| {
            serde_json::json!({ "id": id, "name": name, "path": path, "kind": kind })
        }).collect())
    }

    /// The distinct git-remote owner slugs across a project's folders (lowercased,
    /// first-seen order) — e.g. a project whose repos are `github.com/sensei-hq/*`
    /// yields `["sensei-hq"]`. Feeds `dojo::routing::infer_binding` for the R3
    /// auto-bind suggestion. Reads `sensei.folders.remote_urls`; DB-only.
    pub async fn project_org_owners(&self, project_id: &uuid::Uuid) -> Result<Vec<String>, String> {
        let folders: Vec<(serde_json::Value,)> = sqlx_core::query_as::query_as(
            "SELECT remote_urls FROM sensei.folders WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let mut owners: Vec<String> = Vec::new();
        for (remotes,) in folders {
            if let Some(arr) = remotes.as_array() {
                for r in arr {
                    if let Some(url) = r.get("url").and_then(serde_json::Value::as_str)
                        && let Some(owner) = remote_owner_slug(url)
                        && !owners.contains(&owner)
                    {
                        owners.push(owner);
                    }
                }
            }
        }
        Ok(owners)
    }

    /// Gather every KNOWN sensitive identifier for a project into a
    /// [`crate::dojo::attribution::ProjectIdentifiers`] — the deterministic
    /// client-work dereference (C5) needs these to strip source references before
    /// anything leaves the machine. Reads only; the strip itself is DB-free.
    ///
    /// Sources: `sensei.projects.{name, client}`, `sensei.folders.{name,
    /// abs_path, remote_urls}` (repo name + git owner/repo parsed from remotes),
    /// and `activity.sessions.{id, client_session_id}`.
    pub async fn project_identifiers(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<crate::dojo::attribution::ProjectIdentifiers, String> {
        use crate::dojo::attribution::ProjectIdentifiers;

        let proj: Option<(String, Option<String>)> =
            sqlx_core::query_as::query_as("SELECT name, client FROM sensei.projects WHERE id = $1")
                .bind(project_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        let (project_name, client_name) = match proj {
            Some((name, client)) => (Some(name), client),
            None => (None, None),
        };

        let folders: Vec<(String, String, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT name, abs_path, remote_urls FROM sensei.folders WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let mut repo_names: Vec<String> = Vec::new();
        let mut folder_paths: Vec<String> = Vec::new();
        for (name, abs_path, remotes) in folders {
            if !name.trim().is_empty() {
                repo_names.push(name);
            }
            if !abs_path.trim().is_empty() {
                folder_paths.push(abs_path);
            }
            if let Some(arr) = remotes.as_array() {
                for r in arr {
                    if let Some(url) = r.get("url").and_then(serde_json::Value::as_str) {
                        repo_names.extend(repo_tokens_from_remote(url));
                    }
                }
            }
        }

        let sessions: Vec<(uuid::Uuid, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, client_session_id FROM activity.sessions WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let mut session_ids: Vec<String> = Vec::new();
        for (id, csid) in sessions {
            session_ids.push(id.to_string());
            if let Some(c) = csid.filter(|c| !c.trim().is_empty()) {
                session_ids.push(c);
            }
        }

        for v in [&mut repo_names, &mut folder_paths, &mut session_ids] {
            v.sort();
            v.dedup();
        }

        Ok(ProjectIdentifiers {
            project_name,
            client_name,
            repo_names,
            folder_paths,
            session_ids,
            // No reliable structured person-name source in the schema yet; C6 can
            // enrich this from session/transcript metadata if one lands.
            person_names: Vec::new(),
        })
    }

    // ── Federation ledger ─────────────────────────────────────────────
}
