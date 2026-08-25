//! Sign-in to dōjō from the daemon — the two legs of the PKCE flow.
//!
//! `POST /api/auth/signin` returns a URL for the user to open; the browser lands
//! back on `GET /api/auth/callback`, which exchanges the code and stores the
//! refresh token in the Keychain.
//!
//! ## Why the verifier is held in memory
//!
//! It must survive between the two legs but must NOT be persisted: it is the
//! secret half of the exchange, and writing it anywhere durable would leave a
//! replayable credential behind after a flow that may never complete. Process
//! memory is exactly the right lifetime — a daemon restart mid-sign-in fails the
//! flow, which is the correct outcome.

use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::Deserialize;
use std::sync::Mutex;

use crate::api::state::AppState;
use crate::dojo_client::{dojo_auth, pkce, session};

/// The in-flight verifier, between the two legs.
///
/// One at a time: a second sign-in started before the first completes replaces
/// it, which is the honest behaviour — the abandoned flow's code is then
/// unusable, and that is what we want rather than keeping stale verifiers alive.
/// The in-flight (persona, verifier) pair.
static PENDING_VERIFIER: Mutex<Option<(String, String)>> = Mutex::new(None);

/// Which persona a sign-in is for. Defaults to the label the user is most likely
/// to mean on a single-identity install.
#[derive(Deserialize)]
pub(crate) struct PersonaQuery {
    #[serde(default = "default_persona")]
    persona: String,
    /// Which GitHub account to suggest. Without it the browser's existing
    /// session is reused, so connecting a SECOND persona quietly links the
    /// first account again — a success as the wrong person.
    #[serde(default)]
    github_login: Option<String>,
}
fn default_persona() -> String {
    "default".to_string()
}

use crate::dojo_client::settings::dojo_url;

fn callback_url() -> String {
    format!(
        "http://127.0.0.1:{}/api/auth/callback",
        sensei_bootstrap::SenseiConfig::from_env().daemon_port
    )
}

/// `POST /api/auth/signin` — begin sign-in.
///
/// Returns the URL to open rather than opening it here: the daemon may be
/// headless, and a caller (the desktop app, the CLI) knows better than it does
/// how to present a browser.
pub(crate) async fn signin(Query(p): Query<PersonaQuery>) -> Json<serde_json::Value> {
    let verifier = pkce::generate_verifier();
    // Check our own output before sending the user to a browser. A malformed
    // verifier is rejected at the TOKEN leg — after the user has signed in — with
    // an error that says nothing about length or alphabet, so catching it here
    // turns a baffling late failure into an obvious early one.
    if !pkce::is_valid_verifier(&verifier) {
        return Json(serde_json::json!({
            "ok": false,
            "error": "generated an RFC-invalid PKCE verifier — refusing to start sign-in",
        }));
    }
    let challenge = pkce::challenge_for(&verifier);
    let port = sensei_bootstrap::SenseiConfig::from_env().daemon_port;

    // dōjō builds the URL. That is the point of routing through it: the daemon
    // holds one setting — where dōjō is — and never learns which identity
    // provider sits behind it or what key talks to it.
    let url = match dojo_auth::start(&dojo_url(), &challenge, port, p.github_login.as_deref()).await
    {
        Ok(url) => url,
        Err(e) => return Json(serde_json::json!({ "ok": false, "error": e })),
    };

    // Held only once dōjō has accepted the challenge. Storing it before would
    // leave a stale pending flow behind a failed start, and the NEXT callback —
    // possibly from a different attempt — would try to use it.
    *PENDING_VERIFIER.lock().unwrap_or_else(|e| e.into_inner()) =
        Some((p.persona.clone(), verifier));
    Json(serde_json::json!({
        "authorizeUrl": url,
        "callback": callback_url(),
        "persona": p.persona,
        "dojo": dojo_url(),
        // The hint is not a guarantee — GitHub suggests the account rather than
        // forcing a re-auth — so the caller should show which identity is
        // expected and offer a private window as the certain path.
        "expectedLogin": p.github_login,
        "hint": "if the browser is already signed in to another GitHub account, use a private window",
    }))
}

