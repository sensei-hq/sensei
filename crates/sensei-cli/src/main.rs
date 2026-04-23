use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

const DAEMON_URL: &str = "http://127.0.0.1:7744";

#[derive(Parser)]
#[command(name = "sensei", about = "Sensei — AI coding companion", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize sensei — sets up MCP, commands, skills, agents, mindsets
    Init {
        /// Scope: user (global ~/.claude/) or project (repo .claude/)
        #[arg(long)]
        scope: Option<String>,

        /// Target ACP (default: auto-detect)
        #[arg(long)]
        acp: Option<String>,

        /// Skip interactive prompts — install recommended set
        #[arg(long)]
        recommended: bool,

        /// Path to the plugin/marketplace directory (default: auto-detect)
        #[arg(long)]
        plugin_dir: Option<String>,
    },

    /// Remove sensei configuration
    Remove {
        /// What to remove: "acp" or "all"
        target: String,
        /// For "acp" target: claude, cursor, windsurf, zed, kiro, opencode, vscode, desktop, all
        name: Option<String>,
        /// Also remove data (sessions, indexes, project artifacts)
        #[arg(long)]
        purge: bool,
    },

    /// Start the sensei daemon
    Start {
        #[arg(long, default_value = "7744")]
        port: u16,
    },

    /// Stop the sensei daemon
    Stop,

    /// Restart the sensei daemon
    Restart {
        #[arg(long, default_value = "7744")]
        port: u16,
    },

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
        Commands::Init {
            scope,
            acp,
            recommended,
            plugin_dir,
        } => {
            init(
                scope.as_deref(),
                acp.as_deref(),
                recommended,
                plugin_dir.as_deref(),
            );
        }
        Commands::Remove {
            target,
            name,
            purge,
        } => remove_cmd(&target, name.as_deref(), purge),
        Commands::Start { port } => daemon_cmd("start", Some(port)),
        Commands::Stop => daemon_cmd("stop", None),
        Commands::Restart { port } => restart_daemon(port),
        Commands::Status => daemon_cmd("status", None),
        Commands::Scan { path } => scan(&path),
        Commands::AddLib { name, url } => add_lib(&name, url.as_deref()),
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
}

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

fn which_exists(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

fn daemon_bin() -> PathBuf {
    if which_exists("senseid") {
        return PathBuf::from("senseid");
    }
    home().join(".claude/plugins/sensei/bin/senseid")
}

const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

fn ensure_daemon() {
    if daemon_available() {
        check_daemon_version(true);
        return;
    }

    eprintln!("Daemon not running — starting...");
    start_daemon();

    eprintln!("Could not start daemon. Run: brew services start sensei");
    std::process::exit(1);
}

fn start_daemon() {
    let bin = daemon_bin();
    let _ = std::process::Command::new(&bin)
        .args(["start", "--port", "7744"])
        .spawn();

    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if daemon_available() {
            eprintln!("Daemon started.");
            return;
        }
    }
}

/// Check if daemon version matches CLI. If mismatched, restart once.
fn check_daemon_version(allow_restart: bool) {
    let daemon_version = get_daemon_version();

    if daemon_version == CLI_VERSION {
        if !allow_restart {
            // This is the post-restart verification — confirm it worked
            eprintln!("  ✓ Daemon version now matches: {}", daemon_version);
        }
        return;
    }

    if daemon_version.is_empty() {
        eprintln!("  Warning: daemon did not report version.");
        return;
    }

    eprintln!(
        "  Version mismatch — CLI: {}, daemon: {}",
        CLI_VERSION, daemon_version
    );

    if allow_restart {
        eprintln!("  Restarting daemon...");
        let bin = daemon_bin();
        let _ = std::process::Command::new(&bin).arg("stop").status();
        std::thread::sleep(std::time::Duration::from_millis(500));

        start_daemon();
        check_daemon_version(false);
    } else {
        eprintln!("  ✗ Daemon still out of sync after restart.");
        eprintln!("  Update: brew upgrade sensei && brew services restart sensei");
        std::process::exit(1);
    }
}

fn get_daemon_version() -> String {
    client()
        .get(format!("{}/health", DAEMON_URL))
        .send()
        .ok()
        .and_then(|r| r.json::<serde_json::Value>().ok())
        .and_then(|v| v["version"].as_str().map(String::from))
        .unwrap_or_default()
}

/// Prompt user with [Y/n] — returns true if accepted.
fn confirm(prompt: &str, auto_yes: bool) -> bool {
    if auto_yes {
        return true;
    }
    print!("{} [Y/n] ", prompt);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let trimmed = input.trim().to_lowercase();
    trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
}

