use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::fs;

const MARKETPLACE_REPO: &str = "https://raw.githubusercontent.com/jerrythomas/sensei-marketplace/main";
const MARKETPLACE_CATALOG: &str = "catalog.json";

#[derive(Parser)]
#[command(name = "sensei", about = "Sensei — AI coding companion", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install sensei plugin, skills, and configure ACP
    Install {
        /// Target ACP: claude-code, cursor, windsurf
        #[arg(long, default_value = "claude-code")]
        acp: String,

        /// Skills scope: global, all
        #[arg(long, default_value = "global")]
        scope: String,

        /// Path to local marketplace repo (skips download)
        #[arg(long)]
        marketplace: Option<String>,
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
        Commands::Install { acp, scope, marketplace } => install(&acp, &scope, marketplace.as_deref()),
        Commands::Uninstall => uninstall(),
        Commands::Start { port } => daemon_cmd("start", Some(port)),
        Commands::Stop => daemon_cmd("stop", None),
        Commands::Status => daemon_cmd("status", None),
        Commands::Scan { path } => scan(&path),
        Commands::AddLib { name, url } => add_lib(&name, url.as_deref()),
    }
}

fn sensei_dir() -> PathBuf { dirs::home_dir().unwrap_or_default().join(".sensei") }
fn plugin_dir() -> PathBuf { dirs::home_dir().unwrap_or_default().join(".claude/plugins/sensei") }
fn daemon_bin() -> PathBuf { plugin_dir().join("bin/senseid") }
fn mcp_bin() -> PathBuf { plugin_dir().join("bin/sensei-mcp") }

// ── Install ──────────────────────────────────────────────────────────────────

fn install(acp: &str, scope: &str, marketplace_path: Option<&str>) {
    let plugin = plugin_dir();
    println!("Installing sensei for {}...\n", acp);

    fs::create_dir_all(plugin.join("bin")).ok();
    fs::create_dir_all(plugin.join("hooks")).ok();
    fs::create_dir_all(sensei_dir()).ok();

    // 1. Install binaries
    println!("[1/4] Installing binaries...");
    install_binaries(&plugin);

    // 2. Install hooks (embedded in binary)
    println!("[2/4] Installing hooks...");
    write_hook(&plugin.join("hooks/session-start"), include_str!("../../../plugin/hooks/session-start"));
    write_hook(&plugin.join("hooks/pre-tool"), include_str!("../../../plugin/hooks/pre-tool"));
    write_hook(&plugin.join("hooks/post-tool"), include_str!("../../../plugin/hooks/post-tool"));
    write_hook(&plugin.join("hooks/run-hook.cmd"), include_str!("../../../plugin/hooks/run-hook.cmd"));

    // 3. Install skills & commands from marketplace
    println!("[3/4] Installing skills & commands...");
    install_marketplace(acp, scope, marketplace_path);

    // 4. Configure ACP
    println!("[4/4] Configuring {}...", acp);
    match acp {
        "claude-code" => configure_claude_code(&plugin),
        "cursor" => configure_cursor(),
        "windsurf" => configure_windsurf(),
        _ => println!("  Unknown ACP: {}. Skipping.", acp),
    }

    println!("\nSensei installed successfully!");
    println!("  Start daemon: sensei start");
    println!("  Scan repos:   sensei scan ~/Developer");
}

fn install_binaries(plugin: &Path) {
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
            println!("  {} installed", bin_name);
        } else {
            println!("  {} not found (expected at {:?})", bin_name, src);
        }
    }
}

fn write_hook(path: &Path, content: &str) {
    fs::write(path, content).ok();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).ok();
    }
}

// ── Marketplace ──────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct Catalog {
    items: Vec<CatalogItem>,
}

#[derive(serde::Deserialize)]
struct CatalogItem {
    name: String,
    kind: String,
    description: String,
    scope: String,
    path: String,
    #[serde(default)]
    recommended_for: Vec<String>,
}

