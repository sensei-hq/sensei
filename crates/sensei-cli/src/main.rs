use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::fs;

const DAEMON_URL: &str = "http://127.0.0.1:7744";

#[derive(Parser)]
#[command(name = "sensei", about = "Sensei — AI coding companion", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Detect and configure AI coding platforms (Claude Code, Cursor, Windsurf, etc)
    Configure,

    /// Install sensei: binaries, hooks, skills, MCP — uses configured ACPs
    Install {
        /// Install for specific ACP only
        #[arg(long)]
        acp: Option<String>,

        /// Skills scope: global, all
        #[arg(long, default_value = "global")]
        scope: String,
    },

    /// Uninstall sensei from all configured ACPs
    Uninstall,

    /// Start the sensei daemon
    Start {
        #[arg(long, default_value = "7744")]
        port: u16,
    },

    /// Stop the sensei daemon
    Stop,

    /// Show daemon status
    Status,

    /// Scan a folder and index all repos
    Scan {
        /// Folder to scan
        path: String,
    },

    /// Add an external library's documentation
    AddLib {
        /// Library name
        name: String,
        /// URL to llms.txt (auto-discovered if omitted)
        #[arg(long)]
        url: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Configure => configure(),
        Commands::Install { acp, scope } => install(acp.as_deref(), &scope),
        Commands::Uninstall => uninstall(),
        Commands::Start { port } => daemon_cmd("start", Some(port)),
        Commands::Stop => daemon_cmd("stop", None),
        Commands::Status => daemon_cmd("status", None),
        Commands::Scan { path } => scan(&path),
        Commands::AddLib { name, url } => add_lib(&name, url.as_deref()),
    }
}

// ── Paths ────────────────────────────────────────────────────────────────────
// Mirrors senseid::paths — CLI can't depend on senseid (heavy deps).

fn plugin_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join(".claude/plugins/sensei")
}
fn daemon_bin() -> PathBuf {
    plugin_dir().join("bin/senseid")
}

// ── Daemon helpers ───────────────────────────────────────────────────────────

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap()
}

fn daemon_available() -> bool {
    client()
        .get(format!("{}/health", DAEMON_URL))
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn require_daemon() {
    if !daemon_available() {
        eprintln!("Daemon not running. Start it first: sensei start");
        std::process::exit(1);
    }
}

// ── Configure (thin wrapper → daemon) ────────────────────────────────────────

fn configure() {
    require_daemon();
    println!("Detecting AI coding platforms...\n");

    // Detect
    let acps: Vec<serde_json::Value> = client()
        .get(format!("{}/api/acp/detect", DAEMON_URL))
        .send()
        .ok()
        .and_then(|r| r.json().ok())
        .unwrap_or_default();

    let mut detected: Vec<String> = Vec::new();
    for acp in &acps {
        let id = acp["id"].as_str().unwrap_or("");
        let name = acp["name"].as_str().unwrap_or("");
        let installed = acp["installed"].as_bool().unwrap_or(false);
        let configured = acp["mcp_configured"].as_bool().unwrap_or(false);
        let mark = if installed { "✓" } else { "·" };
        let status = match (installed, configured) {
            (true, true) => "configured",
            (true, false) => "detected",
            _ => "not found",
        };
        println!("  {} {} ({})", mark, name, status);
        if installed { detected.push(id.to_string()); }
    }

    if detected.is_empty() {
        println!("\nNo AI coding platforms detected.");
        return;
    }

    // Configure
    println!("\nConfiguring sensei for: {}", detected.join(", "));
    match client()
        .post(format!("{}/api/acp/configure", DAEMON_URL))
        .json(&serde_json::json!({"acps": detected}))
        .send()
    {
        Ok(r) if r.status().is_success() => {
            let result: serde_json::Value = r.json().unwrap_or_default();
            for c in result["configured"].as_array().unwrap_or(&vec![]) {
                println!("  ✓ {}", c.as_str().unwrap_or(""));
            }
            for e in result["errors"].as_array().unwrap_or(&vec![]) {
                eprintln!("  ✗ {}", e.as_str().unwrap_or(""));
            }
        }
        _ => eprintln!("Failed to configure ACPs"),
    }
    println!("\nDone.");
}

// ── Install ──────────────────────────────────────────────────────────────────

fn install(specific_acp: Option<&str>, scope: &str) {
    println!("Installing sensei...\n");

    // Step 1: Copy binaries (must happen before daemon starts)
    println!("[1/2] Binaries...");
    install_binaries();

    // Step 2: Everything else via daemon
    println!("[2/2] Hooks, skills, commands, ACP config...");

    if !daemon_available() {
        // Try to start the daemon
        let bin = daemon_bin();
        if bin.exists() {
            println!("  Starting daemon...");
            std::process::Command::new(&bin)
                .arg("start")
                .arg("--port")
                .arg("7744")
                .spawn()
                .ok();
            // Wait for it to come up
            for _ in 0..20 {
                std::thread::sleep(std::time::Duration::from_millis(250));
                if daemon_available() { break; }
            }
        }
    }

    if !daemon_available() {
        eprintln!("  Daemon unavailable. Run: sensei start && sensei install");
        return;
    }

    // Detect ACPs if none specified
    let acps: Vec<String> = if let Some(acp) = specific_acp {
        vec![acp.to_string()]
    } else {
        let detected: Vec<serde_json::Value> = client()
            .get(format!("{}/api/acp/detect", DAEMON_URL))
            .send()
            .ok()
            .and_then(|r| r.json().ok())
            .unwrap_or_default();
        detected
            .iter()
            .filter(|a| a["installed"].as_bool() == Some(true))
            .filter_map(|a| a["id"].as_str().map(String::from))
            .collect()
    };

    // Full install via daemon
    match client()
        .post(format!("{}/api/install", DAEMON_URL))
        .json(&serde_json::json!({"acps": acps, "scope": scope}))
        .send()
    {
        Ok(r) if r.status().is_success() => {
            let result: serde_json::Value = r.json().unwrap_or_default();
            let hooks = result["hooks_installed"].as_u64().unwrap_or(0);
            let skills = result["skills_installed"].as_u64().unwrap_or(0);
            let cmds = result["commands_installed"].as_u64().unwrap_or(0);
            println!("  {} hooks, {} skills, {} commands installed", hooks, skills, cmds);
            for c in result["acps_configured"].as_array().unwrap_or(&vec![]) {
                println!("  ✓ {}", c.as_str().unwrap_or(""));
            }
            for e in result["errors"].as_array().unwrap_or(&vec![]) {
                eprintln!("  ✗ {}", e.as_str().unwrap_or(""));
            }
        }
        Ok(r) => eprintln!("  Install failed: HTTP {}", r.status()),
        Err(e) => eprintln!("  Install failed: {}", e),
    }

    println!("\nSensei installed.");
    println!("  Scan repos: sensei scan ~/Developer");
}

fn install_binaries() {
    let plugin = plugin_dir();
    fs::create_dir_all(plugin.join("bin")).ok();

    let self_path = std::env::current_exe().unwrap_or_default();
    let self_dir = self_path.parent().unwrap_or(Path::new("."));

    for bin_name in &["senseid", "sensei-mcp"] {
        let src = self_dir.join(bin_name);
        let dst = plugin.join("bin").join(bin_name);
        if src.exists() {
            fs::copy(&src, &dst).ok();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&dst, fs::Permissions::from_mode(0o755)).ok();
            }
            println!("  {}", bin_name);
        }
    }
}

