mod types;
mod db;
mod adapters;
mod indexer;
mod config;
mod watcher;
mod api;
mod tasks;
pub mod acp;
pub mod installer;
pub mod paths;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "senseid", about = "Sensei indexer daemon")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Port to listen on
    #[arg(long, default_value = "7744")]
    port: u16,

}

#[derive(Subcommand)]
enum Commands {
    /// Start daemon in background
    Start {
        #[arg(long, default_value = "7744")]
        port: u16,
    },
    /// Stop the running daemon
    Stop,
    /// Show daemon status
    Status,
    /// Tail the daemon log
    Logs,
    /// Clear the log file
    ClearLogs,
}

fn sensei_dir() -> PathBuf { paths::sensei_dir() }
fn db_path() -> PathBuf { paths::db_path() }

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Start { port }) => {
            start_daemon(port);
        }
        Some(Commands::Stop) => {
            stop_daemon(cli.port).await;
        }
        Some(Commands::Status) => {
            check_status(cli.port).await;
        }
        Some(Commands::Logs) => {
            tail_logs();
        }
        Some(Commands::ClearLogs) => {
            clear_logs();
        }
        None => {
            run_foreground(cli.port).await;
        }
    }
}

fn start_daemon(port: u16) {
    // Check if already running
    let pid_path = sensei_dir().join("serve.pid");
    if pid_path.exists() {
        if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // Check if process is alive
                let alive = Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                if alive {
                    eprintln!("senseid: already running (pid {})", pid);
                    std::process::exit(1);
                }
                // Stale PID file — clean up and continue
                std::fs::remove_file(&pid_path).ok();
            }
        }
    }

    let log_path = sensei_dir().join("senseid.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("senseid: cannot open log file");
    let log_err = log_file.try_clone().expect("senseid: cannot clone log handle");

    let exe = std::env::current_exe().expect("senseid: cannot resolve own path");
    let child = Command::new(exe)
        .args(["--port", &port.to_string()])
        .stdout(log_file)
        .stderr(log_err)
        .stdin(std::process::Stdio::null())
        .spawn()
        .expect("senseid: failed to spawn daemon");

    println!("senseid: started (pid {})", child.id());
}

fn graph_path() -> PathBuf { paths::graph_dir() }

async fn run_foreground(port: u16) {
    let db_dir = sensei_dir();
    std::fs::create_dir_all(&db_dir).ok();

    let pid_path = db_dir.join("serve.pid");
    std::fs::write(&pid_path, std::process::id().to_string()).ok();

    let store = db::Store::open(&db_path()).expect("Failed to open SQLite");

    // Print project stats
    match store.list_projects() {
        Ok(projects) => {
            let indexed = projects.iter().filter(|p| p.indexed_at.is_some()).count();
            println!("[senseid] {} projects registered ({} indexed)", projects.len(), indexed);
        }
        Err(_) => {}
    }

    let gp = graph_path();
    std::fs::create_dir_all(&gp).ok();
    let graph = indexer::graph::GraphDb::open(&gp).expect("Failed to open graph DB");
    println!("[senseid] Listening on :{}", port);

    if let Err(e) = api::start_server(store, graph, port).await {
        eprintln!("[senseid] Server error: {}", e);
    }
    std::fs::remove_file(&pid_path).ok();
}

async fn stop_daemon(port: u16) {
    let client = reqwest::Client::new();
    match client.post(format!("http://127.0.0.1:{}/stop", port))
        .timeout(std::time::Duration::from_secs(3))
        .send().await
    {
        Ok(resp) if resp.status().is_success() => {
            println!("senseid: stopped.");
            std::fs::remove_file(sensei_dir().join("serve.pid")).ok();
            return;
        }
        _ => {}
    }
    let pid_path = sensei_dir().join("serve.pid");
    if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
        if let Ok(_pid) = pid_str.trim().parse::<u32>() {
            // Send SIGTERM via nix or command
            let _ = std::process::Command::new("kill").arg(&pid_str.trim()).status();
            println!("senseid: sent SIGTERM to pid {}", pid_str.trim());
            std::fs::remove_file(&pid_path).ok();
            return;
        }
    }
    eprintln!("senseid: not running");
    std::process::exit(1);
}

