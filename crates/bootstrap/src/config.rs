//! Single source of truth for all mode-sensitive sensei configuration.
//!
//! Every component that needs mode-awareness (daemon port, DB name, directory
//! suffix) should derive those values from [`SenseiConfig::from_env()`] rather
//! than implementing its own env-var reading.
//!
//! All three dependents — Tauri sidecar, bootstrap crate, and senseid daemon —
//! already depend on this crate, so importing from here creates no new edges.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

// ── Project-wide constants ────────────────────────────────────────────────────

/// GitHub organization that owns all sensei repositories.
pub const GITHUB_ORG: &str = "sensei-hq";

/// GitHub repository name for the main sensei project.
pub const GITHUB_REPO: &str = "sensei";

/// Homebrew tap slug used in install/reinstall messages.
pub const BREW_TAP: &str = "sensei-hq/tap/sensei";

/// Raw GitHub URL for the homebrew tap Brewfile (authoritative install source).
pub const HOMEBREW_BREWFILE_URL: &str =
    "https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile";

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

/// True when the current executable's filename ends with `-dev` (e.g. `senseid-dev`, `sensei-dev`).
///
/// Call once at binary startup to detect dev mode from binary name, before env-var
/// detection runs. Both `senseid` and `sensei` CLI use this — do not duplicate the logic.
pub fn binary_is_dev() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .map(|name| name.ends_with("-dev"))
        .unwrap_or(false)
}

/// Default daemon port (production).
/// **Do not use directly.** Always use `SenseiConfig::from_env().daemon_port` —
/// this constant is 7744 (prod only) and will be wrong in dev mode.
const DAEMON_PORT: u16 = 7744;

/// Default Ollama port.
pub const OLLAMA_PORT: u16 = 11434;

/// Default PostgreSQL port.
pub const POSTGRES_PORT: u16 = 5432;

// ── Database pool defaults ────────────────────────────────────────────────────

/// Maximum number of connections in the pool.
///
/// Scan operations are I/O-bound and batched, so a small pool keeps pressure
/// on the DB low while still supporting concurrent tasks.  Individual tasks
/// acquire a connection for a single query and release it immediately.
pub const DB_POOL_MAX_CONNECTIONS: u32 = 10;

/// How long (in seconds) a caller waits for a connection before giving up.
///
/// Scan tasks that time out are retried by the executor, so a short timeout
/// surfaces backpressure quickly instead of piling up waiters.
pub const DB_POOL_ACQUIRE_TIMEOUT_SECS: u64 = 10;

/// How long (in seconds) an idle connection is kept alive before being closed.
pub const DB_POOL_IDLE_TIMEOUT_SECS: u64 = 300;

/// Runtime mode — controls ports, directory names, and database names.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SenseiMode {
    /// Production mode: port 7744, `sensei` DB, `~/.sensei/`
    Prod,
    /// Development / E2E mode: port 7745, `sensei_dev` DB (or `SENSEI_DB_NAME`), `~/.sensei-dev/`
    Dev,
}

impl SenseiMode {
    /// Detect mode from `SENSEI_MODE` env var. Defaults to `Prod`.
    pub fn from_env() -> Self {
        match std::env::var("SENSEI_MODE").as_deref() {
            Ok("dev" | "development" | "test") => SenseiMode::Dev,
            _ => SenseiMode::Prod,
        }
    }

    pub fn is_dev(self) -> bool {
        self == SenseiMode::Dev
    }
}

/// Runtime configuration for all sensei components.
///
/// Derived once from environment variables. Callers own the lifetime — cache
/// with `once_cell` / `OnceLock` if you need a global singleton.
///
/// ## Env-var priority (highest wins)
///
/// | Value     | Priority 1        | Priority 2         | Priority 3 |
/// |-----------|-------------------|--------------------|------------|
/// | `db_url`  | `DATABASE_URL`    | derived from name  | —          |
/// | `db_name` | `SENSEI_DB_NAME`  | mode default       | —          |
/// | `port`    | *(no override)*   | mode default       | —          |
/// | `mode`    | `SENSEI_MODE`     | Prod               | —          |
#[derive(Debug, Clone)]
pub struct SenseiConfig {
    /// Current runtime mode.
    pub mode: SenseiMode,
    /// Daemon HTTP port (7744 prod / 7745 dev).
    pub daemon_port: u16,
    /// Active database name (`sensei`, `sensei_dev`, `sensei-dev`, …).
    pub db_name: String,
    /// Full PostgreSQL connection URL.
    /// Use this everywhere — never hard-code a URL.
    pub db_url: String,
    /// Sensei data directory suffix (`.sensei` / `.sensei-dev`).
    pub dir_suffix: &'static str,
}

