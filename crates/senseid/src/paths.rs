//! Shared path utilities — single source of truth for all sensei directories.
//!
//! Mode is compile-time via the `dev` Cargo feature:
//!   prod (default) → ~/.sensei/     port 7744
//!   dev            → ~/.sensei-dev/  port 7745

use std::path::PathBuf;

pub use sensei_bootstrap::SenseiMode;

// ── GitHub org/repo constants — re-exported from bootstrap (single source of truth) ────

pub use sensei_bootstrap::config::{
    BREW_TAP, GITHUB_ORG, GITHUB_REPO, MARKETPLACE_RAW_URL, MARKETPLACE_REPO,
};

/// Get the compile-time mode.
pub fn mode() -> SenseiMode {
    SenseiMode::from_env()
}

/// Default port for the current mode.
pub fn default_port() -> u16 {
    sensei_bootstrap::SenseiConfig::from_env().daemon_port
}

/// User's home directory. Panics if HOME is not set — see [`sensei_bootstrap::home_dir`].
pub fn home() -> PathBuf {
    sensei_bootstrap::home_dir()
}

/// Sensei data directory: ~/.sensei/ (prod) or ~/.sensei-dev/ (dev)
pub fn sensei_dir() -> PathBuf {
    sensei_bootstrap::SenseiConfig::from_env().sensei_dir()
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

/// No-op. Mode is compile-time. Kept for backward compatibility.
#[deprecated(note = "Mode is compile-time via Cargo features — no init needed")]
pub fn init_from_env() {}

/// No-op. Mode is compile-time.
#[deprecated(note = "Mode is compile-time via Cargo features — no set needed")]
pub fn set_mode(_mode: SenseiMode) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_matches_compile_flag() {
        let m = mode();
        if sensei_bootstrap::COMPILE_DEV {
            assert_eq!(m, SenseiMode::Dev);
        } else {
            assert_eq!(m, SenseiMode::Prod);
        }
    }

    #[test]
    fn sensei_dir_exists_or_is_creatable() {
        let d = sensei_dir();
        assert!(d.to_string_lossy().contains(".sensei"));
    }
}
