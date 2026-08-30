//! Renew a forge token through the dōjō.
//!
//! The daemon holds the refresh token and cannot spend it: redemption needs the
//! GitHub App's client secret, which lives in the dōjō and deliberately never
//! ships to a user's machine. So this sends the refresh token to
//! `POST /v1/you/forge/refresh` and stores what comes back.
//!
//! ## Why the outcome has three arms and not two
//!
//! The same split [`crate::api::handlers::auth::AuthError`] draws for the
//! session token, for the same reason. A REJECTED refresh is terminal — the
//! grant is gone and only the user can restore it — while an outage is not, and
//! treating the two alike either nags about a blip or stays silent about a grant
//! that will never work again. The dōjō returns `needsSignIn` as a field so this
//! never has to parse English to tell them apart.
//!
//! ## The rotation is the dangerous part
//!
//! GitHub issues a NEW refresh token on every successful redemption and
//! invalidates the old one. If the new token is not stored, the session is
//! unrecoverable without a sign-in — so the write happens before the caller is
//! told anything succeeded, and a failed write is reported as a failure.

use crate::dojo_client::session;
use crate::dojo_client::settings::dojo_url;

/// What a renewal attempt produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshOutcome {
    /// A live token, already stored. `expires_at` is unix seconds, `0` when the
    /// dōjō reported no deadline — distinguishable from a real one, and never
    /// invented.
    Renewed { expires_at: i64 },
    /// The grant is gone. Only a sign-in restores it; retrying cannot.
    Rejected(String),
    /// We could not complete the exchange. Says NOTHING about the token — the
    /// stored credential must be left exactly as it was.
    Unavailable(String),
}

/// The new credentials, before they are stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Renewed {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

/// Classify the dōjō's answer.
///
/// Split from the I/O so every branch is testable without a dōjō. `status` is
/// `None` when no response arrived at all.
pub fn classify_refresh(status: Option<u16>, body: &str) -> Result<Renewed, RefreshOutcome> {
    let Some(status) = status else {
        return Err(RefreshOutcome::Unavailable("no response from the dōjō".into()));
    };
    let parsed: Option<serde_json::Value> = serde_json::from_str(body).ok();

    if !(200..300).contains(&status) {
        // `needsSignIn` is a FIELD, not a status code: a 502 carries both the
        // terminal and the transient case, because both are GitHub refusing.
        let terminal = parsed.as_ref().and_then(|v| v["needsSignIn"].as_bool()).unwrap_or(false);
        let msg = parsed
            .as_ref()
            .and_then(|v| v["error"].as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("dōjō refused the refresh (HTTP {status})"));
        // Absent or unreadable `needsSignIn` is treated as TRANSIENT. Guessing
        // terminal signs the user out over a proxy error; guessing transient
        // costs one wasted retry on the next interval.
        return Err(if terminal {
            RefreshOutcome::Rejected(msg)
        } else {
            RefreshOutcome::Unavailable(msg)
        });
    }

    let Some(v) = parsed else {
        return Err(RefreshOutcome::Unavailable("dōjō returned an unreadable response".into()));
    };
    let access_token = v["access_token"].as_str().unwrap_or_default().to_string();
    let refresh_token = v["refresh_token"].as_str().unwrap_or_default().to_string();
    // A 2xx with no token is a shape we do not understand. Storing the empty
    // string would overwrite a working credential with nothing, and every later
    // call would 401 with no record of why.
    if access_token.is_empty() {
        return Err(RefreshOutcome::Unavailable("dōjō returned no access token".into()));
    }
    if refresh_token.is_empty() {
        return Err(RefreshOutcome::Unavailable("dōjō returned no refresh token".into()));
    }
    Ok(Renewed { access_token, refresh_token, expires_at: v["expires_at"].as_i64().unwrap_or(0) })
}

/// Renew one persona's forge token, storing the result.
///
/// Both writes are attempted and BOTH are load-bearing. The refresh token is the
/// one that cannot be recovered: GitHub already invalidated the old one when it
/// issued this, so failing to store it silently would leave a session that dies
/// at the next expiry with no way back except a sign-in.
pub async fn refresh_persona(persona: &str) -> RefreshOutcome {
    let who = persona.to_string();
    let stored =
        tokio::task::spawn_blocking(move || session::load_provider_refresh_token(&who)).await;
    let refresh_token = match stored {
        Ok(Ok(t)) => t,
        // No refresh token means renewal was never possible for this persona —
        // a session predating provider-refresh capture, or a forge that issued
        // none. Not a fault to retry.
        _ => {
            return RefreshOutcome::Rejected(
                "no forge refresh token is stored for this persona".into(),
            );
        }
    };

    // The dōjō authenticates the caller before it spends anything, so this needs
    // a live SESSION token — a different credential from the one being renewed.
    let bearer = match crate::api::handlers::auth::live_access_token(persona).await {
        Ok(t) => t,
        Err(e) => {
            return RefreshOutcome::Unavailable(format!("no dōjō session to refresh with: {e:?}"));
        }
    };

    let resp = crate::federation::http_client()
        .post(format!("{}/v1/you/forge/refresh", dojo_url()))
        .bearer_auth(bearer)
        .json(&serde_json::json!({ "refresh_token": refresh_token }))
        .send()
        .await;

    let (status, body) = match resp {
        Ok(r) => {
            let s = r.status().as_u16();
            (Some(s), r.text().await.unwrap_or_default())
        }
        Err(e) => {
            return RefreshOutcome::Unavailable(format!("could not reach the dōjō: {e}"));
        }
    };

    let renewed = match classify_refresh(status, &body) {
        Ok(r) => r,
        Err(outcome) => return outcome,
    };

    let (at, rt) = (renewed.access_token.clone(), renewed.refresh_token.clone());
    let who = persona.to_string();
    let written = tokio::task::spawn_blocking(move || {
        session::store_provider_token(&who, &at)?;
        session::store_provider_refresh_token(&who, &rt)
    })
    .await;
    // Reported as a failure, loudly. GitHub has already invalidated the old
    // refresh token, so a persona whose new one did not store now holds a
    // credential nothing can renew — the user must sign in, and answering
    // "renewed" would hide that until the token dies hours later.
    let detail = match written {
        Ok(Ok(())) => return RefreshOutcome::Renewed { expires_at: renewed.expires_at },
        Ok(Err(e)) => e.to_string(),
        Err(e) => format!("the keychain write did not complete: {e}"),
    };
    tracing::error!(persona, error = %detail,
                    "forge token renewed but could NOT be stored — the old token is already \
                     invalid, so this persona needs a sign-in");
    RefreshOutcome::Unavailable(format!("renewed but could not store: {detail}"))
}

