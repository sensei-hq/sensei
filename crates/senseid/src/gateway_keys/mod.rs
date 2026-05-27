//! macOS Keychain access for gateway router API keys.
//!
//! Wraps `/usr/bin/security` so we don't add a new crate dependency.
//! Service name namespace: `com.sensei.gateway.router.<router_id>`.
//! Account name: `"default"` (one key per router for now; per-project
//! overrides would use `account = project_id` later).

use std::process::Command;

const ACCOUNT: &str = "default";

fn service_name(router_id: &str) -> String {
    format!("com.sensei.gateway.router.{router_id}")
}

#[derive(Debug, thiserror::Error)]
pub enum KeychainError {
    #[error("key not found in keychain")]
    NotFound,
    #[error("keychain command failed: {0}")]
    CommandFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Write a key to the Keychain. Replaces an existing entry (`-U`).
///
/// # Blocking
///
/// Shells out to `/usr/bin/security` which spawns a process (~50ms).
/// Callers in an async context must wrap this in
/// `tokio::task::spawn_blocking`.
pub fn set_key(router_id: &str, key: &str) -> Result<(), KeychainError> {
    let service = service_name(router_id);
    let output = Command::new("/usr/bin/security")
        .args([
            "add-generic-password",
            "-s", &service,
            "-a", ACCOUNT,
            "-w", key,
            "-U",
        ])
        .output()?;
    if !output.status.success() {
        return Err(KeychainError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(())
}

/// Read a key from the Keychain. Returns NotFound if absent.
///
/// # Blocking
///
/// Shells out to `/usr/bin/security` which spawns a process (~50ms).
/// Callers in an async context must wrap this in
/// `tokio::task::spawn_blocking`.
pub fn get_key(router_id: &str) -> Result<String, KeychainError> {
    let service = service_name(router_id);
    let output = Command::new("/usr/bin/security")
        .args([
            "find-generic-password",
            "-s", &service,
            "-a", ACCOUNT,
            "-w",
        ])
        .output()?;
    if !output.status.success() {
        if output.status.code() == Some(44) {
            return Err(KeychainError::NotFound);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(KeychainError::CommandFailed(stderr.trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Delete a key. Returns Ok(()) whether or not it existed.
///
/// # Blocking
///
/// Shells out to `/usr/bin/security` which spawns a process (~50ms).
/// Callers in an async context must wrap this in
/// `tokio::task::spawn_blocking`.
pub fn delete_key(router_id: &str) -> Result<(), KeychainError> {
    let service = service_name(router_id);
    let output = Command::new("/usr/bin/security")
        .args([
            "delete-generic-password",
            "-s", &service,
            "-a", ACCOUNT,
        ])
        .output()?;
    if !output.status.success() {
        if output.status.code() == Some(44) {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(KeychainError::CommandFailed(stderr.trim().to_string()));
    }
    Ok(())
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
    #[cfg_attr(not(target_os = "macos"), ignore)]
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
    #[cfg_attr(not(target_os = "macos"), ignore)]
    fn set_replaces_existing_value() {
        let id = unique_id();
        set_key(&id, "first").unwrap();
        set_key(&id, "second").unwrap();
        assert_eq!(get_key(&id).unwrap(), "second");
        delete_key(&id).unwrap();
    }

    #[test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    fn delete_missing_is_noop() {
        let id = unique_id();
        delete_key(&id).expect("delete on missing should not error");
    }

    #[test]
    fn service_name_uses_router_id() {
        assert_eq!(service_name("openai"), "com.sensei.gateway.router.openai");
    }
}
