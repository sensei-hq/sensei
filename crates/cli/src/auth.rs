//! `sensei auth` — see the forge credential's standing, and renew it.
//!
//! ## Why this exists
//!
//! Until now NOTHING called `/api/auth/signin`. Not the desktop app, not the
//! CLI — the only caller was a hand-written `curl`, which is how the one
//! credential the whole sync path depends on came to expire overnight with no
//! surface anywhere that could renew it or even report it was gone.
//!
//! ## Why renewal is a browser round trip
//!
//! Redeeming GitHub's refresh token directly needs the App's client secret, and
//! that secret stays in exactly one place — Supabase's auth provider config.
//! Copying it into a second service would mean recreating the App credential in
//! two dashboards, and the copy that got missed would fail silently months later
//! as a token that stops renewing.
//!
//! So renewal re-runs the authorize flow Supabase already performs. For a user
//! who has already authorized the App and still has live GitHub and Supabase
//! browser sessions, that is a redirect chain with no prompt.

use serde_json::Value;

/// What `sensei auth` was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthAction {
    /// Report the standing and exit. Never opens a browser.
    Status,
    /// Start the authorize flow now, whatever the standing.
    Renew,
    /// Start it only if the token is dead or renewal is due.
    RenewIfNeeded,
}

/// What the daemon's `/api/auth/status` says, reduced to a decision.
///
/// Kept as a pure function over the parsed body so every branch is testable
/// without a daemon, a browser or a network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    /// No session at all, or one the dōjō REJECTED. Signing in is the only
    /// option, and it will work.
    SignedOut,
    /// The daemon could not reach the dōjō. Says NOTHING about the credential —
    /// the stored session was deliberately left alone, and a browser opened for
    /// a 504 makes the user re-authenticate over someone else's outage.
    Unreachable(String),
    /// The forge token is dead — only a fresh authorize restores it.
    Dead,
    /// Alive but near expiry.
    RenewalDue { expires_at: Option<i64> },
    /// Alive with time to spare.
    Healthy { expires_at: Option<i64> },
    /// The daemon answered, but not with a shape we understand. NOT treated as
    /// healthy: reporting "fine" for an answer we could not read is how a broken
    /// endpoint looks identical to a working one.
    Unreadable(String),
}

/// Reduce a status body to a standing.
pub fn standing_of(body: &Value) -> Standing {
    if body.get("signedIn").and_then(Value::as_bool) != Some(true) {
        // NOT automatically "signed out". The daemon distinguishes a REJECTED
        // session (terminal — it already cleared the credential) from an
        // UNREACHABLE dōjō (transient — it deliberately kept it), and reports
        // which via the same `needsSignIn` flag the forge token uses. Reading
        // only `signedIn` conflates them, which is how a 504 from GoTrue opened
        // a browser and told a signed-in user to authenticate again.
        return match body.get("needsSignIn").and_then(Value::as_bool) {
            Some(true) => Standing::SignedOut,
            Some(false) => Standing::Unreachable(
                body["error"].as_str().unwrap_or("the dōjō could not be reached").to_string(),
            ),
            // An older daemon does not send the flag. Refusing to act is the
            // safe direction: the cost is a renewal that has to be asked for
            // explicitly, versus a browser opening on every transient blip.
            None => Standing::Unreachable(
                "this daemon does not report whether a sign-in is needed — reinstall it".into(),
            ),
        };
    }
    // `needsSignIn` is the daemon's own flag for "dead, and only you can fix
    // it". Trusted over re-deriving from the state string, so the CLI and the
    // UI cannot disagree about the same row.
    if body.get("needsSignIn").and_then(Value::as_bool) == Some(true) {
        return Standing::Dead;
    }
    let forge = &body["forgeToken"];
    let expires_at = forge["expiresAt"].as_i64();
    match forge["state"].as_str() {
        Some("active") | Some("unknown") => {
            match body.get("renewalDue").and_then(Value::as_bool) {
                Some(true) => Standing::RenewalDue { expires_at },
                Some(false) => Standing::Healthy { expires_at },
                // The field is missing — an older daemon. Say so rather than
                // guessing healthy, which would silently stop renewing.
                None => Standing::Unreadable(
                    "this daemon does not report `renewalDue` — reinstall it".into(),
                ),
            }
        }
        Some("absent") => Standing::Dead,
        Some(other) => Standing::Unreadable(format!("unrecognised forge token state `{other}`")),
        None => Standing::Unreadable("the daemon reported no forge token state".into()),
    }
}

