use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    pub async fn create_benchmark_report(
        &self, folder_id: Option<&uuid::Uuid>, run_name: &str, strategy: &str,
        score: Option<f64>, tokens: Option<i32>, elapsed_ms: Option<i32>,
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
        type Row = (
            uuid::Uuid, Option<uuid::Uuid>, f64, f64,
            Option<Vec<String>>, String,
        );
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
          LEFT JOIN inference.reasoning_traces t ON t.id = r.reasoning_trace_id"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        if rows.is_empty() { return Ok(0); }

        let mut updated: i64 = 0;
        for (rec_id, trace_id, baseline_ftr, current_ftr, models_used_opt, based_on) in rows {
            let verdict = crate::verdicts::Verdict::from_ftr_delta(current_ftr - baseline_ftr);
            let models_used = models_used_opt.unwrap_or_default();
            let consensus = crate::verdicts::synthesize_reasoning(
                verdict, baseline_ftr, current_ftr, &models_used,
            );

            let upd = sqlx_core::query::query(
                "UPDATE inference.recommendations
                    SET verdict     = $2::sensei.recommendation_verdict,
                        current_ftr = $3,
                        measured_at = now()
                  WHERE id = $1 AND verdict = 'pending'"
            )
            .bind(rec_id)
            .bind(verdict.as_wire())
            .bind(current_ftr)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;

            // The `verdict = 'pending'` guard makes the flip win exactly once, so a
            // concurrent scheduler tick can't measure (or challenge) the same rec
            // twice. `rows_affected == 0` means another tick already claimed it.
            if upd.rows_affected() == 0 { continue; }
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
                        "UPDATE inference.reasoning_traces SET consensus = $2 WHERE id = $1"
                    ).bind(id).bind(&consensus).execute(&self.pool).await {
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
    pub async fn get_ftr_daily(&self, project_id: Option<&uuid::Uuid>, days: i32) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(chrono::NaiveDate, Option<f64>, Option<i64>)> = if let Some(pid) = project_id {
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

    pub async fn get_hotspots(&self, project_id: &uuid::Uuid, days: i32) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, i64, i64)> = sqlx_core::query_as::query_as(
            "SELECT folder, file_path, edit_count, correction_count
             FROM sensei.project_hotspots
             WHERE project_id = $1 AND last_event_at >= (now() - ($2::int || ' days')::interval)
             ORDER BY (edit_count + correction_count) DESC LIMIT 20"
        ).bind(project_id).bind(days).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(folder, path, edits, corrections)| {
            serde_json::json!({ "folder": folder, "file_path": path, "edit_count": edits, "correction_count": corrections })
        }).collect())
    }

    pub async fn get_quality_signals(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
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
    pub async fn upsert_project_metric(
        &self,
        metric_id:   &uuid::Uuid,
        project_id:  &uuid::Uuid,
        folder_id:   Option<&uuid::Uuid>,
        session_id:  Option<&uuid::Uuid>,
        computed_on: chrono::NaiveDate,
        grain:       &str,
        value:       f64,
        props:       &serde_json::Value,
        source:      &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.project_metrics
                (metric_id, project_id, folder_id, session_id, computed_on, grain, value, props, source)
             VALUES ($1, $2, $3, $4, $5, $6::sensei.metric_grain, $7::float8::numeric, $8, $9::sensei.metric_source)
             ON CONFLICT (metric_id, project_id, folder_id, session_id, computed_on, grain) DO UPDATE
                SET value       = EXCLUDED.value,
                    props       = EXCLUDED.props,
                    source      = EXCLUDED.source,
                    modified_at = now()
             RETURNING id",
        )
        .bind(metric_id)
        .bind(project_id)
        .bind(folder_id)
        .bind(session_id)
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
            uuid::Uuid, String, String, String, String, String, Option<String>, String,
            String, String, String, String, f64, Option<f64>, chrono::NaiveDate, Option<chrono::NaiveDate>,
        )> = sqlx_core::query_as::query_as(&sql)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(
                id, key, name, description, family, metric_type, unit, direction,
                purpose, how_to_read, formula, task_name, weight, target, effective_from, effective_until,
            )| Metric {
                id, key, name, description, family, metric_type, unit, direction,
                purpose, how_to_read, formula, task_name, weight, target, effective_from, effective_until,
            })
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
        let rows: Vec<(
            String, chrono::NaiveDate, f64, serde_json::Value, String, String,
            Option<String>, String, String, String,
        )> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT ON (d.metric)
                    d.metric, d.date, d.value::float8, d.props,
                    m.name, m.type::text, m.unit, m.direction::text, m.purpose, m.how_to_read
               FROM sensei.project_metric_daily d
               JOIN sensei.metrics m ON m.key = d.metric
              WHERE d.project_id = $1
              ORDER BY d.metric, d.date DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(
                metric, date, value, props, name, metric_type, unit, direction, purpose, how_to_read,
            )| ProjectMetricRow {
                metric, date, value, props, name, metric_type, unit, direction, purpose, how_to_read,
            })
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
                metric, period, value, prior, delta, direction,
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
        let (view, period_col) = match grain {
            "daily"     => ("sensei.project_metric_daily",     "date"),
            "weekly"    => ("sensei.project_metric_weekly",    "period"),
            "monthly"   => ("sensei.project_metric_monthly",   "period"),
            "quarterly" => ("sensei.project_metric_quarterly", "period"),
            other => return Err(format!("invalid grain: {other:?}")),
        };
        let sql = format!(
            "SELECT {period_col} AS period, value::float8, direction::text
               FROM {view}
              WHERE project_id = $1 AND metric = $2
              ORDER BY {period_col}",
        );
        let rows: Vec<(chrono::NaiveDate, f64, String)> = sqlx_core::query_as::query_as(&sql)
            .bind(project_id)
            .bind(key)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        let points = rows
            .into_iter()
            .map(|(period, value, direction)| ProjectMetricSeriesPoint { period, value, direction })
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
        let rows: Vec<(Option<String>, String, i64, i64, i64, i64)> = sqlx_core::query_as::query_as(
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
        &self, project_id: &uuid::Uuid,
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
            .map(|(provider, model, sessions, ftr)| (provider.unwrap_or_default(), model, sessions, ftr))
            .collect();
        Ok(crate::model_insight::fold_model_stats(raw))
    }

    pub async fn get_project_ftr(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
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
                 WHERE project_id = $1 AND outcome IS NOT NULL AND started_at > now() - interval '14d'
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
    pub async fn get_project_ftr_rate(&self, project_id: &uuid::Uuid) -> Result<Option<f64>, String> {
        let row: (Option<f64>,) = sqlx_core::query_as::query_as(
            "SELECT (sum((props->>'numerator')::float8)
                       / nullif(sum((props->>'denominator')::float8), 0))::float8
               FROM sensei.project_metric_daily
              WHERE metric = 'ftr' AND project_id = $1 AND date > current_date - 14"
        ).bind(project_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
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
                        AND s.ftr IS NOT NULL)
             FROM generate_series(current_date - 13, current_date, interval '1 day') d
             ORDER BY d"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        let trend: Vec<f64> = daily.into_iter().map(|(_, v)| v.unwrap_or(0.0)).collect();

        Ok(Self::ftr_headline_json(ftr_14d, ftr_14d_prev, trend, sessions_7d))
    }

}
