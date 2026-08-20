//! Single source of truth for sensei configuration.
//!
//! Every component that needs configuration (daemon port, DB name, directory)
//! should derive those values from [`SenseiConfig::from_env()`] rather than
//! implementing its own env-var reading.
//!
//! All three dependents — Tauri sidecar, bootstrap crate, and senseid daemon —
//! already depend on this crate, so importing from here creates no new edges.
//!
//! ## Mode
//!
//! Sensei has a single mode now: port 7744, `sensei` database, `~/.sensei/`.
//! The previous `dev` Cargo feature (port 7745, `sensei_dev` DB, `~/.sensei-dev/`)
//! has been removed — developers iterate against the production install. If a
//! clean slate is needed, drop and re-create the DB.
//!
//! The one remaining knob: `SENSEI_DDL_DIR=/path/to/database` overrides the
//! schema source at runtime, so DDL changes can be tested locally before
//! pushing a release tag.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

// ── Project-wide constants ────────────────────────────────────────────────────

/// GitHub organization that owns all sensei repositories.
pub const GITHUB_ORG: &str = "sensei-hq";

/// GitHub repository name for the main sensei project.
pub const GITHUB_REPO: &str = "sensei";

/// Homebrew tap slug used in install/reinstall messages.
pub const BREW_TAP: &str = "sensei-hq/tap/sensei";

// ── Binary names ──────────────────────────────────────────────────────────────
//
// Exposed as `const &str` so they can be used at attribute/macro time
// (e.g. clap's `#[command(name = SENSEI_BIN, ...)]`).

/// `sensei` CLI binary name.
pub const SENSEI_BIN: &str = "sensei";

/// `senseid` daemon binary name.
pub const SENSEID_BIN: &str = "senseid";

/// `sensei-mcp` server binary name.
pub const SENSEI_MCP_BIN: &str = "sensei-mcp";

/// MCP server registry key. Used by sensei CLI and senseid daemon when
/// registering/removing the MCP entry in ACP configs (Claude Code, Cursor, …).
pub const MCP_REGISTRY_KEY: &str = "sensei";

/// Homebrew tap repository slug (for reference/logging).
pub const HOMEBREW_TAP_REPO: &str = "sensei-hq/homebrew-tap";

/// Base raw-content URL for the marketplace repository.
/// Append `/{path}` to download individual files.
pub const MARKETPLACE_RAW_URL: &str =
    "https://raw.githubusercontent.com/sensei-hq/marketplace/main";

/// Marketplace repository slug (for `claude plugin add`).
pub const MARKETPLACE_REPO: &str = "sensei-hq/marketplace";

/// Candidate paths for the Homebrew binary (Apple Silicon first, Intel fallback).
pub const BREW_PATHS: [&str; 2] = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"];

/// Full GitHub URL for the Homebrew tap repository.
pub const HOMEBREW_TAP_URL: &str = "https://github.com/sensei-hq/homebrew-tap";

// ── Service ports ─────────────────────────────────────────────────────────────

/// Default daemon port.
pub const DAEMON_PORT: u16 = 7744;

/// Default Ollama port.
pub const OLLAMA_PORT: u16 = 11434;

/// Default PostgreSQL port.
pub const POSTGRES_PORT: u16 = 5432;

// ── Database pool defaults ────────────────────────────────────────────────────

/// Maximum number of connections in the pool.
///
/// Sized to support a cores-scaled task-worker pool (see `senseid`'s
/// `resolve_worker_count`) plus headroom for the daemon's non-worker DB users —
/// the schedulers (metrics/analyzer/reconcile/pruners/advance_run/watchdog/
/// contribute), the relay loop, and transient HTTP handlers. Workers are capped
/// at `DB_POOL_MAX_CONNECTIONS - WORKER_DB_RESERVE`, leaving the reserve free for
/// those callers. The pool keeps `DB_POOL_MIN_CONNECTIONS` open at rest and grows
/// to this ceiling only under load, so it is modest when idle and stays well
/// under Postgres's default `max_connections = 100`.
pub const DB_POOL_MAX_CONNECTIONS: u32 = 24;

