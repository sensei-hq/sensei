//! Where the dōjō endpoint and its publishable key come from.
//!
//! The daemon runs under `brew services`, which does not inherit a login shell.
//! Reading these from the environment alone therefore works in dev — where the
//! daemon is launched from a terminal that HAS the exports — and silently fails
//! on every installed machine, reporting "not configured" for a dōjō that is
//! configured perfectly well. So the persisted config is the durable source and
//! the environment is an override for dev.

/// The cloud dōjō — the only one that exists.
///
/// dōjō has no service of its own; it IS a Supabase project, so this URL is both
/// "where dōjō lives" and "where auth happens". That is what makes sign-in work
/// with no proxy: the cloud project has the GitHub provider configured, and the
/// daemon's PKCE flow runs straight against it.
///
/// Defaulting to the cloud rather than localhost because a local instance exists
/// only for dev testing. A localhost default would leave every real install
/// pointing at a Supabase that isn't running, reporting "not configured" for a
/// dōjō that needs no configuring.
const DEFAULT_URL: &str = "https://lagwuqrtshjtlcuvjfnd.supabase.co";

/// Resolve a setting: environment first, then persisted config.
///
/// Env-first so a developer can point one run at a different instance without
/// editing (and later forgetting) a config file.
fn resolve(env_var: &str, from_config: impl FnOnce() -> Option<String>) -> Option<String> {
    std::env::var(env_var)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(from_config)
        .filter(|v| !v.trim().is_empty())
}

/// Load the persisted local config.
fn local() -> sensei_bootstrap::SenseiLocalConfig {
    sensei_bootstrap::SenseiLocalConfig::load(
        &sensei_bootstrap::SenseiConfig::from_env().sensei_dir(),
    )
}

/// The dōjō base URL.
pub fn url() -> String {
    resolve("SUPABASE_URL", || local().dojo_url).unwrap_or_else(|| DEFAULT_URL.to_string())
}

/// The publishable key, or `None` when the machine has no dōjō configured.
///
/// `None` rather than a placeholder: without a key every request is rejected,
/// and "not configured" is a state the caller must be able to report as such
/// instead of surfacing an opaque 401.
pub fn anon_key() -> Option<String> {
    resolve("SUPABASE_ANON_KEY", || local().dojo_anon_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_environment_wins_over_the_config() {
        // The dev override. Without it, pointing a single run at a different
        // instance would mean editing a file and remembering to change it back.
        let got = resolve("__SENSEI_TEST_UNSET_VAR__", || Some("from-config".into()));
        assert_eq!(got.as_deref(), Some("from-config"));

        unsafe { std::env::set_var("__SENSEI_TEST_SET_VAR__", "from-env") };
        let got = resolve("__SENSEI_TEST_SET_VAR__", || Some("from-config".into()));
        unsafe { std::env::remove_var("__SENSEI_TEST_SET_VAR__") };
        assert_eq!(got.as_deref(), Some("from-env"));
    }

    #[test]
    fn a_blank_value_is_treated_as_absent() {
        // An empty export is a common way to "unset" a variable, and an empty
        // apikey header produces a 401 that reads like a bad key rather than a
        // missing one. Falling through to the config is the useful behaviour.
        unsafe { std::env::set_var("__SENSEI_TEST_BLANK_VAR__", "   ") };
        let got = resolve("__SENSEI_TEST_BLANK_VAR__", || Some("from-config".into()));
        unsafe { std::env::remove_var("__SENSEI_TEST_BLANK_VAR__") };
        assert_eq!(got.as_deref(), Some("from-config"));
    }

    #[test]
    fn nothing_configured_yields_none_not_a_placeholder() {
        // A placeholder key would fail authentication with an opaque error. The
        // caller needs to distinguish "no dōjō set up" from "the key is wrong".
        assert_eq!(resolve("__SENSEI_TEST_UNSET_VAR__", || None), None);
        assert_eq!(resolve("__SENSEI_TEST_UNSET_VAR__", || Some(String::new())), None);
    }
}
