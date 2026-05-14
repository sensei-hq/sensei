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

/// Raw GitHub URL for the prod homebrew tap Brewfile.
pub const HOMEBREW_BREWFILE_URL: &str =
    "https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile";

/// Raw GitHub URL for the dev homebrew tap Brewfile.
/// Used by dev (`--features dev`) builds to install `*-dev` binaries.
pub const HOMEBREW_BREWFILE_DEV_URL: &str =
    "https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile-dev";

// ── Compile-time binary names (single source of truth) ────────────────────────
//
// These exist as `const &str` so they can be used at attribute / macro time —
// e.g. clap's `#[command(name = SENSEI_BIN, ...)]`. Runtime callers should use
// the equivalent `SenseiConfig::sensei_binary()` etc. accessors.

/// Compile-time `sensei` CLI binary name for the current build mode.
#[cfg(feature = "dev")]
pub const SENSEI_BIN: &str = "sensei-dev";
#[cfg(not(feature = "dev"))]
pub const SENSEI_BIN: &str = "sensei";

/// Compile-time `senseid` daemon binary name for the current build mode.
#[cfg(feature = "dev")]
pub const SENSEID_BIN: &str = "senseid-dev";
#[cfg(not(feature = "dev"))]
pub const SENSEID_BIN: &str = "senseid";

/// Compile-time `sensei-mcp` server binary name for the current build mode.
#[cfg(feature = "dev")]
pub const SENSEI_MCP_BIN: &str = "sensei-mcp-dev";
#[cfg(not(feature = "dev"))]
pub const SENSEI_MCP_BIN: &str = "sensei-mcp";

/// MCP server registry key for the current build mode. Used by sensei CLI and
/// senseid daemon when registering / removing the MCP entry in ACP configs
/// (Claude Code, Cursor, etc.) — dev runs register as a distinct key so dev
/// and prod can coexist in the same ACP without colliding.
#[cfg(feature = "dev")]
pub const MCP_REGISTRY_KEY: &str = "sensei-dev";
#[cfg(not(feature = "dev"))]
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

/// Compile-time mode flag. Set by the `dev` Cargo feature.
/// `true` when built with `cargo build --features dev`.
#[cfg(feature = "dev")]
pub const COMPILE_DEV: bool = true;
#[cfg(not(feature = "dev"))]
pub const COMPILE_DEV: bool = false;

/// Default daemon port — derived at compile time from the `dev` feature.
const DAEMON_PORT: u16 = if COMPILE_DEV { 7745 } else { 7744 };

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
    /// Development / E2E mode: port 7745, `sensei_dev` DB, `~/.sensei-dev/`
    Dev,
}

impl SenseiMode {
    /// Compile-time mode. Determined by the `dev` Cargo feature — not env vars.
    pub fn from_env() -> Self {
        if COMPILE_DEV { SenseiMode::Dev } else { SenseiMode::Prod }
    }

    pub fn is_dev(self) -> bool {
        self == SenseiMode::Dev
    }
}

/// Compile-time configuration for all sensei components.
///
/// All values are determined by the `dev` Cargo feature at build time.
/// No runtime env var overrides.
///
/// | Feature       | Port  | Database     | Directory       |
/// |---------------|-------|-------------|-----------------|
/// | `--features dev` | 7745  | `sensei_dev` | `~/.sensei-dev/` |
/// | *(default)*   | 7744  | `sensei`     | `~/.sensei/`    |
#[derive(Debug, Clone)]
pub struct SenseiConfig {
    pub mode: SenseiMode,
    pub daemon_port: u16,
    pub db_name: String,
    pub db_url: String,
    pub dir_suffix: &'static str,
}

impl SenseiConfig {
    /// Build configuration. All values derived from the compile-time `dev` Cargo feature.
    /// No runtime env var overrides — what you compiled is what you get.
    pub fn from_env() -> Self {
        let mode = SenseiMode::from_env();
        let daemon_port = DAEMON_PORT;
        let db_name = if COMPILE_DEV { "sensei_dev".to_string() } else { "sensei".to_string() };
        let db_url = format!("postgresql://localhost:{POSTGRES_PORT}/{db_name}");
        let dir_suffix = if COMPILE_DEV { ".sensei-dev" } else { ".sensei" };
        Self { mode, daemon_port, db_name, db_url, dir_suffix }
    }

    /// Alias for `from_env()`.
    #[deprecated(note = "Use from_env() — mode is compile-time via Cargo features")]
    pub fn detect() -> Self {
        Self::from_env()
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
    pub fn sensei_binary(&self) -> &'static str { SENSEI_BIN }

    /// Returns the senseid daemon binary name for the current mode.
    pub fn senseid_binary(&self) -> &'static str { SENSEID_BIN }

    /// Returns the Brewfile URL appropriate to the current build mode —
    /// prod Brewfile for release builds, Brewfile-dev for `--features dev`.
    pub fn brewfile_url(&self) -> &'static str {
        if self.is_dev() { HOMEBREW_BREWFILE_DEV_URL } else { HOMEBREW_BREWFILE_URL }
    }

    /// Returns the full `brew bundle --file=<url>` script that installs
    /// the current-mode binaries. Suitable for direct copy/paste in a shell
    /// or for display in a [`Remedy`].
    pub fn brew_bundle_script(&self) -> String {
        format!("brew bundle --file={}", self.brewfile_url())
    }

