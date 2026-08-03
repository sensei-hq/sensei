//! Relay wire contract — the filtered status + segment/gate/reply shapes the
//! daemon publishes to (and reads back from) the Dōjō Worker `/v1/relay/*`, and
//! that the phone/console render.
//!
//! **Zero-knowledge:** only filtered *logical* status + gate prompts + replies
//! cross — never code, diffs, or transcripts (relay-engine.md D10). The
//! `RelayInboxItem::payload` carries a stripped prompt + the rokkit form schema,
//! already confidentiality-checked upstream (like the artifact contract, this seam
//! performs no stripping).
//!
//! Pure serde wire types, no daemon/DB deps. Enum strings MUST match the `dojo.*`
//! relay enum DDL exactly — the `*_db_strings_match_ddl` tests enforce it.

use serde::{Deserialize, Serialize};

/// Cloud-side status of a run, mirrored from the daemon-local `activity.runs`.
/// Matches `dojo.relay_run_status` (and the daemon-local `sensei.run_status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayRunStatus {
    /// Advancing.
    Running,
    /// Intentional pause (rate/weekly limit) — carries a resume time.
    Paused,
    /// No progress within the expected window — auto-diagnosed.
    Stalled,
    /// Unexpected death (distinct from `Failed`).
    Crashed,
    /// Waiting on a hard-block gate.
    Blocked,
    /// Terminal success.
    Done,
    /// Terminal failure verdict.
    Failed,
}

impl RelayRunStatus {
    /// Every variant, in DDL declaration order.
    pub const ALL: [RelayRunStatus; 7] = [
        RelayRunStatus::Running,
        RelayRunStatus::Paused,
        RelayRunStatus::Stalled,
        RelayRunStatus::Crashed,
        RelayRunStatus::Blocked,
        RelayRunStatus::Done,
        RelayRunStatus::Failed,
    ];

    /// The `dojo.relay_run_status` enum string. MUST match `relay_run_status.ddl`.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            RelayRunStatus::Running => "running",
            RelayRunStatus::Paused => "paused",
            RelayRunStatus::Stalled => "stalled",
            RelayRunStatus::Crashed => "crashed",
            RelayRunStatus::Blocked => "blocked",
            RelayRunStatus::Done => "done",
            RelayRunStatus::Failed => "failed",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        RelayRunStatus::ALL.into_iter().find(|v| v.as_db_str() == s)
    }

    /// Terminal states carry no further progress.
    pub fn is_terminal(&self) -> bool {
        matches!(self, RelayRunStatus::Done | RelayRunStatus::Failed)
    }

    /// States that need the human (drives the "needs you" band).
    pub fn needs_attention(&self) -> bool {
        matches!(
            self,
            RelayRunStatus::Blocked | RelayRunStatus::Stalled | RelayRunStatus::Crashed
        )
    }
}

/// State of a segment (a Phase or, nested, a Step). Matches `dojo.segment_state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentState {
    Pending,
    Active,
    Done,
    Skipped,
    Failed,
    Blocked,
    NeedsReview,
}

impl SegmentState {
    pub const ALL: [SegmentState; 7] = [
        SegmentState::Pending,
        SegmentState::Active,
        SegmentState::Done,
        SegmentState::Skipped,
        SegmentState::Failed,
        SegmentState::Blocked,
        SegmentState::NeedsReview,
    ];

    /// MUST match `segment_state.ddl`.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            SegmentState::Pending => "pending",
            SegmentState::Active => "active",
            SegmentState::Done => "done",
            SegmentState::Skipped => "skipped",
            SegmentState::Failed => "failed",
            SegmentState::Blocked => "blocked",
            SegmentState::NeedsReview => "needs_review",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        SegmentState::ALL.into_iter().find(|v| v.as_db_str() == s)
    }
}

/// How a gate interrupts the run. Matches `dojo.gate_severity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateSeverity {
    /// Hard-block — halts the unit until answered.
    Blocking,
    /// Async review — logged, never halts.
    Advisory,
}

impl GateSeverity {
    pub const ALL: [GateSeverity; 2] = [GateSeverity::Blocking, GateSeverity::Advisory];