/// The warm floor: connections the pool keeps open at all times, even when fully
/// idle. The pool starts here, grows to [`DB_POOL_MAX_CONNECTIONS`] under load,
/// then reaps the extras back down to this floor after `DB_POOL_IDLE_TIMEOUT_SECS`
/// (sqlx never reaps below `min_connections`). A small floor avoids cold-connect
/// latency on the first queries after an idle spell while keeping the at-rest
/// process/memory count low — each idle backend maps `shared_buffers`, so fewer
/// idle backends means fewer postgres processes at rest. Must be ≤
/// `DB_POOL_MAX_CONNECTIONS`.
pub const DB_POOL_MIN_CONNECTIONS: u32 = 8;

/// How long (in seconds) a caller waits for a connection before giving up.
///
/// Scan tasks that time out are retried by the executor, so a short timeout
/// surfaces backpressure quickly instead of piling up waiters.
pub const DB_POOL_ACQUIRE_TIMEOUT_SECS: u64 = 10;

/// How long (in seconds) an idle connection is kept alive before being closed.
pub const DB_POOL_IDLE_TIMEOUT_SECS: u64 = 300;

/// Configuration for all sensei components.
///
/// Default: production install — port 7744, DB `sensei`, dir `~/.sensei/`.
///
/// Set `SENSEI_INSTANCE=<name>` at runtime to derive an isolated DB + data
/// directory (DB becomes `sensei_<name>`, dir becomes `~/.sensei-<name>/`).
/// The daemon port is NOT affected — only one instance can run at a time
/// per machine. Used by e2e tests to avoid clobbering the user's real
/// `sensei` DB; set `SENSEI_INSTANCE=e2e` and the daemon talks to a
/// throw-away `sensei_e2e` DB in `~/.sensei-e2e/`.
#[derive(Debug, Clone)]
pub struct SenseiConfig {
    pub daemon_port: u16,
    pub db_name: String,
    pub db_url: String,
    pub dir_suffix: String,
}

impl SenseiConfig {
    pub fn from_env() -> Self {
        let instance = std::env::var("SENSEI_INSTANCE")
            .ok()
            .filter(|s| !s.is_empty());
        let (db_name, dir_suffix) = match instance.as_deref() {
            Some(name) => (format!("sensei_{name}"), format!(".sensei-{name}")),
            None       => ("sensei".to_string(),    ".sensei".to_string()),
        };
        let db_url = format!("postgresql://localhost:{POSTGRES_PORT}/{db_name}");
        Self {
            daemon_port: DAEMON_PORT,
            db_name,
            db_url,
            dir_suffix,
        }
    }

    /// Daemon base URL derived from the configured port.
    pub fn daemon_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.daemon_port)
    }

    /// Daemon binary name.
    pub fn daemon_binary(&self) -> &'static str { SENSEID_BIN }

    /// Sensei data directory (`~/.sensei/`).
    pub fn sensei_dir(&self) -> PathBuf {
        home_dir().join(&self.dir_suffix)
    }

    /// Log file path.
    pub fn log_path(&self) -> PathBuf {
        self.sensei_dir().join("senseid.log")
    }

    /// PID file path.
    pub fn pid_path(&self) -> PathBuf {
        self.sensei_dir().join("serve.pid")
    }

    /// Returns the sensei CLI binary name.
    pub fn sensei_binary(&self) -> &'static str { SENSEI_BIN }

    /// Returns the senseid daemon binary name.
    pub fn senseid_binary(&self) -> &'static str { SENSEID_BIN }

    /// Returns the sensei homebrew tap formula slug.
    pub fn sensei_tap_formula(&self) -> &'static str { BREW_TAP }

    /// Returns the bare brew formula name (no tap prefix). Matches the formula
    /// filename in `homebrew/Formula/` and the registered service name in the
    /// formula's `service do` block. Use with `brew services start <name>` /
    /// `brew services stop <name>` — the tap-qualified slug only works for
    /// `brew install`.
    pub fn brew_service_name(&self) -> &'static str { "sensei" }

    /// Returns the full `brew install <formula>` script. Suitable for direct
    /// copy/paste in a shell or for display in a [`Remedy`].
    pub fn brew_install_script(&self) -> String {
        format!("brew install {}", self.sensei_tap_formula())
    }

    /// Returns the sensei-mcp binary name.
    pub fn sensei_mcp_binary(&self) -> &'static str { SENSEI_MCP_BIN }

    /// Returns the MCP registry key.
    pub fn mcp_registry_key(&self) -> &'static str { MCP_REGISTRY_KEY }

    /// Resolve the database schema source string for dbd-core's
    /// `resolve_source()`.
    ///
    /// Default: the GitHub-tagged release matching `version`.
    /// Override: set `SENSEI_DDL_DIR=/abs/path/to/database` to point at a
    /// local directory — useful when iterating on DDL before publishing a
    /// release tag. The env var is read at runtime so a single binary
    /// supports both flows.
    pub fn db_schema_source(&self, version: &str) -> String {
        if let Ok(local) = std::env::var("SENSEI_DDL_DIR")
            && !local.is_empty()
        {
            return local;
        }
        format!("{GITHUB_ORG}/{GITHUB_REPO}/database@v{version}")
    }
}

