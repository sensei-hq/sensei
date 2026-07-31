use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use sensei_bootstrap::{
    SenseiConfig, SenseiLocalConfig,
    SENSEI_BIN, SENSEID_BIN, SENSEI_MCP_BIN, MCP_REGISTRY_KEY,
};

mod doctor;
mod managed;
mod scaffold;

fn cfg() -> &'static SenseiConfig {
    sensei_bootstrap::config()
}

fn daemon_url() -> String {
    sensei_bootstrap::daemon_url()
}

#[derive(Parser)]
#[command(name = SENSEI_BIN, about = "Sensei — AI coding companion", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Upgrade sensei's integration for ALL detected assistants — a flag alias
    /// for the `upgrade` subcommand, so `sensei --upgrade` refreshes every ACP's
    /// plugin/MCP in one shot after a sensei upgrade.
    #[arg(long)]
    upgrade: bool,
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
    },

    /// Remove sensei configuration
    Remove {
        /// What to remove: "acp" or "all"
        target: String,
        /// For "acp" target: ACP name or ID (e.g. claude, cursor, windsurf) or "all"
        name: Option<String>,
        /// Also remove data (sessions, indexes, project artifacts)
        #[arg(long)]
        purge: bool,
    },

    /// Start the sensei daemon
    Start {
        /// Port (default: 7744 prod, 7745 dev — inferred from binary name)
        #[arg(long)]
        port: Option<u16>,
    },

    /// Stop the sensei daemon
    Stop,

    /// Restart the sensei daemon
    Restart {
        /// Port (default: 7744 prod, 7745 dev — inferred from binary name)
        #[arg(long)]
        port: Option<u16>,
    },

    /// Show daemon status
    Status,

    /// Scan a folder and index all repos
    Scan {
        /// Folder to scan
        path: String,
    },

    /// Scaffold the canonical Sensei doc structure into a project
    Scaffold {
        /// What to scaffold (default: the project-level doc structure)
        #[command(subcommand)]
        what: Option<ScaffoldTarget>,
        /// Target directory (default: current directory)
        #[arg(long, global = true)]
        path: Option<String>,
    },

    /// Inspect the code index (read-only diagnostics)
    Index {
        #[command(subcommand)]
        cmd: IndexCommands,
    },

    /// Manage indexed folders (e.g. repair a rename the auto-detect missed)
    Folder {
        #[command(subcommand)]
        cmd: FolderCommands,
    },

    /// Manage local model provisioning — pull a chat model from Hugging Face on
    /// demand and check what's been pulled.
    Models {
        #[command(subcommand)]
        cmd: ModelsCommands,
    },

    /// Add an external library's documentation
    AddLib {
        /// Library name
        name: String,
        /// URL to llms.txt (auto-discovered if omitted)
        #[arg(long)]
        url: Option<String>,
    },

    /// Diagnose bootstrap state: check + auto-fix dependencies with full
    /// step-by-step trace output. Exits 0 if everything is healthy.
    Doctor {
        /// Attempt to auto-resolve failing adapters (reinstall plugin/marketplace).
        #[arg(long)]
        fix: bool,
    },

    /// Refresh assistant integrations after a sensei upgrade — advances each
    /// assistant's sensei plugin/MCP so a new sensei binary doesn't leave a
    /// stale plugin behind. Claude Code runs `claude plugin update sensei`;
    /// file-based MCP assistants re-read their config on their own restart.
    Upgrade {
        /// Target ACP (default: all detected). e.g. claude, cursor, zed
        #[arg(long)]
        acp: Option<String>,
    },
}

/// `sensei scaffold <what>` targets. Absent = the project-level doc structure.
#[derive(Subcommand)]
enum ScaffoldTarget {
    /// Scaffold a per-feature dossier under docs/features/<name>/
    Feature {
        /// Feature name (a single path segment → docs/features/<name>/)
        name: String,
    },
    /// Scaffold the baseline capability contract (docs/baseline.md)
    Baseline {
        /// Project kind — selects the adapter column
        #[arg(long, value_enum, default_value_t = scaffold::BaselineKind::Code)]
        kind: scaffold::BaselineKind,
    },
}

/// `sensei index <cmd>` — index diagnostics. Read-only; the daemon owns repair.
#[derive(Subcommand)]
enum IndexCommands {
    /// Report index integrity drift (orphan nodes, ghost folders, mis-scoped
    /// roots, duplicate-name projects) without repairing — read-only.
    Doctor,
}

/// `sensei folder <cmd>` — indexed-folder maintenance.
#[derive(Subcommand)]
enum FolderCommands {
    /// Re-point a renamed/moved repo's history onto its new path — the manual
    /// repair for a rename the scan's git-remote auto-detect couldn't catch.
    /// Aliases the old path forward and re-attaches sessions captured under it.
    Remap {
        /// The old (vanished) absolute path.
        old: String,
        /// The new absolute path — must already be indexed (scan it first).
        new: String,
    },
}

