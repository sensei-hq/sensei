//! OS credential store for every secret the daemon keeps — gateway router API
//! keys, dōjō device tokens, Supabase / GitHub session tokens.
//!
//! Secrets live in the platform store, never Postgres; the database holds only a
//! handle (`credential_ref`). The backend is the `keyring` crate on every OS (see
//! [`keyring_store`]). On macOS the original `/usr/bin/security` backend
//! ([`macos_security`]) remains selectable as a rollback with
//! `SENSEI_CREDENTIAL_BACKEND=security`.
//!
//! Router keys use service `com.sensei.gateway.router.<router_id>`, account
//! `"default"` (one key per router for now; per-project overrides would use
//! `account = project_id` later).
//!
//! # Blocking
//! Every call talks to the OS store synchronously (a D-Bus round trip, a
//! Keychain call, or a spawned process). Callers in an async context must wrap
//! them in `tokio::task::spawn_blocking`.

mod keyring_store;
#[cfg(target_os = "macos")]
mod macos_security;

use std::sync::OnceLock;

const ACCOUNT: &str = "default";

/// Environment variable that picks the backend. Unset means `keyring`.
const BACKEND_ENV: &str = "SENSEI_CREDENTIAL_BACKEND";

fn service_name(router_id: &str) -> String {
    format!("com.sensei.gateway.router.{router_id}")
}

#[derive(Debug, thiserror::Error)]
pub enum KeychainError {
    #[error("key not found in keychain")]
    NotFound,
    /// The store exists but cannot be used right now — no Secret Service
    /// running, a locked collection, access denied. Distinct from `NotFound` so a
    /// caller never reads "could not look" as "nothing stored".
    #[error("credential store unavailable: {0}")]
    Unavailable(String),
    #[error("credential store error: {0}")]
    Store(String),
    /// The OS refused without asking: this binary is not trusted on the item
    /// and the daemon does not prompt (#283). Approve it from sensei, or sign
    /// local builds with `make dev-signing-cert` so the trust survives a rebuild.
    #[error("keychain access needs approval")]
    NeedsApproval,
    #[error("keychain command failed: {0}")]
    CommandFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Keyring,
    #[cfg(target_os = "macos")]
    MacosSecurity,
}

/// Resolve the backend from the env value. Pure, so the choice is testable.
///
/// An unrecognised value is not silently ignored: it is warned about and the
/// default is used, because refusing to start over a rollback switch would turn
/// a typo into a daemon that cannot read any credential.
fn parse_backend(value: Option<&str>) -> Backend {
    match value.map(str::trim) {
        None | Some("") | Some("keyring") => Backend::Keyring,
        #[cfg(target_os = "macos")]
        Some("security") => Backend::MacosSecurity,
        Some(other) => {
            tracing::warn!(
                value = other,
                "{BACKEND_ENV} is not a backend available on this platform; using keyring"
            );
            Backend::Keyring
        }
    }
}

fn backend() -> Backend {
    static BACKEND: OnceLock<Backend> = OnceLock::new();
    *BACKEND.get_or_init(|| parse_backend(std::env::var(BACKEND_ENV).ok().as_deref()))
}

/// Run `f` with keychain prompts allowed on this thread (#283).
///
/// The daemon never prompts by default: a background pass that waits on a
/// dialog nobody is watching stops its worker. Only an action the user just
/// took — signing in or out, saving a router key — may ask.
pub fn allow_prompt<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            PROMPT_ALLOWED.set(self.0);
        }
    }
    let _restore = Restore(PROMPT_ALLOWED.replace(true));
    f()
}