/// Whether the action, given the standing, should open a browser.
///
/// Separated from the doing so the "did it need to?" decision is testable. The
/// asymmetry is deliberate: `RenewIfNeeded` runs on a schedule or a shell
/// profile, so it must NOT open a window every invocation; explicit `Renew` is a
/// direct instruction and always acts.
pub fn should_authorize(action: AuthAction, standing: &Standing) -> bool {
    match action {
        AuthAction::Status => false,
        AuthAction::Renew => true,
        AuthAction::RenewIfNeeded => {
            matches!(standing, Standing::Dead | Standing::SignedOut | Standing::RenewalDue { .. })
        }
    }
}

/// A one-line human summary of the standing.
pub fn describe(standing: &Standing, now: i64) -> String {
    let left = |exp: &Option<i64>| match exp {
        Some(e) => {
            let hours = (e - now) as f64 / 3600.0;
            if hours < 1.0 {
                format!("{:.0} minutes left", (hours * 60.0).max(0.0))
            } else {
                format!("{hours:.1} hours left")
            }
        }
        // No deadline recorded. Said plainly rather than shown as a number.
        None => "expiry not yet known".to_string(),
    };
    match standing {
        Standing::SignedOut => "not signed in".into(),
        // The daemon's message already names the dōjō; prefixing it again
        // produced "could not reach the dōjō — could not reach dōjō — …".
        Standing::Unreachable(why) => why.clone(),
        Standing::Dead => "forge token is DEAD — sign in again to restore sync".into(),
        Standing::RenewalDue { expires_at } => {
            format!("forge token expires soon ({}) — renewing", left(expires_at))
        }
        Standing::Healthy { expires_at } => {
            format!("forge token is healthy ({})", left(expires_at))
        }
        Standing::Unreadable(why) => format!("could not read the forge token standing: {why}"),
    }
}

/// Ask the daemon what it believes, and act on it.
///
/// The browser is opened by this command rather than by the daemon. The daemon
/// deliberately returns a URL instead of opening one — it may be running
/// headless, and a service that pops a window is a service that cannot run on a
/// build box. The CLI is invoked by a person, so it is the right place.
pub fn run(action: AuthAction, persona: &str) -> i32 {
    let base = crate::daemon_url();
    let status: Value = match crate::client_with_timeout(45)
        .get(format!("{base}/api/auth/status?persona={persona}"))
        .send()
        .and_then(reqwest::blocking::Response::json)
    {
        Ok(v) => v,
        // The daemon is the only thing that knows. Failing loudly beats
        // printing a standing we did not obtain.
        Err(e) => {
            eprintln!("could not reach the daemon at {base}: {e}");
            eprintln!("  start it with: sensei daemon");
            return 1;
        }
    };

    let standing = standing_of(&status);
    println!("{}", describe(&standing, chrono::Utc::now().timestamp()));

    if !should_authorize(action, &standing) {
        // An unreadable standing is a failure even when we were only asked to
        // report: exiting 0 would let a script conclude all is well.
        // A standing we could not establish exits NON-ZERO even though nothing
        // is provably wrong. Exiting 0 lets a script conclude the credential is
        // healthy on the strength of an answer we never got.
        return i32::from(matches!(standing, Standing::Unreadable(_) | Standing::Unreachable(_)));
    }

    let started: Value = match crate::client_with_timeout(45)
        .post(format!("{base}/api/auth/signin?persona={persona}"))
        .send()
        .and_then(reqwest::blocking::Response::json)
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("could not start the authorize flow: {e}");
            return 1;
        }
    };
    let Some(url) = started["authorizeUrl"].as_str() else {
        let why = started["error"].as_str().unwrap_or("no authorizeUrl in the response");
        eprintln!("the daemon refused to start a sign-in: {why}");
        return 1;
    };

    println!("opening the authorize page…");
    println!("  if GitHub does not prompt, the renewal was silent and the token is already live");
    if let Err(e) = open_url(url) {
        // Printed so the flow is still completable by hand. A failed opener
        // must not strand the user with no way to finish.
        eprintln!("could not open a browser ({e}) — open this yourself:\n  {url}");
        return 1;
    }
    println!("check the result with: sensei auth status");
    0
}

