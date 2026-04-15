use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Parser)]
#[command(name = "sensei", about = "Sensei — AI coding companion", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install sensei plugin for an AI coding platform
    Install {
        /// Target ACP: claude-code, cursor, windsurf
        #[arg(long, default_value = "claude-code")]
        acp: String,

        /// Install skills scope: global, project, all
        #[arg(long, default_value = "global")]
        scope: String,
    },

    /// Uninstall sensei plugin
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
        Commands::Install { acp, scope } => install(&acp, &scope),
        Commands::Uninstall => uninstall(),
        Commands::Start { port } => daemon_cmd("start", Some(port)),
        Commands::Stop => daemon_cmd("stop", None),
        Commands::Status => daemon_cmd("status", None),
        Commands::Scan { path } => scan(&path),
        Commands::AddLib { name, url } => add_lib(&name, url.as_deref()),
    }
}

fn sensei_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".sensei")
}

fn plugin_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".claude/plugins/sensei")
}

fn daemon_bin() -> PathBuf {
    plugin_dir().join("bin/senseid")
}

fn mcp_bin() -> PathBuf {
    plugin_dir().join("bin/sensei-mcp")
}

// ── Install ──────────────────────────────────────────────────────────────────

fn install(acp: &str, scope: &str) {
    let plugin = plugin_dir();
    println!("Installing sensei for {}...", acp);

    // Create plugin directories
    fs::create_dir_all(plugin.join("bin")).ok();
    fs::create_dir_all(plugin.join("hooks")).ok();
    fs::create_dir_all(sensei_dir()).ok();

    // Copy binaries — find them relative to this binary's location
    let self_path = std::env::current_exe().unwrap_or_default();
    let self_dir = self_path.parent().unwrap_or(Path::new("."));

    // Look for sibling binaries (same directory as this CLI)
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
            println!("  Installed {}", bin_name);
        } else {
            println!("  Warning: {} not found at {:?}", bin_name, src);
        }
    }

    // Write hooks
    write_hook(&plugin.join("hooks/session-start"), include_str!("../../../plugin/hooks/session-start"));
    write_hook(&plugin.join("hooks/pre-tool"), include_str!("../../../plugin/hooks/pre-tool"));
    write_hook(&plugin.join("hooks/post-tool"), include_str!("../../../plugin/hooks/post-tool"));
    write_hook(&plugin.join("hooks/run-hook.cmd"), include_str!("../../../plugin/hooks/run-hook.cmd"));

    // Configure ACP
    match acp {
        "claude-code" => configure_claude_code(&plugin),
        "cursor" => configure_cursor(&plugin),
        "windsurf" => configure_windsurf(&plugin),
        _ => println!("  Unknown ACP: {}. Skipping configuration.", acp),
    }

    println!("\nSensei installed successfully!");
    println!("  Start daemon: sensei start");
    println!("  Scan repos:   sensei scan ~/Developer");
}

fn write_hook(path: &Path, content: &str) {
    fs::write(path, content).ok();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).ok();
    }
}

fn configure_claude_code(plugin: &Path) {
    let home = dirs::home_dir().unwrap_or_default();

    // Configure MCP in ~/.claude.json
    let claude_json = home.join(".claude.json");
    let mut config: serde_json::Value = if claude_json.exists() {
        serde_json::from_str(&fs::read_to_string(&claude_json).unwrap_or_default()).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    config.as_object_mut().unwrap()
        .entry("mcpServers")
        .or_insert(serde_json::json!({}))
        .as_object_mut().unwrap()
        .insert("sensei".into(), serde_json::json!({
            "command": mcp_bin().to_string_lossy(),
            "args": []
        }));

    fs::write(&claude_json, serde_json::to_string_pretty(&config).unwrap()).ok();
    println!("  Configured MCP in ~/.claude.json");

    // Configure hooks in ~/.claude/hooks.json
    let hooks_file = home.join(".claude/hooks.json");
    let hooks_dir = plugin.join("hooks").to_string_lossy().to_string();

    let hooks = serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "matcher": "startup|resume|clear|compact",
                "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd session-start", hooks_dir)}]
            }],
            "PreToolExecution": [{
                "matcher": "",
                "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd pre-tool", hooks_dir)}]
            }],
            "PostToolExecution": [{
                "matcher": "",
                "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd post-tool", hooks_dir)}]
            }]
        }
    });

    fs::write(&hooks_file, serde_json::to_string_pretty(&hooks).unwrap()).ok();
    println!("  Configured hooks in ~/.claude/hooks.json");
}

