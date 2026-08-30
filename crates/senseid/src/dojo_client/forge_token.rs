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
            Some(_) => ForgeTokenAction::Refresh,
            None => ForgeTokenAction::Verify,
        },
    }
}

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
