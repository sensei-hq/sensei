//! What the daemon does when the `dojo_sync` schedule fires.
//!
//! The first task with no bespoke worker: no ticker, no interval constant, no
//! `dojo_sync_scheduler` module. `main` hands this `tick` to
//! [`crate::tasks::ticker::run_scheduled`] and the cadence lives in
//! `sensei.schedules` like every other worker's. That is the whole point of
//! docs/spec/daemon/schedules.md step 5 — adding a scheduled task is now a row, a
//! registry entry and a `tick()`.
//!
//! # What one pass does
//!
//! ```text
//! for each persona that has signed in:            PgStore::signed_in_personas
//!     token = live access token                   skip this persona on failure
//!     offered = every CLONED repo with a repo_key the offer set, §8a
//!     POST /v1/you/repositories                   identity: which tenant?
//!         store tenant_id per repo                D2
//!         log unmapped[]                          D6
//!     GET  /v1/you/sync/plan                      may_share AND elected
//!         on failure: record and skip the persona D7
//!     POST /v1/you/metrics                        push the rows the plan allows
//!         mark shared_at ONLY if the whole batch landed  the re-push watermark
//! ```
//!
//! # Who decides that a repository syncs
//!
//! Two questions, not one (`docs/requirements/repository-sharing.md`):
//! **entitlement** — may it? — and **election** — did whoever holds authority
//! choose it? The dōjō answers both, in `dojo.all_my_repositories`, and hands the
//! daemon the conjunction as `plan.allowed`.
//!
//! The daemon's own gate 1 (`sensei.repositories.visibility = 'shared'`) used to
//! filter what was OFFERED. It no longer does, because authority is not always the
//! user's: an organization's private code is elected by the organization, and that
//! has no local representation the daemon could test. What bounds the disclosure
//! instead is the CLONE — `sensei.repositories` comes from the scanner, so
//! belonging to an org whose code is not on this machine discloses nothing.
//!
//! Stated plainly, because it narrows a promise: *"nothing leaves the machine
//! without local consent"* is now *"nothing leaves the machine without local
//! consent, or an organization's mandate on that organization's own private
//! code"*.
//!
//! # What it deliberately does NOT do yet
//!
//! **User-scoped rows are held back**, counted, and logged with the reason.
//! `dojo.repository_metrics.principal_id` is a principal, never a git email —
//! commit trailers are unverified — and `personas.principal_id` is unset until a
//! persona is linked, so there is nothing honest to attribute a per-person row
//! to. The ingest endpoint refuses them, so sending them would earn a rejection
//! every cycle forever.
//!
//! Per-repo governance pull (D3) is not here.

use std::sync::Arc;

use crate::api::handlers::auth::{AuthError, live_access_token};
use crate::db::pg_store::PgStore;
use crate::db::pg_store::sync::SyncMark;
use crate::dojo_client::dojo_auth;
use crate::dojo_client::settings::dojo_url;
use crate::dojo_client::user_plane::{self, HttpUserPlane, RepoInput, UserPlane};

/// How many repositories one pass will offer for registration at a time.
///
/// Since §8a this bounds the CLONE set, not the elected subset, so it is roughly
/// two orders of magnitude closer to biting. `offerable_repositories` orders the
/// window so it stays a throughput bound rather than a ceiling.
const REGISTER_LIMIT: i64 = 500;

/// How many metric rows one pass will push. The dōjō caps a batch at 1000; the
/// daemon pages rather than sending an unbounded body, because a memory profile
/// set by the client is not a decision a client gets to make.
const PUSH_LIMIT: i64 = 500;

/// Reason codes that a fresh capture can actually resolve.
///
/// Kept as a list rather than a substring test: `forge_visibility_unknown` and
/// `forge_visibility_stale` are the two the dōjō emits for "nobody has asked the
/// forge lately", and a prefix match would silently adopt any future
/// `forge_visibility_*` code whose remedy is NOT a re-capture.
const CAPTURE_FIXES: [&str; 2] = ["forge_visibility_unknown", "forge_visibility_stale"];

/// Should this pass ask the dōjō to re-read forge visibility?
///
/// The dōjō captures visibility at SIGN-IN, but repositories keep registering
/// through this task — so a repo cloned afterwards is denied for want of an
/// answer nobody is going to ask for, and the remedy shown to the user is "sign
/// in again". The daemon holds the forge token, so it can ask instead.
///
/// Deliberately narrow. Every other denial — subscription, seat, election — is a
/// decision the forge cannot change, and re-capturing on those would be a
/// standing per-minute call to GitHub that could not move the verdict.
fn needs_forge_capture(denied: &[crate::dojo_client::user_plane::DeniedRepo]) -> bool {
    denied.iter().any(|d| CAPTURE_FIXES.contains(&d.reason.as_str()))
}

/// The `sensei.sync_entity` value a whole-cycle plan fetch is recorded against.
/// Keyed on the persona's KEYCHAIN SLOT (`personas.session_slot`), so two personas'
/// failures stay distinguishable — never the label, which a sign-in rewrites.
const PLAN_ENTITY: &str = "dojo_sync_plan";

/// One pass of the dōjō sync cycle, for every signed-in persona.
///
/// Returns `Err` only when the pass could not be attempted at all (the persona
/// list could not be read). A persona that individually fails is logged and
/// skipped: one expired session must not stall the others, which is the failure
/// D1 exists to prevent.
pub async fn tick(pg: Arc<PgStore>) -> Result<(), String> {
    let personas = pg.signed_in_personas().await?;
    if personas.is_empty() {
        tracing::debug!("dojo_sync: no signed-in personas");
        return Ok(());
    }

    let plane = HttpUserPlane { dojo_url: dojo_url() };
    let total = personas.len();
    let mut failures = Vec::new();
    for persona in personas {
        if let Err(e) = sync_one(&pg, &persona, &plane).await {
            tracing::warn!(persona, error = %e, "dojo_sync: persona skipped");
            failures.push(format!("{persona}: {e}"));
        }
    }
    // A pass where EVERY persona failed is not a success. `run_scheduled` records
    // this return value as the schedule's `last_ok`, so swallowing it printed a
    // green worker over a cycle that moved nothing — the exact false report
    // observed live (plan rows 0, shared 0, last_ok = true). A PARTIAL failure
    // still returns Ok: D1's whole point is that one expired session must not
    // stall the others, and the survivors did work.
    if failures.len() == total {
        return Err(format!("all {total} personas failed: {}", failures.join("; ")));
    }
    Ok(())
}

/// Resolve the persona's credential, then run the cycle.
///
/// Split from [`sync_persona`] so the cycle itself is reachable in a test: token
/// resolution needs the real Keychain, and everything after it does not.
async fn sync_one(pg: &PgStore, persona: &str, plane: &dyn UserPlane) -> Result<(), String> {
    let token = match live_access_token(persona).await {
        Ok(t) => t,
        Err(e) => {
            // Distinguished rather than collapsed: "sign in again" is worth
            // saying out loud, a network blip is not worth saying at all.
            let needs_sign_in = e.needs_sign_in();
            return Err(match e {
                AuthError::SignedOut => "no stored session".to_string(),
                AuthError::Rejected(d) => format!("session rejected, sign in again: {d}"),
                AuthError::Unreachable(d) if !needs_sign_in => {
                    format!("dōjō unreachable, session kept: {d}")
                }
                AuthError::Unreachable(d) => d,
            });
        }
    };
    sync_persona(pg, persona, &token, plane).await
}

