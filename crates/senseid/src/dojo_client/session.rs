//! The daemon's Supabase session: exchanging a PKCE code, refreshing, and
//! keeping the refresh token where it belongs.
//!
//! ## Where the token lives
//!
//! The Keychain, never Postgres. A refresh token IS the user in dōjō — long
//! lived and broad — so the secret sits in the OS keychain and the database
//! holds only a handle (`personas.session_slot`).
//!
//! Access tokens are deliberately NOT persisted. They expire in an hour, and a
//! stored one is a credential at rest with no upside — it is cheaper to refresh
//! than to protect.

use serde::{Deserialize, Serialize};

use crate::gateway_keys::{KeychainError, keychain_get, keychain_set};

/// Keychain service namespace, matching `com.sensei.gateway.router.*`.
const KEYCHAIN_SERVICE: &str = "com.sensei.supabase";

/// The pre-#302 slot for a persona's dōjō refresh token, read only to migrate
/// it into the persona's [`Credentials`].
///
/// PER PERSONA, not one per install. A single slot meant signing in as a second
/// identity silently EVICTED the first — observed live: signing in as
/// hi@sensei-hq.com and then as me@devuser.name left only the second, with
/// no indication the first had gone. That directly contradicts the point of
/// personas, which exist because a user keeps working identities apart and needs
/// both linked at once.
fn account_for(persona: &str) -> String {
    format!("refresh_token.{}", persona.to_lowercase())
}

/// The pre-#302 slot for the PROVIDER token (GitHub's). Read only to migrate it
/// into the persona's [`Credentials`], where it stays a field of its own:
/// revoking the dōjō session still keeps it ([`clear_refresh_token`]).
fn provider_account_for(persona: &str) -> String {
    format!("provider_token.{}", persona.to_lowercase())
}

/// Persist the GitHub token. Keychain, never Postgres — same rule as the refresh
/// token, and more important here because this one reaches GitHub directly.
pub fn store_provider_token(
    persona: &str,
    token: &str,
) -> Result<(), crate::gateway_keys::KeychainError> {
    update_credentials_with(&Keychain, persona, |c| c.provider_token = Some(token.to_string()))
}

/// The GitHub token for a persona, if the sign-in captured one.
pub fn load_provider_token(persona: &str) -> Result<String, crate::gateway_keys::KeychainError> {
    load_credentials(persona)?.provider_token.ok_or(KeychainError::NotFound)
}

/// Slot for GitHub's own refresh token, when the OAuth App issues expiring ones.
fn provider_refresh_account_for(persona: &str) -> String {
    format!("provider_refresh.{}", persona.to_lowercase())
}

/// Discard GitHub's refresh token, and any previously stored for this persona.
///
/// This slot used to be WRITTEN at every sign-in and read by nothing. Renewal
/// now runs the authorize flow Supabase owns rather than redeeming the refresh
/// token directly, because redeeming needs the App's client secret and that
/// secret stays in exactly one place — Supabase's provider config. Duplicating
/// it into a second service would mean recreating the App credential in two
/// dashboards, with the missed copy failing silently months later.
///
/// So this credential can never be spent by anything we run. Keeping it would
/// leave a 182-day GitHub grant at rest with no consumer and no upside — the
/// same reasoning that already keeps access tokens out of the Keychain.
/// Deleting on sign-in also clears the ones earlier builds stored.
///
/// A missing item is SUCCESS: this runs on every sign-in, and the common case is
/// nothing to remove.
pub fn discard_provider_refresh_token(
    persona: &str,
) -> Result<(), crate::gateway_keys::KeychainError> {
    delete_one(&provider_refresh_account_for(persona))
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
/// Talks to the OS credential store synchronously. Async callers must wrap this
/// in `spawn_blocking`, exactly as `gateway_keys` documents.
pub fn store_refresh_token(
    persona: &str,
    token: &str,
) -> Result<(), crate::gateway_keys::KeychainError> {
    update_credentials_with(&Keychain, persona, |c| c.refresh_token = Some(token.to_string()))
}

/// The single un-namespaced slot used before sessions became per-persona.
const LEGACY_ACCOUNT: &str = "refresh_token";

/// Read the stored refresh token, if this persona has signed in.
pub fn load_refresh_token(persona: &str) -> Result<String, crate::gateway_keys::KeychainError> {
    load_credentials(persona)?.refresh_token.ok_or(KeychainError::NotFound)
}

/// Everything a persona's sign-in stored, as ONE keychain item (#302).
///
/// Each keychain item carries its own access list, so a persona kept as two
/// items asked for approval twice. One item means one prompt, and one
/// "Always Allow" covers everything sync reads.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Credentials {
    /// The dōjō (Supabase) refresh token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// GitHub's token, when the sign-in captured one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_token: Option<String>,
}