thread_local! {
    static PROMPT_ALLOWED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether the current thread may prompt — false outside [`allow_prompt`].
pub(crate) fn prompting_allowed() -> bool {
    PROMPT_ALLOWED.get()
}

/// Store a secret, replacing any existing value for `(service, account)`.
pub fn keychain_set(service: &str, account: &str, secret: &str) -> Result<(), KeychainError> {
    match backend() {
        Backend::Keyring => keyring_store::set(service, account, secret),
        #[cfg(target_os = "macos")]
        Backend::MacosSecurity => macos_security::set(service, account, secret),
    }
}

/// Read a secret. `NotFound` only when nothing is stored; a store that cannot be
/// reached is an error of its own.
pub fn keychain_get(service: &str, account: &str) -> Result<String, KeychainError> {
    match backend() {
        Backend::Keyring => keyring_store::get(service, account),
        #[cfg(target_os = "macos")]
        Backend::MacosSecurity => macos_security::get(service, account),
    }
}

/// Remove a secret. A missing entry is success.
pub fn keychain_delete(service: &str, account: &str) -> Result<(), KeychainError> {
    match backend() {
        Backend::Keyring => keyring_store::delete(service, account),
        #[cfg(target_os = "macos")]
        Backend::MacosSecurity => macos_security::delete(service, account),
    }
}

pub fn set_key(router_id: &str, key: &str) -> Result<(), KeychainError> {
    keychain_set(&service_name(router_id), ACCOUNT, key)
}

/// Read a router key. Returns NotFound if absent.
pub fn get_key(router_id: &str) -> Result<String, KeychainError> {
    keychain_get(&service_name(router_id), ACCOUNT)
}

/// Delete a router key. Returns Ok(()) whether or not it existed.
pub fn delete_key(router_id: &str) -> Result<(), KeychainError> {
    keychain_delete(&service_name(router_id), ACCOUNT)
}

/// True when a key exists for this router. Cheap check used by the
/// /api/gateway/routers endpoint to compute `configured`.
pub fn has_key(router_id: &str) -> bool {
    match get_key(router_id) {
        Ok(_) => true,
        Err(KeychainError::NotFound) => false,
        Err(e) => {
            tracing::warn!(router_id, error = %e, "keychain check failed");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_id() -> String {
        format!("test-{}", uuid::Uuid::new_v4())
    }

    #[test]
    #[cfg_attr(target_os = "linux", ignore = "needs a running Secret Service (desktop session)")]
    fn roundtrip_set_get_delete() {
        let id = unique_id();
        assert!(!has_key(&id), "fresh id should not have a key");

        set_key(&id, "sk-test-12345").expect("set should succeed");
        let key = get_key(&id).expect("get should succeed");
        assert_eq!(key, "sk-test-12345");
        assert!(has_key(&id), "has_key should report true after set");

        delete_key(&id).expect("delete should succeed");
        assert!(!has_key(&id), "has_key should be false after delete");
        assert!(matches!(get_key(&id), Err(KeychainError::NotFound)));
    }

    #[test]
    #[cfg_attr(target_os = "linux", ignore = "needs a running Secret Service (desktop session)")]
    fn set_replaces_existing_value() {
        let id = unique_id();
        set_key(&id, "first").unwrap();
        set_key(&id, "second").unwrap();
        assert_eq!(get_key(&id).unwrap(), "second");
        delete_key(&id).unwrap();
    }

    #[test]
    #[cfg_attr(target_os = "linux", ignore = "needs a running Secret Service (desktop session)")]
    fn delete_missing_is_noop() {
        let id = unique_id();
        delete_key(&id).expect("delete on missing should not error");
    }

    /// #283: quiet unless the caller says the user is there to answer.
    #[test]
    fn prompting_is_allowed_only_inside_allow_prompt() {
        assert!(!prompting_allowed());
        assert!(allow_prompt(prompting_allowed));
        assert!(!prompting_allowed(), "the scope ends with the call");
        let other = std::thread::spawn(|| {
            allow_prompt(|| std::thread::spawn(prompting_allowed).join().unwrap())
        });
        assert!(!other.join().unwrap(), "another thread is not in the scope");
    }

    #[test]
    fn service_name_uses_router_id() {
        assert_eq!(service_name("openai"), "com.sensei.gateway.router.openai");
    }

    #[test]
    fn keyring_is_the_default_backend() {
        assert_eq!(parse_backend(None), Backend::Keyring);
        assert_eq!(parse_backend(Some("")), Backend::Keyring);
        assert_eq!(parse_backend(Some("keyring")), Backend::Keyring);
    }

    #[test]
    fn an_unknown_backend_falls_back_to_keyring() {
        assert_eq!(parse_backend(Some("bogus")), Backend::Keyring);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn security_selects_the_legacy_macos_backend() {
        assert_eq!(parse_backend(Some("security")), Backend::MacosSecurity);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn security_is_not_a_backend_off_macos() {
        assert_eq!(parse_backend(Some("security")), Backend::Keyring);
    }
}
