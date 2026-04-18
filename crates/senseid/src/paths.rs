//! Shared path utilities — single source of truth for all sensei directories.
//! Used by main, installer, acp, and sensei-cli.

use std::path::PathBuf;

/// User's home directory. Falls back to /tmp if unavailable.
pub fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// Sensei data directory: ~/.sensei/
pub fn sensei_dir() -> PathBuf {
    home().join(".sensei")
}

/// SQLite database path: ~/.sensei/sensei.db
pub fn db_path() -> PathBuf {
    sensei_dir().join("sensei.db")
}

/// Graph database directory: ~/.sensei/graph/
pub fn graph_dir() -> PathBuf {
    sensei_dir().join("graph")
}

/// Log file: ~/.sensei/senseid.log
pub fn log_path() -> PathBuf {
    sensei_dir().join("senseid.log")
}

/// PID file: ~/.sensei/serve.pid
pub fn pid_path() -> PathBuf {
    sensei_dir().join("serve.pid")
}

/// Claude plugins directory: ~/.claude/plugins/sensei/
pub fn plugin_dir() -> PathBuf {
    home().join(".claude/plugins/sensei")
}

/// Marketplace cache directory: ~/.sensei/cache/marketplace/
pub fn cache_dir() -> PathBuf {
    sensei_dir().join("cache/marketplace")
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
    fn sensei_dir_under_home() {
        let sd = sensei_dir();
        assert!(sd.to_string_lossy().contains(".sensei"));
        assert!(sd.starts_with(home()));
    }

    #[test]
    fn db_path_under_sensei_dir() {
        let db = db_path();
        assert!(db.starts_with(sensei_dir()));
        assert!(db.to_string_lossy().ends_with("sensei.db"));
    }

    #[test]
    fn graph_dir_under_sensei_dir() {
        let gd = graph_dir();
        assert!(gd.starts_with(sensei_dir()));
        assert!(gd.to_string_lossy().ends_with("graph"));
    }

    #[test]
    fn plugin_dir_under_claude() {
        let pd = plugin_dir();
        assert!(pd.to_string_lossy().contains(".claude/plugins/sensei"));
    }

    #[test]
    fn cache_dir_under_sensei_dir() {
        let cd = cache_dir();
        assert!(cd.starts_with(sensei_dir()));
    }

    #[test]
    fn all_paths_are_absolute() {
        assert!(home().is_absolute() || home() == PathBuf::from("/tmp"));
        assert!(sensei_dir().is_absolute() || sensei_dir().starts_with("/tmp"));
    }
}
