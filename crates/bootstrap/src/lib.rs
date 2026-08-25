//! sensei-bootstrap — generic dependency-resolution framework with a public
//! health contract.
//!
//! Public surface:
//!   * health::{HealthPayload, HealthEvent, ...} — wire types matching TS.
//!   * health::check()             — sync fast path; daemon /health uses this.
//!   * health::check_and_resolve() — streaming check + fix; sidecar uses this.
//!   * hardware::{HardwareInfo, ModelTier, detect()} — host hardware probing.
//!   * models::{list(), missing_models(), ...} — Ollama model helpers.
//!   * config::*                    — runtime config (unchanged).
//!   * util::*                      — small utilities (unchanged).

pub mod config;
pub mod database;
pub mod hardware;
pub mod health;
pub mod models;
pub mod tracing_init;
pub mod upgrade;
pub mod util;

pub use config::{
    BREW_PATHS, BREW_TAP, DAEMON_PORT, DB_POOL_ACQUIRE_TIMEOUT_SECS, DB_POOL_IDLE_TIMEOUT_SECS,
    DB_POOL_MAX_CONNECTIONS, DB_POOL_MAX_LIFETIME_SECS, DB_POOL_MIN_CONNECTIONS, GITHUB_ORG,
    GITHUB_REPO, HOMEBREW_TAP_REPO, HOMEBREW_TAP_URL, MARKETPLACE_RAW_URL, MARKETPLACE_REPO,
    MCP_REGISTRY_KEY, OLLAMA_PORT, POSTGRES_PORT, SENSEI_BIN, SENSEI_MCP_BIN, SENSEID_BIN,
    SenseiConfig, SenseiLocalConfig, home_dir,
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
pub fn daemon_url() -> String {
    config().daemon_url()
}
