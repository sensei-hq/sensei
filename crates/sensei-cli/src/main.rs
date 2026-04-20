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
    /// Initialize sensei for the current repo (.sensei/, .claude/, .mcp.json)
    Init {
        /// Path to the plugin/marketplace directory (default: auto-detect)
        #[arg(long)]
        plugin_dir: Option<String>,
    },

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
        Commands::Init { plugin_dir } => init_repo(plugin_dir.as_deref()),
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
    if daemon_available() { return; }

    // Try auto-start
    eprintln!("Daemon not running — starting...");
    let bin = daemon_bin();
    if bin.exists() {
        std::process::Command::new(&bin)
            .args(["start", "--port", "7744"])
            .spawn()
            .ok();
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            if daemon_available() {
                eprintln!("Daemon started.");
                return;
            }
        }
    }

    // Also try senseid on PATH
    if std::process::Command::new("senseid")
        .args(["start", "--port", "7744"])
        .spawn()
        .is_ok()
    {
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            if daemon_available() {
                eprintln!("Daemon started.");
                return;
            }
        }
    }

    eprintln!("Could not start daemon. Run manually: senseid start");
    std::process::exit(1);
}

// ── Init (per-repo, no daemon required) ─────────────────────────────────────

fn init_repo(plugin_dir_arg: Option<&str>) {
    let repo_root = std::env::current_dir().expect("Cannot determine current directory");
    println!("=== sensei init ===");
    println!("Repo: {}\n", repo_root.display());

    // Resolve plugin directory
    let plugin_root = if let Some(pd) = plugin_dir_arg {
        PathBuf::from(pd)
    } else {
        // Auto-detect: check for marketplace/ in repo, then ~/.sensei/marketplace/
        let local = repo_root.join("marketplace");
        if local.join("mindsets").exists() || local.join("templates/mindsets.md").exists() {
            local
        } else {
            let global = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join(".sensei/marketplace");
            if global.join("mindsets").exists() || global.join("templates/mindsets.md").exists() {
                global
            } else {
                eprintln!("Cannot find plugin directory. Use --plugin-dir or ensure marketplace/ exists.");
                std::process::exit(1);
            }
        }
    };
    println!("Plugin: {}\n", plugin_root.display());

    // ── 1. Create .sensei/ ──────────────────────────────────────────────────
    let sensei_dir = repo_root.join(".sensei");
    fs::create_dir_all(&sensei_dir).ok();

    let rules_file = sensei_dir.join("rules.md");
    if !rules_file.exists() {
        let project_name = repo_root.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project");
        let today = {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            // Simple date formatting (YYYY-MM-DD) without chrono
            let days = now / 86400;
            let mut y = 1970i32;
            let mut remaining = days as i32;
            loop {
                let year_days = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
                if remaining < year_days { break; }
                remaining -= year_days;
                y += 1;
            }
            let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
            let mdays = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
            let mut m = 0usize;
            while m < 12 && remaining >= mdays[m] { remaining -= mdays[m]; m += 1; }
            format!("{}-{:02}-{:02}", y, m + 1, remaining + 1)
        };
        fs::write(&rules_file, format!(
            "---\nname: Project Rules — {}\nupdated: {}\nmindsets: .sensei/mindsets.md\npersonas: .sensei/personas/\n---\n\n# Rules\n\n> Mindsets loaded from `.sensei/mindsets.md`. This file contains project-specific rules.\n\n## Patterns\n\n<!-- Add project patterns here -->\n\n## Quality\n\n- **Zero errors** — test suite must pass before and after every change\n\n## Process\n\n- **Design before code** — analyst mindset first\n- **One issue at a time** — complete, verify, close, then next\n",
            project_name, today,
        )).ok();
        println!("[created] .sensei/rules.md");
    } else {
        println!("[exists]  .sensei/rules.md");
    }

    let personas_dir = sensei_dir.join("personas");
    if !personas_dir.exists() {
        fs::create_dir_all(&personas_dir).ok();
        println!("[created] .sensei/personas/ (empty — use /sensei:persona add)");
    } else {
        let count = fs::read_dir(&personas_dir)
            .map(|rd| rd.filter(|e| e.as_ref().map(|e| e.path().extension().map_or(false, |x| x == "md")).unwrap_or(false)).count())
            .unwrap_or(0);
        println!("[exists]  .sensei/personas/ ({} personas)", count);
    }

    // Copy mindsets from plugin into .sensei/mindsets/ (always update on init)
    let mindsets_src = plugin_root.join("mindsets");
    let mindsets_dst = sensei_dir.join("mindsets");
    if mindsets_src.exists() && mindsets_src.is_dir() {
        fs::create_dir_all(&mindsets_dst).ok();
        let mut count = 0;
        for entry in fs::read_dir(&mindsets_src).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "md") {
                let dst = mindsets_dst.join(entry.file_name());
                fs::copy(&path, &dst).ok();
                count += 1;
            }
        }
        println!("[copied]  .sensei/mindsets/ ({} mindsets from plugin)", count);
    } else {
        // Fall back to single mindsets.md
        let single_src = plugin_root.join("templates/mindsets.md");
        if single_src.exists() {
            fs::create_dir_all(&mindsets_dst).ok();
            fs::copy(&single_src, mindsets_dst.join("all.md")).ok();
            println!("[copied]  .sensei/mindsets/ (single file from plugin/templates)");
        } else {
            println!("[WARN]    mindsets not found in plugin");
        }
    }

    // ── 2. Create .mcp.json ─────────────────────────────────────────────────
    let mcp_file = repo_root.join(".mcp.json");
    fs::write(&mcp_file, "{\n  \"mcpServers\": {\n    \"sensei\": {\n      \"command\": \"senseid\",\n      \"args\": [\"--mcp\"]\n    }\n  }\n}\n").ok();
    println!("[ok]      .mcp.json");

    // ── 3. Wire .claude/settings.local.json ─────────────────────────────────
    let claude_dir = repo_root.join(".claude");
    fs::create_dir_all(&claude_dir).ok();

    let settings_file = claude_dir.join("settings.local.json");
    let plugin_root_str = plugin_root.to_string_lossy();

    // Preserve existing permissions
    let permissions: serde_json::Value = if settings_file.exists() {
        let content = fs::read_to_string(&settings_file).unwrap_or_default();
        serde_json::from_str::<serde_json::Value>(&content)
            .ok()
            .and_then(|v| v.get("permissions").cloned())
            .unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let settings = serde_json::json!({
        "permissions": permissions,
        "hooks": {
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": format!("{}/hooks/run-hook.cmd session-start", plugin_root_str)
                }]
            }],
            "PreCompact": [{
                "matcher": "",
                "hooks": [{
                    "type": "command",
                    "command": format!("{}/hooks/run-hook.cmd pre-compact", plugin_root_str)
                }]
            }],
            "UserPromptSubmit": [{
                "matcher": "",
                "hooks": [{
                    "type": "command",
                    "command": format!("{}/hooks/run-hook.cmd user-prompt", plugin_root_str)
                }]
            }]
        }
    });

    fs::write(&settings_file, serde_json::to_string_pretty(&settings).unwrap()).ok();
    println!("[ok]      .claude/settings.local.json");

    // ── 4. Gate check ───────────────────────────────────────────────────────
    println!("\n=== gate check ===");
    let mut pass = true;

    if mindsets_dst.exists() {
        let count = fs::read_dir(&mindsets_dst)
            .map(|rd| rd.filter(|e| e.as_ref().map(|e| e.path().extension().map_or(false, |x| x == "md")).unwrap_or(false)).count())
            .unwrap_or(0);
        println!("[ok]   mindsets/ ({} mindsets)", count);
    } else {
        println!("[FAIL] mindsets/ — not found");
        pass = false;
    }

    if rules_file.exists() {
        println!("[ok]   rules.md");
    }

    if personas_dir.exists() {
        let count = fs::read_dir(&personas_dir)
            .map(|rd| rd.filter(|e| e.as_ref().map(|e| e.path().extension().map_or(false, |x| x == "md")).unwrap_or(false)).count())
            .unwrap_or(0);
        println!("[ok]   personas/ ({} personas)", count);
    }

    if repo_root.join("CLAUDE.md").exists() {
        println!("[ok]   CLAUDE.md");
    } else {
        println!("[info] CLAUDE.md — not found (recommended: add gate check reference)");
    }

    if std::process::Command::new("senseid").arg("--version").output().is_ok() {
        println!("[ok]   senseid on PATH");
    } else {
        println!("[warn] senseid not on PATH (run: sensei install)");
    }

    println!();
    if pass {
        println!("=== init complete ===");
        println!("Start a new Claude Code session to activate sensei.");
    } else {
        println!("=== init incomplete — fix FAIL items above ===");
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