/// `sensei models <cmd>` — on-demand local model provisioning via the daemon.
#[derive(Subcommand)]
enum ModelsCommands {
    /// Pull a model (from Hugging Face) and coldboot it behind the embedded
    /// router. Non-blocking — returns the initial phase; poll `models status`
    /// for progress.
    Pull {
        /// Model id to provision (e.g. `gemma2:2b`).
        id: String,
    },
    /// Show every model the daemon has (or is) provisioning, with its phase.
    Status,
}

/// Daemon route the `upgrade` subcommand POSTs to. Kept as a const so the code
/// path and its parse/target test share one source of truth.
const UPGRADE_ENDPOINT: &str = "/api/assistants/upgrade";

fn main() -> ExitCode {
    let cli = Cli::parse();

    // `sensei --upgrade` (flag form) — refresh every detected assistant, the
    // same all-ACP fan-out as the `upgrade` subcommand with no --acp.
    if cli.upgrade && cli.command.is_none() {
        upgrade_cmd(None);
        return ExitCode::SUCCESS;
    }

    match cli.command {
        None => {
            // No subcommand and no --upgrade → show help rather than erroring.
            let _ = <Cli as clap::CommandFactory>::command().print_help();
            println!();
        }
        Some(Commands::Init {
            scope,
            acp,
            recommended,
        }) => {
            init(scope.as_deref(), acp.as_deref(), recommended);
        }
        Some(Commands::Remove {
            target,
            name,
            purge,
        }) => remove_cmd(&target, name.as_deref(), purge),
        Some(Commands::Start { port }) => {
            daemon_cmd("start", Some(port.unwrap_or_else(|| cfg().daemon_port)))
        }
        Some(Commands::Stop) => daemon_cmd("stop", None),
        Some(Commands::Restart { port }) => restart_daemon(port.unwrap_or_else(|| cfg().daemon_port)),
        Some(Commands::Status) => daemon_cmd("status", None),
        Some(Commands::Scan { path }) => scan(&path),
        Some(Commands::Scaffold { what, path }) => scaffold_cmd(what, path.as_deref()),
        Some(Commands::Index { cmd }) => match cmd {
            IndexCommands::Doctor => index_doctor(),
        },
        Some(Commands::Folder { cmd }) => match cmd {
            FolderCommands::Remap { old, new } => folder_remap(&old, &new),
        },
        Some(Commands::Models { cmd }) => match cmd {
            ModelsCommands::Pull { id } => models_pull(&id),
            ModelsCommands::Status => models_status(),
        },
        Some(Commands::AddLib { name, url }) => add_lib(&name, url.as_deref()),
        Some(Commands::Doctor { fix }) => return ExitCode::from(doctor::run(fix) as u8),
        Some(Commands::Upgrade { acp }) => upgrade_cmd(acp.as_deref()),
    }
    ExitCode::SUCCESS
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn home() -> PathBuf {
    sensei_bootstrap::home_dir()
}

fn client() -> reqwest::blocking::Client {
    client_with_timeout(30)
}

fn client_with_timeout(secs: u64) -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(secs))
        .build()
        .unwrap()
}

fn daemon_available() -> bool {
    client()
        .get(format!("{}/health", daemon_url()))
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn daemon_bin() -> PathBuf {
    let name = cfg().daemon_binary();
    if let Some(p) = sensei_bootstrap::util::which_binary(name) {
        return PathBuf::from(p);
    }
    PathBuf::from(name)
}

const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

fn ensure_daemon() {
    if daemon_available() {
        check_daemon_version(true);
        return;
    }

    eprintln!("Daemon not running — starting...");
    start_daemon();

    if daemon_available() {
        check_daemon_version(true);
        return;
    }

    eprintln!("Could not start daemon. Run: brew services start sensei");
    std::process::exit(1);
}

fn start_daemon() {
    // Prefer `brew services start sensei` — matches the postgres/ollama
    // startup path AND inherits launchd's keep-alive auto-restart on crash.
    // Direct spawn is the fallback when brew isn't available or the service
    // isn't registered yet.
    let cfg = cfg();
    let service = cfg.brew_service_name();
    let mut started_via_brew = false;
    if let Ok(out) = std::process::Command::new("brew")
        .args(["services", "start", service])
        .output()
        && out.status.success()
    {
        started_via_brew = true;
    }

    if !started_via_brew {
        let bin = daemon_bin();
        let port = cfg.daemon_port;
        match std::process::Command::new(&bin)
            .args(["start", "--port", &port.to_string()])
            .spawn()
        {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to spawn daemon ({}): {}", bin.display(), e);
                eprintln!("Try: brew services start {service}");
                return;
            }
        }
    }

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
        .get(format!("{}/health", daemon_url()))
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
    SenseiLocalConfig::load(&cfg().sensei_dir()).user_scope_configured
}

