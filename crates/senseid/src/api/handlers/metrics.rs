//! Metrics read endpoints (Phase 7): the active registry catalog, a project's
//! latest-per-metric values (with trend + the `project_health` composite), and
//! one metric's series at a chosen grain. All three are pure reads over
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

    // Interpreted layer: the local-model headline + per-signal "what sensei
    // noticed" prose, read from cache (a cold miss is warmed in the background
    // and omitted — the app renders its own deterministic sentence in the gap).
    // Inference never runs on this request path (see metric_narrative).
    let narrative =
        crate::analysis::metric_narrative::build_narrative(&state.pg, &state.gateway, &metrics).await;

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
        "series": series,
        "count": series.len(),
    })))
}
