use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;
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
    Uninstall {
        /// Scope: user (global), project (current repo), or all (default)
        #[arg(long, default_value = "all")]
        scope: String,
    },

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
        Commands::Init { scope, acp, recommended, plugin_dir } => {
            init(scope.as_deref(), acp.as_deref(), recommended, plugin_dir.as_deref());
        }
        Commands::Uninstall { scope } => uninstall(&scope),
        Commands::Start { port } => daemon_cmd("start", Some(port)),
        Commands::Stop => daemon_cmd("stop", None),
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

fn ensure_daemon() {
    if daemon_available() { return; }

    eprintln!("Daemon not running — starting...");
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

    eprintln!("Could not start daemon. Run: brew services start sensei");
    std::process::exit(1);
}

/// Prompt user with [Y/n] — returns true if accepted.
fn confirm(prompt: &str, auto_yes: bool) -> bool {
    if auto_yes { return true; }
    print!("{} [Y/n] ", prompt);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let trimmed = input.trim().to_lowercase();
    trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
}

/// Resolve the marketplace directory.
fn find_marketplace(override_path: Option<&str>) -> PathBuf {
    if let Some(pd) = override_path {
        return PathBuf::from(pd);
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let local = cwd.join("marketplace");
    if local.join("mindsets").exists() { return local; }

    let global = home().join(".sensei/marketplace");
    if global.join("mindsets").exists() { return global; }

    let brew = PathBuf::from("/opt/homebrew/share/sensei/marketplace");
    if brew.join("mindsets").exists() { return brew; }

    let brew_intel = PathBuf::from("/usr/local/share/sensei/marketplace");
    if brew_intel.join("mindsets").exists() { return brew_intel; }

    eprintln!("Cannot find marketplace directory. Use --plugin-dir or run: brew install mizukisu/tap/sensei");
    std::process::exit(1);
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
        fs::write(&projects_file, serde_json::to_string_pretty(&projects).unwrap()).ok();
    }
}


