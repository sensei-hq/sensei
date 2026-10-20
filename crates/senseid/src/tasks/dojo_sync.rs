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
//! # User-scoped rows
//!
//! **A user-scoped row leaves only when it is provably the signed-in person's**
//! (#240): a row with no identity — this machine's own session metric — when the
//! persona is the only signed-in one that elected the repository, or a row whose
//! author address the persona CLAIMED (verified by GitHub at sign-in). An
//! unverified `git` address proves nothing and a colleague's never travels; both
//! are held back, counted, and logged. The dōjō stamps the person from the
//! session, never from the row.
//!
//! # Not here yet
//!
//! Per-repo governance pull (D3).

use std::sync::Arc;

use crate::api::handlers::auth::{AuthError, live_access_token};
use crate::db::pg_store::PgStore;
use crate::db::pg_store::sync::SyncMark;
use crate::dojo::memberships::NewConnection;
use crate::dojo_client::dojo_auth;
use crate::dojo_client::settings::dojo_url;
use crate::dojo_client::user_plane::{self, HttpUserPlane, MyMembership, RepoInput, UserPlane};

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

    // Plan EVERY persona before pushing any: a row with no identity is the
    // machine's own session metric, and it is a persona's only when no other
    // signed-in persona elected the same repository (#240).
    let mut planned = Vec::new();
    for persona in personas {
        let outcome = match persona_token(&persona).await {
            Ok(token) => {
                plan_persona(&pg, &persona, &token, &plane).await.map(|p| p.map(|p| (token, p)))
            }
            Err(e) => Err(e),
        };
        match outcome {
            Ok(Some((token, p))) => planned.push((persona, token, p)),
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(persona, error = %e, "dojo_sync: persona skipped");
                failures.push(format!("{persona}: {e}"));
            }
        }
    }
    let plans: Vec<(String, user_plane::SyncPlan)> =
        planned.iter().map(|(who, _, p)| (who.clone(), p.plan.clone())).collect();
    let mut exclusive = exclusive_repos(&plans);
    for (persona, token, p) in &planned {
        let mine = exclusive.remove(persona).unwrap_or_default();
        if let Err(e) = push_planned(&pg, persona, token, &plane, p, &mine).await {
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
    if failures.len() >= total {
        return Err(format!("all {total} personas failed: {}", failures.join("; ")));
    }
    Ok(())
}

/// The persona's live access token.
///
/// Separate from the cycle so the cycle is reachable in a test: token resolution
/// needs the real Keychain, and everything after it does not.
async fn persona_token(persona: &str) -> Result<String, String> {
    live_access_token(persona).await.map_err(token_failure)
}

/// Why a persona has no usable token, as the pass reports it. Distinguished
/// rather than collapsed: "sign in again" is worth saying out loud, a network
/// blip is not worth saying at all.
fn token_failure(e: AuthError) -> String {
    let needs_sign_in = e.needs_sign_in();
    match e {
        AuthError::SignedOut => "no stored session".to_string(),
        AuthError::Rejected(d) => format!("session rejected, sign in again: {d}"),
        AuthError::Unreachable(d) if !needs_sign_in => {
            format!("dōjō unreachable, session kept: {d}")
        }
        AuthError::Unreachable(d) => d,
        AuthError::CredentialUnreadable(d) => format!("stored session unreadable: {d}"),
    }
}

/// Join every dōjō membership the persona holding `slot` belongs to (#236 step
/// 4e): signing in is joining. A listed membership this machine lacks becomes a
/// local one — held to the same rules a connection always was
/// ([`NewConnection::validated`]) — at `registry_url`, the dōjō that issued the
/// session; an unbound local one is bound. Returns how many it joined or bound;
/// a list it could not read is an `Err`, never "you belong to nothing".
pub(crate) async fn join_memberships(
    pg: &PgStore,
    slot: &str,
    token: &str,
    plane: &dyn UserPlane,
    registry_url: &str,
) -> Result<usize, String> {
    let persona = pg
        .persona_id_for_slot(slot)
        .await?
        .ok_or_else(|| format!("no persona holds session slot {slot}"))?;
    let mut joined = 0;
    for m in &plane.my_memberships(token).await? {
        match pg.get_dojo_membership(&m.id).await? {
            Some(local) if local.persona.is_some() => continue,
            Some(_) => {}
            None => {
                crate::dojo::memberships::register(pg, connection_of(m, registry_url)?).await?;
            }
        }
        pg.set_membership_persona(&m.id, &persona).await?;
        joined += 1;
    }
    if joined > 0 {
        tracing::info!(persona = slot, joined, "dojo_sync: memberships joined");
    }
    Ok(joined)
}

/// A listed membership as the local connection it becomes. Pure.
fn connection_of(m: &MyMembership, registry_url: &str) -> Result<NewConnection, String> {
    NewConnection::validated(
        m.id,
        registry_url.to_string(),
        m.tenant_key.clone(),
        crate::dojo::memberships::derive_dojo_url(registry_url, &m.tenant_key),
        &m.kind,
        &m.org_slugs,
        &m.role,
        &m.authenticated_via,
        &m.attribution_default,
        // sync_status has no producer (see dojo_memberships.sync_status).
        "authenticating",
    )
    .map_err(|e| format!("membership {} cannot be joined: {e}", m.id))
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
async fn plan_persona(
    pg: &PgStore,
    persona: &str,
    token: &str,
    plane: &dyn UserPlane,
) -> Result<Option<Planned>, String> {
    // #236 step 4e: join every dōjō this persona belongs to, so its tenant
    // calls present this session. A failure here leaves the memberships for the
    // next pass and does not stop the cycle — but it is said, never swallowed.
    if let Err(e) = join_memberships(pg, persona, token, plane, &dojo_url()).await {
        tracing::warn!(persona, error = %e, "dojo_sync: joining memberships skipped");
    }

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
        return Ok(None);
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

    Ok(Some(Planned { plan, mapped: registered.mapped.len(), unmapped: registered.unmapped.len() }))
}

/// What one persona's pass decided before anything was pushed.
struct Planned {
    plan: user_plane::SyncPlan,
    mapped: usize,
    unmapped: usize,
}

/// Push a persona's planned rows. `exclusive` — the repositories only this
/// persona elected — is where its rows with no identity may go (#240).
async fn push_planned(
    pg: &PgStore,
    persona: &str,
    token: &str,
    plane: &dyn UserPlane,
    planned: &Planned,
    exclusive: &[String],
) -> Result<(), String> {
    let pushed = push_allowed(pg, persona, token, plane, &planned.plan, exclusive).await?;
    // After the metrics: a graph push is larger and slower, and must not hold
    // the metrics back. Each checkout records its own outcome (#225).
    let graph_batches = push_graphs(pg, persona, token, plane, &planned.plan).await?;
    tracing::info!(
        persona,
        allowed = planned.plan.allowed.len(),
        denied = planned.plan.denied.len(),
        mapped = planned.mapped,
        unmapped = planned.unmapped,
        pushed,
        graph_batches,
        "dojo_sync: cycle complete"
    );
    Ok(())
}

/// The `sensei.sync_entity` a graph push records against, keyed `<repo_key>@<branch>`.
const GRAPH_ENTITY: &str = "repository_graph";

/// Bytes of slices per graph request: under the route's 4 MB cap, leaving room
/// for the envelope (repo key, commit) and the last batch's manifest.
const GRAPH_BATCH_BYTES: usize = 3 * 1024 * 1024;

/// Where a checkout's graph push leaves its sync state (#225).
#[derive(Debug, PartialEq, Eq)]
enum GraphOutcome {
    /// Every file's slice landed.
    Synced,
    /// Everything landed but the named files, each over the request cap alone;
    /// the shared graph lacks exactly those.
    Partial(String),
}

fn graph_outcome(oversize: &[String]) -> GraphOutcome {
    if oversize.is_empty() {
        return GraphOutcome::Synced;
    }
    GraphOutcome::Partial(format!(
        "{} file(s) over the {GRAPH_BATCH_BYTES}-byte request cap, not shared: {}",
        oversize.len(),
        oversize.join(", ")
    ))
}

/// Files read from the database per slice query, so one repository's changes are
/// never held in memory all at once.
const GRAPH_SLICE_CHUNK: usize = 200;

/// What became of one checkout.
enum CheckoutPush {
    /// Batches sent; files whose single slice was over the cap, if any.
    Sent {
        batches: usize,
        oversize: Vec<String>,
    },
    Skipped(&'static str),
}

/// Push the code graph of every tracked branch this persona has a checkout of
/// (#225; graph-sync.md §5, §9). Returns the number of batches sent.
///
/// Each (repository, branch) records its outcome in `sensei.sync_state` under
/// `repository_graph`, keyed `<repo_key>@<branch>`: synced, skipped with why
/// (`not_a_tracked_branch`, `index_not_current`), or the error. One checkout's
/// failure does not stop the others.
async fn push_graphs(
    pg: &PgStore,
    persona: &str,
    token: &str,
    plane: &dyn UserPlane,
    plan: &user_plane::SyncPlan,
) -> Result<usize, String> {
    let mut sent = 0;
    for repo in plan.allowed.iter().filter(|r| !r.graph_branches.is_empty()) {
        let tracked: std::collections::HashSet<&str> =
            repo.graph_branches.iter().map(|b| b.branch.as_str()).collect();
        // Two checkouts of one branch (two clones of main): the first one pushed
        // speaks for the branch this cycle.
        let mut pushed_branches = std::collections::HashSet::new();
        for checkout in pg.graph_checkouts(&repo.repo_key).await? {
            let Some(branch) = checkout.branch.clone() else { continue };
            let key = format!("{}@{}", repo.repo_key, branch);
            let mark = SyncMark { entity: GRAPH_ENTITY, key: &key, direction: "push" };
            if !tracked.contains(branch.as_str()) {
                pg.mark_sync_skipped(&mark, "not_a_tracked_branch").await?;
                continue;
            }
            if pushed_branches.contains(&branch) {
                continue;
            }
            match push_checkout(pg, token, plane, &repo.repo_key, &branch, &checkout).await {
                Ok(CheckoutPush::Sent { batches, oversize }) => {
                    sent += batches;
                    pushed_branches.insert(branch.clone());
                    match graph_outcome(&oversize) {
                        GraphOutcome::Synced => pg.mark_synced(&mark, None).await?,
                        GraphOutcome::Partial(missing) => {
                            tracing::warn!(persona, repo = %repo.repo_key, branch, %missing, "dojo_sync: graph incomplete");
                            pg.mark_sync_partial(&mark, &missing).await?;
                        }
                    }
                }
                Ok(CheckoutPush::Skipped(reason)) => pg.mark_sync_skipped(&mark, reason).await?,
                Err(e) => {
                    tracing::warn!(persona, repo = %repo.repo_key, branch, error = %e, "dojo_sync: graph push failed");
                    pg.mark_sync_error(&mark, &e).await?;
                }
            }
        }
    }
    Ok(sent)
}

/// One checkout's push: committed changes in batches, then the unplaced slice
/// and the manifest. See `graph_push` for what is held back and why.
async fn push_checkout(
    pg: &PgStore,
    token: &str,
    plane: &dyn UserPlane,
    repo_key: &str,
    branch: &str,
    checkout: &crate::db::pg_store::GraphCheckout,
) -> Result<CheckoutPush, String> {
    use crate::tasks::graph_push::{HEAD_FORMAT, batch, parse_head, parse_porcelain_z, plan_push};
    let dir = std::path::Path::new(&checkout.abs_path);
    let git = |args: &[&str]| {
        crate::git::run_text(dir, args)
            .map_err(|e| format!("git {args:?} in {}: {e}", checkout.abs_path))
    };

    // The graph is the last index of this folder; if the working tree has since
    // moved to another branch, that index is not this branch's state any more.
    if git(&["rev-parse", "--abbrev-ref", "HEAD"])?.trim() != branch {
        return Ok(CheckoutPush::Skipped("index_not_current"));
    }
    let (sha, committed_at) = parse_head(&git(&["log", "-1", HEAD_FORMAT])?)?;
    let dirty = parse_porcelain_z(
        &crate::git::run_bytes(dir, &["status", "--porcelain", "-z", "--untracked-files=all"])
            .map_err(|e| format!("git status in {}: {e}", checkout.abs_path))?,
    );

    // Held back: what is not committed, and what is committed but not yet parsed
    // (its row has the new hash, its nodes are still the old content's).
    let mut held_back = dirty;
    held_back.extend(pg.graph_parse_pending(&checkout.folder_id).await?);

    let local = pg.graph_manifest(&checkout.folder_id).await?;
    let remote = plane.graph_hashes(token, repo_key, branch).await?.files;
    let plan = plan_push(&local, &remote, &held_back);
    let commit = user_plane::GraphCommit { branch: branch.to_string(), sha, committed_at };

    let mut batches = 0;
    let mut oversize = Vec::new();
    for chunk in plan.changed.chunks(GRAPH_SLICE_CHUNK) {
        let grouped = batch(pg.graph_slices(&checkout.folder_id, chunk).await?, GRAPH_BATCH_BYTES);
        oversize.extend(grouped.oversize);
        for slices in &grouped.batches {
            let push = user_plane::GraphPush { repo_key, commit: &commit, slices, manifest: None };
            plane.push_graph(token, &push).await?;
            batches += 1;
        }
    }

    // Last: the unplaced nodes and the manifest that prunes deleted files.
    let unplaced = pg.graph_unplaced_slice(&checkout.folder_id).await?;
    let mut last = batch(vec![("(unplaced)".to_string(), unplaced)], GRAPH_BATCH_BYTES);
    oversize.append(&mut last.oversize);
    let slices = last.batches.pop().unwrap_or_default();
    let push = user_plane::GraphPush {
        repo_key,
        commit: &commit,
        slices: &slices,
        manifest: Some(&plan.manifest),
    };
    plane.push_graph(token, &push).await?;
    Ok(CheckoutPush::Sent { batches: batches + 1, oversize })
}

/// One persona's whole pass on its own — every repository it may sync is its
/// alone. Tests drive this; [`tick`] plans every persona before pushing any.
#[cfg(test)]
async fn sync_persona(
    pg: &PgStore,
    persona: &str,
    token: &str,
    plane: &dyn UserPlane,
) -> Result<(), String> {
    let Some(planned) = plan_persona(pg, persona, token, plane).await? else {
        return Ok(());
    };
    let exclusive = exclusive_repos(&[(persona.to_string(), planned.plan.clone())])
        .remove(persona)
        .unwrap_or_default();
    push_planned(pg, persona, token, plane, &planned, &exclusive).await
}

/// For each persona, the repositories in its plan that no other signed-in
/// persona's plan allows (#240). Pure.
fn exclusive_repos(
    plans: &[(String, user_plane::SyncPlan)],
) -> std::collections::HashMap<String, Vec<String>> {
    let mut electors: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (_, plan) in plans {
        let keys: std::collections::HashSet<&str> =
            plan.allowed.iter().map(|a| a.repo_key.as_str()).collect();
        for k in keys {
            *electors.entry(k).or_default() += 1;
        }
    }
    plans
        .iter()
        .map(|(who, plan)| {
            let mine = plan
                .allowed
                .iter()
                .filter(|a| electors.get(a.repo_key.as_str()) == Some(&1))
                .map(|a| a.repo_key.clone())
                .collect();
            (who.clone(), mine)
        })
        .collect()
}

/// Hold back user-scoped rows that would land on ONE dōjō row. The dōjō keys a
/// user row by person, not author address, so two of the person's verified
/// addresses with a row for the same metric and day would overwrite each other
/// silently; both are held back instead. Returns what to push and how many were
/// held. Order preserved. Pure.
fn drop_colliding(
    rows: Vec<crate::db::pg_store::sync::PushableMetric>,
) -> (Vec<crate::db::pg_store::sync::PushableMetric>, usize) {
    type Key = (String, String, Option<String>, chrono::NaiveDate, String);
    let key = |m: &crate::db::pg_store::sync::PushableMetric| -> Key {
        (m.repo_key.clone(), m.metric.clone(), m.commit_sha.clone(), m.computed_on, m.grain.clone())
    };
    let mut seen: std::collections::HashMap<Key, usize> = std::collections::HashMap::new();
    for m in rows.iter().filter(|m| m.scope == "user") {
        *seen.entry(key(m)).or_default() += 1;
    }
    let (kept, held): (Vec<_>, Vec<_>) =
        rows.into_iter().partition(|m| m.scope != "user" || seen.get(&key(m)) == Some(&1));
    (kept, held.len())
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
    exclusive: &[String],
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

    // Repo-scoped rows first, then the user-scoped rows PROVABLY this persona's
    // (#240): no identity on a repository only they elected, or an author address
    // they claimed. Everything filtered in SQL so the LIMIT applies to rows that
    // can actually go — held-back rows once crowded the window and a pass pushed
    // 66 of 132. See `unpushed_metric_rows`.
    let keys: Vec<&str> = allowed.iter().copied().collect();
    let exclusive: Vec<&str> = exclusive.iter().map(String::as_str).collect();
    let mut rows = pg.unpushed_metric_rows(&["repo"], &keys, PUSH_LIMIT).await?;
    let room = PUSH_LIMIT - rows.len() as i64;
    let persona_id = pg.persona_id_for_slot(persona).await?;
    let mut unprovable = 0;
    if let Some(pid) = persona_id {
        if room > 0 {
            rows.extend(pg.unpushed_user_metric_rows(&pid, &keys, &exclusive, room).await?);
        }
        // A failure here must not print "0 held back" — that is the exact mislead
        // the counter exists to prevent.
        unprovable = match pg.unprovable_user_metric_count(&pid, &keys, &exclusive).await {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!(persona, error = %e, "could not count held-back rows");
                0
            }
        };
    }
    let (pushable, colliding) = drop_colliding(rows);
    if unprovable > 0 || colliding > 0 {
        tracing::info!(
            persona,
            unprovable,
            colliding,
            "dojo_sync: user-scoped rows held back — not provably this person's, or two of their addresses on one day"
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

    /// #283: a Keychain that would not answer is said as itself — the dōjō
    /// was never asked, so "dōjō unreachable" would send the user the wrong way.
    #[test]
    fn an_unreadable_credential_is_not_reported_as_the_dojo() {
        let why =
            token_failure(AuthError::CredentialUnreadable("keychain access needs approval".into()));
        assert!(why.contains("keychain access needs approval"), "{why}");
        assert!(!why.contains("dōjō unreachable"), "{why}");
    }

    // ── graph_outcome: a checkout push's sync state (#225) ──────────────────

    #[test]
    fn a_whole_push_is_synced() {
        assert_eq!(graph_outcome(&[]), GraphOutcome::Synced);
    }

    #[test]
    fn a_push_missing_an_over_cap_file_is_partial_and_names_it() {
        let oversize = vec!["crates/big.rs".to_string()];
        assert_eq!(
            graph_outcome(&oversize),
            GraphOutcome::Partial(
                "1 file(s) over the 3145728-byte request cap, not shared: crates/big.rs".into()
            )
        );
    }
    use crate::db::pg_store::MetricRow;

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
        ActivationOutcome, DeniedRepo, GraphHashes, GraphPush, GraphPushResult, IngestResult,
        MappedRepo, MetricPush, MyMembership, ProvisionOutcome, RegisterResult, RejectedMetric,
        SyncPlan, UnmappedRepo, VisibilityCounts,
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
        /// What `GET /v1/you/memberships` answers — `None` is an empty list.
        memberships: Option<Result<Vec<MyMembership>, String>>,
        membership_calls: Mutex<usize>,
        /// What `GET /v1/you/graph` answers, per (repo_key, branch); absent → {}.
        graph_remote:
            std::collections::HashMap<(String, String), std::collections::HashMap<String, String>>,
        /// Every graph batch: (repo_key, branch, sha, slice paths, manifest).
        graph_pushes: Mutex<Vec<GraphPushSeen>>,
    }

    /// One graph batch as the stub saw it.
    #[derive(Debug, Clone, PartialEq)]
    struct GraphPushSeen {
        repo_key: String,
        branch: String,
        sha: String,
        paths: Vec<Option<String>>,
        manifest: Option<std::collections::BTreeMap<String, String>>,
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
        async fn my_memberships(&self, _t: &str) -> Result<Vec<MyMembership>, String> {
            *self.membership_calls.lock().unwrap() += 1;
            self.memberships.clone().unwrap_or(Ok(vec![]))
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
        async fn graph_hashes(
            &self,
            _t: &str,
            repo_key: &str,
            branch: &str,
        ) -> Result<GraphHashes, String> {
            Ok(GraphHashes {
                files: self
                    .graph_remote
                    .get(&(repo_key.to_string(), branch.to_string()))
                    .cloned()
                    .unwrap_or_default(),
            })
        }
        async fn push_graph(
            &self,
            _t: &str,
            push: &GraphPush<'_>,
        ) -> Result<GraphPushResult, String> {
            let paths: Vec<Option<String>> =
                push.slices.iter().map(|s| s["path"].as_str().map(str::to_string)).collect();
            let files = paths.iter().filter(|p| p.is_some()).count() as u32;
            self.graph_pushes.lock().unwrap().push(GraphPushSeen {
                repo_key: push.repo_key.to_string(),
                branch: push.commit.branch.clone(),
                sha: push.commit.sha.clone(),
                paths,
                manifest: push.manifest.cloned(),
            });
            Ok(GraphPushResult { files, refused_older: 0, pruned: 0 })
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
                graph_branches: vec![],
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
            graph_branches: vec![],
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
        pg.upsert_project_metric_repo(&MetricRow {
            metric_id: &mid,
            repository_id: &rid,
            scope: "repo",
            identity: None,
            commit_sha: None,
            computed_on: chrono::NaiveDate::from_ymd_opt(2026, 8, 27).unwrap(),
            grain: "daily",
            value: 1.0,
            props: &serde_json::json!({}),
            source: "measured",
        })
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

    /// A signed-in persona and two local memberships for the binding tests.
    /// `sensei_test` is shared, so every row is unique to this call.
    async fn binding_fixture(pg: &PgStore) -> (String, uuid::Uuid, uuid::Uuid, uuid::Uuid) {
        let tag = uuid::Uuid::new_v4();
        let slot = format!("bind-slot-{tag}");
        let persona = pg.upsert_persona(&format!("bind-{tag}"), true).await.unwrap();
        sqlx_core::query::query(
            "UPDATE sensei.personas SET session_slot = $2, verified_at = now() WHERE id = $1",
        )
        .bind(persona)
        .bind(&slot)
        .execute(pg.pool())
        .await
        .unwrap();
        let mut ids = vec![];
        for n in 0..2 {
            let id = uuid::Uuid::new_v4();
            pg.create_dojo_membership(&crate::db::pg_store::NewDojoMembership {
                id,
                registry_url: "http://localhost:7755".into(),
                tenant_key: format!("organization/bind-{tag}-{n}"),
                dojo_url: format!("http://localhost:7755/organization/bind-{tag}-{n}"),
                kind: "employer".into(),
                org_slugs: vec![],
                role: "contributor".into(),
                authenticated_via: "github_oauth".into(),
                attribution_default: "named".into(),
                sync_status: "healthy".into(),
            })
            .await
            .unwrap();
            ids.push(id);
        }
        (slot, persona, ids[0], ids[1])
    }

    async fn drop_binding_fixture(pg: &PgStore, persona: uuid::Uuid, ms: &[uuid::Uuid]) {
        for m in ms {
            pg.delete_dojo_membership(m).await.unwrap();
        }
        sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = $1")
            .bind(persona)
            .execute(pg.pool())
            .await
            .unwrap();
    }

    fn listed(id: uuid::Uuid) -> MyMembership {
        MyMembership {
            id,
            tenant_key: "organization/acme".into(),
            role: "contributor".into(),
            kind: "employer".into(),
            org_slugs: vec![],
            authenticated_via: "github_oauth".into(),
            attribution_default: "named".into(),
        }
    }

    // A persona binds exactly the memberships the dōjō lists under it: a local
    // membership it does not list stays unbound.
    #[tokio::test]
    async fn binding_records_the_persona_of_each_listed_membership() {
        let pg = PgStore::connect_test().await.unwrap();
        let (slot, persona, mine, not_listed) = binding_fixture(&pg).await;
        let plane = StubPlane { memberships: Some(Ok(vec![listed(mine)])), ..Default::default() };

        let bound = join_memberships(&pg, &slot, "jwt", &plane, "http://localhost:7755")
            .await
            .expect("binding runs");

        assert_eq!(bound, 1);
        assert_eq!(
            pg.membership_persona(&mine).await.unwrap().map(|p| p.persona_id),
            Some(persona)
        );
        assert_eq!(pg.membership_persona(&not_listed).await.unwrap(), None);
        drop_binding_fixture(&pg, persona, &[mine, not_listed]).await;
    }

    /// A signed-in persona holding `slot`, with no memberships of its own yet.
    async fn signed_in_persona(pg: &PgStore) -> (String, uuid::Uuid) {
        let tag = uuid::Uuid::new_v4();
        let slot = format!("join-slot-{tag}");
        let persona = pg.upsert_persona(&format!("join-{tag}"), true).await.unwrap();
        sqlx_core::query::query(
            "UPDATE sensei.personas SET session_slot = $2, verified_at = now() WHERE id = $1",
        )
        .bind(persona)
        .bind(&slot)
        .execute(pg.pool())
        .await
        .unwrap();
        (slot, persona)
    }

    // #236 step 4e: signing in is joining. Every dōjō membership the persona's
    // session lists becomes a local membership bound to that persona — what it
    // routes on (kind, org slugs), how it credits, and where its tenant lives.
    #[tokio::test]
    async fn signing_in_joins_every_dojo_the_persona_belongs_to() {
        let pg = PgStore::connect_test().await.unwrap();
        let (slot, persona) = signed_in_persona(&pg).await;
        let id = uuid::Uuid::new_v4();
        let listing = MyMembership {
            id,
            tenant_key: format!("organization/join-{id}"),
            role: "maintainer".into(),
            kind: "client".into(),
            // Unique to this run: `sensei_test` is shared, and a slug another test
            // matches on (binding suggestions) must not leak from a failed run.
            org_slugs: vec![format!("join-{id}")],
            authenticated_via: "github_oauth".into(),
            attribution_default: "anonymous".into(),
        };
        let plane =
            StubPlane { memberships: Some(Ok(vec![listing.clone()])), ..Default::default() };

        let joined = join_memberships(&pg, &slot, "jwt", &plane, "https://dojo.example").await;

        assert_eq!(joined, Ok(1));
        let m = pg.get_dojo_membership(&id).await.unwrap().expect("the membership is local");
        assert_eq!(m.tenant_key, listing.tenant_key);
        assert_eq!((m.kind.as_str(), m.role.as_str()), ("client", "maintainer"));
        assert_eq!(m.org_slugs, vec![format!("join-{id}")]);
        assert_eq!(
            (m.authenticated_via.as_str(), m.attribution_default.as_str()),
            ("github_oauth", "anonymous")
        );
        assert_eq!(m.registry_url, "https://dojo.example");
        assert_eq!(m.dojo_url, format!("https://dojo.example/organization/join-{id}"));
        assert_eq!(m.persona.map(|p| p.persona_id), Some(persona));

        // Joining again creates nothing: the membership is already here.
        assert_eq!(
            join_memberships(&pg, &slot, "jwt", &plane, "https://dojo.example").await,
            Ok(0)
        );
        drop_binding_fixture(&pg, persona, &[id]).await;
    }

    fn plan_of(keys: &[&str]) -> SyncPlan {
        SyncPlan { allowed: keys.iter().map(|k| mapped(k, "t")).collect(), denied: vec![] }
    }

    // #240: a row with no identity is the machine's own — the persona's only when
    // no other signed-in persona elected the same repository.
    #[test]
    fn a_repository_is_exclusive_to_the_one_persona_that_elected_it() {
        let plans = vec![
            ("work".to_string(), plan_of(&["a", "shared"])),
            ("home".to_string(), plan_of(&["b", "shared"])),
        ];
        let ex = exclusive_repos(&plans);
        assert_eq!(ex["work"], vec!["a".to_string()]);
        assert_eq!(ex["home"], vec!["b".to_string()]);
        assert_eq!(
            exclusive_repos(&plans[..1])["work"],
            vec!["a".to_string(), "shared".to_string()]
        );
    }

    fn push_row(metric: &str, scope: &str, day: u32) -> crate::db::pg_store::sync::PushableMetric {
        crate::db::pg_store::sync::PushableMetric {
            id: uuid::Uuid::new_v4(),
            repo_key: "r".into(),
            metric: metric.into(),
            scope: scope.into(),
            grain: "daily".into(),
            computed_on: chrono::NaiveDate::from_ymd_opt(2026, 1, day).unwrap(),
            value: 1.0,
            commit_sha: None,
            props: serde_json::json!({}),
            source: "measured".into(),
        }
    }

    // Two of the person's verified addresses with a row for the same metric and
    // day would land on ONE dōjō row (it keys by person, not address) — one would
    // silently overwrite the other. Both are held back, and counted.
    #[test]
    fn user_rows_that_would_collide_on_the_dojo_are_held_back_together() {
        let rows = vec![
            push_row("churn", "user", 1),
            push_row("churn", "user", 1),
            push_row("churn", "user", 2),
            push_row("churn", "repo", 1),
        ];
        let (kept, held) = drop_colliding(rows);
        assert_eq!(held, 2);
        let kept: Vec<_> = kept
            .iter()
            .map(|r| (r.scope.as_str(), chrono::Datelike::day(&r.computed_on)))
            .collect();
        assert_eq!(kept, vec![("user", 2), ("repo", 1)]);
    }

    #[tokio::test]
    async fn a_failed_list_binds_nothing_and_says_so() {
        let pg = PgStore::connect_test().await.unwrap();
        let (slot, persona, m1, m2) = binding_fixture(&pg).await;
        let plane = StubPlane {
            memberships: Some(Err("dōjō returned 503 for the memberships: down".into())),
            ..Default::default()
        };

        let out = join_memberships(&pg, &slot, "jwt", &plane, "http://localhost:7755").await;

        assert!(out.as_ref().is_err_and(|e| e.contains("503")), "got {out:?}");
        assert_eq!(pg.membership_persona(&m1).await.unwrap(), None);
        drop_binding_fixture(&pg, persona, &[m1, m2]).await;
    }

    #[tokio::test]
    async fn a_slot_with_no_persona_is_an_error_not_a_guess() {
        let pg = PgStore::connect_test().await.unwrap();
        let (_slot, persona, m1, m2) = binding_fixture(&pg).await;
        let plane = StubPlane { memberships: Some(Ok(vec![listed(m1)])), ..Default::default() };

        let out =
            join_memberships(&pg, "no-such-slot", "jwt", &plane, "http://localhost:7755").await;

        assert!(out.is_err(), "got {out:?}");
        assert_eq!(pg.membership_persona(&m1).await.unwrap(), None);
        drop_binding_fixture(&pg, persona, &[m1, m2]).await;
    }

    // The cycle itself binds: a persona's pass records its memberships before it
    // syncs anything, and a binding failure does not stop the sync.
    #[tokio::test]
    async fn the_sync_cycle_binds_the_personas_memberships() {
        let pg = PgStore::connect_test().await.unwrap();
        let (slot, persona, mine, other) = binding_fixture(&pg).await;
        let plane = StubPlane { memberships: Some(Ok(vec![listed(mine)])), ..Default::default() };

        let _ = sync_persona(&pg, &slot, "tok", &plane).await;

        assert_eq!(*plane.membership_calls.lock().unwrap(), 1);
        assert_eq!(
            pg.membership_persona(&mine).await.unwrap().map(|p| p.persona_id),
            Some(persona)
        );
        drop_binding_fixture(&pg, persona, &[mine, other]).await;
    }

    #[tokio::test]
    async fn a_binding_failure_does_not_stop_the_sync() {
        let pg = PgStore::connect_test().await.unwrap();
        let (slot, persona, m1, m2) = binding_fixture(&pg).await;
        let plane = StubPlane { memberships: Some(Err("down".into())), ..Default::default() };

        let _ = sync_persona(&pg, &slot, "tok", &plane).await;

        assert_eq!(*plane.plan_calls.lock().unwrap(), 1, "the plan is still read");
        drop_binding_fixture(&pg, persona, &[m1, m2]).await;
    }

    /// #225: a tracked checkout's graph reaches the dōjō — committed work only.
    mod graph_sync {
        use super::*;
        use std::collections::{BTreeMap, HashMap};

        struct Checkout {
            dir: tempfile::TempDir,
            repo_key: String,
            folder: uuid::Uuid,
            repository: uuid::Uuid,
        }

        fn git(dir: &std::path::Path, args: &[&str]) -> String {
            let out = std::process::Command::new("git")
                .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
                .args(args)
                .current_dir(dir)
                .output()
                .expect("git runs");
            assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
            String::from_utf8(out.stdout).unwrap().trim().to_string()
        }

        /// A git checkout on `main` with src/a.ts and src/b.ts committed, then
        /// src/b.ts EDITED and not committed; indexed as such in the database.
        async fn checkout(pg: &PgStore, indexed_branch: &str) -> Checkout {
            let dir = tempfile::tempdir().unwrap();
            let d = dir.path();
            git(d, &["init", "-q", "-b", "main"]);
            std::fs::create_dir_all(d.join("src")).unwrap();
            std::fs::write(d.join("src/a.ts"), "export const a = 1;").unwrap();
            std::fs::write(d.join("src/b.ts"), "export const b = 1;").unwrap();
            git(d, &["add", "."]);
            git(d, &["commit", "-q", "-m", "c1"]);
            std::fs::write(d.join("src/b.ts"), "export const b = 2; // not committed").unwrap();

            crate::tasks::test_support::ensure_test_watch_root(pg).await;
            let u = uuid::Uuid::new_v4();
            let repo_key = format!("test/graph-sync-{u}");
            let (repository,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
                "insert into sensei.repositories(repo_key, name) values($1, $1) returning id",
            )
            .bind(&repo_key)
            .fetch_one(pg.pool())
            .await
            .unwrap();
            let folder = uuid::Uuid::new_v4();
            sqlx_core::query::query(
                "insert into sensei.folders(id, root_id, kind, name, path, abs_path, repository_id, branch) \
                 values($1, '00000000-0000-0000-0000-000000000001', 'git', $2, $2, $3, $4, $5)",
            )
            .bind(folder)
            .bind(format!("gs-{u}"))
            .bind(d.to_string_lossy().to_string())
            .bind(repository)
            .bind(indexed_branch)
            .execute(pg.pool())
            .await
            .unwrap();
            for (path, hash, fqn) in [("src/a.ts", "ha", "a.x"), ("src/b.ts", "hb-dirty", "b.y")] {
                sqlx_core::query::query(
                    "insert into sensei.files(folder_id, file_path, mtime, content_hash, parsed_at) values($1, $2, 0, $3, now())",
                )
                .bind(folder)
                .bind(path)
                .bind(hash)
                .execute(pg.pool())
                .await
                .unwrap();
                sqlx_core::query::query(
                    "insert into sensei.nodes(folder_id, file_id, kind, name, fqn) \
                     values($1, (select id from sensei.files where folder_id = $1 and file_path = $2), 'function', $3, $3)",
                )
                .bind(folder)
                .bind(path)
                .bind(fqn)
                .execute(pg.pool())
                .await
                .unwrap();
            }
            Checkout { dir, repo_key, folder, repository }
        }

        async fn drop_checkout(pg: &PgStore, c: Checkout) {
            sqlx_core::query::query("delete from sensei.folders where id = $1")
                .bind(c.folder)
                .execute(pg.pool())
                .await
                .unwrap();
            sqlx_core::query::query("delete from sensei.sync_state where entity = 'repository_graph' and entity_key like $1")
                .bind(format!("{}@%", c.repo_key))
                .execute(pg.pool())
                .await
                .unwrap();
            sqlx_core::query::query("delete from sensei.repositories where id = $1")
                .bind(c.repository)
                .execute(pg.pool())
                .await
                .unwrap();
        }

        fn plan_tracking(repo_key: &str, branches: &[&str]) -> SyncPlan {
            SyncPlan {
                allowed: vec![MappedRepo {
                    repo_key: repo_key.into(),
                    tenant: "organization/acme".into(),
                    tenant_id: "t".into(),
                    repo_id: "r".into(),
                    disabled_metrics: vec![],
                    graph_branches: branches
                        .iter()
                        .enumerate()
                        .map(|(i, b)| user_plane::GraphBranch {
                            branch: b.to_string(),
                            is_default: i == 0,
                        })
                        .collect(),
                }],
                denied: vec![],
            }
        }

        async fn state(pg: &PgStore, key: &str) -> Option<(String, Option<String>)> {
            sqlx_core::query_as::query_as(
                "select state::text, last_error from sensei.sync_state where entity = 'repository_graph' and entity_key = $1",
            )
            .bind(key)
            .fetch_optional(pg.pool())
            .await
            .unwrap()
        }

        /// A committed file whose parse has not run yet is held back like an edit
        /// (#225). The structure pass records the new content hash before the
        /// parse rebuilds the file's nodes, so a push in between shared the new
        /// hash with the OLD nodes — and the dōjō, holding that hash, never asked
        /// for the file again (live: scan.rs on the dōjō had 41 nodes, 47 locally).
        #[tokio::test]
        async fn a_committed_file_awaiting_its_parse_is_held_back_until_parsed() {
            let pg = PgStore::connect_test().await.unwrap();
            let c = checkout(&pg, "main").await;
            sqlx_core::query::query(
                "update sensei.files set parsed_at = null where folder_id = $1 and file_path = 'src/a.ts'",
            )
            .bind(c.folder)
            .execute(pg.pool())
            .await
            .unwrap();
            let mut plane = StubPlane::default();
            plane.graph_remote.insert(
                (c.repo_key.clone(), "main".into()),
                HashMap::from([
                    ("src/a.ts".to_string(), "ha-before".to_string()),
                    ("src/b.ts".to_string(), "hb-committed".to_string()),
                ]),
            );

            push_graphs(&pg, "p", "tok", &plane, &plan_tracking(&c.repo_key, &["main"]))
                .await
                .unwrap();

            let seen = plane.graph_pushes.lock().unwrap().clone();
            let pushed: Vec<Option<String>> = seen.iter().flat_map(|p| p.paths.clone()).collect();
            assert!(
                !pushed.contains(&Some("src/a.ts".into())),
                "a file awaiting its parse is not shared: {pushed:?}"
            );
            assert_eq!(
                seen.last().and_then(|p| p.manifest.clone()),
                Some(BTreeMap::from([
                    ("src/a.ts".to_string(), "ha-before".to_string()),
                    ("src/b.ts".to_string(), "hb-committed".to_string()),
                ])),
                "it is kept at the dōjō's hash, so the next push after its parse sends it"
            );
            drop_checkout(&pg, c).await;
        }

        #[tokio::test]
        async fn a_tracked_checkout_pushes_its_committed_changes_and_holds_back_its_edits() {
            let pg = PgStore::connect_test().await.unwrap();
            let c = checkout(&pg, "main").await;
            let head = git(c.dir.path(), &["rev-parse", "HEAD"]);
            let mut plane = StubPlane::default();
            // The dōjō holds b.ts at its committed hash; a.ts is new to it.
            plane.graph_remote.insert(
                (c.repo_key.clone(), "main".into()),
                HashMap::from([("src/b.ts".to_string(), "hb-committed".to_string())]),
            );

            let batches =
                push_graphs(&pg, "p", "tok", &plane, &plan_tracking(&c.repo_key, &["main"]))
                    .await
                    .unwrap();

            let seen = plane.graph_pushes.lock().unwrap().clone();
            assert_eq!(batches, seen.len());
            assert!(
                seen.iter()
                    .all(|p| p.repo_key == c.repo_key && p.branch == "main" && p.sha == head)
            );
            let pushed: Vec<Option<String>> = seen.iter().flat_map(|p| p.paths.clone()).collect();
            assert!(
                pushed.contains(&Some("src/a.ts".into())),
                "the committed change goes: {pushed:?}"
            );
            assert!(
                !pushed.contains(&Some("src/b.ts".into())),
                "the uncommitted edit stays local: {pushed:?}"
            );
            // The last batch carries the unplaced slice and the manifest; b.ts is
            // kept at the dōjō's hash, not pruned and not overwritten.
            let last = seen.last().unwrap();
            assert!(last.paths.contains(&None), "the unplaced slice rides the last batch");
            assert_eq!(
                last.manifest,
                Some(BTreeMap::from([
                    ("src/a.ts".to_string(), "ha".to_string()),
                    ("src/b.ts".to_string(), "hb-committed".to_string()),
                ]))
            );
            assert!(
                seen[..seen.len() - 1].iter().all(|p| p.manifest.is_none()),
                "only the last batch prunes"
            );
            assert_eq!(
                state(&pg, &format!("{}@main", c.repo_key)).await.map(|s| s.0),
                Some("synced".into())
            );
            drop_checkout(&pg, c).await;
        }

        #[tokio::test]
        async fn a_checkout_on_a_branch_the_dojo_does_not_track_pushes_nothing_and_says_why() {
            let pg = PgStore::connect_test().await.unwrap();
            let c = checkout(&pg, "main").await;
            let plane = StubPlane::default();
            push_graphs(&pg, "p", "tok", &plane, &plan_tracking(&c.repo_key, &["production"]))
                .await
                .unwrap();
            assert!(plane.graph_pushes.lock().unwrap().is_empty());
            assert_eq!(
                state(&pg, &format!("{}@main", c.repo_key)).await,
                Some(("skipped".into(), Some("not_a_tracked_branch".into())))
            );
            drop_checkout(&pg, c).await;
        }

        #[tokio::test]
        async fn a_checkout_whose_branch_moved_since_indexing_is_not_pushed_as_the_old_one() {
            // Indexed on main, now checked out on feature-x: its graph is main's
            // as of the last index, but its working tree is not — skip, say why.
            let pg = PgStore::connect_test().await.unwrap();
            let c = checkout(&pg, "main").await;
            git(c.dir.path(), &["checkout", "-q", "-b", "feature-x"]);
            let plane = StubPlane::default();
            push_graphs(&pg, "p", "tok", &plane, &plan_tracking(&c.repo_key, &["main"]))
                .await
                .unwrap();
            assert!(plane.graph_pushes.lock().unwrap().is_empty());
            assert_eq!(
                state(&pg, &format!("{}@main", c.repo_key)).await,
                Some(("skipped".into(), Some("index_not_current".into())))
            );
            drop_checkout(&pg, c).await;
        }

        #[tokio::test]
        async fn a_persona_pass_pushes_the_graph_after_the_metrics() {
            // The wiring: the cycle a schedule runs reaches push_graphs.
            let pg = PgStore::connect_test().await.unwrap();
            let c = checkout(&pg, "main").await;
            let plane = StubPlane {
                plan: Some(plan_tracking(&c.repo_key, &["main"])),
                ..Default::default()
            };
            sync_persona(&pg, "p", "tok", &plane).await.unwrap();
            assert!(
                plane.graph_pushes.lock().unwrap().iter().any(|p| p.repo_key == c.repo_key),
                "the pass never pushed the tracked checkout's graph"
            );
            drop_checkout(&pg, c).await;
        }

        #[tokio::test]
        async fn graph_entity_is_a_real_sync_entity() {
            let pg = PgStore::connect_test().await.unwrap();
            let (labels,): (Vec<String>,) = sqlx_core::query_as::query_as(
                "select enum_range(null::sensei.sync_entity)::text[]",
            )
            .fetch_one(pg.pool())
            .await
            .unwrap();
            assert!(labels.iter().any(|l| l == GRAPH_ENTITY), "{GRAPH_ENTITY} not in {labels:?}");
        }
    }
}