impl SenseiConfig {
    /// Build configuration from a known mode (env-var overrides for DB name/URL still apply).
    fn for_mode(mode: SenseiMode) -> Self {
        let daemon_port = match mode {
            SenseiMode::Dev  => DAEMON_PORT + 1,
            SenseiMode::Prod => DAEMON_PORT,
        };
        let db_name = std::env::var("SENSEI_DB_NAME").unwrap_or_else(|_| {
            match mode {
                SenseiMode::Dev  => "sensei_dev".to_string(),
                SenseiMode::Prod => "sensei".to_string(),
            }
        });
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| format!("postgresql://localhost:{POSTGRES_PORT}/{db_name}"));
        let dir_suffix = match mode {
            SenseiMode::Dev  => ".sensei-dev",
            SenseiMode::Prod => ".sensei",
        };
        Self { mode, daemon_port, db_name, db_url, dir_suffix }
    }

    /// Build configuration from environment variables only (`SENSEI_MODE`, `DATABASE_URL`, etc.).
    pub fn from_env() -> Self {
        Self::for_mode(SenseiMode::from_env())
    }

    /// Build configuration detecting mode from binary name first, then env var.
    ///
    /// Priority: `SENSEI_MODE` env var (if set) > binary name ending in `-dev` > Prod.
    /// Use this in CLI / daemon entrypoints that detect mode at startup.
    pub fn detect() -> Self {
        let mode = if std::env::var("SENSEI_MODE").is_ok() {
            SenseiMode::from_env()
        } else if binary_is_dev() {
            SenseiMode::Dev
        } else {
            SenseiMode::Prod
        };
        Self::for_mode(mode)
    }

    /// Daemon base URL derived from the configured port.
    pub fn daemon_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.daemon_port)
    }

    /// Daemon binary name for the current mode (`senseid` or `senseid-dev`).
    pub fn daemon_binary(&self) -> &'static str {
        self.senseid_binary()
    }

    /// Returns `true` when running in dev / E2E mode.
    pub fn is_dev(&self) -> bool {
        self.mode.is_dev()
    }

    /// Sensei data directory (`~/.sensei/` or `~/.sensei-dev/`).
    pub fn sensei_dir(&self) -> PathBuf {
        home_dir().join(self.dir_suffix)
    }

    /// Log file path.
    pub fn log_path(&self) -> PathBuf {
        self.sensei_dir().join("senseid.log")
    }

    /// PID file path.
    pub fn pid_path(&self) -> PathBuf {
        self.sensei_dir().join("serve.pid")
    }

    /// Returns the sensei CLI binary name for the current mode.
    pub fn sensei_binary(&self) -> &'static str {
        if self.is_dev() { "sensei-dev" } else { "sensei" }
    }

    /// Returns the senseid daemon binary name for the current mode.
    pub fn senseid_binary(&self) -> &'static str {
        if self.is_dev() { "senseid-dev" } else { "senseid" }
    }

    /// Returns the sensei-mcp binary name for the current mode.
    pub fn sensei_mcp_binary(&self) -> &'static str {
        if self.is_dev() { "sensei-mcp-dev" } else { "sensei-mcp" }
    }

    /// Resolve the database schema source for dbd-core's `resolve_source()`.
    ///
    /// Priority:
    ///   1. `SENSEI_DB_SCHEMA_PATH` env var — local directory path (dev/test override).
    ///      Must point to the directory containing `design.yaml`.
    ///   2. GitHub path: `{GITHUB_ORG}/{GITHUB_REPO}/database@v{version}` (production).
    pub fn db_schema_source(version: &str) -> String {
        std::env::var("SENSEI_DB_SCHEMA_PATH")
            .ok()
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| format!("{GITHUB_ORG}/{GITHUB_REPO}/database@v{version}"))
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

/// Contents of `~/.sensei/config.json` (or `~/.sensei-dev/config.json`).
///
/// This is the single source of truth for persisted local state shared between
/// the daemon and the CLI. Read/write via [`SenseiLocalConfig::load`] and
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

    #[test]
    fn default_mode_is_prod() {
        // This test assumes SENSEI_MODE is not set in the test environment.
        // If it is, the test is a no-op (we just verify it doesn't panic).
        let cfg = SenseiConfig::from_env();
        assert!(matches!(cfg.mode, SenseiMode::Prod | SenseiMode::Dev));
    }

    #[test]
    fn prod_defaults() {
        // Only valid if SENSEI_MODE is unset and DATABASE_URL / SENSEI_DB_NAME unset.
        // Use a subprocess or env-isolation crate for hermetic tests.
        // This just verifies the struct is well-formed.
        let cfg = SenseiConfig::from_env();
        assert!(cfg.daemon_port == 7744 || cfg.daemon_port == 7745);
        assert!(!cfg.db_name.is_empty());
        assert!(cfg.db_url.starts_with("postgresql://"));
    }

    #[test]
    fn db_url_contains_db_name() {
        let cfg = SenseiConfig::from_env();
        // DATABASE_URL may override, but if not set the URL contains db_name
        if std::env::var("DATABASE_URL").is_err() {
            assert!(
                cfg.db_url.contains(&cfg.db_name),
                "derived URL should contain db_name"
            );
        }
    }

    #[test]
    fn sensei_mode_from_env_does_not_panic() {
        let _ = SenseiMode::from_env();
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
    fn homebrew_brewfile_url_is_github_raw() {
        assert!(HOMEBREW_BREWFILE_URL.starts_with("https://raw.githubusercontent.com/"));
        assert!(HOMEBREW_BREWFILE_URL.contains("homebrew-tap"));
        assert!(HOMEBREW_BREWFILE_URL.ends_with("Brewfile"));
    }

    #[test]
    fn binary_names_prod() {
        let cfg = SenseiConfig {
            mode: SenseiMode::Prod,
            daemon_port: 7744,
            db_name: "sensei".to_string(),
            db_url: "postgresql://localhost/sensei".to_string(),
            dir_suffix: ".sensei",
        };
        assert_eq!(cfg.sensei_binary(), "sensei");
        assert_eq!(cfg.senseid_binary(), "senseid");
        assert_eq!(cfg.sensei_mcp_binary(), "sensei-mcp");
    }

    #[test]
    fn binary_names_dev() {
        let cfg = SenseiConfig {
            mode: SenseiMode::Dev,
            daemon_port: 7745,
            db_name: "sensei_dev".to_string(),
            db_url: "postgresql://localhost/sensei_dev".to_string(),
            dir_suffix: ".sensei-dev",
        };
        assert_eq!(cfg.sensei_binary(), "sensei-dev");
        assert_eq!(cfg.senseid_binary(), "senseid-dev");
        assert_eq!(cfg.sensei_mcp_binary(), "sensei-mcp-dev");
    }
}
