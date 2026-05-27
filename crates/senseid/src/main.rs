mod types;
mod db;
mod languages;
mod indexer;
mod config;
mod watcher;
mod api;
mod tasks;
pub mod assistants;
pub mod instruments;
pub mod installer;
pub mod paths;
pub mod ir;
pub mod gateway_keys;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "senseid", about = "Sensei indexer daemon")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Port to listen on (default: 7744 prod, 7745 dev)
    #[arg(long)]
    port: Option<u16>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start daemon in background
    Start {
        #[arg(long)]
        port: Option<u16>,
    },
    /// Stop the running daemon
    Stop,
    /// Show daemon status
    Status,
    /// Restart the daemon (stop then start)
    Restart {
        #[arg(long)]
        port: Option<u16>,
    },
    /// Tail the daemon log
    Logs,
    /// Clear the log file
    ClearLogs,
}

fn sensei_dir() -> PathBuf { paths::sensei_dir() }

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Mode is compile-time via Cargo features (--features dev).
    let startup_cfg = sensei_bootstrap::SenseiConfig::from_env();
    let default_port = startup_cfg.daemon_port;

    match cli.command {
        Some(Commands::Start { port }) => {
            let p = port.unwrap_or(default_port);
            start_daemon(p);
        }
        Some(Commands::Stop) => {
            stop_daemon(cli.port.unwrap_or(default_port)).await;
        }
        Some(Commands::Status) => {
            check_status(cli.port.unwrap_or(default_port)).await;
        }
        Some(Commands::Restart { port }) => {
            let p = port.unwrap_or(default_port);
            stop_daemon(p).await;
            // Brief pause so the port is released before re-binding
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            start_daemon(p);
        }
        Some(Commands::Logs) => {
            tail_logs();
        }
        Some(Commands::ClearLogs) => {
            clear_logs();
        }
        None => {
            run_foreground(cli.port.unwrap_or(default_port)).await;
        }
    }
}

fn start_daemon(port: u16) {
    // Check if already running
    let pid_path = sensei_dir().join("serve.pid");
    if pid_path.exists()
        && let Ok(pid_str) = std::fs::read_to_string(&pid_path)
            && let Ok(pid) = pid_str.trim().parse::<u32>() {
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

    std::fs::create_dir_all(sensei_dir()).expect("senseid: cannot create ~/.sensei/");
    let log_path = sensei_dir().join("senseid.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("senseid: cannot open log file");
    let log_err = log_file.try_clone().expect("senseid: cannot clone log handle");

    // Spawn self — mode is baked in at compile time via --features dev.
    let exe = std::env::current_exe().expect("senseid: cannot resolve own path");
    let mut child = Command::new(exe)
        .args(["--port", &port.to_string()])
        .stdout(log_file)
        .stderr(log_err)
        .stdin(std::process::Stdio::null())
        .spawn()
        .expect("senseid: failed to spawn daemon");

    let pid = child.id();
    // Detach: we don't wait — the daemon runs independently.
    // Reap to avoid zombie; the daemon re-parents to init/launchd.
    std::thread::spawn(move || { let _ = child.wait(); });
    println!("senseid: started (pid {})", pid);
}

async fn run_foreground(port: u16) {
    let db_dir = sensei_dir();
    std::fs::create_dir_all(&db_dir).ok();

    let pid_path = db_dir.join("serve.pid");
    std::fs::write(&pid_path, std::process::id().to_string()).ok();

    println!("[senseid] Listening on :{}", port);

    if let Err(e) = api::start_server(port).await {
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
    if let Ok(pid_str) = std::fs::read_to_string(&pid_path)
        && let Ok(_pid) = pid_str.trim().parse::<u32>() {
            // Send SIGTERM via nix or command
            let _ = std::process::Command::new("kill").arg(pid_str.trim()).status();
            println!("senseid: sent SIGTERM to pid {}", pid_str.trim());
            std::fs::remove_file(&pid_path).ok();
            return;
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