/// Check if user-scope init has been done (MCP registered for at least one ACP).
fn is_user_scope_configured() -> bool {
    let config_file = home().join(".sensei/config.json");
    config_file
        .exists()
        .then(|| fs::read_to_string(&config_file).ok())
        .flatten()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["user_scope_configured"].as_bool())
        .unwrap_or(false)
}

/// Mark user scope as configured.
fn mark_user_scope_configured() {
    let sensei_dir = home().join(".sensei");
    fs::create_dir_all(&sensei_dir).ok();
    let config_file = sensei_dir.join("config.json");
    let mut config: serde_json::Value = config_file
        .exists()
        .then(|| fs::read_to_string(&config_file).ok())
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    config["user_scope_configured"] = serde_json::json!(true);
    fs::write(&config_file, serde_json::to_string_pretty(&config).unwrap()).ok();
}

/// Register a project in ~/.sensei/projects.json so uninstall can find it later.
fn register_project(repo_path: &std::path::Path) {
    let projects_file = home().join(".sensei/projects.json");
    let mut projects: Vec<String> = projects_file
        .exists()
        .then(|| fs::read_to_string(&projects_file).ok())
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let path_str = repo_path.to_string_lossy().to_string();
    if !projects.contains(&path_str) {
        projects.push(path_str);
        fs::create_dir_all(home().join(".sensei")).ok();
        fs::write(
            &projects_file,
            serde_json::to_string_pretty(&projects).unwrap(),
        )
        .ok();
    }
}

/// Copy .md files from src dir to dst dir, returns count.
fn copy_md_files(src: &std::path::Path, dst: &std::path::Path) -> u32 {
    fs::create_dir_all(dst).ok();
    let mut count = 0u32;
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                fs::copy(&path, dst.join(entry.file_name())).ok();
                count += 1;
            }
        }
    }
    count
}

// ── Init ────────────────────────────────────────────────────────────────────

fn init(scope: Option<&str>, acp: Option<&str>, recommended: bool, plugin_dir: Option<&str>) {
    println!("=== sensei init ===\n");

    // Verify binaries
    if !which_exists("senseid") || !which_exists("sensei-mcp") {
        eprintln!("Missing binaries. Install: brew install mizukisu/tap/sensei");
        std::process::exit(1);
    }

    let marketplace = find_marketplace(plugin_dir);

    match scope {
        Some("user") => {
            init_user_scope(acp, recommended, &marketplace);
        }
        Some("project") => {
            init_project_scope(recommended, &marketplace);
        }
        Some(other) => {
            eprintln!("Unknown scope: {}. Use 'user' or 'project'.", other);
            std::process::exit(1);
        }
        None => {
            // Auto-detect: if user scope not configured, do both
            if !is_user_scope_configured() {
                println!("First-time setup detected — configuring user + project scope.\n");
                init_user_scope(acp, recommended, &marketplace);
                println!();
            }
            init_project_scope(recommended, &marketplace);
        }
    }

    println!("\n=== init complete ===");
}

// ── User scope ──────────────────────────────────────────────────────────────

