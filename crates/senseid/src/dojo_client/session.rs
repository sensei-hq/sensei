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

/// Slot for the PROVIDER token (GitHub's), kept apart from Supabase's.
///
/// A separate secret with a different lifetime and blast radius: it can read the
/// user's repositories and organisations, so it gets its own slot rather than
/// being bundled — revoking one should not require discarding the other.
fn provider_account_for(persona: &str) -> String {
    format!("provider_token.{}", persona.to_lowercase())
}

/// Persist the GitHub token. Keychain, never Postgres — same rule as the refresh
/// token, and more important here because this one reaches GitHub directly.
pub fn store_provider_token(
    persona: &str,
    token: &str,
) -> Result<(), crate::gateway_keys::KeychainError> {
    keychain_write(KEYCHAIN_SERVICE, &provider_account_for(persona), token)
}

/// The GitHub token for a persona, if the sign-in captured one.
pub fn load_provider_token(persona: &str) -> Result<String, crate::gateway_keys::KeychainError> {
    keychain_read(KEYCHAIN_SERVICE, &provider_account_for(persona))
}

/// Slot for GitHub's own refresh token, when the OAuth App issues expiring ones.
fn provider_refresh_account_for(persona: &str) -> String {
    format!("provider_refresh.{}", persona.to_lowercase())
}

/// Store GitHub's refresh token, when one was issued.
pub fn store_provider_refresh_token(
    persona: &str,
    token: &str,
) -> Result<(), crate::gateway_keys::KeychainError> {
    keychain_write(KEYCHAIN_SERVICE, &provider_refresh_account_for(persona), token)
}

/// Read GitHub's refresh token — the credential renewal is spent from.
///
/// Written at the exchange since sign-in existed and, until renewal was built,
/// never read: the slot held the one thing that could recover a dying session
/// and nothing anywhere could reach it. `Err` covers both "no such item" and a
/// Keychain failure; the caller cannot renew either way.
pub fn load_provider_refresh_token(
    persona: &str,
) -> Result<String, crate::gateway_keys::KeychainError> {
    keychain_read(KEYCHAIN_SERVICE, &provider_refresh_account_for(persona))
}

/// What Supabase returns from `/auth/v1/token`.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    /// Seconds until `access_token` expires.
    #[serde(default)]
    pub expires_in: i64,
    /// The PROVIDER's token — GitHub's, not Supabase's.
    ///
    /// Returned only at the exchange. GoTrue holds it in `auth.flow_state`
    /// briefly and then prunes that row, so a token not captured here is gone: a
    /// later query finds nothing and provisioning has no way to call GitHub.
    ///
    /// This is what `read:org` is FOR. Supabase records the user's profile but
    /// never calls `/user/orgs` itself, so without this token the org list is
    /// unreachable and provisioning would report "no organisations" when the
    /// truth is "we never asked".
    #[serde(default)]
    pub provider_token: Option<String>,
    /// GitHub's refresh token, when the OAuth App issues expiring tokens.
    ///
    /// This used to say it was "usually absent" because a classic GitHub OAuth
    /// App's tokens do not expire. Both halves turned out wrong on this install:
    /// `provider_refresh.default` IS present in the Keychain, and the failure the
    /// note predicted — "provisioning starts failing weeks later with an
    /// unexplained 401" — is exactly what happened.
    ///
    /// Capturing it was long only half the fix: nothing read it, so the one
    /// credential that could recover a dying session sat in the Keychain
    /// unreachable. [`load_provider_refresh_token`] now reads it and
    /// `POST /v1/you/forge/refresh` spends it — in the dōjō, which holds the
    /// client secret the daemon deliberately does not.
    #[serde(default)]
    pub provider_refresh_token: Option<String>,
    /// The authenticated user, as GoTrue returns it alongside the tokens.
    ///
    /// Carries `identities[]`, which is where the VERIFIED GitHub login and
    /// numeric id live. Taking it from here rather than calling `/auth/v1/user`
    /// again keeps the identity and the token from the same response — a second
    /// call could, in principle, answer for a different session.
    #[serde(default)]
    pub user: Option<serde_json::Value>,
}

