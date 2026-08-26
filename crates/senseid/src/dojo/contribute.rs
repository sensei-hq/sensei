//! Upstream contribute path (C6) — turn an APPROVED memory-share batch into
//! Dōjō artifacts and publish them, with the confidentiality gate as the
//! non-negotiable step and a durable outbox so a federation drop replays without
//! double-sending.
//!
//! The loop (`docs/spec/pipeline/dojo-lifecycle.md` → Contribute):
//!
//! 1. Load the approved batch's member memories.
//! 2. Route via [`crate::dojo::routing::client_precedence_route`] (C4) to the
//!    destination membership(s) + the per-target `dereference` decision.
//! 3. For each (destination × memory) build a [`dojo_protocol::PublishedArtifact`]
//!    whose body is the CONFIDENTIALITY-CHECKED text:
//!    - client route (`dereference`) → [`Dereferenced::new`] (C5); residual risk
//!      → the item is HELD, never published — the hard gate;
//!    - the global-dojo tenant → [`anonymize_for_global`] (C5, strip + optional
//!      polish + re-check); residual risk → HELD;
//!    - employer / community / personal (named) → the deterministic
//!      [`Dereferenced::new`] BACKSTOP: already-generalised text has nothing to
//!      strip, but if a raw identifier survives the item is HELD rather than
//!      shipped. The text that leaves the machine is ALWAYS the checked text.
//! 4. Publish via the [`ArtifactPublisher`] seam; on success record the assigned
//!    seq/id in the durable outbox; on a transient failure leave it `queued` for
//!    replay (never lost, never double-sent — dedup by artifact signature).
//!
//! The publish + outbox I/O are behind traits ([`ArtifactPublisher`], [`Outbox`])
//! so the contribution logic — including the confidentiality enforcement — is
//! unit-tested with mocks and needs neither a live Dōjō service nor a database.
//! The live HTTP round-trip against a running `sensei-dojo` is DEFERRED to the
//! integration step (the production seams are [`LivePublisher`] + [`PgOutbox`]).

use std::collections::HashMap;
use std::future::Future;

use uuid::Uuid;

use crate::collective::anonymize::{
    ContributorIdentity, GatewayGeneralizer, Generalizer, ProjectShape, anonymize_for_global,
    current_rotation_bucket,
};
use crate::db::pg_store::{DojoMembership, PgStore, ShareBatchItem};
use crate::dojo::attribution::{Dereferenced, ProjectIdentifiers};
use crate::dojo::client::{DojoClient, DojoClientError};
use crate::dojo::memberships::MembershipKind;
use crate::dojo::routing::{Contribution, MembershipRef, RoutingDecision, client_precedence_route};
use dojo_protocol::{
    ArtifactKind, ArtifactPayload, ArtifactScope, Attribution, AttributionMode, PatternFamily,
    PatternPayload, PrinciplePayload, PublishArtifactResponse, PublishedArtifact,
    artifact_signature,
};

/// The global-collective tenant key — the special-case public Dōjō everyone may
/// join (seeded from `database/import/staging/tenants.jsonl` with a FIXED uuid, so the
/// global tenant is the same row on every plane). A destination with
/// this tenant takes the stricter [`anonymize_for_global`] path.
pub const GLOBAL_DOJO_TENANT_KEY: &str = "org/global-dojo";

/// Whether a tenant is the global collective (→ anonymise, not just dereference).
fn is_global_dojo(tenant_key: &str) -> bool {
    tenant_key == GLOBAL_DOJO_TENANT_KEY
}

// ── Seams (mockable for unit tests) ─────────────────────────────────────────

/// The publish seam — one method, so the contribute logic is testable without a
/// live service. [`LivePublisher`] is the production impl (a [`DojoClient`] POST).
pub trait ArtifactPublisher {
    fn publish(
        &self,
        membership: &DojoMembership,
        artifact: &PublishedArtifact,
    ) -> impl Future<Output = Result<PublishArtifactResponse, DojoClientError>> + Send;
}

/// A non-sent outbox state recorded by the contribute path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxState {
    /// Planned, not yet attempted — the cadence scheduler STAGES a batch here
    /// ([`stage_contribution`]); the outbox→dojo publish stays the manual C6 step.
    Pending,
    /// Confidentiality gate refused it (residual identifier risk).
    Held,
    /// Transient transport failure — awaits replay.
    Queued,
    /// Permanent failure (4xx / undecodable) — logged, not auto-retried.
    Error,
}

impl OutboxState {
    fn as_str(self) -> &'static str {
        match self {
            OutboxState::Pending => "pending",
            OutboxState::Held => "held",
            OutboxState::Queued => "queued",
            OutboxState::Error => "error",
        }
    }
}

/// Identity of one outbox row: the destination membership + the source batch /
/// memory + the artifact signature (the dedup key).
#[derive(Debug, Clone, Copy)]
pub struct OutboxKey<'a> {
    pub membership_id: Uuid,
    pub batch_id: Uuid,
    pub memory_id: Uuid,
    pub signature: &'a str,
}

/// The durable send-ledger seam. [`PgOutbox`] is the production impl over
/// `sensei.dojo_outbox`; tests use an in-memory impl.
pub trait Outbox {
    /// Has `signature` already been published to `membership_id`? The pre-send
    /// dedup check — a replay skips an already-sent artifact.
    fn already_sent(
        &self,
        membership_id: Uuid,
        signature: &str,
    ) -> impl Future<Output = Result<bool, String>> + Send;

    /// Record a successful publish.
    fn mark_sent(
        &self,
        key: OutboxKey<'_>,
        sent_seq: i64,
        remote_id: &str,
    ) -> impl Future<Output = Result<(), String>> + Send;

    /// Record a non-sent state (held / queued / error) without clobbering an
    /// already-sent row.
    fn mark_state(
        &self,
        key: OutboxKey<'_>,
        state: OutboxState,
    ) -> impl Future<Output = Result<(), String>> + Send;
}

/// Production publisher — resolves a [`DojoClient`] per membership and POSTs.
pub struct LivePublisher;

