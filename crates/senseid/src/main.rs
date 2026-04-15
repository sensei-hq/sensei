mod types;
mod db;
mod adapters;
mod indexer;
mod config;
mod watcher;
mod api;
pub mod acp;
pub mod installer;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

fn sensei_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".sensei")
}

fn db_path() -> PathBuf {
    sensei_dir().join("sensei.db")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Start { port }) => {
            run_foreground(port).await;
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

fn graph_path() -> PathBuf {
    sensei_dir().join("graph")
}

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
}
