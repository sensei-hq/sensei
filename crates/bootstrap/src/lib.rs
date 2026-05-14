//! sensei-bootstrap — generic dependency-resolution framework with a public
//! health contract.
//!
//! Public surface:
//!   * health::{HealthPayload, HealthEvent, ...} — wire types matching TS.
//!   * health::check()             — sync fast path; daemon /health uses this.
//!   * health::check_and_resolve() — streaming check + fix; sidecar uses this.
//!   * config::*                    — runtime config (unchanged).
//!   * util::*                      — small utilities (unchanged).

pub mod config;
pub mod util;
pub mod health;

pub use config::{
    SenseiConfig, SenseiMode, SenseiLocalConfig,
    COMPILE_DEV, home_dir,
    BREW_PATHS, BREW_TAP, GITHUB_ORG, GITHUB_REPO,
    HOMEBREW_BREWFILE_URL, HOMEBREW_TAP_REPO, HOMEBREW_TAP_URL,
    MARKETPLACE_RAW_URL, MARKETPLACE_REPO,
    OLLAMA_PORT, POSTGRES_PORT,
    DB_POOL_MAX_CONNECTIONS, DB_POOL_ACQUIRE_TIMEOUT_SECS, DB_POOL_IDLE_TIMEOUT_SECS,
};
#[allow(unused_imports)]
pub use health::*;

/// Daemon port for the current mode.
pub fn daemon_port() -> u16 { SenseiConfig::from_env().daemon_port }

/// Lazily-initialised config singleton.
pub fn config() -> &'static SenseiConfig {
    use std::sync::OnceLock;
    static CFG: OnceLock<SenseiConfig> = OnceLock::new();
    CFG.get_or_init(SenseiConfig::from_env)
}

/// Shorthand for `config().daemon_url()`.
pub fn daemon_url() -> String { config().daemon_url() }
