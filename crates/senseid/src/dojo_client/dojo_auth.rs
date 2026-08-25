//! The daemon's side of dōjō's `/v1/auth/cli/*` sign-in.
//!
//! Three legs, all against dōjō and nothing else:
//!
//! * `start` — hand dōjō a PKCE challenge and our loopback port, get back the URL
//!   to open in a browser.
//! * `exchange` — trade the returned code, plus the verifier we never sent
//!   anywhere, for a session.
//! * `refresh` — trade a stored refresh token for a live session.
//!
//! The verifier stays in this process throughout. dōjō sees only its hash, so a
//! captured redirect is not replayable by anyone — dōjō included.

use super::session::TokenResponse;

/// Path builder, kept in one place so a rename can't half-apply.
fn endpoint(dojo_url: &str, leg: &str) -> String {
    format!("{dojo_url}/v1/auth/cli/{leg}")
}

/// Ask dōjō where to send the user's browser.
///
/// `port` is this daemon's loopback port: dōjō sends the provider back to itself
/// and then forwards here, which is what keeps the provider's redirect allow-list
/// to a single dōjō entry instead of one per machine.
pub async fn start(
    dojo_url: &str,
    challenge: &str,
    port: u16,
    login_hint: Option<&str>,
) -> Result<String, String> {
    let mut req = crate::federation::http_client()
        .get(endpoint(dojo_url, "start"))
        .query(&[("challenge", challenge), ("port", &port.to_string())]);
    // Which account to suggest. Without it the browser reuses whichever GitHub
    // session it already holds, so connecting a SECOND identity quietly links
    // the first one again — a success as the wrong person.
    if let Some(login) = login_hint.filter(|l| !l.is_empty()) {
        req = req.query(&[("login", login)]);
    }

    let r = req.send().await.map_err(|e| format!("could not reach dōjō: {e}"))?;
    let status = r.status();
    let body = r.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(dojo_error(status.as_u16(), &body));
    }
    serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v["authorizeUrl"].as_str().map(String::from))
        .ok_or_else(|| "dōjō returned no authorizeUrl".to_string())
}

/// Trade the code and verifier for a session.
pub async fn exchange(dojo_url: &str, code: &str, verifier: &str) -> Result<TokenResponse, String> {
    post(dojo_url, "token", serde_json::json!({ "code": code, "verifier": verifier })).await
}

/// Prove an access token is usable, and learn who it authenticates as.
///
/// A successful refresh only shows the REFRESH token is live — it says nothing
/// about whether the access token it produced is accepted. Reporting "signed in"
/// without this check is the lie the status endpoint exists to prevent: the
/// caller would find out at the next sync instead.
pub async fn whoami(dojo_url: &str, access_token: &str) -> Result<serde_json::Value, String> {
    let r = crate::federation::http_client()
        .get(endpoint(dojo_url, "whoami"))
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|e| format!("could not reach dōjō: {e}"))?;
    let status = r.status();
    let text = r.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(dojo_error(status.as_u16(), &text));
    }
    serde_json::from_str(&text).map_err(|e| format!("dōjō returned an unreadable identity: {e}"))
}

/// Trade a stored refresh token for a live session.
pub async fn refresh(dojo_url: &str, refresh_token: &str) -> Result<TokenResponse, String> {
    post(dojo_url, "refresh", serde_json::json!({ "refresh_token": refresh_token })).await
}

/// POST a body to a leg and parse the session out of the response.
async fn post(dojo_url: &str, leg: &str, body: serde_json::Value) -> Result<TokenResponse, String> {
    let r = crate::federation::http_client()
        .post(endpoint(dojo_url, leg))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("could not reach dōjō: {e}"))?;
    let status = r.status();
    let text = r.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(dojo_error(status.as_u16(), &text));
    }
    serde_json::from_str::<TokenResponse>(&text)
        .map_err(|e| format!("dōjō returned an unreadable session: {e}"))
}

