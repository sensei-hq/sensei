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
    let Some((persona, verifier)) =
        PENDING_VERIFIER.lock().unwrap_or_else(|e| e.into_inner()).take()
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
                    // Warned, not discarded. This was `let _ =`, unlike the
                    // provider-token write three lines up, so a failed write of
                    // the one credential that could renew a dying forge token
                    // was completely invisible.
                    if let Err(e) = session::store_provider_refresh_token(&who, pr) {
                        tracing::warn!(error = %e, "could not store the GitHub refresh token");
                    }
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
/// Sign-out has TWO halves, and having only the first is what made it a lie:
///
/// 1. **The Keychain.** [`session::clear_session`] removes every slot the
///    sign-in wrote. This used to clear one of three, leaving
///    `provider_token.<slot>` — a GitHub credential with `repo` and `read:org`,
///    verified live still able to read a private repository — at rest with no
///    code path anywhere that could remove it.
/// 2. **The registry.** `personas.session_slot` is what
///    [`crate::db::pg_store::PgStore::signed_in_personas`] enumerates. Leaving it
///    set meant the sync cycle kept selecting a persona with no credential:
///    every 60s it resolved `SignedOut`, and on a single-persona install that
///    made `tick` fail and pinned `schedules.dojo_sync.last_ok = false` forever.
///
/// Both are attempted even if the first fails, and both outcomes are reported.
/// A partial sign-out that answered `ok: true` would be the same class of lie.
pub(crate) async fn signout(
    State(state): State<AppState>,
    Query(p): Query<PersonaQuery>,
) -> Json<serde_json::Value> {
    let who = p.persona.clone();
    let keychain = match tokio::task::spawn_blocking(move || session::clear_session(&who)).await {
        Ok(r) => r.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    // Not short-circuited on a Keychain failure: a credential we could not delete
    // is all the more reason to stop the cycle from presenting it every cadence.
    let registry = state.pg.clear_persona_session(&p.persona).await;

    let errors: Vec<String> = [keychain.as_ref().err().cloned(), registry.as_ref().err().cloned()]
        .into_iter()
        .flatten()
        .collect();
    if !errors.is_empty() {
        tracing::error!(persona = %p.persona, errors = ?errors,
                        "sign-out did not complete — credentials may remain at rest");
    }

    Json(serde_json::json!({
        "ok": errors.is_empty(),
        // Only claimable when the Keychain actually gave the credentials up.
        "signedIn": keychain.is_err(),
        "persona": p.persona,
        // Which halves happened, rather than one boolean covering both. `false`
        // here means the registry held no such slot — not that it failed.
        "credentialsCleared": keychain.is_ok(),
        // `false` here is honest-empty ONLY because a failure is reported beside
        // it: `ok` is false and `errors` names it. Read alone it would be
        // indistinguishable from "no row held that slot".
        "registryReleased": registry.unwrap_or(false),
        "errors": errors,
    }))
}

/// `GET /api/auth/status` — is there a USABLE session?
///
/// Why a persona has no usable access token right now.
///
/// Three variants rather than one error string because they call for three
/// different responses: re-authenticate, wait, or nothing at all. An unattended
/// sync that cannot tell them apart either nags about a network blip or stays
/// quiet about a session that will never work again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AuthError {
    /// No stored session — never signed in, or signed out.
    SignedOut,
    /// The dōjō REJECTED the refresh token (401/403). Terminal, and the stored
    /// session has already been cleared.
    Rejected(String),
    /// The dōjō could not be reached, or failed. Transient — the stored session
    /// was deliberately left alone.
    Unreachable(String),
}

impl AuthError {
    /// Whether the user has to sign in again, as opposed to just waiting.
    pub(crate) fn needs_sign_in(&self) -> bool {
        matches!(self, Self::SignedOut | Self::Rejected(_))
    }
}

