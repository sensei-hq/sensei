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
static PENDING_VERIFIER: Mutex<Option<String>> = Mutex::new(None);

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
pub(crate) async fn signin() -> Json<serde_json::Value> {
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
    let url = pkce::authorize_url(
        &supabase_url(),
        "github",
        &callback_url(),
        &challenge,
        EXTRA_SCOPES,
    );
    *PENDING_VERIFIER.lock().unwrap_or_else(|e| e.into_inner()) = Some(verifier);
    Json(serde_json::json!({ "authorizeUrl": url, "callback": callback_url() }))
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
    let Some(verifier) = PENDING_VERIFIER.lock().unwrap_or_else(|e| e.into_inner()).take() else {
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
                let stored = tokio::task::spawn_blocking(move || {
                    session::store_refresh_token(&refresh)
                })
                .await;
                match stored {
                    Ok(Ok(())) => Json(serde_json::json!({ "ok": true, "signedIn": true })),
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
pub(crate) async fn signout() -> Json<serde_json::Value> {
    match tokio::task::spawn_blocking(session::clear_refresh_token).await {
        Ok(Ok(())) => Json(serde_json::json!({ "ok": true, "signedIn": false })),
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
pub(crate) async fn status() -> Json<serde_json::Value> {
    let stored = tokio::task::spawn_blocking(session::load_refresh_token).await;
    let Ok(Ok(refresh)) = stored else {
        return Json(serde_json::json!({ "signedIn": false, "supabaseUrl": supabase_url() }));
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
                let _ = tokio::task::spawn_blocking(move || {
                    session::store_refresh_token(&rotated)
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
            let _ = tokio::task::spawn_blocking(session::clear_refresh_token).await;
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