/// The HTTP status inside a failure message from this module, if it carries one.
///
/// The alternative — matching on message text at the call site — silently stops
/// working the day the wording changes, and the thing it decides is whether to
/// DELETE the user's stored session.
pub fn status_of(error: &str) -> Option<u16> {
    error
        .strip_prefix("dōjō returned ")
        .and_then(|rest| rest.split([':', ' ']).next())
        .and_then(|code| code.parse().ok())
}

/// Whether a status means the CREDENTIAL is dead, as opposed to the request
/// being wrong or the server being unwell.
///
/// Deliberately narrow: this decides whether the user's stored session is
/// DELETED. Only 401 and 403 are the server's verdict on the credential itself.
///
/// 400 in particular is NOT a rejection — it says our request was malformed,
/// which is a bug on our side and says nothing about the token. Treating it as
/// one destroyed a live session during development: a body that failed to reach
/// dōjō came back "400 refresh_token is required", and the daemon helpfully
/// deleted a perfectly good refresh token. 404 and 405 are the same story, a
/// wrong path or method; 429 is "slow down", not "start over".
pub fn is_rejection(status: u16) -> bool {
    matches!(status, 401 | 403)
}

/// Render a failure body without inventing detail.
fn dojo_error(status: u16, body: &str) -> String {
    let detail = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v["error"].as_str().map(String::from))
        .unwrap_or_else(|| body.chars().take(200).collect());
    if detail.trim().is_empty() {
        format!("dōjō returned {status}")
    } else {
        format!("dōjō returned {status}: {detail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_legs_hang_off_the_dojo_url() {
        // One setting drives all three, so a wrong base fails everywhere at once
        // rather than leaving a half-working sign-in.
        let d = "https://dojo.sensei-hq.com";
        assert_eq!(endpoint(d, "start"), "https://dojo.sensei-hq.com/v1/auth/cli/start");
        assert_eq!(endpoint(d, "token"), "https://dojo.sensei-hq.com/v1/auth/cli/token");
        assert_eq!(endpoint(d, "refresh"), "https://dojo.sensei-hq.com/v1/auth/cli/refresh");
    }

    #[test]
    fn only_an_unauthorized_status_counts_as_a_rejected_credential() {
        // This decides whether the user's stored session is DELETED, so it is
        // deliberately narrow.
        assert!(is_rejection(401));
        assert!(is_rejection(403));
    }

    #[test]
    fn a_bad_request_does_not_destroy_a_working_session() {
        // Learned the hard way: a request body that failed to reach dōjō came
        // back "400 refresh_token is required", and a 4xx-wide rule deleted a
        // live refresh token. 400/404/405 mean WE got it wrong; 429 means slow
        // down; 5xx means dōjō is unwell. None of them impugn the credential.
        for status in [400, 404, 405, 429, 500, 502, 503, 200] {
            assert!(!is_rejection(status), "{status} must not clear the session");
        }
    }

    #[test]
    fn the_status_is_recoverable_from_a_failure_message() {
        // This is what decides whether the stored session is DELETED, so it must
        // not rest on matching prose that a later edit can reword.
        assert_eq!(status_of(&dojo_error(401, r#"{"error":"bad token"}"#)), Some(401));
        assert_eq!(status_of(&dojo_error(502, "")), Some(502));
        assert_eq!(status_of("could not reach dōjō: connection refused"), None);
        assert_eq!(status_of("dōjō returned no authorizeUrl"), None);
    }

    #[test]
    fn a_json_error_is_surfaced_rather_than_the_raw_body() {
        assert_eq!(
            dojo_error(400, r#"{"error":"code and verifier are required"}"#),
            "dōjō returned 400: code and verifier are required"
        );
    }

    #[test]
    fn a_non_json_body_still_produces_something_actionable() {
        // A proxy or a CDN can answer with HTML. Reporting the bare status is
        // less useful than reporting what actually came back.
        assert_eq!(
            dojo_error(502, "<html>bad gateway</html>"),
            "dōjō returned 502: <html>bad gateway</html>"
        );
        assert_eq!(dojo_error(500, "   "), "dōjō returned 500");
    }
}
