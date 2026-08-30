//! Forge-token liveness: what state a persona's GitHub token is in, and whether
//! acting on it still makes sense.
//!
//! ## Why this exists
//!
//! The GitHub token expires — observed, not assumed: a token minted at sign-in
//! was `401 Bad credentials` the next morning, and `provider_refresh.<slot>` is
//! present in the Keychain, which GitHub issues only when expiry is enabled.
//! Nothing renewed it and nothing recorded that it had died, so every
//! forge-dependent operation degraded while `/api/auth/status` kept reporting
//! `signedIn: true` (that flag reflects the SUPABASE session, which refreshes on
//! every use and is entirely separate).
//!
//! ## Why the decision is separate from the doing
//!
//! The scheduler answers "is this task DUE" from `last_run_at + interval`, so a
//! task overdue after downtime simply runs on the next tick. That is the right
//! behaviour for catching up and the wrong behaviour for acting blindly: a
//! refresh scheduled for the 7th hour of an 8-hour token, running at the 9th
//! because the machine was asleep, would spend a call on a credential the forge
//! has already dropped — and a failed refresh in the logs reads like a transient
//! network problem rather than "sign in again".
//!
//! So being due and being worth doing are two questions. This module answers the
//! second, as a pure function, so it can be tested without a scheduler, a
//! Keychain or a network.

/// What we currently believe about a persona's forge token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenState {
    /// A token is stored and has not been observed dead.
    Active,
    /// Observed dead — expired or revoked. Only a fresh sign-in clears this.
    Dead,
    /// No token stored for this persona at all.
    Absent,
}

/// What the scheduled task should do THIS run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeTokenAction {
    /// Still alive: exchange the refresh token for a new access token.
    Refresh,
    /// Believed expired: do NOT spend a refresh. Confirm with the forge and
    /// record the death, so the UI can say "sign in again" instead of leaving
    /// every later failure to be mistaken for a network problem.
    VerifyAndMarkDead,
    /// Expiry unknown: ask the forge what is true rather than guessing. Assuming
    /// alive spends a doomed refresh; assuming dead tells the user to sign in
    /// again for nothing.
    Verify,
    /// Nothing to do, and nothing to learn by trying.
    Skip,
}

/// Decide what to do with a persona's forge token, given what we know.
///
/// `expires_at` / `now` are unix seconds. `expires_at` is `None` for a token
/// captured before the expiry was recorded — GoTrue's exchange does not report
/// the provider's expiry, so it has to come from the forge itself.
///
/// The expiry test is HALF-OPEN: at the instant of expiry the token is treated
/// as gone. Rounding the other way spends a refresh on a credential the forge
/// has already dropped, and the failure surfaces as an unexplained 401.
pub fn forge_token_action(
    expires_at: Option<i64>,
    now: i64,
    state: TokenState,
) -> ForgeTokenAction {
    match state {
        // Re-probing a token already known dead is a per-interval call to GitHub
        // that cannot change the answer. Only a sign-in can.
        TokenState::Dead | TokenState::Absent => ForgeTokenAction::Skip,
        TokenState::Active => match expires_at {
            Some(exp) if now >= exp => ForgeTokenAction::VerifyAndMarkDead,
            // Near the end of its life — renew now, while the refresh can still
            // be spent. NOT on every run: see `REFRESH_MARGIN_SECS`.
            Some(exp) if exp - now <= REFRESH_MARGIN_SECS => ForgeTokenAction::Refresh,
            // Alive with hours left. Still worth asking about: an expiry cannot
            // predict a REVOCATION, and a probe is one cheap call.
            Some(_) => ForgeTokenAction::Verify,
            None => ForgeTokenAction::Verify,
        },
    }
}

/// How close to expiry a token must be before a refresh is spent on it.
///
/// Two competing costs. Refreshing too EAGERLY redeems on every run — GitHub
/// rotates the refresh token each time, so a response lost in flight leaves the
/// daemon holding a token GitHub has already replaced, and the only recovery is
/// a sign-in. Refreshing too LATE misses the window entirely.
///
/// One hour against the 1800s schedule gives two runs inside the window, so a
/// single missed pass — a suspended laptop, a slow tick, a restart — does not
/// cost the renewal. Measured lifetime is 8h, so this renews in the last eighth.
pub const REFRESH_MARGIN_SECS: i64 = 3600;

#[cfg(test)]
mod forge_token_decision {
    use super::*;

    const H: i64 = 3600;

    /// The scheduler runs what is DUE. This decides whether running still makes
    /// sense — the user's requirement: "a refresh token task scheduled after 8
    /// hours runs at the 9th hour, token is dead, so it may not make sense to do
    /// a refresh, just mark it as dead or verify dead and mark".
    #[test]
    fn on_time_while_the_token_is_still_alive_it_refreshes() {
        // Scheduled for 7h against an 8h token; the daemon was up, it runs on time.
        assert_eq!(
            forge_token_action(Some(8 * H), 7 * H, TokenState::Active),
            ForgeTokenAction::Refresh
        );
    }