/// Whether a user-plane failure is the dōjō refusing the CREDENTIAL itself.
///
/// Reuses `dojo_auth`'s classifier rather than matching on message text here.
/// That classifier already parsed this exact shape — `user_plane`'s errors begin
/// "dōjō returned {status} for the {what}" — but it had exactly ONE call site in
/// the workspace, the refresh leg. So a 401 or 403 from `/v1/you/*` was recorded
/// indistinguishably from a 500: the session was kept, nothing said "sign in
/// again", and the daemon re-sent into the same refusal every 60s forever.
///
/// Narrow on purpose. Only 401/403 are the server's verdict on the credential;
/// telling a user to re-authenticate over a 5xx sends them to fix an outage they
/// did not cause.
fn refuses_the_session(error: &str) -> bool {
    dojo_auth::status_of(error).is_some_and(dojo_auth::is_rejection)
}

/// Record a user-plane failure against every row it bears on, and return the
/// message.
///
/// One place for the two obligations that were previously applied unevenly:
///
/// - **Record it.** Without a row a failed cycle is indistinguishable from a
///   cycle with nothing to do. Registration had no such row at all.
/// - **Classify it.** A refusal names its remedy; a fault does not pretend to.
///
/// Takes MARKS rather than one mark because the cycle's three network calls are
/// scoped differently: registration and the plan are per persona, the push is
/// per repository in the batch. The push previously kept its own recording loop
/// and so was the one site that never classified — the same 403 named its remedy
/// on two screens and read as an unexplained fault on the third.
///
/// The bookkeeping write is deliberately NOT `?`: a failed `sync_state` write
/// must not replace the real error with itself, which would report a storage
/// problem where the dōjō gave a perfectly clear answer.
async fn record_plane_failure(
    pg: &PgStore,
    marks: &[SyncMark<'_>],
    persona: &str,
    what: &str,
    error: String,
) -> String {
    let message = match refuses_the_session(&error) {
        true => {
            tracing::error!(persona, what, error = %error,
                            "dojo_sync: the dōjō refused this session — it will not recover on its own");
            format!("sign in again — {error}")
        }
        false => error,
    };
    for mark in marks {
        if let Err(be) = pg.mark_sync_error(mark, &message).await {
            tracing::warn!(persona, what, key = mark.key, error = %be,
                           "dojo_sync: could not record the failure");
        }
    }
    message
}

/// One persona's cycle, with the credential and the transport handed in.
async fn sync_persona(
    pg: &PgStore,
    persona: &str,
    token: &str,
    plane: &dyn UserPlane,
) -> Result<(), String> {
    // THE OFFER SET (§8a, finding B1): every locally-scanned repository with a
    // `repo_key` — what the user CLONED — and NOT gate 1's subset.
    //
    // This used to be `shared_repositories`, and the early return below then fired
    // for a new employee whose only repository is the org-mandated private one:
    // nothing locally elected, so the pass returned before register, before the
    // plan, before any push, and the mandate was unreachable for exactly the
    // population it serves. The daemon holds no local fact that says "this one is
    // mandated" — only the plan does — so it must ask FIRST and filter after.
    //
    // What bounds the disclosure is the CLONE. `sensei.repositories` is populated
    // by the scanner, never by a forge listing, so this is what the user actually
    // works on; membership of an org whose code is not on this machine discloses
    // nothing.
    //
    // This comment used to end "Gate 1 has not been discarded — it moved to the
    // push, below." That is NOT true of the current code and is contradicted
    // twelve lines into `push_allowed`, which says `sensei.repositories
    // .visibility` is "deliberately NOT re-tested here or in the query".
    // `shared_repositories` — the function that applies gate 1 — has zero
    // production callers; every call site is inside a `#[cfg(test)]` module. So
    // local visibility gates nothing today, for user-authority repositories as
    // well as org-mandated ones. Whether the dōjō election is intended to
    // supersede the local column, or the SQL term must be restored, is an open
    // question for the user — recorded rather than silently decided here.
    let offered = pg.offerable_repositories(REGISTER_LIMIT).await?;
    if offered.is_empty() {
        tracing::debug!(persona, "dojo_sync: no cloned repository has a cross-install identity");
        return Ok(());
    }

    let inputs: Vec<RepoInput<'_>> = offered
        .iter()
        .map(|r| RepoInput {
            repo_key: &r.repo_key,
            remote_url: r.remote_url.as_deref(),
            name: &r.name,
        })
        .collect();

    // The mark is taken BEFORE the first network call, not just before the plan.
    // Registration is the first thing that talks to the dōjō, so a `?` here
    // aborted the persona before any `mark_sync_error` could run — leaving the
    // row from the last good pass reading `synced` while nothing worked. That is
    // the exact failure the plan row exists to prevent, and it was unguarded for
    // the call that fails FIRST when a dōjō is down.
    let mark = SyncMark { entity: PLAN_ENTITY, key: persona, direction: "pull" };

    let registered = match plane.register_repositories(token, &inputs).await {
        Ok(r) => r,
        Err(e) => {
            return Err(record_plane_failure(
                pg,
                std::slice::from_ref(&mark),
                persona,
                "registration",
                e,
            )
            .await);
        }
    };
    for m in &registered.mapped {
        match uuid::Uuid::parse_str(&m.tenant_id) {
            // Store what the dōjō said, never a guess: an unparseable tenant id
            // is a bug on one side or the other, and writing a placeholder would
            // bury it under a plausible-looking row.
            Ok(id) => {
                if let Err(e) = pg.set_repository_tenant(&m.repo_key, id).await {
                    tracing::warn!(persona, repo = m.repo_key, error = %e,
                                   "dojo_sync: could not store the tenant mapping");
                }
            }
            Err(e) => tracing::warn!(persona, repo = m.repo_key, tenant_id = m.tenant_id,
                                     error = %e, "dojo_sync: dōjō sent an unparseable tenant id"),
        }
    }
    for u in &registered.unmapped {
        // Each reason is a different problem with different advice (D6), so they
        // are logged as themselves rather than counted.
        tracing::info!(
            persona,
            repo = u.repo_key,
            reason = u.reason,
            "dojo_sync: repository not mapped to a tenant"
        );
    }

    let plan = match plane.sync_plan(token).await {
        Ok(p) => p,
        Err(e) => {
            return Err(record_plane_failure(
                pg,
                std::slice::from_ref(&mark),
                persona,
                "sync plan",
                e,
            )
            .await);
        }
    };
    for d in &plan.denied {
        tracing::info!(
            persona,
            repo = d.repo_key,
            tenant = d.tenant,
            reason = d.reason,
            "dojo_sync: repository not permitted to sync"
        );
    }

    // SELF-HEAL the one denial the daemon can answer itself.
    //
    // The dōjō captures forge visibility at SIGN-IN, but repositories keep
    // registering through this task — so anything cloned afterwards is refused
    // for want of an answer nobody is going to ask for, and the remedy the user
    // is shown is "sign in again". Observed live: four of five registered
    // repositories sat at `forge_visibility_unknown`, the private one included.
    //
    // The daemon holds the forge token, so it asks and re-reads the verdict.
    // Best-effort throughout: a capture failure leaves the denial standing and
    // this pass proceeds with the plan it already has, because a repository that
    // cannot be captured is exactly one that must not sync.
    let plan = if needs_forge_capture(&plan.denied) {
        match forge_token_for(persona).await {
            Ok(provider) => {
                match recapture_and_replan(persona, token, &provider, plane, plan).await {
                    Ok(p) => p,
                    Err((e, kept)) => {
                        tracing::warn!(persona, error = %e,
                                   "dojo_sync: could not refresh forge visibility; keeping the current plan");
                        kept
                    }
                }
            }
            Err(e) => {
                // Nothing to ask WITH. Not a pass failure: the denial simply
                // stands, which is the safe direction.
                tracing::info!(persona, error = %e,
                               "dojo_sync: no forge token, so visibility stays uncaptured");
                plan
            }
        }
    } else {
        plan
    };

    pg.mark_synced(&mark, None).await?;

    // Record what every consuming tenant has switched off, for the metric tasks
    // to skip. Best-effort: a write failure costs one cycle of extra computation,
    // never correctness, so it must not fail the sync that just succeeded.
    let disabled = crate::dojo_client::user_plane::disabled_everywhere(&plan);
    match pg.replace_metric_deactivations(&disabled).await {
        Ok(n) => tracing::debug!(persona, rows = n, "recorded metric deactivations"),
        // Best-effort: a write failure costs one cycle of extra computation,
        // never correctness, so it must not fail the sync that just succeeded.
        Err(e) => tracing::warn!(persona, error = %e, "could not record metric deactivations"),
    }

    let pushed = push_allowed(pg, persona, token, plane, &plan).await?;
    tracing::info!(
        persona,
        allowed = plan.allowed.len(),
        denied = plan.denied.len(),
        mapped = registered.mapped.len(),
        unmapped = registered.unmapped.len(),
        pushed,
        "dojo_sync: cycle complete"
    );
    Ok(())
}

/// Push the metric rows the plan allows, and mark exactly those as shared.
///
/// Returns how many rows the dōjō accepted.
async fn push_allowed(
    pg: &PgStore,
    persona: &str,
    token: &str,
    plane: &dyn UserPlane,
    plan: &user_plane::SyncPlan,
) -> Result<u32, String> {
    // The allow-list, as a set, and since §8a it is the ONLY gate on the push. The
    // daemon syncs the set it was HANDED — it never asks "may I sync X?", so it
    // cannot include a repository it never offered, and offline degrades to no-sync
    // by construction.
    //
    // `sensei.repositories.visibility` is deliberately NOT re-tested here or in the
    // query: the dōjō's `all_my_repositories` is the single place `may_share AND
    // elected` is decided, and an org's mandate over its own private code has no
    // local representation to test. See `unpushed_metric_rows` for the full
    // argument and for the deployment-ordering constraint it carries.
    let allowed: std::collections::HashSet<&str> =
        plan.allowed.iter().map(|a| a.repo_key.as_str()).collect();
    if allowed.is_empty() {
        return Ok(0);
    }

    // `["repo"]`, filtered in SQL so the LIMIT applies to rows we can actually
    // push. Scope is a CAPABILITY question, not an entitlement one:
    // `dojo.repository_metrics.principal_id` is a principal, never a git email,
    // and `personas.principal_id` is unset until a persona is linked, so a
    // per-person row has nothing honest to attribute to. The ingest refuses them.
    //
    // Filtering here rather than after the fetch is load-bearing — see
    // `unpushed_metric_rows`. Held-back rows once crowded the window and a pass
    // pushed 66 of 132.
    let keys: Vec<&str> = allowed.iter().copied().collect();
    let pushable = pg.unpushed_metric_rows(&["repo"], &keys, PUSH_LIMIT).await?;
    // A failure here must not print "0 held back" — that is the exact mislead the
    // counter exists to prevent.
    // Scoped to the SAME allow-list, so the number means "held back out of what this
    // pass could otherwise have sent" rather than "user-scoped rows on this machine".
    let held = match pg.unpushed_metric_count(&["user"], &keys).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(persona, error = %e, "could not count held-back rows");
            0
        }
    };
    if held > 0 {
        tracing::info!(
            persona,
            held,
            "dojo_sync: user-scoped rows held back — no principal to attribute them to yet"
        );
    }
    if pushable.is_empty() {
        return Ok(0);
    }

    let batch: Vec<user_plane::MetricPush<'_>> = pushable
        .iter()
        .map(|m| user_plane::MetricPush {
            repo_key: &m.repo_key,
            metric: &m.metric,
            scope: &m.scope,
            grain: &m.grain,
            computed_on: m.computed_on.to_string(),
            value: m.value,
            commit_sha: m.commit_sha.as_deref(),
            props: &m.props,
            source: &m.source,
        })
        .collect();

    // Which repositories this batch covers — the keys the outcome is recorded
    // against, so a failure is durable rather than a log line nobody tailed.
    let repos: std::collections::BTreeSet<&str> =
        pushable.iter().map(|m| m.repo_key.as_str()).collect();
    fn push_mark<'k>(key: &'k str) -> SyncMark<'k> {
        SyncMark { entity: "repository_metric", key, direction: "push" }
    }

    let result = match plane.push_metrics(token, &batch).await {
        Ok(r) => r,
        Err(e) => {
            // RECORDED, not just logged. Without this a push that fails every
            // cycle is invisible: the plan row still reads `synced`, the schedule
            // still reads ok, and the only symptom is that nothing ever arrives.
            // Observed exactly that way before this existed.
            //
            // Through `record_plane_failure`, like registration and the plan. Its
            // own loop used to live here and it was therefore the ONE site that
            // never classified a refusal — a 403 on the push named no remedy,
            // while the identical 403 on the plan said "sign in again". One mark
            // per repository in the batch, because that is what the push is
            // scoped to.
            let marks: Vec<SyncMark<'_>> = repos.iter().map(|k| push_mark(k)).collect();
            return Err(record_plane_failure(pg, &marks, persona, "metric push", e).await);
        }
    };
    for r in &result.rejected {
        tracing::warn!(
            persona,
            repo = r.repo_key,
            metric = r.metric,
            reason = r.reason,
            "dojo_sync: metric row refused"
        );
    }

    // Mark every row the dōjō did NOT refuse — the COMPLEMENT of `rejected`.
    //
    // This used to require an all-or-nothing batch, on the reasoning that "the
    // response reports a count, not which rows". That was wrong: `rejected[]`
    // carries `(repo_key, metric)`, which identifies exactly what to exclude. The
    // consequence was a livelock — one permanently-refused row (an `unknown_metric`
    // from version skew, say) meant NO row in the 500-row window was ever marked,
    // so the identical batch was re-sent every cadence forever and the queue never
    // drained.
    let refused: std::collections::HashSet<(&str, &str)> =
        result.rejected.iter().map(|r| (r.repo_key.as_str(), r.metric.as_str())).collect();
    let ids: Vec<uuid::Uuid> = pushable
        .iter()
        .filter(|m| !refused.contains(&(m.repo_key.as_str(), m.metric.as_str())))
        .map(|m| m.id)
        .collect();
    pg.mark_metric_rows_shared(&ids).await?;

    // A refusal belongs to the REPOSITORY whose row was refused. The batch is per
    // persona and spans many repositories, so marking them all `skipped` with a
    // `why` assembled from the first five rejections anywhere in the response
    // told a repository whose rows all landed that it was skipped because of a
    // DIFFERENT repository's metric — a denial naming the wrong cause, on the one
    // screen whose job is to name causes.
    let mut refusals: std::collections::HashMap<&str, Vec<String>> =
        std::collections::HashMap::new();
    for r in &result.rejected {
        refusals
            .entry(r.repo_key.as_str())
            .or_default()
            .push(format!("{}: {}", r.metric, r.reason));
    }
    for key in &repos {
        match refusals.get(*key) {
            // Nothing of this repository's was refused, whatever happened to the
            // rest of the batch.
            None => pg.mark_synced(&push_mark(key), None).await?,
            // `skipped`, not `error`: the dōjō answered, and a refusal is a
            // decision rather than a fault. Both mean "not synced", but only one
            // is a problem, and a dashboard that cannot tell them apart cries
            // wolf or stays silent.
            Some(reasons) => {
                let why = reasons.iter().take(5).cloned().collect::<Vec<_>>().join("; ");
                pg.mark_sync_skipped(&push_mark(key), &why).await?
            }
        }
    }
    if !result.rejected.is_empty() {
        tracing::warn!(
            persona,
            accepted = result.accepted,
            refused = result.rejected.len(),
            // This used to read "nothing marked shared, the batch retries next
            // cycle", which had been false since the complement fix above: the
            // rows that were NOT refused are marked, which is the whole point.
            "dojo_sync: partial acceptance — the refused rows stay queued, the rest are marked shared"
        );
    }
    Ok(result.accepted)
}

