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
    quietly(|| entry(service, account)?.set_password(secret).map_err(map_err))
}

pub(super) fn get(service: &str, account: &str) -> Result<String, KeychainError> {
    quietly(|| entry(service, account)?.get_password().map_err(map_err))
}

/// A missing entry is success — the goal is "nothing stored", and that holds.
pub(super) fn delete(service: &str, account: &str) -> Result<(), KeychainError> {
    quietly(|| match entry(service, account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(map_err(e)),
    })
}

/// Run one Keychain call with its UI off unless the thread is inside
/// [`super::allow_prompt`] (#283). The switch is process-wide, so every call
/// holds one lock: a call ending must not turn prompts back on under another
/// still in flight. A prompting call holds it while its dialog is up; that is
/// only ever an action the user just took, and they are there to answer.
fn quietly<T>(call: impl FnOnce() -> Result<T, KeychainError>) -> Result<T, KeychainError> {
    #[cfg(target_os = "macos")]
    {
        use security_framework::os::macos::keychain::SecKeychain;
        static KEYCHAIN: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _held = KEYCHAIN.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if super::prompting_allowed() {
            return call();
        }
        let _no_ui = SecKeychain::disable_user_interaction()
            .map_err(|e| KeychainError::Unavailable(e.to_string()))?;
        call()
    }
    #[cfg(not(target_os = "macos"))]
    call()
}

/// Only `NoEntry` is a miss. A store that cannot be reached (no Secret Service
/// running, a locked collection, a denied Keychain prompt) is `Unavailable`, never
/// `NotFound` — the caller must be able to tell "not signed in" from "could not
/// look".
fn map_err(e: keyring::Error) -> KeychainError {
    match e {
        keyring::Error::NoEntry => KeychainError::NotFound,
        #[cfg(target_os = "macos")]
        keyring::Error::PlatformFailure(ref inner) if refused_without_prompt(inner.as_ref()) => {
            KeychainError::NeedsApproval
        }
        keyring::Error::NoStorageAccess(_) | keyring::Error::NoDefaultStore => {
            KeychainError::Unavailable(e.to_string())
        }
        other => KeychainError::Store(other.to_string()),
    }
}

/// errSecAuthFailed (an item this binary is not trusted on, UI off) or
/// errSecInteractionNotAllowed: the Keychain declined rather than ask.
#[cfg(target_os = "macos")]
fn refused_without_prompt(inner: &(dyn std::error::Error + Send + Sync + 'static)) -> bool {
    inner
        .downcast_ref::<security_framework::base::Error>()
        .is_some_and(|e| matches!(e.code(), -25293 | -25308))
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

    /// #283: the Keychain refusing without a prompt — errSecAuthFailed for an
    /// item this binary is not trusted on, errSecInteractionNotAllowed when the
    /// UI is off — is "needs approval", not a generic store error.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_refusal_without_a_prompt_needs_approval() {
        for code in [-25293, -25308] {
            let e = keyring::Error::PlatformFailure(Box::new(
                security_framework::base::Error::from_code(code),
            ));
            assert!(matches!(map_err(e), KeychainError::NeedsApproval), "code {code}");
        }
        let other = keyring::Error::PlatformFailure(Box::new(
            security_framework::base::Error::from_code(-25291),
        ));
        assert!(matches!(map_err(other), KeychainError::Store(_)), "other codes stay Store");
    }

    /// #283, against the real login keychain: an item this binary is not
    /// trusted on (made by `/usr/bin/security`) fails fast, with no dialog.
    #[cfg(target_os = "macos")]
    #[test]
    fn an_untrusted_item_fails_fast_instead_of_prompting() {
        let service = format!("sensei-test-283-{}", uuid::Uuid::new_v4());
        let made = std::process::Command::new("/usr/bin/security")
            .args(["add-generic-password", "-s", &service, "-a", "probe", "-w", "not-a-secret"])
            .status()
            .unwrap();
        assert!(made.success());
        let started = std::time::Instant::now();
        let got = get(&service, "probe");
        let _ = std::process::Command::new("/usr/bin/security")
            .args(["delete-generic-password", "-s", &service, "-a", "probe"])
            .output();
        assert!(matches!(got, Err(KeychainError::NeedsApproval)), "{got:?}");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "it must not wait on a prompt"
        );
    }
}