fn init_user_scope(acp: Option<&str>, _recommended: bool, _marketplace: &std::path::Path) {
    println!("[user scope] Global setup — daemon, ACP registration\n");

    // 1. Daemon
    ensure_daemon();
    println!("  ✓ daemon running");

    // 2. Register with ACP(s) — handles plugin install (commands, skills, hooks, MCP)
    let acps: Vec<String> = if let Some(a) = acp {
        vec![a.to_string()]
    } else {
        let detected: Vec<serde_json::Value> = client()
            .get(format!("{}/api/acp/detect", DAEMON_URL))
            .send()
            .ok()
            .and_then(|r| r.json().ok())
            .unwrap_or_default();

        let installed: Vec<String> = detected
            .iter()
            .filter(|a| a["installed"].as_bool() == Some(true))
            .filter_map(|a| a["id"].as_str().map(String::from))
            .collect();

        if installed.is_empty() {
            eprintln!("  No AI coding platforms detected.");
        } else {
            println!("  Detected: {}", installed.join(", "));
        }
        installed
    };

    let mut any_success = false;
    let mut all_errors: Vec<String> = Vec::new();

    for acp_id in &acps {
        // Don't pass local marketplace_path — the daemon uses the GitHub repo
        // by default (SENSEI_MARKETPLACE_REPO). Passing a local dev path causes
        // Claude Code to register a directory source that breaks on other machines.
        match client()
            .post(format!("{}/api/acp/configure", DAEMON_URL))
            .json(&serde_json::json!({
                "acps": [acp_id],
            }))
            .send()
        {
            Ok(r) if r.status().is_success() => {
                let body: serde_json::Value = r.json().unwrap_or_default();
                let errors: Vec<String> = body["errors"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|e| e.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let plugin_ok = body["plugin_installed"].as_bool() == Some(true);

                if errors.is_empty() {
                    if plugin_ok {
                        println!(
                            "  ✓ {} — plugin installed (commands, skills, hooks, MCP)",
                            acp_id
                        );
                    } else {
                        println!("  ✓ {} — MCP registered", acp_id);
                    }
                    any_success = true;
                } else {
                    // Partial success — configured but with warnings
                    if plugin_ok {
                        println!("  ~ {} — plugin installed with warnings:", acp_id);
                    } else {
                        println!("  ~ {} — MCP registered with warnings:", acp_id);
                    }
                    for msg in &errors {
                        eprintln!("    ⚠ {}", msg);
                    }
                    all_errors.extend(errors);
                    any_success = true;
                }
            }
            Ok(r) => {
                let status = r.status();
                let body: String = r.text().unwrap_or_default();
                eprintln!(
                    "  ✗ {} — configure failed (HTTP {}): {}",
                    acp_id, status, body
                );
                all_errors.push(format!("{}: HTTP {}", acp_id, status));
            }
            Err(e) => {
                eprintln!("  ✗ {} — configure failed: {}", acp_id, e);
                all_errors.push(format!("{}: {}", acp_id, e));
            }
        }
    }

    if !all_errors.is_empty() {
        eprintln!(
            "\n  {} error(s) during user scope init. Run with RUST_LOG=debug for details.",
            all_errors.len()
        );
    }

    if any_success {
        mark_user_scope_configured();
    } else if !acps.is_empty() {
        eprintln!("  ✗ No ACPs configured successfully. User scope NOT marked as configured.");
    }
}

// ── Project scope ───────────────────────────────────────────────────────────

fn init_project_scope(_recommended: bool, marketplace: &std::path::Path) {
    let repo_root = std::env::current_dir().expect("Cannot determine current directory");
    println!("[project scope] {}\n", repo_root.display());

    // 1. .sensei/ directory — mindsets, personas, rules
    let sensei_dir = repo_root.join(".sensei");
    fs::create_dir_all(&sensei_dir).ok();

    // Register this project — so uninstall can find it across all repos
    register_project(&repo_root);

    // Rules
    let rules_file = sensei_dir.join("rules.md");
    if !rules_file.exists() {
        let project_name = repo_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project");
        let today = format_date();
        fs::write(&rules_file, format!(
            "---\nname: Project Rules — {}\nupdated: {}\nmindsets: .sensei/mindsets/\npersonas: .sensei/personas/\n---\n\n# Rules\n\n## Patterns\n\n<!-- Add project patterns here -->\n\n## Quality\n\n- **Zero errors** — test suite must pass before and after every change\n\n## Process\n\n- **Design before code** — analyst mindset first\n- **One issue at a time** — complete, verify, close, then next\n",
            project_name, today,
        )).ok();
        println!("  [created] .sensei/rules.md");
    } else {
        println!("  [exists]  .sensei/rules.md");
    }

    // Personas
    let personas_dir = sensei_dir.join("personas");
    if !personas_dir.exists() {
        fs::create_dir_all(&personas_dir).ok();
        println!("  [created] .sensei/personas/");
    } else {
        let count = count_md_files(&personas_dir);
        println!("  [exists]  .sensei/personas/ ({} personas)", count);
    }

    // Mindsets
    let mindsets_src = marketplace.join("mindsets");
    let mindsets_dst = sensei_dir.join("mindsets");
    if mindsets_src.exists() {
        let count = copy_md_files(&mindsets_src, &mindsets_dst);
        println!("  [copied]  .sensei/mindsets/ ({} mindsets)", count);
    }

    // 2. .mcp.json — upsert sensei entry (for non-plugin ACPs)
    let mcp_file = repo_root.join(".mcp.json");
    let mut mcp_config: serde_json::Value = mcp_file
        .exists()
        .then(|| fs::read_to_string(&mcp_file).ok())
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({"mcpServers": {}}));

    mcp_config
        .as_object_mut()
        .and_then(|o| {
            o.entry("mcpServers")
                .or_insert(serde_json::json!({}))
                .as_object_mut()
        })
        .map(|servers| {
            servers.insert(
                "sensei".into(),
                serde_json::json!({"command": "sensei-mcp"}),
            )
        });

    fs::write(
        &mcp_file,
        serde_json::to_string_pretty(&mcp_config).unwrap(),
    )
    .ok();
    println!("\n  [ok] .mcp.json");

    // 3. Clean up stale per-project hooks from .claude/settings.local.json
    //    Global plugin hooks handle all hook events — per-project hooks are redundant.
    let settings_local = repo_root.join(".claude/settings.local.json");
    if settings_local.exists()
        && let Ok(content) = fs::read_to_string(&settings_local)
        && let Ok(mut settings) = serde_json::from_str::<serde_json::Value>(&content)
        && settings.get("hooks").is_some()
    {
        settings.as_object_mut().unwrap().remove("hooks");
        if let Ok(json) = serde_json::to_string_pretty(&settings) {
            fs::write(&settings_local, json).ok();
            println!(
                "\n  [cleaned] .claude/settings.local.json — removed stale hooks (handled by global plugin)"
            );
        }
    }

    // 4. Gate check
    println!("\n  --- gate check ---");
    if which_exists("senseid") {
        println!("  ✓ senseid on PATH");
    }
    if which_exists("sensei-mcp") {
        println!("  ✓ sensei-mcp on PATH");
    }
    println!("  ✓ mindsets/ ({} files)", count_md_files(&mindsets_dst));
    if rules_file.exists() {
        println!("  ✓ rules.md");
    }
    if repo_root.join("CLAUDE.md").exists() {
        println!("  ✓ CLAUDE.md");
    }
}

