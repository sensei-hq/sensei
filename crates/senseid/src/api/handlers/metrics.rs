//! Metrics read endpoints (Phase 7): the active registry catalog, a project's
//! latest-per-metric values (with trend + the `project_health` composite), one
//! metric's series at a chosen grain (carrying the metric's `formula`), and the
//! measurable sessions behind one daily datapoint (`{key}/sessions?day=`). The
//! reads are pure reads over
//! [`PgStore::active_metrics`] / [`PgStore::get_project_metrics`] /
//! [`PgStore::get_project_metric_trend`] / [`PgStore::get_project_metric_series`]
//! and the `sensei.project_metric_*` views (the views own the aggregation — the
//! handlers never re-derive ratios in Rust).
//!
//! Fail-closed, honest results (see the #109 audit + the repo governance rules): a
//! fallible read propagates as a 500; a project that does not exist is a 404
//! (matching `GET /api/projects/{id}/ftr`); an invalid `grain` is a 400 (never a
//! silent default that would mismeasure); genuinely-absent data is an honest-empty
//! list, never a fabricated row/value.
//!
//! [`PgStore::active_metrics`]: crate::db::pg_store::PgStore::active_metrics
//! [`PgStore::get_project_metrics`]: crate::db::pg_store::PgStore::get_project_metrics
//! [`PgStore::get_project_metric_trend`]: crate::db::pg_store::PgStore::get_project_metric_trend
//! [`PgStore::get_project_metric_series`]: crate::db::pg_store::PgStore::get_project_metric_series

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

use crate::api::state::AppState;

/// `GET /api/metrics/registry` — the ACTIVE metric catalog ([`active_metrics`]):
/// every metric live on `current_date`, each carrying its self-describing facets
/// (`purpose`, `direction`, `how_to_read`, `formula`, …) so a client renders the
/// catalog from the daemon-owned registry rather than a hardcoded list.
///
/// [`active_metrics`]: crate::db::pg_store::PgStore::active_metrics
pub(crate) async fn get_metrics_registry(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let metrics = state.pg.active_metrics().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "metrics": metrics, "count": metrics.len() })))
}

/// The `sensei.reason_codes` domain the metric status vocabulary lives in. Named
/// once so the reader and the doc comment cannot disagree about which slice of the
/// registry this endpoint serves.
const METRIC_REASON_DOMAIN: &str = "metric_computation";

/// The reason vocabulary for this domain, keyed by code.
///
/// Served with BOTH status shapes rather than from a third endpoint: it is seven
/// small rows, and a client that had to fetch it separately could render a row
/// whose code it cannot resolve — a bare slug, which is the failure the registry
/// exists to prevent. A read failure propagates for the same reason: rows with an
/// empty vocabulary are worse than no rows.
async fn reason_vocabulary(
    state: &AppState,
) -> Result<std::collections::BTreeMap<String, serde_json::Value>, StatusCode> {
    let reasons = state
        .pg
        .reason_codes(METRIC_REASON_DOMAIN)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(reasons.into_iter().map(|r| (r.code.clone(), serde_json::json!(r))).collect())
}

/// `?repo=<repo_key|uuid>` — which repository's metrics to report. REQUIRED.
#[derive(Deserialize)]
pub(crate) struct StatusQuery {
    repo: Option<String>,
}

