//! Where a tracker's API token lives: the OS keyring through `gateway_keys`,
//! behind a narrow trait so the lifecycle is testable. Every keyring call is
//! blocking, so it runs on a blocking thread under a deadline. (Spec R3.)

use std::sync::Arc;
use std::time::Duration;

use crate::gateway_keys::KeychainError;
use crate::tracker::{ApiToken, TrackerError};

/// How long one keyring call may take before it counts as `credential_store`.
pub const KEYRING_TIMEOUT: Duration = Duration::from_secs(5);

/// A blocking secret store. Implementations may block (D-Bus, Keychain); callers
/// go through [`bounded`], never call these on an async thread directly.
pub trait SecretStore: Send + Sync + 'static {
    fn set(&self, service: &str, account: &str, token: &ApiToken) -> Result<(), TrackerError>;
    fn get(&self, service: &str, account: &str) -> Result<ApiToken, TrackerError>;
    /// A missing entry counts as success.
    fn delete(&self, service: &str, account: &str) -> Result<(), TrackerError>;
    /// The prefix of every service name this store holds; drains match on it.
    fn namespace(&self) -> String {
        crate::tracker::connect::SECRET_PREFIX.to_string()
    }
}

/// The production store: `gateway_keys`' keyring backend.
pub struct KeyringSecretStore;

/// Map a keyring error onto the tracker vocabulary (all five variants).
pub fn map_keychain_error(e: &KeychainError) -> TrackerError {
    match e {
        KeychainError::NotFound => TrackerError::TokenMissing,
        // NeedsApproval (#283/#302): the OS refused this binary without asking —
        // the token is there but unreadable until approved or replaced.
        KeychainError::Unavailable(_)
        | KeychainError::NeedsApproval
        | KeychainError::Store(_)
        | KeychainError::CommandFailed(_)
        | KeychainError::Io(_) => TrackerError::CredentialStore,
    }
}

impl SecretStore for KeyringSecretStore {
    fn set(&self, service: &str, account: &str, token: &ApiToken) -> Result<(), TrackerError> {
        crate::gateway_keys::keychain_set(service, account, &token.0)
            .map_err(|e| map_keychain_error(&e))
    }
    fn get(&self, service: &str, account: &str) -> Result<ApiToken, TrackerError> {
        crate::gateway_keys::keychain_get(service, account)
            .map(ApiToken)
            .map_err(|e| map_keychain_error(&e))
    }
    fn delete(&self, service: &str, account: &str) -> Result<(), TrackerError> {
        crate::gateway_keys::keychain_delete(service, account).map_err(|e| map_keychain_error(&e))
    }
}