#[cfg(test)]
mod refresh_classification {
    use super::*;

    const OK: &str = r#"{"access_token":"gho_new","refresh_token":"ghr_new",
                         "expires_at":1788148800,"scope":"read:org,repo"}"#;

    #[test]
    fn a_renewal_carries_both_tokens_and_the_absolute_deadline() {
        let r = classify_refresh(Some(200), OK).expect("a 200 with both tokens is a renewal");
        assert_eq!(r.access_token, "gho_new");
        assert_eq!(r.refresh_token, "ghr_new");
        assert_eq!(r.expires_at, 1_788_148_800);
    }

    #[test]
    fn a_refusal_that_says_it_needs_a_sign_in_is_terminal() {
        // The poison-pill guard. A revoked grant fails identically forever;
        // without this the scheduled check retries it every interval until the
        // user happens to sign in on their own.
        let body =
            r#"{"error":"GitHub refused the refresh: bad_refresh_token","needsSignIn":true}"#;
        assert_eq!(
            classify_refresh(Some(502), body),
            Err(RefreshOutcome::Rejected("GitHub refused the refresh: bad_refresh_token".into()))
        );
    }

    #[test]
    fn a_refusal_without_that_flag_is_treated_as_transient() {
        // Deliberately asymmetric. Guessing TERMINAL on a proxy error signs the
        // user out of a session that was fine; guessing transient costs one
        // wasted retry on the next interval. Only one of those is recoverable.
        let body = r#"{"error":"GitHub refused the refresh (HTTP 502)","needsSignIn":false}"#;
        assert!(matches!(classify_refresh(Some(502), body), Err(RefreshOutcome::Unavailable(_))));
        // And when the field is missing entirely.
        assert!(matches!(
            classify_refresh(Some(500), r#"{"error":"boom"}"#),
            Err(RefreshOutcome::Unavailable(_))
        ));
        // And when the body is not JSON at all — an HTML error page from a proxy.
        assert!(matches!(
            classify_refresh(Some(504), "<html>gateway timeout</html>"),
            Err(RefreshOutcome::Unavailable(_))
        ));
    }

    #[test]
    fn an_unconfigured_dojo_is_transient_not_a_revoked_grant() {
        // 503 from `forgeAppFromEnv() == null`. The deployment is missing its
        // client secret; telling the user to sign in again cannot fix that, and
        // they would destroy a working session trying.
        let body = r#"{"error":"this dōjō is not configured to refresh forge tokens"}"#;
        assert!(matches!(classify_refresh(Some(503), body), Err(RefreshOutcome::Unavailable(_))));
    }

    #[test]
    fn no_response_at_all_says_nothing_about_the_token() {
        assert!(matches!(classify_refresh(None, ""), Err(RefreshOutcome::Unavailable(_))));
    }

    #[test]
    fn a_success_missing_either_token_is_refused_rather_than_stored() {
        // Storing an empty string would overwrite a working credential with
        // nothing; every later call then 401s with no record of why. And a
        // missing REFRESH token is worse than it looks — GitHub has already
        // invalidated the old one by the time it answers, so accepting this
        // leaves a session that cannot be renewed again.
        for body in [
            r#"{"refresh_token":"ghr_new","expires_at":1}"#,
            r#"{"access_token":"gho_new","expires_at":1}"#,
            r#"{"access_token":"","refresh_token":"","expires_at":1}"#,
        ] {
            assert!(
                matches!(classify_refresh(Some(200), body), Err(RefreshOutcome::Unavailable(_))),
                "accepted a partial renewal: {body}"
            );
        }
    }

    #[test]
    fn a_renewal_with_no_stated_deadline_records_zero_rather_than_inventing_one() {
        // `0` is distinguishable and the caller stores it as unknown. Making up
        // "now + 8h" would schedule the next renewal against a fabricated time.
        let r = classify_refresh(Some(200), r#"{"access_token":"a","refresh_token":"b"}"#)
            .expect("a renewal without a deadline is still a renewal");
        assert_eq!(r.expires_at, 0);
    }
}
