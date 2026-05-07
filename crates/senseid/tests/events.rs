//! Integration tests for event endpoints.
//! Requires senseid running on :7744.

use serde_json::json;

fn daemon_url() -> String {
    std::env::var("SENSEID_URL").unwrap_or_else(|_| "http://127.0.0.1:7744".to_string())
}

fn test_project(suffix: &str) -> String {
    format!("test-evt-{}-{}", std::process::id(), suffix)
}

#[tokio::test]
#[ignore]
async fn post_event_returns_id() {
    let client = reqwest::Client::new();
    let proj = test_project("post");

    let resp = client.post(format!("{}/api/events", daemon_url()))
        .json(&json!({
            "project": proj,
            "event_type": "phase_transition",
            "data": {"from": "ideate", "to": "analyze"}
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 201);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], true);
    assert!(body["id"].as_str().is_some());
}

#[tokio::test]
#[ignore]
async fn post_event_requires_type() {
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/api/events", daemon_url()))
        .json(&json!({"project": "test", "data": {}}))
        .send().await.unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
#[ignore]
async fn list_events_filtered() {
    let client = reqwest::Client::new();
    let proj = test_project("filter");

    // Insert 3 events of different types
    for (et, data) in [
        ("turn", json!({"classification": "new_request"})),
        ("tool_used", json!({"tool": "search", "is_mcp": true})),
        ("turn", json!({"classification": "correction"})),
    ] {
        client.post(format!("{}/api/events", daemon_url()))
            .json(&json!({"project": proj, "event_type": et, "data": data}))
            .send().await.unwrap();
    }

    // Get all
    let resp = client.get(format!("{}/api/events/{}", daemon_url(), proj))
        .send().await.unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["count"], 3);

    // Filter by type
    let resp = client.get(format!("{}/api/events/{}?type=turn", daemon_url(), proj))
        .send().await.unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["count"], 2);

    // Filter by type (tool_used)
    let resp = client.get(format!("{}/api/events/{}?type=tool_used", daemon_url(), proj))
        .send().await.unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["count"], 1);
}

#[tokio::test]
#[ignore]
async fn list_events_with_limit() {
    let client = reqwest::Client::new();
    let proj = test_project("limit");

    for i in 0..10 {
        client.post(format!("{}/api/events", daemon_url()))
            .json(&json!({"project": proj, "event_type": "turn", "data": {"i": i}}))
            .send().await.unwrap();
    }

    let resp = client.get(format!("{}/api/events/{}?limit=3", daemon_url(), proj))
        .send().await.unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["count"], 3);
}

#[tokio::test]
#[ignore]
async fn event_data_preserved_as_json() {
    let client = reqwest::Client::new();
    let proj = test_project("json");

    client.post(format!("{}/api/events", daemon_url()))
        .json(&json!({
            "project": proj,
            "event_type": "locate",
            "data": {"tools": ["search", "get_callers"], "files": ["src/main.rs"]}
        }))
        .send().await.unwrap();

    let resp = client.get(format!("{}/api/events/{}", daemon_url(), proj))
        .send().await.unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let event = &body["events"][0];
    assert_eq!(event["data"]["tools"][0], "search");
    assert_eq!(event["data"]["files"][0], "src/main.rs");
}
