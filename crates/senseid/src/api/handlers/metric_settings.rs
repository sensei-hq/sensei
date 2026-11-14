//! A workspace's SDLC metric settings (spec tracker-sdlc-metrics S1). Behind
//! `origin_guard::local_caller_guard`, reads included: the hourly rate is not for
//! other origins to see.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::api::handlers::err;
use crate::api::handlers::trackers::known_workspace;
use crate::api::state::AppState;
use crate::sdlc::settings::parse_patch;

/// `GET /api/workspaces/{id}/metric-settings`
pub async fn get_metric_settings(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let workspace = match known_workspace(&state, &id).await {
        Ok(w) => w,
        Err(e) => return e.into_response(),
    };
    match state.pg.metric_settings(workspace).await {
        Ok(s) => Json(s).into_response(),
        Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "internal").into_response(),
    }
}

/// `PUT /api/workspaces/{id}/metric-settings`: any subset of the three keys; 400
/// `invalid_setting` naming the first bad key.
pub async fn put_metric_settings(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let workspace = match known_workspace(&state, &id).await {
        Ok(w) => w,
        Err(e) => return e.into_response(),
    };
    let patch = match parse_patch(&body) {
        Ok(p) => p,
        Err(key) => {
            let body = serde_json::json!({"error": "invalid_setting", "key": key});
            return (StatusCode::BAD_REQUEST, Json(body)).into_response();
        }
    };
    let internal = || err(StatusCode::INTERNAL_SERVER_ERROR, "internal").into_response();
    let pg = &state.pg;
    let Ok(now) = pg.metric_settings(workspace).await else { return internal() };
    let next = patch.apply(&now);
    if pg.put_metric_settings(workspace, &next).await.is_err() {
        return internal();
    }
    Json(next).into_response()
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{Value, json};
    use tower::ServiceExt;

    async fn app() -> (axum::Router, crate::api::state::AppState) {
        let ctx = crate::tasks::test_support::make_ctx().await;
        let state = ctx.app_state.clone();
        (crate::api::routes::create_router(state.clone()), state)
    }

    async fn call(
        app: &axum::Router,
        method: &str,
        uri: &str,
        body: Option<Value>,
        origin: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut b = Request::builder().method(method).uri(uri).header("host", "127.0.0.1:7744");
        if let Some(o) = origin {
            b = b.header("origin", o);
        }
        let req = match body {
            Some(v) => b
                .header("content-type", "application/json")
                .body(Body::from(v.to_string()))
                .unwrap(),
            None => b.body(Body::empty()).unwrap(),
        };
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    #[tokio::test]
    async fn defaults_set_clear_and_refuse() {
        let (app, state) = app().await;
        let w = state
            .pg
            .create_workspace(
                &format!("_test:metric-settings:{}", uuid::Uuid::new_v4()),
                None,
                None,
            )
            .await
            .unwrap();
        let uri = format!("/api/workspaces/{w}/metric-settings");

        assert_eq!(
            call(&app, "GET", &uri, None, None).await,
            (
                StatusCode::OK,
                json!({"hourly_rate": null, "minutes_per_point": null, "idle_threshold_minutes": 15})
            )
        );
        let set = call(
            &app,
            "PUT",
            &uri,
            Some(
                json!({"hourly_rate": 50, "minutes_per_point": 90.5, "idle_threshold_minutes": 20}),
            ),
            None,
        )
        .await;
        let want =
            json!({"hourly_rate": 50.0, "minutes_per_point": 90.5, "idle_threshold_minutes": 20});
        assert_eq!(set, (StatusCode::OK, want.clone()));
        assert_eq!(call(&app, "GET", &uri, None, None).await, (StatusCode::OK, want));

        let cleared = call(&app, "PUT", &uri, Some(json!({"hourly_rate": null})), None).await;
        assert_eq!(cleared.1["hourly_rate"], Value::Null);
        assert_eq!(cleared.1["idle_threshold_minutes"], json!(20), "a key the body omits is kept");

        for (body, key) in [
            (json!({"hourly_rate": "NaN"}), "hourly_rate"),
            (json!({"hourly_rate": -1}), "hourly_rate"),
            (json!({"idle_threshold_minutes": null}), "idle_threshold_minutes"),
            (json!({"x": 1}), "x"),
        ] {
            assert_eq!(
                call(&app, "PUT", &uri, Some(body.clone()), None).await,
                (StatusCode::BAD_REQUEST, json!({"error": "invalid_setting", "key": key})),
                "{body}"
            );
        }
        assert_eq!(
            call(&app, "GET", &uri, None, None).await.1["idle_threshold_minutes"],
            json!(20)
        );
    }

    #[tokio::test]
    async fn guarded_and_404_for_an_unknown_workspace() {
        let (app, state) = app().await;
        let w = state
            .pg
            .create_workspace(
                &format!("_test:metric-settings:{}", uuid::Uuid::new_v4()),
                None,
                None,
            )
            .await
            .unwrap();
        let uri = format!("/api/workspaces/{w}/metric-settings");
        assert_eq!(
            call(&app, "GET", &uri, None, Some("https://evil.com")).await.0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            call(&app, "PUT", &uri, Some(json!({"hourly_rate": 1})), Some("https://evil.com"))
                .await
                .0,
            StatusCode::FORBIDDEN
        );
        let ghost = uuid::Uuid::new_v4();
        assert_eq!(
            call(&app, "GET", &format!("/api/workspaces/{ghost}/metric-settings"), None, None)
                .await,
            (StatusCode::NOT_FOUND, json!({"error": "workspace_not_found"}))
        );
        let put = call(
            &app,
            "PUT",
            &format!("/api/workspaces/{ghost}/metric-settings"),
            Some(json!({"hourly_rate": 1})),
            None,
        )
        .await;
        assert_eq!(put, (StatusCode::NOT_FOUND, json!({"error": "workspace_not_found"})));
    }
}