    #[test]
    fn a_token_with_hours_left_is_verified_rather_than_refreshed() {
        // The rule this function had at first was "alive and not yet expired ->
        // Refresh", which only looked right while refresh was unimplemented.
        // GitHub ROTATES the refresh token on every redemption, and the check
        // runs every 30 minutes: an 8h token would have been redeemed ~16 times
        // per lifetime, each rotation a chance to lose the new token to a dropped
        // response and strand the user at a sign-in prompt.
        //
        // Verifying is the cheap thing that is still worth doing — it catches a
        // revocation the expiry cannot predict.
        assert_eq!(
            forge_token_action(Some(8 * H), H, TokenState::Active),
            ForgeTokenAction::Verify
        );
    }

    #[test]
    fn the_refresh_window_is_wider_than_the_check_interval() {
        // The window must span more than one tick or the only run inside it can
        // be missed — the daemon asleep, a slow pass, a machine suspended — and
        // the next run finds the token already dead. Two chances, minimum.
        //
        // The interval is asserted against the SEEDED value in
        // `database/import/staging/schedules.jsonl`, not a constant restated
        // here: a copy would keep agreeing with itself while someone shortens
        // the real schedule to 30 minutes and silently leaves one chance to
        // catch the window.
        let seed = include_str!("../../../../database/import/staging/schedules.jsonl");
        let line = seed
            .lines()
            .find(|l| l.contains("\"forge_token\""))
            .expect("the forge_token schedule must be seeded");
        let interval: i64 = serde_json::from_str::<serde_json::Value>(line)
            .expect("a seed line must be JSON")["interval_secs"]
            .as_i64()
            .expect("interval_secs must be a number");
        assert!(
            REFRESH_MARGIN_SECS >= 2 * interval,
            "margin {REFRESH_MARGIN_SECS}s must cover two runs of the seeded {interval}s schedule"
        );
        // Just inside the window: refresh.
        assert_eq!(
            forge_token_action(Some(8 * H), 8 * H - REFRESH_MARGIN_SECS + 1, TokenState::Active),
            ForgeTokenAction::Refresh
        );
        // Just outside it: not yet.
        assert_eq!(
            forge_token_action(Some(8 * H), 8 * H - REFRESH_MARGIN_SECS - 1, TokenState::Active),
            ForgeTokenAction::Verify
        );
    }

    #[test]
    fn late_past_expiry_it_does_not_attempt_a_refresh() {
        // The 9th-hour case. Burning a call on a credential we KNOW is dead teaches
        // the user nothing and can look like a working refresh loop in the logs.
        assert_eq!(
            forge_token_action(Some(8 * H), 9 * H, TokenState::Active),
            ForgeTokenAction::VerifyAndMarkDead
        );
    }

    #[test]
    fn exactly_at_expiry_is_treated_as_dead_not_alive() {
        // Half-open, like the billing period: at the instant it expires it is gone.
        // Guessing the other way spends a refresh on a token the forge has dropped.
        assert_eq!(
            forge_token_action(Some(8 * H), 8 * H, TokenState::Active),
            ForgeTokenAction::VerifyAndMarkDead
        );
    }

    #[test]
    fn an_unknown_expiry_verifies_rather_than_assuming_either_way() {
        // GoTrue hands us no provider expiry, so a token signed in before this
        // column existed has none. Assuming ALIVE spends a doomed refresh;
        // assuming DEAD tells the user to sign in again for nothing.
        assert_eq!(forge_token_action(None, 5 * H, TokenState::Active), ForgeTokenAction::Verify);
    }

    #[test]
    fn a_token_already_known_dead_is_left_alone() {
        // Nothing to refresh and nothing new to learn — re-probing a dead token
        // every tick is a per-interval call to GitHub that cannot change anything.
        // Only a fresh sign-in clears this.
        assert_eq!(
            forge_token_action(Some(8 * H), 9 * H, TokenState::Dead),
            ForgeTokenAction::Skip
        );
        assert_eq!(forge_token_action(None, 9 * H, TokenState::Dead), ForgeTokenAction::Skip);
    }

    #[test]
    fn there_is_no_token_at_all_so_there_is_nothing_to_do() {
        assert_eq!(forge_token_action(None, 0, TokenState::Absent), ForgeTokenAction::Skip);
    }
}

/// What a probe of the forge learned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// The forge accepted the token. `expires_at` is what it said, when it said
    /// anything — GitHub sends `github-authentication-token-expiration` on some
    /// responses and not others.
    Alive { expires_at: Option<i64> },
    /// The forge REFUSED the credential. Definitive.
    Dead,
    /// We could not ask. Says nothing about the token.
    Unreachable,
}

