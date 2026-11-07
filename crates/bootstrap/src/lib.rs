//! sensei-bootstrap — generic dependency-resolution framework with a public
//! health contract.
//!
//! Public surface:
//!   * health::{HealthPayload, HealthEvent, ...} — wire types matching TS.
//!   * health::check()             — sync fast path; daemon /health uses this.
//!   * health::check_and_resolve() — streaming check + fix; sidecar uses this.
//!   * hardware::{HardwareInfo, ModelTier, detect()} — host hardware probing.
//!   * models::{list(), missing_models(), ...} — Ollama model helpers.
//!   * engines::{check(), install(), ensure()} — the indexer's compiler engines.
//!   * config::*                    — runtime config (unchanged).
//!   * util::*                      — small utilities (unchanged).

pub mod config;
pub mod database;
pub mod engines;
pub mod hardware;
pub mod health;
pub mod models;
pub mod startup_error;
pub mod tracing_init;
pub mod upgrade;
pub mod util;

pub use config::{
    BREW_PATHS, BREW_TAP, DAEMON_PORT, DB_POOL_ACQUIRE_TIMEOUT_SECS, DB_POOL_IDLE_TIMEOUT_SECS,
    DB_POOL_MAX_CONNECTIONS, DB_POOL_MAX_LIFETIME_SECS, DB_POOL_MIN_CONNECTIONS, GITHUB_ORG,
    GITHUB_REPO, HOMEBREW_TAP_REPO, HOMEBREW_TAP_URL, MARKETPLACE_RAW_URL, MARKETPLACE_REPO,
    MCP_REGISTRY_KEY, OLLAMA_PORT, POSTGRES_PORT, SENSEI_BIN, SENSEI_MCP_BIN, SENSEID_BIN,
    SenseiConfig, SenseiLocalConfig, engines_dir, home_dir,
};
pub use hardware::{HardwareInfo, ModelTier};
#[allow(unused_imports)]
pub use health::*;

/// Daemon port for the current mode.
pub fn daemon_port() -> u16 {
    SenseiConfig::from_env().daemon_port
}

/// Lazily-initialised config singleton.
pub fn config() -> &'static SenseiConfig {
    use std::sync::OnceLock;
    static CFG: OnceLock<SenseiConfig> = OnceLock::new();
    CFG.get_or_init(SenseiConfig::from_env)
}

/// Shorthand for `config().daemon_url()`.
///
/// Phase B (F1) also honors `SENSEI_DAEMON_URL` here; see `resolve_daemon_url`.
pub fn daemon_url() -> String {
    config().daemon_url()
}

/// Resolve the daemon base URL. Precedence: explicit flag value > `SENSEI_DAEMON_URL`
/// env value > built-in default. Both `host:port` and full `http://host:port` shapes
/// normalize to `http://host:port` (no trailing slash); empty/whitespace values count
/// as unset and fall through to the next level.
///
/// Phase-A stub (F1 tests r1): always returns the default so the precedence reds
/// fail. Phase B implements the chain.
pub fn resolve_daemon_url(_flag: Option<&str>, _env: Option<&str>) -> String {
    "http://127.0.0.1:7744".to_string()
}

#[cfg(test)]
mod daemon_url_tests {
    use super::resolve_daemon_url;

    const DEFAULT: &str = "http://127.0.0.1:7744";

    #[test]
    #[ignore = "pending:F1"]
    fn default_when_neither_set() {
        assert_eq!(resolve_daemon_url(None, None), DEFAULT);
    }

    #[test]
    #[ignore = "pending:F1"]
    fn env_beats_default() {
        assert_eq!(resolve_daemon_url(None, Some("127.0.0.1:7844")), "http://127.0.0.1:7844");
    }

    #[test]
    #[ignore = "pending:F1"]
    fn flag_beats_env() {
        assert_eq!(
            resolve_daemon_url(Some("127.0.0.1:9999"), Some("http://127.0.0.1:7844")),
            "http://127.0.0.1:9999"
        );
    }

    #[test]
    #[ignore = "pending:F1"]
    fn url_shape_passes_through() {
        assert_eq!(
            resolve_daemon_url(None, Some("http://127.0.0.1:7844")),
            "http://127.0.0.1:7844"
        );
    }

    #[test]
    #[ignore = "pending:F1"]
    fn host_port_shape_gets_scheme() {
        assert_eq!(resolve_daemon_url(Some("example:1"), None), "http://example:1");
    }

    #[test]
    #[ignore = "pending:F1"]
    fn trailing_slash_trimmed() {
        assert_eq!(
            resolve_daemon_url(Some("http://127.0.0.1:7844/"), None),
            "http://127.0.0.1:7844"
        );
    }

    #[test]
    #[ignore = "pending:F1"]
    fn empty_means_unset() {
        assert_eq!(resolve_daemon_url(Some(""), Some("  ")), DEFAULT);
    }

    #[test]
    #[ignore = "pending:F1"]
    fn empty_flag_falls_through_to_env() {
        assert_eq!(resolve_daemon_url(Some(""), Some("127.0.0.1:7844")), "http://127.0.0.1:7844");
    }
}
