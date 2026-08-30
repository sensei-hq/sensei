//! Downstream inbox / distribution (C7) — pull APPROVED Dōjō artifacts back, hold
//! them as `pending` upgrades, and land the ones the user Applies.
//!
//! The loop (`docs/spec/pipeline/dojo-lifecycle.md` → Distribute, and
//! `screen/observatory-upgrades.md`):
//!
//! 1. **Pull** — for each Dōjō membership, `GET .../artifacts?since={cursor}`
//!    ([`DojoClient::pull_artifacts`]) and mirror every published artifact into
//!    `sensei.dojo_inbox` as `pending`, deduped by artifact signature so a re-pull
//!    can never duplicate. The cursor is the membership's `last_seq` (the C4-
//!    provisioned "downstream-artifact pull cursor").
//! 2. **Apply** — landing depends on the artifact kind:
//!    - `principle | pattern` → a [`crate::db::pg_store::InsertMemory`] row with
//!      `origin='dojo'` (the artifact → memory mapping is [`build_memory`]); the
//!      inbox row flips to `applied` with `applied_memory_id` set. Scope is
//!      honoured — a project-scoped artifact whose project is not present here is
//!      NOT landed ([`ApplyOutcome::ScopeMismatch`]).
//!    - `skill | agent | prompt | guard` → NOT auto-installed. Installing into the
//!      assistant-plugin / lint surface is a consent-sensitive step, deferred to a
//!      follow-up; Apply returns [`ApplyOutcome::Deferred`] and leaves the item
//!      `pending` with a note (so it still shows on Upgrades).
//! 3. **Mute / Pin** — local overrides. A muted item drops out of the default
//!    Upgrades list; a pinned item floats to the top. Neither lands anything.
//!
//! Both the network pull ([`DojoPuller`]) and the inbox store ([`Inbox`]) are
//! behind traits, so pull + apply + mute/pin are unit-tested with mocks — no live
//! Dōjō service, no database, no LLM (mirrors [`crate::dojo::contribute`]'s
//! `ArtifactPublisher` / `Outbox` seam). The production seams are [`LivePuller`] +
//! [`PgInbox`].

use std::future::Future;

use uuid::Uuid;

use crate::db::pg_store::{DojoMembership, InsertMemory, PgStore};
use crate::dojo::client::{DojoClient, DojoClientError};
use dojo_protocol::{
    ArtifactKind, ArtifactPullResponse, ArtifactScope, ArtifactStatus, Attribution, PulledArtifact,
};

// ── Row shapes shared with the store impl ────────────────────────────────────

/// An artifact about to be mirrored into `sensei.dojo_inbox` as `pending`.
#[derive(Debug, Clone)]
pub struct InboxRow {
    pub membership_id: Uuid,
    pub artifact_seq: i64,
    pub signature: String,
    pub remote_id: String,
    /// `dojo.artifact_kind` string (`principle | pattern | …`).
    pub kind: String,
    pub title: String,
    pub body: String,
    pub scope: ArtifactScope,
    pub attribution: Attribution,
}

impl InboxRow {
    fn from_pulled(membership_id: Uuid, p: &PulledArtifact) -> Self {
        let a = &p.artifact;
        InboxRow {
            membership_id,
            artifact_seq: p.seq,
            signature: a.signature.clone(),
            remote_id: p.id.clone(),
            kind: a.kind.as_db_str().to_string(),
            title: a.title.clone(),
            body: a.body.clone(),
            scope: a.scope.clone(),
            attribution: a.attribution.clone(),
        }
    }
}

/// One inbox item as loaded for Apply and surfaced on `GET /api/upgrades`. The
/// serialized shape follows `screen/observatory-upgrades.md`
/// (`{id, type, title, body, scope, attribution, received_at, state}`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct InboxItem {
    pub id: Uuid,
    pub membership_id: Uuid,
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub body: String,
    pub scope: ArtifactScope,
    pub attribution: Attribution,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_memory_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_at: Option<String>,
    /// The dedup signature — internal (never serialized to the API).
    #[serde(skip)]
    pub artifact_signature: String,
}

// ── Seams (mockable for unit tests) ──────────────────────────────────────────

/// The network pull seam — one method, so the pull logic is testable without a
/// live service. [`LivePuller`] is the production impl (a [`DojoClient`] GET).
pub trait DojoPuller {
    fn pull(
        &self,
        membership: &DojoMembership,
        since: i64,
    ) -> impl Future<Output = Result<ArtifactPullResponse, DojoClientError>> + Send;
}

/// The inbox store seam. [`PgInbox`] is the production impl over
/// `sensei.dojo_inbox` (+ `sensei.memories` for landing); tests use an in-memory
/// impl. Memory landing reuses [`PgStore::insert_memory`] — never reimplemented.
pub trait Inbox {
    /// Upsert one pulled artifact as `pending`, deduped by
    /// `(membership_id, signature)`. Returns `true` when a NEW row was inserted,
    /// `false` when the artifact was already present in any state (idempotent
    /// re-pull).
    fn upsert_pending(&self, row: &InboxRow) -> impl Future<Output = Result<bool, String>> + Send;

