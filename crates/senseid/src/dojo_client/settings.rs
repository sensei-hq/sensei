//! Where dōjō is.
//!
//! One setting, deliberately. Sign-in runs through dōjō's `/v1/auth/cli/*`
//! endpoints, so the daemon needs no identity-provider URL, no publishable key,
//! and no knowledge of how dōjō authenticates people. Those are dōjō's business
//! and can change without touching an installed machine.
//!
//! Persisted rather than read from the environment alone, because the daemon runs
//! under `brew services`, which does not inherit a login shell — a value exported
//! in a shell profile is invisible to it. That is how auth came to report "not
//! configured" on installed machines while working perfectly in dev.

/// The dōjō everyone uses.
///
/// A local dōjō exists only for dev testing, so defaulting to localhost would
/// leave every real install pointing at a site that isn't running.
const DEFAULT_DOJO_URL: &str = "https://dojo.sensei-hq.com";

/// The dōjō base URL, without a trailing slash.
///
/// `DOJO_URL` overrides it so a dev build can point at a local instance without
/// editing (and later forgetting) a config file.
pub fn dojo_url() -> String {
    let configured = std::env::var("DOJO_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| local().dojo_url)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_DOJO_URL.to_string());
    normalize(&configured)
}

/// Trim trailing slashes so callers can join paths without doubling them up.
///
/// Config values routinely carry one, and `//v1/auth/...` does not route.
fn normalize(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

/// Load the persisted local config.
fn local() -> sensei_bootstrap::SenseiLocalConfig {
    sensei_bootstrap::SenseiLocalConfig::load(
        &sensei_bootstrap::SenseiConfig::from_env().sensei_dir(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trailing_slash_is_trimmed() {
        // Callers append `/v1/auth/cli/...`; a doubled slash does not route, and
        // the failure reads as a missing endpoint rather than a config typo.
        assert_eq!(normalize("https://dojo.sensei-hq.com/"), "https://dojo.sensei-hq.com");
        assert_eq!(normalize("https://dojo.sensei-hq.com///"), "https://dojo.sensei-hq.com");
        assert_eq!(normalize("  https://dojo.sensei-hq.com  "), "https://dojo.sensei-hq.com");
    }

    #[test]
    fn the_default_is_the_cloud_dojo() {
        // A local dōjō is a dev-testing convenience; defaulting to it would leave
        // every real install pointing at a site that isn't running.
        assert_eq!(normalize(DEFAULT_DOJO_URL), "https://dojo.sensei-hq.com");
    }
}