impl ArtifactPublisher for LivePublisher {
    fn publish(
        &self,
        membership: &DojoMembership,
        artifact: &PublishedArtifact,
    ) -> impl Future<Output = Result<PublishArtifactResponse, DojoClientError>> + Send {
        let client = DojoClient::for_membership(membership);
        let artifact = artifact.clone();
        async move { client.publish_artifact(&artifact).await }
    }
}

/// Production outbox over `sensei.dojo_outbox`.
pub struct PgOutbox<'a>(pub &'a PgStore);

impl Outbox for PgOutbox<'_> {
    fn already_sent(
        &self,
        membership_id: Uuid,
        signature: &str,
    ) -> impl Future<Output = Result<bool, String>> + Send {
        let store = self.0;
        let signature = signature.to_string();
        async move { store.outbox_already_sent(&membership_id, &signature).await }
    }

    fn mark_sent(
        &self,
        key: OutboxKey<'_>,
        sent_seq: i64,
        remote_id: &str,
    ) -> impl Future<Output = Result<(), String>> + Send {
        let store = self.0;
        let (membership_id, batch_id, memory_id) = (key.membership_id, key.batch_id, key.memory_id);
        let signature = key.signature.to_string();
        let remote_id = remote_id.to_string();
        async move {
            store
                .outbox_mark_sent(
                    &membership_id,
                    Some(&batch_id),
                    Some(&memory_id),
                    &signature,
                    sent_seq,
                    &remote_id,
                )
                .await
        }
    }

    fn mark_state(
        &self,
        key: OutboxKey<'_>,
        state: OutboxState,
    ) -> impl Future<Output = Result<(), String>> + Send {
        let store = self.0;
        let (membership_id, batch_id, memory_id) = (key.membership_id, key.batch_id, key.memory_id);
        let signature = key.signature.to_string();
        async move {
            store
                .outbox_mark_state(
                    &membership_id,
                    Some(&batch_id),
                    Some(&memory_id),
                    &signature,
                    state.as_str(),
                )
                .await
        }
    }
}

// ── Artifact construction (confidentiality is enforced HERE) ─────────────────

/// Map a `sensei.memory_type` to the artifact kind it contributes as: a `pattern`
/// memory → the `pattern` artifact; every other type → a `principle`.
fn artifact_kind_for(memory_type: &str) -> ArtifactKind {
    match memory_type {
        "pattern" => ArtifactKind::Pattern,
        _ => ArtifactKind::Principle,
    }
}

/// The type-specific payload for a kind. Kept minimal: the lesson lives in the
/// artifact `body` (the single confidentiality-checked field); the payload
/// carries only non-sensitive structural metadata.
fn payload_for(kind: ArtifactKind) -> ArtifactPayload {
    match kind {
        ArtifactKind::Pattern => ArtifactPayload::Pattern(PatternPayload {
            family: PatternFamily::Design,
            pattern_id: None,
            ftr_delta_observed: None,
        }),
        _ => ArtifactPayload::Principle(PrinciplePayload { rationale: None }),
    }
}

/// Client-work credit: anonymous with no rotating id (no personal credit at
/// all). The source is stripped by the always-on dereference on the publish
/// path, independent of this credit posture.
fn anonymous_attribution() -> Attribution {
    Attribution { mode: AttributionMode::Anonymous, author: None, org: None, anonymous_id: None }
}

fn named_attribution() -> Attribution {
    Attribution { mode: AttributionMode::Named, author: None, org: None, anonymous_id: None }
}

/// The plan for one (memory × destination): publish the built artifact, or hold
/// it because the confidentiality gate could not guarantee safety. Both carry a
/// stable `signature` so the outbox row is keyed either way.
enum ItemPlan {
    Publish(Box<PublishedArtifact>),
    Held { signature: String },
}

/// Build the publish-safe artifact for one memory routed to one destination, or
/// return [`ItemPlan::Held`] when the confidentiality gate refuses it. DB-free
/// and pure w.r.t. the (mockable) generalizer — the confidentiality enforcement
/// is exercised here without a service or database.
#[allow(clippy::too_many_arguments)]
async fn build_artifact_for_target<G: Generalizer>(
    item: &ShareBatchItem,
    membership: &DojoMembership,
    dereference: bool,
    ctx: &ProjectIdentifiers,
    shape: &ProjectShape,
    contributor: &ContributorIdentity,
    rotation_bucket: i64,
    generalizer: &G,
) -> ItemPlan {
    let kind = artifact_kind_for(&item.memory_type);
    let payload = payload_for(kind);
    // A stable signature over the SOURCE text keys a held/queued outbox row even
    // when no safe artifact could be built.
    let source_signature = artifact_signature(kind, &item.title, &item.body, &payload);
    let held = || ItemPlan::Held { signature: source_signature.clone() };

    // The title is always run through the deterministic strip (short, name-like —
    // never LLM-rewritten). Residual risk in the title holds the whole item.
    let title = match Dereferenced::new(&item.title, ctx) {
        Ok(d) => d.into_text(),
        Err(_) => return held(),
    };

    let global = is_global_dojo(&membership.tenant_key);
    // Every branch strips the source (the deterministic `Dereferenced::new`, or
    // the global anonymiser which strips): source-dereference is always-on, not
    // a stored flag. The branches differ only in CREDIT posture.
    let (body, attribution) = if dereference {
        // CLIENT — anonymous credit, no rotating id. Fail closed: residual risk → held.
        match Dereferenced::new(&item.body, ctx) {
            Ok(d) => (d.into_text(), anonymous_attribution()),
            Err(_) => return held(),
        }
    } else if global {
        // GLOBAL — anonymise (deterministic strip + optional polish + re-check).
        match anonymize_for_global(
            &item.body,
            ctx,
            shape.clone(),
            contributor,
            rotation_bucket,
            generalizer,
        )
        .await
        {
            Ok(a) => (a.text, a.attribution),
            Err(_) => return held(),
        }
    } else {
        // NAMED (employer / community / personal) — deterministic BACKSTOP. The
        // memory is already generalised; the strip is a no-op in the happy path,
        // but a surviving raw identifier holds the item rather than shipping it.
        match Dereferenced::new(&item.body, ctx) {
            Ok(d) => (d.into_text(), named_attribution()),
            Err(_) => return held(),
        }
    };

    // Sign over the CHECKED text — the outbox dedups on exactly what ships.
    let signature = artifact_signature(kind, &title, &body, &payload);
    ItemPlan::Publish(Box::new(PublishedArtifact {
        signature,
        tenant_key: membership.tenant_key.clone(),
        engagement_id: None,
        kind,
        title,
        body,
        payload,
        scope: ArtifactScope { stack: shape.stack.first().cloned(), ..Default::default() },
        attribution,
        contributed_by: None, // the Dōjō stamps the contributor from the auth token
        published_at: None,
    }))
}

