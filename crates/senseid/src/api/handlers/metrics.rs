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
    let metrics = state
        .pg
        .active_metrics()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "metrics": metrics, "count": metrics.len() })))
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
    let uuid = crate::api::util::resolve_project_uuid(&state, &id)
        .await?
        .ok_or(StatusCode::NOT_FOUND)?;
    state
        .pg
        .get_project(&uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let rows = state
        .pg
        .get_project_metrics(&uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let trend = state
        .pg
        .get_project_metric_trend(&uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let trend_by_metric: std::collections::HashMap<String, (Option<f64>, Option<f64>)> = trend
        .into_iter()
        .map(|t| (t.metric, (t.prior, t.delta)))
        .collect();

    let mut metrics = Vec::with_capacity(rows.len());
    for row in rows {
        let (prior, delta) = trend_by_metric
            .get(&row.metric)
            .copied()
            .unwrap_or((None, None));
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
    let uuid = crate::api::util::resolve_project_uuid(&state, &id)
        .await?
        .ok_or(StatusCode::NOT_FOUND)?;
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
    let uuid = crate::api::util::resolve_project_uuid(&state, &id)
        .await?
        .ok_or(StatusCode::NOT_FOUND)?;
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
    let uuid = crate::api::util::resolve_project_uuid(&state, &id)
        .await?
        .ok_or(StatusCode::NOT_FOUND)?;
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
            session, &key, &label, &how_to_read,
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
