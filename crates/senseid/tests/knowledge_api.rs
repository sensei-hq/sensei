//! End-to-end: HTTP request → daemon → DB → response.
//! Requires SENSEI_API_URL pointing at a running daemon (e.g. http://127.0.0.1:7745)
//! and SENSEI_TEST_PROJECT_ID for a real project_id in the live dev DB.
//! Self-skips when either env var is absent.

use reqwest::Client;
use serde_json::json;

fn base_url() -> Option<String> { std::env::var("SENSEI_API_URL").ok() }

#[tokio::test]
async fn full_lifecycle_proposal_accept_outcome() {
    let Some(url) = base_url() else { return; };
    let Some(pid) = std::env::var("SENSEI_TEST_PROJECT_ID").ok() else { return; };
    let c = Client::new();

    // 1. Propose
    let propose = c.post(format!("{url}/api/knowledge/proposals"))
        .json(&json!({
            "project_id":    pid,
            "scope":         "project",
            "type":          "convention",
            "title":         "lifecycle-test",
            "content":       "do the thing",
            "tags":          ["test"],
            "triage_signal": "revert",
        }))
        .send().await.unwrap();
    assert!(propose.status().is_success(), "propose status: {}", propose.status());
    let mid = propose.json::<serde_json::Value>().await.unwrap()["id"].as_str().unwrap().to_string();

    // 2. It appears in status=proposed
    let listing = c.get(format!("{url}/api/knowledge/memories?status=proposed&project_id={pid}"))
        .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    let titles: Vec<&str> = listing["memories"].as_array().unwrap().iter()
        .map(|m| m["title"].as_str().unwrap()).collect();
    assert!(titles.contains(&"lifecycle-test"));

    // 3. Accept
    let accept = c.post(format!("{url}/api/knowledge/proposals/{mid}/accept"))
        .json(&json!({})).send().await.unwrap();
    assert!(accept.status().is_success(), "accept: {}", accept.status());

    // 4. Context now contains it
    let ctx = c.get(format!("{url}/api/knowledge/context?project_id={pid}"))
        .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    let ctx_titles: Vec<&str> = ctx["memories"].as_array().unwrap().iter()
        .map(|m| m["title"].as_str().unwrap()).collect();
    assert!(ctx_titles.contains(&"lifecycle-test"));

    // 5. Record an outcome
    let outcome = c.post(format!("{url}/api/knowledge/outcomes"))
        .json(&json!({"outcomes":[{"memory_id":mid, "outcome":"applied"}]}))
        .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    assert_eq!(outcome["recorded"].as_i64().unwrap(), 1);
    assert_eq!(outcome["skipped"].as_array().unwrap().len(), 0);

    // 6. Detail shows the outcome
    let detail = c.get(format!("{url}/api/knowledge/memories/{mid}"))
        .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    assert_eq!(detail["outcomes"].as_array().unwrap().len(), 1);
}