// ── Outcome types ────────────────────────────────────────────────────────────

/// The result of contributing one memory to one destination.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ItemResult {
    /// Published — the Dōjō assigned `seq` / `remote_id`.
    Published { seq: i64, remote_id: String },
    /// Staged into the durable outbox as `pending` by the cadence scheduler
    /// ([`stage_contribution`]) — prepared, not yet published. The outbox→dojo
    /// send stays the explicit manual C6 step.
    Staged,
    /// Held: the confidentiality gate refused it (residual identifier risk).
    HeldResidualRisk,
    /// A transient failure — left queued for replay.
    QueuedRetry,
    /// Already published earlier (dedup) — skipped, not re-sent.
    AlreadySent,
    /// A permanent failure.
    Error { message: String },
    /// No Dōjō destination for this batch's project (unbound / no memberships).
    NoDestination,
}

/// The outcome of contributing one memory to one destination.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ItemOutcome {
    pub memory_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub membership_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_key: Option<String>,
    pub kind: ArtifactKind,
    #[serde(flatten)]
    pub result: ItemResult,
}

/// The outcome of contributing a whole batch — per-item plus roll-up counts.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContributeOutcome {
    pub batch_id: Uuid,
    pub published: usize,
    pub held: usize,
    pub queued: usize,
    pub errored: usize,
    pub already_sent: usize,
    pub items: Vec<ItemOutcome>,
}

impl ContributeOutcome {
    fn from_items(batch_id: Uuid, items: Vec<ItemOutcome>) -> Self {
        let mut o = ContributeOutcome {
            batch_id,
            published: 0,
            held: 0,
            queued: 0,
            errored: 0,
            already_sent: 0,
            items,
        };
        for it in &o.items {
            match it.result {
                ItemResult::Published { .. } => o.published += 1,
                ItemResult::HeldResidualRisk => o.held += 1,
                ItemResult::QueuedRetry => o.queued += 1,
                ItemResult::Error { .. } => o.errored += 1,
                ItemResult::AlreadySent => o.already_sent += 1,
                // `Staged` is only produced by the stage-only path; the publish
                // path never yields it.
                ItemResult::Staged | ItemResult::NoDestination => {}
            }
        }
        o
    }
}

/// The outcome of STAGING a whole batch into the durable outbox (the prepare-only
/// path the cadence scheduler runs) — per-item plus roll-up counts. Distinct from
/// [`ContributeOutcome`] because staging never publishes: an item is `staged`
/// (`pending` in the outbox), `held` (confidentiality gate), `already_sent`
/// (skipped — earlier manual publish), or `errored` (outbox write failed).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StageOutcome {
    pub batch_id: Uuid,
    pub staged: usize,
    pub held: usize,
    pub already_sent: usize,
    pub errored: usize,
    pub items: Vec<ItemOutcome>,
}

impl StageOutcome {
    fn from_items(batch_id: Uuid, items: Vec<ItemOutcome>) -> Self {
        let mut o =
            StageOutcome { batch_id, staged: 0, held: 0, already_sent: 0, errored: 0, items };
        for it in &o.items {
            match it.result {
                ItemResult::Staged => o.staged += 1,
                ItemResult::HeldResidualRisk => o.held += 1,
                ItemResult::AlreadySent => o.already_sent += 1,
                ItemResult::Error { .. } => o.errored += 1,
                // Published / QueuedRetry never occur on the stage-only path;
                // NoDestination is reported but not counted.
                _ => {}
            }
        }
        o
    }
}

// ── Loaded batch + orchestration ─────────────────────────────────────────────

/// Everything the contribution needs, loaded from the DB once so the
/// orchestration itself is DB-free and testable.
pub struct LoadedBatch {
    pub batch_id: Uuid,
    pub routing: RoutingDecision,
    pub memberships: HashMap<Uuid, DojoMembership>,
    pub identifiers: ProjectIdentifiers,
    pub shape: ProjectShape,
    pub items: Vec<ShareBatchItem>,
}

/// Load an APPROVED batch into a [`LoadedBatch`]: its member memories, the
/// project's routing (bound membership + candidate memberships → client-precedence
/// decision), and the project identifiers the confidentiality strip needs.
///
/// Errors when the batch is missing or not `approved` — only approved batches
/// contribute (the human review already happened).
pub async fn load_batch(store: &PgStore, batch_id: Uuid) -> Result<LoadedBatch, String> {
    let (project_id, status, items) = store
        .batch_share_items(&batch_id)
        .await?
        .ok_or_else(|| format!("share batch {batch_id} not found"))?;
    if status != "approved" {
        return Err(format!(
            "batch {batch_id} is '{status}', not 'approved' — only approved batches contribute"
        ));
    }

    let membership_rows = store.list_dojo_memberships().await?;
    let candidates: Vec<MembershipRef> = membership_rows
        .iter()
        .filter_map(|m| {
            MembershipKind::from_db_str(&m.kind)
                .map(|kind| MembershipRef { membership_id: m.id, kind })
        })
        .collect();
    let memberships: HashMap<Uuid, DojoMembership> =
        membership_rows.into_iter().map(|m| (m.id, m)).collect();

    let bound = store.project_bound_membership(&project_id).await?;
    let routing = client_precedence_route(&candidates, &Contribution { bound_membership: bound });
    let identifiers = store.project_identifiers(&project_id).await?;

    Ok(LoadedBatch {
        batch_id,
        routing,
        memberships,
        identifiers,
        // Coarse project-shape descriptor (stack/size/kind) is deferred for C6 —
        // the global anonymise path is safe with an empty shape (it carries no
        // identifiers); enriching it is a follow-up.
        shape: ProjectShape::default(),
        items,
    })
}