/// Classify a failed refresh: does it destroy the stored session or not?
///
/// Split out from the I/O so the decision can be tested without a dōjō. Only a
/// 401/403 is terminal; everything else — a 5xx, a DNS failure, a timeout —
/// leaves the credential in place, because clearing on those signs the user out
/// for an outage they did not cause.
fn refresh_failure(e: &str) -> AuthError {
    match dojo_auth::status_of(e).is_some_and(dojo_auth::is_rejection) {
        true => AuthError::Rejected(e.to_string()),
        false => AuthError::Unreachable(e.to_string()),
    }
}

/// A persona's session, refreshed and re-stored.
pub(crate) struct LiveSession {
    pub tokens: session::TokenResponse,
    pub session: session::Session,
}

/// Refresh one persona's stored session and hand back a usable one.
///
/// The single implementation of "get me a working credential for this persona".
/// [`status`] renders it for a human and the dōjō sync cycle uses it unattended;
/// before this existed only `status` had it, so the sync path would have grown a
/// second copy that drifted — starting with the rotation below, which is easy to
/// leave out and fails a whole session later.
///
/// Three obligations, all of which have bitten:
/// - **Rotate.** The dōjō issues a new refresh token on every use. Keeping the
///   old one invalidates the session on the NEXT call, which reads as a random
///   sign-out.
/// - **Clear only on rejection.** See [`refresh_failure`].
/// - **Never fabricate.** A persona that cannot be refreshed returns `Err`; it
///   never yields an empty or stale token that a caller would send as a bearer.
pub(crate) async fn live_session(persona: &str) -> Result<LiveSession, AuthError> {
    let who = persona.to_string();
    let stored = tokio::task::spawn_blocking(move || session::load_refresh_token(&who)).await;
    let Ok(Ok(refresh)) = stored else {
        return Err(AuthError::SignedOut);
    };

    let tokens = match dojo_auth::refresh(&dojo_url(), &refresh).await {
        Ok(t) => t,
        Err(e) => {
            let verdict = refresh_failure(&e);
            if matches!(verdict, AuthError::Rejected(_)) {
                let who = persona.to_string();
                let cleared =
                    tokio::task::spawn_blocking(move || session::clear_refresh_token(&who)).await;
                // `Rejected`'s own contract says "the stored session has already
                // been cleared". If the clear FAILED that is untrue: the dead token
                // stays in the Keychain, every cadence hits the same 401 forever, and
                // `signout`'s stated purpose is silently defeated. Report what
                // actually happened rather than asserting a cleanup that did not.
                let clear_err = match cleared {
                    Err(join) => Some(join.to_string()),
                    Ok(Err(e)) => Some(e.to_string()),
                    Ok(Ok(())) => None,
                };
                if let Some(why) = clear_err {
                    tracing::error!(persona, error = %why,
                                    "a REJECTED session could not be cleared — it will keep failing");
                    return Err(AuthError::Unreachable(format!(
                        "session was rejected but could not be cleared: {why}"
                    )));
                }
            }
            return Err(verdict);
        }
    };

    let now = chrono::Utc::now().timestamp();
    let sess = session::Session::from_response(&tokens, now);
    let rotated = tokens.refresh_token.clone();
    let who = persona.to_string();
    let stored =
        tokio::task::spawn_blocking(move || session::store_refresh_token(&who, &rotated)).await;
    // NOT discarded. The dōjō has ALREADY rotated, so the token still sitting in the
    // Keychain is dead: if this write failed, the next cadence gets a 401, clears the
    // session, and the user is signed out — with nothing anywhere explaining that a
    // healthy session was destroyed by a silent write failure. A session whose
    // rotation was not persisted must not be handed out as live.
    match stored {
        Err(e) => {
            tracing::error!(persona, error = %e, "rotated refresh token could not be stored — this session is now dead");
            return Err(AuthError::Unreachable(format!("could not store the rotated token: {e}")));
        }
        Ok(Err(e)) => {
            tracing::error!(persona, error = %e, "rotated refresh token could not be stored — this session is now dead");
            return Err(AuthError::Unreachable(format!("could not store the rotated token: {e}")));
        }
        Ok(Ok(())) => {}
    }

    Ok(LiveSession { tokens, session: sess })
}