#[derive(Deserialize)]
pub(crate) struct CallbackQuery {
    code: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// `GET /api/auth/callback` — finish sign-in.
///
/// Takes the verifier rather than cloning it: a code may be exchanged once, so
/// leaving the verifier in place would let a replayed callback attempt a second
/// exchange.
pub(crate) async fn callback(
    State(state): State<AppState>,
    Query(q): Query<CallbackQuery>,
) -> Json<serde_json::Value> {
    // The provider can redirect back with a refusal — a declined consent screen
    // is not an error to swallow, it is the answer.
    if let Some(err) = q.error {
        return Json(serde_json::json!({
            "ok": false,
            "error": err,
            "detail": q.error_description,
        }));
    }

    let Some(code) = q.code else {
        return Json(serde_json::json!({ "ok": false, "error": "no code in callback" }));
    };
    let Some((persona, verifier)) = PENDING_VERIFIER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
    else {
        return Json(serde_json::json!({
            "ok": false,
            "error": "no sign-in in progress — start with POST /api/auth/signin",
        }));
    };

    match dojo_auth::exchange(&dojo_url(), &code, &verifier).await {
        Ok(tokens) => {
            // Blocking keychain write off the async runtime, as gateway_keys
            // documents.
            let refresh = tokens.refresh_token.clone();
            let who = persona.clone();
            // Capture GitHub's token NOW or lose it: GoTrue returns it only
            // at the exchange and prunes its flow_state row afterwards, so
            // there is no later query that recovers it. This is the token
            // read:org exists for — provisioning cannot list organisations
            // without it, and would report "none" rather than "never asked".
            let provider = tokens.provider_token.clone();
            let provider_refresh = tokens.provider_refresh_token.clone();
            let stored = tokio::task::spawn_blocking(move || {
                if let Some(pt) = provider.as_deref() {
                    // Non-fatal: a failed provider-token write costs
                    // provisioning, not the sign-in itself.
                    if let Err(e) = session::store_provider_token(&who, pt) {
                        tracing::warn!(error = %e, "could not store the GitHub token");
                    }
                }
                if let Some(pr) = provider_refresh.as_deref() {
                    let _ = session::store_provider_refresh_token(&who, pr);
                }
                session::store_refresh_token(&who, &refresh)
            })
            .await;
            // Derive the identity from AUTH rather than leaving it a guess.
            // A persona discovered from git carries an inferred label — and
            // inference is wrong: `sensei-hq` came from an email domain when
            // the real login is `sensei-hq-org`. OAuth knows the answer, so
            // record it, along with the account's verified emails as claimed
            // aliases.
            let linked = match tokens.user.as_ref() {
                Some(u) => {
                    link_verified_identity(&state, &persona, u, tokens.provider_token.as_deref())
                        .await
                }
                None => serde_json::json!({
                        "verified": false, "reason": "the exchange returned no user" }),
            };

            match stored {
                Ok(Ok(())) => Json(serde_json::json!({
                    "ok": true,
                    "signedIn": true,
                    "persona": persona,
                    "identity": linked,
                    // Whether org provisioning will be possible for this
                    // persona — surfaced so a missing token is visible now
                    // rather than as an empty org list later.
                    "canReadOrgs": tokens.provider_token.is_some(),
                })),
                Ok(Err(e)) => Json(serde_json::json!({
                    "ok": false,
                    "error": format!("signed in, but the refresh token could not be stored: {e}"),
                })),
                Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
            }
        }
        // Surface dōjō's own message rather than a generic failure. "invalid
        // grant" with no context is the most confusing outcome in this flow, and
        // the body usually says which half is wrong.
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

/// `POST /api/auth/signout` — forget the stored session.
///
/// Clearing a rejected token matters: a permanently-invalid one otherwise makes
/// every refresh fail identically and the daemon retries forever instead of
/// surfacing "sign in again".
pub(crate) async fn signout(Query(p): Query<PersonaQuery>) -> Json<serde_json::Value> {
    let who = p.persona.clone();
    match tokio::task::spawn_blocking(move || session::clear_refresh_token(&who)).await {
        Ok(Ok(())) => {
            Json(serde_json::json!({ "ok": true, "signedIn": false, "persona": p.persona }))
        }
        Ok(Err(e)) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}

/// `GET /api/auth/status` — is there a USABLE session?
///
/// Reports whether the stored token still works, not merely that one exists. A
/// revoked or expired refresh token sits in the Keychain looking healthy, so
/// "signedIn: true" based on presence alone would be a lie the caller only
/// discovers when a sync fails.
///
/// Never reports the token itself.
pub(crate) async fn status(
    State(state): State<AppState>,
    Query(p): Query<PersonaQuery>,
) -> Json<serde_json::Value> {
    let who = p.persona.clone();
    let stored = tokio::task::spawn_blocking(move || session::load_refresh_token(&who)).await;
    let Ok(Ok(refresh)) = stored else {
        return Json(
            serde_json::json!({ "signedIn": false, "persona": p.persona, "dojo": dojo_url() }),
        );
    };

    let tokens = match dojo_auth::refresh(&dojo_url(), &refresh).await {
        Ok(t) => t,
        Err(e) => {
            // A REJECTED refresh token is terminal, so clear it: otherwise the
            // daemon retries a credential the server will never accept and the
            // user is never told to sign in again. A transport failure is NOT
            // terminal and must not clear anything — dōjō being briefly
            // unreachable would otherwise sign the user out for nothing.
            let rejected = dojo_auth::status_of(&e).is_some_and(dojo_auth::is_rejection);
            if rejected {
                let who3 = p.persona.clone();
                let _ =
                    tokio::task::spawn_blocking(move || session::clear_refresh_token(&who3)).await;
            }
            return Json(serde_json::json!({
                "signedIn": false,
                "persona": p.persona,
                "error": if rejected {
                    "stored session was rejected — sign in again"
                } else {
                    "could not reach dōjō — the stored session was left alone"
                },
                "detail": e,
                "dojo": dojo_url(),
            }));
        }
    };

    let now = chrono::Utc::now().timestamp();
    let sess = session::Session::from_response(&tokens, now);
    // dōjō rotates the refresh token on use; storing the new one is not optional
    // — keeping the old would invalidate the session on the NEXT call, which
    // looks like a random sign-out.
    let rotated = tokens.refresh_token.clone();
    let who2 = p.persona.clone();
    let _ =
        tokio::task::spawn_blocking(move || session::store_refresh_token(&who2, &rotated)).await;

    // Prove the ACCESS token authenticates, not merely that the refresh did.
    // "signedIn" on the strength of a refresh alone is the lie this endpoint
    // exists to prevent — the caller would otherwise discover it at the next sync.
    let usable = dojo_auth::whoami(&dojo_url(), &tokens.access_token).await;
    let auth_user_id = usable
        .as_ref()
        .ok()
        .and_then(|v| v["userId"].as_str().map(String::from));
    let email = usable
        .as_ref()
        .ok()
        .and_then(|v| v["email"].as_str().map(String::from));

    // Backfill the verified identity for a persona connected before this
    // existed, so an established session need not be re-signed just to learn a
    // login the provider already told us. The GitHub token was captured at the
    // exchange and kept, because it is returned exactly once.
    let linked = match tokens.user.as_ref() {
        Some(u) => {
            let who4 = p.persona.clone();
            let pt = tokio::task::spawn_blocking(move || session::load_provider_token(&who4))
                .await
                .ok()
                .and_then(|r| r.ok());
            Some(link_verified_identity(&state, &p.persona, u, pt.as_deref()).await)
        }
        None => None,
    };

    Json(serde_json::json!({
        "signedIn": auth_user_id.is_some(),
        "persona": p.persona,
        "authUserId": auth_user_id,
        "email": email,
        "identity": linked,
        "expiresAt": sess.expires_at,
        "needsRefresh": sess.needs_refresh(now),
        "dojo": dojo_url(),
        // Present only when the token could NOT be used — an unusable session is
        // reported as such rather than as a bare signedIn:false.
        "error": usable.as_ref().err(),
    }))
}

/// `GET /api/auth/orgs?persona=…` — the GitHub organisations this persona can see.
///
/// The first step of provisioning: an org becomes a tenant. Reads GitHub
/// directly with the stored provider token, because Supabase records the user's
/// PROFILE but never calls `/user/orgs` — the `read:org` scope grants the right
/// to ask, it does not fetch anything.
///
/// Fails loudly when the token is missing rather than returning an empty list. A
/// bare `[]` is indistinguishable from "this user belongs to no organisations",
/// which is the wrong conclusion to hand a provisioning step that would then
/// create nothing and report success.
pub(crate) async fn orgs(Query(p): Query<PersonaQuery>) -> Json<serde_json::Value> {
    let who = p.persona.clone();
    let token = match tokio::task::spawn_blocking(move || session::load_provider_token(&who)).await
    {
        Ok(Ok(t)) => t,
        _ => {
            return Json(serde_json::json!({
                "ok": false,
                "persona": p.persona,
                "error": "no GitHub token stored for this persona — sign in again to capture one",
                "detail": "the provider token is returned only at the exchange; a session created before it was captured has none",
            }));
        }
    };

    let resp = crate::federation::http_client()
        .get("https://api.github.com/user/orgs")
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        // GitHub rejects requests without one.
        .header("User-Agent", "sensei")
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let body: serde_json::Value = r.json().await.unwrap_or_default();
            let orgs: Vec<serde_json::Value> = body
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|o| {
                            serde_json::json!({
                                // `login` is the durable natural key an org is
                                // known by — the tenant key is built from it, so
                                // both sides converge without sharing a uuid.
                                "login": o["login"],
                                "githubId": o["id"],
                                "tenantKey": format!("github/{}", o["login"].as_str().unwrap_or_default()),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            Json(serde_json::json!({ "ok": true, "persona": p.persona, "orgs": orgs }))
        }
        Ok(r) => {
            let status = r.status().as_u16();
            let body = r.text().await.unwrap_or_default();
            // 403 here usually means the org enforces SSO or the token lacks
            // read:org — worth saying, because the symptom is an empty list.
            Json(serde_json::json!({
                "ok": false, "persona": p.persona,
                "error": "GitHub rejected the org request", "status": status, "detail": body,
            }))
        }
        Err(e) => {
            Json(serde_json::json!({ "ok": false, "persona": p.persona, "error": e.to_string() }))
        }
    }
}

/// Record the verified GitHub identity for a persona, and adopt the account's
/// emails as claimed aliases.
///
/// Takes the user object the caller already holds rather than re-fetching it:
/// both sign-in and status read it as part of their own work, and a second call
/// could in principle answer for a different session.
///
/// Best-effort by design: the session is already usable by the time this runs, so
/// a failure here degrades the persona to "connected but unverified" rather than
/// discarding a working session. The outcome is returned so the caller can say
/// which it is instead of showing a confident label that was never proven.
async fn link_verified_identity(
    state: &AppState,
    persona: &str,
    user: &serde_json::Value,
    provider_token: Option<&str>,
) -> serde_json::Value {
    let Some((login, gh_id)) = github_identity(user) else {
        return serde_json::json!({ "verified": false, "reason": "no github identity on this account" });
    };

    // The account's identifying address — what resolution matches an existing
    // persona on.
    let primary = user["email"].as_str();

    // All verified addresses, not just the primary: the point of aliases is that
    // one human commits under several, and only GitHub knows the full set.
    let mut emails: Vec<String> = primary.map(|e| vec![e.to_string()]).unwrap_or_default();
    if let Some(pt) = provider_token {
        emails.extend(github_verified_emails(pt).await);
    }
    emails.sort();
    emails.dedup();

    match state
        .pg
        .link_persona_identity(persona, login, gh_id, primary, &emails)
        .await
    {
        Ok(id) => serde_json::json!({
            "verified": true,
            "personaId": id,
            "githubLogin": login,
            "githubUserId": gh_id,
            "claimedEmails": emails,
        }),
        Err(e) => serde_json::json!({ "verified": false, "reason": e }),
    }
}

/// The GitHub login and numeric id from a GoTrue user object.
///
/// An account can hold several linked identities (Supabase links them
/// automatically when the email matches), so the github one is SELECTED rather
/// than assumed to be first — taking `identities[0]` would read a Google or
/// email identity's fields and find neither.
fn github_identity(user: &serde_json::Value) -> Option<(&str, i64)> {
    let i = user["identities"]
        .as_array()?
        .iter()
        .find(|i| i["provider"].as_str() == Some("github"))?;
    let login = i["identity_data"]["user_name"].as_str()?;
    // GitHub's id arrives as a JSON string from some GoTrue versions and a
    // number from others; both name the same account, and accepting only one
    // form would silently leave the persona unverified against the other.
    let v = &i["identity_data"]["provider_id"];
    let id = v
        .as_i64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))?;
    Some((login, id))
}

/// The account's VERIFIED email addresses from GitHub.
///
/// Unverified addresses are skipped: anyone can add any address to a GitHub
/// account and leave it unconfirmed, so an unverified one asserts nothing and
/// would let a stranger's commits be attributed to this persona.
///
/// An empty result on failure is correct here and not a masked error — the
/// caller treats these as ADDITIONS to the primary address, so "none found"
/// simply links fewer aliases rather than fabricating any.
async fn github_verified_emails(provider_token: &str) -> Vec<String> {
    let Ok(r) = crate::federation::http_client()
        .get("https://api.github.com/user/emails")
        .header("Authorization", format!("Bearer {provider_token}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "sensei")
        .send()
        .await
    else {
        return Vec::new();
    };
    if !r.status().is_success() {
        return Vec::new();
    }
    let list: serde_json::Value = r.json().await.unwrap_or_default();
    list.as_array()
        .map(|a| {
            a.iter()
                .filter(|e| e["verified"].as_bool() == Some(true))
                .filter_map(|e| e["email"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape GoTrue actually returned for the two live sign-ins.
    fn user_with(identities: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ "id": "abc", "email": "dev@sensei-hq.com", "identities": identities })
    }

    #[test]
    fn the_github_login_and_id_are_read_from_the_identity() {
        let u = user_with(serde_json::json!([{
            "provider": "github",
            "identity_data": { "user_name": "sensei-hq-org", "provider_id": "293381742" }
        }]));
        assert_eq!(github_identity(&u), Some(("sensei-hq-org", 293381742)));
    }

    #[test]
    fn a_numeric_provider_id_is_accepted_too() {
        // GoTrue versions differ on whether this is a string or a number, and
        // accepting only one form leaves the persona silently unverified.
        let u = user_with(serde_json::json!([{
            "provider": "github",
            "identity_data": { "user_name": "jerrythomas", "provider_id": 1749920 }
        }]));
        assert_eq!(github_identity(&u), Some(("jerrythomas", 1749920)));
    }

    #[test]
    fn the_github_identity_is_selected_not_assumed_first() {
        // Supabase links identities automatically when the email matches, so
        // `identities[0]` may well be a different provider — reading it would
        // find no user_name and report "not verified" for a verified account.
        let u = user_with(serde_json::json!([
            { "provider": "email", "identity_data": { "email": "dev@sensei-hq.com" } },
            { "provider": "github",
              "identity_data": { "user_name": "sensei-hq-org", "provider_id": "293381742" } }
        ]));
        assert_eq!(github_identity(&u), Some(("sensei-hq-org", 293381742)));
    }

    #[test]
    fn an_account_without_a_github_identity_yields_nothing() {
        // Not an error: it means we cannot verify a GitHub login, and the
        // persona stays unverified rather than acquiring an invented one.
        let u = user_with(serde_json::json!([{ "provider": "email", "identity_data": {} }]));
        assert_eq!(github_identity(&u), None);
        assert_eq!(github_identity(&serde_json::json!({ "id": "abc" })), None);
    }
}