/// Mark user scope as configured.
fn mark_user_scope_configured() {
    let sensei_dir = cfg().sensei_dir();
    let mut local = SenseiLocalConfig::load(&sensei_dir);
    local.user_scope_configured = true;
    if let Err(e) = local.save(&sensei_dir) {
        eprintln!("Warning: failed to write sensei config: {e}");
    }
}


// ── Init ────────────────────────────────────────────────────────────────────

fn init(scope: Option<&str>, acp: Option<&str>, recommended: bool) {
    println!("=== sensei init ===\n");

    // Verify binaries
    if sensei_bootstrap::util::which_binary(SENSEID_BIN).is_none()
        || sensei_bootstrap::util::which_binary(SENSEI_MCP_BIN).is_none()
    {
        eprintln!("Missing binaries. Install: {}", cfg().brew_install_script());
        std::process::exit(1);
    }

    match scope {
        Some("user") => {
            init_user_scope(acp, recommended);
        }
        Some("project") => {
            init_project_scope(recommended);
        }
        Some(other) => {
            eprintln!("Unknown scope: {}. Use 'user' or 'project'.", other);
            std::process::exit(1);
        }
        None => {
            // Auto-detect: if user scope not configured, do both
            if !is_user_scope_configured() {
                println!("First-time setup detected — configuring user + project scope.\n");
                init_user_scope(acp, recommended);
                println!();
            }
            init_project_scope(recommended);
        }
    }

    println!("\n=== init complete ===");
}

// ── User scope ──────────────────────────────────────────────────────────────

