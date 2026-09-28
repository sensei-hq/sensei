//! Keep each persona's forge-token standing current — and say so when it dies.
//!
//! ## What this is for
//!
//! The GitHub token expires on roughly an 8-hour cycle (observed: a token minted
//! at sign-in was `401 Bad credentials` the next morning, and GitHub only issues
//! the `provider_refresh.<slot>` that sits in the Keychain when expiry is
//! enabled). Nothing renewed it and nothing recorded the death, so every
//! forge-dependent operation degraded while `/api/auth/status` kept reporting
//! `signedIn: true` — that flag describes the SUPABASE session, a different
//! credential that refreshes on every use.
//!
//! ## Due is not the same as worth doing
//!
//! The ticker decides DUE from `last_run_at + interval`, so a run missed while
//! the machine slept simply happens on the next poll. That is right for catching
//! up and wrong for acting blindly: a refresh scheduled for the 7th hour of an
//! 8-hour token, running at the 9th, would spend a call on a credential the
//! forge has already dropped — and the resulting 401 reads like a network blip
//! rather than "sign in again".
//!
//! So [`forge_token_action`] decides what is worth doing, per persona, and this
//! module carries it out.
//!
//! ## Renewal is decided here and performed by the app
//!
//! Nothing in this task renews a token. Redeeming a refresh token requires the
//! GitHub App's client secret, and that secret lives in exactly ONE place —
//! Supabase's auth provider configuration. A second copy in the dōjō would mean
//! recreating the App credential in two dashboards, and the copy that got missed
//! would fail silently months later as an unrenewable token.
//!
//! So renewal runs the authorize flow Supabase already owns — the same PKCE flow
//! `POST /api/auth/signin` starts. For a user who has already authorized the App
//! that is a redirect with no prompt, but it still needs a browser, which this
//! task does not have.
//!
//! What this task does is keep the recorded expiry current. `GET
//! /api/auth/status` derives `renewalDue` from it using the SAME
//! [`forge_token_action`] called here, so the worker and the UI cannot disagree
//! about when a token is near enough to expiry to act on.

use std::sync::Arc;

use crate::db::pg_store::PgStore;
use crate::dojo_client::forge_token::{
    EXPIRY_HEADER, ForgeTokenAction, ProbeOutcome, classify_probe, forge_token_action,
    token_state_of,
};

/// GitHub's own account probe. Cheap, and the response carries the expiry header.
const PROBE_URL: &str = "https://api.github.com/user";

/// Ask the forge about one token.
async fn probe(token: &str) -> ProbeOutcome {
    let resp = crate::federation::http_client()
        .get(PROBE_URL)
        .bearer_auth(token)
        .header("User-Agent", "sensei-daemon")
        .send()
        .await;
    match resp {
        Ok(r) => {
            let status = r.status().as_u16();
            let exp =
                r.headers().get(EXPIRY_HEADER).and_then(|v| v.to_str().ok()).map(str::to_owned);
            classify_probe(Some(status), exp.as_deref())
        }
        // No response at all. Says nothing about the token — see `classify_probe`.
        Err(_) => classify_probe(None, None),
    }
}

/// One pass: consider every signed-in persona.
pub async fn tick(pg: Arc<PgStore>) -> Result<(), String> {
    let rows = pg.forge_token_rows().await?;
    let now = chrono::Utc::now().timestamp();

    for row in rows {
        let action = forge_token_action(row.expires_at, now, token_state_of(&row.state));
        match action {
            // Nothing to learn by asking. Re-probing a token already known dead
            // is a per-interval call to GitHub that cannot change the answer.
            ForgeTokenAction::Skip => continue,

            // Near expiry. The daemon CANNOT renew this itself, so it falls
            // through to the same probe as `Verify` — which keeps the recorded
            // expiry current, and that expiry is exactly what
            // `GET /api/auth/status` turns into `renewalDue` for the app to act
            // on. Nothing is written here that says "renewal due": the deadline
            // already implies it, and a second field would be a copy that can
            // disagree with the timestamp it was derived from.
            ForgeTokenAction::Refresh
            | ForgeTokenAction::VerifyAndMarkDead
            | ForgeTokenAction::Verify => {
                let slot = row.session_slot.clone();
                let token = match tokio::task::spawn_blocking(move || {
                    crate::dojo_client::session::load_provider_token(&slot)
                })
                .await
                {
                    Ok(Ok(t)) => t,
                    // No credential in the Keychain for a persona the table says
                    // is signed in. Record `absent` rather than leaving the row
                    // claiming a token that is not there.
                    _ => {
                        if let Err(e) =
                            pg.set_forge_token_state(&row.session_slot, "absent", None).await
                        {
                            tracing::warn!(slot = row.session_slot, error = %e,
                                           "forge_token_check: could not record absent");
                        }
                        continue;
                    }
                };

                let outcome = probe(&token).await;
                let (state, expires_at) = match outcome {
                    ProbeOutcome::Alive { expires_at } => ("active", expires_at),
                    ProbeOutcome::Dead => ("dead", None),
                    // We could not ask. Write NOTHING: leaving the previous
                    // standing in place is the only honest option, and stamping
                    // `checked_at` would claim we learned something.
                    ProbeOutcome::Unreachable => {
                        tracing::debug!(
                            slot = row.session_slot,
                            "forge_token_check: forge unreachable, standing unchanged"
                        );
                        continue;
                    }
                };

                if let Err(e) = pg.set_forge_token_state(&row.session_slot, state, expires_at).await
                {
                    tracing::warn!(slot = row.session_slot, error = %e,
                                   "forge_token_check: could not record the outcome");
                    continue;
                }

                match (action, state) {
                    (_, "dead") => tracing::warn!(
                        slot = row.session_slot,
                        "forge_token_check: forge token is DEAD — sign in again to restore sync"
                    ),
                    _ => tracing::debug!(
                        slot = row.session_slot,
                        state,
                        "forge_token_check: recorded"
                    ),
                }
            }
        }
    }
    Ok(())
}

/// Spawn the scheduled loop. Cadence lives in `sensei.schedules` (`forge_token`).
pub fn spawn(pg: Arc<PgStore>) {
    tokio::spawn(async move {
        crate::tasks::ticker::run_scheduled(pg.clone(), "forge_token", move || {
            let pg = pg.clone();
            async move { tick(pg).await }
        })
        .await;
    });
}