fn install_marketplace(acp: &str, scope: &str, local_path: Option<&str>) {
    // Load catalog from local path or download
    let catalog = if let Some(path) = local_path {
        load_local_catalog(path)
    } else {
        download_catalog()
    };

    let catalog = match catalog {
        Some(c) => c,
        None => {
            println!("  Could not load marketplace catalog. Skipping skills/commands.");
            return;
        }
    };

    // Filter by scope
    let items: Vec<&CatalogItem> = catalog.items.iter()
        .filter(|i| scope == "all" || i.scope == scope || i.scope == "global")
        .filter(|i| i.kind == "skill" || i.kind == "command")
        .collect();

    println!("  {} items to install", items.len());

    let home = dirs::home_dir().unwrap_or_default();
    let mut installed = 0;

    for item in &items {
        let content = if let Some(path) = local_path {
            fs::read_to_string(Path::new(path).join(&item.path)).ok()
        } else {
            download_file(&format!("{}/{}", MARKETPLACE_REPO, item.path))
        };

        let content = match content {
            Some(c) => c,
            None => continue,
        };

        match item.kind.as_str() {
            "skill" => {
                let dest = match acp {
                    "claude-code" => home.join(".claude/skills").join(format!("{}.md", item.name)),
                    _ => continue, // other ACPs don't support skills
                };
                fs::create_dir_all(dest.parent().unwrap()).ok();
                fs::write(&dest, &content).ok();
                installed += 1;
            }
            "command" => {
                let dest = match acp {
                    "claude-code" => home.join(".claude/commands").join(format!("{}.md", item.name)),
                    _ => continue,
                };
                fs::create_dir_all(dest.parent().unwrap()).ok();
                fs::write(&dest, &content).ok();
                installed += 1;
            }
            _ => {}
        }
    }

    println!("  {} skills/commands installed", installed);
}

fn load_local_catalog(path: &str) -> Option<Catalog> {
    let content = fs::read_to_string(Path::new(path).join(MARKETPLACE_CATALOG)).ok()?;
    serde_json::from_str(&content).ok()
}

fn download_catalog() -> Option<Catalog> {
    let url = format!("{}/{}", MARKETPLACE_REPO, MARKETPLACE_CATALOG);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build().ok()?;
    let resp = client.get(&url).send().ok()?;
    if !resp.status().is_success() { return None; }
    resp.json().ok()
}

fn download_file(url: &str) -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build().ok()?;
    let resp = client.get(url).send().ok()?;
    if !resp.status().is_success() { return None; }
    resp.text().ok()
}

// ── ACP Configuration ────────────────────────────────────────────────────────

fn configure_claude_code(plugin: &Path) {
    let home = dirs::home_dir().unwrap_or_default();
    let hooks_dir = plugin.join("hooks").to_string_lossy().to_string();

    // MCP in ~/.claude.json
    let claude_json = home.join(".claude.json");
    let mut config: serde_json::Value = if claude_json.exists() {
        serde_json::from_str(&fs::read_to_string(&claude_json).unwrap_or_default()).unwrap_or(serde_json::json!({}))
    } else { serde_json::json!({}) };

    config.as_object_mut().unwrap()
        .entry("mcpServers").or_insert(serde_json::json!({}))
        .as_object_mut().unwrap()
        .insert("sensei".into(), serde_json::json!({
            "command": mcp_bin().to_string_lossy(),
            "args": []
        }));

    fs::write(&claude_json, serde_json::to_string_pretty(&config).unwrap()).ok();
    println!("  MCP configured in ~/.claude.json");

    // Hooks in ~/.claude/hooks.json
    let hooks_file = home.join(".claude/hooks.json");
    let hooks = serde_json::json!({
        "hooks": {
            "SessionStart": [{"matcher": "startup|resume|clear|compact", "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd session-start", hooks_dir)}]}],
            "PreToolExecution": [{"matcher": "", "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd pre-tool", hooks_dir)}]}],
            "PostToolExecution": [{"matcher": "", "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd post-tool", hooks_dir)}]}]
        }
    });
    fs::write(&hooks_file, serde_json::to_string_pretty(&hooks).unwrap()).ok();
    println!("  Hooks configured in ~/.claude/hooks.json");
}

