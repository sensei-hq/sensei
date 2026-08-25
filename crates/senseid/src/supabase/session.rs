//! The daemon's Supabase session: exchanging a PKCE code, refreshing, and
//! keeping the refresh token where it belongs.
//!
//! ## Where the token lives
//!
//! The Keychain, never Postgres. A refresh token IS the user in dōjō — longer
//! lived and broader than the device tokens it replaces — so it gets the same
//! treatment `dojo_memberships.credential_ref` already establishes: the secret
//! sits in the OS keychain and the database holds only a handle.
//!
//! Access tokens are deliberately NOT persisted. They expire in an hour, and a
//! stored one is a credential at rest with no upside — it is cheaper to refresh
//! than to protect.

use serde::Deserialize;

/// Keychain service namespace, matching `com.sensei.gateway.router.*`.
const KEYCHAIN_SERVICE: &str = "com.sensei.supabase";

/// The account slot for a persona's session.
///
/// PER PERSONA, not one per install. A single slot meant signing in as a second
/// identity silently EVICTED the first — observed live: signing in as
/// hi@sensei-hq.com and then as me@jerrythomas.name left only the second, with
/// no indication the first had gone. That directly contradicts the point of
/// personas, which exist because a user keeps working identities apart and needs
/// both linked at once.
fn account_for(persona: &str) -> String {
    format!("refresh_token.{}", persona.to_lowercase())
}

/// What Supabase returns from `/auth/v1/token`.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    /// Seconds until `access_token` expires.
    #[serde(default)]
    pub expires_in: i64,
}

/// A live session held in memory for the process's lifetime.
#[derive(Debug, Clone)]
pub struct Session {
    pub access_token: String,
    /// When the access token stops being usable, as an epoch second.
    pub expires_at: i64,
}

impl Session {
    /// Whether the access token should be refreshed before the next call.
    ///
    /// Refreshes 60 seconds EARLY rather than on expiry. A token that expires
    /// mid-request fails the request, and the caller cannot tell an expired
    /// credential from a revoked one — so the retry looks like an auth failure
    /// rather than a clock boundary.
    pub fn needs_refresh(&self, now_epoch_secs: i64) -> bool {
        now_epoch_secs >= self.expires_at - 60
    }

    pub fn from_response(r: &TokenResponse, now_epoch_secs: i64) -> Self {
        Self {
            access_token: r.access_token.clone(),
            expires_at: now_epoch_secs + r.expires_in.max(0),
        }
    }
}

/// The body for the PKCE code exchange (`grant_type=pkce`).
///
/// Built as a value rather than inline so the shape is testable without a
/// network: a wrong field name here fails as "invalid grant", which reads like a
/// credential problem rather than a serialization one.
pub fn pkce_exchange_body(auth_code: &str, code_verifier: &str) -> serde_json::Value {
    serde_json::json!({ "auth_code": auth_code, "code_verifier": code_verifier })
}

/// The body for a refresh (`grant_type=refresh_token`).
pub fn refresh_body(refresh_token: &str) -> serde_json::Value {
    serde_json::json!({ "refresh_token": refresh_token })
}

/// The token endpoint for a grant type.
pub fn token_url(supabase_url: &str, grant_type: &str) -> String {
    format!(
        "{}/auth/v1/token?grant_type={}",
        supabase_url.trim_end_matches('/'),
        grant_type
    )
}

/// Persist the refresh token.
///
/// # Blocking
///
/// Shells out to `/usr/bin/security` (~50ms). Async callers must wrap this in
/// `spawn_blocking`, exactly as `gateway_keys` documents.
pub fn store_refresh_token(persona: &str, token: &str) -> Result<(), crate::gateway_keys::KeychainError> {
    keychain_write(KEYCHAIN_SERVICE, &account_for(persona), token)
}

/// Read the stored refresh token, if the user has signed in.
pub fn load_refresh_token(persona: &str) -> Result<String, crate::gateway_keys::KeychainError> {
    keychain_read(KEYCHAIN_SERVICE, &account_for(persona))
}

