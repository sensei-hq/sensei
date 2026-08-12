use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    pub async fn create_memory(
        &self, project_id: Option<&uuid::Uuid>, scope: &str, scope_filter: Option<&str>,
        mem_type: &str, title: &str, content: &str, impact: Option<&str>,
        session_id: Option<&uuid::Uuid>, spine_slot: Option<&str>, feature: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memories(project_id, scope, scope_filter, type, title, content, impact, session_id, spine_slot, feature)
             VALUES($1, $2::sensei.memory_scope, $3, $4::sensei.memory_type, $5, $6, $7, $8, $9::sensei.spine_slot, $10) RETURNING id"
        ).bind(project_id).bind(scope).bind(scope_filter).bind(mem_type)
            .bind(title).bind(content).bind(impact).bind(session_id)
            .bind(spine_slot).bind(feature)
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

    /// In-force adopted memories across ALL projects — powers the Observatory ·
    /// Today adopted lane. Same in-force filter as [`Self::list_active_memories`]
    /// (`status='active'`, `strength>=1.0`) but not scoped to a single
    /// project/global namespace.
    pub async fn list_active_memories_global(&self, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, f64, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT m.id, m.title, m.scope::text, m.impact, m.strength::float8, m.modified_at
             FROM sensei.memories m
             WHERE m.status = 'active' AND m.strength >= 1.0
             ORDER BY m.strength DESC, m.modified_at DESC
             LIMIT $1"
        ).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, scope, impact, strength, modified)| {
            serde_json::json!({ "id": id, "title": title, "scope": scope,
                                "impact": impact, "strength": strength,
                                "modified_at": modified.to_rfc3339() })
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

    /// Attach one piece of evidence to a memory: a session where it was learned/
    /// confirmed (`session_id = Some`), OR a save-time source note (`session_id =
    /// None`, e.g. a file:line / test / run ref supplied with the memory).
    pub async fn add_memory_evidence(&self, memory_id: &uuid::Uuid, session_id: Option<&uuid::Uuid>, note: Option<&str>) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memory_evidence(memory_id, session_id, note) VALUES($1, $2, $3) RETURNING id"
        ).bind(memory_id).bind(session_id).bind(note)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_memory_evidence(&self, memory_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, Option<uuid::Uuid>, Option<String>, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT id, session_id, note, modified_at FROM sensei.memory_evidence WHERE memory_id = $1 ORDER BY modified_at"
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

    /// Active memories offered to the corrections summarizer for linking: (id,
    /// title). Bounded; most-recent first.
    pub async fn get_learned_memories_for_matching(
        &self,
    ) -> Result<Vec<(uuid::Uuid, String)>, String> {
        let rows: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT id, title FROM sensei.memories
              WHERE status = 'active'
              ORDER BY created_at DESC
              LIMIT 100",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    pub async fn get_project_memories(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        // `strength` is `real` (Postgres 4-byte float) — sqlx decodes it as
        // `f32`, not `f64`. A mismatched decode-target quietly failed the
        // whole query and made the endpoint 500. `last_relevant_at` is
        // likewise nullable for freshly minted memories that haven't been
        // reinforced or violated, so decode as Option so a NULL doesn't
        // fail the row.
        //
        // `content`, `impact`, and the two counts power the Memory Anatomy
        // detail drawer (What / Because / Consequence + evidence). Cheap
        // to project — all existing columns on `sensei.memories`.
        // `generalised` / `generalised_content` power the ready-to-share lane:
        // the flag says sensei has rewritten this memory project-agnostic, and
        // `generalised_content` carries that portable rewrite (null until then).
        type MemRow = (
            uuid::Uuid, String, String, String, String, Option<String>,
            f32, i32, i32, String, Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            bool, Option<String>,
        );
        let rows: Vec<MemRow> = sqlx_core::query_as::query_as(
                "SELECT id, title, type::text, status::text, content, impact,
                        strength, reinforced_count, violated_count,
                        scope::text, scope_filter, last_relevant_at,
                        generalised, generalised_content
                 FROM sensei.memories WHERE project_id = $1
                 ORDER BY last_relevant_at DESC NULLS LAST LIMIT 100"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let total = rows.len();
        let active: Vec<_> = rows.into_iter()
            .filter(|r| r.3 == "active")
            .map(|(id, title, typ, status, content, impact, strength, reinforced, violated, scope, scope_filter, last, generalised, generalised_content)| {
                serde_json::json!({
                    "id": id, "title": title, "type": typ, "status": status,
                    "content": content, "impact": impact,
                    "strength": strength,
                    "reinforcedCount": reinforced,
                    "violatedCount": violated,
                    "scope": scope, "scopeFilter": scope_filter,
                    "lastRelevantAt": last.map(|t| t.to_rfc3339()),
                    "generalised": generalised,
                    "generalisedContent": generalised_content,
                })
            }).collect();

        Ok(serde_json::json!({ "active": active, "total": total }))
    }

    pub async fn insert_memory(&self, m: &InsertMemory) -> Result<uuid::Uuid, String> {
        let id: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memories
                (project_id, scope, scope_filter, type, title, content, impact,
                 tags, triage_signal, status, namespace_id, enforcement, origin, source_id,
                 spine_slot, feature)
             VALUES ($1, $2::sensei.memory_scope, $3, $4::sensei.memory_type, $5, $6, $7,
                     $8, $9, $10::sensei.memory_status, $11,
                     COALESCE($12::sensei.enforcement, 'recommended'::sensei.enforcement),
                     COALESCE($13, 'learned'), $14, $15::sensei.spine_slot, $16)
             RETURNING id"
        )
            .bind(m.project_id)
            .bind(&m.scope).bind(&m.scope_filter)
            .bind(&m.mtype).bind(&m.title).bind(&m.content).bind(&m.impact)
            .bind(&m.tags).bind(&m.triage_signal).bind(&m.status)
            .bind(m.namespace_id).bind(&m.enforcement).bind(&m.origin).bind(m.source_id)
            .bind(&m.spine_slot).bind(&m.feature)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(id.0)
    }

    /// Set the learnings-anatomy `category` on a memory (correctness/convention/
    /// pattern/preference). Separate from `insert_memory` so the existing
    /// callers (API, federation) need no change (#69).
    pub async fn set_memory_category(&self, id: &uuid::Uuid, category: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.memories SET category = $2::sensei.memory_category WHERE id = $1"
        ).bind(id).bind(category).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// True if a learned memory already sources `source_id` (a detected-pattern
    /// id). The L2 generator's idempotency guard for memories.
    pub async fn memory_exists_with_source(&self, source_id: &uuid::Uuid) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM sensei.memories WHERE source_id = $1 AND origin = 'learned')"
        ).bind(source_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Fetch the learned memory that sources `source_id` (a detected-pattern id),
    /// if any. Companion to [`Self::memory_exists_with_source`] — returns the id
    /// so a caller can act on the memory (e.g. record a challenge outcome when a
    /// recommendation built on the same pattern later regresses).
    pub async fn memory_id_by_source(&self, source_id: &uuid::Uuid) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.memories WHERE source_id = $1 AND origin = 'learned' LIMIT 1"
        ).bind(source_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// Challenge (weaken) the learned memory that sourced a now-regressed
    /// recommendation. Resolves `based_on.patterns[0]` → the convention memory
    /// (`source_id = pattern`), then records ONE `'violated'` memory_outcome so
    /// the `memory_outcome_apply` trigger does the actual strength/status math —
    /// no hand-rolled weakening here, DRY with the outcome pipeline.
    ///
    /// Idempotent: the outcome `context` carries a `rec:<id>` marker and the write
    /// is gated on that marker not already existing, so a rec that is somehow
    /// re-measured never penalises the same memory twice. Returns `Ok(true)` when
    /// a fresh violation was recorded, `Ok(false)` for the no-op paths (the rec
    /// has no source memory, was already challenged for this rec, or the memory is
    /// archived/rejected).
    pub async fn challenge_source_memory_for_rec(
        &self, rec_id: &uuid::Uuid, based_on_json: &str,
    ) -> Result<bool, String> {
        // A missing/empty/non-uuid `patterns[0]` → manual rec / no provenance → no-op.
        let Some(pattern_id) = Self::based_on_first_pattern(based_on_json) else {
            return Ok(false);
        };
        let Some(memory_id) = self.memory_id_by_source(&pattern_id).await? else {
            return Ok(false); // the rec's pattern never spawned a memory
        };
        let marker = format!("rec:{rec_id} regression");
        // Idempotency guard: skip if this rec already challenged this memory.
        let already: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM sensei.memory_outcomes
                            WHERE memory_id = $1 AND outcome = 'violated' AND context = $2)"
        ).bind(memory_id).bind(&marker).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        if already.0 {
            return Ok(false);
        }
        // The memory_outcome_apply trigger applies strength -= 0.7 and moves the
        // memory to challenged/archived. record_outcomes_batch skips archived/
        // rejected memories, so an empty `skipped` means a violation landed.
        let skipped = self.record_outcomes_batch(&[OutcomeRow {
            memory_id,
            session_id: None,
            outcome: "violated".to_string(),
            context: Some(marker),
        }]).await?;
        Ok(skipped.is_empty())
    }

    /// Learning-loop feedback (positive side): an accepted rec whose FTR IMPROVED
    /// after acceptance vindicates the memory that spawned it. Reinforce that
    /// source memory through the same `memory_outcome` pipeline the challenge path
    /// uses — recording an `applied` outcome fires the `memory_outcome_apply`
    /// trigger, which bumps `reinforced_count`, raises `strength`, and drives the
    /// promotion ladder (active → reinforced → battle_tested). This is the bridge
    /// that lets a proven recommendation promote its memory (closes G1→G2). Fires
    /// at most once per rec (idempotency marker). Mirror of
    /// [`Self::challenge_source_memory_for_rec`].
    pub async fn reinforce_source_memory_for_rec(
        &self, rec_id: &uuid::Uuid, based_on_json: &str,
    ) -> Result<bool, String> {
        // A missing/empty/non-uuid `patterns[0]` → manual rec / no provenance → no-op.
        let Some(pattern_id) = Self::based_on_first_pattern(based_on_json) else {
            return Ok(false);
        };
        let Some(memory_id) = self.memory_id_by_source(&pattern_id).await? else {
            return Ok(false); // the rec's pattern never spawned a memory
        };
        let marker = format!("rec:{rec_id} confirmed");
        // Idempotency guard: skip if this rec already reinforced this memory.
        let already: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM sensei.memory_outcomes
                            WHERE memory_id = $1 AND outcome = 'applied' AND context = $2)"
        ).bind(memory_id).bind(&marker).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        if already.0 {
            return Ok(false);
        }
        // The memory_outcome_apply trigger applies strength += 0.5 (capped 5.0),
        // reinforced_count += 1, and promotes to battle_tested at strength >= 4.0
        // with violated_count = 0.
        let skipped = self.record_outcomes_batch(&[OutcomeRow {
            memory_id,
            session_id: None,
            outcome: "applied".to_string(),
            context: Some(marker),
        }]).await?;
        Ok(skipped.is_empty())
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

    /// Persist the project-agnostic rewrite of a memory and flag it
    /// ready-to-share. Sets `generalised_content`, `generalised = true`, and
    /// bumps `modified_at`. Returns the id when a row was updated, `None` when
    /// no memory matched. Never panics — a DB error surfaces as `Err` for the
    /// caller to log; the caller only sets the flag on success (never fabricated).
    pub async fn set_memory_generalisation(
        &self,
        id: uuid::Uuid,
        generalised: &str,
    ) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "UPDATE sensei.memories
                SET generalised_content = $2
                  , generalised         = true
                  , modified_at         = now()
              WHERE id = $1
              RETURNING id"
        )
            .bind(id)
            .bind(generalised)
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

    /// Rolling 7-day telemetry for one memory: `(loaded, followed, skipped)`.
    /// - `loaded`   = load events in `activity.memory_loads` (injected into context)
    /// - `followed` = `memory_outcomes` with outcome `applied` (used in output)
    /// - `skipped`  = `memory_outcomes` with outcome `ignored` (loaded but discarded)
    ///
    /// `consulted`/`violated` are deliberately NOT folded into followed/skipped.
    /// One round-trip via scalar subqueries (loads and outcomes live in different
    /// tables) — fewer round-trips than three separate readers.
    pub async fn memory_telemetry_7d(&self, memory_id: uuid::Uuid) -> Result<(i64, i64, i64), String> {
        let row: (i64, i64, i64) = sqlx_core::query_as::query_as(
            "SELECT
               (SELECT count(*) FROM activity.memory_loads
                 WHERE memory_id = $1 AND loaded_at   > now() - interval '7 days'),
               (SELECT count(*) FROM sensei.memory_outcomes
                 WHERE memory_id = $1 AND outcome = 'applied' AND recorded_at > now() - interval '7 days'),
               (SELECT count(*) FROM sensei.memory_outcomes
                 WHERE memory_id = $1 AND outcome = 'ignored' AND recorded_at > now() - interval '7 days')"
        )
            .bind(memory_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
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
        // category + created_at fetched separately (the main row tuple is at
        // sqlx's 16-element FromRow limit).
        let (category, created_at): (Option<String>, chrono::DateTime<chrono::Utc>) =
            sqlx_core::query_as::query_as(
                "SELECT category::text, created_at FROM sensei.memories WHERE id = $1"
            ).bind(id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
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
            "category":         category,
            "created_at":       created_at.to_rfc3339(),
        });

        // Related memories (the anatomy "related" links — both directions).
        let related: Vec<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT child_id  FROM sensei.memory_links WHERE parent_id = $1
             UNION
             SELECT parent_id FROM sensei.memory_links WHERE child_id = $1"
        ).bind(id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

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

        // Rolling 7-day telemetry ("did injected memory help?"): loaded / followed
        // / skipped. Additive to the lifetime applied_count/violated_count on the
        // memory row above.
        let (loaded_7d, followed_7d, skipped_7d) = self.memory_telemetry_7d(id).await?;

        Ok(serde_json::json!({
            "memory":   memory,
            "loaded_last_7d":   loaded_7d,
            "followed_last_7d": followed_7d,
            "skipped_last_7d":  skipped_7d,
            "evidence": evidence.into_iter().map(|(session_id, note, ts)|
                serde_json::json!({ "session_id": session_id, "note": note, "recorded_at": ts.to_rfc3339() })
            ).collect::<Vec<_>>(),
            "examples": examples.into_iter().map(|(node, is_good, note)|
                serde_json::json!({ "node_id": node, "is_good": is_good, "note": note })
            ).collect::<Vec<_>>(),
            "outcomes": outcomes.into_iter().map(|(outcome, sess, ctx, ts)|
                serde_json::json!({ "outcome": outcome, "session_id": sess, "context": ctx, "recorded_at": ts.to_rfc3339() })
            ).collect::<Vec<_>>(),
            "related": related.into_iter().map(|(rid,)| rid).collect::<Vec<_>>(),
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

}
