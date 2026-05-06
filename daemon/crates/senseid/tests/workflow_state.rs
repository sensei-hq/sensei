//! Integration tests for workflow state endpoints.
//! Requires senseid running on :7744 (cargo test won't start it automatically).
//! These tests are ignored by default — run with: cargo test -- --ignored workflow

use serde_json::json;

fn daemon_url() -> String {
    std::env::var("SENSEID_URL").unwrap_or_else(|_| "http://127.0.0.1:7744".to_string())
}

fn test_project(suffix: &str) -> String {
    format!("test-wf-{}-{}", std::process::id(), suffix)
}

#[tokio::test]
#[ignore] // requires running daemon
async fn get_state_empty_project() {
    let client = reqwest::Client::new();
    let proj = test_project("empty");

    let resp = client.get(format!("{}/api/state/{}", daemon_url(), proj))
        .send().await.unwrap();
    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["project"], proj);
    assert!(body["active_phase"].is_null());
    assert!(body["active_task"].is_null());
    assert!(body["active_issue"].is_null());
}

#[tokio::test]
#[ignore]
async fn put_and_get_state() {
    let client = reqwest::Client::new();
    let proj = test_project("putget");

    // PUT
    let resp = client.put(format!("{}/api/state/{}", daemon_url(), proj))
        .json(&json!({
            "active_phase": "build",
            "active_task": "implement feature X",
            "active_issue": 42
        }))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], true);

    // GET
    let resp = client.get(format!("{}/api/state/{}", daemon_url(), proj))
        .send().await.unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["active_phase"], "build");
    assert_eq!(body["active_task"], "implement feature X");
    assert_eq!(body["active_issue"], 42);
}

#[tokio::test]
#[ignore]
async fn put_partial_preserves_existing() {
    let client = reqwest::Client::new();
    let proj = test_project("partial");

    // Set all fields
    client.put(format!("{}/api/state/{}", daemon_url(), proj))
        .json(&json!({
            "active_phase": "build",
            "active_task": "task 1",
            "active_issue": 99,
            "active_plan": "docs/plans/test.md"
        }))
        .send().await.unwrap();

    // Update only phase
    client.put(format!("{}/api/state/{}", daemon_url(), proj))
        .json(&json!({"active_phase": "validate"}))
        .send().await.unwrap();

    // Verify other fields preserved
    let resp = client.get(format!("{}/api/state/{}", daemon_url(), proj))
        .send().await.unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["active_phase"], "validate");
    assert_eq!(body["active_task"], "task 1");
    assert_eq!(body["active_issue"], 99);
    assert_eq!(body["active_plan"], "docs/plans/test.md");
}

#[tokio::test]
#[ignore]
async fn put_with_project_path_syncs_state_yaml() {
    let client = reqwest::Client::new();
    let proj = test_project("sync");
    let tmp = tempfile::TempDir::new().unwrap();
    let sensei_dir = tmp.path().join(".sensei");

    client.put(format!("{}/api/state/{}", daemon_url(), proj))
        .json(&json!({
            "active_phase": "ideate",
            "active_task": "brainstorm feature",
            "project_path": tmp.path().to_string_lossy()
        }))
        .send().await.unwrap();

    // Verify state.yaml was written
    let state_file = sensei_dir.join("state.yaml");
    assert!(state_file.exists(), "state.yaml should be created");

    let content = std::fs::read_to_string(&state_file).unwrap();
    assert!(content.contains("active_phase: ideate"), "phase should be in state.yaml");
    assert!(content.contains("brainstorm feature"), "task should be in state.yaml");
}