    /// MUST match `gate_severity.ddl`.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            GateSeverity::Blocking => "blocking",
            GateSeverity::Advisory => "advisory",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        GateSeverity::ALL.into_iter().find(|v| v.as_db_str() == s)
    }
}

/// What a Relay inbox row is asking for. Matches `dojo.relay_inbox_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayInboxKind {
    Approval,
    Decision,
    Chat,
    Nudge,
    Stall,
}

impl RelayInboxKind {
    pub const ALL: [RelayInboxKind; 5] = [
        RelayInboxKind::Approval,
        RelayInboxKind::Decision,
        RelayInboxKind::Chat,
        RelayInboxKind::Nudge,
        RelayInboxKind::Stall,
    ];

    /// MUST match `relay_inbox_kind.ddl`.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            RelayInboxKind::Approval => "approval",
            RelayInboxKind::Decision => "decision",
            RelayInboxKind::Chat => "chat",
            RelayInboxKind::Nudge => "nudge",
            RelayInboxKind::Stall => "stall",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        RelayInboxKind::ALL.into_iter().find(|v| v.as_db_str() == s)
    }
}

/// Which way an inbox row flows. Matches `dojo.relay_message_direction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayMessageDirection {
    /// The daemon asking (gate/decision/stall).
    AgentToHuman,
    /// The person answering or steering (reply/nudge/chat).
    HumanToAgent,
}

impl RelayMessageDirection {
    pub const ALL: [RelayMessageDirection; 2] = [
        RelayMessageDirection::AgentToHuman,
        RelayMessageDirection::HumanToAgent,
    ];

    /// MUST match `relay_message_direction.ddl`.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            RelayMessageDirection::AgentToHuman => "agent_to_human",
            RelayMessageDirection::HumanToAgent => "human_to_agent",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        RelayMessageDirection::ALL.into_iter().find(|v| v.as_db_str() == s)
    }
}

/// Lifecycle of an inbox row. Matches `dojo.relay_inbox_status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayInboxStatus {
    Pending,
    Answered,
    Expired,
    Superseded,
}

impl RelayInboxStatus {
    pub const ALL: [RelayInboxStatus; 4] = [
        RelayInboxStatus::Pending,
        RelayInboxStatus::Answered,
        RelayInboxStatus::Expired,
        RelayInboxStatus::Superseded,
    ];

    /// MUST match `relay_inbox_status.ddl`.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            RelayInboxStatus::Pending => "pending",
            RelayInboxStatus::Answered => "answered",
            RelayInboxStatus::Expired => "expired",
            RelayInboxStatus::Superseded => "superseded",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        RelayInboxStatus::ALL.into_iter().find(|v| v.as_db_str() == s)
    }
}

// ---------------------------------------------------------------------------
// Wire structs — the filtered status the daemon publishes and the phone reads.
// ---------------------------------------------------------------------------

/// The filtered status snapshot the daemon publishes for a run — mirrors
/// `dojo.relay_sessions` (cloud). `run_id` is the daemon-local `activity.runs` id
/// (a plain uuid across the DB boundary — no cross-DB FK). Logical fields only:
/// no code, no diffs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelaySessionUpdate {
    pub run_id: String,
    pub status: RelayRunStatus,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    pub progress_done: i32,
    pub progress_total: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_feature: Option<String>,
    /// RFC 3339 — newest run_event ts (drives "last progress N min ago").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_event_at: Option<String>,
    /// RFC 3339 — when a limit pause auto-resumes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paused_until: Option<String>,
    /// Why the run is paused (e.g. "rate limit", "weekly cap").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pause_reason: Option<String>,
    /// RFC 3339 — the run's liveness heartbeat (`activity.runs.heartbeat_at`).
    /// Mirrors `dojo.relay_sessions.heartbeat_at`. The phone/console badges
    /// staleness from this instant's age (no update in ~5 min = stale), and it
    /// backs the watchdog's `stalled` status the bridge also publishes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heartbeat_at: Option<String>,
    /// The slug of the run's project namespace (`sensei.namespaces` scope=project).
    /// Lets the Worker open/refresh the caller's billing seat on this project —
    /// proof the user is actively using sensei there (only such users are billed).
    /// Optional + backward-compatible: absent → no seat is touched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_slug: Option<String>,
    /// The run's project as the dōjō models it (`dojo.projects`) — populated so the
    /// user sees their own projects in their own RLS-scoped console. Distinct from
    /// `project_slug` above (which stays the raw billing seat key): this is the
    /// display row. Still the user's own project metadata, federated as-is — there
    /// is no client-specific dereference (content is dereferenced at derivation;
    /// project identity is not derived content). Optional + backward-compatible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<RelayProjectInfo>,
}