/// When the current access token runs out.
///
/// Only the expiry, not the token: the daemon's durable credential is the
/// REFRESH token in the Keychain, and every caller mints a fresh access token
/// from it. A caller that needs the token takes it from the [`TokenResponse`] it
/// just received, which is the only place it is guaranteed current — holding a
/// copy here would invite using a stale one.
#[derive(Debug, Clone)]
pub struct Session {
    /// When the access token stops being usable, as an epoch second.
    pub expires_at: i64,
}

impl Session {
    /// Whether the access token would be due for refresh — REPORTED, not acted on.
    ///
    /// The 60-second early margin below guards nothing today, because no access
    /// token is ever held across calls: `live_session` performs a full network
    /// refresh on every single use, so the token a caller holds is always seconds
    /// old. The only non-test consumer is the `needsRefresh` field in
    /// `GET /api/auth/status`; nothing branches on it.
    ///
    /// Kept because the field is honest as a report and the margin is the right
    /// rule if a caller ever does cache a token — but the doc used to describe it
    /// as the thing preventing mid-request expiry, which it is not.
    pub fn needs_refresh(&self, now_epoch_secs: i64) -> bool {
        now_epoch_secs >= self.expires_at - 60
    }

    pub fn from_response(r: &TokenResponse, now_epoch_secs: i64) -> Self {
        Self { expires_at: now_epoch_secs + r.expires_in.max(0) }
    }
}

/// Persist the refresh token.
///
/// # Blocking
///
/// Shells out to `/usr/bin/security` (~50ms). Async callers must wrap this in
/// `spawn_blocking`, exactly as `gateway_keys` documents.
pub fn store_refresh_token(
    persona: &str,
    token: &str,
) -> Result<(), crate::gateway_keys::KeychainError> {
    keychain_write(KEYCHAIN_SERVICE, &account_for(persona), token)
}

/// The single un-namespaced slot used before sessions became per-persona.
const LEGACY_ACCOUNT: &str = "refresh_token";

/// Read the stored refresh token, if this persona has signed in.
///
/// Falls back to the pre-persona slot ONCE and migrates it forward. Without this
/// the rename would strand an existing session in the Keychain: unreachable by
/// the daemon, invisible in the UI, and impossible to sign out of — a credential
/// left behind with no way to revoke it.
pub fn load_refresh_token(persona: &str) -> Result<String, crate::gateway_keys::KeychainError> {
    match keychain_read(KEYCHAIN_SERVICE, &account_for(persona)) {
        Ok(t) => Ok(t),
        Err(_) => {
            let legacy = keychain_read(KEYCHAIN_SERVICE, LEGACY_ACCOUNT)?;
            // Move it under the persona, then drop the old slot so the migration
            // happens once and no orphan credential lingers.
            keychain_write(KEYCHAIN_SERVICE, &account_for(persona), &legacy)?;
            let _ = std::process::Command::new("/usr/bin/security")
                .args(["delete-generic-password", "-s", KEYCHAIN_SERVICE, "-a", LEGACY_ACCOUNT])
                .output();
            tracing::info!(persona, "migrated a pre-persona Supabase session into its own slot");
            Ok(legacy)
        }
    }
}

/// The SUPABASE slots that must go when a session is rejected or signed out.
///
/// Includes [`LEGACY_ACCOUNT`] because [`load_refresh_token`] falls back to it
/// and migrates it forward: leaving it behind meant the next cycle after a
/// sign-out read the pre-persona slot, re-wrote the per-persona one, and signed
/// the user back in — with a single `info!` as the only trace.
fn refresh_accounts_for(persona: &str) -> Vec<String> {
    vec![account_for(persona), LEGACY_ACCOUNT.to_string()]
}