    /// Returns the sensei-mcp binary name for the current mode.
    pub fn sensei_mcp_binary(&self) -> &'static str { SENSEI_MCP_BIN }

    /// Returns the MCP registry key for the current mode — `"sensei"` in prod,
    /// `"sensei-dev"` in dev. Used so dev and prod can coexist in the same
    /// ACP config without overwriting each other.
    pub fn mcp_registry_key(&self) -> &'static str { MCP_REGISTRY_KEY }

    /// Resolve the database schema source for dbd-core's `resolve_source()`.
    ///
    /// Priority:
    /// Schema source for database deployment.
    /// Dev builds: local `database/` directory via `SENSEI_DB_SCHEMA_PATH` if set.
    /// Prod builds: GitHub download at matching release tag.
    pub fn db_schema_source(version: &str) -> String {
        // Dev builds always look for a local schema path first.
        // The Tauri sidecar sets SENSEI_DB_SCHEMA_PATH; standalone daemon does not.
        if COMPILE_DEV {
            if let Ok(path) = std::env::var("SENSEI_DB_SCHEMA_PATH") {
                if !path.is_empty() {
                    return path;
                }
            }
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
    fn mode_matches_compile_flag() {
        let cfg = SenseiConfig::from_env();
        if COMPILE_DEV {
            assert_eq!(cfg.mode, SenseiMode::Dev);
            assert_eq!(cfg.daemon_port, 7745);
            assert_eq!(cfg.db_name, "sensei_dev");
            assert_eq!(cfg.dir_suffix, ".sensei-dev");
        } else {
            assert_eq!(cfg.mode, SenseiMode::Prod);
            assert_eq!(cfg.daemon_port, 7744);
            assert_eq!(cfg.db_name, "sensei");
            assert_eq!(cfg.dir_suffix, ".sensei");
        }
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
        assert_eq!(OLLAMA_PORT, 11434);
        assert_eq!(POSTGRES_PORT, 5432);
        // DAEMON_PORT depends on compile-time feature
        if COMPILE_DEV {
            assert_eq!(DAEMON_PORT, 7745);
        } else {
            assert_eq!(DAEMON_PORT, 7744);
        }
    }

    #[test]
    fn homebrew_brewfile_url_is_github_raw() {
        assert!(HOMEBREW_BREWFILE_URL.starts_with("https://raw.githubusercontent.com/"));
        assert!(HOMEBREW_BREWFILE_URL.contains("homebrew-tap"));
        assert!(HOMEBREW_BREWFILE_URL.ends_with("Brewfile"));
    }

    #[test]
    fn binary_names_match_mode() {
        let cfg = SenseiConfig::from_env();
        if COMPILE_DEV {
            assert_eq!(cfg.sensei_binary(), "sensei-dev");
            assert_eq!(cfg.senseid_binary(), "senseid-dev");
            assert_eq!(cfg.sensei_mcp_binary(), "sensei-mcp-dev");
        } else {
            assert_eq!(cfg.sensei_binary(), "sensei");
            assert_eq!(cfg.senseid_binary(), "senseid");
            assert_eq!(cfg.sensei_mcp_binary(), "sensei-mcp");
        }
    }

    #[test]
    fn brewfile_dev_url_is_github_raw() {
        assert!(HOMEBREW_BREWFILE_DEV_URL.starts_with("https://raw.githubusercontent.com/"));
        assert!(HOMEBREW_BREWFILE_DEV_URL.contains("homebrew-tap"));
        assert!(HOMEBREW_BREWFILE_DEV_URL.ends_with("Brewfile-dev"));
    }

    #[test]
    fn brewfile_url_matches_mode() {
        let cfg = SenseiConfig::from_env();
        if COMPILE_DEV {
            assert_eq!(cfg.brewfile_url(), HOMEBREW_BREWFILE_DEV_URL);
        } else {
            assert_eq!(cfg.brewfile_url(), HOMEBREW_BREWFILE_URL);
        }
    }

    #[test]
    fn brew_bundle_script_uses_mode_url() {
        let cfg = SenseiConfig::from_env();
        let script = cfg.brew_bundle_script();
        assert!(script.starts_with("brew bundle --file="));
        assert!(script.contains(cfg.brewfile_url()));
        // Single `=` after --file, not the broken `--file==URL` form.
        assert!(!script.contains("--file=="));
    }

    #[test]
    fn compile_time_binary_consts_match_runtime_accessors() {
        let cfg = SenseiConfig::from_env();
        assert_eq!(SENSEI_BIN, cfg.sensei_binary());
        assert_eq!(SENSEID_BIN, cfg.senseid_binary());
        assert_eq!(SENSEI_MCP_BIN, cfg.sensei_mcp_binary());
    }

    #[test]
    fn mcp_registry_key_matches_mode() {
        let cfg = SenseiConfig::from_env();
        if COMPILE_DEV {
            assert_eq!(MCP_REGISTRY_KEY, "sensei-dev");
        } else {
            assert_eq!(MCP_REGISTRY_KEY, "sensei");
        }
        assert_eq!(MCP_REGISTRY_KEY, cfg.mcp_registry_key());
    }
}