/// Hand a URL to the platform's browser.
fn open_url(url: &str) -> std::io::Result<()> {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    let ok = std::process::Command::new(program).arg(url).status()?.success();
    match ok {
        true => Ok(()),
        false => Err(std::io::Error::other(format!("{program} exited non-zero"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const NOW: i64 = 1_788_120_000;

    #[test]
    fn a_dojo_outage_is_not_mistaken_for_a_signed_out_user() {
        // Measured, not hypothetical: GoTrue answered 504 request_timeout, the
        // daemon reported `signedIn:false` with "the stored session was left
        // alone", and this command opened a browser telling a signed-in user to
        // authenticate again — over an outage that had nothing to do with them.
        let body = json!({
            "signedIn": false,
            "needsSignIn": false,
            "error": "could not reach dōjō — the stored session was left alone",
            "detail": "dōjō returned 504"
        });
        let s = standing_of(&body);
        assert!(matches!(s, Standing::Unreachable(_)), "{s:?}");
        // The property that matters: no browser, for any action that is allowed
        // to decide for itself.
        assert!(!should_authorize(AuthAction::RenewIfNeeded, &s));
        assert!(!should_authorize(AuthAction::Status, &s));
    }

    #[test]
    fn the_outage_message_is_the_daemons_own_and_is_not_prefixed_twice() {
        // Observed: "could not reach the dōjō — could not reach dōjō — the
        // stored session was left alone". The daemon's sentence already names
        // the dōjō and says what it did with the session.
        let msg = describe(
            &Standing::Unreachable(
                "could not reach dōjō — the stored session was left alone".into(),
            ),
            NOW,
        );
        assert_eq!(msg, "could not reach dōjō — the stored session was left alone");
        assert_eq!(msg.matches("could not reach").count(), 1, "said once: {msg}");
    }

    #[test]
    fn a_rejected_session_still_asks_for_a_sign_in() {
        // The other half. The daemon has already CLEARED the credential, so a
        // sign-in is both necessary and sufficient — refusing to act here would
        // leave the user stuck with no path forward.
        let body = json!({
            "signedIn": false, "needsSignIn": true,
            "error": "stored session was rejected — sign in again"
        });
        assert_eq!(standing_of(&body), Standing::SignedOut);
        assert!(should_authorize(AuthAction::RenewIfNeeded, &Standing::SignedOut));
    }

    #[test]
    fn an_older_daemon_that_omits_the_flag_does_not_open_a_browser() {
        // Absent means unknown. Guessing "signed out" reintroduces exactly the
        // bug above on every daemon that has not been reinstalled yet.
        let s = standing_of(&json!({ "signedIn": false }));
        assert!(matches!(s, Standing::Unreachable(_)), "{s:?}");
        assert!(!should_authorize(AuthAction::RenewIfNeeded, &s));
    }

    #[test]
    fn a_signed_out_daemon_is_not_read_for_a_forge_token() {
        // The forge fields are absent when there is no session. Reading them
        // anyway would land in the `None` arm and report "unrecognised state",
        // burying the one fact that matters.
        assert_eq!(
            standing_of(&json!({ "signedIn": false, "needsSignIn": true })),
            Standing::SignedOut
        );
    }

    #[test]
    fn needs_sign_in_wins_over_everything_else() {
        // A dead token can still carry a stale future `expiresAt` — the probe
        // records the death and deliberately preserves the last known deadline
        // as evidence. Reading the expiry first would call that healthy.
        let body = json!({
            "signedIn": true, "needsSignIn": true, "renewalDue": false,
            "forgeToken": { "state": "dead", "expiresAt": NOW + 9999 }
        });
        assert_eq!(standing_of(&body), Standing::Dead);
    }

    #[test]
    fn renewal_due_is_taken_from_the_daemon_not_recomputed() {
        // The margin lives in `forge_token_action`, once. If the CLI recomputed
        // "near expiry" from the timestamp it would drift from the scheduler the
        // moment either changed.
        let body = json!({
            "signedIn": true, "needsSignIn": false, "renewalDue": true,
            "forgeToken": { "state": "active", "expiresAt": NOW + 600 }
        });
        assert_eq!(standing_of(&body), Standing::RenewalDue { expires_at: Some(NOW + 600) });
    }

    #[test]
    fn a_daemon_that_does_not_report_renewal_due_is_flagged_not_assumed_healthy() {
        // An older binary. Defaulting to healthy would make `renew-if-needed`
        // silently never fire, and the token would die exactly as it did before
        // any of this existed — with the command reporting success.
        let body = json!({
            "signedIn": true, "forgeToken": { "state": "active", "expiresAt": NOW + 600 }
        });
        assert!(matches!(standing_of(&body), Standing::Unreadable(_)));
    }

    #[test]
    fn an_unrecognised_state_is_reported_rather_than_treated_as_fine() {
        let body = json!({
            "signedIn": true, "renewalDue": false,
            "forgeToken": { "state": "banana", "expiresAt": null }
        });
        assert!(matches!(standing_of(&body), Standing::Unreadable(_)));
    }

    #[test]
    fn status_never_opens_a_browser_whatever_the_standing() {
        // It is a read. A command that reports state must not have a side
        // effect a user did not ask for.
        for s in [
            Standing::SignedOut,
            Standing::Dead,
            Standing::RenewalDue { expires_at: Some(NOW) },
            Standing::Healthy { expires_at: Some(NOW) },
        ] {
            assert!(!should_authorize(AuthAction::Status, &s), "{s:?}");
        }
    }

    #[test]
    fn renew_if_needed_stays_quiet_while_the_token_is_healthy() {
        // The whole point of the variant: safe to run often. Opening a browser
        // on every invocation would make it unusable from a shell profile.
        assert!(!should_authorize(
            AuthAction::RenewIfNeeded,
            &Standing::Healthy { expires_at: Some(NOW + 25_000) }
        ));
        // And it must not act on an answer it could not read — that would open
        // a window every run against a daemon it does not understand.
        assert!(!should_authorize(
            AuthAction::RenewIfNeeded,
            &Standing::Unreadable("older daemon".into())
        ));
    }

    #[test]
    fn renew_if_needed_acts_on_dead_signed_out_and_due() {
        for s in [
            Standing::Dead,
            Standing::SignedOut,
            Standing::RenewalDue { expires_at: Some(NOW + 600) },
        ] {
            assert!(should_authorize(AuthAction::RenewIfNeeded, &s), "{s:?}");
        }
    }

    #[test]
    fn an_explicit_renew_acts_even_on_a_healthy_token() {
        // "I said renew." Refusing because we judged it unnecessary is the
        // command second-guessing a direct instruction.
        assert!(should_authorize(
            AuthAction::Renew,
            &Standing::Healthy { expires_at: Some(NOW + 25_000) }
        ));
    }

    #[test]
    fn the_summary_says_minutes_when_under_an_hour_and_never_a_negative() {
        assert!(
            describe(&Standing::RenewalDue { expires_at: Some(NOW + 1500) }, NOW)
                .contains("25 minutes")
        );
        assert!(
            describe(&Standing::Healthy { expires_at: Some(NOW + 25_200) }, NOW)
                .contains("7.0 hours")
        );
        // A past deadline on a token not yet marked dead must not print
        // "-3 minutes left", which reads like a bug rather than a lapsed token.
        let past = describe(&Standing::Healthy { expires_at: Some(NOW - 9999) }, NOW);
        assert!(!past.contains('-'), "no negative durations: {past}");
    }

    #[test]
    fn an_unknown_expiry_is_said_plainly_rather_than_shown_as_a_number() {
        let text = describe(&Standing::Healthy { expires_at: None }, NOW);
        assert!(text.contains("not yet known"), "{text}");
        assert!(!text.contains('0'), "no fabricated duration: {text}");
    }
}