/// The run's project for the dōjō's `dojo.projects` upsert. `slug`/`name` are the
/// user's own project metadata (the namespace slug + `sensei.projects.name`),
/// `classification` = company|client|personal|community (from the bound membership
/// kind; unbound → personal), `phase` = the adoption phase (the daemon sends the
/// `watch` default; the dōjō advances it later).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayProjectInfo {
    pub slug: String,
    pub name: String,
    pub classification: String,
    pub phase: String,
    /// The project's resolved constitution (the daemon composes it locally from
    /// the folder's namespaces + packs). Federated so the dōjō can DISPLAY the
    /// "before you start" preview without re-resolving. Optional + backward-
    /// compatible: absent → the dōjō shows its "resolves in your editor" state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constitution: Option<RelayConstitution>,
}

/// A project's resolved governing constitution, composed by the daemon. The
/// AUTHORITY resolution — which rules apply, dedup, mandatory locks, the discards
/// the ladder made — is server-side (resolved design Q2/Q3); the dōjō only maps
/// each rule's `scope_key`/`namespace` to a display rung and renders. `rules` are
/// the effective set, strongest-first; `conflicts` are the discards (a weaker
/// duplicate that lost to a higher-authority scope); `locks` counts the ★
/// (mandatory) rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayConstitution {
    pub rules: Vec<RelayConstitutionRule>,
    #[serde(default)]
    pub conflicts: Vec<RelayConstitutionConflict>,
    pub locks: u32,
}

/// One effective rule on a project's constitution — carried in the daemon's own
/// resolution vocabulary (`scope_key` from `sensei.scopes`, the rule `title`, its
/// `enforcement`, and the owning `namespace` name). The dōjō maps `scope_key` to
/// a ladder rung + glyph for display (it does not re-resolve).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayConstitutionRule {
    pub scope_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    pub title: String,
    pub enforcement: String,
}

/// A rule the ladder discarded: a weaker-authority duplicate (`loser_scope`) that
/// lost to the same rule stated at a higher-authority scope (`winner_scope`).
/// `topic` is the shared rule text; `locked` = the winner is mandatory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayConstitutionConflict {
    pub topic: String,
    pub loser_scope: String,
    pub winner_scope: String,
    pub why: String,
    pub locked: bool,
}

/// One node of the phone-renderable outline — mirrors `dojo.relay_segments`. A
/// top-level segment (`parent_id` = None) is a **Phase**; a nested one is a
/// **Step**. `summary` is the one-line, diff-free feed text; `detail` is the
/// expandable body. `id` is server-assigned (None on create).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelaySegment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// The `seq` of this node's parent phase, when authored (publish-time only —
    /// the daemon can't set `parent_id` because the Worker assigns ids on upsert).
    /// The Worker resolves `parent_id` from this after the `(session_id, seq)` upsert
    /// so Phase/Step nesting survives. None for top-level nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_seq: Option<i32>,
    pub seq: i32,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub state: SegmentState,
    #[serde(default)]
    pub is_gate: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_severity: Option<GateSeverity>,
    /// approve | request_changes | comment (None until reviewed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_verdict: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_note: Option<String>,
    /// The subagent role this task is dispatched to — plan-authored, label only.
    /// Set only for segments authored from a registered plan; None otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// The model assigned to this task — plan-authored, label only. Recorded for
    /// Dōjō visibility + phase-2 routing; phase-1 execution runs Claude subagents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Path#anchor to the task's detailed spec — a reference, never the body
    /// (zero-knowledge D10). Plan-authored; None otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_ref: Option<String>,
}