/// EVERY slot a sign-in may have written — what a sign-out must clear.
///
/// A superset of [`refresh_accounts_for`], because the two provider slots are
/// GitHub's credentials rather than the dōjō's: a rejected dōjō session says
/// nothing about GitHub's token, so only an explicit sign-out takes those.
fn session_accounts_for(persona: &str) -> Vec<String> {
    let mut all = refresh_accounts_for(persona);
    all.push(provider_account_for(persona));
    all.push(provider_refresh_account_for(persona));
    all
}

/// Remove ONE Keychain entry.
///
/// A MISSING entry is success — the goal is "no token stored", and that already
/// holds.
fn delete_one(account: &str) -> Result<(), crate::gateway_keys::KeychainError> {
    let out = std::process::Command::new("/usr/bin/security")
        .args(["delete-generic-password", "-s", KEYCHAIN_SERVICE, "-a", account])
        .output()
        .map_err(crate::gateway_keys::KeychainError::from)?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    match out.status.success() || stderr.contains("could not be found") {
        true => Ok(()),
        false => Err(crate::gateway_keys::KeychainError::CommandFailed(format!(
            "{account}: {}",
            stderr.trim()
        ))),
    }
}

/// Delete each account, and report the first real failure.
///
/// Every account is attempted even after one fails, because stopping at the
/// first would leave the remaining credentials at rest, which is the exact
/// failure this function exists to prevent.
///
/// The deleter is a PARAMETER so the wiring is testable. It is the wiring that
/// carries the security property — `clear_session` and `clear_refresh_token` are
/// each one line binding a list to this loop, and swapping one list for the
/// other silently restores the defect where a GitHub `repo` token survives a
/// sign-out. Asserting on the lists alone left that swap green, so the seam is
/// here rather than a comment asking the next reader to be careful.
fn delete_each(
    accounts: &[String],
    mut delete: impl FnMut(&str) -> Result<(), crate::gateway_keys::KeychainError>,
) -> Result<(), crate::gateway_keys::KeychainError> {
    let mut first_error = None;
    for account in accounts {
        if let Err(e) = delete(account) {
            tracing::warn!(account, error = %e, "could not remove a stored credential");
            first_error.get_or_insert(e);
        }
    }
    match first_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Forget the dōjō session — a refresh token the server has rejected.
///
/// Removing a rejected token matters: a permanently-invalid one otherwise makes
/// every subsequent refresh fail identically, and the daemon retries forever
/// instead of surfacing "you need to sign in again".
///
/// Leaves the GitHub slots ALONE. They are a different credential with a
/// different lifetime, and the dōjō rejecting our session is no evidence about
/// GitHub's token. [`clear_session`] is the one that takes everything.
pub fn clear_refresh_token(persona: &str) -> Result<(), crate::gateway_keys::KeychainError> {
    clear_refresh_token_with(persona, delete_one)
}

/// [`clear_refresh_token`] with the deleter handed in — see [`clear_session_with`].
fn clear_refresh_token_with(
    persona: &str,
    delete: impl FnMut(&str) -> Result<(), crate::gateway_keys::KeychainError>,
) -> Result<(), crate::gateway_keys::KeychainError> {
    delete_each(&refresh_accounts_for(persona), delete)
}

/// Sign out — remove every credential this persona's sign-in stored.
///
/// Sign-out used to delete ONE of the three slots. `provider_token.<slot>`, a
/// GitHub token carrying `repo` and `read:org`, survived it: verified live still
/// able to read a private repository, with no code path in the daemon, CLI, app
/// or dōjō that could remove it. "Sign out" that leaves the broadest credential
/// at rest is not a sign-out.
pub fn clear_session(persona: &str) -> Result<(), crate::gateway_keys::KeychainError> {
    clear_session_with(persona, delete_one)
}

/// [`clear_session`] with the deleter handed in.
///
/// The split exists so a test can watch WHICH slots are attempted without a
/// Keychain. That is the security property — the public function is one line
/// choosing a list, and choosing the wrong one puts a live GitHub token back on
/// disk after a sign-out. Asserting on `session_accounts_for` alone left exactly
/// that swap passing green, which was verified by making it.
fn clear_session_with(
    persona: &str,
    delete: impl FnMut(&str) -> Result<(), crate::gateway_keys::KeychainError>,
) -> Result<(), crate::gateway_keys::KeychainError> {
    delete_each(&session_accounts_for(persona), delete)
}

/// Store a session secret. Delegates to [`crate::gateway_keys::keychain_set`],
/// which keeps the secret out of `argv` — this used to pass it as an argument,
/// where an unprivileged `ps` captured a live refresh token once per cadence.
fn keychain_write(
    service: &str,
    account: &str,
    secret: &str,
) -> Result<(), crate::gateway_keys::KeychainError> {
    crate::gateway_keys::keychain_set(service, account, secret)
}

fn keychain_read(
    service: &str,
    account: &str,
) -> Result<String, crate::gateway_keys::KeychainError> {
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
        let s = Session { expires_at: 1_000 };
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
            provider_token: None,
            provider_refresh_token: None,
            user: None,
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
            provider_token: None,
            provider_refresh_token: None,
            user: None,
        };
        assert_eq!(Session::from_response(&r, 100).expires_at, 100);
    }

    #[test]
    fn the_provider_token_has_its_own_slot() {
        // GitHub's token and Supabase's are different secrets with different
        // reach — this one can read repositories and organisations — so revoking
        // one must not force discarding the other.
        assert_ne!(provider_account_for("p"), account_for("p"));
        assert!(provider_account_for("p").starts_with("provider_token."));
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
    fn signing_out_targets_every_slot_the_sign_in_wrote() {
        // THE sign-out defect. A sign-in writes up to three Keychain items;
        // `signout` deleted exactly one — the Supabase refresh token. Left behind
        // was `provider_token.<slot>`: a live GitHub credential with `repo` and
        // `read:org`, confirmed on this machine to read a PRIVATE repository, at
        // rest with no code path anywhere that removed it. There was no
        // `clear_provider_token` function in the repository at all.
        let slots = session_accounts_for("default");
        assert!(
            slots.contains(&provider_account_for("default")),
            "the GitHub token must not survive a sign-out: {slots:?}"
        );
        assert!(
            slots.contains(&provider_refresh_account_for("default")),
            "nor GitHub's refresh token: {slots:?}"
        );
        assert!(slots.contains(&account_for("default")), "nor the Supabase one: {slots:?}");
    }

    /// Record what was attempted, and optionally refuse some of it.
    ///
    /// `failing` names accounts the Keychain rejects, so a test can assert the
    /// loop KEEPS GOING — the guarantee that stops one stuck entry leaving the
    /// rest of the credentials at rest.
    fn recording_deleter<'a>(
        seen: &'a mut Vec<String>,
        failing: &'static [&'static str],
    ) -> impl FnMut(&str) -> Result<(), crate::gateway_keys::KeychainError> + 'a {
        move |account: &str| {
            seen.push(account.to_string());
            match failing.iter().any(|f| account.starts_with(f)) {
                true => Err(crate::gateway_keys::KeychainError::CommandFailed(account.to_string())),
                false => Ok(()),
            }
        }
    }

    #[test]
    fn signing_out_actually_asks_for_every_one_of_those_slots() {
        // The list being right is not the same as the sign-out USING it, and the
        // difference is the whole security property. `clear_session` is one line
        // choosing between two lists; pointing it at `refresh_accounts_for`
        // silently restores the defect — a GitHub `repo`+`read:org` token left on
        // disk after a sign-out — and that mutation was RUN and passed the entire
        // suite while only `session_accounts_for` was asserted on.
        let mut seen = Vec::new();
        clear_session_with("default", recording_deleter(&mut seen, &[])).expect("all slots gone");

        assert!(seen.contains(&provider_account_for("default")), "the GitHub token: {seen:?}");
        assert!(seen.contains(&provider_refresh_account_for("default")), "its refresh: {seen:?}");
        assert!(seen.contains(&account_for("default")), "the Supabase token: {seen:?}");
        assert!(seen.iter().any(|a| a == LEGACY_ACCOUNT), "the legacy slot: {seen:?}");
    }

    #[test]
    fn a_rejected_session_asks_for_the_supabase_slots_and_no_others() {
        // The complement, and it has to be tested on the CALL too: a dōjō that
        // rejects our refresh token is no evidence about GitHub's, and the
        // separate slots exist precisely so revoking one does not discard the
        // other. Widening this function is a silent data-loss change — the user
        // would be re-prompted for GitHub over an unrelated dōjō 401.
        let mut seen = Vec::new();
        clear_refresh_token_with("default", recording_deleter(&mut seen, &[])).expect("ok");

        assert!(seen.contains(&account_for("default")), "the Supabase token: {seen:?}");
        assert!(!seen.contains(&provider_account_for("default")), "NOT GitHub's: {seen:?}");
        assert!(!seen.contains(&provider_refresh_account_for("default")), "nor its refresh");
    }

    #[test]
    fn one_stuck_slot_does_not_leave_the_others_at_rest() {
        // Stopping at the first error is the failure mode this function exists to
        // prevent: the Supabase slot is attempted FIRST, so a `?` there would
        // return before the GitHub token — the broadest credential — was ever
        // touched, and a sign-out would report an error having left the worst of
        // it behind. Every slot is attempted, and the failure is still reported.
        let mut seen = Vec::new();
        let out = clear_session_with("default", recording_deleter(&mut seen, &["refresh_token."]));

        assert!(out.is_err(), "a real Keychain failure is still surfaced");
        assert!(
            seen.contains(&provider_account_for("default")),
            "the GitHub token is attempted even after an earlier slot failed: {seen:?}"
        );
    }

    #[test]
    fn forgetting_a_refresh_token_also_drops_the_legacy_slot() {
        // `load_refresh_token` falls back to the un-namespaced slot and MIGRATES
        // it forward. While `clear_refresh_token` left that slot alone, the next
        // cycle after a sign-out read it, re-wrote `refresh_token.<persona>`, and
        // signed the user back in — with one info! line as the only trace.
        assert!(
            refresh_accounts_for("default").iter().any(|a| a == LEGACY_ACCOUNT),
            "the pre-persona slot can resurrect a session that was signed out of"
        );
    }

    #[test]
    fn clearing_a_rejected_supabase_token_leaves_the_forge_token_alone() {
        // The two credentials have different lifetimes and different blast
        // radii, which is why they have separate slots. A dōjō that rejects our
        // refresh token says nothing about GitHub's, so a 401 on the refresh leg
        // must not throw away a working forge token — only an explicit sign-out
        // does that.
        let on_rejection = refresh_accounts_for("default");
        assert!(!on_rejection.contains(&provider_account_for("default")));
        assert!(!on_rejection.contains(&provider_refresh_account_for("default")));
    }

    #[test]
    fn the_forge_refresh_token_round_trips_through_its_own_slot() {
        // The write existed from the first sign-in; the READ did not, so the one
        // credential that can renew a dying session sat in the Keychain with no
        // code path able to reach it. This pins the pair to the same slot — a
        // loader reading a different account name returns "not found" forever,
        // which reads exactly like "no refresh token was ever issued".
        let persona = "round-trip-test";
        let token = "ghr_roundtrip_probe";
        // Skipped rather than failed where the Keychain is unavailable (CI, a
        // locked login keychain): the assertion is about slot agreement, and a
        // machine that cannot store secrets is not evidence against it.
        if store_provider_refresh_token(persona, token).is_err() {
            return;
        }
        assert_eq!(load_provider_refresh_token(persona).ok().as_deref(), Some(token));
        // And it must NOT collide with the access-token slot.
        assert_ne!(provider_refresh_account_for(persona), provider_account_for(persona));
        let _ = clear_session(persona);
    }

    #[test]
    fn the_persona_slot_is_case_insensitive() {
        // The label is user-chosen and reaches here from a query string, so
        // "Sensei-HQ" and "sensei-hq" must not become two half-signed-in states.
        assert_eq!(account_for("Sensei-HQ"), account_for("sensei-hq"));
    }
}