fn init_user_scope(acp: Option<&str>, _recommended: bool) {
    println!("[user scope] Global setup — daemon, ACP registration\n");

    // 1. Daemon
    ensure_daemon();
    println!("  ✓ daemon running");

    // 2. Register with ACP(s) — handles plugin install (commands, skills, hooks, MCP)
    let acps: Vec<String> = if let Some(a) = acp {
        vec![a.to_string()]
    } else {
        let detected: Vec<serde_json::Value> = client()
            .get(format!("{}/api/assistants/detect", daemon_url()))
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
            .post(format!("{}/api/assistants/configure", daemon_url()))
            .json(&serde_json::json!({
                "acps": [acp_id],
            }))
            .send()
        {
            Ok(r) if r.status().is_success() => {
                // An unreadable 200 is NOT a success. Parsing it to `Null` used to
                // collapse to "no errors" → print "✓ registered", set any_success,
                // and persist user_scope_configured=true — so a broken/drifted
                // configure looked permanently healthy and later inits skipped setup.
                let body: serde_json::Value = match r.json() {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("  ✗ {} — configure returned an unreadable response: {}", acp_id, e);
                        all_errors.push(format!("{}: unreadable configure response: {}", acp_id, e));
                        continue;
                    }
                };
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

fn init_project_scope(_recommended: bool) {
    let repo_root = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: cannot determine current directory: {e}");
            std::process::exit(1);
        }
    };
    println!("[project scope] {}\n", repo_root.display());

    // 1. .sensei/ directory — mindsets, personas, rules
    let sensei_dir = repo_root.join(".sensei");
    fs::create_dir_all(&sensei_dir).ok();

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
    let mindsets_dst = sensei_dir.join("mindsets");
    fs::create_dir_all(&mindsets_dst).ok();

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
                MCP_REGISTRY_KEY.into(),
                serde_json::json!({"command": SENSEI_MCP_BIN}),
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

    // 4. Managed "load governance first" directive (idempotent) into CLAUDE.md +
    //    AGENTS.md — the durable pull-first fallback to the SessionStart/PreCompact
    //    hooks, and the only governance signal non-Claude assistants get.
    println!();
    for name in ["CLAUDE.md", "AGENTS.md"] {
        match managed::write_directive(&repo_root.join(name)) {
            Ok(managed::Change::Created) => println!("  [created] {name} — sensei governance directive"),
            Ok(managed::Change::Updated) => println!("  [updated] {name} — sensei governance directive"),
            Ok(managed::Change::Unchanged) => println!("  [ok]      {name} — governance directive current"),
            Err(e) => eprintln!("  [failed]  {name} — {e}"),
        }
    }

    // 4. Gate check
    println!("\n  --- gate check ---");
    if sensei_bootstrap::util::which_binary(SENSEID_BIN).is_some() {
        println!("  ✓ {SENSEID_BIN} on PATH");
    }
    if sensei_bootstrap::util::which_binary(SENSEI_MCP_BIN).is_some() {
        println!("  ✓ {SENSEI_MCP_BIN} on PATH");
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
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

// ── Remove ──────────────────────────────────────────────────────────────────

/// Resolve a user-supplied ACP name to a daemon ACP ID by querying
/// `/api/assistants/detect`. Returns `None` for "all" (remove everything).
/// Exits with an error if the name doesn't match any known ACP.
fn resolve_acp_id(name: &str) -> Option<String> {
    if name == "all" {
        return None;
    }

    let detected: Vec<serde_json::Value> = client()
        .get(format!("{}/api/assistants/detect", daemon_url()))
        .send()
        .ok()
        .and_then(|r| r.json().ok())
        .unwrap_or_default();

    let q = name.to_lowercase();
    let id = detected.iter().find_map(|a| {
        let id = a["id"].as_str()?;
        let display = a["name"].as_str().unwrap_or("").to_lowercase();
        // exact ID, word in ID (e.g. "desktop" → "claude-desktop"),
        // or display name starts with the query (e.g. "claude" → "Claude Code")
        if id == name || id.split('-').any(|w| w == q) || display.starts_with(&q) {
            Some(id.to_string())
        } else {
            None
        }
    });

    if id.is_none() {
        let available: Vec<&str> = detected.iter().filter_map(|a| a["id"].as_str()).collect();
        eprintln!(
            "Unknown ACP: '{}'. Available: {}",
            name,
            if available.is_empty() {
                "none detected".to_string()
            } else {
                available.join(", ")
            }
        );
        std::process::exit(1);
    }
    id
}

fn remove_cmd(target: &str, name: Option<&str>, purge: bool) {
    match target {
        "acp" => remove_acp(name.unwrap_or("all")),
        "all" => remove_all(purge),
        _ => {
            eprintln!("Unknown target: {target}. Usage:");
            eprintln!("  sensei remove acp <name|all>");
            eprintln!("  sensei remove all [--purge]");
            std::process::exit(1);
        }
    }
}

fn remove_acp(name: &str) {
    println!("=== sensei remove acp {} ===\n", name);

    // Daemon must be available before we can resolve the ACP ID.
    ensure_daemon();

    let acps: Vec<String> = match resolve_acp_id(name) {
        Some(id) => vec![id],
        None => vec![], // empty = remove all
    };

    match client()
        .post(format!("{}/api/assistants/remove", daemon_url()))
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
        .post(format!("{}/api/remove", daemon_url()))
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

        // Note: pre-brew installs dropped binaries in ~/.local/bin/. We
        // intentionally don't reach into that directory here — brew is the
        // single source of truth and ~/.local/bin/ is now user-owned.

        println!(
            "\nSensei fully removed. To reinstall: {} && {SENSEI_BIN} init",
            cfg().brew_install_script()
        );
    } else {
        println!("\nData preserved. To reinstall: {SENSEI_BIN} init");
    }
}

// ── Upgrade ───────────────────────────────────────────────────────────────

/// Refresh assistant integrations after a sensei upgrade. POSTs
/// [`UPGRADE_ENDPOINT`] and prints one line per assistant (name → ok/failed +
/// message). With no `--acp`, the daemon targets every detected assistant.
fn upgrade_cmd(acp: Option<&str>) {
    println!("=== sensei upgrade ===\n");

    // Daemon must be available to resolve the ACP id and run the upgrade.
    ensure_daemon();

    let acps: Vec<String> = match acp {
        // resolve_acp_id exits on an unknown name and returns None for "all".
        Some(name) => resolve_acp_id(name).into_iter().collect(),
        None => vec![], // empty = all detected
    };

    match client()
        .post(format!("{}{}", daemon_url(), UPGRADE_ENDPOINT))
        .json(&serde_json::json!({ "acps": acps }))
        .send()
    {
        Ok(r) if r.status().is_success() => {
            let reports: Vec<serde_json::Value> = r.json().unwrap_or_default();
            if reports.is_empty() {
                println!("  No assistants to upgrade.");
                return;
            }
            for rep in &reports {
                let id = rep["adapter_id"].as_str().unwrap_or("?");
                let ok = rep["ok"].as_bool() == Some(true);
                if ok {
                    let msg = join_strs(&rep["actions"]);
                    println!("  ✓ {} — {}", id, if msg.is_empty() { "ok".into() } else { msg });
                } else {
                    let msg = join_strs(&rep["errors"]);
                    eprintln!("  ✗ {} — {}", id, if msg.is_empty() { "failed".into() } else { msg });
                }
            }
        }
        Ok(r) => eprintln!("Upgrade failed: HTTP {}", r.status()),
        Err(e) => eprintln!("Upgrade failed: {}", e),
    }
}

/// Join a JSON array of strings into a `; `-separated line (empty when the
/// value isn't an array of strings).
fn join_strs(v: &serde_json::Value) -> String {
    v.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default()
}

// ── Daemon / Scan / AddLib ──────────────────────────────────────────────────

fn restart_daemon(port: u16) {
    // Prefer brew services restart for parity with start (keep-alive +
    // launchd ownership). Fall back to direct binary stop+start when brew
    // isn't on PATH or the service isn't registered.
    let service = cfg().brew_service_name();
    if let Ok(out) = std::process::Command::new("brew")
        .args(["services", "restart", service])
        .status()
        && out.success()
    {
        std::process::exit(0);
    }

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
                "Failed to run {SENSEID_BIN}: {e}. Install: {}",
                cfg().brew_install_script()
            );
            std::process::exit(1);
        }
    }
}

