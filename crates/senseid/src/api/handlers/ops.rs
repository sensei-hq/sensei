//! Operations alert dispositions — ack state that survives reloads.
//!
//! Alerts re-derive from the feeds; this surface persists only what
//! derivation cannot recompute: which standing alerts are acked, and how
//! many times each condition cleared (recurrence memory for escalation).

use axum::{extract::State, http::StatusCode, response::Json};
use serde_json::Value;

use super::err;
use crate::api::state::AppState;

/// The alert classes the center derives — the only values a disposition may
/// name. Mirrors the client's `AlertClass`; a disposition for a class that
/// derives nothing could never clear.
const ALERT_CLASSES: &[&str] = &["task", "schedule", "sync", "stream", "daemon"];

/// `GET /api/ops/alerts/acks` — every ack disposition.
pub(crate) async fn list_alert_acks(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let acks = state.pg.list_alert_acks().await.map_err(|e| {
        tracing::warn!(error = %e, "list_alert_acks: read failed");
        err(StatusCode::INTERNAL_SERVER_ERROR, "acks read failed")
    })?;
    Ok(Json(serde_json::json!({ "acks": acks })))
}

/// `PUT /api/ops/alerts/acks` — ack (or record a clear for) one alert.
///
/// Body `{ class, key, acked }`: the key rides in the body because alert
/// keys are paths (`abandoned:kind:/r/f.ts`), which no single route segment
/// can match. `acked=false` is a client-observed clear, not an unack — it
/// bumps recurrence memory.
pub(crate) async fn put_alert_ack(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let class = body.get("class").and_then(|v| v.as_str()).unwrap_or_default();
    if !ALERT_CLASSES.contains(&class) {
        return Err(err(StatusCode::BAD_REQUEST, format!("unknown alert class: {class:?}")));
    }
    let key = body.get("key").and_then(|v| v.as_str()).unwrap_or_default();
    if key.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "key must be a non-empty string"));
    }
    let Some(acked) = body.get("acked").and_then(|v| v.as_bool()) else {
        return Err(err(StatusCode::BAD_REQUEST, "acked must be true or false"));
    };
    let row = state.pg.set_alert_ack(class, key, acked).await.map_err(|e| {
        tracing::warn!(error = %e, class, key, "put_alert_ack: write failed");
        err(StatusCode::INTERNAL_SERVER_ERROR, "ack write failed")
    })?;
    Ok(Json(row))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn state() -> crate::api::state::AppState {
        crate::tasks::test_support::make_ctx().await.app_state.clone()
    }

    #[tokio::test]
    async fn put_ack_round_trips_through_get() {
        let state = state().await;
        let key = format!("ops-test-{}", uuid::Uuid::new_v4());
        let wrote = put_alert_ack(
            State(state.clone()),
            Json(serde_json::json!({ "class": "task", "key": key, "acked": true })),
        )
        .await
        .expect("ack writes");
        assert_eq!(wrote.0["acked"], true);
        let read = list_alert_acks(State(state.clone())).await.expect("acks read");
        let found = read.0["acks"]
            .as_array()
            .expect("acks array")
            .iter()
            .find(|r| r["key"] == key)
            .expect("the written ack lists");
        assert_eq!(found["acked"], true);
    }

    #[tokio::test]
    async fn put_ack_refuses_a_body_without_a_boolean() {
        let state = state().await;
        let err = put_alert_ack(
            State(state),
            Json(serde_json::json!({ "class": "task", "key": "x", "acked": "yes" })),
        )
        .await
        .expect_err("non-boolean acked is a 400");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn put_ack_refuses_an_unknown_class() {
        let state = state().await;
        let err = put_alert_ack(
            State(state),
            Json(serde_json::json!({ "class": "nag", "key": "x", "acked": true })),
        )
        .await
        .expect_err("unknown class is a 400");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }
}