    /// Advance a membership's downstream pull cursor (`dojo_memberships.last_seq`).
    fn set_cursor(
        &self,
        membership_id: Uuid,
        cursor: i64,
    ) -> impl Future<Output = Result<(), String>> + Send;

    /// Load one inbox item by id.
    fn get(&self, inbox_id: Uuid)
    -> impl Future<Output = Result<Option<InboxItem>, String>> + Send;

    /// Resolve a local project by name → its id (scope-match for a
    /// project-scoped artifact). `None` = no such project on this install.
    fn resolve_project(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Option<Uuid>, String>> + Send;

    /// Land a memory and mark the inbox row `applied` + `applied_memory_id` in one
    /// step. Returns the new memory id.
    fn land_and_apply(
        &self,
        inbox_id: Uuid,
        mem: &InsertMemory,
    ) -> impl Future<Output = Result<Uuid, String>> + Send;

    /// Record why an Apply did not land (deferred kind / scope mismatch); the item
    /// stays `pending`.
    fn set_note(
        &self,
        inbox_id: Uuid,
        note: &str,
    ) -> impl Future<Output = Result<(), String>> + Send;

    /// Set an inbox item's `state` (mute → `muted`, pin → `pinned`). Returns
    /// `false` when the id is unknown (drives a 404). Never lands anything.
    fn set_state(
        &self,
        inbox_id: Uuid,
        state: &str,
    ) -> impl Future<Output = Result<bool, String>> + Send;
}

// ── Outcomes ─────────────────────────────────────────────────────────────────

/// The result of one pull pass over a membership's downstream artifacts.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PullOutcome {
    pub membership_id: Uuid,
    /// Artifacts in the page.
    pub pulled: usize,
    /// New `pending` rows inserted.
    pub inserted: usize,
    /// Already-present (dedup) or non-published artifacts skipped.
    pub skipped: usize,
    /// The cursor persisted after this pass (the response cursor).
    pub new_cursor: i64,
}

/// The result of Applying one inbox item.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ApplyOutcome {
    /// A `principle | pattern` landed as a `sensei.memories` row (origin='dojo').
    Landed { memory_id: Uuid },
    /// Already applied earlier — idempotent no-op; returns the existing memory.
    AlreadyApplied { memory_id: Uuid },
    /// `skill | agent | prompt | guard`: install into the assistant-plugin / lint
    /// surface is a consent-sensitive step, deferred. Nothing written; item stays
    /// pending with a note.
    Deferred { kind: ArtifactKind },
    /// The artifact's scope does not apply to this install (e.g. a project-scoped
    /// artifact whose project is absent). Nothing landed; item stays pending.
    ScopeMismatch { reason: String },
    /// Unknown inbox id.
    NotFound,
}

// ── Pull ─────────────────────────────────────────────────────────────────────

/// Pull one membership's downstream artifacts since its cursor and mirror them as
/// `pending` (deduped by signature), then advance the cursor to the response
/// cursor. Idempotent: a re-pull re-inserts nothing (the `(membership, signature)`
/// key). Per-artifact upsert failures are logged and skipped — never a poison pill
/// (mirrors the rules-federation per-delta isolation) — but a cursor-write failure
/// is surfaced so the pass can be retried.
pub async fn pull_membership_inbox<P, I>(
    puller: &P,
    inbox: &I,
    membership: &DojoMembership,
) -> Result<PullOutcome, String>
where
    P: DojoPuller,
    I: Inbox,
{
    let page = puller.pull(membership, membership.last_seq).await.map_err(|e| e.to_string())?;

    let mut out =
        PullOutcome { membership_id: membership.id, new_cursor: page.cursor, ..Default::default() };
    for pulled in &page.artifacts {
        out.pulled += 1;
        // Only PUBLISHED artifacts distribute downstream; submitted/archived are
        // not consumable (an archived artifact is a retraction — a follow-up).
        if pulled.status != ArtifactStatus::Published {
            out.skipped += 1;
            continue;
        }
        let row = InboxRow::from_pulled(membership.id, pulled);
        match inbox.upsert_pending(&row).await {
            Ok(true) => out.inserted += 1,
            Ok(false) => out.skipped += 1,
            Err(e) => {
                out.skipped += 1;
                tracing::error!(membership = %membership.id, remote = %pulled.id, error = %e,
                    "dojo inbox: upsert failed — skipping this artifact");
            }
        }
    }
    inbox.set_cursor(membership.id, page.cursor).await?;
    Ok(out)
}

// ── Apply ────────────────────────────────────────────────────────────────────

/// Where an artifact's scope lands as a memory. Company/team/no-scope → a global
/// memory (org-wide practice applies to every project); a `stack` scope → a
/// stack-scoped memory; a `project` scope → that project (resolved by name).
enum ScopeTarget {
    Global,
    Stack(String),
    Project { name: String },
}

