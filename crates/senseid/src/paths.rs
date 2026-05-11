//! Shared path utilities — single source of truth for all sensei directories.
//! Used by main, installer, acp, and sensei-cli.
//!
//! Modes:
//!   prod (default) → ~/.sensei/     port 7744
//!   dev            → ~/.sensei-dev/  port 7745
//!
//! Set mode via SENSEI_MODE=dev env var or --mode dev CLI flag.

use std::path::PathBuf;
use std::sync::OnceLock;

pub use sensei_bootstrap::SenseiMode;

// ── GitHub org/repo constants — re-exported from bootstrap (single source of truth) ────

pub use sensei_bootstrap::config::{
    BREW_TAP, GITHUB_ORG, GITHUB_REPO, MARKETPLACE_RAW_URL, MARKETPLACE_REPO,
};

static MODE: OnceLock<SenseiMode> = OnceLock::new();

/// Set the runtime mode. Must be called once at startup before any path access.
pub fn set_mode(mode: SenseiMode) {
    MODE.set(mode).ok(); // ignore if already set
}

/// Get the current mode. Defaults to Prod.
pub fn mode() -> SenseiMode {
    *MODE.get().unwrap_or(&SenseiMode::Prod)
}

/// Default port for the current mode.
pub fn default_port() -> u16 {
    match mode() {
        SenseiMode::Prod => 7744,
        SenseiMode::Dev  => 7745,
    }
}

/// Directory suffix for the current mode.
fn dir_name() -> &'static str {
    match mode() {
        SenseiMode::Prod => ".sensei",
        SenseiMode::Dev  => ".sensei-dev",
    }
}

/// User's home directory. Panics if HOME is not set — see [`sensei_bootstrap::home_dir`].
pub fn home() -> PathBuf {
    sensei_bootstrap::home_dir()
}

/// Sensei data directory: ~/.sensei/ (prod) or ~/.sensei-dev/ (dev)
pub fn sensei_dir() -> PathBuf {
    home().join(dir_name())
}

/// Log file
pub fn log_path() -> PathBuf {
    sensei_dir().join("senseid.log")
}

/// PID file
pub fn pid_path() -> PathBuf {
    sensei_dir().join("serve.pid")
}

/// Claude plugins directory: ~/.claude/plugins/sensei/
pub fn plugin_dir() -> PathBuf {
    home().join(".claude/plugins/sensei")
}

/// Marketplace cache directory
pub fn cache_dir() -> PathBuf {
    sensei_dir().join("cache/marketplace")
}

/// Initialize mode from environment variable SENSEI_MODE.
/// Delegates to [`sensei_bootstrap::SenseiConfig`] for consistent mode detection.
/// Call this at startup before any path access.
pub fn init_from_env() {
    set_mode(sensei_bootstrap::SenseiConfig::from_env().mode);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_is_not_empty() {
        let h = home();
        assert!(!h.as_os_str().is_empty());
    }

    #[test]
    fn prod_mode_uses_sensei_dir() {
        // Note: can't test set_mode in parallel tests due to OnceLock
        // but we can verify the dir_name logic
        assert_eq!(dir_name(), ".sensei"); // default is prod
    }

    #[test]
    fn default_port_is_7744() {
        assert_eq!(default_port(), 7744);
    }

    #[test]
    fn all_paths_are_absolute() {
        assert!(home().is_absolute() || home() == PathBuf::from("/tmp"));
        assert!(sensei_dir().is_absolute() || sensei_dir().starts_with("/tmp"));
    }
}