/// A durable live-interaction row — mirrors `dojo.relay_inbox`. `payload` is the
/// stripped prompt + rokkit form schema (options/fields); `reply` is the human's
/// response, consumed by the daemon to continue the run. Never carries code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayInboxItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The run (activity.runs id) this row belongs to.
    pub run_id: String,
    /// The outline segment this gates/annotates (None for a free-standing chat/nudge).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_id: Option<String>,
    pub kind: RelayInboxKind,
    pub direction: RelayMessageDirection,
    #[serde(default = "default_pending")]
    pub status: RelayInboxStatus,
    /// Stripped prompt + rokkit form schema. No code, no diffs. Defaults to an
    /// empty object (matching the DDL `jsonb not null default '{}'`) — never null.
    #[serde(default = "empty_object")]
    pub payload: serde_json::Value,
    /// The human's response (verdict / chosen option / free text).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered_at: Option<String>,
}

fn default_pending() -> RelayInboxStatus {
    RelayInboxStatus::Pending
}

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}

/// The phone's answer to an inbox row — `POST /v1/relay/reply`. The daemon polls
/// for answered rows and consumes the `reply` to continue the held run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayReply {
    pub inbox_id: String,
    /// verdict / chosen option / free text — shape depends on the row's kind.
    pub reply: serde_json::Value,
}

/// The Worker's ack for a raised inbox row — the response body of
/// `POST relay/inbox`. `id` is the server-assigned `dojo.relay_inbox.id`
/// (the handle the daemon awaits a reply against); `seq` is the row's monotonic
/// sequence (the cursor from which `poll_inbox` should scan for the answer).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayInboxAck {
    pub id: String,
    pub seq: i64,
}

/// The Worker's ack for a published status snapshot — the response body of
/// `POST relay/session`. `id` is the upserted `dojo.relay_sessions.id` (the cloud
/// session this run mirrors); the daemon persists it into
/// `activity.runs.dojo_session_id` so the local run joins to its relay session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelaySessionAck {
    pub id: String,
}

/// Response to the daemon's inbox poll — the answered/pending rows since a cursor
/// plus the new cursor to persist. Mirrors the artifact contract's pull shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayInboxPull {
    pub items: Vec<RelayInboxItem>,
    pub cursor: i64,
}