/// User's home directory.
///
/// Panics if the `HOME` environment variable is not set — we cannot safely
/// fall back to `/tmp` because that would silently write config/data files in
/// the wrong location.
pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .expect("HOME environment variable is not set — cannot determine user home directory")
}

// ── Local config file ─────────────────────────────────────────────────────────

/// Contents of `~/.sensei/config.json`.
///
/// Single source of truth for persisted local state shared between the daemon
/// and the CLI. Read/write via [`SenseiLocalConfig::load`] and
/// [`SenseiLocalConfig::save`].
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SenseiLocalConfig {
    /// Set to `true` after the user completes their first `sensei init`.
    #[serde(default)]
    pub user_scope_configured: bool,

    /// IDs of assistants that have been successfully configured (e.g. `"claude-code"`).
    #[serde(default)]
    pub configured_assistants: Vec<String>,

    /// Installed marketplace version string (e.g. `"0.3.1"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marketplace_version: Option<String>,
}

impl SenseiLocalConfig {
    /// Load from `<sensei_dir>/config.json`. Returns `Default` if the file is
    /// missing or cannot be parsed — config errors are non-fatal.
    pub fn load(sensei_dir: &Path) -> Self {
        let path = sensei_dir.join("config.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save to `<sensei_dir>/config.json`, creating the directory if needed.
    pub fn save(&self, sensei_dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(sensei_dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(sensei_dir.join("config.json"), json).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These assertions pin a compile-time CONTRACT between two consts (a regression
    // guard if either is edited), so a constant condition is the point — not a bug.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn pool_warm_floor_is_below_the_ceiling() {
        // sqlx rejects a pool with min_connections > max_connections at build
        // time, and a floor that swallows the whole pool would leave no elastic
        // headroom. Keep the warm floor a strict, non-zero fraction of the max.
        assert!(DB_POOL_MIN_CONNECTIONS >= 1, "keep a warm floor so first queries don't cold-connect");
        assert!(
            DB_POOL_MIN_CONNECTIONS < DB_POOL_MAX_CONNECTIONS,
            "warm floor ({DB_POOL_MIN_CONNECTIONS}) must leave room to grow up to the ceiling ({DB_POOL_MAX_CONNECTIONS})"
        );
    }

    #[test]
    fn config_instance_override_and_defaults() {
        // Combined into one test because cargo runs tests in parallel and
        // these branches all mutate the shared SENSEI_INSTANCE env var.
        // Same pattern as `db_schema_source_default_and_override`.
        //
        // SAFETY: process-global env mutation. Safe here because no other
        // test in this binary reads SENSEI_INSTANCE and the var is cleared
        // before returning.

        // 1. Default — no env var set.
        unsafe { std::env::remove_var("SENSEI_INSTANCE"); }
        let cfg = SenseiConfig::from_env();
        assert_eq!(cfg.daemon_port, 7744);
        assert_eq!(cfg.db_name,    "sensei");
        assert_eq!(cfg.dir_suffix, ".sensei");

        // 2. Override — SENSEI_INSTANCE=e2e.
        unsafe { std::env::set_var("SENSEI_INSTANCE", "e2e"); }
        let cfg = SenseiConfig::from_env();
        assert_eq!(cfg.db_name,    "sensei_e2e",  "DB derived from instance");
        assert_eq!(cfg.dir_suffix, ".sensei-e2e", "dir derived from instance");
        assert!(
            cfg.db_url.ends_with("/sensei_e2e"),
            "db_url embeds the derived db_name: {}",
            cfg.db_url,
        );
        // Port is NOT instance-scoped — only one instance runs at a time.
        assert_eq!(cfg.daemon_port, 7744);

        // 3. Empty value — falls back to default.
        unsafe { std::env::set_var("SENSEI_INSTANCE", ""); }
        let cfg = SenseiConfig::from_env();
        assert_eq!(cfg.db_name,    "sensei", "empty instance falls back to default");
        assert_eq!(cfg.dir_suffix, ".sensei");

        unsafe { std::env::remove_var("SENSEI_INSTANCE"); }
    }

    #[test]
    fn db_url_contains_db_name() {
        let cfg = SenseiConfig::from_env();
        assert!(cfg.db_url.contains(&cfg.db_name));
    }

    #[test]
    fn db_url_is_well_formed() {
        let cfg = SenseiConfig::from_env();
        assert!(cfg.db_url.starts_with("postgresql://"));
    }

    #[test]
    fn home_dir_is_not_empty() {
        let h = home_dir();
        assert!(!h.as_os_str().is_empty());
    }

    #[test]
    fn port_constants() {
        assert_eq!(DAEMON_PORT, 7744);
        assert_eq!(OLLAMA_PORT, 11434);
        assert_eq!(POSTGRES_PORT, 5432);
    }

    #[test]
    fn binary_names() {
        let cfg = SenseiConfig::from_env();
        assert_eq!(cfg.sensei_binary(), "sensei");
        assert_eq!(cfg.senseid_binary(), "senseid");
        assert_eq!(cfg.sensei_mcp_binary(), "sensei-mcp");
    }

    #[test]
    fn const_binary_names_match_accessors() {
        let cfg = SenseiConfig::from_env();
        assert_eq!(SENSEI_BIN, cfg.sensei_binary());
        assert_eq!(SENSEID_BIN, cfg.senseid_binary());
        assert_eq!(SENSEI_MCP_BIN, cfg.sensei_mcp_binary());
    }

    #[test]
    fn sensei_tap_formula_is_prod_tap() {
        let cfg = SenseiConfig::from_env();
        assert_eq!(cfg.sensei_tap_formula(), "sensei-hq/tap/sensei");
    }

    #[test]
    fn brew_install_script_has_no_head_flag() {
        let cfg = SenseiConfig::from_env();
        let script = cfg.brew_install_script();
        assert_eq!(script, "brew install sensei-hq/tap/sensei");
        assert!(!script.contains("--HEAD"));
    }

    #[test]
    fn mcp_registry_key_is_sensei() {
        let cfg = SenseiConfig::from_env();
        assert_eq!(MCP_REGISTRY_KEY, "sensei");
        assert_eq!(cfg.mcp_registry_key(), "sensei");
    }

    #[test]
    fn db_schema_source_default_and_override() {
        // Combined into one test because cargo runs tests in parallel by
        // default and these two cases both mutate the shared SENSEI_DDL_DIR
        // env var. Splitting them caused intermittent failures where the
        // "default" test would see the env var set by the "override" test.
        // Each branch sets/clears the env var in the order it expects.
        //
        // SAFETY: process-global env mutation. Safe here because no other
        // test in this binary reads SENSEI_DDL_DIR and the var is cleared
        // before returning.

        unsafe { std::env::remove_var("SENSEI_DDL_DIR"); }
        let cfg = SenseiConfig::from_env();
        assert_eq!(
            cfg.db_schema_source("1.2.3"),
            "sensei-hq/sensei/database@v1.2.3",
            "default → GitHub tag",
        );

        unsafe { std::env::set_var("SENSEI_DDL_DIR", "/tmp/test-ddl"); }
        let cfg = SenseiConfig::from_env();
        assert_eq!(
            cfg.db_schema_source("1.2.3"),
            "/tmp/test-ddl",
            "SENSEI_DDL_DIR set → override path",
        );

        unsafe { std::env::remove_var("SENSEI_DDL_DIR"); }
    }
}