impl Credentials {
    fn is_empty(&self) -> bool {
        self.refresh_token.is_none() && self.provider_token.is_none()
    }
}

/// The account holding a persona's [`Credentials`].
fn credentials_account_for(persona: &str) -> String {
    format!("credentials.{}", persona.to_lowercase())
}

/// The keychain, as the session code sees it — a seam so the migration and its
/// never-drop-a-credential rules are testable without one.
trait SlotStore {
    fn get(&self, account: &str) -> Result<String, KeychainError>;
    fn set(&self, account: &str, secret: &str) -> Result<(), KeychainError>;
    fn delete(&self, account: &str) -> Result<(), KeychainError>;
}

/// The OS keychain under [`KEYCHAIN_SERVICE`].
struct Keychain;

impl SlotStore for Keychain {
    fn get(&self, account: &str) -> Result<String, KeychainError> {
        keychain_get(KEYCHAIN_SERVICE, account)
    }
    fn set(&self, account: &str, secret: &str) -> Result<(), KeychainError> {
        keychain_set(KEYCHAIN_SERVICE, account, secret)
    }
    fn delete(&self, account: &str) -> Result<(), KeychainError> {
        delete_one(account)
    }
}

/// A persona's credentials — one keychain read.
pub fn load_credentials(persona: &str) -> Result<Credentials, KeychainError> {
    load_credentials_with(&Keychain, persona)
}

