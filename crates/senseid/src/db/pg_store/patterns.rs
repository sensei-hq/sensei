use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    /// Cache read for the insight-copy pipeline. Returns the persisted
    /// `(title, detail)` for `(kind, facts_hash)` and bumps `last_used_at`
    /// in the same statement so hot copy stays warm. `None` on cache miss or
    /// DB error (the caller then generates fresh or falls back to a static
    /// template). DB errors are logged, never swallowed silently.
    pub async fn get_insight_copy(&self, kind: &str, facts_hash: &str) -> Option<(String, String)> {
        let row: Result<Option<(String, String)>, _> = sqlx_core::query_as::query_as(
            "UPDATE sensei.insight_copy SET last_used_at = now() \
             WHERE kind = $1 AND facts_hash = $2 RETURNING title, detail"
        )
            .bind(kind)
            .bind(facts_hash)
            .fetch_optional(&self.pool)
            .await;
        match row {
            Ok(hit) => hit,
            Err(e) => {
                tracing::warn!(error = %e, kind, "get_insight_copy: DB error — treating as cache miss");
                None
            }
        }
    }

    /// Cache write for the insight-copy pipeline. Upserts the generated copy
    /// for `(kind, facts_hash)`; a newer generation wins on conflict and both
    /// timestamps reset. DB errors are logged and swallowed (the caller has
    /// already returned copy to the user — a failed cache write is not fatal).
    pub async fn upsert_insight_copy(
        &self,
        kind: &str,
        facts_hash: &str,
        title: &str,
        detail: &str,
        model_provider: Option<&str>,
        model_id: Option<&str>,
    ) {
        let res = sqlx_core::query::query(
            "INSERT INTO sensei.insight_copy \
               (kind, facts_hash, title, detail, model_provider, model_id) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (kind, facts_hash) DO UPDATE SET \
               title = EXCLUDED.title, detail = EXCLUDED.detail, \
               model_provider = EXCLUDED.model_provider, model_id = EXCLUDED.model_id, \
               generated_at = now(), last_used_at = now()"
        )
            .bind(kind)
            .bind(facts_hash)
            .bind(title)
            .bind(detail)
            .bind(model_provider)
            .bind(model_id)
            .execute(&self.pool)
            .await;
        if let Err(e) = res {
            tracing::warn!(error = %e, kind, "upsert_insight_copy: DB error — copy not cached");
        }
    }

    // ── Tags (controlled vocabulary) ──────────────────────────────────

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

    /// Accept a `pending` recommendation and carry out the action it stands for.
    ///
    /// Guards the transition to `accepted` at the `pending` state so a
    /// double-click / stale UI can't push an already-decided rec back to
    /// accepted — errors (verbatim) when the row is missing or already decided,
    /// which the HTTP handler maps to 409. Because the guard fires at most once,
    /// the action side effect below runs at most once too.
    ///
    /// A `promote_pattern` rec advances its source pattern's lifecycle to `rule`
    /// (`based_on.patterns[0]`), which the Patterns read path then renders as an
    /// `adopted` pattern. Non-atomic by design: the status flip and the
    /// lifecycle advance are two autocommit statements rather than one
    /// transaction, because reusing `promote_pattern` (DRY) precludes enrolling
    /// it in a caller-side tx without duplicating its SQL. The pending-guard
    /// already makes re-promotion impossible, so the only failure window —
    /// status flipped, promote failed — is a logged inconsistency (surfaced at
    /// error level), never a double-write.
    pub async fn accept_recommendation(&self, id: &uuid::Uuid) -> Result<(), String> {
        let row: Option<(String, String)> = sqlx_core::query_as::query_as(
            "UPDATE inference.recommendations
                SET status = 'accepted'::sensei.recommendation_status,
                    acted_at = now()
              WHERE id = $1 AND status = 'pending'
          RETURNING action_type, based_on::text"
        ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        let Some((action_type, based_on)) = row else {
            return Err("recommendation not found or already decided".into());
        };

        // A missing/empty/non-uuid `based_on.patterns[0]` yields None here, so a
        // promote_pattern rec with no provenance short-circuits to a no-op.
        if action_type == "promote_pattern"
            && let Some(pattern_id) = Self::based_on_first_pattern(&based_on)
            && let Err(e) = self.promote_pattern(&pattern_id, "rule").await
        {
            // The status flip already committed; the guard blocks a
            // retry-promotion, so log loudly rather than swallow — the
            // rec IS accepted, only the lifecycle advance was lost.
            tracing::error!(
                error = %e, recommendation = %id, pattern = %pattern_id,
                "accept_recommendation: pattern promotion failed after status flip"
            );
        }
        Ok(())
    }

    /// The `action_type`s that materialize a governance RULE on accept (spec
    /// 2026-08-20 insight-acceptance-materialization, P-A). Everything else falls
    /// back to the plain [`Self::accept_recommendation`] status flip.
    pub fn is_rule_class_action(action_type: &str) -> bool {
        matches!(action_type, "revise_rule" | "promote_pattern" | "enrich_memory")
    }

    /// Read a pending recommendation's fields for a materialization preview WITHOUT
    /// deciding it: `(action_type, title, why, impact, project_id, based_on)`.
    /// `None` when the rec is absent or already decided (so a stale UI previews
    /// nothing rather than a fabricated artifact).
    pub async fn recommendation_for_materialize(
        &self, id: &uuid::Uuid,
    ) -> Result<Option<(String, String, String, Option<String>, uuid::Uuid, String)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT action_type, title, why, impact, project_id, based_on::text
               FROM inference.recommendations
              WHERE id = $1 AND status = 'pending'",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    /// The `prompt` (ready-to-send instruction; for `create_agent` it's a full agent
    /// system-prompt) of a recommendation — the body seed a file materialization
    /// (P-B) renders into a SKILL.md / agent `.md`. `None` when absent or unset.
    pub async fn recommendation_prompt(&self, id: &uuid::Uuid) -> Result<Option<String>, String> {
        let row: Option<(Option<String>,)> = sqlx_core::query_as::query_as(
            "SELECT prompt FROM inference.recommendations WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.and_then(|(p,)| p))
    }

    /// Accept a rule-class recommendation AND materialize it into a durable
    /// governance rule — a `sensei.memories` row at `namespace_id` (the resolved
    /// scope; `None` = the always-on general/user rung) with `enforcement`
    /// (default `recommended`), which immediately enters live rule resolution
    /// (`get_rules` / the SessionStart push). `title`/`content` default to the
    /// rec's `title`/`why`; callers may override (the review-before-apply edit).
    /// Records `materialized_ref` on the rec so the effect can be measured + undone.
    ///
    /// Order mirrors [`Self::accept_recommendation`]'s documented tradeoff: flip the
    /// status under the pending-guard FIRST (idempotent — a double click 409s), then
    /// materialize. A materialize failure AFTER the flip is logged loudly (the rec
    /// is accepted, the rule write is lost + recoverable), never a silent swallow —
    /// and never a fabricated rule. Returns the `materialized_ref` written, or
    /// `Err("... not rule-class")` when the caller mis-routed a non-rule action.
    pub async fn accept_recommendation_as_rule(
        &self,
        id: &uuid::Uuid,
        namespace_id: Option<&uuid::Uuid>,
        enforcement: Option<&str>,
        gov_scope: &str,
        title_override: Option<&str>,
        body_override: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let row: Option<(String, String, String, Option<String>, uuid::Uuid, String)> =
            sqlx_core::query_as::query_as(
                "UPDATE inference.recommendations
                    SET status = 'accepted'::sensei.recommendation_status, acted_at = now()
                  WHERE id = $1 AND status = 'pending'
              RETURNING action_type, title, why, impact, project_id, based_on::text",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        let Some((action_type, title, why, impact, project_id, based_on)) = row else {
            return Err("recommendation not found or already decided".into());
        };
        if !Self::is_rule_class_action(&action_type) {
            // The caller should have routed this to the plain accept; the status is
            // already flipped, so surface the mis-route rather than fabricate a rule.
            return Err(format!("recommendation action_type '{action_type}' is not rule-class"));
        }

        // A rule needs a body. Recs always carry a non-empty `why`; an override wins.
        let content = body_override
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| why.trim().to_string());
        let rule_title = title_override
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(title.trim())
            .to_string();
        // Legacy memory_scope mirrors the governance scope for the older read paths;
        // the real binding is namespace_id + enforcement.
        let mem_scope = match gov_scope {
            "technology" | "team" | "organization" | "client" => "stack",
            "general" | "user" => "global",
            _ => "project",
        };

        let mem = crate::db::pg_store::InsertMemory {
            project_id: Some(project_id),
            scope: mem_scope.to_string(),
            scope_filter: None,
            mtype: "convention".to_string(),
            title: rule_title,
            content,
            impact,
            tags: vec!["accepted-recommendation".to_string()],
            triage_signal: None,
            status: "active".to_string(), // an accepted rule is live immediately
            namespace_id: namespace_id.copied(),
            enforcement: enforcement.map(str::to_string),
            origin: Some("authored".to_string()), // the user accepted → authored
            source_id: None,
            spine_slot: None,
            feature: None,
        };
        let memory_id = self.insert_memory(&mem).await.map_err(|e| {
            tracing::error!(error = %e, recommendation = %id, "accept_recommendation_as_rule: memory insert failed AFTER status flip — rec accepted, rule lost");
            e
        })?;

        // promote_pattern also advances the source pattern's lifecycle (preserve the
        // legacy side-effect) — best-effort, never blocks the rule write.
        if action_type == "promote_pattern"
            && let Some(pattern_id) = Self::based_on_first_pattern(&based_on)
            && let Err(e) = self.promote_pattern(&pattern_id, "rule").await
        {
            tracing::error!(error = %e, recommendation = %id, pattern = %pattern_id, "accept_recommendation_as_rule: pattern lifecycle advance failed (rule still written)");
        }

        let materialized = serde_json::json!({
            "kind": "rule",
            "memory_id": memory_id,
            "namespace_id": namespace_id,
            "scope": gov_scope,
            "enforcement": enforcement.unwrap_or("recommended"),
        });
        sqlx_core::query::query(
            "UPDATE inference.recommendations SET materialized_ref = $2 WHERE id = $1",
        )
        .bind(id)
        .bind(&materialized)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(materialized)
    }

    /// Flip a pending recommendation to `accepted` (guarded, idempotent) and RETURN
    /// the seed a file-materializer (P-B skill/agent) needs: `(action_type, title,
    /// why, prompt)`. The file write + `materialized_ref` happen in the handler
    /// (I/O lives outside the store); [`Self::set_recommendation_materialized`]
    /// records the ref after the file lands. Mirrors the accept-then-side-effect
    /// tradeoff of [`Self::accept_recommendation`]. `None` ⇒ absent/already decided.
    pub async fn begin_file_materialization(
        &self, id: &uuid::Uuid,
    ) -> Result<Option<(String, String, String, Option<String>)>, String> {
        sqlx_core::query_as::query_as(
            "UPDATE inference.recommendations
                SET status = 'accepted'::sensei.recommendation_status, acted_at = now()
              WHERE id = $1 AND status = 'pending'
          RETURNING action_type, title, why, prompt",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    /// Record what an accept materialized (the `materialized_ref` provenance) on an
    /// already-accepted recommendation. Called after a file write (P-B) so the ref
    /// points at the written path. Idempotent set.
    pub async fn set_recommendation_materialized(
        &self, id: &uuid::Uuid, materialized: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE inference.recommendations SET materialized_ref = $2 WHERE id = $1",
        )
        .bind(id)
        .bind(materialized)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Move a `pending` recommendation to `dismissed` (the reject terminal —
    /// the enum uses `dismissed`, not `rejected`). Same shape as accept:
    /// idempotency-guarded so a stale UI can't clobber a real decision.
    pub async fn reject_recommendation(&self, id: &uuid::Uuid) -> Result<(), String> {
        let result = sqlx_core::query::query(
            "UPDATE inference.recommendations
                SET status = 'dismissed'::sensei.recommendation_status,
                    acted_at = now()
              WHERE id = $1 AND status = 'pending'"
        ).bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        if result.rows_affected() == 0 {
            return Err("recommendation not found or already decided".into());
        }
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

    /// Insert a recommendation with provenance (#69 L2 generator). `based_on`
    /// links the L1/L2 artifacts reasoned over (`{patterns,memories,corrections}`),
    /// distinct from raw session/file `evidence`. Used for idempotency.
    pub async fn create_recommendation_full(
        &self, project_id: &uuid::Uuid, title: &str, why: &str, impact: Option<&str>,
        action_type: &str, urgency: &str, based_on: &serde_json::Value,
        reasoning_trace_id: Option<&uuid::Uuid>, prompt: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.recommendations(project_id, title, why, impact, action_type, urgency, based_on, reasoning_trace_id, prompt)
             VALUES($1, $2, $3, $4, $5, $6::sensei.recommendation_urgency, $7::jsonb, $8, $9) RETURNING id"
        ).bind(project_id).bind(title).bind(why).bind(impact).bind(action_type).bind(urgency)
            .bind(based_on.to_string()).bind(reasoning_trace_id).bind(prompt)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// True if any recommendation for `project_id` already cites `pattern_id` in
    /// `based_on.patterns`. The L2 generator's idempotency guard.
    pub async fn recommendation_exists_for_pattern(
        &self, project_id: &uuid::Uuid, pattern_id: &uuid::Uuid,
    ) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(
               SELECT 1 FROM inference.recommendations
                WHERE project_id = $1 AND based_on->'patterns' @> to_jsonb($2::text)
             )"
        ).bind(project_id).bind(pattern_id.to_string())
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    // ── Recommendation ranking (ranking.rs) ──────────────────────────

    /// Pending recs for a project with the scoring factors joined from their
    /// source patterns (`based_on.patterns` → `detected_patterns`): returns
    /// `(id, action_type, urgency, avg_confidence, max_recurrence)`. A rec with
    /// no joinable pattern yields `avg_confidence = None`, `max_recurrence = 0`.
    pub async fn get_pending_recs_for_ranking(
        &self, project_id: &uuid::Uuid,
    ) -> Result<Vec<(uuid::Uuid, String, String, Option<f64>, i32)>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<f64>, i32)> = sqlx_core::query_as::query_as(
            "SELECT r.id, r.action_type, r.urgency::text,
                    avg(dp.confidence)::float8 AS avg_conf,
                    COALESCE(max(dp.instance_count), 0)::int4 AS max_recur
               FROM inference.recommendations r
               LEFT JOIN LATERAL jsonb_array_elements_text(
                     CASE WHEN jsonb_typeof(r.based_on->'patterns') = 'array'
                          THEN r.based_on->'patterns' ELSE '[]'::jsonb END
                   ) AS pid(v) ON true
               LEFT JOIN inference.detected_patterns dp ON dp.id = pid.v::uuid
              WHERE r.project_id = $1 AND r.status = 'pending'
              GROUP BY r.id, r.action_type, r.urgency",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Clear the focal flag across a project (before a fresh ranking pass marks a
    /// new one) so a previously-focal rec that has since been acted on or
    /// out-scored never stays flagged.
    pub async fn clear_project_focal(&self, project_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE inference.recommendations SET focal = false WHERE project_id = $1 AND focal",
        )
        .bind(project_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Persist a rec's computed `score` + `focal`, mirroring the factor
    /// breakdown into `based_on.score_factors` for explainability.
    pub async fn set_recommendation_rank(
        &self, id: &uuid::Uuid, score: f64, focal: bool, factors: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE inference.recommendations
                SET score = $2::float8::numeric(5,2),
                    focal = $3,
                    based_on = jsonb_set(based_on, '{score_factors}', $4::jsonb, true)
              WHERE id = $1",
        )
        .bind(id)
        .bind(score)
        .bind(focal)
        .bind(factors.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Communities (inference) ───────────────────────────────────────

    pub async fn insert_reasoning_trace(
        &self, project_id: Option<&uuid::Uuid>, trigger_event: &str, trigger_detail: &serde_json::Value,
        models_used: &[String], exchanges: &serde_json::Value, consensus: &serde_json::Value,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.reasoning_traces(project_id, trigger_event, trigger_detail, models_used, exchanges, consensus) VALUES($1, $2, $3, $4, $5, $6) RETURNING id"
        ).bind(project_id).bind(trigger_event).bind(trigger_detail).bind(models_used).bind(exchanges).bind(consensus)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// True if a reasoning trace for `project_id` already carries this
    /// finding-set `signature` (in `trigger_detail`). The consolidation tier's
    /// idempotency guard — keeps the LLM call from re-firing on the same signals.
    pub async fn reasoning_trace_exists_with_signature(&self, project_id: &uuid::Uuid, signature: &str) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM inference.reasoning_traces WHERE project_id = $1 AND trigger_detail->>'signature' = $2)"
        ).bind(project_id).bind(signature).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Impact reports (#70 read-path): recommendations that have been acted on
    /// or carry a consolidation trace, joined to that trace. Powers the
    /// Observatory Impact view (before/after FTR + the MOE-style reasoning).
    pub async fn get_project_impact(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, Option<f64>, Option<f64>, serde_json::Value, Option<Vec<String>>, Option<serde_json::Value>)> =
            sqlx_core::query_as::query_as(
                "SELECT r.id, r.title, r.action_type, r.status::text, r.verdict::text,
                        r.baseline_ftr::float8, r.current_ftr::float8, r.props,
                        t.models_used, t.consensus
                   FROM inference.recommendations r
                   LEFT JOIN inference.reasoning_traces t ON t.id = r.reasoning_trace_id
                  WHERE r.project_id = $1
                    AND (r.reasoning_trace_id IS NOT NULL OR r.verdict <> 'pending'::sensei.recommendation_verdict)
                  ORDER BY r.measured_at DESC NULLS LAST"
            ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, action_type, status, verdict, baseline, current, props, models, consensus)| {
            // The reasoning field carries the honest single-verdict JSON to the
            // UI: `{headline, body, modelsUsed: string[], suggestedRevision}`
            // when measure has populated it, or null when the rec has no trace
            // yet. HONEST SINGLE VERDICT (#109 audit): no fabricated consensus
            // tally or per-model panelist verdicts — there is one FTR-delta
            // verdict, and `modelsUsed` lists the models that actually ran.
            let reasoning = consensus.map(|synth| {
                // The honest synth is marked by `headline` — flow it straight through.
                if synth.get("headline").is_some() {
                    synth
                } else {
                    // Legacy/old-shape trace (e.g. `{conclusion}` from the retired
                    // consensus path). Surface the REAL model names from the trace;
                    // never fabricate per-model roles/notes/verdicts.
                    let conclusion = synth.get("conclusion")
                        .and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let names = models.clone().unwrap_or_default();
                    serde_json::json!({
                        "headline":          if conclusion.is_empty() { "Reasoning captured (no narrative)".into() } else { conclusion },
                        "body":              serde_json::Value::Null,
                        "modelsUsed":        names,
                        "suggestedRevision": serde_json::Value::Null,
                    })
                }
            });

            serde_json::json!({
                "id": id, "title": title, "actionType": action_type, "status": status,
                "verdict": verdict, "baselineFtr": baseline, "currentFtr": current,
                "ftrDelta": match (current, baseline) { (Some(c), Some(b)) => Some(((c - b) * 1000.0).round() / 1000.0), _ => None },
                "props": props,
                "reasoning": reasoning,
            })
        }).collect())
    }

    // ── Tool insights cache (T2 Slice D) ─────────────────────────────────

    /// Append a snapshot row for one tool. Called by the
    /// `AggregateToolInsights` task once per tool per tick. Historical rows
    /// stay in place so a follow-up trend chart can walk `computed_at` back
    /// in time.
    pub async fn insert_tool_insight(
        &self,
        tool_name: &str,
        metrics: &serde_json::Value,
        signal: Option<&crate::api::handlers::tool_signals::Signal>,
    ) -> Result<(), String> {
        use crate::tasks::handlers::tool_insights::variant_str;
        let (variant, title, detail) = match signal {
            Some(s) => (Some(variant_str(s.variant)), Some(s.title.as_str()), Some(s.detail.as_str())),
            None => (None, None, None),
        };
        sqlx_core::query::query(
            "INSERT INTO sensei.tool_insights
                (tool_name, metrics, signal_variant, signal_title, signal_detail)
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(tool_name)
        .bind(metrics)
        .bind(variant)
        .bind(title)
        .bind(detail)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Read the latest cached insight per tool. Returns `(tool_name,
    /// computed_at, metrics, signal_variant, signal_title, signal_detail)`
    /// tuples ordered by variant priority (warn > opportunity > unused >
    /// win > null) so the caller can render them straight through.
    pub async fn get_latest_tool_insights(
        &self,
    ) -> Result<Vec<serde_json::Value>, String> {
        // DISTINCT ON (tool_name) with ORDER BY tool_name, computed_at DESC
        // is the compact "latest row per tool" trick — Postgres picks the
        // first tuple per group. Wrapped in an outer SELECT so we can add
        // the variant-priority ordering the endpoint expects.
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            String,                                // tool_name
            chrono::DateTime<chrono::Utc>,         // computed_at
            serde_json::Value,                     // metrics
            Option<String>,                        // variant
            Option<String>,                        // title
            Option<String>,                        // detail
        )> = sqlx_core::query_as::query_as(
            "SELECT tool_name, computed_at, metrics,
                    signal_variant, signal_title, signal_detail
               FROM (
                 SELECT DISTINCT ON (tool_name)
                        tool_name, computed_at, metrics,
                        signal_variant, signal_title, signal_detail
                   FROM sensei.tool_insights
                  ORDER BY tool_name, computed_at DESC
               ) latest
              ORDER BY CASE signal_variant
                         WHEN 'warn'        THEN 0
                         WHEN 'opportunity' THEN 1
                         WHEN 'unused'      THEN 2
                         WHEN 'win'         THEN 3
                         ELSE 4
                       END,
                       computed_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(tool_name, computed_at, metrics, variant, title, detail)| {
            serde_json::json!({
                "toolName":   tool_name,
                "computedAt": computed_at.to_rfc3339(),
                "metrics":    metrics,
                "variant":    variant,
                "title":      title,
                "detail":     detail,
            })
        }).collect())
    }

    // ── Doc-drift scan (T3 Slice 2.3) ────────────────────────────────────

    /// List manual impact-verdict entries for a project, newest first.
    /// Optional `verdict` filter narrows to one lifecycle stage (`pending`,
    /// `success`, `mixed`, `failure`).
    pub async fn list_impact_verdicts(
        &self,
        project_id: &uuid::Uuid,
        verdict: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, session_id, title, note, verdict::text, created_at, decided_at
                   FROM sensei.impact_verdicts
                  WHERE project_id = $1
                    AND ($2::text IS NULL OR verdict::text = $2)
                  ORDER BY created_at DESC
                  LIMIT 200"
            )
            .bind(project_id)
            .bind(verdict)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, session_id, title, note, verdict, created_at, decided_at)| {
            serde_json::json!({
                "id":         id,
                "sessionId":  session_id,
                "title":      title,
                "note":       note,
                "verdict":    verdict,
                "createdAt":  created_at.to_rfc3339(),
                "decidedAt":  decided_at.map(|t| t.to_rfc3339()),
            })
        }).collect())
    }

    /// Log a new impact entry. Verdict defaults to `pending`; the caller
    /// hits `set_impact_verdict_outcome` later to record the outcome.
    pub async fn create_impact_verdict(
        &self,
        project_id: &uuid::Uuid,
        title: &str,
        note: Option<&str>,
        session_id: Option<&uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        if title.trim().is_empty() {
            return Err("title required".into());
        }
        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.impact_verdicts (project_id, title, note, session_id)
             VALUES ($1, $2, $3, $4) RETURNING id"
        )
        .bind(project_id)
        .bind(title)
        .bind(note)
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// Assign a terminal verdict (`success` | `mixed` | `failure`) to a
    /// pending impact log entry. Stamps `decided_at = now()`. Errors when
    /// the entry doesn't exist or has already been decided.
    pub async fn set_impact_verdict_outcome(
        &self,
        verdict_id: &uuid::Uuid,
        outcome: &str,
        note: Option<&str>,
    ) -> Result<(), String> {
        if !matches!(outcome, "success" | "mixed" | "failure") {
            return Err(format!("invalid verdict {outcome}"));
        }
        let result = sqlx_core::query::query(
            "UPDATE sensei.impact_verdicts
                SET verdict = $1::sensei.impact_verdict,
                    note = COALESCE($2, note),
                    decided_at = now()
              WHERE id = $3
                AND verdict = 'pending'"
        )
        .bind(outcome)
        .bind(note)
        .bind(verdict_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        if result.rows_affected() == 0 {
            return Err("verdict not found or already decided".into());
        }
        Ok(())
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

    /// Upsert a detected pattern at PROJECT scope (#82). `folder_id` is
    /// preserved as an optional locus pointer for file/folder-scoped signals
    /// (churn); it is not part of the uniqueness key. Passing the same
    /// (project_id, name, is_anti_pattern) with a different folder_id
    /// updates the same row and overwrites the locus — that's the desired
    /// merge behaviour when a single file's pattern shows up across sibling
    /// folders inside the project.
    pub async fn upsert_pattern(
        &self, project_id: &uuid::Uuid, folder_id: Option<&uuid::Uuid>,
        name: &str, is_anti: bool,
        confidence: Option<f64>, instances: &serde_json::Value,
    ) -> Result<uuid::Uuid, String> {
        let count = instances.as_array().map(|a| a.len() as i32).unwrap_or(0);
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.detected_patterns(project_id, folder_id, name, is_anti_pattern, confidence, instance_count, instances)
             VALUES($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT(project_id, name, is_anti_pattern) DO UPDATE SET
               folder_id = COALESCE(EXCLUDED.folder_id, detected_patterns.folder_id),
               confidence = COALESCE(EXCLUDED.confidence, detected_patterns.confidence),
               instance_count = EXCLUDED.instance_count,
               instances = EXCLUDED.instances,
               modified_at = now()
             RETURNING id"
        ).bind(project_id).bind(folder_id).bind(name).bind(is_anti).bind(confidence).bind(count).bind(instances)
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

    /// Patterns a symbol participates in — FILE-level membership. `detected_patterns`
    /// records file `instances`, not per-symbol members, so we match the symbol's
    /// file: resolve nodes named `symbol` in the project's folders, then return the
    /// project's patterns whose `instances[].file` is that node's file. `nodes.file_path`
    /// is repo-RELATIVE and `instances[].file` is ABSOLUTE, so the match is an
    /// equality-or-path-suffix. `[]` when the symbol's file is in no pattern
    /// (honest-empty, NOT the old always-null mask that read a nonexistent `members`).
    pub async fn patterns_for_symbol(
        &self, project_id: &uuid::Uuid, folder_ids: &[uuid::Uuid], symbol: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        if folder_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows: Vec<(uuid::Uuid, String, Option<String>, String, bool, Option<f64>, i32)> =
            sqlx_core::query_as::query_as(
                "SELECT DISTINCT p.id, p.name, p.family, p.lifecycle::text, p.is_anti_pattern, p.confidence::float8, p.instance_count
                   FROM inference.detected_patterns p
                  WHERE p.project_id = $1
                    AND EXISTS (
                        SELECT 1 FROM sensei.nodes n
                        JOIN jsonb_array_elements(p.instances) e
                          ON (e->>'file' = n.file_path OR e->>'file' LIKE '%/' || n.file_path)
                        WHERE n.folder_id = ANY($2::uuid[]) AND n.name = $3 AND n.file_path <> ''
                    )
                  ORDER BY p.instance_count DESC",
            )
            .bind(project_id).bind(folder_ids).bind(symbol)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, family, lc, anti, conf, count)| {
            serde_json::json!({
                "id": id, "name": name, "family": family, "lifecycle": lc,
                "is_anti_pattern": anti, "confidence": conf, "instance_count": count,
            })
        }).collect())
    }

    /// Read a project's detected patterns for the L2 generator: `(id, folder_id,
    /// folder_label, name, is_anti_pattern, instance_count, instances_json_text)`.
    /// `instances` is returned as text (parsed by the caller) to avoid a sqlx
    /// json-feature dependency.
    ///
    /// Attribution matches L1 (`derive_signals`): patterns belong to a project
    /// via the **folders that have sessions for that project** (`sessions.project_id`),
    /// not `folders.project_id` — the two can diverge, and L1 keys off the
    /// session path, so the generator must read the same set.
    pub async fn get_patterns_for_generation(
        &self, project_id: &uuid::Uuid,
    ) -> Result<Vec<(uuid::Uuid, uuid::Uuid, String, String, bool, i32, String)>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, String, String, bool, i32, String)> =
            sqlx_core::query_as::query_as(
                "SELECT dp.id, dp.folder_id, COALESCE(f.name, ''), dp.name, dp.is_anti_pattern, dp.instance_count, dp.instances::text
                   FROM inference.detected_patterns dp
                   JOIN sensei.folders f ON f.id = dp.folder_id
                  WHERE dp.folder_id IN (
                          SELECT DISTINCT folder_id FROM activity.sessions
                           WHERE project_id = $1 AND folder_id IS NOT NULL
                        )
                  ORDER BY dp.instance_count DESC, dp.id"
            ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    // ── Corrections aggregation (#65 step 5) ─────────────────────────────────

    /// All captured user prompts across every project: (project_id, project_name,
    /// session_id, ts_ms, prompt). Ordered by ts so the handler's clustering seeds
    /// on the earliest member. The handler filters to corrections.
    pub async fn get_all_user_prompts(
        &self,
    ) -> Result<Vec<(uuid::Uuid, String, String, i64, String)>, String> {
        let rows: Vec<(uuid::Uuid, String, String, i64, String)> = sqlx_core::query_as::query_as(
            "SELECT s.project_id, COALESCE(p.name, ''), ae.session_id, ae.ts, ae.payload->>'prompt'
               FROM activity.assistant_events ae
               JOIN activity.sessions s ON s.client_session_id = ae.session_id
               JOIN sensei.projects p ON p.id = s.project_id
              WHERE ae.event_type = 'UserPromptSubmit'
                AND ae.payload->>'prompt' IS NOT NULL
                AND s.project_id IS NOT NULL
              ORDER BY ae.ts",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Upsert one aggregated correction, keyed by its stable `signature` (so `id`
    /// stays constant across re-derivations).
    pub async fn upsert_correction(
        &self,
        row: &crate::corrections::CorrectionRow,
    ) -> Result<uuid::Uuid, String> {
        let r: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.corrections
                (signature, text, suggestion, count, project_ids, last_seen, memory_id, instances, modified_at)
             VALUES($1, $2, $3, $4, $5, $6, $7, $8, now())
             ON CONFLICT(signature) DO UPDATE SET
               text = EXCLUDED.text,
               suggestion = EXCLUDED.suggestion,
               count = EXCLUDED.count,
               project_ids = EXCLUDED.project_ids,
               last_seen = EXCLUDED.last_seen,
               memory_id = EXCLUDED.memory_id,
               instances = EXCLUDED.instances,
               modified_at = now()
             RETURNING id",
        )
        .bind(&row.signature)
        .bind(&row.text)
        .bind(&row.suggestion)
        .bind(row.count)
        .bind(&row.project_ids)
        .bind(row.last_seen)
        .bind(row.memory_id)
        .bind(&row.instances)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(r.0)
    }

    /// Delete corrections whose signature is not in `keep`. With an empty slice
    /// this clears the table (no corrections currently recur). Returns row count.
    pub async fn delete_corrections_not_in(&self, keep: &[String]) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "DELETE FROM inference.corrections WHERE signature <> ALL($1)",
        )
        .bind(keep)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    /// Global corrections list (camelCase, projects resolved to {id, name}).
    pub async fn list_corrections(&self) -> Result<serde_json::Value, String> {
        self.query_corrections(None).await
    }

    /// Corrections touching a specific project.
    pub async fn list_corrections_for_project(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<serde_json::Value, String> {
        self.query_corrections(Some(project_id)).await
    }

    pub async fn get_pending_recommendations(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<String>, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT id, urgency::text, title, why, impact, evidence
             FROM inference.recommendations
             WHERE project_id = $1 AND status = 'pending'
             ORDER BY CASE urgency WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, id
             LIMIT 10"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, urgency, title, why, impact, evidence)| {
            serde_json::json!({ "id": id, "urgency": urgency, "title": title,
                                "why": why, "impact": impact, "evidence": evidence })
        }).collect())
    }

    /// Highest-priority pending recommendations across all projects — powers the
    /// Observatory · Today hero + insight strip. Mirrors
    /// [`Self::get_pending_recommendations`] without the project filter.
    pub async fn get_pending_recommendations_global(&self, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<String>, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT id, urgency::text, title, why, impact, evidence
             FROM inference.recommendations
             WHERE status = 'pending'
             ORDER BY CASE urgency WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, id
             LIMIT $1"
        ).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, urgency, title, why, impact, evidence)| {
            serde_json::json!({ "id": id, "urgency": urgency, "title": title,
                                "why": why, "impact": impact, "evidence": evidence })
        }).collect())
    }

    // ── Insights (Learnings Triage) aggregator sources (#Slot 5) ──────────
    // Each carries `project_id` so the UI can call the per-project
    // accept/reject action; `project` is None for the cross-project view.

    /// Pending recommendations + their project name, ordered high→low urgency.
    /// Capped: the triage screen shows the highest-urgency first (Now/Soon are
    /// complete; low-urgency Settled recs beyond the cap fall off the shelf).
    pub async fn get_insights_recommendations(&self, project: Option<&uuid::Uuid>) -> Result<Vec<serde_json::Value>, String> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(uuid::Uuid, String, String, String, Option<String>, serde_json::Value, Option<uuid::Uuid>, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT r.id, r.urgency::text, r.title, r.why, r.impact, r.evidence, r.project_id, p.name
                 FROM inference.recommendations r
                 LEFT JOIN sensei.projects p ON p.id = r.project_id
                 WHERE r.status = 'pending' AND ($1::uuid IS NULL OR r.project_id = $1)
                 ORDER BY CASE r.urgency WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, r.id
                 LIMIT 200"
            ).bind(project).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        // No silent caps: the Insights board shows the top-200 pending recs by
        // urgency; if the cap is hit, lower-urgency recs are not surfaced. Log it
        // so the truncation is observable (a "showing N of M" UI hint is a follow-up).
        if rows.len() >= 200 {
            tracing::warn!(returned = rows.len(),
                "get_insights_recommendations hit the 200-row cap — lower-urgency pending recs are not surfaced on the triage board");
        }
        Ok(rows.into_iter().map(|(id, urgency, title, why, impact, evidence, project_id, name)| {
            serde_json::json!({ "id": id, "urgency": urgency, "title": title, "why": why,
                                "impact": impact, "evidence": evidence,
                                "project_id": project_id, "name": name })
        }).collect())
    }

    /// Memories eligible for the triage screen: proposed, in-force, or violated
    /// (non-archived). Column assignment happens in `crate::insights`.
    pub async fn get_insights_memories(&self, project: Option<&uuid::Uuid>) -> Result<Vec<serde_json::Value>, String> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, i32, Option<f64>, String, Option<uuid::Uuid>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, status::text, title, content, violated_count, strength::float8, scope::text, project_id
                 FROM sensei.memories
                 WHERE ($1::uuid IS NULL OR project_id = $1)
                   AND ( status IN ('proposed','active','reinforced','battle_tested')
                         OR (violated_count > 0 AND status != 'archived') )
                 ORDER BY strength DESC NULLS LAST
                 LIMIT 100"
            ).bind(project).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, status, title, content, violated_count, strength, scope, project_id)| {
            serde_json::json!({ "id": id, "status": status, "title": title, "content": content,
                                "violated_count": violated_count, "strength": strength,
                                "scope": scope, "project_id": project_id })
        }).collect())
    }

    /// Suggested + rule patterns for the triage screen (anti-patterns excluded).
    pub async fn get_insights_patterns(&self, project: Option<&uuid::Uuid>) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, String, i32, Option<uuid::Uuid>)> =
            sqlx_core::query_as::query_as(
                "SELECT dp.id, dp.name, dp.family, dp.lifecycle::text, dp.instance_count, f.project_id
                 FROM inference.detected_patterns dp
                 JOIN sensei.folders f ON f.id = dp.folder_id
                 WHERE dp.lifecycle IN ('suggested','rule') AND NOT dp.is_anti_pattern
                   AND ($1::uuid IS NULL OR f.project_id = $1)
                 ORDER BY dp.instance_count DESC
                 LIMIT 100"
            ).bind(project).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, family, lifecycle, instance_count, project_id)| {
            serde_json::json!({ "id": id, "name": name, "family": family, "lifecycle": lifecycle,
                                "instance_count": instance_count, "project_id": project_id })
        }).collect())
    }

    /// Top recurring corrections by count → the Now column. `project` scopes via
    /// the `project_ids` array membership.
    pub async fn get_insights_corrections(&self, project: Option<&uuid::Uuid>, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, i32)> = sqlx_core::query_as::query_as(
            "SELECT id, text, suggestion, count
             FROM inference.corrections
             WHERE ($1::uuid IS NULL OR $1 = ANY(project_ids))
             ORDER BY count DESC LIMIT $2"
        ).bind(project).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, text, suggestion, count)| {
            serde_json::json!({ "id": id, "text": text, "suggestion": suggestion, "count": count })
        }).collect())
    }

    pub async fn get_adopted_teachings(&self, project_id: &uuid::Uuid, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, i32, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT dp.id, dp.name, dp.family, dp.instance_count, dp.modified_at
             FROM inference.detected_patterns dp
             JOIN sensei.folders f ON f.id = dp.folder_id
             WHERE f.project_id = $1 AND dp.lifecycle = 'rule' AND NOT dp.is_anti_pattern
             ORDER BY dp.modified_at DESC LIMIT $2"
        ).bind(project_id).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, family, count, modified)| {
            serde_json::json!({ "id": id, "name": name, "family": family,
                                "instance_count": count, "modified_at": modified.to_rfc3339() })
        }).collect())
    }

    // ── Sessions (activity) ────────────────────────────────────────────

    /// True if a pending recommendation already proposes `model` for this project
    /// (the model-insight generator's idempotency guard).
    pub async fn model_recommendation_exists(
        &self, project_id: &uuid::Uuid, model: &str,
    ) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(
               SELECT 1 FROM inference.recommendations
                WHERE project_id = $1 AND status = 'pending'
                  AND based_on->>'recommended_model' = $2
             )",
        )
        .bind(project_id)
        .bind(model)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Top pending recommendation for a project — highest urgency, then id —
    /// including `default_acp` for the Overview hero's "send to {acp}" action.
    /// `None` when the project has no pending recommendation.
    pub async fn get_top_recommendation(&self, project_id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, serde_json::Value, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, title, why, evidence, default_acp
             FROM inference.recommendations
             WHERE project_id = $1 AND status = 'pending'
             ORDER BY CASE urgency WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, id
             LIMIT 1"
        ).bind(project_id).fetch_optional(&self.pool).await
            .map_err(|e| { tracing::error!(error = %e, "get_top_recommendation failed"); e.to_string() })?;
        Ok(row.map(|(id, title, why, evidence, default_acp)| serde_json::json!({
            "id": id, "title": title, "why": why, "evidence": evidence, "defaultAcp": default_acp,
        })))
    }

    pub async fn get_project_patterns(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        // Project baseline FTR — average First-Try-Right across the project's
        // FTR-scored sessions. `ftrDelta` per pattern is its folder's FTR minus this.
        let project_ftr_row: (Option<f64>,) = sqlx_core::query_as::query_as(
            "SELECT avg(CASE WHEN ftr THEN 1.0 ELSE 0.0 END)::float8
             FROM activity.sessions WHERE project_id = $1 AND ftr IS NOT NULL"
        ).bind(project_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        let project_ftr: Option<f64> = project_ftr_row.0;

        // Each pattern + its folder's average FTR (locus signal).
        // confidence is nullable (correction-prone / rule-candidate patterns set
        // no confidence) — decode as Option to avoid a NULL→f64 decode failure.
        // description / example / enforcement are exposed here (previously
        // dropped) so the Patterns screen can render the guidance the analyzer
        // captured with each pattern (T3 Slice 2.1).
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            uuid::Uuid,      // id
            String,          // name
            Option<String>,  // family
            bool,            // is_anti_pattern
            String,          // lifecycle
            Option<f64>,     // confidence
            i32,             // instance_count
            Option<f64>,     // folder_ftr
            Option<String>,  // description
            Option<String>,  // example
            Option<String>,  // enforcement
        )> = sqlx_core::query_as::query_as(
                "SELECT pp.id, pp.name, pp.family, pp.is_anti_pattern, pp.lifecycle::text,
                        pp.confidence::float8, pp.instance_count,
                        (SELECT avg(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)::float8
                           FROM activity.sessions s
                          WHERE s.folder_id = pp.folder_id AND s.ftr IS NOT NULL
                            AND s.outcome <> 'empty'::sensei.session_outcome) AS folder_ftr,
                        pp.description, pp.example, pp.enforcement
                 FROM sensei.project_patterns pp WHERE pp.project_id = $1
                 ORDER BY pp.is_anti_pattern, pp.name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let (followed, anti): (Vec<_>, Vec<_>) = rows.into_iter().partition(|r| !r.3);
        let map_row = |(id, name, family, is_anti, lifecycle, confidence, count, folder_ftr, description, example, enforcement): (uuid::Uuid, String, Option<String>, bool, String, Option<f64>, i32, Option<f64>, Option<String>, Option<String>, Option<String>)| {
            let kind = crate::pattern_effectiveness::pattern_kind(is_anti, &lifecycle);
            let ftr_delta = crate::pattern_effectiveness::ftr_delta(folder_ftr, project_ftr);
            serde_json::json!({
                "id":            id,
                "name":          name,
                "family":        family,
                "isAntiPattern": is_anti,
                "lifecycle":     lifecycle,
                "confidence":    confidence,
                "instanceCount": count,
                "kind":          kind,
                "ftrDelta":      ftr_delta,
                "description":   description,
                "example":       example,
                "enforcement":   enforcement,
            })
        };
        Ok(serde_json::json!({
            "followed": followed.into_iter().map(map_row).collect::<Vec<_>>(),
            "antiPatterns": anti.into_iter().map(map_row).collect::<Vec<_>>(),
        }))
    }

    pub async fn get_project_recommendations(&self, project_id: &uuid::Uuid, status: Option<&str>) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<String>, String, Option<String>,
                        Option<f64>, Option<f64>, Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>,
                        Option<f64>, bool, String)> =
            sqlx_core::query_as::query_as(
                // `action_type` powers the Upgrades screen's installable filter; it is
                // `not null` on the table and mirrors the impact serializer's `actionType`.
                "SELECT id, title, urgency::text, status::text, verdict::text, why, impact,
                        baseline_ftr::float8, current_ftr::float8, acted_at, measured_at,
                        score::float8, focal, action_type
                 FROM inference.recommendations WHERE project_id = $1
                   AND ($2::text IS NULL OR status::text = $2)
                 ORDER BY focal DESC, score DESC NULLS LAST,
                          CASE urgency WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, id
                 LIMIT 50"
            ).bind(project_id).bind(status)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, title, urgency, status, verdict, why, impact, baseline, current, acted, measured, score, focal, action_type)| {
            serde_json::json!({
                "id": id, "title": title, "urgency": urgency, "status": status, "verdict": verdict,
                "why": why, "impact": impact, "actionType": action_type,
                "baseline_ftr": baseline, "current_ftr": current,
                "acted_at": acted.map(|t| t.to_rfc3339()), "measured_at": measured.map(|t| t.to_rfc3339()),
                "score": score, "focal": focal,
            })
        }).collect())
    }

    // ── Index Errors ──────────────────────────────────────────────────

}
