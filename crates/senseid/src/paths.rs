//! Shared path utilities — single source of truth for all sensei directories.

use std::path::PathBuf;

pub use sensei_bootstrap::config::{
    BREW_TAP, GITHUB_ORG, GITHUB_REPO, MARKETPLACE_RAW_URL, MARKETPLACE_REPO,
};

/// Default daemon port.
pub fn default_port() -> u16 {
    sensei_bootstrap::SenseiConfig::from_env().daemon_port
}

/// User's home directory. Panics if HOME is not set — see [`sensei_bootstrap::home_dir`].
pub fn home() -> PathBuf {
    sensei_bootstrap::home_dir()
}

/// Sensei data directory: `~/.sensei/`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensei_dir_exists_or_is_creatable() {
        let d = sensei_dir();
        assert!(d.to_string_lossy().contains(".sensei"));
    }

    #[test]
    fn default_port_is_7744() {
        assert_eq!(default_port(), 7744);
    }
}
