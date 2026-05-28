//! Scan lifecycle integration — exercises the daemon end-to-end via HTTP
//! and asserts the SSE event stream emits throttled folder_update events
//! with progress + a terminal status=indexed + a final project_update::active.
//!
//! Self-skips if SENSEI_API_URL is not set.

use reqwest::Client;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio_stream::StreamExt;

fn base_url() -> Option<String> { std::env::var("SENSEI_API_URL").ok() }

#[tokio::test]
async fn scan_emits_folder_progress_and_project_active() {
    let Some(url) = base_url() else { return; };
    let c = Client::new();

    // 1. Seed a tiny git folder with ~30 indexable files.
    //    scan_root scans a root dir and discovers git repos inside it,
    //    so the layout is: root/ → repo/ (git) → file_NNN.rs files.
    let root = PathBuf::from(format!("/tmp/sensei-scan-lifecycle-{}", std::process::id()));
    let repo = root.join("repo");
    fs::create_dir_all(repo.join(".git")).unwrap();  // make it look like a git repo
    for i in 0..30 {
        fs::write(repo.join(format!("file_{i:03}.rs")), "fn main() {}").unwrap();
    }

    // 2. Connect to the SSE stream BEFORE triggering the scan so we don't miss events.
    let sse_resp = c.get(format!("{url}/api/scan/events")).send().await.unwrap();
    let mut sse_stream = sse_resp.bytes_stream();

    // 3. Trigger the scan. Endpoint is POST /api/scan with body { "root": "..." }.
    //    We pass the parent directory — scan_root discovers git repos inside it.
    let _ = c.post(format!("{url}/api/scan"))
        .json(&serde_json::json!({ "root": root.display().to_string() }))
        .send().await.unwrap();

    // 4. Collect events for up to 60 seconds (indexer can be slow on first run).
    let mut folder_adds = 0;
    let mut folder_updates_with_progress = 0;
    let mut folder_terminal_indexed = 0;
    let mut project_active = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);

    while tokio::time::Instant::now() < deadline {
        let res = tokio::time::timeout(Duration::from_secs(2), sse_stream.next()).await;
        let Ok(Some(Ok(chunk))) = res else { continue; };
        let text = String::from_utf8_lossy(&chunk);
        for line in text.lines() {
            let Some(json) = line.strip_prefix("data: ") else { continue; };
            let Ok(evt): Result<serde_json::Value, _> = serde_json::from_str(json) else { continue; };
            let entity = evt["entity"].as_str().unwrap_or("");
            let action = evt["action"].as_str().unwrap_or("");
            eprintln!("[scan_lifecycle] event: entity={entity} action={action} data={}",
                serde_json::to_string(&evt["data"]).unwrap_or_default());
            match (entity, action) {
                ("folder", "add")    => folder_adds += 1,
                ("folder", "update") => {
                    let status = evt["data"]["status"].as_str().unwrap_or("");
                    let done = evt["data"]["filesCompleted"].as_u64().unwrap_or(0);
                    if status == "indexed" {
                        folder_terminal_indexed += 1;
                    } else if done > 0 {
                        folder_updates_with_progress += 1;
                    }
                }
                ("project", "update")
                    if evt["data"]["status"].as_str() == Some("active") => {
                    project_active += 1;
                }
                ("project", "update") => {}
                _ => {}
            }
        }
        if project_active >= 1 && folder_terminal_indexed >= 1 { break; }
    }

    fs::remove_dir_all(&root).ok();

    eprintln!("[scan_lifecycle] counts: folder_adds={folder_adds} progress_updates={folder_updates_with_progress} terminal_indexed={folder_terminal_indexed} project_active={project_active}");

    assert!(folder_adds >= 1,                  "expected ≥1 folder add, got {folder_adds}");
    assert!(folder_updates_with_progress >= 1, "expected ≥1 progress folder_update, got {folder_updates_with_progress}");
    assert!(folder_terminal_indexed >= 1,      "expected ≥1 terminal indexed folder_update, got {folder_terminal_indexed}");
    assert!(project_active >= 1,               "expected ≥1 project_update::active, got {project_active}");
}
