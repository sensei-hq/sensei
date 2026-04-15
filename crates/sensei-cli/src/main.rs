use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::fs;

const MARKETPLACE_REPO: &str = "https://raw.githubusercontent.com/mizukisu/sensei-marketplace/main";
const MARKETPLACE_CATALOG: &str = "catalog.json";

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

fn sensei_dir() -> PathBuf { dirs::home_dir().unwrap_or_default().join(".sensei") }
fn plugin_dir() -> PathBuf { dirs::home_dir().unwrap_or_default().join(".claude/plugins/sensei") }
fn daemon_bin() -> PathBuf { plugin_dir().join("bin/senseid") }
fn mcp_bin() -> PathBuf { plugin_dir().join("bin/sensei-mcp") }
fn config_file() -> PathBuf { sensei_dir().join("config.json") }
fn cache_dir() -> PathBuf { sensei_dir().join("cache/marketplace") }

// ── Config ───────────────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct SenseiConfig {
    #[serde(default)]
    configured_acps: Vec<String>,
    #[serde(default)]
    marketplace_version: String,
}

fn load_config() -> SenseiConfig {
    config_file().exists()
        .then(|| fs::read_to_string(config_file()).ok())
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(config: &SenseiConfig) {
    fs::create_dir_all(sensei_dir()).ok();
    fs::write(config_file(), serde_json::to_string_pretty(config).unwrap()).ok();
}

// ── ACP Detection ────────────────────────────────────────────────────────────

struct AcpInfo {
    name: &'static str,
    id: &'static str,
    detected: bool,
}

fn detect_acps() -> Vec<AcpInfo> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        AcpInfo { name: "Claude Code", id: "claude-code", detected: home.join(".claude").exists() },
        AcpInfo { name: "Cursor", id: "cursor", detected: home.join(".cursor").exists() || which("cursor") },
        AcpInfo { name: "Windsurf", id: "windsurf", detected: home.join(".windsurf").exists() || which("windsurf") },
        AcpInfo { name: "Zed", id: "zed", detected: which("zed") },
        AcpInfo { name: "VS Code", id: "vscode", detected: which("code") },
    ]
}

fn which(cmd: &str) -> bool {
    std::process::Command::new("which").arg(cmd).output().map(|o| o.status.success()).unwrap_or(false)
}

// ── Configure ────────────────────────────────────────────────────────────────

fn configure() {
    println!("Detecting AI coding platforms...\n");

    let acps = detect_acps();
    let mut detected: Vec<&str> = Vec::new();

    for acp in &acps {
        let status = if acp.detected { "detected" } else { "not found" };
        let mark = if acp.detected { "✓" } else { "·" };
        println!("  {} {} ({})", mark, acp.name, status);
        if acp.detected { detected.push(acp.id); }
    }

    if detected.is_empty() {
        println!("\nNo AI coding platforms detected.");
        println!("Install Claude Code, Cursor, or Windsurf and run again.");
        return;
    }

    println!("\nConfiguring sensei for: {}", detected.join(", "));

    let mut config = load_config();
    config.configured_acps = detected.iter().map(|s| s.to_string()).collect();
    save_config(&config);

    println!("Configuration saved. Run: sensei install");
}

// ── Install ──────────────────────────────────────────────────────────────────

fn install(specific_acp: Option<&str>, scope: &str) {
    let mut config = load_config();

    // If no ACPs configured, run configure first
    if config.configured_acps.is_empty() && specific_acp.is_none() {
        println!("No ACPs configured. Running detection...\n");
        configure();
        config = load_config();
        if config.configured_acps.is_empty() { return; }
        println!();
    }

    let acps: Vec<String> = if let Some(acp) = specific_acp {
        vec![acp.to_string()]
    } else {
        config.configured_acps.clone()
    };

    println!("Installing sensei...\n");

    // 1. Install binaries
    println!("[1/4] Binaries...");
    install_binaries();

    // 2. Install hooks (embedded)
    println!("[2/4] Hooks...");
    let plugin = plugin_dir();
    fs::create_dir_all(plugin.join("hooks")).ok();
    write_hook(&plugin.join("hooks/session-start"), include_str!("../../../marketplace/hooks/session-start"));
    write_hook(&plugin.join("hooks/pre-tool"), include_str!("../../../marketplace/hooks/pre-tool"));
    write_hook(&plugin.join("hooks/post-tool"), include_str!("../../../marketplace/hooks/post-tool"));
    write_hook(&plugin.join("hooks/run-hook.cmd"), include_str!("../../../marketplace/hooks/run-hook.cmd"));

    // 3. Install skills & commands (cached marketplace)
    println!("[3/4] Skills & commands...");
    install_marketplace(scope, &acps, &mut config);

    // 4. Configure each ACP
    println!("[4/4] Configuring ACPs...");
    for acp in &acps {
        match acp.as_str() {
            "claude-code" => configure_claude_code(),
            "cursor" => configure_generic_mcp("cursor", ".cursor/mcp.json"),
            "windsurf" => configure_generic_mcp("windsurf", ".windsurf/mcp.json"),
            "zed" => configure_generic_mcp("zed", ".config/zed/mcp.json"),
            _ => println!("  {} — skipped (unknown ACP)", acp),
        }
    }

    save_config(&config);

    println!("\nSensei installed for: {}", acps.join(", "));
    println!("  Start daemon: sensei start");
    println!("  Scan repos:   sensei scan ~/Developer");
}

