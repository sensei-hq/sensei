//! Lightweight, dependency-free sensei configuration values.
//!
//! Companion to [`sensei_bootstrap::config::SenseiConfig`] for configuration
//! that must stay free of the daemon's heavy runtime dependencies. Every value
//! here follows the same env-override-with-default discipline as
//! `SenseiConfig::db_schema_source` / `SENSEI_DDL_DIR`: read at runtime so a
//! single binary supports both the default and an override without a rebuild.
//!
//! Currently this exposes the Dōjō registry base URL — the localhost service
//! registry the daemon links to (Fork 1: `dojo.*` lives in the Dōjō service's
//! own Postgres; the daemon only talks HTTP to it, one config value here).

/// Default Dōjō registry base URL. The localhost dev registry — matches the
/// console's `PUBLIC_DOJO_API_URL` and the `_dojo-build-plan.md` default.
pub const DEFAULT_DOJO_REGISTRY_URL: &str = "http://localhost:7755";

/// Env var overriding [`DEFAULT_DOJO_REGISTRY_URL`]. Mirrors `SENSEI_DDL_DIR`.
pub const DOJO_REGISTRY_URL_ENV: &str = "SENSEI_DOJO_URL";

/// Resolve the Dōjō registry base URL.
///
/// `$SENSEI_DOJO_URL` when set and non-empty (trailing slash trimmed so callers
/// can safely `format!("{base}/v1/...")`), else [`DEFAULT_DOJO_REGISTRY_URL`].
pub fn dojo_registry_url() -> String {
    std::env::var(DOJO_REGISTRY_URL_ENV)
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_DOJO_REGISTRY_URL.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dojo_registry_url_default_override_and_empty_fallback() {
        // Combined into one test because cargo runs tests in parallel and these
        // branches all mutate the shared SENSEI_DOJO_URL env var — same idiom as
        // `SenseiConfig::db_schema_source_default_and_override`.
        //
        // SAFETY: process-global env mutation. Safe here because no other test in
        // this crate reads SENSEI_DOJO_URL and the var is cleared before return.

        // 1. Default — no env var set.
        unsafe {
            std::env::remove_var(DOJO_REGISTRY_URL_ENV);
        }
        assert_eq!(dojo_registry_url(), "http://localhost:7755", "default → localhost registry");

        // 2. Override — SENSEI_DOJO_URL set (trailing slash trimmed).
        unsafe {
            std::env::set_var(DOJO_REGISTRY_URL_ENV, "https://dojo.sensei-hq.org/");
        }
        assert_eq!(
            dojo_registry_url(),
            "https://dojo.sensei-hq.org",
            "override + trailing slash trimmed"
        );

        // 3. Empty value — falls back to default.
        unsafe {
            std::env::set_var(DOJO_REGISTRY_URL_ENV, "   ");
        }
        assert_eq!(dojo_registry_url(), "http://localhost:7755", "blank falls back to default");

        unsafe {
            std::env::remove_var(DOJO_REGISTRY_URL_ENV);
        }
    }
}