/// Run one store call on a blocking thread under `timeout`. A timeout, or a
/// panicked call, is `credential_store`.
pub async fn bounded<T, F>(
    store: Arc<dyn SecretStore>,
    timeout: Duration,
    call: F,
) -> Result<T, TrackerError>
where
    T: Send + 'static,
    F: FnOnce(&dyn SecretStore) -> Result<T, TrackerError> + Send + 'static,
{
    let task = tokio::task::spawn_blocking(move || call(store.as_ref()));
    match tokio::time::timeout(timeout, task).await {
        Ok(Ok(result)) => result,
        // Timed out (the blocking thread keeps running and is abandoned) or panicked.
        Ok(Err(_)) | Err(_) => Err(TrackerError::CredentialStore),
    }
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// In-memory store with failure injection, for lifecycle tests. Each one has
    /// its own namespace, so a drain run by one test never reaches another's.
    pub struct MemoryStore {
        pub namespace: String,
        pub entries: Mutex<HashMap<(String, String), String>>,
        pub fail_set: Mutex<bool>,
        pub fail_delete: Mutex<bool>,
        /// Fail every `get` with `credential_store` while set (write-back gate 22a).
        pub fail_get: Mutex<bool>,
        pub deletes: Mutex<Vec<String>>,
        /// Each delete blocks this long first (a slow or prompting keyring).
        pub delete_delay: Mutex<Option<Duration>>,
    }
    impl Default for MemoryStore {
        fn default() -> Self {
            MemoryStore {
                namespace: format!(
                    "{}test-{}.",
                    crate::tracker::connect::SECRET_PREFIX,
                    uuid::Uuid::new_v4().simple()
                ),
                entries: Mutex::default(),
                fail_set: Mutex::default(),
                fail_delete: Mutex::default(),
                fail_get: Mutex::default(),
                deletes: Mutex::default(),
                delete_delay: Mutex::default(),
            }
        }
    }
    impl MemoryStore {
        pub fn has(&self, service: &str) -> bool {
            self.entries.lock().unwrap().keys().any(|(s, _)| s == service)
        }
    }
    impl SecretStore for MemoryStore {
        fn set(&self, service: &str, account: &str, token: &ApiToken) -> Result<(), TrackerError> {
            if *self.fail_set.lock().unwrap() {
                return Err(TrackerError::CredentialStore);
            }
            self.entries.lock().unwrap().insert((service.into(), account.into()), token.0.clone());
            Ok(())
        }
        fn get(&self, service: &str, account: &str) -> Result<ApiToken, TrackerError> {
            if *self.fail_get.lock().unwrap() {
                return Err(TrackerError::CredentialStore);
            }
            self.entries
                .lock()
                .unwrap()
                .get(&(service.into(), account.into()))
                .cloned()
                .map(ApiToken)
                .ok_or(TrackerError::TokenMissing)
        }
        fn delete(&self, service: &str, account: &str) -> Result<(), TrackerError> {
            let delay = *self.delete_delay.lock().unwrap();
            if let Some(d) = delay {
                std::thread::sleep(d);
            }
            if *self.fail_delete.lock().unwrap() {
                return Err(TrackerError::CredentialStore);
            }
            self.deletes.lock().unwrap().push(service.into());
            self.entries.lock().unwrap().remove(&(service.into(), account.into()));
            Ok(())
        }
        fn namespace(&self) -> String {
            self.namespace.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_keychain_error_variant_is_mapped() {
        assert_eq!(map_keychain_error(&KeychainError::NotFound), TrackerError::TokenMissing);
        for e in [
            KeychainError::Unavailable("locked".into()),
            KeychainError::NeedsApproval,
            KeychainError::Store("x".into()),
            KeychainError::CommandFailed("x".into()),
            KeychainError::Io(std::io::Error::other("x")),
        ] {
            assert_eq!(map_keychain_error(&e), TrackerError::CredentialStore, "{e:?}");
        }
    }

    /// A store whose every call blocks far longer than any deadline.
    struct Hung;
    impl SecretStore for Hung {
        fn set(&self, _: &str, _: &str, _: &ApiToken) -> Result<(), TrackerError> {
            std::thread::sleep(Duration::from_secs(3));
            Ok(())
        }
        fn get(&self, _: &str, _: &str) -> Result<ApiToken, TrackerError> {
            std::thread::sleep(Duration::from_secs(3));
            Ok(ApiToken("x".into()))
        }
        fn delete(&self, _: &str, _: &str) -> Result<(), TrackerError> {
            std::thread::sleep(Duration::from_secs(3));
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_hung_keyring_times_out_as_credential_store() {
        let started = std::time::Instant::now();
        let r = bounded(Arc::new(Hung), Duration::from_millis(100), |s| s.get("svc", "acct")).await;
        assert_eq!(r, Err(TrackerError::CredentialStore));
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "the caller is released at the deadline"
        );
    }

    #[tokio::test]
    async fn a_fast_call_returns_its_result() {
        let store = Arc::new(test_support::MemoryStore::default());
        let r = bounded(store.clone(), KEYRING_TIMEOUT, |s| {
            s.set("svc", "acct", &ApiToken("t".into()))
        })
        .await;
        assert_eq!(r, Ok(()));
        assert!(store.has("svc"));
    }
}