fn daemon_cmd(cmd: &str, port: Option<u16>) {
    // Lifecycle commands (stop) prefer brew services so launchd's keep-alive
    // marker stays in sync with the daemon's actual state. Status stays a
    // direct call — it's an informational probe of the binary, not a
    // lifecycle change.
    if cmd == "stop" {
        let service = cfg().brew_service_name();
        if let Ok(out) = std::process::Command::new("brew")
            .args(["services", "stop", service])
            .status()
            && out.success()
        {
            std::process::exit(0);
        }
        // Fall through to direct spawn — brew not available or service not
        // registered with launchd.
    }

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
                "Failed to run {SENSEID_BIN}: {e}. Install: {}",
                cfg().brew_install_script()
            );
            std::process::exit(1);
        }
    }
}

fn scan(path: &str) {
    ensure_daemon();
    match client()
        .post(format!("{}/api/scan", daemon_url()))
        .json(&serde_json::json!({"root": path, "max_depth": 4}))
        .send()
    {
        Ok(r) if r.status().is_success() => println!("Scanning {} (background)...", path),
        _ => eprintln!("Scan request failed"),
    }
}

/// `sensei scaffold` — materialize the canonical doc structure into `path`
/// (default: current directory). Prints a `[created]`/`[exists]` report per
/// path and exits non-zero if anything failed (no silent errors).
fn scaffold_cmd(what: Option<ScaffoldTarget>, path: Option<&str>) {
    let target = match path {
        Some(p) => PathBuf::from(p),
        None => match std::env::current_dir() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: cannot determine current directory: {e}");
                std::process::exit(1);
            }
        },
    };
    let report = match what {
        Some(ScaffoldTarget::Feature { name }) => {
            println!(
                "=== sensei scaffold feature {name} ===\n{}\n",
                target.display()
            );
            match scaffold::run_feature(&target, &name) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(ScaffoldTarget::Baseline { kind }) => {
            println!(
                "=== sensei scaffold baseline --kind {} ===\n{}\n",
                kind.slug(),
                target.display()
            );
            scaffold::run_baseline(&target, kind)
        }
        None => {
            println!("=== sensei scaffold ===\n{}\n", target.display());
            scaffold::run(&target)
        }
    };
    for p in &report.created {
        println!("  [created] {p}");
    }
    for p in &report.skipped {
        println!("  [exists]  {p}");
    }
    for (p, err) in &report.failed {
        eprintln!("  [failed]  {p} — {err}");
    }
    println!(
        "\n{} created, {} already present, {} failed.",
        report.created.len(),
        report.skipped.len(),
        report.failed.len()
    );
    if !report.failed.is_empty() {
        std::process::exit(1);
    }
}

/// `sensei index doctor` — GET the daemon's read-only index integrity report and
/// print per-class drift counts + a few samples. Read-only: the daemon's periodic
/// audit owns repair.
fn index_doctor() {
    ensure_daemon();
    match client().get(format!("{}/api/index/doctor", daemon_url())).send() {
        Ok(r) if r.status().is_success() => {
            let report: serde_json::Value = r.json().unwrap_or_default();
            print_index_doctor(&report);
        }
        Ok(r) => eprintln!("index doctor failed: HTTP {}", r.status()),
        Err(e) => eprintln!("index doctor failed: {}", e),
    }
}

/// Render an index-doctor report. Kept separate from the HTTP call so the format
/// is pure over the JSON payload.
fn print_index_doctor(r: &serde_json::Value) {
    let n = |k: &str| r[k].as_u64().unwrap_or(0);
    println!("=== index doctor ===\n");
    println!(
        "roots: {} checked, {} present, {} absent (unmounted — skipped)\n",
        n("roots_checked"), n("roots_present"), n("roots_absent")
    );

    let classes = [
        ("orphan nodes (indexed file gone)", "orphan_files", "orphan_files"),
        ("ghost folders (directory gone)", "ghost_folders", "ghost_folders"),
        ("nested standalone (mis-scoped in a repo)", "nested_standalone", "nested_standalone"),
        ("duplicate-name projects", "duplicate_name_projects", "duplicate_name_projects"),
    ];
    let total: u64 = classes.iter().map(|(_, count_key, _)| n(count_key)).sum();
    if total == 0 {
        println!("index is invariant-clean — no drift detected.");
        return;
    }

    println!("drift detected (repaired automatically by the daemon's periodic audit):");
    for (label, count_key, sample_key) in classes {
        let count = n(count_key);
        println!("  {:<42} {}", label, count);
        if count > 0
            && let Some(samples) = r["samples"][sample_key].as_array()
        {
            for s in samples.iter().filter_map(|s| s.as_str()) {
                println!("      - {s}");
            }
        }
    }
}