/// Forget the session — sign-out, or a refresh token the server has rejected.
///
/// Removing a rejected token matters: a permanently-invalid one otherwise makes
/// every subsequent refresh fail identically, and the daemon retries forever
/// instead of surfacing "you need to sign in again".
pub fn clear_refresh_token(persona: &str) -> Result<(), crate::gateway_keys::KeychainError> {
    let account = account_for(persona);
    let out = std::process::Command::new("/usr/bin/security")
        .args(["delete-generic-password", "-s", KEYCHAIN_SERVICE, "-a", &account])
        .output()?;
    // A missing entry is success: the goal is "no token stored", and that holds.
    if out.status.success() || String::from_utf8_lossy(&out.stderr).contains("could not be found") {
        Ok(())
    } else {
        Err(crate::gateway_keys::KeychainError::CommandFailed(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ))
    }
}

fn keychain_write(service: &str, account: &str, secret: &str) -> Result<(), crate::gateway_keys::KeychainError> {
    let out = std::process::Command::new("/usr/bin/security")
        .args(["add-generic-password", "-U", "-s", service, "-a", account, "-w", secret])
        .output()?;
    if out.status.success() {
        Ok(())
    } else {
        Err(crate::gateway_keys::KeychainError::CommandFailed(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ))
    }
}

fn keychain_read(service: &str, account: &str) -> Result<String, crate::gateway_keys::KeychainError> {
    let out = std::process::Command::new("/usr/bin/security")
        .args(["find-generic-password", "-s", service, "-a", account, "-w"])
        .output()?;
    if !out.status.success() {
        return Err(crate::gateway_keys::KeychainError::NotFound);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_refreshes_before_it_expires_not_after() {
        // Refreshing ON expiry loses the request that discovers it, and the
        // failure is indistinguishable from a revoked credential.
        let s = Session { access_token: "t".into(), expires_at: 1_000 };
        assert!(!s.needs_refresh(800), "not yet due");
        assert!(s.needs_refresh(941), "due inside the 60s margin");
        assert!(s.needs_refresh(1_000), "due at expiry");
        assert!(s.needs_refresh(1_200), "overdue");
    }

    #[test]
    fn expiry_is_derived_from_the_response() {
        let r = TokenResponse {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_in: 3600,
        };
        assert_eq!(Session::from_response(&r, 100).expires_at, 3700);
    }

    #[test]
    fn a_missing_expires_in_does_not_produce_a_session_from_the_past() {
        // Supabase always sends it, but a negative or absent value must not make
        // expires_at earlier than now — that would refresh on every single call.
        let r = TokenResponse {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_in: -5,
        };
        assert_eq!(Session::from_response(&r, 100).expires_at, 100);
    }

    #[test]
    fn the_exchange_body_uses_the_field_names_supabase_expects() {
        // A wrong name here comes back as "invalid grant", which reads like a bad
        // credential rather than a bad request body.
        let b = pkce_exchange_body("code-1", "verifier-1");
        assert_eq!(b["auth_code"], "code-1");
        assert_eq!(b["code_verifier"], "verifier-1");
        assert_eq!(refresh_body("r-1")["refresh_token"], "r-1");
    }

    #[test]
    fn each_persona_gets_its_own_keychain_slot() {
        // The bug this fixes, found live: one slot meant signing in as a second
        // identity evicted the first, silently. Personas exist precisely so two
        // working identities can be linked at once.
        assert_ne!(account_for("sensei-hq"), account_for("jerrythomas"));
        assert!(account_for("sensei-hq").starts_with("refresh_token."));
    }

    #[test]
    fn the_persona_slot_is_case_insensitive() {
        // The label is user-chosen and reaches here from a query string, so
        // "Sensei-HQ" and "sensei-hq" must not become two half-signed-in states.
        assert_eq!(account_for("Sensei-HQ"), account_for("sensei-hq"));
    }

    #[test]
    fn the_token_url_carries_the_grant_type_and_survives_a_trailing_slash() {
        assert_eq!(
            token_url("http://127.0.0.1:54321/", "pkce"),
            "http://127.0.0.1:54321/auth/v1/token?grant_type=pkce"
        );
        assert!(token_url("http://x", "refresh_token").ends_with("grant_type=refresh_token"));
    }
}
