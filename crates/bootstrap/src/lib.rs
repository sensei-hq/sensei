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

/// The daemon base URL: `--daemon-url` flag > `SENSEI_DAEMON_URL` env >
/// built-in default. See [`resolve_daemon_url`] for the accepted shapes.
/// With neither set this is byte-identical to the historical default.
pub fn daemon_url() -> String {
    daemon_url_with_flag(None)
}

/// [`daemon_url`] with an explicit `--daemon-url` flag value. The single
/// owner of the `SENSEI_DAEMON_URL` read — MCP and CLI call this rather than
/// reading the env var themselves, so the variable name appears once.
pub fn daemon_url_with_flag(flag: Option<&str>) -> String {
    let env = std::env::var("SENSEI_DAEMON_URL").ok();
    resolve_daemon_url(flag, env.as_deref())
}

/// Resolve the daemon base URL. Precedence: explicit flag value > `SENSEI_DAEMON_URL`
/// env value > built-in default (`http://127.0.0.1:7744`).
///
/// Accepted shapes (normalized one way to `http://host:port`, no trailing slash):
/// `host:port` gains the `http://` scheme; a value already carrying a scheme
/// (`://`) passes through with any trailing `/` trimmed. Empty/whitespace
/// values count as unset and fall through to the next level.
pub fn resolve_daemon_url(flag: Option<&str>, env: Option<&str>) -> String {
    let picked = [flag, env].into_iter().flatten().map(str::trim).find(|s| !s.is_empty());
    match picked {
        None => format!("http://127.0.0.1:{DAEMON_PORT}"),
        Some(value) => {
            let value = value.trim_end_matches('/');
            if value.contains("://") { value.to_string() } else { format!("http://{value}") }
        }
    }
}

#[cfg(test)]
mod daemon_url_tests {
    use super::resolve_daemon_url;

    const DEFAULT: &str = "http://127.0.0.1:7744";

    #[test]
    fn default_when_neither_set() {
        assert_eq!(resolve_daemon_url(None, None), DEFAULT);
    }

    #[test]
    fn env_beats_default() {
        assert_eq!(resolve_daemon_url(None, Some("127.0.0.1:7844")), "http://127.0.0.1:7844");
    }

    #[test]
    fn flag_beats_env() {
        assert_eq!(
            resolve_daemon_url(Some("127.0.0.1:9999"), Some("http://127.0.0.1:7844")),
            "http://127.0.0.1:9999"
        );
    }

    #[test]
    fn url_shape_passes_through() {
        assert_eq!(
            resolve_daemon_url(None, Some("http://127.0.0.1:7844")),
            "http://127.0.0.1:7844"
        );
    }

    #[test]
    fn host_port_shape_gets_scheme() {
        assert_eq!(resolve_daemon_url(Some("example:1"), None), "http://example:1");
    }

    #[test]
    fn trailing_slash_trimmed() {
        assert_eq!(
            resolve_daemon_url(Some("http://127.0.0.1:7844/"), None),
            "http://127.0.0.1:7844"
        );
    }

    #[test]
    fn empty_means_unset() {
        assert_eq!(resolve_daemon_url(Some(""), Some("  ")), DEFAULT);
    }

    #[test]
    fn empty_flag_falls_through_to_env() {
        assert_eq!(resolve_daemon_url(Some(""), Some("127.0.0.1:7844")), "http://127.0.0.1:7844");
    }
}