fn configure_cursor(plugin: &Path) {
    let home = dirs::home_dir().unwrap_or_default();
    let config_path = home.join(".cursor/mcp.json");
    fs::create_dir_all(config_path.parent().unwrap()).ok();

    let mut config: serde_json::Value = if config_path.exists() {
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap_or_default()).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    config.as_object_mut().unwrap()
        .entry("mcpServers")
        .or_insert(serde_json::json!({}))
        .as_object_mut().unwrap()
        .insert("sensei".into(), serde_json::json!({
            "command": mcp_bin().to_string_lossy(),
            "args": []
        }));

    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).ok();
    println!("  Configured MCP in ~/.cursor/mcp.json");
}

fn configure_windsurf(plugin: &Path) {
    let home = dirs::home_dir().unwrap_or_default();
    let config_path = home.join(".windsurf/mcp.json");
    fs::create_dir_all(config_path.parent().unwrap()).ok();

    let mut config: serde_json::Value = if config_path.exists() {
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap_or_default()).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    config.as_object_mut().unwrap()
        .entry("mcpServers")
        .or_insert(serde_json::json!({}))
        .as_object_mut().unwrap()
        .insert("sensei".into(), serde_json::json!({
            "command": mcp_bin().to_string_lossy(),
            "args": []
        }));

    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).ok();
    println!("  Configured MCP in ~/.windsurf/mcp.json");
}

// ── Uninstall ────────────────────────────────────────────────────────────────

fn uninstall() {
    let plugin = plugin_dir();
    if plugin.exists() {
        fs::remove_dir_all(&plugin).ok();
        println!("Removed {}", plugin.display());
    }

    // Remove MCP from ~/.claude.json
    let claude_json = dirs::home_dir().unwrap_or_default().join(".claude.json");
    if claude_json.exists() {
        if let Ok(content) = fs::read_to_string(&claude_json) {
            if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(servers) = config.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
                    servers.remove("sensei");
                }
                fs::write(&claude_json, serde_json::to_string_pretty(&config).unwrap()).ok();
            }
        }
    }
    println!("Sensei uninstalled.");
}

// ── Daemon commands ──────────────────────────────────────────────────────────

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
    let status = std::process::Command::new(&bin).args(&args).status();
    match status {
        Ok(s) => std::process::exit(s.code().unwrap_or(0)),
        Err(e) => { eprintln!("Failed to run senseid: {}", e); std::process::exit(1); }
    }
}

// ── Scan ─────────────────────────────────────────────────────────────────────

fn scan(path: &str) {
    let client = reqwest::blocking::Client::new();
    match client.post("http://127.0.0.1:7744/api/scan")
        .json(&serde_json::json!({"root": path, "max_depth": 4}))
        .send()
    {
        Ok(resp) if resp.status().is_success() => {
            println!("Scanning {} (background)...", path);
            println!("Projects will appear in the desktop app as they're discovered.");
        }
        Ok(resp) => eprintln!("Scan failed: HTTP {}", resp.status()),
        Err(e) => eprintln!("Cannot reach daemon: {}. Run: sensei start", e),
    }
}

// ── Add library ──────────────────────────────────────────────────────────────

fn add_lib(name: &str, url: Option<&str>) {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build().unwrap();

    let mut body = serde_json::json!({"tool": "add_library", "params": {"name": name}});
    if let Some(u) = url {
        body["params"]["url"] = serde_json::json!(u);
    }

    match client.post("http://127.0.0.1:7744/api/mcp/call")
        .json(&body)
        .send()
    {
        Ok(resp) if resp.status().is_success() => {
            let data: serde_json::Value = resp.json().unwrap_or_default();
            if data["ok"].as_bool() == Some(true) {
                println!("Indexed {} docs for {} from {}", data["docsIndexed"], name, data["url"].as_str().unwrap_or("auto-discovered URL"));
            } else {
                println!("{}", data.get("error").and_then(|e| e.as_str()).unwrap_or("Failed to index"));
            }
        }
        Ok(resp) => eprintln!("Failed: HTTP {}", resp.status()),
        Err(e) => eprintln!("Cannot reach daemon: {}. Run: sensei start", e),
    }
}