/// `GET /api/metrics/status?repo=<repo_key|uuid>` — one repository's per-metric
/// computation state: should this compute, how far has it got, and if it is not
/// current, WHY.
///
/// One repository at a time, and `repo` is REQUIRED (400 when absent). The
/// underlying view cross-joins the catalogue, so "every repository" is
/// `repositories × metrics` and unbounded — 1,943 rows on this install but
/// 10,928,780 in `sensei_test`, where an unfiltered read exhausted the request.
/// [`get_metric_status_summary`] is the whole-estate shape, aggregated in SQL.
///
/// A repository is also the grain a deactivation is DECIDED at (one tenant, one
/// repository, one metric), so this read and the write that changes it
/// (`PATCH /api/dojo/metric-activation`) speak the same shape.
///
/// `repo` accepts a `repo_key` OR a uuid, matching the name-or-uuid convention the
/// project endpoints already use: `repo_key` is null for a local-only repository,
/// whose metrics compute normally, so a key-only parameter would leave it
/// unaddressable.
///
/// Rows carry a `reason_code`; `reasons` resolves it (see [`reason_vocabulary`]).
/// An unknown `repo` is a 404, decided on `sensei.repositories` rather than on an
/// empty result — a known repository always has rows, so a 200 with `[]` would
/// read as "this repository has no metrics".
pub(crate) async fn get_metric_status(
    State(state): State<AppState>,
    Query(q): Query<StatusQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let reference = q
        .repo
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let repository_id = state
        .pg
        .resolve_repository_id(reference)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let rows = state
        .pg
        .metric_status(&repository_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let reasons = reason_vocabulary(&state).await?;
    // The repository's identity travels once per response, not once per row. Both
    // are honest-null when the registry is empty — the one case with no rows to read
    // them from — never an echo of whatever the caller passed in.
    let name = rows.first().map(|r| r.repository_name.clone());
    let repo_key = rows.first().and_then(|r| r.repo_key.clone());

    Ok(Json(serde_json::json!({
        "repository_id": repository_id,
        "repo_key": repo_key,
        "name": name,
        "metrics": rows,
        "reasons": reasons,
        "count": rows.len(),
    })))
}

/// `GET /api/metrics/status/summary` — the whole estate at one row per
/// (repository × reason), so the landing view costs the same whether a repository
/// carries 29 metrics or 3,000.
///
/// `by_reason` is a code → count map; the caller ranks it using the `precedence`
/// that travels in `reasons`. Ranking is deliberately NOT done here — precedence is
/// owned by `sensei.reason_codes`, and a worst-first ordering computed in SQL would
/// be a second copy of it.
///
/// Entries are keyed on `repository_id`, and `repo_key` is nullable: a local-only
/// repository computes metrics normally but no dōjō can rule on them, so the client
/// renders its state without an activation control. Grouping on the key instead
/// would fold every such repository into one entry whose counts belong to none of
/// them.
pub(crate) async fn get_metric_status_summary(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let rows =
        state.pg.metric_status_summary().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let reasons = reason_vocabulary(&state).await?;

    /// One repository's accumulating entry.
    struct Entry {
        repo_key: Option<String>,
        by_reason: serde_json::Map<String, serde_json::Value>,
        total: i64,
    }

    // Keyed accumulation, NOT a run-length group over a sorted stream. The
    // run-length form was written first and is wrong here: it emits a repository
    // once per CONTIGUOUS run, so any change to the SQL ordering silently splits a
    // repository into several entries with partial counts — the same repository
    // listed twice with different numbers. Whether it splits depends on the
    // planner's aggregate output order, so it also cannot be reliably tested.
    // A map makes one-entry-per-repository structural, and `BTreeMap` keyed on
    // (name, id) makes the response order deterministic without the SQL owning it.
    let mut by_repo: std::collections::BTreeMap<(String, uuid::Uuid), Entry> =
        std::collections::BTreeMap::new();
    for (id, repo_key, name, reason_code, count) in rows {
        let entry = by_repo.entry((name, id)).or_insert_with(|| Entry {
            repo_key,
            by_reason: serde_json::Map::new(),
            total: 0,
        });
        entry.by_reason.insert(reason_code, serde_json::json!(count));
        entry.total += count;
    }

    let repositories: Vec<serde_json::Value> = by_repo
        .into_iter()
        .map(|((name, id), e)| {
            serde_json::json!({
                "repository_id": id,
                "repo_key": e.repo_key,
                "name": name,
                "by_reason": e.by_reason,
                "total": e.total,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "count": repositories.len(),
        "repositories": repositories,
        "reasons": reasons,
    })))
}

/// `GET /api/projects/{id}/metrics` — latest-per-metric values for a project
/// (project scope, daily grain) with the catalog facets attached and the weekly
/// trend (`prior`/`delta`) merged in where available. Includes the
/// `project_health` composite (it is a metric). `{id}` is name-or-uuid.
///
/// Fail-closed: a lookup error is a 500; a project that does not exist is a 404
/// (matching `GET /api/projects/{id}/ftr`); a project with no metric rows yet is a
/// 200 with an empty list (honest-empty, never a fabricated row). Trend is
/// `null`/`null` for a metric without a prior weekly period — an honest gap, not a
/// fabricated 0.
pub(crate) async fn get_project_metrics(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid =
        crate::api::util::resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    state
        .pg
        .get_project(&uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let rows =
        state.pg.get_project_metrics(&uuid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let trend = state
        .pg
        .get_project_metric_trend(&uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let trend_by_metric: std::collections::HashMap<String, (Option<f64>, Option<f64>)> =
        trend.into_iter().map(|t| (t.metric, (t.prior, t.delta))).collect();

    let mut metrics = Vec::with_capacity(rows.len());
    for row in rows {
        let (prior, delta) = trend_by_metric.get(&row.metric).copied().unwrap_or((None, None));
        let mut value =
            serde_json::to_value(&row).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if let Some(obj) = value.as_object_mut() {
            obj.insert("prior".into(), serde_json::json!(prior));
            obj.insert("delta".into(), serde_json::json!(delta));
        }
        metrics.push(value);
    }

    // The full daily series per metric — the narrative reads the OVERALL trend
    // from this (the direction the sparkline shows), so a one-week dip can't be
    // reported as the trend. A read error is a 500 (fail-closed), never a
    // fabricated-empty that would silently drop the trend fact.
    let series_by_metric = state
        .pg
        .get_project_metric_daily_series_all(&uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Interpreted layer: the local-model headline + per-signal "what sensei
    // noticed" prose, read from cache (a cold miss is warmed in the background
    // and omitted — the app renders its own deterministic sentence in the gap).
    // Inference never runs on this request path (see metric_narrative).
    let narrative = crate::analysis::metric_narrative::build_narrative(
        &state.pg,
        &state.gateway,
        &metrics,
        &series_by_metric,
    )
    .await;

    Ok(Json(serde_json::json!({
        "metrics": metrics,
        "count": metrics.len(),
        "narrative": narrative,
    })))
}

/// Query for `GET /api/projects/{id}/metrics/{key}`.
#[derive(Deserialize)]
pub(crate) struct SeriesQuery {
    /// Roll-up grain — one of `daily`|`weekly`|`monthly`|`quarterly`. Absent →
    /// `daily` (the finest, raw grain). Any other value is a 400 (never a silent
    /// fallback to a grain the caller did not ask for).
    grain: Option<String>,
}

/// The grains a series can be read at — the allowlist behind the 400 on an invalid
/// `?grain=`. Kept next to the handler that validates against it.
const SERIES_GRAINS: [&str; 4] = ["daily", "weekly", "monthly", "quarterly"];

/// `GET /api/projects/{id}/metrics/{key}?grain=weekly` — the time series for one
/// metric at a grain, from the matching roll-up view (ratios re-derived Σnum/Σden
/// per period by the view, never the mean of daily ratios). `{id}` is name-or-uuid.
///
/// Fail-closed: a lookup error is a 500; a project that does not exist is a 404; an
/// invalid `grain` is a 400 (never a silent default that would mismeasure); an
/// unknown metric key (no rows) is a 200 with an empty series (honest-empty).
pub(crate) async fn get_project_metric_series(
    State(state): State<AppState>,
    Path((id, key)): Path<(String, String)>,
    Query(q): Query<SeriesQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let grain = q.grain.as_deref().unwrap_or("daily");
    if !SERIES_GRAINS.contains(&grain) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let uuid =
        crate::api::util::resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    state
        .pg
        .get_project(&uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let series = state
        .pg
        .get_project_metric_series(&uuid, &key, grain)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({
        "metric": key,
        "grain": grain,
        // The metric's `formula` (the registry's "how it's calculated" facet)
        // travels with the series so the detail screen renders it beside the chart.
        // Honest-null when the key names no registered metric; present even when the
        // series is empty (a valid metric with no data yet).
        "formula": series.formula,
        "series": series.points,
        "count": series.points.len(),
    })))
}

/// Query for `GET /api/projects/{id}/metrics/{key}/sessions`.
#[derive(Deserialize)]
pub(crate) struct DaySessionsQuery {
    /// The calendar day to drill into, `YYYY-MM-DD`. Required: without a day there
    /// is no datapoint to scope to, so an absent or unparseable value is a 400
    /// (never a silent default that would return the wrong day's sessions).
    day: Option<String>,
}

/// `GET /api/projects/{id}/metrics/{key}/sessions?day=YYYY-MM-DD` — the measurable
/// sessions behind one daily metric datapoint (the datapoint→sessions drill-down).
/// Each session carries the structural fields the client renders a one-liner from
/// (`outcome` + `ftr` + `turns` + `corrections`) plus the `client_session_id`
/// reference, `started_at`, `task`, the existing `summary` column (the "what was
/// achieved" half), and an `observation` (`{ title, detail }`) — the per-session,
/// per-metric "why this session moved this metric" line (the drill-down's other
/// half). `{id}` is name-or-uuid; `{key}` names the drilled-into metric — it
/// selects which metric the observation is written against (the meaning it reads),
/// though the measurable-session *base* is the same per day across the daily
/// metrics.
///
/// The observation is a wire-path read via
/// [`session_metric_observation`](crate::analysis::session_metric_note::session_metric_observation):
/// INTENDED (not a bug) — the FIRST drill-down for a (session, metric) returns the
/// deterministic, row-derived fallback line and warms the model copy off-wire; the
/// NEXT load returns the model copy. Inference never blocks this request path.
///
/// Fail-closed: a lookup error is a 500; a project that does not exist is a 404; an
/// absent or malformed `day` is a 400; a day with no measurable session is a 200
/// with an empty list (honest-empty, never a fabricated row).
/// `GET /api/projects/{id}/tools`: the per-tool usage breakdown (which tools the
/// project's ACPs actually invoked, with call/failure counts) behind the
/// tool-usage bubble view + "which tools were used" evidence. Fail-closed: 500 on
/// a read error, 404 for an unknown project, 200 with an empty list when no tool
/// usage was captured (honest-empty).
pub(crate) async fn get_project_tools(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid =
        crate::api::util::resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    state
        .pg
        .get_project(&uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let tools = state
        .pg
        .get_project_tool_breakdown(&uuid, 24)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let count = tools.len();
    Ok(Json(serde_json::json!({ "tools": tools, "count": count })))
}

pub(crate) async fn get_project_metric_day_sessions(
    State(state): State<AppState>,
    Path((id, key)): Path<(String, String)>,
    Query(q): Query<DaySessionsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let day = q
        .day
        .as_deref()
        .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let uuid =
        crate::api::util::resolve_project_uuid(&state, &id).await?.ok_or(StatusCode::NOT_FOUND)?;
    state
        .pg
        .get_project(&uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut sessions = state
        .pg
        .get_project_sessions_for_day(&uuid, day)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let count = sessions.len();

    // The metric's meaning, looked up ONCE per request from the registry by key
    // (fail-closed on a read error). Honest-null for an unregistered key → the
    // observation label falls back to the key and the meaning stays empty.
    let (label, how_to_read) = match state
        .pg
        .get_metric_meaning(&key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        Some(m) => (m.name, m.how_to_read),
        None => (key.clone(), String::new()),
    };

    // Per-session, per-metric observation — grounded strictly in the session ROW
    // + the metric's meaning, never fabricated. copy_or_warm returns the cached
    // model copy on a hit or the deterministic fallback on a miss (see the doc
    // comment above — the warm is off-wire, never on this request path).
    for session in &mut sessions {
        let facts = crate::analysis::session_metric_note::SessionMetricFacts::from_session_row(
            session,
            &key,
            &label,
            &how_to_read,
        );
        let obs = crate::analysis::session_metric_note::session_metric_observation(
            &state.pg,
            &state.gateway,
            &facts,
        )
        .await;
        if let Some(obj) = session.as_object_mut() {
            obj.insert(
                "observation".into(),
                serde_json::json!({ "title": obs.title, "detail": obs.detail }),
            );
        }
    }

    Ok(Json(serde_json::json!({
        "metric": key,
        "day": day.to_string(),
        "sessions": sessions,
        "count": count,
    })))
}