/// `sensei folder remap <old> <new>` — POST the daemon's manual rename-repair
/// route and print what it did. `new` must already be an indexed folder.
fn folder_remap(old: &str, new: &str) {
    ensure_daemon();
    match client()
        .post(format!("{}/api/folders/remap", daemon_url()))
        .json(&serde_json::json!({"old": old, "new": new}))
        .send()
    {
        Ok(r) if r.status().is_success() => {
            let d: serde_json::Value = r.json().unwrap_or_default();
            let action = if d["remapped"].as_bool() == Some(true) { "re-pointed history and aliased" } else { "aliased" };
            println!(
                "Remapped {old} → {new}: {action}; {} session(s) re-attached.",
                d["sessions_repaired"].as_u64().unwrap_or(0)
            );
        }
        Ok(r) if r.status().as_u16() == 404 => {
            eprintln!("remap failed: '{new}' is not an indexed folder — run `sensei scan` on it first.");
            std::process::exit(1);
        }
        Ok(r) => { eprintln!("remap failed: HTTP {}", r.status()); std::process::exit(1); }
        Err(e) => { eprintln!("remap failed: {e}"); std::process::exit(1); }
    }
}

fn add_lib(name: &str, url: Option<&str>) {
    ensure_daemon();
    let c = client_with_timeout(45);
    let mut body = serde_json::json!({"tool": "add_library", "params": {"name": name}});
    if let Some(u) = url {
        body["params"]["url"] = serde_json::json!(u);
    }
    match c
        .post(format!("{}/api/mcp/call", daemon_url()))
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

/// `sensei models pull <id>` — POST the daemon's provision route and print the
/// initial phase. Non-blocking on the daemon side; this just reports the phase
/// the pull started in (`queued`, or the live phase if already in flight/ready).
fn models_pull(id: &str) {
    ensure_daemon();
    let url = format!("{}/api/gateway/models/{}/provision", daemon_url(), id);
    match client().post(url).send() {
        Ok(r) if r.status().is_success() => {
            let d: serde_json::Value = r.json().unwrap_or_default();
            println!("{}  {}", id, format_phase(&d["phase"]));
            println!("Pull started in the background — track it with: sensei models status");
        }
        // The daemon returns 501 when built without the embedded engine; surface
        // its JSON `error` verbatim so the reason is actionable, not a bare code.
        Ok(r) => {
            let status = r.status();
            let d: serde_json::Value = r.json().unwrap_or_default();
            match d["error"].as_str() {
                Some(msg) => eprintln!("models pull failed: {msg}"),
                None => eprintln!("models pull failed: HTTP {status}"),
            }
        }
        Err(e) => eprintln!("models pull failed: {e}"),
    }
}

/// `sensei models status` — GET the daemon's provision-status snapshot and print
/// a small `id  phase` table (or a friendly note when nothing is tracked).
fn models_status() {
    ensure_daemon();
    match client().get(format!("{}/api/gateway/models/provision/status", daemon_url())).send() {
        Ok(r) if r.status().is_success() => {
            let d: serde_json::Value = r.json().unwrap_or_default();
            print_models_status(&d);
        }
        Ok(r) => eprintln!("models status failed: HTTP {}", r.status()),
        Err(e) => eprintln!("models status failed: {e}"),
    }
}

/// Render a provision-status payload. Kept pure over the JSON so the table
/// format is unit-testable without a daemon.
///
/// The status now lists the full provisionable catalog — every pullable model
/// with its current phase (`not pulled` before any pull, via `format_phase` of
/// `{"phase":"absent"}`). An empty list therefore means the daemon has no
/// catalog at all (built without the embedded engine), not "nothing pulled".
fn print_models_status(d: &serde_json::Value) {
    let models = d["models"].as_array().cloned().unwrap_or_default();
    if models.is_empty() {
        println!("No local models available — this build has no embedded runtime.");
        return;
    }
    println!("{:<24} PHASE", "MODEL");
    for m in &models {
        let id = m["id"].as_str().unwrap_or("?");
        println!("{:<24} {}", id, format_phase(&m["phase"]));
    }
}

/// Format a `ProvisionPhase` JSON value (`{"phase":"...", ...}`) as a compact,
/// human-readable string. Pure over the daemon's serde shape (kernel
/// `ProvisionPhase`, internally tagged `"phase"`, snake_case).
fn format_phase(phase: &serde_json::Value) -> String {
    match phase["phase"].as_str() {
        Some("downloading") => {
            let done = phase["done"].as_u64().unwrap_or(0);
            match phase["total"].as_u64() {
                Some(total) if total > 0 => {
                    let pct = (done as f64 / total as f64 * 100.0).round() as u64;
                    format!("downloading ({pct}%)")
                }
                _ => format!("downloading ({} bytes)", done),
            }
        }
        Some("failed") => match phase["error"].as_str() {
            Some(err) => format!("failed: {err}"),
            None => "failed".to_string(),
        },
        Some(other) => other.to_string(),
        None => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_subcommand_parses_without_acp() {
        let cli = Cli::parse_from(["sensei", "upgrade"]);
        match cli.command {
            Some(Commands::Upgrade { acp }) => assert!(acp.is_none(), "no --acp → all detected"),
            _ => panic!("expected Upgrade command"),
        }
    }

    #[test]
    fn upgrade_subcommand_parses_with_acp() {
        let cli = Cli::parse_from(["sensei", "upgrade", "--acp", "claude"]);
        match cli.command {
            Some(Commands::Upgrade { acp }) => assert_eq!(acp.as_deref(), Some("claude")),
            _ => panic!("expected Upgrade command"),
        }
    }

    #[test]
    fn upgrade_flag_parses_as_all_acp_shortcut() {
        // `sensei --upgrade` (flag form Jerry asked for) → no subcommand, flag
        // set → main dispatches upgrade_cmd(None) = all detected assistants.
        let cli = Cli::parse_from(["sensei", "--upgrade"]);
        assert!(cli.upgrade, "--upgrade flag set");
        assert!(cli.command.is_none(), "flag form carries no subcommand");
    }

    #[test]
    fn bare_invocation_has_no_command_and_no_upgrade() {
        // `sensei` alone → help path (command None, upgrade false), never a panic.
        let cli = Cli::parse_from(["sensei"]);
        assert!(cli.command.is_none());
        assert!(!cli.upgrade);
    }

    #[test]
    fn upgrade_targets_assistants_upgrade_endpoint() {
        // The subcommand must POST to the daemon's assistant-upgrade route.
        assert_eq!(UPGRADE_ENDPOINT, "/api/assistants/upgrade");
    }

    #[test]
    fn index_doctor_subcommand_parses_and_dispatches() {
        // `sensei index doctor` → Index { cmd: Doctor }. Unit-level: proves the
        // arg surface parses + routes to the doctor branch without a live daemon.
        let cli = Cli::parse_from(["sensei", "index", "doctor"]);
        match cli.command {
            Some(Commands::Index { cmd: IndexCommands::Doctor }) => {}
            other => panic!("expected Index doctor, got {:?}", other.is_some()),
        }
    }

    #[test]
    fn index_requires_a_subcommand() {
        // Bare `sensei index` must not parse (a subcommand is required), so the
        // command never silently no-ops.
        assert!(Cli::try_parse_from(["sensei", "index"]).is_err());
    }

    #[test]
    fn models_pull_subcommand_parses_the_id() {
        // `sensei models pull gemma2:2b` → Models { Pull { id } } with the id
        // carried through (including the `:` in the model id).
        let cli = Cli::parse_from(["sensei", "models", "pull", "gemma2:2b"]);
        match cli.command {
            Some(Commands::Models { cmd: ModelsCommands::Pull { id } }) => {
                assert_eq!(id, "gemma2:2b");
            }
            other => panic!("expected Models pull, got {:?}", other.is_some()),
        }
    }

    #[test]
    fn models_status_subcommand_parses() {
        let cli = Cli::parse_from(["sensei", "models", "status"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Models { cmd: ModelsCommands::Status })
        ));
    }

    #[test]
    fn models_requires_a_subcommand() {
        // Bare `sensei models` must not parse — a subcommand is required.
        assert!(Cli::try_parse_from(["sensei", "models"]).is_err());
    }

    #[test]
    fn format_phase_renders_each_kernel_phase_shape() {
        use serde_json::json;
        // Simple tagged phases render as their name.
        assert_eq!(format_phase(&json!({"phase": "queued"})), "queued");
        assert_eq!(format_phase(&json!({"phase": "ready"})), "ready");
        assert_eq!(format_phase(&json!({"phase": "verifying"})), "verifying");
        // Downloading with a known total → percentage.
        assert_eq!(
            format_phase(&json!({"phase": "downloading", "done": 50, "total": 100})),
            "downloading (50%)"
        );
        // Downloading with an unknown total → byte count (no divide-by-zero).
        assert_eq!(
            format_phase(&json!({"phase": "downloading", "done": 42, "total": null})),
            "downloading (42 bytes)"
        );
        // Failed carries the reason.
        assert_eq!(
            format_phase(&json!({"phase": "failed", "error": "disk full"})),
            "failed: disk full"
        );
        // Malformed / missing tag never panics.
        assert_eq!(format_phase(&json!({})), "unknown");
    }

    #[test]
    fn print_models_status_handles_empty_and_populated() {
        // Empty list → the daemon has no catalog (non-embedded build); prints
        // the "no embedded runtime" note (no panic).
        print_models_status(&serde_json::json!({"models": []}));
        // Populated catalog → renders the table (no panic) across phase shapes,
        // including a not-yet-pulled `absent` catalog row.
        print_models_status(&serde_json::json!({"models": [
            {"id": "gemma2:2b", "name": "Gemma 2 2B Instruct", "phase": {"phase": "absent"}},
            {"id": "downloading-one", "name": "DL", "phase": {"phase": "downloading", "done": 1, "total": 4}},
            {"id": "other", "name": "Other", "phase": {"phase": "ready"}},
        ]}));
    }

    #[test]
    fn format_phase_renders_absent_as_its_name() {
        // Catalog rows carry `absent` before any pull; format_phase falls
        // through to the raw phase name — never "unknown", never a panic.
        assert_eq!(format_phase(&serde_json::json!({"phase": "absent"})), "absent");
    }

    #[test]
    fn print_index_doctor_handles_clean_and_drift_reports() {
        // Clean report: zero counts → no panic (renders the clean line).
        print_index_doctor(&serde_json::json!({
            "roots_checked": 3, "roots_present": 3, "roots_absent": 0,
            "orphan_files": 0, "ghost_folders": 0, "nested_standalone": 0,
            "duplicate_name_projects": 0, "samples": {}
        }));
        // Drift report with samples → no panic (renders per-class + samples).
        print_index_doctor(&serde_json::json!({
            "roots_checked": 2, "roots_present": 1, "roots_absent": 1,
            "orphan_files": 2, "ghost_folders": 1, "nested_standalone": 0,
            "duplicate_name_projects": 0,
            "samples": { "orphan_files": ["/a/b.rs", "/a/c.rs"], "ghost_folders": ["/a/gone"] }
        }));
    }

    #[test]
    fn join_strs_joins_string_arrays_and_ignores_non_arrays() {
        assert_eq!(
            join_strs(&serde_json::json!(["upgraded claude-code plugin"])),
            "upgraded claude-code plugin"
        );
        assert_eq!(
            join_strs(&serde_json::json!(["a", "b"])),
            "a; b"
        );
        assert_eq!(join_strs(&serde_json::json!([])), "");
        assert_eq!(join_strs(&serde_json::json!("not an array")), "");
    }

    #[test]
    fn scaffold_subcommand_parses_project_form() {
        let bare = Cli::parse_from(["sensei", "scaffold"]);
        match bare.command {
            Some(Commands::Scaffold { what, path }) => {
                assert!(what.is_none());
                assert!(path.is_none());
            }
            _ => panic!("expected Scaffold command"),
        }
        let with = Cli::parse_from(["sensei", "scaffold", "--path", "/tmp/x"]);
        match with.command {
            Some(Commands::Scaffold { path, .. }) => assert_eq!(path.as_deref(), Some("/tmp/x")),
            _ => panic!("expected Scaffold command"),
        }
    }

    #[test]
    fn scaffold_feature_subcommand_parses() {
        let cli = Cli::parse_from(["sensei", "scaffold", "feature", "auth"]);
        match cli.command {
            Some(Commands::Scaffold {
                what: Some(ScaffoldTarget::Feature { name }),
                ..
            }) => assert_eq!(name, "auth"),
            _ => panic!("expected Scaffold feature command"),
        }
    }

    #[test]
    fn scaffold_baseline_subcommand_parses_kind() {
        let cli = Cli::parse_from(["sensei", "scaffold", "baseline", "--kind", "content"]);
        match cli.command {
            Some(Commands::Scaffold {
                what: Some(ScaffoldTarget::Baseline { kind }),
                ..
            }) => assert_eq!(kind, scaffold::BaselineKind::Content),
            _ => panic!("expected Scaffold baseline command"),
        }
        // default kind = code
        let d = Cli::parse_from(["sensei", "scaffold", "baseline"]);
        match d.command {
            Some(Commands::Scaffold {
                what: Some(ScaffoldTarget::Baseline { kind }),
                ..
            }) => assert_eq!(kind, scaffold::BaselineKind::Code),
            _ => panic!("expected Scaffold baseline command"),
        }
    }
}