/// Orchestrate a loaded batch through the seams: build the safe artifact, dedup
/// against the outbox, publish, and record the outcome. DB-free (the DB lives
/// behind [`Outbox`]); the confidentiality gate is inside [`build_artifact_for_target`].
pub async fn run_contribution<P, O, G>(
    loaded: &LoadedBatch,
    publisher: &P,
    outbox: &O,
    generalizer: &G,
    contributor: &ContributorIdentity,
    rotation_bucket: i64,
) -> ContributeOutcome
where
    P: ArtifactPublisher,
    O: Outbox,
    G: Generalizer,
{
    let mut items = Vec::new();

    if loaded.routing.targets.is_empty() {
        // Nothing is bound / no memberships — surface it, never silently drop.
        for it in &loaded.items {
            items.push(ItemOutcome {
                memory_id: it.memory_id,
                membership_id: None,
                tenant_key: None,
                kind: artifact_kind_for(&it.memory_type),
                result: ItemResult::NoDestination,
            });
        }
        return ContributeOutcome::from_items(loaded.batch_id, items);
    }

    for target in &loaded.routing.targets {
        let Some(membership) = loaded.memberships.get(&target.membership_id) else {
            tracing::error!(membership = %target.membership_id, "contribute: routed target missing from membership map — skipping");
            continue;
        };
        for it in &loaded.items {
            let kind = artifact_kind_for(&it.memory_type);
            let plan = build_artifact_for_target(
                it,
                membership,
                target.dereference,
                &loaded.identifiers,
                &loaded.shape,
                contributor,
                rotation_bucket,
                generalizer,
            )
            .await;

            let result = match plan {
                ItemPlan::Held { signature } => {
                    let key = OutboxKey {
                        membership_id: target.membership_id,
                        batch_id: loaded.batch_id,
                        memory_id: it.memory_id,
                        signature: &signature,
                    };
                    if let Err(e) = outbox.mark_state(key, OutboxState::Held).await {
                        tracing::error!(error = %e, memory = %it.memory_id, "contribute: outbox held-record failed");
                    }
                    ItemResult::HeldResidualRisk
                }
                ItemPlan::Publish(art) => {
                    match outbox.already_sent(target.membership_id, &art.signature).await {
                        Ok(true) => ItemResult::AlreadySent,
                        Ok(false) => {
                            publish_one(
                                publisher,
                                outbox,
                                membership,
                                target.membership_id,
                                loaded.batch_id,
                                it.memory_id,
                                &art,
                            )
                            .await
                        }
                        Err(e) => {
                            tracing::error!(error = %e, memory = %it.memory_id, "contribute: outbox dedup check failed");
                            ItemResult::Error { message: e }
                        }
                    }
                }
            };

            items.push(ItemOutcome {
                memory_id: it.memory_id,
                membership_id: Some(target.membership_id),
                tenant_key: Some(membership.tenant_key.clone()),
                kind,
                result,
            });
        }
    }

    ContributeOutcome::from_items(loaded.batch_id, items)
}

/// Publish one already-checked artifact and record the outcome in the outbox.
#[allow(clippy::too_many_arguments)]
async fn publish_one<P: ArtifactPublisher, O: Outbox>(
    publisher: &P,
    outbox: &O,
    membership: &DojoMembership,
    membership_id: Uuid,
    batch_id: Uuid,
    memory_id: Uuid,
    art: &PublishedArtifact,
) -> ItemResult {
    let key = OutboxKey { membership_id, batch_id, memory_id, signature: &art.signature };
    match publisher.publish(membership, art).await {
        Ok(resp) => {
            if let Err(e) = outbox.mark_sent(key, resp.seq, &resp.id).await {
                // The publish succeeded; a failed ledger write is logged (never
                // silent) — the server-side signature dedup protects a replay.
                tracing::error!(error = %e, memory = %memory_id, "contribute: outbox sent-record failed after publish");
            }
            ItemResult::Published { seq: resp.seq, remote_id: resp.id }
        }
        Err(e) => {
            let retryable = e.is_retryable();
            let state = if retryable { OutboxState::Queued } else { OutboxState::Error };
            if let Err(oe) = outbox.mark_state(key, state).await {
                tracing::error!(error = %oe, memory = %memory_id, "contribute: outbox failure-record failed");
            }
            if retryable {
                tracing::warn!(error = %e, memory = %memory_id, "contribute: publish failed transiently — queued for replay");
                ItemResult::QueuedRetry
            } else {
                tracing::error!(error = %e, memory = %memory_id, "contribute: publish failed permanently");
                ItemResult::Error { message: e.to_string() }
            }
        }
    }
}

