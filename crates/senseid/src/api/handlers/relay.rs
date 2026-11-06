//! Relay lanes slice 1, item 2 — the daemon HTTP surface (phase A part 2).
//!
//! Lane routes under `/api/relay/…` (MCP tools call these; the paths extend
//! the part-1 pins) and person routes exactly as `relay-api.ts` calls them
//! (`/relay/now`, `/relay/queue`, `/relay/tasks/{crew}/{task}`,
//! `/relay/messages`, `/relay/answers`, `/relay/backlog`).
//!
//! URL paths are L3's to publish (Q9). The full table:
//!
//! | Caller | Method + path | Op |
//! |---|---|---|
//! | lane | `POST /api/relay/register` | register (mints the token; no auth) |
//! | lane | `POST /api/relay/heartbeat` | renew the lease |
//! | lane | `POST /api/relay/reregister` | resume with the checkpoint |
//! | lane | `POST /api/relay/unregister` | leave |
//! | lane | `GET /api/relay/tasks?lane_id=` | the envelope, now |
//! | lane | `GET /api/relay/tasks/wait?lane_id=&timeout_ms=` | the envelope, held open ≤ 50 s |
//! | lane | `POST /api/relay/tasks/plan` | plan an item (coordinator) |
//! | lane | `POST /api/relay/tasks/{id}/assign` | hand to a lane (coordinator) |
//! | lane | `POST /api/relay/tasks/{id}/report` | a work/rework turn |
//! | lane | `POST /api/relay/tasks/{id}/review` | a review turn |
//! | lane | `POST /api/relay/decisions` | ask |
//! | lane | `POST /api/relay/decisions/{id}/answer` | answer (lane asked) |
//! | lane | `POST /api/relay/messages` | message (lane sent) |
//! | lane | `POST /api/relay/tasks/{id}/merge` | merge (coordinator) |
//! | person | `GET /relay/now`, `GET /relay/queue`, `GET /relay/tasks/{crew}/{task}` | reads |
//! | person | `POST /relay/messages`, `POST /relay/answers`, `POST /relay/backlog` | writes |
//! | person | `POST /relay/triage` | add a manual triage item (derived: nothing
//! | | else creates one in slice 1 — the tracker pull is slice 2) |
//!
//! Lane calls authenticate over the `Authorization: Bearer <lane_token>`
//! header and name their `lane_id` in the query (GET) or body (POST); the
//! token never appears anywhere else. Person calls ride the daemon's
//! localhost trust; the daemon user comes from `relay_current_user` (Q19).
//!
//! Phase A: types and reds only — the handlers and the `routes.rs`
//! registration land in phase B, so every red below fails on the missing
//! route (404 with an empty body) or the missing store op, never on setup.
//! Failure tests assert the `relay_*` error code in the body, so none can
//! pass vacuously on the router's bare 404.

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{Value, json};
    use tower::ServiceExt;

    struct Harness {
        app: axum::Router,
        state: crate::api::state::AppState,
    }

    async fn harness() -> Harness {
        let ctx = crate::tasks::test_support::make_ctx().await;
        let state = ctx.app_state.clone();
        Harness { app: crate::api::routes::create_router(state.clone()), state }
    }

    /// One request through the real router, from this machine by default.
    async fn call(
        h: &Harness,
        method: &str,
        uri: &str,
        body: Option<Value>,
        token: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut b = Request::builder().method(method).uri(uri).header("host", "127.0.0.1:7744");
        if let Some(t) = token {
            b = b.header("authorization", format!("Bearer {t}"));
        }
        let req = match body {
            Some(v) => b
                .header("content-type", "application/json")
                .body(Body::from(v.to_string()))
                .unwrap(),
            None => b.body(Body::empty()).unwrap(),
        };
        let resp = h.app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json_body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, json_body)
    }

    fn lane_body(lane_id: &str, extra: Value) -> Value {
        let mut obj = extra.as_object().cloned().unwrap_or_default();
        obj.insert("lane_id".into(), json!(lane_id));
        Value::Object(obj)
    }

    /// Register a coordinator (minting its crew) and return
    /// (lane_id, token, crew_id). Every lane-flow test starts here.
    async fn register_coordinator(h: &Harness) -> (String, String, String) {
        let (status, body) = call(
            h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "coordinator", "family": "claude-code"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        (
            body["lane_id"].as_str().unwrap().to_string(),
            body["lane_token"].as_str().unwrap().to_string(),
            body["crew_id"].as_str().unwrap().to_string(),
        )
    }

    async fn register_contributor(h: &Harness, crew_id: &str) -> (String, String) {
        let (status, body) = call(
            h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "contributor", "family": "claude-code", "crew_id": crew_id})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        (
            body["lane_id"].as_str().unwrap().to_string(),
            body["lane_token"].as_str().unwrap().to_string(),
        )
    }

    /// Coordinator plans + assigns a tests-phase task to the lane; returns
    /// the task id.
    async fn assigned_task(h: &Harness, coord: &(String, String, String), lane_id: &str) -> String {
        let (status, body) = call(
            h,
            "POST",
            "/api/relay/tasks/plan",
            Some(lane_body(
                &coord.0,
                json!({"title": "T", "phase": "tests", "repo_key": "github.com/o/r"}),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let task_id = body["task_id"].as_str().unwrap().to_string();
        let (status, body) = call(
            h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/assign"),
            Some(lane_body(
                &coord.0,
                json!({
                    "to_lane": lane_id, "title": "T", "phase": "tests",
                    "brief": {"summary": "Do it.", "docs": []},
                    "acceptance": ["green"], "owned_paths": ["app/**"],
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        task_id
    }

    // ── register / heartbeat ──

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn register_mint_shape_q16rec() {
        let h = harness().await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "coordinator", "family": "claude-code", "goal": "Ship"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        // Q16-REC: the mint response carries the crew for the paste line.
        assert!(body["lane_token"].is_string());
        assert!(body["lane_id"].is_string());
        assert_eq!(body["lane_title"], Value::Null);
        assert!(body["crew_id"].is_string());
        assert!(uuid::Uuid::parse_str(body["crew_id"].as_str().unwrap()).is_ok());
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn register_contributor_without_crew_is_422() {
        let h = harness().await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "contributor", "family": "claude-code"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["error"], "relay_invalid");
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn heartbeat_renews_and_auth_is_checked() {
        let h = harness().await;
        let (lane_id, token, _) = register_coordinator(&h).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["expires_at"].is_string());
        // No header, wrong token: 401 with the code (never the bare 404).
        let (status, body) =
            call(&h, "POST", "/api/relay/heartbeat", Some(json!({"lane_id": lane_id})), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
        assert_eq!(body["error"], "relay_unauthorized");
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(json!({"lane_id": lane_id})),
            Some("tok-bogus"),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
        assert_eq!(body["error"], "relay_unauthorized");
    }

    // ── get_tasks / wait_for_task ──

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn get_tasks_envelope_keys_and_assignment() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let (status, body) =
            call(&h, "GET", &format!("/api/relay/tasks?lane_id={lane_id}"), None, Some(&token))
                .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        // Keys in ruling order; the assignment carries the frozen brief.
        let obj = body.as_object().unwrap();
        let keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        assert_eq!(keys, vec!["messages", "answers", "reviews", "assignments"]);
        assert_eq!(body["assignments"][0]["task_id"], task_id);
        assert_eq!(body["assignments"][0]["brief"]["summary"], "Do it.");
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn wait_for_task_timeout_returns_empty_envelope() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let (status, body) = call(
            &h,
            "GET",
            &format!("/api/relay/tasks/wait?lane_id={lane_id}&timeout_ms=10"),
            None,
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["messages"], json!([]));
        assert_eq!(body["answers"], json!([]));
        assert_eq!(body["reviews"], json!([]));
        assert_eq!(body["assignments"], json!([]));
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn wait_for_task_wakes_on_message() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        // A person message 100 ms in must wake the 5 s hold promptly.
        let app = h.app.clone();
        let crew = coord.2.clone();
        let lane = lane_id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let req = Request::builder()
                .method("POST")
                .uri("/relay/messages")
                .header("host", "127.0.0.1:7744")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "crew_id": crew,
                        "to": {"kind": "lane", "lane_id": lane},
                        "task_id": Value::Null, "text": "ping", "surface": "desktop",
                    })
                    .to_string(),
                ))
                .unwrap();
            let _ = app.oneshot(req).await.unwrap();
        });
        let start = std::time::Instant::now();
        let (status, body) = call(
            &h,
            "GET",
            &format!("/api/relay/tasks/wait?lane_id={lane_id}&timeout_ms=5000"),
            None,
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["messages"][0]["text"], "ping");
        assert!(start.elapsed() < std::time::Duration::from_millis(4000), "woke, not timed out");
    }

    // ── report / review / merge ──

    fn report_body(lane_id: &str) -> Value {
        lane_body(
            lane_id,
            json!({
                "phase": "tests", "round": 1,
                "summary": {"changed": "Wired it.", "look_at": "app/", "flags": null},
                "files": [{"path": "app/a.ts", "added": 1, "removed": 0}],
            }),
        )
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn report_then_review_then_merge_flow() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        // Delivery marks started; the report moves to in_review…
        let _ = call(&h, "GET", &format!("/api/relay/tasks?lane_id={lane_id}"), None, Some(&token))
            .await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/report"),
            Some(report_body(&lane_id)),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["state"], "in_review");
        // …the coordinator (picked reviewer on a two-lane crew) approves…
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/review"),
            Some(lane_body(&coord.0, json!({"verdict": "approved", "summary": "Clean."}))),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["state"], "approved");
        // …and merges.
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/merge"),
            Some(lane_body(&coord.0, json!({"commits": ["abc"], "into": "develop", "note": "M."}))),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["task"]["state"], "done");
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn assign_merge_phase_is_422_q3() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/tasks/plan",
            Some(lane_body(
                &coord.0,
                json!({"title": "T", "phase": "tests", "repo_key": "github.com/o/r"}),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let task_id = body["task_id"].as_str().unwrap().to_string();
        // No merge phase (Q3): merging is the merged action.
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/assign"),
            Some(lane_body(
                &coord.0,
                json!({
                    "to_lane": lane_id, "title": "T", "phase": "merge",
                    "brief": {"summary": "x", "docs": []},
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["error"], "relay_invalid");
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn unknown_task_is_404_with_code() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let missing = "01938931-0000-7000-8000-000000000099";
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{missing}/report"),
            Some(report_body(&coord.0)),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        assert_eq!(body["error"], "relay_not_found");
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn plan_requires_coordinator_403() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/tasks/plan",
            Some(lane_body(&lane_id, json!({"title": "T", "phase": "tests"}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "relay_forbidden");
    }

    // ── decisions / messages ──

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn ask_then_person_answers_flow() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/decisions",
            Some(lane_body(
                &coord.0,
                json!({
                    "task_id": task_id, "question": "Where?",
                    "options": [{"id": "db", "label": "Db"}],
                    "blocks": [task_id], "asked_of": "person",
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let decision_id = body["decision_id"].as_str().unwrap().to_string();
        let (status, body) = call(
            &h,
            "POST",
            "/relay/answers",
            Some(json!({"decision_id": decision_id, "option": "db", "note": null, "surface": "desktop"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["action_id"].is_string());
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn lane_answers_lane_asked_decision() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/decisions",
            Some(lane_body(
                &coord.0,
                json!({
                    "task_id": task_id, "question": "Which?",
                    "options": [{"id": "a", "label": "A"}],
                    "blocks": [], "asked_of": lane_id,
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let decision_id = body["decision_id"].as_str().unwrap().to_string();
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/decisions/{decision_id}/answer"),
            Some(lane_body(&lane_id, json!({"option": "a"}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn person_and_lane_messages_shape() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        // Person → lane (relay-api.ts shape)…
        let (status, body) = call(
            &h,
            "POST",
            "/relay/messages",
            Some(json!({
                "crew_id": coord.2,
                "to": {"kind": "lane", "lane_id": lane_id},
                "task_id": Value::Null, "text": "hi", "surface": "desktop",
            })),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["message_id"].is_string());
        // …lane → coordinator (lane shape)…
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/messages",
            Some(lane_body(&lane_id, json!({"to": {"kind": "coordinator"}, "text": "ack"}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["message_id"].is_string());
    }

    // ── person reads ──

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn now_queue_read_and_filter() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let _ = assigned_task(&h, &coord, &lane_id).await;
        let (status, body) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["crews"].as_array().unwrap().len(), 1);
        assert!(body["lanes"].is_array() && body["decisions"].is_array());
        let (status, body) =
            call(&h, "GET", "/relay/now?repo_key=github.com/nope", None, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["crews"], json!([]));
        let (status, body) = call(&h, "GET", "/relay/queue", None, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["triage"].is_array() && body["backlog"].is_array());
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn task_view_cross_crew_is_404() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        // A second crew: its id with the first crew's task is not found.
        let (_, _, crew2) = register_coordinator(&h).await;
        let (status, body) =
            call(&h, "GET", &format!("/relay/tasks/{crew2}/{task_id}"), None, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        assert_eq!(body["error"], "relay_not_found");
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn unregister_then_reregister_flow() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/unregister",
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        // Left: the lane reads gone…
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::GONE, "{body}");
        assert_eq!(body["error"], "relay_lane_gone");
        // …until it reregisters with the same token.
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/reregister",
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["lane_id"], lane_id);
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn backlog_add_item_and_ticket_is_501() {
        let h = harness().await;
        let _ = register_coordinator(&h).await;
        // A manual triage item, then the person's +.
        let (status, body) = call(
            &h,
            "POST",
            "/relay/triage",
            Some(json!({"title": "Gap", "kind": "gap", "tags": ["ux"]})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let item_id = body["item_id"].as_str().unwrap().to_string();
        let (status, body) =
            call(&h, "POST", "/relay/backlog", Some(json!({"item_id": item_id})), None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["item_id"], item_id);
        // Slice 2 owns the tracker pull: a ticketed add is 501, never a
        // fabricated title (derived).
        let (status, body) = call(
            &h,
            "POST",
            "/relay/backlog",
            Some(json!({"ticket_provider": "github", "ticket_key": "SEN-52"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{body}");
        assert_eq!(body["error"], "relay_not_implemented");
    }

    // ── failure paths ──

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn read_failure_is_500_never_empty() {
        let h = harness().await;
        h.state.pg.pool().close().await;
        let (status, body) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
        assert_eq!(body["error"], "relay_internal");
        // A failed read never answers 200-with-empty either.
        assert_ne!(status, StatusCode::OK);
    }
}