fn count_md_files(dir: &std::path::Path) -> usize {
    fs::read_dir(dir)
        .map(|rd| {
            rd.filter(|e| {
                e.as_ref()
                    .map(|e| e.path().extension().is_some_and(|x| x == "md"))
                    .unwrap_or(false)
            })
            .count()
        })
        .unwrap_or(0)
}

fn format_date() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = now / 86400;
    let mut y = 1970i32;
    let mut remaining = days as i32;
    loop {
        let year_days = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if remaining < year_days {
            break;
        }
        remaining -= year_days;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mdays = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0usize;
    while m < 12 && remaining >= mdays[m] {
        remaining -= mdays[m];
        m += 1;
    }
    format!("{}-{:02}-{:02}", y, m + 1, remaining + 1)
}

// ── Remove ──────────────────────────────────────────────────────────────────

/// Map friendly CLI name to ACP ID.
fn acp_name_to_id(name: &str) -> Option<String> {
    match name {
        "claude" => Some("claude-code".into()),
        "desktop" => Some("claude-desktop".into()),
        "cursor" => Some("cursor".into()),
        "windsurf" => Some("windsurf".into()),
        "zed" => Some("zed".into()),
        "kiro" => Some("kiro".into()),
        "opencode" => Some("opencode".into()),
        "vscode" => Some("vscode".into()),
        "all" => None, // None means all
        _ => {
            eprintln!(
                "Unknown ACP: {}. Available: claude, desktop, cursor, windsurf, zed, kiro, opencode, vscode, all",
                name
            );
            std::process::exit(1);
        }
    }
}

fn remove_cmd(target: &str, name: Option<&str>, purge: bool) {
    match target {
        "acp" => remove_acp(name.unwrap_or("all")),
        "all" => remove_all(purge),
        _ => {
            eprintln!("Unknown target: {}. Usage:", target);
            eprintln!("  sensei remove acp <claude|cursor|...>");
            eprintln!("  sensei remove all [--purge]");
            std::process::exit(1);
        }
    }
}

fn remove_acp(name: &str) {
    println!("=== sensei remove acp {} ===\n", name);

    let acps: Vec<String> = match acp_name_to_id(name) {
        Some(id) => vec![id],
        None => vec![], // empty = all
    };

    ensure_daemon();

    match client()
        .post(format!("{}/api/acp/remove", DAEMON_URL))
        .json(&serde_json::json!({"acps": acps}))
        .send()
    {
        Ok(r) if r.status().is_success() => {
            let result: serde_json::Value = r.json().unwrap_or_default();
            let removed = result["acps_removed"]
                .as_array()
                .cloned()
                .unwrap_or_default();

            if removed.is_empty() {
                println!("  No ACPs to remove.");
            } else {
                for id in &removed {
                    println!("  ✓ Removed {}", id.as_str().unwrap_or("?"));
                }
            }

            for e in result["errors"].as_array().unwrap_or(&vec![]) {
                eprintln!("  ✗ {}", e.as_str().unwrap_or("?"));
            }
        }
        Ok(r) => eprintln!("Remove failed: HTTP {}", r.status()),
        Err(e) => eprintln!("Remove failed: {}", e),
    }

    println!("\nRe-add with: sensei init --acp {}", name);
}

