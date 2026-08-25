use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    pub async fn list_playbooks(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT name, title, when_to_use, opening_tone, method_ref
               FROM sensei.playbooks WHERE enabled ORDER BY name",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name,title,wtu,tone,mref)| serde_json::json!({
            "name":name,"title":title,"when_to_use":wtu,"opening_tone":tone,"method_ref":mref
        })).collect())
    }

    /// Fetch a single playbook by name (any enabled state), for enriching a
    /// recommendation response with its `opening_tone` + `when_to_use`.
    pub async fn get_playbook(&self, name: &str) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(String, String, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT name, title, when_to_use, opening_tone, method_ref
               FROM sensei.playbooks WHERE name = $1",
            )
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(|(name, title, wtu, tone, mref)| serde_json::json!({
            "name": name, "title": title, "when_to_use": wtu, "opening_tone": tone, "method_ref": mref
        })))
    }

    pub async fn list_intake_guide(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, Option<String>, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT kind, axis, prompt, help FROM sensei.intake_guide WHERE enabled
              ORDER BY (kind='frame') DESC, axis",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(kind, axis, prompt, help)| {
                serde_json::json!({
                    "kind":kind,"axis":axis,"prompt":prompt,"help":help
                })
            })
            .collect())
    }

    /// Returns the rule set as pure `crate::playbook::Rule`s (ready for the resolver).
    pub async fn list_playbook_rules(&self) -> Result<Vec<crate::playbook::Rule>, String> {
        use crate::playbook::{Intent, Lifecycle, Risk, Rule};
        let rows: Vec<(
            uuid::Uuid,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
            String,
            i32,
            i32,
        )> = sqlx_core::query_as::query_as(
            "SELECT id, name, match_lifecycle::text, match_intent::text, match_risk::text,
                        playbook, rationale, priority, coalesce(base_priority, priority)
                   FROM sensei.playbook_rules WHERE enabled ORDER BY priority DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(id, name, lf, it, rk, pb, rat, pri, base_pri)| Rule {
                id: Some(id),
                name,
                match_lifecycle: lf.as_deref().and_then(Lifecycle::parse),
                match_intent: it.as_deref().and_then(Intent::parse),
                match_risk: rk.as_deref().and_then(Risk::parse),
                playbook: pb,
                rationale: rat,
                priority: pri,
                base_priority: base_pri,
            })
            .collect())
    }

    /// Snapshot the session's outcome onto confirmed, not-yet-attributed runs. Returns rows updated.
    pub async fn attribute_playbook_outcomes(&self) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.playbook_run pr
                SET outcome = s.outcome::text, outcome_ftr = s.ftr
               FROM activity.sessions s
              WHERE pr.session_id = s.id AND pr.confirmed
                AND pr.outcome IS NULL AND s.outcome IS NOT NULL",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    pub async fn playbook_combo_stats(
        &self,
    ) -> Result<Vec<crate::playbook::ComboPlaybookStat>, String> {
        use crate::playbook::{ComboPlaybookStat, Intent, Lifecycle, Risk};
        let rows: Vec<(String, String, String, String, i64, f64)> = sqlx_core::query_as::query_as(
            "SELECT lifecycle::text, intent::text, risk::text, playbook,
                    count(*)::int8, avg(outcome_ftr::int)::float8
               FROM sensei.playbook_run
              WHERE confirmed AND outcome_ftr IS NOT NULL
              GROUP BY lifecycle, intent, risk, playbook",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .filter_map(|(l, i, r, pb, n, ftr)| {
                Some(ComboPlaybookStat {
                    lifecycle: Lifecycle::parse(&l)?,
                    intent: Intent::parse(&i)?,
                    risk: Risk::parse(&r)?,
                    playbook: pb,
                    n,
                    ftr_rate: ftr,
                })
            })
            .collect())
    }

    /// Confirmed+attributed sample size + FTR rate for one exact (lifecycle, intent,
    /// risk, playbook) combo — the auto-select-on-trust gate's evidence lookup.
    /// Auto-select trust for a `(lifecycle, intent, risk, playbook)` combo,
    /// scoped to ONE project. A playbook run always happens in a project, so
    /// trust is "does this playbook earn FTR in THIS project" — never a global
    /// average across unrelated projects (which would auto-select on the wrong
    /// signal). Returns `(n confirmed+attributed runs, avg FTR)` for the combo
    /// within `project_id`.
    pub async fn playbook_combo_trust(
        &self,
        lifecycle: &str,
        intent: &str,
        risk: &str,
        playbook: &str,
        project_id: &uuid::Uuid,
    ) -> Result<(i64, f64), String> {
        let row: (i64, f64) = sqlx_core::query_as::query_as(
            "SELECT count(*)::int8, coalesce(avg(outcome_ftr::int)::float8, 0.0)
               FROM sensei.playbook_run
              WHERE confirmed AND outcome_ftr IS NOT NULL
                AND lifecycle=$1::sensei.chunk_lifecycle AND intent=$2::sensei.chunk_intent
                AND risk=$3::sensei.chunk_risk AND playbook=$4 AND project_id=$5",
        )
        .bind(lifecycle)
        .bind(intent)
        .bind(risk)
        .bind(playbook)
        .bind(project_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// `classified_by` records how the axes were derived (e.g. "manual",
    /// a gateway model id, or "heuristic") and `model_fallback`
    /// flags whether the local-model path fell back to the heuristic —
    /// both feed the §9 measurement of local-model usefulness.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_playbook_run(
        &self,
        session_id: Option<uuid::Uuid>,
        feature: Option<&str>,
        lifecycle: &str,
        intent: &str,
        risk: &str,
        rule_id: Option<uuid::Uuid>,
        playbook: &str,
        rationale: &str,
        confirmed: bool,
        classified_by: Option<&str>,
        model_fallback: bool,
        project_id: uuid::Uuid,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.playbook_run
               (session_id, feature, lifecycle, intent, risk, rule_id, playbook, rationale, confirmed,
                classified_by, model_fallback, project_id)
             VALUES ($1,$2,$3::sensei.chunk_lifecycle,$4::sensei.chunk_intent,$5::sensei.chunk_risk,$6,$7,$8,$9,$10,$11,$12)
             RETURNING id"
        ).bind(session_id).bind(feature).bind(lifecycle).bind(intent).bind(risk)
         .bind(rule_id).bind(playbook).bind(rationale).bind(confirmed)
         .bind(classified_by).bind(model_fallback).bind(project_id)
         .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Whether `session_id` has a *confirmed* playbook_run — the gate the
    /// nudge hook (`POST /hook/nudge`) uses to decide whether to suggest
    /// `/sensei:intake`. A session with no confirmed run yet is nudged;
    /// one that already confirmed a playbook is left alone.
    pub async fn session_has_confirmed_run(&self, session_id: &uuid::Uuid) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT exists(SELECT 1 FROM sensei.playbook_run WHERE session_id = $1 AND confirmed)",
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Apply a §9 `learn()` plan: reweight existing rules' `priority` in place
    /// (off their immutable `base_priority`), and UPSERT proposed new rules as
    /// `source='learned', enabled=false` (invisible to the resolver's
    /// `list_playbook_rules` until accepted). Upsert targets the learned
    /// partial-unique index so re-running the same plan is idempotent.
    pub async fn apply_learn_plan(&self, plan: &crate::playbook::LearnPlan) -> Result<(), String> {
        for (id, new_priority) in &plan.reweights {
            sqlx_core::query::query("UPDATE sensei.playbook_rules SET priority = $2 WHERE id = $1")
                .bind(id)
                .bind(new_priority)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }
        for p in &plan.proposals {
            sqlx_core::query::query(
                "INSERT INTO sensei.playbook_rules
                   (name, match_lifecycle, match_intent, match_risk, playbook, rationale,
                    priority, base_priority, enabled, source)
                 VALUES ($1, $2::sensei.chunk_lifecycle, $3::sensei.chunk_intent, $4::sensei.chunk_risk,
                         $5, $6, $7, $7, false, 'learned')
                 ON CONFLICT (match_lifecycle, match_intent, match_risk, playbook)
                   WHERE source='learned'
                 DO UPDATE SET rationale = excluded.rationale, priority = excluded.priority, base_priority = excluded.priority"
            )
            .bind(format!("learned: {}", p.playbook))
            .bind(p.lifecycle.as_str()).bind(p.intent.as_str()).bind(p.risk.as_str())
            .bind(&p.playbook).bind(&p.rationale).bind(p.priority)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Pending §9 learned-rule proposals (`source='learned' AND NOT enabled`) —
    /// invisible to the resolver until accepted. Backs the accept-path list
    /// endpoint/MCP tool (Task 5) and is exercised directly by the T4 apply test.
    pub async fn list_playbook_rule_proposals(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, Option<String>, Option<String>, String, String, i32, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, match_lifecycle::text, match_intent::text, match_risk::text,
                        playbook, rationale, priority, created_at
                   FROM sensei.playbook_rules WHERE source='learned' AND NOT enabled ORDER BY created_at DESC"
            ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id,name,lf,it,rk,pb,rat,pri,created)| serde_json::json!({
            "id": id, "name": name, "match_lifecycle": lf, "match_intent": it, "match_risk": rk,
            "playbook": pb, "rationale": rat, "priority": pri, "created_at": created,
        })).collect())
    }

    /// Accept a §9 learned-rule proposal: flip it `enabled=true` so the resolver's
    /// `list_playbook_rules` (which filters `WHERE enabled`) picks it up. Scoped to
    /// `source='learned'` — never flips a builtin/manual rule via this path.
    ///
    /// Returns `Ok(true)` only when a row actually flipped; `Ok(false)` when no
    /// matching learned proposal exists (unknown id, or a builtin/manual rule) — so
    /// the caller can 404 instead of fabricating `{accepted}` for a no-op UPDATE.
    pub async fn accept_playbook_rule(&self, id: &uuid::Uuid) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.playbook_rules SET enabled=true WHERE id=$1 AND source='learned'",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// FTR by `classified_by` (+ `model_fallback`) — measures whether the local
    /// gateway model's chunk classification is actually useful vs. the heuristic
    /// fallback (§9 model-stats read).
    pub async fn playbook_model_stats(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(Option<String>, Option<bool>, i64, f64)> = sqlx_core::query_as::query_as(
            "SELECT classified_by, model_fallback, count(*)::int8, avg(outcome_ftr::int)::float8
               FROM sensei.playbook_run WHERE confirmed AND outcome_ftr IS NOT NULL
              GROUP BY classified_by, model_fallback ORDER BY count(*) DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(cb, mf, n, ftr)| {
                serde_json::json!({
                    "classified_by": cb, "model_fallback": mf, "n": n, "ftr_rate": ftr
                })
            })
            .collect())
    }
}
