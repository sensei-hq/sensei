use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    pub async fn create_benchmark_report(
        &self,
        folder_id: Option<&uuid::Uuid>,
        run_name: &str,
        strategy: &str,
        score: Option<f64>,
        tokens: Option<i32>,
        elapsed_ms: Option<i32>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.benchmark_reports(folder_id, run_name, strategy, score, tokens, elapsed_ms) VALUES($1, $2, $3, $4, $5, $6) RETURNING id"
        ).bind(folder_id).bind(run_name).bind(strategy).bind(score).bind(tokens).bind(elapsed_ms)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_benchmark_reports(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<f64>, Option<i32>, bool, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT id, run_name, strategy, score::float8, tokens, promoted, modified_at FROM sensei.benchmark_reports ORDER BY modified_at DESC"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, strategy, score, tokens, promoted, modified)| {
            serde_json::json!({ "id": id, "run_name": name, "strategy": strategy, "score": score, "tokens": tokens, "promoted": promoted, "modified_at": modified.to_rfc3339() })
        }).collect())
    }

    // ── Views (read-only) ────────────────────────────────────────────

    /// Recompute FTR deltas for accepted recommendations with pending verdict.
    /// Compares current 14-day FTR against baseline_ftr snapshot at time of acceptance.
    /// Returns number of recommendations updated.
    pub async fn measure_pending_verdicts(&self) -> Result<i64, String> {
        // Per-row measure so we can also compose the MOE consensus JSON
        // the Observatory Impact panel renders. The classification rule
        // is the same ±0.05 FTR band the old bulk UPDATE used; kept in
        // `crate::verdicts::Verdict::from_ftr_delta` so it's testable.
        //
        // Two-phase per rec:
        //   1. UPDATE the rec (verdict / current_ftr / measured_at).
        //   2. Insert or update the linked reasoning_trace's `consensus`
        //      JSON with the synth helper. If the rec has no trace yet,
        //      we mint one with trigger_event = 'verdict_measurement'
        //      and link it back onto the rec.
        //
        // Failures in the reasoning-trace write are logged but don't
        // abort the whole batch — verdict measurement is best-effort by
        // design (the scheduler retries every full-refresh window).
        type Row = (uuid::Uuid, Option<uuid::Uuid>, f64, f64, Option<Vec<String>>, String);
        let rows: Vec<Row> = sqlx_core::query_as::query_as(
            "WITH current AS (
               SELECT r.id AS rec_id,
                      AVG(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)::float8 AS current_ftr
                 FROM inference.recommendations r
                 JOIN activity.sessions s ON s.project_id = r.project_id
                                         AND s.started_at > r.acted_at
                WHERE r.status = 'accepted'
                  AND r.verdict = 'pending'
                  AND r.acted_at < now() - interval '3 days'
                  AND s.outcome IS NOT NULL
                  AND s.outcome <> 'empty'::sensei.session_outcome
                GROUP BY r.id
                HAVING COUNT(*) >= 3
             )
             SELECT r.id,
                    r.reasoning_trace_id,
                    COALESCE(r.baseline_ftr, 0)::float8,
                    c.current_ftr,
                    t.models_used,
                    r.based_on::text
               FROM inference.recommendations r
               JOIN current c ON c.rec_id = r.id
          LEFT JOIN inference.reasoning_traces t ON t.id = r.reasoning_trace_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        if rows.is_empty() {
            return Ok(0);
        }

        let mut updated: i64 = 0;
        for (rec_id, trace_id, baseline_ftr, current_ftr, models_used_opt, based_on) in rows {
            let verdict = crate::verdicts::Verdict::from_ftr_delta(current_ftr - baseline_ftr);
            let models_used = models_used_opt.unwrap_or_default();
            let consensus = crate::verdicts::synthesize_reasoning(
                verdict,
                baseline_ftr,
                current_ftr,
                &models_used,
            );

            let upd = sqlx_core::query::query(
                "UPDATE inference.recommendations
                    SET verdict     = $2::sensei.recommendation_verdict,
                        current_ftr = $3,
                        measured_at = now()
                  WHERE id = $1 AND verdict = 'pending'",
            )
            .bind(rec_id)
            .bind(verdict.as_wire())
            .bind(current_ftr)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

            // The `verdict = 'pending'` guard makes the flip win exactly once, so a
            // concurrent scheduler tick can't measure (or challenge) the same rec
            // twice. `rows_affected == 0` means another tick already claimed it.
            if upd.rows_affected() == 0 {
                continue;
            }
            updated += 1;

            // Learning-loop feedback: an accepted rec whose FTR REGRESSED after
            // acceptance discredits the memory that spawned it. Challenge (weaken)
            // that source memory through the existing memory_outcome pipeline — the
            // `memory_outcome_apply` trigger does the strength/status math. This
            // fires at most once per rec (the atomic pending→negative flip above is
            // the transition signal). Non-fatal: a challenge-write failure must not
            // abort verdict measurement.
            if verdict == crate::verdicts::Verdict::Negative
                && let Err(e) = self.challenge_source_memory_for_rec(&rec_id, &based_on).await
            {
                tracing::warn!(error = %e, rec = %rec_id, "measure_pending_verdicts: challenge source memory failed");
            }
            // Positive mirror: an FTR improvement vindicates the source memory —
            // reinforce it (bumps reinforced_count/strength, drives the promotion
            // ladder). Same once-per-rec transition signal; non-fatal.
            if verdict == crate::verdicts::Verdict::Positive
                && let Err(e) = self.reinforce_source_memory_for_rec(&rec_id, &based_on).await
            {
                tracing::warn!(error = %e, rec = %rec_id, "measure_pending_verdicts: reinforce source memory failed");
            }

            match trace_id {
                Some(id) => {
                    if let Err(e) = sqlx_core::query::query(
                        "UPDATE inference.reasoning_traces SET consensus = $2 WHERE id = $1",
                    )
                    .bind(id)
                    .bind(&consensus)
                    .execute(&self.pool)
                    .await
                    {
                        tracing::warn!(error = %e, rec = %rec_id, trace = %id, "measure_pending_verdicts: consensus update failed");
                    }
                }
                None => {
                    match sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
                        "INSERT INTO inference.reasoning_traces
                            (trigger_event, trigger_detail, models_used, consensus)
                         VALUES ($1, $2, $3, $4)
                         RETURNING id"
                    )
                    .bind("verdict_measurement")
                    .bind(serde_json::json!({ "recId": rec_id, "verdict": verdict.as_wire() }))
                    .bind::<Vec<String>>(models_used)
                    .bind(&consensus)
                    .fetch_one(&self.pool).await
                    {
                        Ok((new_trace_id,)) => {
                            if let Err(e) = sqlx_core::query::query(
                                "UPDATE inference.recommendations SET reasoning_trace_id = $2 WHERE id = $1"
                            ).bind(rec_id).bind(new_trace_id).execute(&self.pool).await {
                                tracing::warn!(error = %e, rec = %rec_id, trace = %new_trace_id, "measure_pending_verdicts: relink failed");
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, rec = %rec_id, "measure_pending_verdicts: mint trace failed");
                        }
                    }
                }
            }
        }

        Ok(updated)
    }

    // ── Observatory views ──────────────────────────────────────────────

    /// Daily FTR sparkline. Re-sourced from the daily `ftr` rows in
    /// `sensei.project_metric_daily` (metric='ftr') — the single FTR source of
    /// truth: `ftr_rate` = the stored `value` (num/den), `session_count` =
    /// `props.denominator`. Per-project rows read straight through (one row per
    /// day). The holistic (no project filter) branch POOLS the parts per day —
    /// Σnumerator / Σdenominator — so it stays session-weighted and consistent
    /// with every other rollup, honouring the `project_metrics` invariant that
    /// ratios re-derive from their parts (never an average-of-averages). Response
    /// shape unchanged (`{day, ftr_rate, session_count}`);
    /// `props.correction_count`/`avg_turns` are carried in the store but were
    /// never part of this getter's shape, so they stay unexposed.
    pub async fn get_ftr_daily(
        &self,
        project_id: Option<&uuid::Uuid>,
        days: i32,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(chrono::NaiveDate, Option<f64>, Option<i64>)> = if let Some(pid) = project_id
        {
            sqlx_core::query_as::query_as(
                "SELECT d.date, d.value::float8, (d.props->>'denominator')::int8
                   FROM sensei.project_metric_daily d
                  WHERE d.metric = 'ftr' AND d.project_id = $1 AND d.date >= (current_date - $2::int)
                  ORDER BY d.date"
            ).bind(pid).bind(days).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
        } else {
            sqlx_core::query_as::query_as(
                "SELECT d.date,
                        (SUM((d.props->>'numerator')::float8) / NULLIF(SUM((d.props->>'denominator')::float8), 0))::float8 AS ftr_rate,
                        SUM((d.props->>'denominator')::int8)::int8 AS session_count
                   FROM sensei.project_metric_daily d
                  WHERE d.metric = 'ftr' AND d.date >= (current_date - $1::int)
                  GROUP BY d.date ORDER BY d.date"
            ).bind(days).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
        };
        Ok(rows.into_iter().map(|(day, ftr, count)| {
            serde_json::json!({ "day": day.to_string(), "ftr_rate": ftr.unwrap_or(0.0), "session_count": count.unwrap_or(0) })
        }).collect())
    }

    pub async fn get_hotspots(
        &self,
        project_id: &uuid::Uuid,
        days: i32,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, i64, i64)> = sqlx_core::query_as::query_as(
            "SELECT folder, file_path, edit_count, correction_count
             FROM sensei.project_hotspots
             WHERE project_id = $1 AND last_event_at >= (now() - ($2::int || ' days')::interval)
             ORDER BY (edit_count + correction_count) DESC LIMIT 20",
        )
        .bind(project_id)
        .bind(days)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(folder, path, edits, corrections)| {
            serde_json::json!({ "folder": folder, "file_path": path, "edit_count": edits, "correction_count": corrections })
        }).collect())
    }

    pub async fn get_quality_signals(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<serde_json::Value, String> {
        let row: Option<(f64, Option<f64>, i64, Option<f64>)> = sqlx_core::query_as::query_as(
            "SELECT ftr_7d::float8, pattern_compliance::float8, open_drift_count, test_pass_rate::float8
             FROM sensei.project_quality_signals WHERE project_id = $1"
        ).bind(project_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(match row {
            Some((ftr, compliance, drift, tests)) => serde_json::json!({
                "ftr_7d": ftr, "pattern_compliance": compliance,
                "open_drift_count": drift, "test_pass_rate": tests
            }),
            None => serde_json::json!({
                "ftr_7d": 0, "pattern_compliance": null, "open_drift_count": 0, "test_pass_rate": null
            }),
        })
    }

    /// Upsert one `sensei.project_metrics` row, keyed on its identity
    /// `(metric_id, project_id, folder_id, session_id, computed_on, grain)` — the
    /// `project_metrics_identity` unique index (`nulls not distinct`, so a
    /// project-scope / daily-grain row's null `folder_id`/`session_id` collide
    /// rather than duplicate). A re-run with the same identity BACKFILLS in place —
    /// updates `value`, `props`, `source` and bumps `modified_at` — so the compute
    /// tasks are idempotent. Returns the row id. `grain` is the
    /// `sensei.metric_grain` enum (`daily`|`session`); `source` the
    /// `sensei.metric_source` enum (`measured`|`estimated`).
    ///
    /// `project_metrics_identity` is a unique INDEX, not a named constraint, so the
    /// conflict target is the column list — Postgres infers the arbiter index
    /// (honouring its `nulls not distinct`); `ON CONFLICT ON CONSTRAINT <name>`
    /// would not resolve against an index.
    #[allow(clippy::too_many_arguments)]
    /// Metric correlations for a project: which signals move together.
    ///
    /// Reads the daily project-scope values as (project, day) CELLS and the
    /// registry's `derives_from` suppression lists, then defers to
    /// [`crate::correlate::correlations`] for the statistics — the maths is pure
    /// and unit-tested there, this only supplies observations.
    /// `project_id = None` correlates across EVERY project (the portfolio view).
    ///
    /// That is usually the useful call. Per project the data is thin — measured
    /// here, most projects have zero daily rows and the busiest yields exactly one
    /// reportable pair — because a correlation needs both metrics present on the
    /// same day, repeatedly. Pooling projects raises the paired count enough for
    /// the weaker-but-real relationships to clear the gates, at the cost of mixing
    /// codebases: a portfolio finding describes how the SIGNALS relate, not how one
    /// project behaves.
    pub async fn get_metric_correlations(
        &self,
        project_id: Option<&uuid::Uuid>,
    ) -> Result<serde_json::Value, String> {
        // Cells are keyed by (project, day) even in the portfolio view: pooling two
        // projects' values into one day would correlate unrelated codebases and
        // manufacture a relationship that exists in neither.
        let obs: Vec<(uuid::Uuid, chrono::NaiveDate, String, f64)> = sqlx_core::query_as::query_as(
            "SELECT pm.project_id, pm.computed_on, m.key, pm.value::float8 \
               FROM sensei.project_metrics pm \
               JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE ($1::uuid IS NULL OR pm.project_id = $1) \
                AND pm.grain = 'daily' \
                AND pm.scope = 'user' AND pm.value IS NOT NULL",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let mut by_day: std::collections::HashMap<
            (uuid::Uuid, chrono::NaiveDate),
            crate::correlate::Cell,
        > = std::collections::HashMap::new();
        for (pid, day, key, value) in obs {
            by_day.entry((pid, day)).or_default().insert(key, value);
        }
        let cells: Vec<crate::correlate::Cell> = by_day.into_values().collect();

        let rows: Vec<(String, Option<Vec<String>>)> = sqlx_core::query_as::query_as(
            "SELECT key, derives_from FROM sensei.metrics WHERE derives_from IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let derives: std::collections::HashMap<String, Vec<String>> =
            rows.into_iter().filter_map(|(k, v)| v.map(|v| (k, v))).collect();

        let found = crate::correlate::correlations(&cells, &derives);
        Ok(serde_json::json!({
            "cells": cells.len(),
            "min_pairs": crate::correlate::MIN_PAIRS,
            "min_rho": crate::correlate::MIN_RHO,
            "correlations": found.iter().map(|c| serde_json::json!({
                "a": c.a, "b": c.b, "rho": c.rho, "n": c.n,
            })).collect::<Vec<_>>(),
        }))
    }

    /// Weeks of composite-score history the health payload carries. Twelve ≈ a
    /// quarter: enough to read a direction, few enough to plot legibly in the
    /// hero's 240px sparkline. The label the card renders says the same thing, so
    /// this number and that copy have to agree.
    const HEALTH_TREND_WEEKS: i64 = 12;

    /// Project health from the rating views (spec 2026-08-20): the weighted 0–100
    /// `health_score` + per-metric 0–5 `ratings` (the radar spokes) + the `components`
    /// map the score was built from. `health_score`/`components` are null when nothing
    /// is rated yet (honest-empty — never a fabricated 0). `ratings` lists EVERY metric
    /// with a current reading (rated or not) so the radar can show all spokes + values.
    pub async fn get_project_health(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<serde_json::Value, String> {
        let ratings: Vec<(serde_json::Value,)> = sqlx_core::query_as::query_as(
            "SELECT jsonb_build_object( \
                 'metric', metric, 'name', metric_name, 'family', family, \
                 'value', value, 'rating', rating, 'weight', weight) \
               FROM sensei.metric_ratings WHERE project_id = $1 \
              ORDER BY family::text, metric",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let health: Option<(i32, i32, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT health_score, rated_metrics, components \
               FROM sensei.project_health_score WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let (score, rated, components) = match health {
            Some((s, r, c)) => (serde_json::json!(s), serde_json::json!(r), c),
            None => (serde_json::Value::Null, serde_json::json!(0), serde_json::Value::Null),
        };
        // The score's own history, oldest-first, so the hero readout can show a
        // trend + sparkline instead of falling back to FTR. Weekly: the score is a
        // weighted roll-up of ratings, and a daily series of it is mostly noise.
        // Read from `project_health_trend` (a view over the same rating facts), so
        // there is no stored-and-recomputed second copy of the number.
        //
        // BOUNDED to the last `HEALTH_TREND_WEEKS`. The view keeps every period, so
        // an unbounded read grows forever — a three-year-old project would ship
        // ~156 points on every metrics load and cram them into a 240px sparkline.
        // Newest-first inside the subquery to apply the limit, then re-sorted
        // oldest-first because that is the order a sparkline plots.
        let trend: Vec<(chrono::NaiveDate, i32)> = sqlx_core::query_as::query_as(
            "SELECT period, health_score FROM ( \
                 SELECT period, health_score \
                   FROM sensei.project_health_trend \
                  WHERE project_id = $1 AND grain = 'weekly' \
                  ORDER BY period DESC \
                  LIMIT $2 \
             ) recent \
             ORDER BY period",
        )
        .bind(project_id)
        .bind(Self::HEALTH_TREND_WEEKS)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "health_score": score,       // 0–100, or null when nothing rated
            "rated_metrics": rated,
            "components": components,     // {metric → {rating, weight, name}} or null
            "ratings": ratings.into_iter().map(|(j,)| j).collect::<Vec<_>>(),
            "trend": trend.into_iter()
                .map(|(period, s)| serde_json::json!({ "period": period, "health_score": s }))
                .collect::<Vec<_>>(),
        }))
    }

    pub async fn upsert_project_metric_repo(
        &self,
        metric_id: &uuid::Uuid,
        repository_id: &uuid::Uuid,
        scope: &str,
        identity: Option<&str>,
        commit_sha: Option<&str>,
        computed_on: chrono::NaiveDate,
        grain: &str,
        value: f64,
        props: &serde_json::Value,
        source: &str,
    ) -> Result<uuid::Uuid, String> {
        // Writes target the TABLE, never the `project_metrics` compatibility view:
        // the view's project_id is derived and has no inverse, so an insert through
        // it could not know which repository the value belongs to.
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.repository_metrics
                (metric_id, repository_id, scope, identity, commit_sha,
                 computed_on, grain, value, props, source)
             VALUES ($1, $2, $3::sensei.metric_scope, $4, $5, $6,
                     $7::sensei.metric_grain, $8::float8::numeric, $9, $10::sensei.metric_source)
             ON CONFLICT (metric_id, repository_id, scope, identity, commit_sha, computed_on, grain) DO UPDATE
                SET value       = EXCLUDED.value,
                    props       = EXCLUDED.props,
                    source      = EXCLUDED.source,
                    modified_at = now()
             RETURNING id",
        )
        .bind(metric_id)
        .bind(repository_id)
        .bind(scope)
        .bind(identity)
        .bind(commit_sha)
        .bind(computed_on)
        .bind(grain)
        .bind(value)
        .bind(props)
        .bind(source)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Project-grain convenience wrapper — `repository_id = NULL`, `scope = 'user'`.
    /// Convenience wrapper for callers that only vary `(metric, day, value)` —
    /// tests and the metric-preview route. Fills the scope/identity/commit_sha
    /// dimensions with the defaults a simple daily value has.
    ///
    /// `repository_id` is now REQUIRED rather than defaulted to NULL: the column
    /// is NOT NULL, because every row in the store did in fact have one and a
    /// nullable grain column was an invitation to write a row that joins to
    /// nothing.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_project_metric(
        &self,
        metric_id: &uuid::Uuid,
        repository_id: &uuid::Uuid,
        computed_on: chrono::NaiveDate,
        grain: &str,
        value: f64,
        props: &serde_json::Value,
        source: &str,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_project_metric_repo(
            metric_id,
            repository_id,
            "user",
            None,
            None,
            computed_on,
            grain,
            value,
            props,
            source,
        )
        .await
    }

    // ── Per-datapoint explainer enrichment (compute-time) ─────────────────

    /// The scope=`user` DAILY rows one metric GROUP wrote for `day` — `(row_id,
    /// metric_key, value)` — the datapoints the compute-time explainer enrichment
    /// (see [`crate::tasks::handlers::metrics::explainer`]) annotates. Scoped to the
    /// group via the metric's `task_name`, to the local-user value via `scope =
    /// 'user'` (the same rows `sensei.project_metric_daily` pools to the project),
    /// and to daily grain. Under the repo grain the `_v2` identity carries no
    /// `folder_id`/`session_id`, so this now returns one row PER REPOSITORY the
    /// group wrote for the day (each row_id gets its own explainer). Empty when the
    /// group wrote no scope=`user` daily row that day (honest-empty). Propagates the
    /// read error; never masks it.
    pub async fn get_group_daily_metrics_for_day(
        &self,
        project_id: &uuid::Uuid,
        task_name: &str,
        day: chrono::NaiveDate,
    ) -> Result<Vec<(uuid::Uuid, String, f64)>, String> {
        let rows: Vec<(uuid::Uuid, String, f64)> = sqlx_core::query_as::query_as(
            "SELECT pm.id, m.key, pm.value::float8
               FROM sensei.project_metrics pm
               JOIN sensei.metrics         m ON m.id = pm.metric_id
              WHERE pm.project_id = $1
                AND m.task_name   = $2
                AND pm.grain      = 'daily'
                AND pm.scope      = 'user'
                AND pm.computed_on = $3
              ORDER BY m.key",
        )
        .bind(project_id)
        .bind(task_name)
        .bind(day)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// The immediately-prior day's POOLED project value for one metric — the
    /// `prev_value` the explainer's `delta` is measured against. Reads the most
    /// recent row of the `sensei.project_metric_daily` VIEW (the repo-grain rows
    /// already pooled to the project) with `date < day` for that metric key, so the
    /// delta compares like-for-like against today's pooled value rather than a
    /// single repository's base row. `None` when this is the metric's first day
    /// (honest-null, never a fabricated 0). Propagates the read error; never masks it.
    pub async fn get_prev_daily_metric_value(
        &self,
        project_id: &uuid::Uuid,
        key: &str,
        day: chrono::NaiveDate,
    ) -> Result<Option<f64>, String> {
        let row: Option<(f64,)> = sqlx_core::query_as::query_as(
            "SELECT d.value::float8
               FROM sensei.project_metric_daily d
              WHERE d.project_id = $1
                AND d.metric     = $2
                AND d.date       < $3
              ORDER BY d.date DESC
              LIMIT 1",
        )
        .bind(project_id)
        .bind(key)
        .bind(day)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(v,)| v))
    }

    /// One day's measurable session-outcome counts for a project — `(total,
    /// completed, first_try)` — the day context the explainer grounds in. Same
    /// measurable base as [`Self::get_project_sessions_for_day`] (folder-join scope,
    /// `outcome IS NOT NULL`, day pinned on `date_trunc('day', started_at)`), read as
    /// one cheap COUNT-by-outcome. All-zero when no measurable session ran that day
    /// (honest, never fabricated). Propagates the read error; never masks it.
    pub async fn get_day_session_outcome_counts(
        &self,
        project_id: &uuid::Uuid,
        day: chrono::NaiveDate,
    ) -> Result<(i64, i64, i64), String> {
        let row: (i64, i64, i64) = sqlx_core::query_as::query_as(
            "SELECT count(*)::int8                                                                  AS total
                  , count(*) FILTER (WHERE s.outcome = 'completed'::sensei.session_outcome)::int8   AS completed
                  , count(*) FILTER (WHERE s.ftr)::int8                                             AS first_try
               FROM activity.sessions s
               JOIN sensei.folders    f ON f.id = s.folder_id
              WHERE f.project_id = $1
                AND s.outcome   IS NOT NULL
                AND s.outcome   <> 'empty'::sensei.session_outcome
                AND date_trunc('day', s.started_at)::date = $2",
        )
        .bind(project_id)
        .bind(day)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// MERGE the per-datapoint `explainer` into one project_metrics row's `props`,
    /// preserving every other key (`props || jsonb_build_object('explainer', …)` —
    /// so `numerator`/`denominator`/`n`/… survive). Runs AFTER the value upsert
    /// (whose `ON CONFLICT` overwrites `props` wholesale), so the merge is the last
    /// writer. Idempotent: re-merging the same string is a no-op change. Propagates
    /// the write error; never masks it.
    pub async fn merge_metric_explainer(
        &self,
        row_id: &uuid::Uuid,
        explainer: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.project_metrics
                SET props       = props || jsonb_build_object('explainer', $2::text),
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(row_id)
        .bind(explainer)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The ACTIVE metric registry: rows that live on `current_date` (see
    /// [`Self::ACTIVE_METRIC_PREDICATE`]) — retired (past/at `effective_until`) and
    /// not-yet-effective (future `effective_from`) rows are excluded. Drives the
    /// scheduler and the compute handlers.
    pub async fn active_metrics(&self) -> Result<Vec<Metric>, String> {
        let sql = format!(
            "SELECT id, key, name, description, family::text, type::text, unit, direction::text,
                    purpose, how_to_read, formula, task_name, weight::float8, target::float8,
                    effective_from, effective_until
               FROM sensei.metrics
              WHERE {}
              ORDER BY key",
            Self::ACTIVE_METRIC_PREDICATE,
        );
        let rows: Vec<(
            uuid::Uuid,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            String,
            String,
            String,
            String,
            String,
            f64,
            Option<f64>,
            chrono::NaiveDate,
            Option<chrono::NaiveDate>,
        )> = sqlx_core::query_as::query_as(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    key,
                    name,
                    description,
                    family,
                    metric_type,
                    unit,
                    direction,
                    purpose,
                    how_to_read,
                    formula,
                    task_name,
                    weight,
                    target,
                    effective_from,
                    effective_until,
                )| Metric {
                    id,
                    key,
                    name,
                    description,
                    family,
                    metric_type,
                    unit,
                    direction,
                    purpose,
                    how_to_read,
                    formula,
                    task_name,
                    weight,
                    target,
                    effective_from,
                    effective_until,
                },
            )
            .collect())
    }

    /// Distinct `task_name`s over the ACTIVE metric registry — the set of compiled
    /// TaskKinds the scheduler must dispatch. Same active-window filter as
    /// [`Self::active_metrics`].
    pub async fn active_task_names(&self) -> Result<Vec<String>, String> {
        let sql = format!(
            "SELECT DISTINCT task_name FROM sensei.metrics WHERE {} ORDER BY task_name",
            Self::ACTIVE_METRIC_PREDICATE,
        );
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(t,)| t).collect())
    }

    /// `key → metric_id` for every ACTIVE metric whose `task_name` matches — the
    /// map a per-group compute handler resolves its metrics through (shared by all
    /// six base groups so each doesn't re-implement the filter-by-task_name loop).
    /// Built on [`Self::active_metrics`] (same active-window filter), so a retired /
    /// not-yet-effective / unseeded metric is simply ABSENT from the map and the
    /// caller skips it (`ids.get(key)` → `None`) — an inactive metric is never
    /// computed. Propagates the read error; never masks it.
    pub async fn active_metric_ids(
        &self,
        task_name: &str,
    ) -> Result<std::collections::HashMap<String, uuid::Uuid>, String> {
        Ok(self
            .active_metrics()
            .await?
            .into_iter()
            .filter(|m| m.task_name == task_name)
            .map(|m| (m.key, m.id))
            .collect())
    }

    /// The normalized remote key for a repository, when it has one.
    ///
    /// `None` for a local-only repository (no remote, so no key and never
    /// federated) and for an id with no row. Used to look a repository up in the
    /// dōjō's activation ruling, which is keyed on `repo_key` because that is the
    /// one identity both planes share — `repositories.id` is per-install.
    pub async fn repo_key_for_repository(
        &self,
        repository_id: &uuid::Uuid,
    ) -> Result<Option<String>, String> {
        let row: Option<(Option<String>,)> =
            sqlx_core::query_as::query_as("SELECT repo_key FROM sensei.repositories WHERE id = $1")
                .bind(repository_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| format!("repo_key_for_repository: {e}"))?;
        Ok(row.and_then(|(k,)| k))
    }

    /// Latest stored value per metric for a project, with the catalog facets it is
    /// read through — reads `sensei.project_metric_daily` (project-scope daily
    /// rows) and keeps the newest `date` per metric (`DISTINCT ON`), joining
    /// `sensei.metrics` for name/type/unit/direction/purpose/how_to_read. Empty
    /// when the project has no daily rows yet (honest-empty, not a failure). Trend
    /// (prior/delta over `project_metric_trend`) is deferred to the Phase 7
    /// endpoint.
    pub async fn get_project_metrics(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<ProjectMetricRow>, String> {
        // Only ACTIVE metrics: a RETIRED metric (past `effective_until`, e.g.
        // project_health) leaves durable rows in project_metrics — retirement is
        // "in place, never hand-delete a row" — so the values read must exclude
        // them by the active window, or the stale rows keep rendering as a card.
        // Same predicate the registry endpoint uses (effective_from/until resolve
        // to `m` — only sensei.metrics carries them).
        let sql = format!(
            "SELECT DISTINCT ON (d.metric)
                    d.metric, d.date, d.value::float8, d.props,
                    m.name, m.type::text, m.unit, m.direction::text, m.purpose, m.how_to_read
               FROM sensei.project_metric_daily d
               JOIN sensei.metrics m ON m.key = d.metric
              WHERE d.project_id = $1 AND {}
              ORDER BY d.metric, d.date DESC",
            Self::ACTIVE_METRIC_PREDICATE,
        );
        let rows: Vec<(
            String,
            chrono::NaiveDate,
            f64,
            serde_json::Value,
            String,
            String,
            Option<String>,
            String,
            String,
            String,
        )> = sqlx_core::query_as::query_as(&sql)
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    metric,
                    date,
                    value,
                    props,
                    name,
                    metric_type,
                    unit,
                    direction,
                    purpose,
                    how_to_read,
                )| ProjectMetricRow {
                    metric,
                    date,
                    value,
                    props,
                    name,
                    metric_type,
                    unit,
                    direction,
                    purpose,
                    how_to_read,
                },
            )
            .collect())
    }

    /// Latest weekly trend point per metric for a project — reads
    /// `sensei.project_metric_trend` (the weekly `lag()` view), keeping the newest
    /// `period` per metric (`DISTINCT ON`). Powers the trend arrow on the project
    /// metrics endpoint: `prior`/`delta` are `None` for a metric with a single
    /// weekly period (honest-null, never a fabricated 0). Empty when the project
    /// has no daily rows yet (honest-empty, not a failure). Propagates the read
    /// error; never masks it.
    pub async fn get_project_metric_trend(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<ProjectMetricTrendRow>, String> {
        let rows: Vec<(String, chrono::NaiveDate, f64, Option<f64>, Option<f64>, String)> =
            sqlx_core::query_as::query_as(
                "SELECT DISTINCT ON (metric)
                        metric, period, value::float8, prior::float8, delta::float8, direction::text
                   FROM sensei.project_metric_trend
                  WHERE project_id = $1
                  ORDER BY metric, period DESC",
            )
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(metric, period, value, prior, delta, direction)| ProjectMetricTrendRow {
                metric,
                period,
                value,
                prior,
                delta,
                direction,
            })
            .collect())
    }

    /// The time series for ONE metric of a project at a chosen `grain`, read from
    /// the matching roll-up view: `daily` → `project_metric_daily` (raw stored
    /// values); `weekly`/`monthly`/`quarterly` → the roll-up view that re-derives
    /// each period from sums (Σnum/Σden for ratio/pct — NEVER the mean of daily
    /// ratios). `grain` MUST be one of `daily`|`weekly`|`monthly`|`quarterly`; any
    /// other value is an `Err` (the caller 400s) rather than a silent default that
    /// would mismeasure. An unknown metric key — or a project with no rows — yields
    /// an empty `points` list (honest-empty, not a failure). Propagates the read
    /// error; never masks it.
    ///
    /// The metric's `formula` (the registry's "how it's calculated" facet) travels
    /// with the series so the detail screen renders it beside the chart. It is read
    /// from `sensei.metrics` by key — independent of the view — so it is present
    /// even when `points` is empty (a valid metric with no data yet) and is `None`
    /// only when the key names no registered metric (honest-null).
    pub async fn get_project_metric_series(
        &self,
        project_id: &uuid::Uuid,
        key: &str,
        grain: &str,
    ) -> Result<ProjectMetricSeries, String> {
        // The view + its period column are chosen from a fixed allowlist keyed on
        // the validated grain — no user-supplied string ever reaches the SQL, so
        // the `format!` is injection-safe.
        // `explainer_col` is chosen from the same validated-grain allowlist: only the
        // daily base view carries `props` (the per-datapoint explainer lives there),
        // so coarser grains select a literal NULL — the explainer is a per-day
        // artifact and is never rolled up. No user string reaches the SQL.
        let (view, period_col, explainer_col) = match grain {
            "daily" => ("sensei.project_metric_daily", "date", "props->>'explainer'"),
            "weekly" => ("sensei.project_metric_weekly", "period", "null::text"),
            "monthly" => ("sensei.project_metric_monthly", "period", "null::text"),
            "quarterly" => ("sensei.project_metric_quarterly", "period", "null::text"),
            other => return Err(format!("invalid grain: {other:?}")),
        };
        let sql = format!(
            "SELECT {period_col} AS period, value::float8, direction::text, {explainer_col} AS explainer
               FROM {view}
              WHERE project_id = $1 AND metric = $2
              ORDER BY {period_col}",
        );
        let rows: Vec<(chrono::NaiveDate, f64, String, Option<String>)> =
            sqlx_core::query_as::query_as(&sql)
                .bind(project_id)
                .bind(key)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        let points = rows
            .into_iter()
            .map(|(period, value, direction, explainer)| ProjectMetricSeriesPoint {
                period,
                value,
                direction,
                explainer,
            })
            .collect();
        // `formula` is a metric-level facet, read by key from the registry so it
        // survives an empty series and stays honest-null for an unknown key.
        let formula: Option<String> = sqlx_core::query_as::query_as::<_, (String,)>(
            "SELECT formula FROM sensei.metrics WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?
        .map(|(f,)| f);
        Ok(ProjectMetricSeries { formula, points })
    }

    /// The descriptive meaning of ONE metric by registry key — its display `name`
    /// and `how_to_read` line — for grounding the drill-down's per-session
    /// observation. Reads `sensei.metrics` by key (the same by-key path
    /// [`Self::get_project_metric_series`] reads `formula` through), independent of
    /// any project rows. `None` when the key names no registered metric
    /// (honest-null, never a fabricated meaning). Propagates the read error; never
    /// masks it.
    pub async fn get_metric_meaning(
        &self,
        key: &str,
    ) -> Result<Option<crate::db::pg_store::MetricMeaning>, String> {
        let row: Option<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT name, how_to_read FROM sensei.metrics WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(name, how_to_read)| crate::db::pg_store::MetricMeaning { name, how_to_read }))
    }

    /// The full daily series (chronological) for EVERY metric of a project, in a
    /// single query — reads `sensei.project_metric_daily` (project scope) and
    /// groups the values by metric, keeping each metric's points in `date` order.
    /// Powers the narrative's overall-trend fact: the direction the daily
    /// sparkline shows, so a rising `lower_better` metric reads as worsening even
    /// when its most-recent weekly step dipped. Empty when the project has no
    /// daily rows yet (honest-empty, not a failure). Propagates the read error;
    /// never masks it.
    pub async fn get_project_metric_daily_series_all(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<std::collections::HashMap<String, Vec<f64>>, String> {
        let rows: Vec<(String, f64)> = sqlx_core::query_as::query_as(
            "SELECT metric, value::float8
               FROM sensei.project_metric_daily
              WHERE project_id = $1
              ORDER BY metric, date",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let mut by_metric: std::collections::HashMap<String, Vec<f64>> =
            std::collections::HashMap::new();
        for (metric, value) in rows {
            by_metric.entry(metric).or_default().push(value);
        }
        Ok(by_metric)
    }

    /// Effectiveness aggregated by model: per (provider, model), how many
    /// enriched sessions, the First-Try Rate, average corrections, and average
    /// turns. The cross-model comparison the multi-model corpus (Zed + Claude)
    /// unlocks. Ordered by session volume.
    pub async fn get_model_effectiveness(&self) -> Result<Vec<serde_json::Value>, String> {
        // Raw per-(provider, raw-model) SUMS; folded by canonical model in Rust
        // (re-weighting FTR) so label variants aggregate — see model_insight.
        let rows: Vec<(Option<String>, String, i64, i64, i64, i64)> =
            sqlx_core::query_as::query_as(
                "SELECT provider, model,
                    count(*) AS sessions,
                    count(*) FILTER (WHERE ftr)::int8 AS ftr_sessions,
                    sum(corrections)::int8 AS corrections,
                    sum(turns)::int8 AS turns
               FROM activity.sessions
              WHERE model IS NOT NULL AND analyzed_at IS NOT NULL
              GROUP BY provider, model",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        let raw = rows
            .into_iter()
            .map(|(provider, model, sessions, ftr, corr, turns)| {
                (provider.unwrap_or_default(), model, sessions, ftr, corr, turns)
            })
            .collect();
        Ok(crate::model_insight::fold_effectiveness(raw))
    }

    /// Per-(provider, canonical-model) FTR over a project's enriched, model-tagged
    /// sessions — the input to the model-effectiveness recommendation. Label
    /// variants are folded to a canonical model (model_insight::fold_model_stats).
    pub async fn get_project_model_stats(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<crate::model_insight::ModelStat>, String> {
        let rows: Vec<(Option<String>, String, i64, i64)> = sqlx_core::query_as::query_as(
            "SELECT provider, model, count(*) AS sessions,
                    count(*) FILTER (WHERE ftr)::int8 AS ftr_sessions
               FROM activity.sessions
              WHERE project_id = $1 AND model IS NOT NULL AND analyzed_at IS NOT NULL
              GROUP BY provider, model",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let raw = rows
            .into_iter()
            .map(|(provider, model, sessions, ftr)| {
                (provider.unwrap_or_default(), model, sessions, ftr)
            })
            .collect();
        Ok(crate::model_insight::fold_model_stats(raw))
    }

    pub async fn get_project_ftr(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<serde_json::Value, String> {
        // Headline re-derived from the daily `ftr` rows in
        // `sensei.project_metric_daily` (metric='ftr') — the single FTR source of
        // truth. `ftr14d` reuses [`Self::get_project_ftr_rate`] (same 14d Σnum/Σden)
        // so the window can't drift between the two; `ftr14dPrev` is the same
        // pooled ratio over the prior-14d window and `sessions7d` is Σdenominator
        // over 7d. Scoped to the analyzed base (`outcome is not null`, the store's
        // denominator); `nullif(...,0)` keeps an empty window honest-null.
        let ftr_14d = self.get_project_ftr_rate(project_id).await?;
        let (ftr_14d_prev, sessions_7d): (Option<f64>, i64) =
            sqlx_core::query_as::query_as(
                "SELECT
                   (sum((props->>'numerator')::float8) FILTER (WHERE date > current_date - 28 AND date <= current_date - 14)
                      / nullif(sum((props->>'denominator')::float8) FILTER (WHERE date > current_date - 28 AND date <= current_date - 14), 0))::float8,
                   coalesce(sum((props->>'denominator')::int8) FILTER (WHERE date > current_date - 7), 0)::int8
                 FROM sensei.project_metric_daily
                 WHERE metric = 'ftr' AND project_id = $1"
            ).bind(project_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;

        // 14-day daily trend array — reads activity.sessions directly, filtered to
        // the SAME analyzed base as the headline (`outcome is not null`) so the
        // last trend point agrees with `ftr14d` for a day with in-flight sessions.
        let daily: Vec<(chrono::NaiveDate, Option<f64>)> =
            sqlx_core::query_as::query_as(
                "SELECT date_trunc('day', started_at)::date AS day,
                        AVG(CASE WHEN ftr THEN 1.0 ELSE 0.0 END)::float8 AS daily_ftr
                 FROM activity.sessions
                 WHERE project_id = $1 AND outcome IS NOT NULL AND outcome <> 'empty'::sensei.session_outcome AND started_at > now() - interval '14d'
                 GROUP BY day ORDER BY day"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let trend: Vec<f64> = daily.into_iter().map(|(_, v)| v.unwrap_or(0.0)).collect();

        Ok(Self::ftr_headline_json(ftr_14d, ftr_14d_prev, trend, sessions_7d))
    }

    /// The project's 14-day session-weighted FTR — Σ(`props.numerator`) /
    /// Σ(`props.denominator`) over the daily `ftr` rows in
    /// `sensei.project_metric_daily` (metric='ftr'), the single FTR source of
    /// truth. Same 14d window and derivation as [`Self::get_project_ftr`]'s
    /// `ftr14d` (which calls this), so both agree. Returns `None` when the project
    /// has no `ftr` rows in the window — honest-absent, NEVER a fabricated `0`.
    /// Shared by the legacy `/api/metrics/{project}` route and the MCP
    /// `get_metrics` tool so those surfaces report the same number.
    pub async fn get_project_ftr_rate(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Option<f64>, String> {
        let row: (Option<f64>,) = sqlx_core::query_as::query_as(
            "SELECT (sum((props->>'numerator')::float8)
                       / nullif(sum((props->>'denominator')::float8), 0))::float8
               FROM sensei.project_metric_daily
              WHERE metric = 'ftr' AND project_id = $1 AND date > current_date - 14",
        )
        .bind(project_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Holistic First-Try-Right rollup across all sessions — powers the
    /// Observatory · Today header. Mirrors [`Self::get_project_ftr`] without the
    /// project filter: the 14d / prior-14d headline is session-weighted
    /// (fraction of FTR-scored sessions, honest-null when there are none), and
    /// the trend is a fixed 14 calendar-day array (0-filled on empty days) so the
    /// sparkline always has 14 points.
    pub async fn get_holistic_ftr(&self) -> Result<serde_json::Value, String> {
        let row: (Option<f64>, Option<f64>, i64) = sqlx_core::query_as::query_as(
            "SELECT
               (avg(CASE WHEN ftr THEN 1.0 ELSE 0.0 END)
                  FILTER (WHERE ftr IS NOT NULL AND started_at > now() - interval '14 days'))::float8,
               (avg(CASE WHEN ftr THEN 1.0 ELSE 0.0 END)
                  FILTER (WHERE ftr IS NOT NULL
                          AND started_at <= now() - interval '14 days'
                          AND started_at >  now() - interval '28 days'))::float8,
               count(*) FILTER (WHERE started_at > now() - interval '7 days')
             FROM activity.sessions"
        ).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        let (ftr_14d, ftr_14d_prev, sessions_7d) = row;

        // Exactly 14 calendar-day trend points, oldest → newest, 0-filled on
        // days with no FTR-scored session.
        let daily: Vec<(chrono::NaiveDate, Option<f64>)> = sqlx_core::query_as::query_as(
            "SELECT d::date,
                    (SELECT avg(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)::float8
                       FROM activity.sessions s
                      WHERE date_trunc('day', s.started_at)::date = d::date
                        AND s.ftr IS NOT NULL
                        AND s.outcome <> 'empty'::sensei.session_outcome)
             FROM generate_series(current_date - 13, current_date, interval '1 day') d
             ORDER BY d",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let trend: Vec<f64> = daily.into_iter().map(|(_, v)| v.unwrap_or(0.0)).collect();

        Ok(Self::ftr_headline_json(ftr_14d, ftr_14d_prev, trend, sessions_7d))
    }

    // ── Metric watermarks (repo-grain compute cursor) ──────────────

    /// The `sealed_through` cursor for one (repository, metric_group) row of
    /// `sensei.metric_watermarks` — how far this group's calendar days are settled
    /// for the repo. `None` when no watermark row exists yet (the group has never
    /// been sealed for this repo → full-history fill), and equally `None` when the
    /// row exists but `sealed_through` is still NULL (never a fabricated date).
    /// Propagates the read error; never masks it.
    pub async fn metric_watermark_sealed_through(
        &self,
        repository_id: &uuid::Uuid,
        group: &str,
    ) -> Result<Option<chrono::NaiveDate>, String> {
        let row: Option<(Option<chrono::NaiveDate>,)> = sqlx_core::query_as::query_as(
            "SELECT sealed_through
               FROM sensei.metric_watermarks
              WHERE repository_id = $1 AND metric_group = $2",
        )
        .bind(repository_id)
        .bind(group)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        // No row → None; a row with NULL sealed_through → None (both = unset).
        Ok(row.and_then(|(d,)| d))
    }

    /// The MIN `sealed_through` across `repository_ids` for one metric group — the
    /// project-level cursor the day-keyed engine plans from (a project's group is
    /// only settled through the least-sealed of its repos). Honest full-fill signal:
    /// returns `None` if ANY requested repo is unset — no watermark row, or a row
    /// whose `sealed_through` is still NULL — so the engine refills the full history
    /// (spec test 1, min_date fill); otherwise `Some(min)`. Propagates the read
    /// error; never masks it.
    pub async fn min_sealed_through_for_repos(
        &self,
        repository_ids: &[uuid::Uuid],
        group: &str,
    ) -> Result<Option<chrono::NaiveDate>, String> {
        // No repos → nothing sealed → honest None (full fill / no-op upstream).
        if repository_ids.is_empty() {
            return Ok(None);
        }
        // Count the DISTINCT repos that have a non-null sealed_through and the MIN of
        // those dates in one read. A duplicate id in the input can't inflate the
        // count (COUNT DISTINCT), so the comparison against the distinct requested
        // set is exact.
        let row: (i64, Option<chrono::NaiveDate>) = sqlx_core::query_as::query_as(
            "SELECT count(DISTINCT repository_id) FILTER (WHERE sealed_through IS NOT NULL)::int8,
                    min(sealed_through)
               FROM sensei.metric_watermarks
              WHERE repository_id = ANY($1) AND metric_group = $2",
        )
        .bind(repository_ids)
        .bind(group)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let (sealed_repos, min_sealed) = row;
        let distinct_requested = {
            let mut ids = repository_ids.to_vec();
            ids.sort();
            ids.dedup();
            ids.len() as i64
        };
        // Any repo unset → min is undefined at the project level → full fill.
        if sealed_repos < distinct_requested { Ok(None) } else { Ok(min_sealed) }
    }

    /// Advance one (repository, metric_group) watermark to `sealed_through` (upsert).
    /// Called ONLY after a group's compute succeeds for the repo, so a failed group
    /// holds its cursor and retries next run (fail-closed — spec test 6). Today is
    /// never sealed: the caller passes `as_of - 1` (spec test 2). `last_sha` is left
    /// untouched — reserved for a future per-commit optimization; the day-bucketed
    /// churn + sampled quality groups seal by commit-DAY via `sealed_through`.
    /// Propagates the write error; never masks it.
    pub async fn advance_metric_watermark(
        &self,
        repository_id: &uuid::Uuid,
        group: &str,
        sealed_through: chrono::NaiveDate,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.metric_watermarks (repository_id, metric_group, sealed_through, updated_at)
             VALUES ($1, $2, $3, now())
             ON CONFLICT (repository_id, metric_group) DO UPDATE
                SET sealed_through = EXCLUDED.sealed_through,
                    updated_at     = now()",
        )
        .bind(repository_id)
        .bind(group)
        .bind(sealed_through)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}