// ── Uninstall ────────────────────────────────────────────────────────────────

fn uninstall() {
    if daemon_available() {
        match client()
            .post(format!("{}/api/uninstall", DAEMON_URL))
            .send()
        {
            Ok(r) if r.status().is_success() => {
                let result: serde_json::Value = r.json().unwrap_or_default();
                for id in result["acps_removed"].as_array().unwrap_or(&vec![]) {
                    println!("  Removed MCP from {}", id.as_str().unwrap_or(""));
                }
                if result["skills_removed"].as_u64().unwrap_or(0) > 0 {
                    println!("  Removed {} skills", result["skills_removed"]);
                }
            }
            _ => eprintln!("Daemon uninstall failed, cleaning up manually"),
        }
    }

    // Always clean up local plugin dir
    if plugin_dir().exists() {
        fs::remove_dir_all(plugin_dir()).ok();
        println!("Removed plugin directory");
    }

    println!("Sensei uninstalled.");
}

// ── Daemon / Scan / AddLib ───────────────────────────────────────────────────

fn daemon_cmd(cmd: &str, port: Option<u16>) {
    let bin = daemon_bin();
    if !bin.exists() {
        eprintln!("senseid not found. Run: sensei install");
        std::process::exit(1);
    }
    let mut args = vec![cmd.to_string()];
    if let Some(p) = port {
        args.push("--port".into());
        args.push(p.to_string());
    }
    match std::process::Command::new(&bin).args(&args).status() {
        Ok(s) => std::process::exit(s.code().unwrap_or(0)),
        Err(e) => {
            eprintln!("Failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn scan(path: &str) {
    require_daemon();
    match client()
        .post(format!("{}/api/scan", DAEMON_URL))
        .json(&serde_json::json!({"root": path, "max_depth": 4}))
        .send()
    {
        Ok(r) if r.status().is_success() => println!("Scanning {} (background)...", path),
        _ => eprintln!("Scan request failed"),
    }
}

fn add_lib(name: &str, url: Option<&str>) {
    require_daemon();
    let c = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .unwrap();
    let mut body = serde_json::json!({"tool": "add_library", "params": {"name": name}});
    if let Some(u) = url {
        body["params"]["url"] = serde_json::json!(u);
    }
    match c
        .post(format!("{}/api/mcp/call", DAEMON_URL))
        .json(&body)
        .send()
    {
        Ok(r) if r.status().is_success() => {
            let d: serde_json::Value = r.json().unwrap_or_default();
            if d["ok"].as_bool() == Some(true) {
                println!(
                    "Indexed {} docs for {} from {}",
                    d["docsIndexed"],
                    name,
                    d["url"].as_str().unwrap_or("?")
                );
            } else {
                println!("{}", d["error"].as_str().unwrap_or("Failed"));
            }
        }
        _ => eprintln!("Request failed"),
    }
}