fn install_binaries() {
    let plugin = plugin_dir();
    fs::create_dir_all(plugin.join("bin")).ok();
    fs::create_dir_all(sensei_dir()).ok();

    let self_path = std::env::current_exe().unwrap_or_default();
    let self_dir = self_path.parent().unwrap_or(Path::new("."));

    for bin_name in &["senseid", "sensei-mcp"] {
        let src = self_dir.join(bin_name);
        let dst = plugin.join("bin").join(bin_name);
        if src.exists() {
            fs::copy(&src, &dst).ok();
            #[cfg(unix)]
            { use std::os::unix::fs::PermissionsExt; fs::set_permissions(&dst, fs::Permissions::from_mode(0o755)).ok(); }
            println!("  {}", bin_name);
        }
    }
}

fn write_hook(path: &Path, content: &str) {
    fs::write(path, content).ok();
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; fs::set_permissions(path, fs::Permissions::from_mode(0o755)).ok(); }
}

// ── Marketplace (cached) ─────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct Catalog { version: Option<String>, items: Vec<CatalogItem> }

#[derive(serde::Deserialize)]
struct CatalogItem { name: String, kind: String, scope: String, path: String }

fn install_marketplace(scope: &str, acps: &[String], config: &mut SenseiConfig) {
    let catalog = load_or_fetch_catalog(config);
    let catalog = match catalog {
        Some(c) => c,
        None => { println!("  Could not load marketplace. Skipping."); return; }
    };

    let supports_skills = acps.iter().any(|a| a == "claude-code");
    let items: Vec<&CatalogItem> = catalog.items.iter()
        .filter(|i| scope == "all" || i.scope == scope || i.scope == "global")
        .filter(|i| match i.kind.as_str() {
            "skill" | "command" => supports_skills,
            _ => false,
        })
        .collect();

    let home = dirs::home_dir().unwrap_or_default();
    let cache = cache_dir();
    let mut installed = 0;

    for item in &items {
        let cached = cache.join(&item.path);
        let content = if cached.exists() {
            fs::read_to_string(&cached).ok()
        } else {
            download_and_cache(&item.path)
        };

        let content = match content { Some(c) => c, None => continue };

        let dest = match item.kind.as_str() {
            "skill" => home.join(".claude/skills").join(format!("{}.md", item.name)),
            "command" => home.join(".claude/commands").join(format!("{}.md", item.name)),
            _ => continue,
        };
        fs::create_dir_all(dest.parent().unwrap()).ok();
        fs::write(&dest, &content).ok();
        installed += 1;
    }

    println!("  {} items installed", installed);
}

fn load_or_fetch_catalog(config: &mut SenseiConfig) -> Option<Catalog> {
    let cache = cache_dir();
    let cached_catalog = cache.join(MARKETPLACE_CATALOG);

    // Check if cached version matches
    if cached_catalog.exists() {
        if let Ok(content) = fs::read_to_string(&cached_catalog) {
            if let Ok(catalog) = serde_json::from_str::<Catalog>(&content) {
                let cached_ver = catalog.version.as_deref().unwrap_or("");
                if !cached_ver.is_empty() && cached_ver == config.marketplace_version {
                    println!("  Using cached marketplace v{}", cached_ver);
                    return Some(catalog);
                }
            }
        }
    }

    // Download fresh
    println!("  Downloading marketplace catalog...");
    let url = format!("{}/{}", MARKETPLACE_REPO, MARKETPLACE_CATALOG);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10)).build().ok()?;
    let resp = client.get(&url).send().ok()?;
    if !resp.status().is_success() { return None; }
    let text = resp.text().ok()?;

    // Cache it
    fs::create_dir_all(&cache).ok();
    fs::write(&cached_catalog, &text).ok();

    let catalog: Catalog = serde_json::from_str(&text).ok()?;
    config.marketplace_version = catalog.version.clone().unwrap_or_default();
    Some(catalog)
}

fn download_and_cache(path: &str) -> Option<String> {
    let url = format!("{}/{}", MARKETPLACE_REPO, path);
    let cache = cache_dir().join(path);
    fs::create_dir_all(cache.parent()?).ok();

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10)).build().ok()?;
    let resp = client.get(&url).send().ok()?;
    if !resp.status().is_success() { return None; }
    let text = resp.text().ok()?;
    fs::write(&cache, &text).ok();
    Some(text)
}

// ── ACP Configuration ────────────────────────────────────────────────────────