/// Orchestrate a loaded batch through the STAGE-ONLY path: build the safe
/// artifact for each (memory × destination) using the SAME confidentiality gate
/// as the publish path ([`build_artifact_for_target`]) and record it in the
/// durable outbox as `pending` — it NEVER publishes / egresses to any Dōjō.
///
/// This is what the contribute *cadence scheduler* runs: it PREPARES an approved
/// batch into the local outbox; the outbox→dojo send stays the explicit manual
/// C6 step ([`run_contribution`]). A held item is recorded `held` (never staged),
/// and an item already `sent` by an earlier manual publish is skipped — so
/// re-staging within a cadence window is idempotent (also guarded by the outbox's
/// `unique(membership_id, signature)` + the `state <> 'sent'` write guard).
pub async fn stage_contribution<O, G>(
    loaded: &LoadedBatch,
    outbox: &O,
    generalizer: &G,
    contributor: &ContributorIdentity,
    rotation_bucket: i64,
) -> StageOutcome
where
    O: Outbox,
    G: Generalizer,
{
    let mut items = Vec::new();

    if loaded.routing.targets.is_empty() {
        // Nothing is bound / no memberships — surface it, never silently drop.
        for it in &loaded.items {
            items.push(ItemOutcome {
                memory_id: it.memory_id,
                membership_id: None,
                tenant_key: None,
                kind: artifact_kind_for(&it.memory_type),
                result: ItemResult::NoDestination,
            });
        }
        return StageOutcome::from_items(loaded.batch_id, items);
    }

    for target in &loaded.routing.targets {
        let Some(membership) = loaded.memberships.get(&target.membership_id) else {
            tracing::error!(membership = %target.membership_id, "stage: routed target missing from membership map — skipping");
            continue;
        };
        for it in &loaded.items {
            let kind = artifact_kind_for(&it.memory_type);
            let plan = build_artifact_for_target(
                it,
                membership,
                target.dereference,
                &loaded.identifiers,
                &loaded.shape,
                contributor,
                rotation_bucket,
                generalizer,
            )
            .await;

            let result = match plan {
                ItemPlan::Held { signature } => {
                    let key = OutboxKey {
                        membership_id: target.membership_id,
                        batch_id: loaded.batch_id,
                        memory_id: it.memory_id,
                        signature: &signature,
                    };
                    if let Err(e) = outbox.mark_state(key, OutboxState::Held).await {
                        tracing::error!(error = %e, memory = %it.memory_id, "stage: outbox held-record failed");
                    }
                    ItemResult::HeldResidualRisk
                }
                ItemPlan::Publish(art) => {
                    let key = OutboxKey {
                        membership_id: target.membership_id,
                        batch_id: loaded.batch_id,
                        memory_id: it.memory_id,
                        signature: &art.signature,
                    };
                    match outbox.already_sent(target.membership_id, &art.signature).await {
                        // A prior manual publish already sent this — never downgrade to pending.
                        Ok(true) => ItemResult::AlreadySent,
                        Ok(false) => match outbox.mark_state(key, OutboxState::Pending).await {
                            Ok(()) => ItemResult::Staged,
                            Err(e) => {
                                tracing::error!(error = %e, memory = %it.memory_id, "stage: outbox pending-record failed");
                                ItemResult::Error { message: e }
                            }
                        },
                        Err(e) => {
                            tracing::error!(error = %e, memory = %it.memory_id, "stage: outbox dedup check failed");
                            ItemResult::Error { message: e }
                        }
                    }
                }
            };

            items.push(ItemOutcome {
                memory_id: it.memory_id,
                membership_id: Some(target.membership_id),
                tenant_key: Some(membership.tenant_key.clone()),
                kind,
                result,
            });
        }
    }

    StageOutcome::from_items(loaded.batch_id, items)
}

/// Production entry point: contribute an approved batch to its routed Dōjō(s).
/// Wires the live seams ([`LivePublisher`] + [`PgOutbox`] + [`GatewayGeneralizer`]).
pub async fn contribute_batch(
    store: &PgStore,
    gateway: std::sync::Arc<gateway::Gateway>,
    batch_id: Uuid,
) -> Result<ContributeOutcome, String> {
    let loaded = load_batch(store, batch_id).await?;
    let generalizer = GatewayGeneralizer::new(gateway);
    let contributor =
        ContributorIdentity { user_key: store.get_or_create_contributor_key().await? };
    let outbox = PgOutbox(store);
    let publisher = LivePublisher;
    Ok(run_contribution(
        &loaded,
        &publisher,
        &outbox,
        &generalizer,
        &contributor,
        current_rotation_bucket(),
    )
    .await)
}

// ── Share-review preview (no publish) ────────────────────────────────────────

/// A generalizer that never calls a model — the deterministic strip only. Used
/// for the share-review PREVIEW (cheap + deterministic) and for tests exercising
/// the confidentiality guarantee with no model available.
pub struct NoLlm;

impl Generalizer for NoLlm {
    async fn generalize(&self, _text: &str) -> Option<String> {
        None
    }
}

/// One item as previewed on the share-review surface: the redacted text the user
/// is about to share and whether it is held for residual risk.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ItemPreview {
    pub memory_id: Uuid,
    #[serde(rename = "type")]
    pub kind: ArtifactKind,
    pub title: String,
    pub body: String,
    pub attribution: Attribution,
    pub will_dereference: bool,
    /// `queued` (ships next batch) or `held` (residual risk — won't ship).
    pub state: &'static str,
}

/// The next batch preview surfaced by `GET /api/share-review/next-batch`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BatchPreview {
    pub batch_id: Uuid,
    /// The routed destination tenant keys (`[]` when unbound / no memberships).
    pub destination: Vec<String>,
    /// Cadence controls (observatory-collective / C9) are not modelled yet.
    pub cadence: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_batch_at: Option<String>,
    pub items: Vec<ItemPreview>,
}

/// Build the share-review preview for one approved batch WITHOUT publishing:
/// deterministically dereference each item against the PRIMARY routed destination
/// and report which items would ship vs. be held. `contributor` seeds the global
/// anonymous-id derivation for a preview against the global collective.
pub async fn preview_batch(
    store: &PgStore,
    batch_id: Uuid,
    contributor: &ContributorIdentity,
) -> Result<BatchPreview, String> {
    let loaded = load_batch(store, batch_id).await?;
    let destination: Vec<String> = loaded
        .routing
        .targets
        .iter()
        .filter_map(|t| loaded.memberships.get(&t.membership_id).map(|m| m.tenant_key.clone()))
        .collect();

    let mut items = Vec::with_capacity(loaded.items.len());
    for it in &loaded.items {
        let kind = artifact_kind_for(&it.memory_type);
        // Preview against the primary (highest-precedence) destination.
        let preview = match loaded.routing.targets.first() {
            Some(target) => {
                let membership = loaded.memberships.get(&target.membership_id);
                match membership {
                    Some(m) => {
                        let plan = build_artifact_for_target(
                            it,
                            m,
                            target.dereference,
                            &loaded.identifiers,
                            &loaded.shape,
                            contributor,
                            current_rotation_bucket(),
                            &NoLlm,
                        )
                        .await;
                        let will_dereference = target.dereference || is_global_dojo(&m.tenant_key);
                        match plan {
                            ItemPlan::Publish(art) => ItemPreview {
                                memory_id: it.memory_id,
                                kind,
                                title: art.title.clone(),
                                body: art.body.clone(),
                                attribution: art.attribution.clone(),
                                will_dereference,
                                state: "queued",
                            },
                            ItemPlan::Held { .. } => ItemPreview {
                                memory_id: it.memory_id,
                                kind,
                                title: String::new(),
                                body: String::new(),
                                attribution: if will_dereference {
                                    anonymous_attribution()
                                } else {
                                    named_attribution()
                                },
                                will_dereference,
                                state: "held",
                            },
                        }
                    }
                    None => held_no_destination(it, kind),
                }
            }
            None => held_no_destination(it, kind),
        };
        items.push(preview);
    }

    Ok(BatchPreview { batch_id, destination, cadence: "manual", next_batch_at: None, items })
}