/// A bearer token for `persona`, for callers that need only that.
///
/// Thin wrapper over [`live_session`], so the refresh, the rotation and the
/// clear-on-rejection rule cannot drift from what [`status`] reports.
pub(crate) async fn live_access_token(persona: &str) -> Result<String, AuthError> {
    live_session(persona).await.map(|s| s.tokens.access_token)
}

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
    // The refresh, the rotation and the clear-on-rejection rule all live in
    // `live_session` now, so the sync cycle runs exactly what this reports.
    let (tokens, sess) = match live_session(&p.persona).await {
        Ok(live) => (live.tokens, live.session),
        Err(AuthError::SignedOut) => {
            return Json(
                serde_json::json!({ "signedIn": false, "persona": p.persona, "dojo": dojo_url() }),
            );
        }
        Err(e @ (AuthError::Rejected(_) | AuthError::Unreachable(_))) => {
            let detail = match &e {
                AuthError::Rejected(d) | AuthError::Unreachable(d) => d.clone(),
                AuthError::SignedOut => unreachable!("handled above"),
            };
            return Json(serde_json::json!({
                "signedIn": false,
                "persona": p.persona,
                "error": if e.needs_sign_in() {
                    "stored session was rejected — sign in again"
                } else {
                    "could not reach dōjō — the stored session was left alone"
                },
                "detail": detail,
                "dojo": dojo_url(),
            }));
        }
    };
    let now = chrono::Utc::now().timestamp();

    // Prove the ACCESS token authenticates, not merely that the refresh did.
    // "signedIn" on the strength of a refresh alone is the lie this endpoint
    // exists to prevent — the caller would otherwise discover it at the next sync.
    let usable = dojo_auth::whoami(&dojo_url(), &tokens.access_token).await;
    let auth_user_id = usable.as_ref().ok().and_then(|v| v["userId"].as_str().map(String::from));
    let email = usable.as_ref().ok().and_then(|v| v["email"].as_str().map(String::from));

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

    match state.pg.link_persona_identity(persona, login, gh_id, primary, &emails).await {
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
    let i =
        user["identities"].as_array()?.iter().find(|i| i["provider"].as_str() == Some("github"))?;
    let login = i["identity_data"]["user_name"].as_str()?;
    // GitHub's id arrives as a JSON string from some GoTrue versions and a
    // number from others; both name the same account, and accepting only one
    // form would silently leave the persona unverified against the other.
    let v = &i["identity_data"]["provider_id"];
    let id = v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))?;
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

    #[test]
    fn only_a_401_or_403_is_terminal_enough_to_destroy_the_stored_session() {
        // This classification decides whether a refresh token is DELETED. Get it
        // backwards and a dōjō that is briefly unreachable signs the user out of
        // every persona — recoverable only by a fresh browser sign-in.
        //
        // A 5xx is the case that matters: it comes back from the same code path
        // as a 401, looks equally like "the server said no", and is precisely the
        // failure a retry fixes.
        for terminal in ["dōjō returned 401: bad refresh", "dōjō returned 403"] {
            assert!(
                matches!(refresh_failure(terminal), AuthError::Rejected(_)),
                "{terminal} must clear the session"
            );
        }
        for transient in [
            "dōjō returned 500: boom",
            "dōjō returned 502",
            "error sending request: connection refused",
            "operation timed out",
        ] {
            assert!(
                matches!(refresh_failure(transient), AuthError::Unreachable(_)),
                "{transient} must LEAVE the session alone"
            );
        }
    }

    #[test]
    fn every_auth_error_says_whether_signing_in_again_is_required() {
        // The cycle skips a persona on any of these; only one of them is worth
        // telling the user about, so the variants must stay distinguishable
        // rather than collapsing into one "auth failed".
        assert!(AuthError::SignedOut.needs_sign_in());
        assert!(AuthError::Rejected("401".into()).needs_sign_in());
        assert!(
            !AuthError::Unreachable("timeout".into()).needs_sign_in(),
            "a network blip is not a sign-out"
        );
    }

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