/// Classify a probe response.
///
/// The distinction that matters: **only the forge refusing the credential means
/// dead.** A timeout, a DNS failure, a 500, a captive portal — none of those are
/// evidence about the token, and recording `dead` on any of them would tell the
/// user to sign in again because their wifi dropped. `dead` is the one state a
/// sign-in is needed to leave, so it must never be entered on a guess.
pub fn classify_probe(status: Option<u16>, expiry_header: Option<&str>) -> ProbeOutcome {
    match status {
        // GitHub answers 401 for a bad credential. 403 is a live token being
        // refused a resource — a scope or rate-limit problem, not a dead token,
        // and re-signing-in would not fix it.
        Some(401) => ProbeOutcome::Dead,
        Some(s) if (200..300).contains(&s) => {
            ProbeOutcome::Alive { expires_at: expiry_header.and_then(parse_expiry) }
        }
        // Any other status, or no response at all: we learned nothing.
        _ => ProbeOutcome::Unreachable,
    }
}

/// GitHub's `github-authentication-token-expiration` is an RFC3339-ish stamp
/// (`2026-08-30 12:00:00 UTC`). Unparseable means we learned no expiry — never a
/// fabricated one, because a wrong deadline schedules a refresh at the wrong
/// moment and looks exactly like a correct one.
fn parse_expiry(raw: &str) -> Option<i64> {
    let cleaned = raw.trim().replace(" UTC", "Z").replace(' ', "T");
    chrono::DateTime::parse_from_rfc3339(&cleaned).ok().map(|t| t.timestamp())
}

#[cfg(test)]
mod probe_classification {
    use super::*;

    #[test]
    fn only_a_401_means_dead() {
        assert_eq!(classify_probe(Some(401), None), ProbeOutcome::Dead);
    }

    #[test]
    fn a_403_is_not_dead_it_is_a_live_token_refused_a_resource() {
        // Scope or rate limit. Signing in again does not fix it, and marking the
        // token dead would send the user to do exactly that.
        assert_eq!(classify_probe(Some(403), None), ProbeOutcome::Unreachable);
    }

    #[test]
    fn a_network_failure_says_nothing_about_the_token() {
        // The important one. `dead` is the state only a sign-in can leave, so
        // entering it because the wifi dropped strands a perfectly good token.
        assert_eq!(classify_probe(None, None), ProbeOutcome::Unreachable);
        assert_eq!(classify_probe(Some(500), None), ProbeOutcome::Unreachable);
        assert_eq!(classify_probe(Some(502), None), ProbeOutcome::Unreachable);
    }

    #[test]
    fn a_success_carries_the_expiry_when_the_forge_states_one() {
        assert_eq!(
            classify_probe(Some(200), Some("2026-08-30 12:00:00 UTC")),
            ProbeOutcome::Alive { expires_at: Some(1_788_091_200) }
        );
    }

    #[test]
    fn a_success_without_the_header_is_alive_with_an_unknown_expiry() {
        // Not an error, and not a guess. The store keeps any expiry it already
        // had rather than overwriting it with None.
        assert_eq!(classify_probe(Some(200), None), ProbeOutcome::Alive { expires_at: None });
    }

    #[test]
    fn an_unparseable_expiry_is_dropped_not_invented() {
        // A wrong deadline schedules the refresh at the wrong moment and looks
        // exactly like a right one.
        assert_eq!(
            classify_probe(Some(200), Some("whenever")),
            ProbeOutcome::Alive { expires_at: None }
        );
    }
}

/// Record what a forge response implies about the token, opportunistically.
///
/// The scheduled check is not the only thing that talks to GitHub — sign-in
/// reads `/user/orgs` and `/user/emails`, and provisioning reads repositories.
/// Every one of those responses carries the same evidence, so learning from them
/// means a token gets its deadline recorded at SIGN-IN rather than up to a
/// scheduling interval later. It also means a token that dies between checks is
/// noticed by the next thing that uses it, not by the next check.
///
/// Best-effort by construction: this observes a call made for another purpose
/// and must never change its outcome. A write failure is logged, not propagated
/// — the caller's own result is what matters, and failing an org list because a
/// bookkeeping UPDATE failed would trade a working feature for a diagnostic.
///
/// `Unreachable` writes NOTHING. The caller may be handling its own network
/// error; recording a standing we did not learn would be a fabrication.
pub async fn observe(
    pg: &crate::db::pg_store::PgStore,
    session_slot: &str,
    status: Option<u16>,
    expiry_header: Option<&str>,
) {
    let (state, expires_at) = match classify_probe(status, expiry_header) {
        ProbeOutcome::Alive { expires_at } => ("active", expires_at),
        ProbeOutcome::Dead => ("dead", None),
        ProbeOutcome::Unreachable => return,
    };
    if let Err(e) = pg.set_forge_token_state(session_slot, state, expires_at).await {
        tracing::debug!(slot = session_slot, error = %e,
                        "forge token: could not record what this call implied");
    }
}

/// The header GitHub sets when a token has a deadline.
pub const EXPIRY_HEADER: &str = "github-authentication-token-expiration";