/// Run the cycle on its schedule, forever.
///
/// No interval argument and no config key: `run_scheduled` reads the cadence
/// from `sensei.schedules` (name `dojo_sync`) on every poll, so changing it is a
/// PATCH rather than a restart.
pub fn spawn(pg: Arc<PgStore>) {
    tokio::spawn(async move {
        let store = pg.clone();
        crate::tasks::ticker::run_scheduled(pg, "dojo_sync", move || {
            let pg = store.clone();
            async move { tick(pg).await }
        })
        .await;
    });
}

/// The persona's stored forge token. Split out so `recapture_and_replan` stays
/// free of the Keychain and is therefore testable against a scripted dōjō —
/// without this the success path could only ever run on a real machine with a
/// real sign-in, which is precisely the path most worth testing.
async fn forge_token_for(persona: &str) -> Result<String, String> {
    let who = persona.to_string();
    match tokio::task::spawn_blocking(move || {
        crate::dojo_client::session::load_provider_token(&who)
    })
    .await
    {
        Ok(Ok(t)) => Ok(t),
        Ok(Err(e)) => Err(format!("no forge token for {persona}: {e}")),
        Err(e) => Err(format!("forge token read panicked: {e}")),
    }
}

/// Hand the dōjō the forge token, then re-read the plan.
///
/// Returns the REFRESHED plan on success. On failure it returns the error
/// alongside the plan it was given, so the caller carries on with a verdict it
/// already trusts rather than losing the pass — a repository whose visibility
/// cannot be established is precisely one that must not sync, so degrading to
/// the stale (more restrictive) plan is the safe direction.
async fn recapture_and_replan(
    persona: &str,
    token: &str,
    provider: &str,
    plane: &dyn UserPlane,
    current: user_plane::SyncPlan,
) -> Result<user_plane::SyncPlan, (String, user_plane::SyncPlan)> {
    // `provision` now reports the dōjō's OWN verdict on the call, which arrives
    // as an HTTP 200 either way. Before this, a `{synced:false,
    // reason:'forge_unreachable'}` body was discarded and this function took its
    // success branch — announcing a refresh that never happened.
    let outcome = match plane.provision(token, provider).await {
        Ok(o) => o,
        Err(e) => return Err((e, current)),
    };
    let plan = match plane.sync_plan(token).await {
        Ok(p) => p,
        Err(e) => return Err((e, current)),
    };
    // NOT defaulted to zeros. The dōjō omits `visibility` precisely so a pass
    // that read nothing cannot be mistaken for one that read nothing NEW, and
    // filling in five zeros here would re-create the ambiguity it avoids.
    match outcome.visibility {
        // `captured == 0` is the shape that loops: the denial that triggered
        // this capture will still be there next cadence, and we will ask again.
        Some(v) if v.captured == 0 => tracing::warn!(
            persona,
            unavailable = v.unavailable,
            failed = v.failed,
            deferred = v.deferred,
            unsupported = v.unsupported,
            allowed = plan.allowed.len(),
            denied = plan.denied.len(),
            "dojo_sync: the forge capture moved nothing — the same denial will return next cycle"
        ),
        Some(v) => tracing::info!(
            persona,
            captured = v.captured,
            unavailable = v.unavailable,
            failed = v.failed,
            deferred = v.deferred,
            allowed = plan.allowed.len(),
            denied = plan.denied.len(),
            "dojo_sync: refreshed forge visibility and re-read the plan"
        ),
        None => tracing::warn!(
            persona,
            allowed = plan.allowed.len(),
            denied = plan.denied.len(),
            "dojo_sync: the dōjō provisioned but reported no forge capture at all"
        ),
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `forge_visibility_unknown` is the ONE denial the daemon can act on itself.
    ///
    /// The dōjō captures forge visibility only at sign-in, but repositories keep
    /// registering through this task — so a repo cloned after sign-in is denied
    /// for want of an answer nobody is going to ask for. Observed live: four of
    /// five registered repos sat at `forge_visibility_unknown`, including the one
    /// PRIVATE repo the whole authority model exists for, and the remedy shown to
    /// the user was "sign in again".
    ///
    /// The daemon holds the forge token. When the refusal names this reason, it
    /// can ask — so the refusal becomes self-healing instead of a standing chore.
    mod capture_trigger {
        use super::*;
        use crate::dojo_client::user_plane::DeniedRepo;

        fn denied(reason: &str) -> DeniedRepo {
            DeniedRepo {
                repo_key: "github.com/acme/api".into(),
                tenant: "organization/acme".into(),
                reason: reason.into(),
            }
        }

        #[test]
        fn asks_when_a_repo_is_denied_for_want_of_a_forge_answer() {
            assert!(needs_forge_capture(&[denied("forge_visibility_unknown")]));
        }

        #[test]
        fn does_not_ask_when_nothing_was_denied() {
            assert!(!needs_forge_capture(&[]));
        }

        #[test]
        fn does_not_ask_for_denials_capture_cannot_fix() {
            // Asking again changes nothing for these: the forge already answered,
            // and the refusal is a subscription or an election. Re-capturing every
            // 60s would be a standing call to GitHub that cannot move the verdict.
            for reason in [
                "not_subscribed",
                "subscription_expired",
                "no_seat",
                "not_elected_user",
                "not_elected_org",
                "unclaimed",
            ] {
                assert!(
                    !needs_forge_capture(&[denied(reason)]),
                    "{reason} must not trigger capture"
                );
            }
        }

        #[test]
        fn asks_when_at_least_one_of_several_denials_is_a_missing_answer() {
            assert!(needs_forge_capture(&[
                denied("not_subscribed"),
                denied("forge_visibility_unknown"),
            ]));
        }

        #[test]
        fn matches_the_list_not_the_prefix() {
            // The comment on CAPTURE_FIXES claims a prefix match would be wrong.
            // Without this test that claim is unenforced — `starts_with(
            // "forge_visibility")` passed every other test here.
            //
            // A future `forge_visibility_refused` (the forge answered: you may not
            // see this) is NOT fixable by asking again. Under a prefix match the
            // daemon would call GitHub every 60s forever for a verdict that will
            // never change.
            assert!(!needs_forge_capture(&[denied("forge_visibility_refused")]));
        }

        #[test]
        fn a_stale_answer_also_warrants_asking() {
            // `forge_visibility_stale` is the sibling code: an answer was captured
            // once and has since aged out. Same remedy, same actor.
            assert!(needs_forge_capture(&[denied("forge_visibility_stale")]));
        }
    }

    use crate::dojo_client::user_plane::{
        ActivationOutcome, DeniedRepo, IngestResult, MappedRepo, MetricPush, ProvisionOutcome,
        RegisterResult, RejectedMetric, SyncPlan, UnmappedRepo, VisibilityCounts,
    };
    use std::sync::Mutex;

    /// A scripted dōjō. Records what it was asked and answers what the test says.
    ///
    /// This is what makes the cycle testable at all: before the `UserPlane` trait,
    /// `tick`'s whole body could be replaced with `Ok(())` and both of its tests
    /// still passed.
    #[derive(Default)]
    struct StubPlane {
        register: Option<RegisterResult>,
        plan: Option<SyncPlan>,
        push: Option<Result<IngestResult, String>>,
        /// Every `repo_key` the daemon DISCLOSED — the offer set, as it reached the
        /// wire. Recorded because B1 is invisible from the outcome: a cycle that
        /// never asks looks exactly like a cycle the dōjō answered "nothing".
        offered: Mutex<Vec<String>>,
        plan_calls: Mutex<usize>,
        pushed_rows: Mutex<Vec<(String, String)>>,
        push_calls: Mutex<usize>,
        /// The plan served AFTER a successful `provision` — i.e. what the dōjō
        /// says once it has actually asked the forge. `None` means capture
        /// changed nothing, which is itself a case worth testing.
        plan_after_provision: Option<SyncPlan>,
        provision_calls: Mutex<usize>,
        /// Set to make `provision` fail, so the caller's degrade-to-stale path is
        /// exercised rather than assumed.
        provision_err: Option<String>,
        /// Set to make REGISTRATION fail — the cycle's FIRST network call, and
        /// the one whose failure left no trace at all.
        register_err: Option<String>,
        /// Set to make the PLAN read fail, so the classification of a refusal
        /// (401/403) against a fault (5xx) is exercised on the real path.
        plan_err: Option<String>,
    }

    #[async_trait::async_trait]
    impl UserPlane for StubPlane {
        async fn register_repositories(
            &self,
            _t: &str,
            r: &[RepoInput<'_>],
        ) -> Result<RegisterResult, String> {
            self.offered.lock().unwrap().extend(r.iter().map(|i| i.repo_key.to_string()));
            if let Some(e) = &self.register_err {
                return Err(e.clone());
            }
            Ok(self.register.clone().unwrap_or(RegisterResult { mapped: vec![], unmapped: vec![] }))
        }
        // dojo_sync never writes activation — it only READS the plan's
        // `disabled_metrics`. Unreachable rather than a fabricated Ok: a stub
        // that answered here would let a sync that started writing tenant cost
        // decisions pass its tests.
        async fn set_metric_activation(
            &self,
            _t: &str,
            _repo_key: &str,
            _metric: &str,
            _enabled: bool,
        ) -> Result<ActivationOutcome, String> {
            unreachable!("dojo_sync does not write metric activation")
        }
        async fn sync_plan(&self, _t: &str) -> Result<SyncPlan, String> {
            *self.plan_calls.lock().unwrap() += 1;
            if let Some(e) = &self.plan_err {
                return Err(e.clone());
            }
            // Keyed on whether the forge was actually ASKED, not on the call
            // count: the dōjō's answer changes because it captured, and a test
            // that calls `recapture_and_replan` directly makes only one plan
            // read. Counting reads made the stub model the call sequence rather
            // than the cause, and the success path silently tested nothing.
            if *self.provision_calls.lock().unwrap() > 0
                && let Some(after) = self.plan_after_provision.clone()
            {
                return Ok(after);
            }
            Ok(self.plan.clone().unwrap_or(SyncPlan { allowed: vec![], denied: vec![] }))
        }
        async fn provision(&self, _t: &str, _p: &str) -> Result<ProvisionOutcome, String> {
            *self.provision_calls.lock().unwrap() += 1;
            match &self.provision_err {
                Some(e) => Err(e.clone()),
                None => Ok(ProvisionOutcome {
                    synced: true,
                    reason: None,
                    visibility: Some(VisibilityCounts { captured: 1, ..Default::default() }),
                }),
            }
        }
        async fn push_metrics(
            &self,
            _t: &str,
            m: &[MetricPush<'_>],
        ) -> Result<IngestResult, String> {
            *self.push_calls.lock().unwrap() += 1;
            self.pushed_rows
                .lock()
                .unwrap()
                .extend(m.iter().map(|x| (x.repo_key.to_string(), x.metric.to_string())));
            self.push
                .clone()
                .unwrap_or(Ok(IngestResult { accepted: m.len() as u32, rejected: vec![] }))
        }
    }

    /// The self-heal, end to end against a scripted dōjō.
    ///
    /// `recapture_and_replan` takes the forge token as an argument precisely so
    /// these can run: with the Keychain read inline, the SUCCESS path was
    /// reachable only on a machine with a real sign-in.
    mod self_heal {
        use super::*;

        fn denied(reason: &str) -> DeniedRepo {
            DeniedRepo {
                repo_key: "github.com/acme/api".into(),
                tenant: "organization/acme".into(),
                reason: reason.into(),
            }
        }
        fn allowed(repo_key: &str) -> MappedRepo {
            MappedRepo {
                repo_key: repo_key.into(),
                tenant: "organization/acme".into(),
                tenant_id: "11111111-1111-1111-1111-111111111111".into(),
                repo_id: "22222222-2222-2222-2222-222222222222".into(),
                // Nothing switched off: the default every dōjō starts in.
                disabled_metrics: vec![],
            }
        }

        #[tokio::test]
        async fn capture_flips_a_denial_into_an_allowance() {
            // The whole point: a repo registered after sign-in is denied for want
            // of a forge answer; the daemon asks, and the SECOND plan permits it.
            let plane = StubPlane {
                plan: Some(SyncPlan {
                    allowed: vec![],
                    denied: vec![denied("forge_visibility_unknown")],
                }),
                plan_after_provision: Some(SyncPlan {
                    allowed: vec![allowed("github.com/acme/api")],
                    denied: vec![],
                }),
                ..Default::default()
            };
            let stale =
                SyncPlan { allowed: vec![], denied: vec![denied("forge_visibility_unknown")] };
            let out = recapture_and_replan("p", "tok", "gh-token", &plane, stale).await;
            let plan = out.expect("capture succeeded");
            assert_eq!(plan.allowed.len(), 1, "the refreshed plan must be the one returned");
            assert_eq!(
                *plane.provision_calls.lock().unwrap(),
                1,
                "the forge must actually be asked"
            );
        }

        #[tokio::test]
        async fn a_failed_capture_keeps_the_stale_plan_rather_than_losing_the_pass() {
            // Degrading to the stale plan is the safe direction: it is strictly
            // MORE restrictive, and a repository whose visibility cannot be
            // established is exactly one that must not sync.
            let plane = StubPlane {
                plan: Some(SyncPlan {
                    allowed: vec![],
                    denied: vec![denied("forge_visibility_unknown")],
                }),
                provision_err: Some("github unreachable".into()),
                ..Default::default()
            };
            let stale = SyncPlan {
                allowed: vec![allowed("github.com/acme/already-ok")],
                denied: vec![denied("forge_visibility_unknown")],
            };
            let (err, kept) = recapture_and_replan("p", "tok", "gh-token", &plane, stale)
                .await
                .expect_err("provision failed");
            assert!(err.contains("github unreachable"));
            assert_eq!(kept.allowed.len(), 1, "the pass keeps the verdict it already trusted");
        }

        #[tokio::test]
        async fn a_capture_that_changes_nothing_is_not_an_error() {
            // The forge answered and the repo is still refused — e.g. it really is
            // private and the org has no subscription. That is a legitimate
            // outcome, not a failure, and must not be logged as one.
            let plane = StubPlane {
                plan: Some(SyncPlan {
                    allowed: vec![],
                    denied: vec![denied("forge_visibility_unknown")],
                }),
                plan_after_provision: Some(SyncPlan {
                    allowed: vec![],
                    denied: vec![denied("not_subscribed")],
                }),
                ..Default::default()
            };
            let stale =
                SyncPlan { allowed: vec![], denied: vec![denied("forge_visibility_unknown")] };
            let plan = recapture_and_replan("p", "tok", "gh-token", &plane, stale)
                .await
                .expect("a still-denied repo is a valid answer");
            assert_eq!(
                plan.denied[0].reason, "not_subscribed",
                "the NEW reason must replace the old"
            );
        }
    }

    fn mapped(repo_key: &str, tenant_id: &str) -> MappedRepo {
        MappedRepo {
            repo_key: repo_key.to_string(),
            tenant: "organization/ztest".to_string(),
            tenant_id: tenant_id.to_string(),
            repo_id: uuid::Uuid::new_v4().to_string(),
            disabled_metrics: vec![],
        }
    }

    /// A locally-ELECTED repository with one pushable repo-scoped metric row.
    async fn seed(pg: &PgStore) -> (uuid::Uuid, String, String) {
        seed_at(pg, "shared").await
    }

    /// The same fixture at a chosen local `sensei.repositories.visibility`.
    ///
    /// `'private'` is the org-mandate case: the user has elected nothing, so every
    /// step of the cycle has to happen without local intent to authorise it.
    ///
    /// `modified_at` is dated forward deliberately. The offer set is now EVERY
    /// cloned repository, and the shared test database carries thousands of
    /// leftover fixtures — so with the real `REGISTER_LIMIT` these tests would
    /// otherwise assert on whichever rows happened to win the window, not on the
    /// row they seeded. Forward-dating puts the fixture at the head of the
    /// recency order, which is where a just-cloned repository sits in production
    /// anyway.
    async fn seed_at(pg: &PgStore, visibility: &str) -> (uuid::Uuid, String, String) {
        let uniq = uuid::Uuid::new_v4();
        let pid = pg.create_project(&format!("_test:cycle:{uniq}"), None, None).await.unwrap();
        let rid = crate::tasks::test_support::seed_bare_repository(pg, &pid, &uniq).await;
        sqlx_core::query::query(
            "UPDATE sensei.repositories \
                SET visibility = $2::sensei.repo_visibility, \
                    modified_at = now() + interval '10 years' \
              WHERE id = $1",
        )
        .bind(rid)
        .bind(visibility)
        .execute(pg.pool())
        .await
        .unwrap();
        let metric_key = format!("_test:cycle:{uniq}:m");
        // Seeded here rather than reaching into another module's private test
        // helpers — the cycle's tests own their own fixture.
        let mid: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.metrics \
                 (key, name, description, family, type, direction, purpose, how_to_read, \
                  formula, task_name, effective_from) \
             VALUES ($1, $1, 'cycle test', 'quality'::sensei.metric_family, \
                     'ratio'::sensei.metric_type, 'higher_better'::sensei.metric_direction, \
                     'p', 'h', 'f', 'ComputeFtr', current_date) \
             RETURNING id",
        )
        .bind(&metric_key)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let mid = mid.0;
        pg.upsert_project_metric_repo(
            &mid,
            &rid,
            "repo",
            None,
            None,
            chrono::NaiveDate::from_ymd_opt(2026, 8, 27).unwrap(),
            "daily",
            1.0,
            &serde_json::json!({}),
            "measured",
        )
        .await
        .unwrap();
        (pid, format!("test/bare-{uniq}"), metric_key)
    }

    async fn shared_count(pg: &PgStore, repo_key: &str) -> i64 {
        let n: (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.repository_metrics rm \
               JOIN sensei.repositories r ON r.id = rm.repository_id \
              WHERE r.repo_key = $1 AND rm.shared_at IS NOT NULL",
        )
        .bind(repo_key)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        n.0
    }

    async fn push_state(pg: &PgStore, repo_key: &str) -> Option<(String, Option<String>)> {
        sqlx_core::query_as::query_as(
            "SELECT state, last_error FROM sensei.sync_state \
              WHERE entity = 'repository_metric' AND entity_key = $1 AND direction = 'push'",
        )
        .bind(repo_key)
        .fetch_optional(pg.pool())
        .await
        .unwrap()
    }

    async fn plan_state(pg: &PgStore, slot: &str) -> Option<(String, Option<String>)> {
        sqlx_core::query_as::query_as(
            "SELECT state, last_error FROM sensei.sync_state \
              WHERE entity = 'dojo_sync_plan' AND entity_key = $1 AND direction = 'pull'",
        )
        .bind(slot)
        .fetch_optional(pg.pool())
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_failed_registration_is_recorded_rather_than_leaving_the_plan_reading_synced() {
        // Registration is the cycle's FIRST network call, and its failure used to
        // be a bare `?`. So a dōjō that was down aborted the persona BEFORE the
        // plan's `mark_sync_error` could run: the row from the last good pass
        // stayed `synced` and merely went stale. That is precisely the failure the
        // plan row's own comment says it exists to prevent, applied to the plan
        // and not to the call before it.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, _key, _m) = seed(&pg).await;
        let slot = format!("ztest-reg-{}", uuid::Uuid::new_v4());
        let plane = StubPlane {
            register_err: Some(
                "could not reach dōjō for the registration: connection refused".into(),
            ),
            ..Default::default()
        };

        let out = sync_persona(&pg, &slot, "tok", &plane).await;
        let state = plan_state(&pg, &slot).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;
        let _ = sqlx_core::query::query("DELETE FROM sensei.sync_state WHERE entity_key = $1")
            .bind(&slot)
            .execute(pg.pool())
            .await;

        assert!(out.is_err(), "the persona's cycle reports the failure");
        let (st, err) = state.expect("a failed registration is RECORDED, not just returned");
        assert_eq!(st, "error");
        assert!(
            err.unwrap_or_default().contains("connection refused"),
            "the transport's own reason is preserved"
        );
    }

    #[tokio::test]
    async fn a_dojo_that_refuses_the_session_says_sign_in_again_rather_than_reading_as_a_5xx() {
        // `is_rejection` was wired ONLY to the refresh leg. A 401/403 from
        // `/v1/you/*` came back as a raw string and was recorded exactly like a
        // 500 — so a revoked principal, a tightened RLS policy or a rotated
        // service-role key produced an unbounded 60s retry of a request that can
        // never succeed, while `GET /api/auth/status` kept answering signedIn.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, _key, _m) = seed(&pg).await;
        let slot = format!("ztest-401-{}", uuid::Uuid::new_v4());
        let plane = StubPlane {
            register: Some(RegisterResult { mapped: vec![], unmapped: vec![] }),
            plan_err: Some("dōjō returned 403 for the sync plan: row-level security".into()),
            ..Default::default()
        };

        let out = sync_persona(&pg, &slot, "tok", &plane).await;
        let state = plan_state(&pg, &slot).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;
        let _ = sqlx_core::query::query("DELETE FROM sensei.sync_state WHERE entity_key = $1")
            .bind(&slot)
            .execute(pg.pool())
            .await;

        let e = out.unwrap_err();
        assert!(e.contains("sign in again"), "a refusal must name its remedy, got: {e}");
        let (_st, err) = state.expect("recorded");
        assert!(
            err.unwrap_or_default().contains("sign in again"),
            "and the recorded row must carry it too — that row is the durable evidence"
        );
    }

    #[tokio::test]
    async fn a_server_fault_is_not_dressed_up_as_a_credential_problem() {
        // The other half, and the one that matters more: telling a user to sign
        // in again over a 500 sends them to re-authenticate for an outage they
        // did not cause. Only 401/403 are the server's verdict on the credential.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, _key, _m) = seed(&pg).await;
        let slot = format!("ztest-500-{}", uuid::Uuid::new_v4());
        let plane = StubPlane {
            plan_err: Some("dōjō returned 500 for the sync plan: boom".into()),
            ..Default::default()
        };

        let out = sync_persona(&pg, &slot, "tok", &plane).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;
        let _ = sqlx_core::query::query("DELETE FROM sensei.sync_state WHERE entity_key = $1")
            .bind(&slot)
            .execute(pg.pool())
            .await;

        let e = out.unwrap_err();
        assert!(!e.contains("sign in again"), "a 5xx is not a rejection, got: {e}");
        assert!(e.contains("boom"), "and the server's own reason survives");
    }

    #[tokio::test]
    async fn a_failed_push_is_recorded_and_marks_nothing_shared() {
        // Live bug #2. Deleting the `mark_sync_error` loop leaves the plan row
        // `synced` and the schedule `ok` while nothing ever arrives — the failure
        // is invisible.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed(&pg).await;
        let plane = StubPlane {
            plan: Some(SyncPlan {
                allowed: vec![mapped(&key, &uuid::Uuid::new_v4().to_string())],
                denied: vec![],
            }),
            push: Some(Err("dōjō returned 500 for the metric push: boom".into())),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let state = push_state(&pg, &key).await;
        let shared = shared_count(&pg, &key).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_err(), "the persona's cycle reports the failure");
        let (st, err) = state.expect("the push failure is RECORDED, not just logged");
        assert_eq!(st, "error");
        assert!(err.unwrap_or_default().contains("boom"), "the dōjō's reason is preserved");
        assert_eq!(shared, 0, "a failed push must not mark anything shared");
    }

    #[tokio::test]
    async fn a_dojo_that_refuses_the_push_names_its_remedy_too() {
        // The THIRD failure site. Registration and the plan were routed through
        // `record_plane_failure`, which classifies a 401/403 as "sign in again";
        // the push kept its own hand-rolled loop and did not. So the identical
        // refusal — a revoked principal, a tightened RLS policy, a rotated
        // service-role key — read as a remedy on two screens and as an
        // unexplained fault on the third, which is the only one the user has been
        // watching, because it is the one that carries their data.
        //
        // The push is also the site where it matters most: it is the LAST call in
        // the cycle, so a refusal here is the one that arrives after everything
        // else reported success.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed(&pg).await;
        let plane = StubPlane {
            plan: Some(SyncPlan {
                allowed: vec![mapped(&key, &uuid::Uuid::new_v4().to_string())],
                denied: vec![],
            }),
            push: Some(Err("dōjō returned 403 for the metric push: row-level security".into())),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let state = push_state(&pg, &key).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        let e = out.unwrap_err();
        assert!(e.contains("sign in again"), "a refusal must name its remedy, got: {e}");
        let (st, err) = state.expect("recorded");
        assert_eq!(st, "error");
        assert!(
            err.unwrap_or_default().contains("sign in again"),
            "and the RECORDED row must carry it — that row is the durable evidence, \
             and the returned string is gone as soon as the tick ends"
        );
    }

    #[tokio::test]
    async fn a_partial_acceptance_marks_only_what_was_not_refused() {
        // The livelock: gating the watermark on an EMPTY `rejected` meant one
        // permanently-refused row kept the whole window queued forever. `rejected[]`
        // names the rows, so the complement is markable.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, metric_key) = seed(&pg).await;
        let plane = StubPlane {
            plan: Some(SyncPlan {
                allowed: vec![mapped(&key, &uuid::Uuid::new_v4().to_string())],
                denied: vec![],
            }),
            push: Some(Ok(IngestResult {
                accepted: 0,
                rejected: vec![RejectedMetric {
                    repo_key: key.clone(),
                    metric: metric_key.clone(),
                    reason: "unknown_metric".into(),
                }],
            })),
            ..Default::default()
        };

        let _ = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let shared = shared_count(&pg, &key).await;
        let state = push_state(&pg, &key).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert_eq!(shared, 0, "the REFUSED row stays queued so it can be retried");
        let (st, _) = state.expect("a refusal is recorded");
        assert_eq!(st, "skipped", "the dōjō answered — a refusal is a decision, not a fault");
    }

    #[tokio::test]
    async fn a_refusal_is_recorded_against_the_repository_it_belongs_to() {
        // The batch is per PERSONA but a refusal is per ROW. Every repository in
        // the batch used to be marked `skipped` with a `why` assembled from the
        // first five rejections in the whole response — so a repository whose
        // rows all landed read `skipped: <some other repo's metric>:
        // unknown_metric`, which is a denial naming the wrong cause on a screen
        // whose entire job is to name causes.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid_a, key_a, metric_a) = seed(&pg).await;
        let (pid_b, key_b, _metric_b) = seed(&pg).await;
        let plane = StubPlane {
            plan: Some(SyncPlan {
                allowed: vec![
                    mapped(&key_a, &uuid::Uuid::new_v4().to_string()),
                    mapped(&key_b, &uuid::Uuid::new_v4().to_string()),
                ],
                denied: vec![],
            }),
            // Only repo A is refused. Repo B's rows were accepted.
            push: Some(Ok(IngestResult {
                accepted: 1,
                rejected: vec![RejectedMetric {
                    repo_key: key_a.clone(),
                    metric: metric_a.clone(),
                    reason: "unknown_metric".into(),
                }],
            })),
            ..Default::default()
        };

        let _ = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let a = push_state(&pg, &key_a).await;
        let b = push_state(&pg, &key_b).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid_a, None, &[]).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid_b, None, &[]).await;

        let (state_a, why_a) = a.expect("the refused repository is recorded");
        assert_eq!(state_a, "skipped");
        assert!(
            why_a.unwrap_or_default().contains("unknown_metric"),
            "and it carries its OWN reason"
        );

        let (state_b, why_b) = b.expect("the accepted repository is recorded too");
        assert_eq!(state_b, "synced", "a repository nothing was refused for is not skipped");
        assert_eq!(why_b, None, "and it is not annotated with another repository's refusal");
    }

    #[tokio::test]
    async fn an_empty_allow_list_pushes_nothing_and_does_not_error() {
        // The plan is an allow-list: an empty one means "sync nothing", which is a
        // legitimate answer and must not look like a failure.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed(&pg).await;
        let plane = StubPlane {
            plan: Some(SyncPlan { allowed: vec![], denied: vec![] }),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let calls = *plane.push_calls.lock().unwrap();
        let shared = shared_count(&pg, &key).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_ok(), "nothing allowed is not an error");
        assert_eq!(calls, 0, "and nothing is sent");
        assert_eq!(shared, 0);
    }

    #[tokio::test]
    async fn a_machine_with_nothing_locally_elected_still_asks_the_dojo() {
        // B1, docs/spec/dojo/daemon-sync.md §8a. The cycle used to open with
        // `shared_repositories(); if shared.is_empty() { return Ok(()) }`, so a new
        // employee whose only repository is the org-mandated private one returned
        // BEFORE register, BEFORE the plan, before any push — the mandate
        // structurally unreachable for exactly the population it serves.
        //
        // The daemon cannot pre-filter on "is this mandated": it holds no local fact
        // that answers it. Only the plan does. So it has to ASK first and filter
        // after, and this test is the one that says asking happened.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed_at(&pg, "private").await;
        let plane = StubPlane {
            plan: Some(SyncPlan { allowed: vec![], denied: vec![] }),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let offered = plane.offered.lock().unwrap().clone();
        let plans = *plane.plan_calls.lock().unwrap();
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_ok(), "{out:?}");
        assert!(
            offered.contains(&key),
            "the unelected repository must be DISCLOSED — the dōjō is the only party \
             that can know it is org-mandated; got {} keys, none of them {key}",
            offered.len()
        );
        assert_eq!(plans, 1, "and the plan must be fetched, not short-circuited past");
    }

    #[tokio::test]
    async fn an_unparseable_tenant_id_is_skipped_without_failing_the_cycle() {
        // The dōjō sending a non-uuid tenant id is a bug on one side or the other.
        // It must not be written as a placeholder, and must not take the cycle down.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed(&pg).await;
        let mut bad = mapped(&key, "not-a-uuid");
        bad.tenant_id = "not-a-uuid".into();
        let plane = StubPlane {
            register: Some(RegisterResult { mapped: vec![bad], unmapped: vec![] }),
            plan: Some(SyncPlan {
                allowed: vec![mapped(&key, &uuid::Uuid::new_v4().to_string())],
                denied: vec![],
            }),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let stored: (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT tenant_id FROM sensei.repositories WHERE repo_key = $1",
        )
        .bind(&key)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_ok(), "one bad tenant id does not fail the cycle");
        assert_eq!(stored.0, None, "and nothing plausible-looking is written in its place");
    }

    #[tokio::test]
    async fn a_successful_push_marks_the_rows_and_records_synced() {
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed(&pg).await;
        let plane = StubPlane {
            plan: Some(SyncPlan {
                allowed: vec![mapped(&key, &uuid::Uuid::new_v4().to_string())],
                denied: vec![],
            }),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let sent = plane.pushed_rows.lock().unwrap().clone();
        let shared = shared_count(&pg, &key).await;
        let state = push_state(&pg, &key).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_ok());
        assert_eq!(sent.len(), 1, "the allowed repo's row is sent");
        assert_eq!(shared, 1, "and marked shared so the next cycle does not re-send it");
        assert_eq!(state.expect("recorded").0, "synced");
    }

    #[tokio::test]
    async fn a_denied_repository_is_never_pushed() {
        // Entitlement: the daemon syncs the set it was HANDED. A repo in `denied`
        // must not reach the wire even though it passed gate 1 locally.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed(&pg).await;
        let plane = StubPlane {
            plan: Some(SyncPlan {
                allowed: vec![],
                denied: vec![DeniedRepo {
                    repo_key: key.clone(),
                    tenant: "organization/ztest".into(),
                    reason: "no_seat".into(),
                }],
            }),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let sent = plane.pushed_rows.lock().unwrap().clone();
        let shared = shared_count(&pg, &key).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_ok());
        assert!(sent.is_empty(), "a denied repository's rows never reach the wire");
        assert_eq!(shared, 0);
    }

    #[tokio::test]
    async fn a_mandated_repository_is_pushed_although_the_user_elected_nothing() {
        // B2, docs/spec/dojo/daemon-sync.md §8a and scenario F of §8b. The mandate:
        // an organization's own PRIVATE code, on the organization's subscription,
        // under the organization's governance obligation. The user's local
        // `visibility` is `private` and — for this one class of repository —
        // irrelevant. Fixing the offer set (B1) does nothing here: the push query
        // carried gate 1 in SQL of its own, so a correctly registered, correctly
        // PLANNED repository still had every metric row excluded at push time.
        //
        // The allow-list IS the decision. It comes from `dojo.all_my_repositories`,
        // which is `may_share AND elected` — so re-testing the local flag here would
        // be a second derivation of a question the dōjō already answered.
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed_at(&pg, "private").await;
        let plane = StubPlane {
            plan: Some(SyncPlan {
                allowed: vec![mapped(&key, &uuid::Uuid::new_v4().to_string())],
                denied: vec![],
            }),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let sent = plane.pushed_rows.lock().unwrap().clone();
        let shared = shared_count(&pg, &key).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_ok(), "{out:?}");
        assert_eq!(
            sent.iter().filter(|(r, _)| r == &key).count(),
            1,
            "a repository the PLAN allows is pushed even though the user elected \
             nothing locally — that is what the org mandate means"
        );
        assert_eq!(shared, 1, "and its row is watermarked so the next cycle does not re-send it");
    }

    #[tokio::test]
    async fn a_repository_that_is_neither_elected_nor_allowed_is_never_pushed() {
        // The other half of B2, and the reason dropping `visibility = 'shared'` from
        // the push query is safe: the allow-list still bounds it absolutely. Two
        // repositories in ONE cycle, so this proves a per-repository filter rather
        // than "the pass happened to send nothing".
        let pg = PgStore::connect_test().await.unwrap();
        let (allowed_pid, allowed_key, _am) = seed_at(&pg, "shared").await;
        let (excluded_pid, excluded_key, _em) = seed_at(&pg, "private").await;
        let plane = StubPlane {
            plan: Some(SyncPlan {
                allowed: vec![mapped(&allowed_key, &uuid::Uuid::new_v4().to_string())],
                denied: vec![],
            }),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let sent = plane.pushed_rows.lock().unwrap().clone();
        let offered = plane.offered.lock().unwrap().clone();
        let excluded_shared = shared_count(&pg, &excluded_key).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &allowed_pid, None, &[]).await;
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &excluded_pid, None, &[]).await;

        assert!(out.is_ok(), "{out:?}");
        assert!(
            offered.contains(&excluded_key),
            "it was still OFFERED — disclosure is the clone, the decision is the dōjō's"
        );
        assert!(
            sent.iter().any(|(r, _)| r == &allowed_key),
            "the allowed repository's row is pushed"
        );
        assert!(
            !sent.iter().any(|(r, _)| r == &excluded_key),
            "a repository the plan did not allow must not reach the wire, whatever its \
             local visibility says"
        );
        assert_eq!(excluded_shared, 0, "and nothing about it is marked shared");
    }

    #[tokio::test]
    async fn an_unmapped_repository_is_reported_and_not_pushed() {
        let pg = PgStore::connect_test().await.unwrap();
        let (pid, key, _m) = seed(&pg).await;
        let plane = StubPlane {
            register: Some(RegisterResult {
                mapped: vec![],
                unmapped: vec![UnmappedRepo {
                    repo_key: key.clone(),
                    reason: "unknown_host".into(),
                }],
            }),
            plan: Some(SyncPlan { allowed: vec![], denied: vec![] }),
            ..Default::default()
        };

        let out = sync_persona(&pg, "ztest-slot", "tok", &plane).await;
        let sent = plane.pushed_rows.lock().unwrap().clone();
        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, None, &[]).await;

        assert!(out.is_ok(), "an unmapped repository is a reportable outcome, not a failure");
        assert!(sent.is_empty());
    }

    #[tokio::test]
    async fn a_pass_with_no_signed_in_personas_is_a_no_op_not_an_error() {
        // The state of a fresh install. It must not log an error every cadence,
        // or the daemon cries wolf until the user signs in.
        //
        // THIS TEST DOES NOT OWN THE DATABASE. `tick` reads `sensei.personas`
        // directly, `sensei_test` is shared, and cargo runs tests CONCURRENTLY —
        // so another test holding a seeded persona mid-run is a legitimate state
        // this one cannot exclude. It used to assert `is_ok()` flatly and passed
        // only because nothing else had ever created a persona; two separate
        // additions have since broken it, each time for a reason unrelated to
        // dojo_sync.
        //
        // So it asserts the pair of outcomes that are BOTH correct: no personas
        // is a no-op, and personas that all fail is a reported failure (`tick`
        // returns Err only when EVERY persona failed — see its comment; that
        // property exists so a green worker cannot print over a cycle that moved
        // nothing, and must not be weakened to make a test convenient).
        //
        // What is still pinned: `tick` must never PANIC, and must never return an
        // error for an EMPTY persona set.
        let pg = Arc::new(PgStore::connect_test().await.unwrap());
        let personas = pg.signed_in_personas().await.expect("persona read works");
        let out = tick(pg).await;
        if personas.is_empty() {
            assert!(out.is_ok(), "an empty install must be a no-op, got {out:?}");
        } else {
            assert!(
                out.is_ok() || out.as_ref().is_err_and(|e| e.contains("personas failed")),
                "with personas present the only correct outcomes are success or a \
                 reported all-failed, got {out:?}"
            );
        }
    }

    #[tokio::test]
    async fn the_plan_entity_exists_in_the_database_enum() {
        // Was `assert_eq!(PLAN_ENTITY, "dojo_sync_plan")` — a const compared to its
        // own literal, which passed even with the enum value deleted. A typo or a
        // dropped label fails `mark_sync_error`'s cast at INSERT time inside an
        // unattended worker, where the only symptom is a warning nobody reads.
        let pg = PgStore::connect_test().await.unwrap();
        let labels: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT e.enumlabel::text FROM pg_enum e \
               JOIN pg_type t ON t.oid = e.enumtypid WHERE t.typname = 'sync_entity'",
        )
        .fetch_all(pg.pool())
        .await
        .unwrap();
        let labels: Vec<String> = labels.into_iter().map(|(l,)| l).collect();
        assert!(
            labels.iter().any(|l| l == PLAN_ENTITY),
            "{PLAN_ENTITY} is not a sensei.sync_entity value; the enum holds {labels:?}"
        );
    }
}