/// Map an [`ArtifactScope`] to where it should land. Project scope is the most
/// specific, then stack; company/team/none fall back to global (an org- or
/// team-wide principle applies across the user's projects — the memory records
/// it globally rather than dropping the tag on the floor).
fn scope_target(scope: &ArtifactScope) -> ScopeTarget {
    if let Some(p) = scope.project.as_ref().filter(|s| !s.trim().is_empty()) {
        ScopeTarget::Project { name: p.clone() }
    } else if let Some(s) = scope.stack.as_ref().filter(|s| !s.trim().is_empty()) {
        ScopeTarget::Stack(s.clone())
    } else {
        ScopeTarget::Global
    }
}

/// The `sensei.memory_type` a landed artifact takes. Inverse of
/// [`crate::dojo::contribute`]'s `artifact_kind_for`: a `pattern` artifact → a
/// `pattern` memory; a `principle` → a `convention` (a durable team norm).
fn memory_type_for(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Pattern => "pattern",
        _ => "convention",
    }
}

/// Build the `origin='dojo'` memory an Applied principle/pattern lands as. Title →
/// title, body → content, kind → memory type, scope → memory scope/scope_filter.
/// Tagged `dojo` so the provenance is visible; attribution stays on the inbox row
/// (`applied_memory_id` links the two). Enforcement defaults to `recommended`
/// (downstream practice is advisory-ish, never a non-overridable `mandatory`).
fn build_memory(
    item: &InboxItem,
    kind: ArtifactKind,
    project_id: Option<Uuid>,
    scope: &str,
    scope_filter: Option<String>,
) -> InsertMemory {
    InsertMemory {
        project_id,
        scope: scope.to_string(),
        scope_filter,
        mtype: memory_type_for(kind).to_string(),
        title: item.title.clone(),
        content: item.body.clone(),
        impact: None,
        tags: vec!["dojo".to_string()],
        triage_signal: None,
        status: "active".to_string(),
        namespace_id: None,
        enforcement: Some("recommended".to_string()),
        origin: Some("dojo".to_string()),
        // The dojo artifact id is a remote string, not a local memory uuid; the
        // link back lives on dojo_inbox.applied_memory_id instead.
        source_id: None,
        spine_slot: None,
        feature: None,
    }
}

/// A skill/agent/prompt/guard Apply is a deferred no-op — record why so the
/// Upgrades card can explain it.
fn deferred_note(kind: ArtifactKind) -> String {
    format!(
        "apply deferred: a '{}' artifact installs into the assistant-plugin / lint surface, a consent-sensitive step not yet automated — left pending for manual install",
        kind.as_db_str()
    )
}

fn scope_mismatch_reason(name: &str) -> String {
    format!(
        "scope mismatch: artifact is scoped to project '{name}', which is not present on this install — not landed"
    )
}

/// Apply one inbox item. Idempotent (an already-applied item is a no-op). For a
/// deferred kind or a non-matching scope nothing is written and the item stays
/// `pending` with a note, so `GET /api/upgrades` still shows it.
pub async fn apply_inbox_item<I: Inbox>(inbox: &I, inbox_id: Uuid) -> Result<ApplyOutcome, String> {
    let Some(item) = inbox.get(inbox_id).await? else {
        return Ok(ApplyOutcome::NotFound);
    };
    // Idempotent: a row that already landed a memory is a no-op.
    if let Some(mid) = item.applied_memory_id {
        return Ok(ApplyOutcome::AlreadyApplied { memory_id: mid });
    }
    let Some(kind) = ArtifactKind::from_db_str(&item.kind) else {
        return Err(format!("dojo inbox: unknown artifact kind '{}'", item.kind));
    };

    match kind {
        // Consent-sensitive plugin / lint install — deferred (documented scope cut).
        ArtifactKind::Skill | ArtifactKind::Agent | ArtifactKind::Prompt | ArtifactKind::Guard => {
            inbox.set_note(inbox_id, &deferred_note(kind)).await?;
            Ok(ApplyOutcome::Deferred { kind })
        }
        ArtifactKind::Principle | ArtifactKind::Pattern => {
            let (project_id, scope, scope_filter) = match scope_target(&item.scope) {
                ScopeTarget::Global => (None, "global", None),
                ScopeTarget::Stack(s) => (None, "stack", Some(s)),
                ScopeTarget::Project { name } => match inbox.resolve_project(&name).await? {
                    Some(pid) => (Some(pid), "project", None),
                    None => {
                        let reason = scope_mismatch_reason(&name);
                        inbox.set_note(inbox_id, &reason).await?;
                        return Ok(ApplyOutcome::ScopeMismatch { reason });
                    }
                },
            };
            let mem = build_memory(&item, kind, project_id, scope, scope_filter);
            let memory_id = inbox.land_and_apply(inbox_id, &mem).await?;
            Ok(ApplyOutcome::Landed { memory_id })
        }
    }
}

// ── List ordering (pure — the Upgrades default view) ─────────────────────────