/// Load all registered project paths.
fn list_registered_projects() -> Vec<String> {
    let projects_file = home().join(".sensei/projects.json");
    projects_file
        .exists()
        .then(|| fs::read_to_string(&projects_file).ok())
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Copy .md files from src dir to dst dir, returns count.
fn copy_md_files(src: &std::path::Path, dst: &std::path::Path) -> u32 {
    fs::create_dir_all(dst).ok();
    let mut count = 0u32;
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "md") {
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

fn init_user_scope(acp: Option<&str>, recommended: bool, marketplace: &std::path::Path) {
    println!("[user scope] Global setup — MCP, daemon, global commands\n");

    // 1. Daemon
    ensure_daemon();
    println!("  ✓ daemon running");

    // 2. Register MCP with ACP(s)
    let acps: Vec<String> = if let Some(a) = acp {
        vec![a.to_string()]
    } else {
        // Auto-detect via daemon
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

    for acp_id in &acps {
        match client()
            .post(format!("{}/api/acp/configure", DAEMON_URL))
            .json(&serde_json::json!({"acps": [acp_id]}))
            .send()
        {
            Ok(r) if r.status().is_success() => println!("  ✓ {} — MCP registered", acp_id),
            _ => eprintln!("  ✗ {} — configure failed", acp_id),
        }
    }

    // 3. Install global commands
    let global_commands = list_catalog_items(marketplace, "command", "global");
    if !global_commands.is_empty() {
        println!("\n  Global commands:");
        for item in &global_commands {
            println!("    • {} — {}", item.0, item.1);
        }
        if confirm("\n  Install these?", recommended) {
            let dest = home().join(".claude/commands");
            let cache = home().join(".sensei/cache/marketplace");
            let count = install_items_from_marketplace(marketplace, &cache, &global_commands, &dest);
            println!("  ✓ {} global commands installed", count);
        }
    }

    mark_user_scope_configured();
}

// ── Project scope ───────────────────────────────────────────────────────────

fn init_project_scope(recommended: bool, marketplace: &std::path::Path) {
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
        let project_name = repo_root.file_name().and_then(|n| n.to_str()).unwrap_or("project");
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

    // 2. Project commands
    let project_commands = list_catalog_items(marketplace, "command", "project");
    if !project_commands.is_empty() {
        println!("\n  Project commands ({} available):", project_commands.len());
        // Show summary, not full list
        let names: Vec<&str> = project_commands.iter().map(|c| c.0.as_str()).collect();
        for chunk in names.chunks(7) {
            println!("    {}", chunk.join(", "));
        }
        if confirm("\n  Install recommended set?", recommended) {
            let dest = repo_root.join(".claude/commands");
            let cache = home().join(".sensei/cache/marketplace");
            let count = install_items_from_marketplace(marketplace, &cache, &project_commands, &dest);
            println!("  ✓ {} project commands installed", count);
        }
    }

    // 3. Project skills
    let project_skills = list_catalog_items(marketplace, "skill", "project");
    if !project_skills.is_empty() {
        println!("\n  Project skills ({} available):", project_skills.len());
        for item in &project_skills {
            println!("    • {} — {}", item.0, item.1);
        }
        if confirm("\n  Install recommended set?", recommended) {
            let dest = repo_root.join(".claude/skills");
            let cache = home().join(".sensei/cache/marketplace");
            let count = install_items_from_marketplace(marketplace, &cache, &project_skills, &dest);
            println!("  ✓ {} project skills installed", count);
        }
    }

    // 4. Agents
    let agents_src = marketplace.join("agents");
    let agents_dst = repo_root.join(".claude/agents");
    if agents_src.exists() {
        let agent_count = count_md_files(&agents_src);
        if agent_count > 0 {
            println!("\n  Agents ({} available):", agent_count);
            if let Ok(entries) = fs::read_dir(&agents_src) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "md") {
                        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
                        println!("    • {}", name);
                    }
                }
            }
            if confirm("\n  Install agents?", recommended) {
                let count = copy_md_files(&agents_src, &agents_dst);
                println!("  ✓ {} agents installed", count);
            }
        }
    }

    // 5. .mcp.json — upsert sensei entry (for non-plugin ACPs)
    let mcp_file = repo_root.join(".mcp.json");
    let mut mcp_config: serde_json::Value = mcp_file
        .exists()
        .then(|| fs::read_to_string(&mcp_file).ok())
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({"mcpServers": {}}));

    mcp_config
        .as_object_mut()
        .and_then(|o| o.entry("mcpServers").or_insert(serde_json::json!({})).as_object_mut())
        .map(|servers| servers.insert("sensei".into(), serde_json::json!({"command": "sensei-mcp"})));

    fs::write(&mcp_file, serde_json::to_string_pretty(&mcp_config).unwrap()).ok();
    println!("\n  [ok] .mcp.json");

    // 6. Gate check
    println!("\n  --- gate check ---");
    if which_exists("senseid") { println!("  ✓ senseid on PATH"); }
    if which_exists("sensei-mcp") { println!("  ✓ sensei-mcp on PATH"); }
    println!("  ✓ mindsets/ ({} files)", count_md_files(&mindsets_dst));
    if rules_file.exists() { println!("  ✓ rules.md"); }
    if agents_dst.exists() { println!("  ✓ agents/ ({} files)", count_md_files(&agents_dst)); }
    if repo_root.join("CLAUDE.md").exists() { println!("  ✓ CLAUDE.md"); }
}

// ── Catalog helpers ─────────────────────────────────────────────────────────

