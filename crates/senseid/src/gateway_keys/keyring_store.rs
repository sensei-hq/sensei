//! Default backend: the `keyring` crate.
//!
//! One API over each platform's native store — Keychain Services on macOS,
//! Credential Manager on Windows, the Secret Service (GNOME Keyring / KWallet)
//! over D-Bus on Linux. In-process, so there is no `argv` for a secret to leak
//! through and no helper binary that has to be installed.

use super::KeychainError;

fn entry(service: &str, account: &str) -> Result<keyring::Entry, KeychainError> {
    keyring::Entry::new(service, account).map_err(map_err)
}

pub(super) fn set(service: &str, account: &str, secret: &str) -> Result<(), KeychainError> {
    entry(service, account)?.set_password(secret).map_err(map_err)
}

pub(super) fn get(service: &str, account: &str) -> Result<String, KeychainError> {
    entry(service, account)?.get_password().map_err(map_err)
}

/// A missing entry is success — the goal is "nothing stored", and that holds.
pub(super) fn delete(service: &str, account: &str) -> Result<(), KeychainError> {
    match entry(service, account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(map_err(e)),
    }
}

/// Only `NoEntry` is a miss. A store that cannot be reached (no Secret Service
/// running, a locked collection, a denied Keychain prompt) is `Unavailable`, never
/// `NotFound` — the caller must be able to tell "not signed in" from "could not
/// look".
fn map_err(e: keyring::Error) -> KeychainError {
    match e {
        keyring::Error::NoEntry => KeychainError::NotFound,
        keyring::Error::NoStorageAccess(_) | keyring::Error::NoDefaultStore => {
            KeychainError::Unavailable(e.to_string())
        }
        other => KeychainError::Store(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_missing_entry_maps_to_not_found() {
        assert!(matches!(map_err(keyring::Error::NoEntry), KeychainError::NotFound));
        assert!(matches!(map_err(keyring::Error::NoDefaultStore), KeychainError::Unavailable(_)));
        assert!(matches!(
            map_err(keyring::Error::Invalid("service".into(), "empty".into())),
            KeychainError::Store(_)
        ));
    }
}