/// Order + filter the inbox for the default Upgrades list: drop muted items
/// (unless `include_muted`), float pinned to the top, then newest-first. Pure so
/// the "mute hides / pin floats" contract is unit-tested without a database and is
/// the single ordering both the Pg list and tests use.
pub fn order_and_filter(mut items: Vec<InboxItem>, include_muted: bool) -> Vec<InboxItem> {
    if !include_muted {
        items.retain(|i| i.state != "muted");
    }
    items.sort_by(|a, b| {
        let pinned = |i: &InboxItem| u8::from(i.state == "pinned");
        // pinned first (desc), then most-recent first (desc on the RFC-3339 text).
        pinned(b).cmp(&pinned(a)).then_with(|| b.received_at.cmp(&a.received_at))
    });
    items
}

/// Unread = items still awaiting a decision (`pending`). Pinned/applied/muted have
/// been acted on. Drives the Upgrades `unread_count`.
pub fn unread_count(items: &[InboxItem]) -> usize {
    items.iter().filter(|i| i.state == "pending").count()
}

// ── Production entry points (live seams) ─────────────────────────────────────

/// Production puller — resolves a [`DojoClient`] per membership and GETs.
pub struct LivePuller;

impl DojoPuller for LivePuller {
    fn pull(
        &self,
        membership: &DojoMembership,
        since: i64,
    ) -> impl Future<Output = Result<ArtifactPullResponse, DojoClientError>> + Send {
        let client = DojoClient::for_membership(membership);
        async move { client.pull_artifacts(since).await }
    }
}

/// Production inbox over `sensei.dojo_inbox` (+ `sensei.memories`).
pub struct PgInbox<'a>(pub &'a PgStore);

impl Inbox for PgInbox<'_> {
    fn upsert_pending(&self, row: &InboxRow) -> impl Future<Output = Result<bool, String>> + Send {
        self.0.upsert_dojo_inbox(row)
    }
    fn set_cursor(
        &self,
        membership_id: Uuid,
        cursor: i64,
    ) -> impl Future<Output = Result<(), String>> + Send {
        self.0.set_dojo_pull_cursor(membership_id, cursor)
    }
    fn get(
        &self,
        inbox_id: Uuid,
    ) -> impl Future<Output = Result<Option<InboxItem>, String>> + Send {
        self.0.get_dojo_inbox(inbox_id)
    }
    fn resolve_project(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Option<Uuid>, String>> + Send {
        self.0.resolve_project_by_name(name.to_string())
    }
    fn land_and_apply(
        &self,
        inbox_id: Uuid,
        mem: &InsertMemory,
    ) -> impl Future<Output = Result<Uuid, String>> + Send {
        self.0.land_dojo_inbox_memory(inbox_id, mem)
    }
    fn set_note(
        &self,
        inbox_id: Uuid,
        note: &str,
    ) -> impl Future<Output = Result<(), String>> + Send {
        self.0.set_dojo_inbox_note(inbox_id, note.to_string())
    }
    fn set_state(
        &self,
        inbox_id: Uuid,
        state: &str,
    ) -> impl Future<Output = Result<bool, String>> + Send {
        let state = state.to_string();
        async move { self.0.set_dojo_inbox_state(inbox_id, &state).await }
    }
}

/// Pull one membership's downstream inbox with the live seams. Used by the
/// federation pull loop.
pub async fn pull_membership(
    store: &PgStore,
    membership: &DojoMembership,
) -> Result<PullOutcome, String> {
    pull_membership_inbox(&LivePuller, &PgInbox(store), membership).await
}

/// Apply an inbox item with the live seam (`POST /api/upgrades/{id}/apply`).
pub async fn apply(store: &PgStore, inbox_id: Uuid) -> Result<ApplyOutcome, String> {
    apply_inbox_item(&PgInbox(store), inbox_id).await
}

/// Mute an inbox item (`POST /api/upgrades/{id}/mute`). `Ok(false)` = unknown id.
pub async fn mute(store: &PgStore, inbox_id: Uuid) -> Result<bool, String> {
    PgInbox(store).set_state(inbox_id, "muted").await
}

