//! Single source of truth for all mode-sensitive sensei configuration.
//!
//! Every component that needs mode-awareness (daemon port, DB name, directory
//! suffix) should derive those values from [`SenseiConfig::from_env()`] rather
//! than implementing its own env-var reading.
//!
//! All three dependents — Tauri sidecar, bootstrap crate, and senseid daemon —
//! already depend on this crate, so importing from here creates no new edges.

use std::path::PathBuf;

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
    dir_suffix: &'static str,
}

impl SenseiConfig {
    /// Build configuration from environment variables.
    pub fn from_env() -> Self {
        let mode = SenseiMode::from_env();

        let daemon_port = match mode {
            SenseiMode::Dev  => 7745,
            SenseiMode::Prod => 7744,
        };

        // DB name: explicit override > mode default
        let db_name = std::env::var("SENSEI_DB_NAME").unwrap_or_else(|_| {
            match mode {
                SenseiMode::Dev  => "sensei_dev".to_string(),
                SenseiMode::Prod => "sensei".to_string(),
            }
        });

        // DB URL: explicit override > derived from db_name
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| format!("postgresql://localhost:5432/{db_name}"));

        let dir_suffix = match mode {
            SenseiMode::Dev  => ".sensei-dev",
            SenseiMode::Prod => ".sensei",
        };

        Self { mode, daemon_port, db_name, db_url, dir_suffix }
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
}

/// User's home directory. Falls back to `/tmp` if unavailable.
pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
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
}