fn configure_claude_code() {
    let home = dirs::home_dir().unwrap_or_default();
    let plugin = plugin_dir();
    let hooks_dir = plugin.join("hooks").to_string_lossy().to_string();

    // MCP
    let claude_json = home.join(".claude.json");
    let mut config: serde_json::Value = claude_json.exists()
        .then(|| fs::read_to_string(&claude_json).ok()).flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));

    config.as_object_mut().unwrap()
        .entry("mcpServers").or_insert(serde_json::json!({}))
        .as_object_mut().unwrap()
        .insert("sensei".into(), serde_json::json!({ "command": mcp_bin().to_string_lossy(), "args": [] }));
    fs::write(&claude_json, serde_json::to_string_pretty(&config).unwrap()).ok();

    // Hooks
    let hooks_file = home.join(".claude/hooks.json");
    let hooks = serde_json::json!({ "hooks": {
        "SessionStart": [{"matcher": "startup|resume|clear|compact", "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd session-start", hooks_dir)}]}],
        "PreToolExecution": [{"matcher": "", "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd pre-tool", hooks_dir)}]}],
        "PostToolExecution": [{"matcher": "", "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd post-tool", hooks_dir)}]}],
    }});
    fs::write(&hooks_file, serde_json::to_string_pretty(&hooks).unwrap()).ok();

    println!("  Claude Code — MCP + hooks + skills");
}

fn configure_generic_mcp(name: &str, config_path: &str) {
    let home = dirs::home_dir().unwrap_or_default();
    let full_path = home.join(config_path);
    fs::create_dir_all(full_path.parent().unwrap()).ok();

    let mut config: serde_json::Value = full_path.exists()
        .then(|| fs::read_to_string(&full_path).ok()).flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));

    config.as_object_mut().unwrap()
        .entry("mcpServers").or_insert(serde_json::json!({}))
        .as_object_mut().unwrap()
        .insert("sensei".into(), serde_json::json!({ "command": mcp_bin().to_string_lossy(), "args": [] }));
    fs::write(&full_path, serde_json::to_string_pretty(&config).unwrap()).ok();

    println!("  {} — MCP", name);
}

// ── Uninstall ────────────────────────────────────────────────────────────────

fn uninstall() {
    let home = dirs::home_dir().unwrap_or_default();

    if plugin_dir().exists() { fs::remove_dir_all(plugin_dir()).ok(); println!("Removed plugin"); }
    if cache_dir().exists() { fs::remove_dir_all(cache_dir()).ok(); }

    // Remove MCP from all known configs
    for path in &[".claude.json", ".cursor/mcp.json", ".windsurf/mcp.json", ".config/zed/mcp.json"] {
        let full = home.join(path);
        if full.exists() {
            if let Ok(s) = fs::read_to_string(&full) {
                if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&s) {
                    if let Some(servers) = config.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
                        servers.remove("sensei");
                    }
                    fs::write(&full, serde_json::to_string_pretty(&config).unwrap()).ok();
                }
            }
        }
    }

    // Remove hooks
    let hooks_file = home.join(".claude/hooks.json");
    if hooks_file.exists() { fs::remove_file(&hooks_file).ok(); }

    // Clear config
    if config_file().exists() { fs::remove_file(config_file()).ok(); }

    println!("Sensei uninstalled.");
}

// ── Daemon / Scan / AddLib ───────────────────────────────────────────────────

fn daemon_cmd(cmd: &str, port: Option<u16>) {
    let bin = daemon_bin();
    if !bin.exists() { eprintln!("senseid not found. Run: sensei install"); std::process::exit(1); }
    let mut args = vec![cmd.to_string()];
    if let Some(p) = port { args.push("--port".into()); args.push(p.to_string()); }
    match std::process::Command::new(&bin).args(&args).status() {
        Ok(s) => std::process::exit(s.code().unwrap_or(0)),
        Err(e) => { eprintln!("Failed: {}", e); std::process::exit(1); }
    }
}

fn scan(path: &str) {
    let client = reqwest::blocking::Client::new();
    match client.post("http://127.0.0.1:7744/api/scan")
        .json(&serde_json::json!({"root": path, "max_depth": 4})).send()
    {
        Ok(r) if r.status().is_success() => println!("Scanning {} (background)...", path),
        _ => eprintln!("Cannot reach daemon. Run: sensei start"),
    }
}

fn add_lib(name: &str, url: Option<&str>) {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(45)).build().unwrap();
    let mut body = serde_json::json!({"tool": "add_library", "params": {"name": name}});
    if let Some(u) = url { body["params"]["url"] = serde_json::json!(u); }
    match client.post("http://127.0.0.1:7744/api/mcp/call").json(&body).send() {
        Ok(r) if r.status().is_success() => {
            let d: serde_json::Value = r.json().unwrap_or_default();
            if d["ok"].as_bool() == Some(true) {
                println!("Indexed {} docs for {} from {}", d["docsIndexed"], name, d["url"].as_str().unwrap_or("?"));
            } else { println!("{}", d["error"].as_str().unwrap_or("Failed")); }
        }
        _ => eprintln!("Cannot reach daemon. Run: sensei start"),
    }
}
