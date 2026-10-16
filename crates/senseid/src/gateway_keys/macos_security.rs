//! LEGACY macOS backend: shells out to `/usr/bin/security`.
//!
//! The daemon's original credential store, kept only as a rollback while the
//! [`keyring`](super::keyring_store) backend proves itself on macOS. Selected with
//! `SENSEI_CREDENTIAL_BACKEND=security`; scheduled for deletion once that
//! migration is confirmed (see `docs/backlog.md`).
//!
//! Both backends address the same login-Keychain generic-password items
//! (service + account), so switching between them needs no data migration.

use std::process::Command;

use super::KeychainError;

const SECURITY: &str = "/usr/bin/security";

/// `security`'s exit status for `errSecItemNotFound`, on both find and delete.
const ITEM_NOT_FOUND: i32 = 44;

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
pub(super) fn set(service: &str, account: &str, secret: &str) -> Result<(), KeychainError> {
    use std::io::Write;
    let mut child = Command::new(SECURITY)
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

pub(super) fn get(service: &str, account: &str) -> Result<String, KeychainError> {
    let output = Command::new(SECURITY)
        .args(["find-generic-password", "-s", service, "-a", account, "-w"])
        .output()?;
    if !output.status.success() {
        if output.status.code() == Some(ITEM_NOT_FOUND) {
            return Err(KeychainError::NotFound);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(KeychainError::CommandFailed(stderr.trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(super) fn delete(service: &str, account: &str) -> Result<(), KeychainError> {
    let output = Command::new(SECURITY)
        .args(["delete-generic-password", "-s", service, "-a", account])
        .output()?;
    if !output.status.success() {
        if output.status.code() == Some(ITEM_NOT_FOUND) {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(KeychainError::CommandFailed(format!("{account}: {}", stderr.trim())));
    }
    Ok(())
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
        set(service, account, &secret).expect("the write succeeds");
        let got = get(service, account);
        let _ = delete(service, account);
        assert_eq!(got.expect("read back"), secret, "the stdin-fed secret must store verbatim");
    }

    #[test]
    fn deleting_a_missing_item_is_success() {
        let account = format!("missing-{}", uuid::Uuid::new_v4());
        delete("ztest.sensei.roundtrip", &account).expect("missing is not an error");
    }
}
