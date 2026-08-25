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
use crate::supabase::{pkce, session};

/// Scopes beyond the provider default (`user:email`).
///
/// `read:org` is what lets provisioning see the user's organisations at all —
/// without it GitHub returns none, which reads as "you belong to no orgs" rather
/// than "we did not ask".
const EXTRA_SCOPES: &[&str] = &["read:org"];

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
fn default_persona() -> String { "default".to_string() }

fn supabase_url() -> String {
    std::env::var("SUPABASE_URL").unwrap_or_else(|_| "http://127.0.0.1:54321".into())
}

fn anon_key() -> Option<String> {
    std::env::var("SUPABASE_ANON_KEY").ok()
}

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
    let mut url = pkce::authorize_url(
        &supabase_url(),
        "github",
        &callback_url(),
        &challenge,
        EXTRA_SCOPES,
    );
    if let Some(login) = p.github_login.as_deref().filter(|l| !l.is_empty()) {
        url = pkce::with_login_hint(&url, login);
    }
    *PENDING_VERIFIER.lock().unwrap_or_else(|e| e.into_inner()) = Some((p.persona.clone(), verifier));
    Json(serde_json::json!({
        "authorizeUrl": url,
        "callback": callback_url(),
        "persona": p.persona,
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
    let _ = &state;

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
    let Some((persona, verifier)) = PENDING_VERIFIER.lock().unwrap_or_else(|e| e.into_inner()).take() else {
        return Json(serde_json::json!({
            "ok": false,
            "error": "no sign-in in progress — start with POST /api/auth/signin",
        }));
    };

    let Some(key) = anon_key() else {
        return Json(serde_json::json!({
            "ok": false,
            "error": "SUPABASE_ANON_KEY is not set — the token endpoint requires an apikey header",
        }));
    };

    let resp = crate::federation::http_client()
        .post(session::token_url(&supabase_url(), "pkce"))
        .header("apikey", &key)
        .header("Content-Type", "application/json")
        .json(&session::pkce_exchange_body(&code, &verifier))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => match r.json::<session::TokenResponse>().await {
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
                let linked = link_verified_identity(
                    &state, &persona, &tokens.access_token, tokens.provider_token.as_deref(),
                )
                .await;

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
            Err(e) => Json(serde_json::json!({
                "ok": false, "error": format!("token response was not readable: {e}"),
            })),
        },
        // Surface the provider's own message. "invalid grant" with no context is
        // the single most confusing failure in this flow, and the body usually
        // says which half is wrong.
        Ok(r) => {
            let status = r.status().as_u16();
            let body = r.text().await.unwrap_or_default();
            Json(serde_json::json!({ "ok": false, "error": "token exchange rejected", "status": status, "detail": body }))
        }
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
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
        Ok(Ok(())) => Json(serde_json::json!({ "ok": true, "signedIn": false, "persona": p.persona })),
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
pub(crate) async fn status(Query(p): Query<PersonaQuery>) -> Json<serde_json::Value> {
    let who = p.persona.clone();
    let stored = tokio::task::spawn_blocking(move || session::load_refresh_token(&who)).await;
    let Ok(Ok(refresh)) = stored else {
        return Json(serde_json::json!({
            "signedIn": false, "persona": p.persona, "supabaseUrl": supabase_url() }));
    };
    let Some(key) = anon_key() else {
        return Json(serde_json::json!({
            "signedIn": false,
            "error": "SUPABASE_ANON_KEY is not set",
            "supabaseUrl": supabase_url(),
        }));
    };

    let resp = crate::federation::http_client()
        .post(session::token_url(&supabase_url(), "refresh_token"))
        .header("apikey", &key)
        .header("Content-Type", "application/json")
        .json(&session::refresh_body(&refresh))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => match r.json::<session::TokenResponse>().await {
            Ok(t) => {
                let now = chrono::Utc::now().timestamp();
                let s = session::Session::from_response(&t, now);
                // Supabase rotates the refresh token on use; storing the new one
                // is not optional — keeping the old would invalidate the session
                // on the NEXT call, which looks like a random sign-out.
                let rotated = t.refresh_token.clone();
                let who2 = p.persona.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    session::store_refresh_token(&who2, &rotated)
                })
                .await;
                // Prove the access token actually authenticates, and learn WHO
                // it authenticates as. A refresh that succeeds only shows the
                // refresh token is live; it says nothing about whether the
                // resulting access token is accepted, and the auth user id is
                // what a principal is keyed to — so this is also the lookup the
                // linking step needs.
                let identity = crate::federation::http_client()
                    .get(format!("{}/auth/v1/user", supabase_url().trim_end_matches('/')))
                    .header("apikey", &key)
                    .header("Authorization", format!("Bearer {}", s.access_token))
                    .send()
                    .await
                    .ok();
                let (auth_user_id, email) = match identity {
                    Some(r) if r.status().is_success() => {
                        let v: serde_json::Value = r.json().await.unwrap_or_default();
                        (v["id"].as_str().map(String::from), v["email"].as_str().map(String::from))
                    }
                    _ => (None, None),
                };
                Json(serde_json::json!({
                    "signedIn": auth_user_id.is_some(),
                    "persona": p.persona,
                    "authUserId": auth_user_id,
                    "email": email,
                    "expiresAt": s.expires_at,
                    "needsRefresh": s.needs_refresh(now),
                    "supabaseUrl": supabase_url(),
                }))
            }
            Err(e) => Json(serde_json::json!({ "signedIn": false, "error": e.to_string() })),
        },
        Ok(r) => {
            // A rejected refresh token is terminal, not transient: clear it so the
            // daemon stops retrying a credential the server will never accept and
            // the user is told to sign in again.
            let status = r.status().as_u16();
            let who3 = p.persona.clone();
            let _ = tokio::task::spawn_blocking(move || session::clear_refresh_token(&who3)).await;
            Json(serde_json::json!({
                "signedIn": false,
                "error": "stored session was rejected — sign in again",
                "status": status,
                "supabaseUrl": supabase_url(),
            }))
        }
        Err(e) => Json(serde_json::json!({
            "signedIn": false,
            "error": format!("could not reach Supabase: {e}"),
            "supabaseUrl": supabase_url(),
        })),
    }
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
    let token = match tokio::task::spawn_blocking(move || session::load_provider_token(&who)).await {
        Ok(Ok(t)) => t,
        _ => {
            return Json(serde_json::json!({
                "ok": false,
                "persona": p.persona,
                "error": "no GitHub token stored for this persona — sign in again to capture one",
                "detail": "the provider token is returned only at the exchange; a session created before it was captured has none",
            }))
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
        Err(e) => Json(serde_json::json!({ "ok": false, "persona": p.persona, "error": e.to_string() })),
    }
}

/// Record the verified GitHub identity for a persona, and adopt the account's
/// emails as claimed aliases.
///
/// Best-effort by design: sign-in has already succeeded by the time this runs, so
/// a failure here degrades the persona to "connected but unverified" rather than
/// discarding a working session. The outcome is reported so the caller can say
/// so rather than showing a confident-looking label that was never proven.
async fn link_verified_identity(
    state: &AppState,
    persona: &str,
    access_token: &str,
    provider_token: Option<&str>,
) -> serde_json::Value {
    let Some(key) = anon_key() else {
        return serde_json::json!({ "verified": false, "reason": "SUPABASE_ANON_KEY not set" });
    };

    // The GitHub login and numeric id come from Supabase's own record of the
    // identity — no extra GitHub call, and it is what the provider actually
    // asserted at sign-in.
    let user: serde_json::Value = match crate::federation::http_client()
        .get(format!("{}/auth/v1/user", supabase_url().trim_end_matches('/')))
        .header("apikey", &key)
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
        _ => return serde_json::json!({ "verified": false, "reason": "could not read the auth user" }),
    };

    let ident = user["identities"].as_array().and_then(|a| {
        a.iter().find(|i| i["provider"].as_str() == Some("github"))
    });
    let (Some(login), Some(gh_id)) = (
        ident.and_then(|i| i["identity_data"]["user_name"].as_str()),
        ident
            .and_then(|i| i["identity_data"]["provider_id"].as_str())
            .and_then(|v| v.parse::<i64>().ok())
            .or_else(|| ident.and_then(|i| i["identity_data"]["provider_id"].as_i64())),
    ) else {
        return serde_json::json!({ "verified": false, "reason": "no github identity on this account" });
    };

    // All verified addresses, not just the primary — the point of aliases is
    // that one human commits under several, and only GitHub knows the full set.
    let mut emails: Vec<String> = user["email"].as_str().map(|e| vec![e.to_string()]).unwrap_or_default();
    if let Some(pt) = provider_token
        && let Ok(r) = crate::federation::http_client()
            .get("https://api.github.com/user/emails")
            .header("Authorization", format!("Bearer {pt}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "sensei")
            .send()
            .await
            && r.status().is_success()
        {
            let list: serde_json::Value = r.json().await.unwrap_or_default();
            if let Some(a) = list.as_array() {
                for e in a {
                    // Only VERIFIED addresses become claimed aliases. An
                    // unverified one proves nothing — anyone can add any address
                    // to a GitHub account and leave it unconfirmed.
                    if e["verified"].as_bool() == Some(true)
                        && let Some(addr) = e["email"].as_str()
                    {
                        emails.push(addr.to_string());
                    }
                }
            }
        }
    emails.sort();
    emails.dedup();

    match state.pg.link_persona_identity(persona, login, gh_id, &emails).await {
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