/// Pin an inbox item (`POST /api/upgrades/{id}/pin`). `Ok(false)` = unknown id.
pub async fn pin(store: &PgStore, inbox_id: Uuid) -> Result<bool, String> {
    PgInbox(store).set_state(inbox_id, "pinned").await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use dojo_protocol::{
        ArtifactPayload, AttributionMode, PatternFamily, PatternPayload, PrinciplePayload,
        PublishedArtifact, artifact_signature,
    };

    // ── fixtures ──────────────────────────────────────────────────────────────

    fn membership() -> DojoMembership {
        DojoMembership {
            id: Uuid::new_v4(),
            registry_url: "http://localhost:7755".into(),
            tenant_key: "github/acme".into(),
            dojo_url: "http://localhost:7755/github/acme".into(),
            kind: "community".into(),
            org_slugs: vec![],
            role: "contributor".into(),
            authenticated_via: "device_code".into(),
            attribution_default: "anonymous".into(),
            credential_ref: "cref".into(),
            sync_status: "healthy".into(),
            last_seq: 0,
            last_heartbeat_at: None,
            enabled: true,
        }
    }

    fn attribution() -> Attribution {
        Attribution {
            mode: AttributionMode::Anonymous,
            author: None,
            org: None,
            anonymous_id: Some("anon-1".into()),
        }
    }

    fn artifact(
        kind: ArtifactKind,
        title: &str,
        body: &str,
        scope: ArtifactScope,
    ) -> PublishedArtifact {
        let payload = match kind {
            ArtifactKind::Pattern => ArtifactPayload::Pattern(PatternPayload {
                family: PatternFamily::Design,
                pattern_id: None,
                ftr_delta_observed: None,
            }),
            ArtifactKind::Skill => ArtifactPayload::Skill(dojo_protocol::SkillPayload {
                name: "s".into(),
                description: None,
                manifest: serde_json::Value::Null,
            }),
            ArtifactKind::Agent => ArtifactPayload::Agent(dojo_protocol::AgentPayload {
                name: "a".into(),
                description: None,
                tools: vec![],
                agent_scope: None,
            }),
            ArtifactKind::Prompt => ArtifactPayload::Prompt(dojo_protocol::PromptPayload {
                prompt_kind: dojo_protocol::PromptKind::Template,
            }),
            ArtifactKind::Guard => ArtifactPayload::Guard(dojo_protocol::GuardPayload {
                check: "no unwrap".into(),
                tool: None,
            }),
            _ => ArtifactPayload::Principle(PrinciplePayload { rationale: None }),
        };
        PublishedArtifact {
            signature: artifact_signature(kind, title, body, None, &payload),
            tenant_key: "github/acme".into(),
            engagement_id: None,
            kind,
            title: title.into(),
            body: body.into(),
            example: None,
            payload,
            scope,
            attribution: attribution(),
            contributed_by: None,
            published_at: None,
        }
    }

    fn pulled(seq: i64, status: ArtifactStatus, art: PublishedArtifact) -> PulledArtifact {
        PulledArtifact { id: format!("art-{seq}"), seq, status, artifact: art }
    }

    // ── mock puller ───────────────────────────────────────────────────────────

    struct MockPuller {
        page: ArtifactPullResponse,
    }
    impl DojoPuller for MockPuller {
        async fn pull(
            &self,
            _m: &DojoMembership,
            _since: i64,
        ) -> Result<ArtifactPullResponse, DojoClientError> {
            Ok(self.page.clone())
        }
    }

    struct ErrPuller;
    impl DojoPuller for ErrPuller {
        async fn pull(
            &self,
            _m: &DojoMembership,
            _since: i64,
        ) -> Result<ArtifactPullResponse, DojoClientError> {
            Err(DojoClientError::Status(503))
        }
    }

    // ── mock inbox ────────────────────────────────────────────────────────────

    #[derive(Default)]
    struct MemInbox {
        rows: Mutex<Vec<InboxItem>>,
        cursors: Mutex<std::collections::HashMap<Uuid, i64>>,
        landed: Mutex<Vec<InsertMemory>>,
        projects: std::collections::HashMap<String, Uuid>,
    }

    impl MemInbox {
        fn rows(&self) -> Vec<InboxItem> {
            self.rows.lock().unwrap().clone()
        }
        fn landed(&self) -> Vec<InsertMemory> {
            self.landed.lock().unwrap().clone()
        }
        fn cursor(&self, m: Uuid) -> Option<i64> {
            self.cursors.lock().unwrap().get(&m).copied()
        }
    }

    impl Inbox for MemInbox {
        fn upsert_pending(
            &self,
            row: &InboxRow,
        ) -> impl Future<Output = Result<bool, String>> + Send {
            let mut rows = self.rows.lock().unwrap();
            let dup = rows.iter().any(|r| {
                r.membership_id == row.membership_id && r.artifact_signature == row.signature
            });
            let inserted = if dup {
                false
            } else {
                rows.push(InboxItem {
                    id: Uuid::new_v4(),
                    membership_id: row.membership_id,
                    kind: row.kind.clone(),
                    title: row.title.clone(),
                    body: row.body.clone(),
                    scope: row.scope.clone(),
                    attribution: row.attribution.clone(),
                    state: "pending".into(),
                    note: None,
                    applied_memory_id: None,
                    received_at: None,
                    artifact_signature: row.signature.clone(),
                });
                true
            };
            async move { Ok(inserted) }
        }
        fn set_cursor(
            &self,
            membership_id: Uuid,
            cursor: i64,
        ) -> impl Future<Output = Result<(), String>> + Send {
            self.cursors.lock().unwrap().insert(membership_id, cursor);
            async move { Ok(()) }
        }
        fn get(
            &self,
            inbox_id: Uuid,
        ) -> impl Future<Output = Result<Option<InboxItem>, String>> + Send {
            let hit = self.rows.lock().unwrap().iter().find(|r| r.id == inbox_id).cloned();
            async move { Ok(hit) }
        }
        fn resolve_project(
            &self,
            name: &str,
        ) -> impl Future<Output = Result<Option<Uuid>, String>> + Send {
            let hit = self.projects.get(name).copied();
            async move { Ok(hit) }
        }
        fn land_and_apply(
            &self,
            inbox_id: Uuid,
            mem: &InsertMemory,
        ) -> impl Future<Output = Result<Uuid, String>> + Send {
            self.landed.lock().unwrap().push(mem.clone());
            let memory_id = Uuid::new_v4();
            {
                let mut rows = self.rows.lock().unwrap();
                if let Some(r) = rows.iter_mut().find(|r| r.id == inbox_id) {
                    r.state = "applied".into();
                    r.applied_memory_id = Some(memory_id);
                }
            }
            async move { Ok(memory_id) }
        }
        fn set_note(
            &self,
            inbox_id: Uuid,
            note: &str,
        ) -> impl Future<Output = Result<(), String>> + Send {
            if let Some(r) = self.rows.lock().unwrap().iter_mut().find(|r| r.id == inbox_id) {
                r.note = Some(note.to_string());
            }
            async move { Ok(()) }
        }
        fn set_state(
            &self,
            inbox_id: Uuid,
            state: &str,
        ) -> impl Future<Output = Result<bool, String>> + Send {
            let mut found = false;
            if let Some(r) = self.rows.lock().unwrap().iter_mut().find(|r| r.id == inbox_id) {
                r.state = state.to_string();
                found = true;
            }
            async move { Ok(found) }
        }
    }

    fn first_id(inbox: &MemInbox) -> Uuid {
        inbox.rows()[0].id
    }

    // ── pull: upsert pending + advance cursor + idempotent ────────────────────

    #[tokio::test]
    async fn pull_upserts_pending_advances_cursor_and_is_idempotent_on_repull() {
        let m = membership();
        let page = ArtifactPullResponse {
            artifacts: vec![
                pulled(
                    1,
                    ArtifactStatus::Published,
                    artifact(
                        ArtifactKind::Principle,
                        "small fns",
                        "keep them small",
                        ArtifactScope::default(),
                    ),
                ),
                pulled(
                    2,
                    ArtifactStatus::Published,
                    artifact(
                        ArtifactKind::Pattern,
                        "adapter",
                        "one per lang",
                        ArtifactScope { stack: Some("rust".into()), ..Default::default() },
                    ),
                ),
            ],
            cursor: 2,
        };
        let puller = MockPuller { page };
        let inbox = MemInbox::default();

        let out = pull_membership_inbox(&puller, &inbox, &m).await.unwrap();
        assert_eq!(out.pulled, 2);
        assert_eq!(out.inserted, 2);
        assert_eq!(out.skipped, 0);
        assert_eq!(out.new_cursor, 2);
        assert_eq!(inbox.cursor(m.id), Some(2));
        assert_eq!(inbox.rows().len(), 2);
        assert!(inbox.rows().iter().all(|r| r.state == "pending"));

        // Re-pull the SAME page (dedup by signature) — nothing new, no duplicates.
        let out2 = pull_membership_inbox(&puller, &inbox, &m).await.unwrap();
        assert_eq!(out2.inserted, 0);
        assert_eq!(out2.skipped, 2);
        assert_eq!(inbox.rows().len(), 2, "a re-pull must not duplicate rows");
    }

    #[tokio::test]
    async fn pull_skips_non_published_artifacts() {
        let m = membership();
        let page = ArtifactPullResponse {
            artifacts: vec![
                pulled(
                    1,
                    ArtifactStatus::Published,
                    artifact(ArtifactKind::Principle, "a", "b", ArtifactScope::default()),
                ),
                pulled(
                    2,
                    ArtifactStatus::Submitted,
                    artifact(ArtifactKind::Principle, "c", "d", ArtifactScope::default()),
                ),
                pulled(
                    3,
                    ArtifactStatus::Archived,
                    artifact(ArtifactKind::Principle, "e", "f", ArtifactScope::default()),
                ),
            ],
            cursor: 3,
        };
        let inbox = MemInbox::default();
        let out = pull_membership_inbox(&MockPuller { page }, &inbox, &m).await.unwrap();
        assert_eq!(out.inserted, 1);
        assert_eq!(out.skipped, 2);
        assert_eq!(inbox.rows().len(), 1);
    }

    #[tokio::test]
    async fn pull_error_propagates_for_the_loop_to_guard() {
        let m = membership();
        let inbox = MemInbox::default();
        let err = pull_membership_inbox(&ErrPuller, &inbox, &m).await.unwrap_err();
        assert!(err.contains("503"), "the client error surfaces: {err}");
        assert!(inbox.rows().is_empty());
    }

    // ── apply: principle → a dojo memory ──────────────────────────────────────

    #[tokio::test]
    async fn apply_principle_lands_a_dojo_memory_and_is_idempotent() {
        let m = membership();
        let page = ArtifactPullResponse {
            artifacts: vec![pulled(
                1,
                ArtifactStatus::Published,
                artifact(
                    ArtifactKind::Principle,
                    "prefer small fns",
                    "keep units testable",
                    ArtifactScope::default(),
                ),
            )],
            cursor: 1,
        };
        let inbox = MemInbox::default();
        pull_membership_inbox(&MockPuller { page }, &inbox, &m).await.unwrap();
        let id = first_id(&inbox);

        let out = apply_inbox_item(&inbox, id).await.unwrap();
        let memory_id = match out {
            ApplyOutcome::Landed { memory_id } => memory_id,
            other => panic!("expected Landed, got {other:?}"),
        };
        // Exactly one memory, origin='dojo', global scope, convention type.
        let landed = inbox.landed();
        assert_eq!(landed.len(), 1);
        assert_eq!(landed[0].origin.as_deref(), Some("dojo"));
        assert_eq!(landed[0].scope, "global");
        assert_eq!(landed[0].mtype, "convention");
        assert_eq!(landed[0].title, "prefer small fns");
        assert!(landed[0].tags.iter().any(|t| t == "dojo"));
        // The inbox row flipped to applied + linked to the memory.
        let row = inbox.rows().into_iter().find(|r| r.id == id).unwrap();
        assert_eq!(row.state, "applied");
        assert_eq!(row.applied_memory_id, Some(memory_id));

        // Idempotent: re-apply is a no-op returning the same memory, no 2nd write.
        let again = apply_inbox_item(&inbox, id).await.unwrap();
        assert_eq!(again, ApplyOutcome::AlreadyApplied { memory_id });
        assert_eq!(inbox.landed().len(), 1, "re-apply must not write a second memory");
    }

    #[tokio::test]
    async fn apply_pattern_maps_to_pattern_memory_at_stack_scope() {
        let m = membership();
        let scope = ArtifactScope { stack: Some("rust".into()), ..Default::default() };
        let page = ArtifactPullResponse {
            artifacts: vec![pulled(
                1,
                ArtifactStatus::Published,
                artifact(
                    ArtifactKind::Pattern,
                    "adapter per lang",
                    "one adapter per language",
                    scope,
                ),
            )],
            cursor: 1,
        };
        let inbox = MemInbox::default();
        pull_membership_inbox(&MockPuller { page }, &inbox, &m).await.unwrap();
        let id = first_id(&inbox);

        assert!(matches!(apply_inbox_item(&inbox, id).await.unwrap(), ApplyOutcome::Landed { .. }));
        let landed = inbox.landed();
        assert_eq!(landed[0].mtype, "pattern");
        assert_eq!(landed[0].scope, "stack");
        assert_eq!(landed[0].scope_filter.as_deref(), Some("rust"));
    }

    // ── apply: skill/agent/prompt/guard → deferred, nothing written ───────────

    #[tokio::test]
    async fn apply_deferred_kinds_write_nothing_and_stay_pending() {
        for kind in
            [ArtifactKind::Skill, ArtifactKind::Agent, ArtifactKind::Prompt, ArtifactKind::Guard]
        {
            let m = membership();
            let page = ArtifactPullResponse {
                artifacts: vec![pulled(
                    1,
                    ArtifactStatus::Published,
                    artifact(kind, "x", "y", ArtifactScope::default()),
                )],
                cursor: 1,
            };
            let inbox = MemInbox::default();
            pull_membership_inbox(&MockPuller { page }, &inbox, &m).await.unwrap();
            let id = first_id(&inbox);

            let out = apply_inbox_item(&inbox, id).await.unwrap();
            assert_eq!(out, ApplyOutcome::Deferred { kind }, "{kind:?} must defer");
            assert!(inbox.landed().is_empty(), "{kind:?} apply must write nothing");
            let row = inbox.rows().into_iter().find(|r| r.id == id).unwrap();
            assert_eq!(row.state, "pending", "{kind:?} stays pending");
            assert!(row.note.is_some(), "{kind:?} records a deferred note");
        }
    }

    // ── apply: scope mismatch returns a reason, lands nothing ─────────────────

    #[tokio::test]
    async fn apply_project_scope_mismatch_returns_reason_and_lands_nothing() {
        let m = membership();
        let scope = ArtifactScope { project: Some("Ghost Project".into()), ..Default::default() };
        let page = ArtifactPullResponse {
            artifacts: vec![pulled(
                1,
                ArtifactStatus::Published,
                artifact(ArtifactKind::Principle, "p", "b", scope),
            )],
            cursor: 1,
        };
        let inbox = MemInbox::default(); // no projects registered
        pull_membership_inbox(&MockPuller { page }, &inbox, &m).await.unwrap();
        let id = first_id(&inbox);

        let out = apply_inbox_item(&inbox, id).await.unwrap();
        match out {
            ApplyOutcome::ScopeMismatch { reason } => assert!(reason.contains("Ghost Project")),
            other => panic!("expected ScopeMismatch, got {other:?}"),
        }
        assert!(inbox.landed().is_empty(), "a scope mismatch must land nothing");
        let row = inbox.rows().into_iter().find(|r| r.id == id).unwrap();
        assert_eq!(row.state, "pending");
        assert!(row.note.is_some());
    }

    #[tokio::test]
    async fn apply_project_scope_lands_when_project_is_present() {
        let m = membership();
        let scope = ArtifactScope { project: Some("Known".into()), ..Default::default() };
        let page = ArtifactPullResponse {
            artifacts: vec![pulled(
                1,
                ArtifactStatus::Published,
                artifact(ArtifactKind::Principle, "p", "b", scope),
            )],
            cursor: 1,
        };
        let pid = Uuid::new_v4();
        let mut inbox = MemInbox::default();
        inbox.projects.insert("Known".into(), pid);
        pull_membership_inbox(&MockPuller { page }, &inbox, &m).await.unwrap();
        let id = first_id(&inbox);

        assert!(matches!(apply_inbox_item(&inbox, id).await.unwrap(), ApplyOutcome::Landed { .. }));
        let landed = inbox.landed();
        assert_eq!(landed[0].scope, "project");
        assert_eq!(landed[0].project_id, Some(pid));
    }

    #[tokio::test]
    async fn apply_unknown_id_is_not_found() {
        let inbox = MemInbox::default();
        assert_eq!(apply_inbox_item(&inbox, Uuid::new_v4()).await.unwrap(), ApplyOutcome::NotFound);
    }

    // ── mute hides / pin floats ───────────────────────────────────────────────

    fn item(state: &str, received_at: &str) -> InboxItem {
        InboxItem {
            id: Uuid::new_v4(),
            membership_id: Uuid::new_v4(),
            kind: "principle".into(),
            title: "t".into(),
            body: "b".into(),
            scope: ArtifactScope::default(),
            attribution: attribution(),
            state: state.into(),
            note: None,
            applied_memory_id: None,
            received_at: Some(received_at.into()),
            artifact_signature: "sig".into(),
        }
    }

    #[test]
    fn order_and_filter_hides_muted_and_floats_pinned() {
        let items = vec![
            item("pending", "2026-07-01T00:00:00Z"),
            item("muted", "2026-07-02T00:00:00Z"),
            item("pinned", "2026-06-01T00:00:00Z"), // oldest, but pinned
            item("pending", "2026-07-03T00:00:00Z"),
        ];
        // Default: muted hidden, pinned first, then newest.
        let default = order_and_filter(items.clone(), false);
        assert_eq!(default.len(), 3, "muted is hidden by default");
        assert!(default.iter().all(|i| i.state != "muted"));
        assert_eq!(
            default[0].state, "pinned",
            "pinned floats to the top even though it is the oldest"
        );
        assert_eq!(
            default[1].received_at.as_deref(),
            Some("2026-07-03T00:00:00Z"),
            "then newest-first"
        );

        // include_muted surfaces it again.
        let all = order_and_filter(items, true);
        assert_eq!(all.len(), 4);
        assert!(all.iter().any(|i| i.state == "muted"));
    }

    #[tokio::test]
    async fn mute_and_pin_set_state_and_never_land() {
        let m = membership();
        let page = ArtifactPullResponse {
            artifacts: vec![
                pulled(
                    1,
                    ArtifactStatus::Published,
                    artifact(ArtifactKind::Principle, "a", "b", ArtifactScope::default()),
                ),
                pulled(
                    2,
                    ArtifactStatus::Published,
                    artifact(ArtifactKind::Principle, "c", "d", ArtifactScope::default()),
                ),
            ],
            cursor: 2,
        };
        let inbox = MemInbox::default();
        pull_membership_inbox(&MockPuller { page }, &inbox, &m).await.unwrap();
        let ids: Vec<Uuid> = inbox.rows().iter().map(|r| r.id).collect();

        inbox.set_state(ids[0], "muted").await.unwrap();
        inbox.set_state(ids[1], "pinned").await.unwrap();
        assert!(inbox.landed().is_empty(), "mute/pin never land anything");

        let listed = order_and_filter(inbox.rows(), false);
        assert!(
            listed.iter().all(|i| i.id != ids[0]),
            "muted item is hidden from the default list"
        );
        assert_eq!(listed[0].id, ids[1], "pinned item floats to the top");
    }

    #[test]
    fn unread_count_counts_only_pending() {
        let items = vec![
            item("pending", "2026-07-01T00:00:00Z"),
            item("pinned", "2026-07-02T00:00:00Z"),
            item("applied", "2026-07-03T00:00:00Z"),
            item("pending", "2026-07-04T00:00:00Z"),
        ];
        assert_eq!(unread_count(&items), 2);
    }

    // ── pure mapping helpers ──────────────────────────────────────────────────

    #[test]
    fn memory_type_mapping_is_the_inverse_of_contribute() {
        assert_eq!(memory_type_for(ArtifactKind::Pattern), "pattern");
        assert_eq!(memory_type_for(ArtifactKind::Principle), "convention");
    }

    #[test]
    fn scope_target_prefers_project_then_stack_then_global() {
        assert!(matches!(scope_target(&ArtifactScope::default()), ScopeTarget::Global));
        assert!(matches!(
            scope_target(&ArtifactScope { stack: Some("rust".into()), ..Default::default() }),
            ScopeTarget::Stack(s) if s == "rust"
        ));
        assert!(matches!(
            scope_target(&ArtifactScope { project: Some("P".into()), stack: Some("rust".into()), ..Default::default() }),
            ScopeTarget::Project { name } if name == "P"
        ));
        // company/team without project/stack → global (org-wide practice).
        assert!(matches!(
            scope_target(&ArtifactScope {
                company: Some("acme".into()),
                team: Some("mobile".into()),
                ..Default::default()
            }),
            ScopeTarget::Global
        ));
    }
}