/// `Ok(None)` for a missing item; any other failure is the caller's to see.
fn read_optional(store: &impl SlotStore, account: &str) -> Result<Option<String>, KeychainError> {
    match store.get(account) {
        Ok(v) => Ok(Some(v)),
        Err(KeychainError::NotFound) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Read the combined item, migrating the old per-token items into it the first
/// time. `NotFound` only when nothing at all is stored.
///
/// The migration never drops a credential it could not read: any old item that
/// fails to read fails the whole load and nothing is written or deleted. The old
/// items go only after the combined one is written; a failed write leaves them
/// in place and the next read tries again.
fn load_credentials_with(
    store: &impl SlotStore,
    persona: &str,
) -> Result<Credentials, KeychainError> {
    let account = credentials_account_for(persona);
    if let Some(json) = read_optional(store, &account)? {
        return serde_json::from_str(&json).map_err(|e| {
            KeychainError::Store(format!("{account} is not a credentials record: {e}"))
        });
    }
    let refresh_token = match read_optional(store, &account_for(persona))? {
        Some(t) => Some(t),
        None => read_optional(store, LEGACY_ACCOUNT)?,
    };
    let provider_token = read_optional(store, &provider_account_for(persona))?;
    let creds = Credentials { refresh_token, provider_token };
    if creds.is_empty() {
        return Err(KeychainError::NotFound);
    }
    match write_credentials(store, persona, &creds) {
        Ok(()) => {
            for old in
                [account_for(persona), LEGACY_ACCOUNT.to_string(), provider_account_for(persona)]
            {
                if let Err(e) = store.delete(&old) {
                    tracing::warn!(persona, account = %old, error = %e, "could not remove a migrated keychain item");
                }
            }
            tracing::info!(persona, "moved the persona's keychain items into one");
        }
        Err(e) => {
            tracing::warn!(persona, error = %e, "could not write the combined keychain item; the old ones stay");
        }
    }
    Ok(creds)
}

fn write_credentials(
    store: &impl SlotStore,
    persona: &str,
    creds: &Credentials,
) -> Result<(), KeychainError> {
    let json = serde_json::to_string(creds).map_err(|e| KeychainError::Store(e.to_string()))?;
    store.set(&credentials_account_for(persona), &json)
}

/// Change one field of a persona's credentials and write them back. A record
/// that cannot be read is NOT overwritten — that would drop whichever credential
/// was unreadable.
fn update_credentials_with(
    store: &impl SlotStore,
    persona: &str,
    change: impl FnOnce(&mut Credentials),
) -> Result<(), KeychainError> {
    let mut creds = match load_credentials_with(store, persona) {
        Ok(c) => c,
        Err(KeychainError::NotFound) => Credentials::default(),
        Err(e) => return Err(e),
    };
    change(&mut creds);
    if creds.is_empty() {
        return store.delete(&credentials_account_for(persona));
    }
    write_credentials(store, persona, &creds)
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
    all.push(credentials_account_for(persona));
    all.push(provider_account_for(persona));
    all.push(provider_refresh_account_for(persona));
    all
}

/// Remove ONE Keychain entry.
///
/// A MISSING entry is success — the goal is "no token stored", and that already
/// holds.
fn delete_one(account: &str) -> Result<(), crate::gateway_keys::KeychainError> {
    crate::gateway_keys::keychain_delete(KEYCHAIN_SERVICE, account)
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
    clear_refresh_token_in(&Keychain, persona)
}

/// [`clear_refresh_token`] against a given store: the old slots go, and the
/// combined record keeps GitHub's token without the rejected one.
fn clear_refresh_token_in(store: &impl SlotStore, persona: &str) -> Result<(), KeychainError> {
    let old = clear_refresh_token_with(persona, |a| store.delete(a));
    let record = update_credentials_with(store, persona, |c| c.refresh_token = None);
    old.and(record)
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
        assert_ne!(account_for("sensei-hq"), account_for("devuser"));
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
    fn discarding_the_forge_refresh_token_removes_it_and_leaves_the_others() {
        // The credential is unspendable under the current design — renewal goes
        // through Supabase's authorize flow, not `grant_type=refresh_token` —
        // so sign-in deletes it rather than storing a 182-day GitHub grant that
        // nothing can use.
        let persona = "discard-test";
        // Skipped where the Keychain is unavailable (CI, a locked login
        // keychain). A machine that cannot store secrets is not evidence.
        if keychain_set(KEYCHAIN_SERVICE, &provider_refresh_account_for(persona), "ghr_x").is_err()
        {
            return;
        }
        let _ = store_provider_token(persona, "gho_keep");

        assert!(discard_provider_refresh_token(persona).is_ok());
        assert!(
            matches!(
                keychain_get(KEYCHAIN_SERVICE, &provider_refresh_account_for(persona)),
                Err(crate::gateway_keys::KeychainError::NotFound)
            ),
            "the refresh slot must be gone"
        );
        // The ACCESS token is a different credential with a live consumer and
        // must survive: deleting it here would sign the user out of the forge on
        // every sign-in.
        assert_eq!(load_provider_token(persona).ok().as_deref(), Some("gho_keep"));

        // Idempotent — it runs on every sign-in, and usually there is nothing
        // to remove. A missing item must not be an error.
        assert!(discard_provider_refresh_token(persona).is_ok());
        let _ = clear_session(persona);
    }

    // ── one keychain item per persona (#302) ──────────────────────────

    /// An in-memory keychain that counts reads and can refuse some of them.
    #[derive(Default)]
    struct MemStore {
        items: std::cell::RefCell<std::collections::BTreeMap<String, String>>,
        reads: std::cell::RefCell<Vec<String>>,
        unreadable: Vec<String>,
        refuse_writes: bool,
    }

    impl MemStore {
        fn with(items: &[(&str, &str)]) -> Self {
            let s = Self::default();
            for (k, v) in items {
                s.items.borrow_mut().insert(k.to_string(), v.to_string());
            }
            s
        }
        fn has(&self, account: &str) -> bool {
            self.items.borrow().contains_key(account)
        }
        fn accounts(&self) -> Vec<String> {
            self.items.borrow().keys().cloned().collect()
        }
    }

    impl SlotStore for MemStore {
        fn get(&self, account: &str) -> Result<String, crate::gateway_keys::KeychainError> {
            self.reads.borrow_mut().push(account.to_string());
            if self.unreadable.iter().any(|a| a == account) {
                return Err(crate::gateway_keys::KeychainError::NeedsApproval);
            }
            self.items
                .borrow()
                .get(account)
                .cloned()
                .ok_or(crate::gateway_keys::KeychainError::NotFound)
        }
        fn set(
            &self,
            account: &str,
            secret: &str,
        ) -> Result<(), crate::gateway_keys::KeychainError> {
            if self.refuse_writes {
                return Err(crate::gateway_keys::KeychainError::NeedsApproval);
            }
            self.items.borrow_mut().insert(account.to_string(), secret.to_string());
            Ok(())
        }
        fn delete(&self, account: &str) -> Result<(), crate::gateway_keys::KeychainError> {
            self.items.borrow_mut().remove(account);
            Ok(())
        }
    }

    fn old_items(slot: &str) -> MemStore {
        MemStore::with(&[
            (&account_for(slot), "sb_refresh"),
            (&provider_account_for(slot), "gho_token"),
        ])
    }

    #[test]
    fn a_persona_with_old_items_is_migrated_into_one() {
        let store = old_items("p");
        let c = load_credentials_with(&store, "p").expect("readable");
        assert_eq!(c.refresh_token.as_deref(), Some("sb_refresh"));
        assert_eq!(c.provider_token.as_deref(), Some("gho_token"));
        assert_eq!(store.accounts(), vec![credentials_account_for("p")], "the old items are gone");
    }

    #[test]
    fn approving_a_migrated_persona_reads_exactly_one_item() {
        // The prompt count IS the read count: each item has its own ACL.
        let store = old_items("p");
        load_credentials_with(&store, "p").expect("migrates");
        store.reads.borrow_mut().clear();

        let c = load_credentials_with(&store, "p").expect("readable");
        assert_eq!(*store.reads.borrow(), vec![credentials_account_for("p")]);
        assert_eq!(c.provider_token.as_deref(), Some("gho_token"), "and keeps working");
    }

    #[test]
    fn an_unreadable_old_item_is_never_dropped() {
        let mut store = old_items("p");
        store.unreadable = vec![provider_account_for("p")];
        assert!(load_credentials_with(&store, "p").is_err(), "the failure is surfaced");
        assert!(store.has(&account_for("p")) && store.has(&provider_account_for("p")));
        assert!(!store.has(&credentials_account_for("p")), "nothing half-migrated");
    }

    #[test]
    fn a_failed_combined_write_keeps_the_old_items_and_still_reads() {
        let mut store = old_items("p");
        store.refuse_writes = true;
        let c = load_credentials_with(&store, "p").expect("the read itself succeeded");
        assert_eq!(c.refresh_token.as_deref(), Some("sb_refresh"));
        assert!(store.has(&account_for("p")) && store.has(&provider_account_for("p")));
    }

    #[test]
    fn the_pre_persona_slot_is_migrated_too() {
        let store = MemStore::with(&[(LEGACY_ACCOUNT, "sb_old")]);
        let c = load_credentials_with(&store, "p").expect("readable");
        assert_eq!(c.refresh_token.as_deref(), Some("sb_old"));
        assert!(c.provider_token.is_none());
        assert_eq!(store.accounts(), vec![credentials_account_for("p")]);
    }

    #[test]
    fn a_persona_that_never_signed_in_has_nothing() {
        let store = MemStore::default();
        assert!(matches!(
            load_credentials_with(&store, "p"),
            Err(crate::gateway_keys::KeychainError::NotFound)
        ));
        assert!(store.accounts().is_empty(), "and nothing is written");
    }

    #[test]
    fn a_rotated_dojo_token_keeps_the_github_token() {
        let store = old_items("p");
        update_credentials_with(&store, "p", |c| c.refresh_token = Some("sb_rotated".into()))
            .expect("written");
        let c = load_credentials_with(&store, "p").unwrap();
        assert_eq!(c.refresh_token.as_deref(), Some("sb_rotated"));
        assert_eq!(c.provider_token.as_deref(), Some("gho_token"));
    }

    #[test]
    fn a_record_that_cannot_be_read_is_not_overwritten() {
        // Writing over it would drop whichever credential we could not see.
        let mut store =
            MemStore::with(&[(&credentials_account_for("p"), r#"{"provider_token":"gho"}"#)]);
        store.unreadable = vec![credentials_account_for("p")];
        assert!(
            update_credentials_with(&store, "p", |c| c.refresh_token = Some("x".into())).is_err()
        );
        assert_eq!(
            store.items.borrow()[&credentials_account_for("p")],
            r#"{"provider_token":"gho"}"#
        );
    }

    #[test]
    fn a_rejected_dojo_session_keeps_the_github_token_in_the_record() {
        let store = old_items("p");
        clear_refresh_token_in(&store, "p").expect("cleared");
        let c = load_credentials_with(&store, "p").unwrap();
        assert!(c.refresh_token.is_none(), "the rejected token is gone");
        assert_eq!(c.provider_token.as_deref(), Some("gho_token"), "GitHub's is untouched");
    }

    #[test]
    fn signing_out_also_takes_the_combined_record() {
        let mut seen = Vec::new();
        clear_session_with("p", recording_deleter(&mut seen, &[])).expect("ok");
        assert!(seen.contains(&credentials_account_for("p")), "{seen:?}");
    }

    #[test]
    fn the_persona_slot_is_case_insensitive() {
        // The label is user-chosen and reaches here from a query string, so
        // "Sensei-HQ" and "sensei-hq" must not become two half-signed-in states.
        assert_eq!(account_for("Sensei-HQ"), account_for("sensei-hq"));
    }
}
