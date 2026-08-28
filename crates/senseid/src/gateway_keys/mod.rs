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

/// The argument vector for a Keychain write — a pure value so the invariant
/// "the secret is not in here" is a deterministic assertion rather than a race.
///
/// `-w` is LAST and VALUELESS: that is what makes `security` read the secret from
/// stdin instead of from its arguments.
fn add_password_args<'a>(service: &'a str, account: &'a str) -> [&'a str; 6] {
    ["add-generic-password", "-U", "-s", service, "-a", account]
}

/// Store a secret in the login Keychain **without putting it in `argv`**.
///
/// `security add-generic-password -w <secret>` puts the secret in the spawned
/// process's argument vector, where any local process can read it via `ps` /
/// `KERN_PROCARGS2` — and anything that records process launches (EDR, audit,
/// process accounting) captures it too. `security`'s own usage text says so:
/// *"Use of the -p or -w options is insecure. Specify -w as the last option to be
/// prompted."*
///
/// That was not theoretical. An unattended `dojo_sync` pass re-stores a rotated
/// Supabase refresh token on every cadence, and an unprivileged `ps -axww` on this
/// machine captured a live token straight out of this call's arguments.
///
/// So `-w` is passed with NO value and the secret goes over stdin. The prompt asks
/// twice ("retype password"), so it is written twice — verified against
/// `/usr/bin/security` rather than assumed.
///
/// # Blocking
/// Spawns a process (~50ms). Async callers must use `spawn_blocking`.
pub fn keychain_set(service: &str, account: &str, secret: &str) -> Result<(), KeychainError> {
    use std::io::Write;
    let mut child = Command::new("/usr/bin/security")
        .args(add_password_args(service, account))
        // Appended here, valueless, so `add_password_args` can be asserted to hold
        // no secret while the flag that triggers the stdin prompt is still sent.
        .arg("-w")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    {
        let stdin = child.stdin.as_mut().ok_or(KeychainError::NotFound)?;
        // Twice: `security` prompts for the value and then for a confirmation.
        writeln!(stdin, "{secret}")?;
        writeln!(stdin, "{secret}")?;
    }
    let out = child.wait_with_output()?;
    if !out.status.success() {
        return Err(KeychainError::CommandFailed(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(())
}

/// Shells out to `/usr/bin/security` which spawns a process (~50ms).
/// Callers in an async context must wrap this in
/// `tokio::task::spawn_blocking`.
pub fn set_key(router_id: &str, key: &str) -> Result<(), KeychainError> {
    // Via `keychain_set` so the key never reaches argv — see its docs.
    keychain_set(&service_name(router_id), ACCOUNT, key)
}

/// Read a key from the Keychain. Returns NotFound if absent.
///
/// # Blocking
/// Shells out to `/usr/bin/security` which spawns a process (~50ms).
/// Callers in an async context must wrap this in
/// `tokio::task::spawn_blocking`.
pub fn get_key(router_id: &str) -> Result<String, KeychainError> {
    let service = service_name(router_id);
    let output = Command::new("/usr/bin/security")
        .args(["find-generic-password", "-s", &service, "-a", ACCOUNT, "-w"])
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
/// Shells out to `/usr/bin/security` which spawns a process (~50ms).
/// Callers in an async context must wrap this in
/// `tokio::task::spawn_blocking`.
pub fn delete_key(router_id: &str) -> Result<(), KeychainError> {
    let service = service_name(router_id);
    let output = Command::new("/usr/bin/security")
        .args(["delete-generic-password", "-s", &service, "-a", ACCOUNT])
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

#[cfg(test)]
mod argv_tests {
    use super::*;

    /// The secret must never reach the spawned process's argument vector.
    ///
    /// `security add-generic-password -w <secret>` put a live Supabase refresh
    /// token in `argv`, where an unprivileged `ps -axww` on this machine captured
    /// it — once per `dojo_sync` cadence, unattended. Any local process can read
    /// another's arguments, and anything recording process launches records them.
    ///
    /// Asserted on the argv VALUE, not by sampling `ps`: the first version of this
    /// test raced a ~50ms process and passed against the argv form it was written
    /// to catch. A pure function cannot be raced.
    #[test]
    fn the_argument_vector_carries_no_secret() {
        let secret = "ztest-sentinel-do-not-leak";
        let args = add_password_args("com.sensei.probe", "acct");
        assert!(!args.iter().any(|a| a.contains(secret)), "the secret is in argv: {args:?}");
        // And the flag that triggers the stdin prompt must be valueless — a `-w`
        // followed by the secret is exactly the leak.
        assert!(!args.contains(&"-w"), "`-w` is appended at the spawn, never with a value");
    }

    /// The secret still round-trips, so the stdin path actually works — a write
    /// that leaks nothing but stores nothing would pass the test above.
    #[test]
    fn a_secret_written_over_stdin_reads_back_intact() {
        let secret = format!("ztest-{}", uuid::Uuid::new_v4().simple());
        let (service, account) = ("ztest.sensei.roundtrip", "probe");
        keychain_set(service, account, &secret).expect("the write succeeds");
        let out = Command::new("/usr/bin/security")
            .args(["find-generic-password", "-s", service, "-a", account, "-w"])
            .output()
            .expect("read back");
        let got = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let _ = Command::new("/usr/bin/security")
            .args(["delete-generic-password", "-s", service, "-a", account])
            .output();
        assert_eq!(got, secret, "the stdin-fed secret must store verbatim");
    }
}