/// List items from the local marketplace catalog.json matching kind and scope.
/// Returns Vec<(name, description, path)>.
fn list_catalog_items(marketplace: &std::path::Path, kind: &str, scope: &str) -> Vec<(String, String, String)> {
    let catalog_path = marketplace.join("catalog.json");
    let content = match fs::read_to_string(&catalog_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let catalog: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    catalog["items"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|item| {
            item["kind"].as_str() == Some(kind) && item["scope"].as_str() == Some(scope)
        })
        .map(|item| {
            (
                item["name"].as_str().unwrap_or("").to_string(),
                item["description"].as_str().unwrap_or("").to_string(),
                item["path"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect()
}

/// Install items by reading from the local marketplace directory.
fn install_items_from_marketplace(
    marketplace: &std::path::Path,
    _cache: &std::path::Path,
    items: &[(String, String, String)],
    dest_dir: &std::path::Path,
) -> u32 {
    fs::create_dir_all(dest_dir).ok();
    let mut count = 0u32;
    for (name, _, path) in items {
        let src = marketplace.join(path);
        if src.exists() {
            let content = fs::read_to_string(&src).unwrap_or_default();
            let dest = dest_dir.join(format!("{}.md", name));
            fs::write(&dest, &content).ok();
            count += 1;
        } else {
            eprintln!("  ✗ {} — not found at {}", name, src.display());
        }
    }
    count
}

fn count_md_files(dir: &std::path::Path) -> usize {
    fs::read_dir(dir)
        .map(|rd| {
            rd.filter(|e| {
                e.as_ref()
                    .map(|e| e.path().extension().map_or(false, |x| x == "md"))
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
}

// ── Uninstall ───────────────────────────────────────────────────────────────

fn uninstall(scope: &str) {
    println!("=== sensei uninstall (scope: {}) ===\n", scope);

    let do_project = scope == "all" || scope == "project";

    // Collect project paths for project-scope cleanup
    let projects: Vec<String> = if do_project {
        let registered = list_registered_projects();
        let cwd = std::env::current_dir().unwrap_or_default().to_string_lossy().to_string();
        let mut paths = registered;
        if !paths.contains(&cwd) {
            paths.push(cwd);
        }
        // Filter to projects that actually have sensei artifacts
        let active: Vec<String> = paths
            .into_iter()
            .filter(|p| {
                let root = std::path::Path::new(p);
                root.join(".sensei").exists()
                    || root.join(".claude/commands").exists()
                    || root.join(".claude/agents").exists()
            })
            .collect();

        if !active.is_empty() {
            println!("[project scope] {} project(s) found:", active.len());
            for p in &active {
                println!("  {}", p);
            }
            if !confirm("\nClean all listed projects?", false) {
                println!("  Skipped project cleanup.\n");
                vec![] // user declined
            } else {
                active
            }
        } else {
            println!("[project scope] No sensei-managed projects found.\n");
            vec![]
        }
    } else {
        vec![]
    };

    // Delegate everything to the daemon
    ensure_daemon();

    match client()
        .post(format!("{}/api/uninstall", DAEMON_URL))
        .json(&serde_json::json!({"scope": scope, "projects": projects}))
        .send()
    {
        Ok(r) if r.status().is_success() => {
            let result: serde_json::Value = r.json().unwrap_or_default();

            println!("\n--- cleaned ---");

            // ACP removals
            for id in result["acps_removed"].as_array().unwrap_or(&vec![]) {
                println!("  ✓ MCP removed from {}", id.as_str().unwrap_or("?"));
            }

            // Counts
            let skills = result["skills_removed"].as_u64().unwrap_or(0);
            let cmds = result["commands_removed"].as_u64().unwrap_or(0);
            if skills > 0 { println!("  ✓ {} skills removed", skills); }
            if cmds > 0 { println!("  ✓ {} commands removed", cmds); }
            if result["hooks_removed"].as_bool() == Some(true) { println!("  ✓ Hooks removed"); }
            if result["cache_cleared"].as_bool() == Some(true) { println!("  ✓ Cache cleared"); }
            if result["plugin_removed"].as_bool() == Some(true) { println!("  ✓ Legacy plugin removed"); }

            // Projects
            for p in result["projects_cleaned"].as_array().unwrap_or(&vec![]) {
                println!("  ✓ {}", p.as_str().unwrap_or("?"));
            }

            // Errors
            for e in result["errors"].as_array().unwrap_or(&vec![]) {
                eprintln!("  ✗ {}", e.as_str().unwrap_or("?"));
            }
        }
        Ok(r) => eprintln!("Uninstall failed: HTTP {}", r.status()),
        Err(e) => eprintln!("Uninstall failed: {}", e),
    }

    // Not managed by sensei
    println!("\nNot managed by sensei uninstall:");
    println!("  - Binaries (managed by Homebrew)");
    println!("  - Daemon process (managed by brew services)");
    println!("  - Desktop app (managed by Homebrew Cask)");

    println!("\nTo fully remove sensei:");
    println!("  brew services stop sensei");
    println!("  brew uninstall sensei");
    println!("  brew uninstall --cask sensei-app  # if desktop app installed");

    println!("\nSensei uninstalled.");
}

// ── Daemon / Scan / AddLib ──────────────────────────────────────────────────

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
            eprintln!("Failed to run senseid: {}. Install: brew install mizukisu/tap/sensei", e);
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