fn remove_all(purge: bool) {
    if purge {
        println!("=== sensei remove all --purge ===\n");
        println!(
            "This will remove ALL sensei data including sessions, indexes, and project artifacts."
        );
        if !confirm("Continue?", false) {
            println!("Cancelled.");
            return;
        }
    } else {
        println!("=== sensei remove all ===\n");
        println!("Removing ACPs and plugin artifacts. Data (sessions, indexes) will be preserved.");
    }

    ensure_daemon();

    match client()
        .post(format!("{}/api/remove", DAEMON_URL))
        .json(&serde_json::json!({"purge": purge}))
        .send()
    {
        Ok(r) if r.status().is_success() => {
            let result: serde_json::Value = r.json().unwrap_or_default();

            println!("\n--- removed ---");

            for id in result["acps_removed"].as_array().unwrap_or(&vec![]) {
                println!("  ✓ ACP removed: {}", id.as_str().unwrap_or("?"));
            }
            let skills = result["skills_removed"].as_u64().unwrap_or(0);
            let cmds = result["commands_removed"].as_u64().unwrap_or(0);
            let agents = result["agents_removed"].as_u64().unwrap_or(0);
            if skills > 0 {
                println!("  ✓ {} skills removed", skills);
            }
            if cmds > 0 {
                println!("  ✓ {} commands removed", cmds);
            }
            if agents > 0 {
                println!("  ✓ {} agents removed", agents);
            }
            if result["hooks_removed"].as_bool() == Some(true) {
                println!("  ✓ Hooks removed");
            }
            if result["plugin_removed"].as_bool() == Some(true) {
                println!("  ✓ Plugin removed");
            }
            if result["cache_cleared"].as_bool() == Some(true) {
                println!("  ✓ Cache cleared");
            }

            for p in result["projects_cleaned"].as_array().unwrap_or(&vec![]) {
                println!("  ✓ Project cleaned: {}", p.as_str().unwrap_or("?"));
            }

            for e in result["errors"].as_array().unwrap_or(&vec![]) {
                eprintln!("  ✗ {}", e.as_str().unwrap_or("?"));
            }
        }
        Ok(r) => eprintln!("Remove failed: HTTP {}", r.status()),
        Err(e) => eprintln!("Remove failed: {}", e),
    }

    // Purge: stop daemon and delete data directory
    if purge {
        println!("\nStopping daemon...");
        let bin = daemon_bin();
        let _ = std::process::Command::new(&bin).arg("stop").status();

        let sensei_dir = home().join(".sensei");
        if sensei_dir.exists() {
            fs::remove_dir_all(&sensei_dir).ok();
            println!("  ✓ Data directory removed (~/.sensei/)");
        }

        println!(
            "\nSensei fully removed. To reinstall: brew install mizukisu/tap/sensei && sensei init"
        );
    } else {
        println!("\nData preserved. To reinstall: sensei init");
    }
}

// ── Daemon / Scan / AddLib ──────────────────────────────────────────────────

fn restart_daemon(port: u16) {
    let bin = daemon_bin();
    let _ = std::process::Command::new(&bin).arg("stop").status();
    std::thread::sleep(std::time::Duration::from_millis(500));
    match std::process::Command::new(&bin)
        .args(["start", "--port", &port.to_string()])
        .status()
    {
        Ok(s) => std::process::exit(s.code().unwrap_or(0)),
        Err(e) => {
            eprintln!(
                "Failed to run senseid: {}. Install: brew install mizukisu/tap/sensei",
                e
            );
            std::process::exit(1);
        }
    }
}

fn daemon_cmd(cmd: &str, port: Option<u16>) {
    let bin = daemon_bin();
    let mut args = vec![cmd.to_string()];
    if let Some(p) = port {
        args.push("--port".into());
        args.push(p.to_string());
    }
    match std::process::Command::new(&bin).args(&args).status() {
        Ok(s) => std::process::exit(s.code().unwrap_or(0)),
        Err(e) => {
            eprintln!(
                "Failed to run senseid: {}. Install: brew install mizukisu/tap/sensei",
                e
            );
            std::process::exit(1);
        }
    }
}

fn scan(path: &str) {
    ensure_daemon();
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
    ensure_daemon();
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