async fn check_status(port: u16) {
    let client = reqwest::Client::new();
    match client.get(format!("http://127.0.0.1:{}/health", port))
        .timeout(std::time::Duration::from_secs(3))
        .send().await
    {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                println!("senseid: running on :{}", port);
                if let Some(v) = json.get("version").and_then(|v| v.as_str()) {
                    println!("  version: {}", v);
                }
            }
            if let Ok(pid) = std::fs::read_to_string(sensei_dir().join("serve.pid")) {
                println!("  pid: {}", pid.trim());
            }
        }
        _ => {
            let pid_path = sensei_dir().join("serve.pid");
            if pid_path.exists() {
                std::fs::remove_file(&pid_path).ok();
                println!("senseid: not running (cleaned stale PID)");
            } else {
                println!("senseid: not running");
            }
        }
    }
}

fn tail_logs() {
    let log_path = sensei_dir().join("senseid.log");
    if !log_path.exists() {
        eprintln!("senseid: no log file at {}", log_path.display());
        std::process::exit(1);
    }
    let status = std::process::Command::new("tail")
        .args(["-f", "-n", "50"])
        .arg(&log_path)
        .status()
        .expect("failed to run tail");
    std::process::exit(status.code().unwrap_or(0));
}

fn clear_logs() {
    let log_path = sensei_dir().join("senseid.log");
    match std::fs::write(&log_path, "") {
        Ok(_) => println!("senseid: logs cleared"),
        Err(_) => eprintln!("senseid: no log file to clear"),
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn store_opens_in_memory() {
        let store = db::Store::open_memory().unwrap();
        let projects = store.list_projects().unwrap();
        assert!(projects.is_empty());
    }

    #[test]
    fn sensei_dir_path() {
        let dir = sensei_dir();
        assert!(dir.to_string_lossy().contains(".sensei"));
    }

    // ── Workflow State Tests ─────────────────────────────────────────────

    #[test]
    fn workflow_state_upsert_and_get() {
        let store = db::Store::open_memory().unwrap();

        // Initially no state
        let state = store.get_workflow_state("test-project").unwrap();
        assert!(state.is_none());

        // Upsert phase
        store.upsert_workflow_state(
            "test-project",
            Some("ideate"), None, None, None, None, None,
        ).unwrap();

        let state = store.get_workflow_state("test-project").unwrap().unwrap();
        assert_eq!(state["active_phase"], "ideate");
        assert!(state["active_task"].is_null());

        // Update task without overwriting phase
        store.upsert_workflow_state(
            "test-project",
            None, None, Some("implement SqlAdapter"), Some(42), None, None,
        ).unwrap();

        let state = store.get_workflow_state("test-project").unwrap().unwrap();
        assert_eq!(state["active_phase"], "ideate"); // preserved
        assert_eq!(state["active_task"], "implement SqlAdapter");
        assert_eq!(state["active_issue"], 42);
    }

    #[test]
    fn workflow_state_full_update() {
        let store = db::Store::open_memory().unwrap();

        store.upsert_workflow_state(
            "test-project",
            Some("build"),
            Some("docs/plans/wave1.md"),
            Some("feature #1"),
            Some(55),
            Some("2026-04-17T12:00:00Z"),
            Some("abc123"),
        ).unwrap();

        let state = store.get_workflow_state("test-project").unwrap().unwrap();
        assert_eq!(state["active_phase"], "build");
        assert_eq!(state["active_plan"], "docs/plans/wave1.md");
        assert_eq!(state["active_task"], "feature #1");
        assert_eq!(state["active_issue"], 55);
        assert_eq!(state["last_checkpoint"], "2026-04-17T12:00:00Z");
        assert_eq!(state["rules_hash"], "abc123");
        assert!(state["updated_at"].as_str().is_some());
    }

    #[test]
    fn workflow_state_multiple_projects() {
        let store = db::Store::open_memory().unwrap();

        store.upsert_workflow_state("project-a", Some("ideate"), None, None, None, None, None).unwrap();
        store.upsert_workflow_state("project-b", Some("build"), None, None, Some(10), None, None).unwrap();

        let a = store.get_workflow_state("project-a").unwrap().unwrap();
        let b = store.get_workflow_state("project-b").unwrap().unwrap();
        assert_eq!(a["active_phase"], "ideate");
        assert_eq!(b["active_phase"], "build");
        assert_eq!(b["active_issue"], 10);
        assert!(a["active_issue"].is_null());
    }

    #[test]
    fn workflow_state_partial_update_preserves_fields() {
        let store = db::Store::open_memory().unwrap();

        // Set all fields
        store.upsert_workflow_state(
            "test", Some("build"), Some("plan.md"), Some("task 1"), Some(42),
            Some("2026-04-17T12:00:00Z"), Some("hash123"),
        ).unwrap();

        // Update only phase — others should be preserved via COALESCE
        store.upsert_workflow_state(
            "test", Some("validate"), None, None, None, None, None,
        ).unwrap();

        let state = store.get_workflow_state("test").unwrap().unwrap();
        assert_eq!(state["active_phase"], "validate"); // updated
        assert_eq!(state["active_plan"], "plan.md"); // preserved
        assert_eq!(state["active_task"], "task 1"); // preserved
        assert_eq!(state["active_issue"], 42); // preserved
        assert_eq!(state["last_checkpoint"], "2026-04-17T12:00:00Z"); // preserved
        assert_eq!(state["rules_hash"], "hash123"); // preserved
    }

    #[test]
    fn workflow_state_nonexistent_project_returns_none() {
        let store = db::Store::open_memory().unwrap();
        let state = store.get_workflow_state("does-not-exist").unwrap();
        assert!(state.is_none());
    }

    #[test]
    fn workflow_state_updated_at_changes() {
        let store = db::Store::open_memory().unwrap();

        store.upsert_workflow_state("test", Some("ideate"), None, None, None, None, None).unwrap();
        let s1 = store.get_workflow_state("test").unwrap().unwrap();
        let t1 = s1["updated_at"].as_str().unwrap().to_string();

        // Small delay to ensure timestamp differs
        std::thread::sleep(std::time::Duration::from_millis(10));

        store.upsert_workflow_state("test", Some("analyze"), None, None, None, None, None).unwrap();
        let s2 = store.get_workflow_state("test").unwrap().unwrap();
        let t2 = s2["updated_at"].as_str().unwrap().to_string();

        assert_ne!(t1, t2);
    }

    #[test]
    fn get_project_path_found() {
        let store = db::Store::open_memory().unwrap();
        store.upsert_project_basic("test-repo", "test-project", "/home/user/project").unwrap();

        // Match by name
        let path = store.get_project_path("test-project").unwrap();
        assert_eq!(path, Some("/home/user/project".to_string()));

        // Match by repo_id
        let path = store.get_project_path("test-repo").unwrap();
        assert_eq!(path, Some("/home/user/project".to_string()));
    }

    #[test]
    fn get_project_path_not_found() {
        let store = db::Store::open_memory().unwrap();
        let path = store.get_project_path("nonexistent").unwrap();
        assert!(path.is_none());
    }

    // ── Event Store Tests ────────────────────────────────────────────────

    #[test]
    fn event_insert_and_list() {
        let store = db::Store::open_memory().unwrap();

        store.insert_event("e1", "proj-a", Some("s1"), "phase_transition", r#"{"from":"ideate","to":"analyze"}"#).unwrap();
        store.insert_event("e2", "proj-a", Some("s1"), "tool_used", r#"{"tool":"search","is_mcp":true}"#).unwrap();
        store.insert_event("e3", "proj-a", Some("s1"), "turn", r#"{"classification":"new_request"}"#).unwrap();

        let events = store.list_events("proj-a", None, None, 50).unwrap();
        assert_eq!(events.len(), 3);
        // Ordered by created_at DESC — most recent first
        assert_eq!(events[0]["event_type"], "turn");
    }

    #[test]
    fn event_filter_by_type() {
        let store = db::Store::open_memory().unwrap();

        store.insert_event("e1", "proj-b", None, "phase_transition", "{}").unwrap();
        store.insert_event("e2", "proj-b", None, "tool_used", "{}").unwrap();
        store.insert_event("e3", "proj-b", None, "tool_used", "{}").unwrap();
        store.insert_event("e4", "proj-b", None, "turn", "{}").unwrap();

        let tool_events = store.list_events("proj-b", Some("tool_used"), None, 50).unwrap();
        assert_eq!(tool_events.len(), 2);
        assert!(tool_events.iter().all(|e| e["event_type"] == "tool_used"));
    }

    #[test]
    fn event_filter_by_session() {
        let store = db::Store::open_memory().unwrap();

        store.insert_event("e1", "proj-c", Some("s1"), "turn", "{}").unwrap();
        store.insert_event("e2", "proj-c", Some("s2"), "turn", "{}").unwrap();
        store.insert_event("e3", "proj-c", Some("s1"), "turn", "{}").unwrap();

        let s1_events = store.list_events("proj-c", None, Some("s1"), 50).unwrap();
        assert_eq!(s1_events.len(), 2);
        assert!(s1_events.iter().all(|e| e["session_id"] == "s1"));
    }

    #[test]
    fn event_filter_by_type_and_session() {
        let store = db::Store::open_memory().unwrap();

        store.insert_event("e1", "proj-d", Some("s1"), "turn", "{}").unwrap();
        store.insert_event("e2", "proj-d", Some("s1"), "tool_used", "{}").unwrap();
        store.insert_event("e3", "proj-d", Some("s2"), "turn", "{}").unwrap();

        let filtered = store.list_events("proj-d", Some("turn"), Some("s1"), 50).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0]["event_type"], "turn");
        assert_eq!(filtered[0]["session_id"], "s1");
    }

    #[test]
    fn event_limit_respected() {
        let store = db::Store::open_memory().unwrap();

        for i in 0..10 {
            store.insert_event(&format!("e{}", i), "proj-e", None, "turn", "{}").unwrap();
        }

        let limited = store.list_events("proj-e", None, None, 3).unwrap();
        assert_eq!(limited.len(), 3);
    }

    #[test]
    fn event_count() {
        let store = db::Store::open_memory().unwrap();

        store.insert_event("e1", "proj-f", None, "turn", "{}").unwrap();
        store.insert_event("e2", "proj-f", None, "turn", "{}").unwrap();
        store.insert_event("e3", "proj-f", None, "tool_used", "{}").unwrap();

        assert_eq!(store.count_events("proj-f", None).unwrap(), 3);
        assert_eq!(store.count_events("proj-f", Some("turn")).unwrap(), 2);
        assert_eq!(store.count_events("proj-f", Some("tool_used")).unwrap(), 1);
        assert_eq!(store.count_events("proj-f", Some("nonexistent")).unwrap(), 0);
    }

    #[test]
    fn event_data_parsed_as_json() {
        let store = db::Store::open_memory().unwrap();

        store.insert_event("e1", "proj-g", None, "locate", r#"{"tools":["search","get_callers"],"files":["src/main.rs"]}"#).unwrap();

        let events = store.list_events("proj-g", None, None, 1).unwrap();
        assert_eq!(events[0]["data"]["tools"][0], "search");
        assert_eq!(events[0]["data"]["files"][0], "src/main.rs");
    }

    // ── Pattern Detection Tests ────────────────────────────────────────

    #[test]
    fn detect_patterns_by_naming() {
        let store = db::Store::open_memory().unwrap();
        // Create a graph DB with adapter-named types
        let graph_conn = rusqlite::Connection::open_in_memory().unwrap();
        graph_conn.execute_batch("
            CREATE TABLE hierarchy_nodes(id TEXT PRIMARY KEY, name TEXT, kind TEXT, level TEXT, parent_id TEXT, file TEXT, line INTEGER, project TEXT, sig TEXT, body TEXT, docstring TEXT, complexity INTEGER, tags TEXT, doc_type TEXT, doc_category TEXT);
        ").unwrap();

        // Insert 3 adapters (should detect pattern) and 1 factory (below threshold)
        for (name, file) in [("TypeScriptAdapter", "ts.rs"), ("PythonAdapter", "py.rs"), ("RustAdapter", "rust.rs")] {
            graph_conn.execute(
                "INSERT INTO hierarchy_nodes(id, name, kind, project, file) VALUES(?1,?2,'class','test-proj',?3)",
                rusqlite::params![format!("type:{}",name), name, file],
            ).unwrap();
        }
        graph_conn.execute(
            "INSERT INTO hierarchy_nodes(id, name, kind, project, file) VALUES('type:TaskFactory','TaskFactory','class','test-proj','factory.rs')",
            [],
        ).unwrap();

        let patterns = store.detect_patterns_from_graph(&graph_conn, "test-proj").unwrap();
        assert_eq!(patterns.len(), 1); // Only adapter (3 instances), not factory (1 instance < threshold of 2)
        assert_eq!(patterns[0]["pattern_type"], "adapter");
        assert_eq!(patterns[0]["instance_count"], 3);
    }

    #[test]
    fn detect_patterns_requires_two_instances() {
        let store = db::Store::open_memory().unwrap();
        let graph_conn = rusqlite::Connection::open_in_memory().unwrap();
        graph_conn.execute_batch("
            CREATE TABLE hierarchy_nodes(id TEXT PRIMARY KEY, name TEXT, kind TEXT, level TEXT, parent_id TEXT, file TEXT, line INTEGER, project TEXT, sig TEXT, body TEXT, docstring TEXT, complexity INTEGER, tags TEXT, doc_type TEXT, doc_category TEXT);
        ").unwrap();

        // Only 1 adapter — should NOT detect pattern
        graph_conn.execute(
            "INSERT INTO hierarchy_nodes(id, name, kind, project) VALUES('type:SqlAdapter','SqlAdapter','class','test')",
            [],
        ).unwrap();

        let patterns = store.detect_patterns_from_graph(&graph_conn, "test").unwrap();
        assert!(patterns.is_empty());
    }

    #[test]
    fn match_pattern_by_keyword() {
        let store = db::Store::open_memory().unwrap();
        let graph_conn = rusqlite::Connection::open_in_memory().unwrap();
        graph_conn.execute_batch("
            CREATE TABLE hierarchy_nodes(id TEXT PRIMARY KEY, name TEXT, kind TEXT, level TEXT, parent_id TEXT, file TEXT, line INTEGER, project TEXT, sig TEXT, body TEXT, docstring TEXT, complexity INTEGER, tags TEXT, doc_type TEXT, doc_category TEXT);
        ").unwrap();

        for name in ["TypeScriptAdapter", "PythonAdapter", "RustAdapter"] {
            graph_conn.execute(
                "INSERT INTO hierarchy_nodes(id, name, kind, project) VALUES(?1,?2,'class','test')",
                rusqlite::params![format!("type:{}", name), name],
            ).unwrap();
        }
        store.detect_patterns_from_graph(&graph_conn, "test").unwrap();

        // Match by pattern type
        let matches = store.match_pattern("test", "add SQL adapter").unwrap();
        assert!(!matches.is_empty());
        assert_eq!(matches[0]["pattern_type"], "adapter");

        // No match returns all as context
        let matches = store.match_pattern("test", "add logging").unwrap();
        assert!(!matches.is_empty());
        assert_eq!(matches[0]["match_type"], "context");
    }

    #[test]
    fn list_detected_patterns() {
        let store = db::Store::open_memory().unwrap();
        let graph_conn = rusqlite::Connection::open_in_memory().unwrap();
        graph_conn.execute_batch("
            CREATE TABLE hierarchy_nodes(id TEXT PRIMARY KEY, name TEXT, kind TEXT, level TEXT, parent_id TEXT, file TEXT, line INTEGER, project TEXT, sig TEXT, body TEXT, docstring TEXT, complexity INTEGER, tags TEXT, doc_type TEXT, doc_category TEXT);
        ").unwrap();

        for name in ["FooHandler", "BarHandler", "FooWorker", "BarWorker"] {
            graph_conn.execute(
                "INSERT INTO hierarchy_nodes(id, name, kind, project) VALUES(?1,?2,'class','test')",
                rusqlite::params![format!("type:{}", name), name],
            ).unwrap();
        }
        store.detect_patterns_from_graph(&graph_conn, "test").unwrap();

        let patterns = store.list_detected_patterns("test").unwrap();
        assert_eq!(patterns.len(), 2); // handler + worker
    }

    #[test]
    fn event_isolates_projects() {
        let store = db::Store::open_memory().unwrap();

        store.insert_event("e1", "proj-x", None, "turn", "{}").unwrap();
        store.insert_event("e2", "proj-y", None, "turn", "{}").unwrap();

        assert_eq!(store.list_events("proj-x", None, None, 50).unwrap().len(), 1);
        assert_eq!(store.list_events("proj-y", None, None, 50).unwrap().len(), 1);
        assert_eq!(store.list_events("proj-z", None, None, 50).unwrap().len(), 0);
    }
}