/// The daemon's outline publish for a run — `POST relay/segments`. Carries the
/// run's segments as a batch; the Worker maps `run_id` → the cloud session (like
/// the inbox publish) and upserts each segment by (session, seq). Re-publishing an
/// updated outline is idempotent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelaySegmentsPublish {
    pub run_id: String,
    pub segments: Vec<RelaySegment>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- enum <-> DB string mapping (MUST match the dojo.* relay enum DDL) ----

    #[test]
    fn relay_run_status_db_strings_match_ddl() {
        let expected = ["running", "paused", "stalled", "crashed", "blocked", "done", "failed"];
        let got: Vec<&str> = RelayRunStatus::ALL.iter().map(|v| v.as_db_str()).collect();
        assert_eq!(got, expected);
        for v in RelayRunStatus::ALL {
            assert_eq!(RelayRunStatus::from_db_str(v.as_db_str()), Some(v));
            assert_eq!(serde_json::to_string(&v).unwrap(), format!("\"{}\"", v.as_db_str()));
        }
        assert_eq!(RelayRunStatus::from_db_str("nope"), None);
    }

    #[test]
    fn segment_state_db_strings_match_ddl() {
        let expected = ["pending", "active", "done", "skipped", "failed", "blocked", "needs_review"];
        let got: Vec<&str> = SegmentState::ALL.iter().map(|v| v.as_db_str()).collect();
        assert_eq!(got, expected);
        for v in SegmentState::ALL {
            assert_eq!(SegmentState::from_db_str(v.as_db_str()), Some(v));
            assert_eq!(serde_json::to_string(&v).unwrap(), format!("\"{}\"", v.as_db_str()));
        }
    }

    #[test]
    fn gate_severity_db_strings_match_ddl() {
        let expected = ["blocking", "advisory"];
        let got: Vec<&str> = GateSeverity::ALL.iter().map(|v| v.as_db_str()).collect();
        assert_eq!(got, expected);
        for v in GateSeverity::ALL {
            assert_eq!(GateSeverity::from_db_str(v.as_db_str()), Some(v));
            assert_eq!(serde_json::to_string(&v).unwrap(), format!("\"{}\"", v.as_db_str()));
        }
    }

    #[test]
    fn relay_inbox_kind_db_strings_match_ddl() {
        let expected = ["approval", "decision", "chat", "nudge", "stall"];
        let got: Vec<&str> = RelayInboxKind::ALL.iter().map(|v| v.as_db_str()).collect();
        assert_eq!(got, expected);
        for v in RelayInboxKind::ALL {
            assert_eq!(RelayInboxKind::from_db_str(v.as_db_str()), Some(v));
            assert_eq!(serde_json::to_string(&v).unwrap(), format!("\"{}\"", v.as_db_str()));
        }
    }

    #[test]
    fn relay_message_direction_db_strings_match_ddl() {
        let expected = ["agent_to_human", "human_to_agent"];
        let got: Vec<&str> = RelayMessageDirection::ALL.iter().map(|v| v.as_db_str()).collect();
        assert_eq!(got, expected);
        for v in RelayMessageDirection::ALL {
            assert_eq!(RelayMessageDirection::from_db_str(v.as_db_str()), Some(v));
            assert_eq!(serde_json::to_string(&v).unwrap(), format!("\"{}\"", v.as_db_str()));
        }
    }

    #[test]
    fn relay_inbox_status_db_strings_match_ddl() {
        let expected = ["pending", "answered", "expired", "superseded"];
        let got: Vec<&str> = RelayInboxStatus::ALL.iter().map(|v| v.as_db_str()).collect();
        assert_eq!(got, expected);
        for v in RelayInboxStatus::ALL {
            assert_eq!(RelayInboxStatus::from_db_str(v.as_db_str()), Some(v));
            assert_eq!(serde_json::to_string(&v).unwrap(), format!("\"{}\"", v.as_db_str()));
        }
    }

    #[test]
    fn run_status_helpers() {
        assert!(RelayRunStatus::Done.is_terminal());
        assert!(RelayRunStatus::Failed.is_terminal());
        assert!(!RelayRunStatus::Running.is_terminal());
        assert!(RelayRunStatus::Blocked.needs_attention());
        assert!(RelayRunStatus::Stalled.needs_attention());
        assert!(RelayRunStatus::Crashed.needs_attention());
        assert!(!RelayRunStatus::Running.needs_attention());
        assert!(!RelayRunStatus::Paused.needs_attention());
    }

    // ---- wire struct round-trips ----

    #[test]
    fn session_update_round_trips_and_omits_none() {
        let u = RelaySessionUpdate {
            run_id: "11111111-1111-1111-1111-111111111111".into(),
            status: RelayRunStatus::Paused,
            title: "Relay engine".into(),
            goal: None,
            progress_done: 3,
            progress_total: 7,
            current_phase: Some("P3".into()),
            current_feature: None,
            last_event_at: Some("2026-07-16T10:00:00Z".into()),
            paused_until: Some("2026-07-16T11:29:00Z".into()),
            pause_reason: Some("weekly cap".into()),
            heartbeat_at: Some("2026-07-16T10:04:30Z".into()),
            project_slug: None,
            project: None,
        };
        let js = serde_json::to_string(&u).unwrap();
        let back: RelaySessionUpdate = serde_json::from_str(&js).unwrap();
        assert_eq!(back, u);
        let v: serde_json::Value = serde_json::from_str(&js).unwrap();
        assert_eq!(v["status"], json!("paused"));
        assert_eq!(v["heartbeat_at"], json!("2026-07-16T10:04:30Z"));
        assert!(v.get("goal").is_none(), "None fields are omitted");
        assert!(v.get("current_feature").is_none());
        assert!(v.get("project_slug").is_none(), "None project_slug is omitted");

        // With a project set, the slug rides the wire (drives seat attribution).
        let seated = RelaySessionUpdate { project_slug: Some("my-proj".into()), ..u };
        let jv: serde_json::Value = serde_json::from_str(&serde_json::to_string(&seated).unwrap()).unwrap();
        assert_eq!(jv["project_slug"], json!("my-proj"));
    }

    #[test]
    fn segment_round_trips_phase_and_gate() {
        let phase = RelaySegment {
            id: Some("22222222-2222-2222-2222-222222222222".into()),
            parent_id: None,
            parent_seq: None,
            seq: 0,
            title: "Auth".into(),
            summary: Some("wire the join flow".into()),
            detail: None,
            state: SegmentState::Active,
            is_gate: true,
            gate_severity: Some(GateSeverity::Blocking),
            response_verdict: None,
            response_note: None,
            agent: None,
            model: None,
            spec_ref: None,
        };
        let js = serde_json::to_string(&phase).unwrap();
        let back: RelaySegment = serde_json::from_str(&js).unwrap();
        assert_eq!(back, phase);
        let v: serde_json::Value = serde_json::from_str(&js).unwrap();
        assert_eq!(v["state"], json!("active"));
        assert_eq!(v["gate_severity"], json!("blocking"));
        assert_eq!(v["is_gate"], json!(true));
    }

    #[test]
    fn segment_carries_plan_metadata_and_omits_none() {
        // A plan-authored task node: agent/model/spec_ref ride the wire (labels only).
        let task = RelaySegment {
            id: None,
            parent_id: Some("22222222-2222-2222-2222-222222222222".into()),
            parent_seq: None,
            seq: 3,
            title: "Add register_plan handler".into(),
            summary: None,
            detail: None,
            state: SegmentState::Pending,
            is_gate: false,
            gate_severity: None,
            response_verdict: None,
            response_note: None,
            agent: Some("general-purpose".into()),
            model: Some("sonnet".into()),
            spec_ref: Some("docs/plan/2026-07-26-automated-run/tasks/register-plan.md".into()),
        };
        let js = serde_json::to_string(&task).unwrap();
        let back: RelaySegment = serde_json::from_str(&js).unwrap();
        assert_eq!(back, task);
        let v: serde_json::Value = serde_json::from_str(&js).unwrap();
        assert_eq!(v["agent"], json!("general-purpose"));
        assert_eq!(v["model"], json!("sonnet"));
        assert_eq!(v["spec_ref"], json!("docs/plan/2026-07-26-automated-run/tasks/register-plan.md"));

        // Unset plan fields are omitted from the wire — backward-compatible with the
        // cadence-derived + TodoWrite segments that never author them.
        let bare = RelaySegment { agent: None, model: None, spec_ref: None, ..task };
        let bj: serde_json::Value = serde_json::from_str(&serde_json::to_string(&bare).unwrap()).unwrap();
        assert!(bj.get("agent").is_none(), "None agent omitted");
        assert!(bj.get("model").is_none(), "None model omitted");
        assert!(bj.get("spec_ref").is_none(), "None spec_ref omitted");
    }

    #[test]
    fn inbox_item_round_trips_and_defaults_pending() {
        let item = RelayInboxItem {
            id: None,
            run_id: "33333333-3333-3333-3333-333333333333".into(),
            segment_id: Some("44444444-4444-4444-4444-444444444444".into()),
            kind: RelayInboxKind::Decision,
            direction: RelayMessageDirection::AgentToHuman,
            status: RelayInboxStatus::Pending,
            payload: json!({"prompt": "which migration policy?", "options": ["a", "b", "c"]}),
            reply: None,
            created_at: None,
            answered_at: None,
        };
        let js = serde_json::to_string(&item).unwrap();
        let back: RelayInboxItem = serde_json::from_str(&js).unwrap();
        assert_eq!(back, item);
        let v: serde_json::Value = serde_json::from_str(&js).unwrap();
        assert_eq!(v["kind"], json!("decision"));
        assert_eq!(v["direction"], json!("agent_to_human"));
        assert_eq!(v["status"], json!("pending"));

        // status defaults to pending when absent on the wire.
        let minimal = json!({
            "run_id": "33333333-3333-3333-3333-333333333333",
            "kind": "chat",
            "direction": "human_to_agent"
        });
        let parsed: RelayInboxItem = serde_json::from_value(minimal).unwrap();
        assert_eq!(parsed.status, RelayInboxStatus::Pending);
        assert_eq!(parsed.kind, RelayInboxKind::Chat);
        // payload defaults to an empty object (matching the DDL not-null default),
        // NOT null — so it never violates `jsonb not null` on the Worker insert.
        assert_eq!(parsed.payload, json!({}), "payload defaults to {{}}, not null");
    }

    #[test]
    fn inbox_pull_round_trips_with_cursor() {
        let pull = RelayInboxPull {
            items: vec![RelayInboxItem {
                id: Some("55555555-5555-5555-5555-555555555555".into()),
                run_id: "33333333-3333-3333-3333-333333333333".into(),
                segment_id: None,
                kind: RelayInboxKind::Approval,
                direction: RelayMessageDirection::AgentToHuman,
                status: RelayInboxStatus::Answered,
                payload: json!({"prompt": "run cargo test?"}),
                reply: Some(json!({"verdict": "approve"})),
                created_at: Some("2026-07-16T10:00:00Z".into()),
                answered_at: Some("2026-07-16T10:01:00Z".into()),
            }],
            cursor: 99,
        };
        let js = serde_json::to_string(&pull).unwrap();
        let back: RelayInboxPull = serde_json::from_str(&js).unwrap();
        assert_eq!(back, pull);
        assert_eq!(back.cursor, 99);
    }

    #[test]
    fn reply_round_trips() {
        let r = RelayReply {
            inbox_id: "55555555-5555-5555-5555-555555555555".into(),
            reply: json!({"verdict": "request_changes", "note": "add a test"}),
        };
        let js = serde_json::to_string(&r).unwrap();
        let back: RelayReply = serde_json::from_str(&js).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn inbox_ack_round_trips() {
        // The Worker's `POST relay/inbox` ack: `{id, seq}`.
        let ack = RelayInboxAck { id: "inbox-7".into(), seq: 42 };
        let js = serde_json::to_string(&ack).unwrap();
        let back: RelayInboxAck = serde_json::from_str(&js).unwrap();
        assert_eq!(back, ack);
        let v: serde_json::Value = serde_json::from_str(&js).unwrap();
        assert_eq!(v["id"], json!("inbox-7"));
        assert_eq!(v["seq"], json!(42));
    }

    #[test]
    fn session_ack_round_trips() {
        // The Worker's `POST relay/session` ack: `{id}` (the cloud session id).
        let ack = RelaySessionAck { id: "sess-9".into() };
        let js = serde_json::to_string(&ack).unwrap();
        let back: RelaySessionAck = serde_json::from_str(&js).unwrap();
        assert_eq!(back, ack);
        let v: serde_json::Value = serde_json::from_str(&js).unwrap();
        assert_eq!(v["id"], json!("sess-9"));
    }

    #[test]
    fn segments_publish_round_trips() {
        let p = RelaySegmentsPublish {
            run_id: "11111111-1111-1111-1111-111111111111".into(),
            segments: vec![RelaySegment {
                id: None,
                parent_id: None,
                parent_seq: None,
                seq: 0,
                title: "Auth".into(),
                summary: Some("wire join flow".into()),
                detail: None,
                state: SegmentState::Active,
                is_gate: false,
                gate_severity: None,
                response_verdict: None,
                response_note: None,
                agent: None,
                model: None,
                spec_ref: None,
            }],
        };
        let js = serde_json::to_string(&p).unwrap();
        let back: RelaySegmentsPublish = serde_json::from_str(&js).unwrap();
        assert_eq!(back, p);
        assert_eq!(back.segments.len(), 1);
    }
}
