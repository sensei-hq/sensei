use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    /// Assemble a blended context blob: project-scoped + stack-scoped + global memories.
    /// Only active/reinforced/battle_tested/challenged memories are included.
    /// Governance Tier-1 resolution: the active rules that apply to a repo,
    /// ordered strongest-first. A rule applies when it sits on one of the repo's
    /// member namespaces (`folder_namespaces`), on an always-on `general`/`user`
    /// scope, is genuinely global (unscoped **and** not tied to a project —
    /// `namespace_id IS NULL AND project_id IS NULL`), or is a project-tied
    /// learned convention for **this repo's own project** (`namespace_id IS NULL
    /// AND project_id = the folder's project`). The last clause is what keeps a
    /// project's learned principle scoped to that project instead of bleeding
    /// into every repo's always-on `general` set: an unscoped memory carrying a
    /// `project_id` is that project's convention, not a global rule. Ordering is
    /// the two-axis precedence — enforcement desc (mandatory first), then scope
    /// level desc (most-specific first), then strength. Structuring (dedup +
    /// mandatory-lock) is done by `crate::governance::structure_ruleset` so it
    /// stays pure.
    pub async fn resolve_rules_raw(&self, folder_id: &uuid::Uuid) -> Result<Vec<crate::governance::RawRule>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT m.id, m.title, m.content, m.impact, m.enforcement::text,
                        COALESCE(n.scope_key,
                                 CASE WHEN m.project_id IS NOT NULL THEN 'project' ELSE 'general' END) AS scope,
                        n.name AS namespace
                   FROM sensei.memories m
                   LEFT JOIN sensei.namespaces n ON n.id = m.namespace_id
                   LEFT JOIN sensei.scopes s ON s.key = n.scope_key
                  WHERE m.status IN ('active'::sensei.memory_status,
                                     'reinforced'::sensei.memory_status,
                                     'battle_tested'::sensei.memory_status)
                    AND ( (m.namespace_id IS NULL AND m.project_id IS NULL)
                          OR n.scope_key IN ('general', 'user')
                          OR m.namespace_id IN (
                                SELECT namespace_id FROM sensei.folder_namespaces WHERE folder_id = $1
                          )
                          OR ( m.namespace_id IS NULL
                               AND m.project_id = (SELECT project_id FROM sensei.folders WHERE id = $1) ) )
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

    /// The LOCAL authoritative raw ruleset for a folder — resolved memories
    /// ([`Self::resolve_rules_raw`]) plus adopted LOCAL rule-pack rules
    /// ([`Self::resolve_local_pack_raws`]), memories strongest-first then packs.
    /// This is the offline constitution the editor resolves; the dōjō
    /// constitution federation composes it into a preview. Fails closed on either
    /// read — a DB error must never silently drop governance. The remote Dōjō pack
    /// fold-in is layered by the api-handler resolver (needs `AppState` + network),
    /// not here, so a task-context federation stays offline.
    pub async fn resolve_repo_raw_local(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<crate::governance::RawRule>, String> {
        let mut raw = self.resolve_rules_raw(folder_id).await?;
        raw.extend(self.resolve_local_pack_raws(Some(folder_id)).await?);
        Ok(raw)
    }

    /// The rules of rule packs adopted at a folder's namespaces (or at the
    /// always-on general/user scopes) resolved from the LOCAL `sensei.rule_packs`
    /// replica (D-LOCAL-PACKS) — offline, in tandem with the remote Dōjō fold-in.
    /// Pass `Some(folder)` for a repo's ruleset; pass `None` for the always-on
    /// GLOBAL set (`~/.sensei/rules.md`), where a NULL bind makes the folder
    /// clause match nothing, leaving only the general/user adoptions.
    /// Effective tier is never-weaken: an adoption override can only RAISE a rule's
    /// enforcement, never lower it (ranked in SQL so the enum's storage order does
    /// not matter). Maps to `RawRule`: scope = the GOVERNANCE scope the pack was
    /// ADOPTED at (the adoption namespace's `scope_key` — general/user/project/…, as
    /// `resolve_rules_raw` does for memories), NOT the pack's own area/category, so
    /// the constitution ladder groups pack rules on the same scope axis as memories;
    /// namespace = the pack source.
    pub async fn resolve_local_pack_raws(
        &self,
        folder_id: Option<&uuid::Uuid>,
    ) -> Result<Vec<crate::governance::RawRule>, String> {
        let rows: Vec<(String, String, String, Option<String>, String, String, String)> =
            sqlx_core::query_as::query_as(
                "SELECT r.id::text, r.statement, r.body, r.rationale,
                        CASE WHEN a.enforcement IS NULL THEN r.enforcement::text
                             WHEN (CASE a.enforcement::text WHEN 'advisory' THEN 1 WHEN 'recommended' THEN 2 WHEN 'required' THEN 3 WHEN 'mandatory' THEN 4 ELSE 0 END)
                                > (CASE r.enforcement::text WHEN 'advisory' THEN 1 WHEN 'recommended' THEN 2 WHEN 'required' THEN 3 WHEN 'mandatory' THEN 4 ELSE 0 END)
                             THEN a.enforcement::text ELSE r.enforcement::text END,
                        COALESCE(n.scope_key, 'general'),
                        p.source
                   FROM sensei.rule_pack_adoptions a
                   JOIN sensei.rule_packs p      ON p.id = a.pack_id
                   JOIN sensei.rule_pack_rules r ON r.pack_id = p.id
                   LEFT JOIN sensei.namespaces n ON n.id = a.namespace_id
                  WHERE a.namespace_id IN (
                            SELECT namespace_id FROM sensei.folder_namespaces WHERE folder_id = $1)
                     OR a.namespace_id IN (
                            SELECT id FROM sensei.namespaces WHERE scope_key IN ('general', 'user'))
                  ORDER BY r.ordinal",
            )
            .bind(folder_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(id, title, content, impact, enforcement, scope, source)| {
                crate::governance::RawRule {
                    id,
                    title,
                    content,
                    impact,
                    enforcement,
                    scope,
                    namespace: if source.is_empty() { None } else { Some(source) },
                }
            })
            .collect())
    }

    /// The checker-backed rules that govern a folder (D-CHECKER): adopted pack
    /// rules with `verification = 'checker'` and a non-empty `checker_ref`,
    /// resolved from the same two planes as [`Self::resolve_local_pack_raws`] (the
    /// folder's namespaces plus the always-on general/user adoptions). Returns
    /// `(rule_statement, checker_ref)` — the statement is the stable handle, the
    /// checker_ref the canonical command verb to run.
    pub async fn resolve_local_checker_rules(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<(String, String)>, String> {
        let rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT r.statement, r.checker_ref
               FROM sensei.rule_pack_adoptions a
               JOIN sensei.rule_packs p      ON p.id = a.pack_id
               JOIN sensei.rule_pack_rules r ON r.pack_id = p.id
              WHERE r.verification = 'checker'
                AND r.checker_ref IS NOT NULL AND r.checker_ref <> ''
                AND ( a.namespace_id IN (
                          SELECT namespace_id FROM sensei.folder_namespaces WHERE folder_id = $1)
                      OR a.namespace_id IN (
                          SELECT id FROM sensei.namespaces WHERE scope_key IN ('general', 'user')) )
              ORDER BY r.statement",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Append a checker run to `rule_check_runs` (D-CHECKER).
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_check_run(
        &self,
        folder_id: &uuid::Uuid,
        rule_statement: &str,
        checker_ref: &str,
        command: &str,
        verdict: &str,
        exit_code: Option<i32>,
        output_tail: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_check_runs
                (folder_id, rule_statement, checker_ref, command, verdict, exit_code, output_tail)
             VALUES ($1, $2, $3, $4, $5::sensei.check_verdict, $6, $7)",
        )
        .bind(folder_id)
        .bind(rule_statement)
        .bind(checker_ref)
        .bind(command)
        .bind(verdict)
        .bind(exit_code)
        .bind(output_tail)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The governance scope ladder — `(key, name, level)` ordered most-general
    /// first (ascending level). Feeds the constitution endpoint, which groups a
    /// repo's resolved rules into one rung per scope.
    pub async fn list_scopes(&self) -> Result<Vec<(String, String, i32)>, String> {
        let rows: Vec<(String, String, i32)> = sqlx_core::query_as::query_as(
            "SELECT key, name, level FROM sensei.scopes ORDER BY level",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Resolve a user's effective behavioural stance for a repo: the most-specific
    /// namespace stance on the `sensei.scopes` ladder wins, falling back to the
    /// user's namespace-less default, then to the enum defaults (via
    /// [`crate::stance::pick_stance`]). `folder_id` is optional — with `None` (the
    /// repo isn't indexed / unknown) only the user's default row is a candidate.
    /// Daemon-local (D-STANCE-SCOPE): stance drives the local session, never a
    /// tenant-shared value.
    pub async fn resolve_stance(
        &self,
        user_key: &str,
        folder_id: Option<&uuid::Uuid>,
    ) -> Result<crate::stance::ResolvedStance, String> {
        // Candidate rows: the user's namespace-less default (level NULL) plus any
        // stance bound to a namespace this folder belongs to. The pure
        // pick_stance applies precedence, so SQL only needs to gather + tag level.
        let rows: Vec<(Option<i32>, String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT s.level, st.autonomy::text, st.sharing::text, st.review::text
               FROM sensei.stances st
               LEFT JOIN sensei.namespaces n ON n.id = st.namespace_id
               LEFT JOIN sensei.scopes s ON s.key = n.scope_key
              WHERE st.user_key = $1
                AND ( st.namespace_id IS NULL
                      OR st.namespace_id IN (
                            SELECT namespace_id FROM sensei.folder_namespaces
                             WHERE folder_id = $2 ) )",
        )
        .bind(user_key)
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let candidates: Vec<crate::stance::StanceCandidate> = rows
            .into_iter()
            .map(|(level, autonomy, sharing, review)| crate::stance::StanceCandidate {
                level,
                autonomy,
                sharing,
                review,
            })
            .collect();
        Ok(crate::stance::pick_stance(&candidates))
    }

    /// Upsert a user's stance at a scope and return the new `updated_at`.
    /// `namespace_id = None` writes the user's default row (namespace-less);
    /// `Some(ns)` writes the stance for that scope namespace. The default row and
    /// the scoped rows have different uniqueness (a partial unique index on
    /// `user_key` where `namespace_id IS NULL` vs. the `(user_key, namespace_id)`
    /// composite), so the conflict target differs by branch. Callers validate the
    /// enum fields first (via [`crate::stance::StanceInput`]).
    pub async fn upsert_stance(
        &self,
        user_key: &str,
        namespace_id: Option<&uuid::Uuid>,
        autonomy: &str,
        sharing: &str,
        review: &str,
    ) -> Result<String, String> {
        let sql = if namespace_id.is_some() {
            "INSERT INTO sensei.stances (user_key, namespace_id, autonomy, sharing, review, updated_at)
             VALUES ($1, $2, $3::sensei.stance_autonomy, $4::sensei.stance_sharing, $5::sensei.stance_review, now())
             ON CONFLICT (user_key, namespace_id) DO UPDATE SET
                autonomy = EXCLUDED.autonomy, sharing = EXCLUDED.sharing,
                review = EXCLUDED.review, updated_at = now()
             RETURNING updated_at::text"
        } else {
            // The default row: NULLs are distinct under the composite unique, so
            // target the partial unique index (user_key where namespace_id IS NULL).
            "INSERT INTO sensei.stances (user_key, namespace_id, autonomy, sharing, review, updated_at)
             VALUES ($1, $2, $3::sensei.stance_autonomy, $4::sensei.stance_sharing, $5::sensei.stance_review, now())
             ON CONFLICT (user_key) WHERE namespace_id IS NULL DO UPDATE SET
                autonomy = EXCLUDED.autonomy, sharing = EXCLUDED.sharing,
                review = EXCLUDED.review, updated_at = now()
             RETURNING updated_at::text"
        };
        let (updated_at,): (String,) = sqlx_core::query_as::query_as(sql)
            .bind(user_key)
            .bind(namespace_id)
            .bind(autonomy)
            .bind(sharing)
            .bind(review)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(updated_at)
    }

    /// The global, repo-independent ruleset: rules at the always-on `general`
    /// and `user` scopes plus genuinely-global unscoped rules (`namespace_id IS
    /// NULL AND project_id IS NULL`). These apply everywhere and are what the
    /// daemon materializes into `~/.sensei/rules.md`. A project-tied unscoped
    /// memory (a learned convention with a `project_id`) is that project's, not
    /// global, so it is deliberately excluded here — it surfaces only via
    /// [`Self::resolve_rules_raw`] for its own repo. Same ordering as
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
                    AND ( (m.namespace_id IS NULL AND m.project_id IS NULL)
                          OR n.scope_key IN ('general', 'user') )
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

    /// Active memories anchored to (slot[, feature]) for a project. `feature=None`
    /// matches project-scope (feature IS NULL); `Some(f)` matches that feature.
    pub async fn list_memories_for_slot(
        &self, project_id: &uuid::Uuid, slot: &str, feature: Option<&str>, limit: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, title, content, feature FROM sensei.memories
                  WHERE status='active' AND project_id = $1
                    AND spine_slot = $2::sensei.spine_slot
                    AND feature IS NOT DISTINCT FROM $3
                  ORDER BY strength DESC, modified_at DESC LIMIT $4"
            ).bind(project_id).bind(slot).bind(feature).bind(limit)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, content, feature)|
            serde_json::json!({ "id": id, "title": title, "content": content, "feature": feature })
        ).collect())
    }

    pub async fn assemble_context(
        &self,
        project_id: uuid::Uuid,
        stack_ids:  &[String],
        tags:       Option<&[String]>,
        limit:      i64,
        slot:       Option<(&str, Option<&str>)>,
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

        // Telemetry: log one memory_loads row per delivered memory ("did injected
        // memory help?" — loads here vs applied/ignored outcomes there). The status
        // filter above already excludes archived/rejected, so these are the same
        // memories record_outcomes_batch would accept. NON-FATAL: this is the hot
        // context-delivery path — a logging failure must warn and continue, never
        // block or error the returned context.
        let memory_ids: Vec<uuid::Uuid> = rows.iter().map(|r| r.0).collect();
        if !memory_ids.is_empty() {
            let logged = sqlx_core::query::query(
                // `FOR SHARE` pins each referenced memory row for the duration of
                // this insert, so a concurrent DELETE (which cascades to memory_loads)
                // blocks until we commit rather than racing the FK check. The CTE
                // also filters to memories that still exist — a memory already gone
                // is simply not logged, never a whole-batch FK abort.
                "WITH existing AS (
                     SELECT id FROM sensei.memories WHERE id = ANY($1::uuid[]) FOR SHARE
                 )
                 INSERT INTO activity.memory_loads (memory_id, project_id, source)
                 SELECT id, $2, 'get_layered_context' FROM existing"
            )
                .bind(&memory_ids).bind(project_id)
                .execute(&self.pool).await;
            if let Err(e) = logged {
                tracing::warn!(error = %e, count = memory_ids.len(),
                    "assemble_context: failed to log memory loads (non-fatal — context still delivered)");
            }
        }

        let mut memories: Vec<serde_json::Value> = rows.into_iter().map(|r| serde_json::json!({
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

        // Slot hint: lead the bundle with slot-anchored memories, deduped against
        // the general blend above (a slot-anchored memory that also matched the
        // scope/tag blend must not appear twice).
        if let Some((s, feature)) = slot {
            let anchored = self.list_memories_for_slot(&project_id, s, feature, limit).await?;
            if !anchored.is_empty() {
                let anchored_ids: std::collections::HashSet<String> = anchored.iter()
                    .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                    .collect();
                memories.retain(|m| {
                    m["id"].as_str().map(|id| !anchored_ids.contains(id)).unwrap_or(true)
                });
                let mut led = anchored;
                led.append(&mut memories);
                memories = led;
            }
        }

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

}