fn configure_cursor() {
    let config_path = dirs::home_dir().unwrap_or_default().join(".cursor/mcp.json");
    fs::create_dir_all(config_path.parent().unwrap()).ok();
    let mut config: serde_json::Value = if config_path.exists() {
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap_or_default()).unwrap_or(serde_json::json!({}))
    } else { serde_json::json!({}) };

    config.as_object_mut().unwrap()
        .entry("mcpServers").or_insert(serde_json::json!({}))
        .as_object_mut().unwrap()
        .insert("sensei".into(), serde_json::json!({
            "command": mcp_bin().to_string_lossy(), "args": []
        }));

    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).ok();
    println!("  MCP configured in ~/.cursor/mcp.json");
}

fn configure_windsurf() {
    let config_path = dirs::home_dir().unwrap_or_default().join(".windsurf/mcp.json");
    fs::create_dir_all(config_path.parent().unwrap()).ok();
    let mut config: serde_json::Value = if config_path.exists() {
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap_or_default()).unwrap_or(serde_json::json!({}))
    } else { serde_json::json!({}) };

    config.as_object_mut().unwrap()
        .entry("mcpServers").or_insert(serde_json::json!({}))
        .as_object_mut().unwrap()
        .insert("sensei".into(), serde_json::json!({
            "command": mcp_bin().to_string_lossy(), "args": []
        }));

    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).ok();
    println!("  MCP configured in ~/.windsurf/mcp.json");
}

// ── Uninstall ────────────────────────────────────────────────────────────────

fn uninstall() {
    let home = dirs::home_dir().unwrap_or_default();

    // Remove plugin dir
    let plugin = plugin_dir();
    if plugin.exists() {
        fs::remove_dir_all(&plugin).ok();
        println!("Removed plugin: {}", plugin.display());
    }

    // Remove installed skills/commands
    let skills_dir = home.join(".claude/skills");
    let commands_dir = home.join(".claude/commands");
    // Only remove sensei-installed files (check marketplace catalog names)
    for dir in &[&skills_dir, &commands_dir] {
        if dir.exists() {
            // Remove all .md files (sensei installed them)
            for entry in fs::read_dir(dir).into_iter().flatten() {
                if let Ok(e) = entry {
                    if e.path().extension().map(|x| x == "md").unwrap_or(false) {
                        fs::remove_file(e.path()).ok();
                    }
                }
            }
        }
    }
    println!("Removed skills and commands");

    // Remove MCP from configs
    for config_path in &[
        home.join(".claude.json"),
        home.join(".cursor/mcp.json"),
        home.join(".windsurf/mcp.json"),
    ] {
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(config_path) {
                if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(servers) = config.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
                        servers.remove("sensei");
                    }
                    fs::write(config_path, serde_json::to_string_pretty(&config).unwrap()).ok();
                }
            }
        }
    }

    // Remove hooks
    let hooks_file = home.join(".claude/hooks.json");
    if hooks_file.exists() {
        fs::remove_file(&hooks_file).ok();
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
    match std::process::Command::new(&bin).args(&args).status() {
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
            println!("Projects will appear as they're discovered.");
        }
        Ok(resp) => eprintln!("Scan failed: HTTP {}", resp.status()),
        Err(e) => eprintln!("Cannot reach daemon: {}. Run: sensei start", e),
    }
}

// ── Add library ──────────────────────────────────────────────────────────────

fn add_lib(name: &str, url: Option<&str>) {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build().unwrap();

    let mut body = serde_json::json!({"tool": "add_library", "params": {"name": name}});
    if let Some(u) = url { body["params"]["url"] = serde_json::json!(u); }

    match client.post("http://127.0.0.1:7744/api/mcp/call").json(&body).send() {
        Ok(resp) if resp.status().is_success() => {
            let data: serde_json::Value = resp.json().unwrap_or_default();
            if data["ok"].as_bool() == Some(true) {
                println!("Indexed {} docs for {} from {}", data["docsIndexed"], name, data["url"].as_str().unwrap_or("?"));
            } else {
                println!("{}", data.get("error").and_then(|e| e.as_str()).unwrap_or("Failed"));
            }
        }
        Ok(resp) => eprintln!("Failed: HTTP {}", resp.status()),
        Err(e) => eprintln!("Cannot reach daemon: {}. Run: sensei start", e),
    }
}