fn held_no_destination(it: &ShareBatchItem, kind: ArtifactKind) -> ItemPreview {
    ItemPreview {
        memory_id: it.memory_id,
        kind,
        title: String::new(),
        body: String::new(),
        attribution: named_attribution(),
        will_dereference: false,
        state: "held",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Mutex;

    use crate::dojo::routing::{RouteTarget, RoutingDecision};

    fn ctx() -> ProjectIdentifiers {
        ProjectIdentifiers {
            project_name: Some("Acme API".into()),
            client_name: Some("Acme Corp".into()),
            repo_names: vec!["acme-api".into()],
            folder_paths: vec!["/Users/jerry/work/acme-api".into()],
            session_ids: vec!["s-9931".into()],
            person_names: vec!["Jane Doe".into()],
        }
    }

    fn contributor() -> ContributorIdentity {
        ContributorIdentity { user_key: "local-user-secret".into() }
    }

    fn membership(kind: &str, tenant: &str) -> DojoMembership {
        DojoMembership {
            id: Uuid::new_v4(),
            registry_url: "http://localhost:7755".into(),
            tenant_key: tenant.into(),
            dojo_url: format!("http://localhost:7755/{tenant}"),
            kind: kind.into(),
            org_slugs: vec![],
            role: "contributor".into(),
            authenticated_via: "device_code".into(),
            attribution_default: "named".into(),
            credential_ref: "cref".into(),
            sync_status: "healthy".into(),
            last_seq: 0,
            last_heartbeat_at: None,
            enabled: true,
        }
    }

    fn item(title: &str, body: &str, mtype: &str) -> ShareBatchItem {
        ShareBatchItem {
            memory_id: Uuid::new_v4(),
            title: title.into(),
            body: body.into(),
            memory_type: mtype.into(),
        }
    }

    fn loaded(m: DojoMembership, dereference: bool, items: Vec<ShareBatchItem>) -> LoadedBatch {
        let mid = m.id;
        let kind = MembershipKind::from_db_str(&m.kind).unwrap();
        LoadedBatch {
            batch_id: Uuid::new_v4(),
            routing: RoutingDecision {
                targets: vec![RouteTarget { membership_id: mid, kind, dereference }],
                excluded: vec![],
            },
            memberships: HashMap::from([(mid, m)]),
            identifiers: ctx(),
            shape: ProjectShape::default(),
            items,
        }
    }

    #[derive(Clone, Copy)]
    enum Behavior {
        Ok,
        Network,
        Bad4xx,
    }

    struct RecordingPublisher {
        calls: Mutex<Vec<(Uuid, PublishedArtifact)>>,
        behavior: Behavior,
    }

    impl RecordingPublisher {
        fn new(behavior: Behavior) -> Self {
            Self { calls: Mutex::new(Vec::new()), behavior }
        }
        fn published(&self) -> Vec<PublishedArtifact> {
            self.calls.lock().unwrap().iter().map(|(_, a)| a.clone()).collect()
        }
    }

    impl ArtifactPublisher for RecordingPublisher {
        fn publish(
            &self,
            membership: &DojoMembership,
            artifact: &PublishedArtifact,
        ) -> impl Future<Output = Result<PublishArtifactResponse, DojoClientError>> + Send {
            self.calls.lock().unwrap().push((membership.id, artifact.clone()));
            let behavior = self.behavior;
            let id = membership.id;
            async move {
                match behavior {
                    Behavior::Ok => Ok(PublishArtifactResponse { id: format!("art-{id}"), seq: 7 }),
                    Behavior::Network => Err(DojoClientError::Network("connection reset".into())),
                    Behavior::Bad4xx => Err(DojoClientError::Status(400)),
                }
            }
        }
    }

    #[derive(Default)]
    struct MemOutbox {
        sent: Mutex<HashSet<(Uuid, String)>>,
        states: Mutex<Vec<(Uuid, String, OutboxState)>>,
    }

    impl Outbox for MemOutbox {
        fn already_sent(
            &self,
            membership_id: Uuid,
            signature: &str,
        ) -> impl Future<Output = Result<bool, String>> + Send {
            let hit = self.sent.lock().unwrap().contains(&(membership_id, signature.to_string()));
            async move { Ok(hit) }
        }
        fn mark_sent(
            &self,
            key: OutboxKey<'_>,
            _sent_seq: i64,
            _remote_id: &str,
        ) -> impl Future<Output = Result<(), String>> + Send {
            self.sent.lock().unwrap().insert((key.membership_id, key.signature.to_string()));
            async move { Ok(()) }
        }
        fn mark_state(
            &self,
            key: OutboxKey<'_>,
            state: OutboxState,
        ) -> impl Future<Output = Result<(), String>> + Send {
            self.states.lock().unwrap().push((key.membership_id, key.signature.to_string(), state));
            async move { Ok(()) }
        }
    }

    fn no_identifier(text: &str) {
        let low = text.to_ascii_lowercase();
        assert!(!low.contains("acme"), "leaked project/client/repo name: {text:?}");
        assert!(!low.contains("jane"), "leaked person name: {text:?}");
        assert!(!text.contains("/Users/"), "leaked path: {text:?}");
        assert!(!low.contains("s-9931"), "leaked session id: {text:?}");
    }

    // ── the HARD confidentiality gate ────────────────────────────────────────

    #[tokio::test]
    async fn client_route_holds_residual_risk_item_and_never_publishes_it() {
        // A SCREAMING_SNAKE secret survives the strip → residual risk → the item
        // MUST be held, NOT published. Confidentiality — nothing matters more.
        let l = loaded(
            membership("client", "github/acme"),
            true,
            vec![item("a lesson", "export ACME_PROD_DB_PASSWORD before deploy", "convention")],
        );
        let pubr = RecordingPublisher::new(Behavior::Ok);
        let obx = MemOutbox::default();
        let out = run_contribution(&l, &pubr, &obx, &NoLlm, &contributor(), 1).await;

        assert_eq!(out.held, 1);
        assert_eq!(out.published, 0);
        assert!(matches!(out.items[0].result, ItemResult::HeldResidualRisk));
        assert!(pubr.published().is_empty(), "a held item must never reach the publisher");
        // Recorded as held in the outbox (not silently dropped).
        assert_eq!(obx.states.lock().unwrap().len(), 1);
        assert_eq!(obx.states.lock().unwrap()[0].2, OutboxState::Held);
    }

    #[tokio::test]
    async fn clean_client_item_is_published_dereferenced() {
        let l = loaded(
            membership("client", "github/acme"),
            true,
            vec![item(
                "prefer migration tools",
                "In acme-api we adopted a dedicated migration tool over hand-rolled SQL.",
                "convention",
            )],
        );
        let pubr = RecordingPublisher::new(Behavior::Ok);
        let obx = MemOutbox::default();
        let out = run_contribution(&l, &pubr, &obx, &NoLlm, &contributor(), 1).await;

        assert_eq!(out.published, 1);
        assert!(matches!(out.items[0].result, ItemResult::Published { seq: 7, .. }));
        let published = pubr.published();
        assert_eq!(published.len(), 1);
        let art = &published[0];
        // Client work is anonymous-credit with no rotating id; the source is
        // STILL stripped (the always-on dereference), proven by no_identifier.
        assert_eq!(art.attribution.mode, AttributionMode::Anonymous);
        assert!(art.attribution.anonymous_id.is_none(), "client work carries no rotating id");
        no_identifier(&art.body);
        no_identifier(&art.title);
    }

    #[tokio::test]
    async fn global_route_publishes_anonymized() {
        // A community membership on the global tenant → the anonymise path.
        let l = loaded(
            membership("community", GLOBAL_DOJO_TENANT_KEY),
            false,
            vec![item(
                "write tests",
                "In acme-api, Jane Doe pushed writing tests first.",
                "convention",
            )],
        );
        let pubr = RecordingPublisher::new(Behavior::Ok);
        let obx = MemOutbox::default();
        let out = run_contribution(&l, &pubr, &obx, &NoLlm, &contributor(), 1).await;

        assert_eq!(out.published, 1);
        let art = &pubr.published()[0];
        assert_eq!(
            art.attribution.mode,
            AttributionMode::Anonymous,
            "global uses anonymous attribution"
        );
        assert!(art.attribution.anonymous_id.is_some(), "global carries a rotating anon id");
        no_identifier(&art.body);
    }

    #[tokio::test]
    async fn named_employer_backstop_publishes_but_holds_a_surviving_secret() {
        // Employer/named still runs the deterministic backstop: a clean item ships
        // named-not-dereferenced; a surviving secret is held, not shipped.
        let clean = loaded(
            membership("employer", "github/acme-corp"),
            false,
            vec![item("use ci gates", "Gate merges on a green pipeline.", "convention")],
        );
        let pubr = RecordingPublisher::new(Behavior::Ok);
        let obx = MemOutbox::default();
        let out = run_contribution(&clean, &pubr, &obx, &NoLlm, &contributor(), 1).await;
        assert_eq!(out.published, 1);
        let art = &pubr.published()[0];
        assert_eq!(art.attribution.mode, AttributionMode::Named);
        no_identifier(&art.body); // named work is STILL source-stripped (the always-on invariant)

        let leaky = loaded(
            membership("employer", "github/acme-corp"),
            false,
            vec![item("bad", "set ACME_SECRET_TOKEN=1 first", "convention")],
        );
        let pubr2 = RecordingPublisher::new(Behavior::Ok);
        let obx2 = MemOutbox::default();
        let out2 = run_contribution(&leaky, &pubr2, &obx2, &NoLlm, &contributor(), 1).await;
        assert_eq!(out2.held, 1);
        assert!(pubr2.published().is_empty(), "the backstop must hold a surviving secret");
    }

    // ── outbox dedup: no double-send on replay ───────────────────────────────

    #[tokio::test]
    async fn outbox_dedups_by_signature_so_a_replay_does_not_double_send() {
        let l = loaded(
            membership("client", "github/acme"),
            true,
            vec![item(
                "prefer tools",
                "We used a dedicated migration tool in acme-api.",
                "convention",
            )],
        );
        let pubr = RecordingPublisher::new(Behavior::Ok);
        let obx = MemOutbox::default();

        let first = run_contribution(&l, &pubr, &obx, &NoLlm, &contributor(), 1).await;
        assert_eq!(first.published, 1);
        // Replay the same batch against the same outbox — must skip, not re-send.
        let second = run_contribution(&l, &pubr, &obx, &NoLlm, &contributor(), 1).await;
        assert_eq!(second.published, 0);
        assert_eq!(second.already_sent, 1);
        assert!(matches!(second.items[0].result, ItemResult::AlreadySent));
        assert_eq!(
            pubr.published().len(),
            1,
            "the publisher must be hit exactly once across the replay"
        );
    }

    // ── transient failure queues for replay, then succeeds ───────────────────

    #[tokio::test]
    async fn transient_failure_queues_then_replays_to_success() {
        let l = loaded(
            membership("client", "github/acme"),
            true,
            vec![item(
                "prefer tools",
                "We used a dedicated migration tool in acme-api.",
                "convention",
            )],
        );
        let obx = MemOutbox::default();

        let down = RecordingPublisher::new(Behavior::Network);
        let out1 = run_contribution(&l, &down, &obx, &NoLlm, &contributor(), 1).await;
        assert_eq!(out1.queued, 1);
        assert!(matches!(out1.items[0].result, ItemResult::QueuedRetry));
        assert_eq!(obx.states.lock().unwrap().last().unwrap().2, OutboxState::Queued);
        assert!(obx.sent.lock().unwrap().is_empty(), "a failed publish is not recorded sent");

        // Reconnect: the queued item replays (not yet sent) and now succeeds.
        let up = RecordingPublisher::new(Behavior::Ok);
        let out2 = run_contribution(&l, &up, &obx, &NoLlm, &contributor(), 1).await;
        assert_eq!(out2.published, 1);
    }

    #[tokio::test]
    async fn permanent_4xx_is_error_not_queued() {
        let l = loaded(
            membership("client", "github/acme"),
            true,
            vec![item("prefer tools", "A dedicated migration tool in acme-api.", "convention")],
        );
        let pubr = RecordingPublisher::new(Behavior::Bad4xx);
        let obx = MemOutbox::default();
        let out = run_contribution(&l, &pubr, &obx, &NoLlm, &contributor(), 1).await;
        assert_eq!(out.errored, 1);
        assert!(matches!(out.items[0].result, ItemResult::Error { .. }));
        assert_eq!(obx.states.lock().unwrap().last().unwrap().2, OutboxState::Error);
    }

    #[tokio::test]
    async fn no_destination_when_unbound_and_no_memberships() {
        let mut l =
            loaded(membership("client", "github/acme"), true, vec![item("x", "y", "convention")]);
        l.routing = RoutingDecision::default(); // no targets
        let pubr = RecordingPublisher::new(Behavior::Ok);
        let obx = MemOutbox::default();
        let out = run_contribution(&l, &pubr, &obx, &NoLlm, &contributor(), 1).await;
        assert!(matches!(out.items[0].result, ItemResult::NoDestination));
        assert!(pubr.published().is_empty());
    }

    // ── build_artifact_for_target directly (DB-free confidentiality unit) ─────

    #[tokio::test]
    async fn outcome_serializes_for_the_api() {
        // The share-review handler serializes ContributeOutcome — guard the
        // flatten-over-internally-tagged-enum shape.
        let l = loaded(
            membership("client", "github/acme"),
            true,
            vec![item("prefer tools", "A dedicated migration tool in acme-api.", "convention")],
        );
        let out = run_contribution(
            &l,
            &RecordingPublisher::new(Behavior::Ok),
            &MemOutbox::default(),
            &NoLlm,
            &contributor(),
            1,
        )
        .await;
        let v = serde_json::to_value(&out).expect("outcome serializes");
        assert_eq!(v["published"], serde_json::json!(1));
        assert_eq!(v["items"][0]["result"], serde_json::json!("published"));
        assert_eq!(v["items"][0]["seq"], serde_json::json!(7));
        assert!(v["items"][0]["tenant_key"].is_string());
    }

    // ── stage-only path (cadence scheduler): NEVER publishes ─────────────────

    #[tokio::test]
    async fn stage_records_pending_and_never_publishes() {
        // A clean employer (named) item → the stage path records it `pending` in
        // the outbox and — structurally — cannot publish (no publisher seam).
        let l = loaded(
            membership("employer", "github/acme-corp"),
            false,
            vec![item("use ci gates", "Gate merges on a green pipeline.", "convention")],
        );
        let obx = MemOutbox::default();
        let out = stage_contribution(&l, &obx, &NoLlm, &contributor(), 1).await;

        assert_eq!(out.staged, 1);
        assert_eq!(out.held, 0);
        assert!(matches!(out.items[0].result, ItemResult::Staged));
        // Exactly one `pending` outbox row recorded — nothing sent.
        let states = obx.states.lock().unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].2, OutboxState::Pending);
        assert!(obx.sent.lock().unwrap().is_empty(), "stage-only must never mark anything sent");
    }

    #[tokio::test]
    async fn stage_holds_residual_risk_and_stages_nothing() {
        // The SAME hard confidentiality gate as the publish path: a surviving
        // secret is held, never staged.
        let l = loaded(
            membership("client", "github/acme"),
            true,
            vec![item("a lesson", "export ACME_PROD_DB_PASSWORD before deploy", "convention")],
        );
        let obx = MemOutbox::default();
        let out = stage_contribution(&l, &obx, &NoLlm, &contributor(), 1).await;

        assert_eq!(out.held, 1);
        assert_eq!(out.staged, 0);
        assert!(matches!(out.items[0].result, ItemResult::HeldResidualRisk));
        assert_eq!(obx.states.lock().unwrap()[0].2, OutboxState::Held);
    }

    #[tokio::test]
    async fn stage_skips_an_already_sent_item() {
        // If an earlier manual publish already sent this artifact, staging must
        // skip it (never downgrade sent→pending).
        let l = loaded(
            membership("employer", "github/acme-corp"),
            false,
            vec![item("use ci gates", "Gate merges on a green pipeline.", "convention")],
        );
        let obx = MemOutbox::default();
        // First stage computes the signature and records pending; capture it, then
        // mark it sent to simulate a prior manual publish.
        let first = stage_contribution(&l, &obx, &NoLlm, &contributor(), 1).await;
        assert_eq!(first.staged, 1);
        let sig = obx.states.lock().unwrap()[0].1.clone();
        let mid = l.routing.targets[0].membership_id;
        obx.sent.lock().unwrap().insert((mid, sig));

        let second = stage_contribution(&l, &obx, &NoLlm, &contributor(), 1).await;
        assert_eq!(second.staged, 0);
        assert_eq!(second.already_sent, 1);
        assert!(matches!(second.items[0].result, ItemResult::AlreadySent));
    }

    #[tokio::test]
    async fn stage_no_destination_when_unbound() {
        let mut l =
            loaded(membership("client", "github/acme"), true, vec![item("x", "y", "convention")]);
        l.routing = RoutingDecision::default(); // no targets
        let obx = MemOutbox::default();
        let out = stage_contribution(&l, &obx, &NoLlm, &contributor(), 1).await;
        assert!(matches!(out.items[0].result, ItemResult::NoDestination));
        assert_eq!(out.staged, 0);
        assert!(obx.states.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn build_publishes_pattern_kind_for_pattern_memory() {
        let m = membership("client", "github/acme");
        let plan = build_artifact_for_target(
            &item("adapter", "Use an adapter per language in acme-api.", "pattern"),
            &m,
            true,
            &ctx(),
            &ProjectShape::default(),
            &contributor(),
            1,
            &NoLlm,
        )
        .await;
        match plan {
            ItemPlan::Publish(art) => {
                assert_eq!(art.kind, ArtifactKind::Pattern);
                assert!(art.kind_matches_payload());
                no_identifier(&art.body);
            }
            ItemPlan::Held { .. } => panic!("clean pattern memory should publish"),
        }
    }
}
