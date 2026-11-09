//! Relay lanes slice 1, item 2 — the pure core.
//!
//! Wire types, per-kind props validation, the task state machine, the leak
//! guard, and lane-token hashing. Everything here is DB-free so the phase-A
//! reds run without L4's DDL.
//!
//! Shapes: `app/src/lib/relay-api.ts` (person-facing), `lanes-data.md` r2
//! (store), `CONTRACT.md` §3 (lane wire). Open rulings live in the L3 inbox
//! note `20261105-1545-L3-relay-design-questions.md`; tests that encode a
//! recommendation rather than a settled reading carry a `Qn-REC` marker.

// Phase-A scaffolding: only the tests use this module until the handlers and
// the store land in phase B. Removed then (like L4's F6 comments).
#![allow(dead_code)]

pub mod envelope;

use serde::{Deserialize, Serialize};

// ─── Vocabulary ───
// Exact strings from relay-api.ts. Unknown values fail to parse — the daemon
// never coerces an unknown enum.

/// `RelayRole`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayRole {
    Coordinator,
    Contributor,
}

/// `RelayLimitScope`: which quota a limit-paused lane ran out of. Mirrors
/// the assistant warnings (`daily`/`weekly`); `other` is everything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayLimitScope {
    Daily,
    Weekly,
    Other,
}

/// `RelayTaskPhase`. No `merge` phase (Q3 settled): merging is the `merged`
/// action, `approved → done`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayTaskPhase {
    Analysis,
    Tests,
    Impl,
    Docs,
    Mockup,
}

/// `RelayTaskState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayTaskState {
    Assigned,
    Working,
    InReview,
    Rework,
    Approved,
    Done,
    Blocked,
    Interrupted,
    Cancelled,
}

/// A checklist step's state (F8): `open` advances to `done`, or to
/// `skipped` with a reason. Done/skipped never reopen — a new turn
/// posts a new key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayStepState {
    Open,
    Done,
    Skipped,
}

/// `RelayLeaseState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayLeaseState {
    Queued,
    Granted,
    Released,
    Expired,
    Refused,
}

/// `RelayItemKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayItemKind {
    Bug,
    Feature,
    Gap,
    Issue,
    Finding,
}

/// `RelayTicketProvider`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayTicketProvider {
    Jira,
    Ado,
    Github,
}

/// `RelayActionKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayActionKind {
    Assigned,
    BriefUpdated,
    Started,
    Reported,
    Reviewed,
    Asked,
    Answered,
    Message,
    Delivered,
    Read,
    Merged,
    Interrupted,
    Reoffered,
    Compacted,
    HandedOver,
}

/// `reviewed.props.verdict`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    Approved,
    ChangesRequested,
}

/// `RelaySurface`. `lane` for lane-sent messages/answers (Q12, settled).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelaySurface {
    Desktop,
    Dojo,
    Lane,
}

/// `RelayFindingSeverity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayFindingSeverity {
    High,
    Medium,
    Low,
}

// ─── Task state machine (lanes-data *Stages*) ───

/// Events that move a task. `Unblocked` carries the pre-blocked state the
/// daemon stored when the ask blocked the task (Q5: `tasks.blocked_from_state`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskEvent {
    Started,
    Reported,
    ReviewApproved,
    ReviewChangesRequested,
    Merged,
    LeaseLost,
    Reoffered,
    Cancelled,
    Blocked,
    Unblocked {
        blocked_from: RelayTaskState,
    },
    /// The coordinator re-assigns the next phase (Q11, settled): an approved
    /// task goes back to `working` and starts a new round (the round is the
    /// store's write, part 2).
    Reassigned,
    /// The coordinator pulls a held task off its lane mid-turn (F1): it
    /// lands `assigned` on the new lane with a fresh round.
    HandedOver,
    /// A limit-paused lane's held task suspends for re-offer elsewhere.
    /// Same target as `LeaseLost`, distinct cause: the lease may be live.
    LimitPaused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionError {
    pub from: RelayTaskState,
    pub event: TaskEvent,
}

/// The next state, or the rejected move. Pure: the store writes the state and
/// its action in one transaction (part 2).
pub fn transition(
    from: RelayTaskState,
    event: TaskEvent,
) -> Result<RelayTaskState, TransitionError> {
    use RelayTaskState as S;
    use TaskEvent as E;
    let bad = || TransitionError { from, event };
    match (from, event) {
        (S::Assigned, E::Started) | (S::Rework, E::Started) => Ok(S::Working),
        (S::Working, E::Reported) | (S::Rework, E::Reported) => Ok(S::InReview),
        (S::InReview, E::ReviewChangesRequested) => Ok(S::Rework),
        (S::InReview, E::ReviewApproved) => Ok(S::Approved),
        (S::Approved, E::Merged) => Ok(S::Done),
        (S::Approved, E::Reassigned) => Ok(S::Working),
        (S::Working, E::Blocked) | (S::InReview, E::Blocked) => Ok(S::Blocked),
        // Assigned blocks too (implied: Assigned is a valid resume target,
        // and the part-2 ask test pins an assigned task blocking).
        (S::Assigned, E::Blocked) => Ok(S::Blocked),
        (S::Blocked, E::Unblocked { blocked_from })
            if !matches!(blocked_from, S::Blocked | S::Interrupted | S::Cancelled | S::Done) =>
        {
            Ok(blocked_from)
        }
        // Assigned/Rework join the pinned three (unpinned edge: the sweeper
        // interrupts the lapsed lane's whole active set, and both hold the
        // one-active-task unique; approved/done never interrupt).
        (S::Assigned, E::LeaseLost)
        | (S::Working, E::LeaseLost)
        | (S::InReview, E::LeaseLost)
        | (S::Rework, E::LeaseLost)
        | (S::Blocked, E::LeaseLost) => Ok(S::Interrupted),
        // Limit exhaustion suspends the same active set (distinct cause,
        // same landing: the sweep re-offers from `interrupted`).
        (S::Assigned, E::LimitPaused)
        | (S::Working, E::LimitPaused)
        | (S::InReview, E::LimitPaused)
        | (S::Rework, E::LimitPaused)
        | (S::Blocked, E::LimitPaused) => Ok(S::Interrupted),
        (S::Interrupted, E::Reoffered) => Ok(S::Assigned),
        // F1: handover pulls from the three held states only (blocked and
        // interrupted keep their own exits; assigned/approved have nothing
        // to pull).
        (S::Working, E::HandedOver) | (S::InReview, E::HandedOver) | (S::Rework, E::HandedOver) => {
            Ok(S::Assigned)
        }
        (cur, E::Cancelled) if !matches!(cur, S::Done | S::Cancelled) => Ok(S::Cancelled),
        _ => Err(bad()),
    }
}

// ─── wait_for_task timeout (brief §3: a wait holds at most 50 s) ───

/// Default hold when the caller omits `timeout_ms` (Q10 settled).
pub const WAIT_TIMEOUT_DEFAULT_MS: u64 = 25_000;
/// The longest hold the daemon grants.
pub const WAIT_TIMEOUT_MAX_MS: u64 = 50_000;

/// Omitted → default; otherwise clamped to 1 ms – 50 s (Q10 settled).
pub fn clamp_wait_timeout(timeout_ms: Option<u64>) -> u64 {
    timeout_ms.unwrap_or(25_000).clamp(1, 50_000)
}

// ─── Lane tokens ───

/// `token_hash`: SHA-256 hex of the minted token. The raw token is returned
/// once at register and never stored.
pub fn hash_lane_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

// ─── Leak guard (Q8 settled: fail-closed, kinds only) ───

/// Reject text carrying a secret, surfacing kinds only — never values.
/// Runs over `body_md`, `report_md`, and message/summary text on write.
pub fn guard_markdown(text: &str) -> Result<(), Vec<String>> {
    // The shared detector owns the patterns (DRY: no second secret list);
    // relay reports its kinds verbatim — kinds only, never a value.
    let mut kinds: Vec<String> = Vec::new();
    for finding in crate::secret_scan::scan(text) {
        if !kinds.iter().any(|k: &String| k == finding.kind) {
            kinds.push(finding.kind.to_string());
        }
    }
    if kinds.is_empty() { Ok(()) } else { Err(kinds) }
}

// ─── Summary length (CONTRACT §3j) ───

/// Summaries are at most 600 characters.
pub const SUMMARY_MAX_CHARS: usize = 600;

/// Character count check. `Err` carries the actual count for the message.
/// Person-facing summaries are capped at 600 chars (the DDL check).
pub const MAX_SUMMARY_CHARS: usize = 600;

pub fn check_summary_len(text: &str) -> Result<(), usize> {
    let n = text.chars().count();
    if n > MAX_SUMMARY_CHARS { Err(n) } else { Ok(()) }
}

/// Action kinds whose `body_md` is a summary (capped) rather than a brief or
/// a full report (uncapped). Derived reading: the `merged` note is
/// summary-like ("1–2 sentences", §3g); `assigned`/`brief_updated` bodies are
/// briefs and `report_md` is the full report.
pub fn body_is_summary(kind: RelayActionKind) -> bool {
    use RelayActionKind as K;
    matches!(kind, K::Reported | K::Reviewed | K::Asked | K::Answered | K::Message | K::Merged)
}

// ─── Props validation ───

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropsError {
    pub message: String,
}

/// Validate `props` keys for `kind` (lanes-data table; required = non-`?` in
/// relay-api.ts). Unknown keys are rejected; an explicit null on a required
/// key counts as missing (D83: null = not reported). Finding-class *existence*
/// needs the DB (part 2); here each finding's shape is checked.
/// Settled refinements: `message.to.kind` ∈ coordinator/lane/everyone/person
/// (Q2), lane-carrying refs are uuids (Q1), `surface` ∈ desktop/dojo/lane (Q12).
fn props_err(message: String) -> PropsError {
    PropsError { message }
}

fn is_uuid_str(s: &str) -> bool {
    uuid::Uuid::parse_str(s).is_ok()
}

fn check_surface(value: &serde_json::Value) -> Result<(), PropsError> {
    match value.as_str() {
        Some("lane") | Some("desktop") | Some("dojo") => Ok(()),
        _ => Err(props_err(format!("surface must be lane|desktop|dojo, saw {value}"))),
    }
}

fn check_to_shape(to: &serde_json::Value) -> Result<(), PropsError> {
    let obj = to.as_object().ok_or_else(|| props_err("to must be an object".into()))?;
    let kind = obj
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| props_err("to.kind is required".into()))?;
    match kind {
        "lane" => {
            let id = obj
                .get("lane_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| props_err("to.lane_id is required for kind lane".into()))?;
            if !is_uuid_str(id) {
                return Err(props_err(format!("to.lane_id must be a uuid, saw {id}")));
            }
        }
        "everyone" | "coordinator" | "person" => {
            if obj.contains_key("lane_id") {
                return Err(props_err(format!("to.kind {kind} carries no lane_id")));
            }
        }
        other => return Err(props_err(format!("unknown to.kind {other}"))),
    }
    for key in obj.keys() {
        if !matches!(key.as_str(), "kind" | "lane_id") {
            return Err(props_err(format!("unknown to key {key}")));
        }
    }
    Ok(())
}

pub fn validate_props(kind: RelayActionKind, props: &serde_json::Value) -> Result<(), PropsError> {
    use RelayActionKind as K;
    let obj = props.as_object().ok_or_else(|| props_err("props must be an object".into()))?;
    let missing = |key: &str| props_err(format!("{key} is required"));
    let present = |key: &str| obj.get(key).filter(|v| !v.is_null());
    // Unknown-key sweep over an allowlist; required keys checked by kind below.
    let allow = |keys: &[&str]| -> Result<(), PropsError> {
        for key in obj.keys() {
            if !keys.contains(&key.as_str()) {
                return Err(props_err(format!("unknown props key {key}")));
            }
        }
        Ok(())
    };
    match kind {
        K::Assigned => {
            allow(&["docs", "owned_paths", "acceptance"])?;
            for key in ["docs", "owned_paths", "acceptance"] {
                match present(key) {
                    Some(v) if v.is_array() => {}
                    _ => return Err(missing(key)),
                }
            }
            Ok(())
        }
        K::BriefUpdated => {
            allow(&["docs", "owned_paths", "acceptance"])?;
            for key in ["docs", "owned_paths", "acceptance"] {
                if let Some(v) = present(key)
                    && !v.is_array()
                {
                    return Err(props_err(format!("{key} must be an array")));
                }
            }
            Ok(())
        }
        K::Reported => {
            const REPORTED_KEYS: &[&str] = &[
                "look_at",
                "flags",
                "base",
                "branch",
                "commits",
                "files",
                "gates",
                // F1: partial progress marker (bool).
                "partial",
                "tokens_in",
                "tokens_out",
                "tokens_cache",
                "context_tokens",
                "compactions",
                "peak_memory_mb",
                "subagents",
                "working_secs",
                "waiting_secs",
                "started_at",
                "ended_at",
            ];
            allow(REPORTED_KEYS)?;
            if let Some(v) = present("partial")
                && v.as_bool().is_none()
            {
                return Err(props_err("partial must be a bool".into()));
            }
            for key in ["files", "gates", "commits"] {
                if let Some(v) = present(key)
                    && !v.is_array()
                {
                    return Err(props_err(format!("{key} must be an array")));
                }
            }
            if let Some(files) = present("files") {
                for f in files.as_array().unwrap_or(&vec![]) {
                    let ok = f.get("path").and_then(|v| v.as_str()).is_some()
                        && f.get("added").and_then(|v| v.as_i64()).is_some()
                        && f.get("removed").and_then(|v| v.as_i64()).is_some();
                    if !ok {
                        return Err(props_err("files[] needs path/added/removed".into()));
                    }
                }
            }
            if let Some(gates) = present("gates") {
                for g in gates.as_array().unwrap_or(&vec![]) {
                    let ok = g.get("cmd").and_then(|v| v.as_str()).is_some()
                        && g.get("exit").and_then(|v| v.as_i64()).is_some();
                    if !ok {
                        return Err(props_err("gates[] needs cmd/exit".into()));
                    }
                }
            }
            Ok(())
        }
        K::Reviewed => {
            const REVIEWED_KEYS: &[&str] = &[
                "verdict",
                "findings",
                "probes",
                "tokens_in",
                "tokens_out",
                "tokens_cache",
                "context_tokens",
                "compactions",
                "peak_memory_mb",
                "subagents",
                "working_secs",
                "waiting_secs",
                "started_at",
                "ended_at",
            ];
            allow(REVIEWED_KEYS)?;
            match present("verdict").and_then(|v| v.as_str()) {
                Some("approved") | Some("changes_requested") => {}
                _ => return Err(props_err("verdict must be approved|changes_requested".into())),
            }
            let findings = present("findings")
                .and_then(|v| v.as_array())
                .ok_or_else(|| missing("findings"))?;
            for f in findings {
                match f.get("severity").and_then(|v| v.as_str()) {
                    Some("low") | Some("medium") | Some("high") => {}
                    _ => return Err(props_err("findings[] needs severity low|medium|high".into())),
                }
                match f.get("class").and_then(|v| v.as_str()) {
                    Some(c) if !c.is_empty() => {}
                    _ => return Err(props_err("findings[] needs a non-empty class".into())),
                }
                if f.get("text").and_then(|v| v.as_str()).is_none() {
                    return Err(props_err("findings[] needs text".into()));
                }
            }
            Ok(())
        }
        K::Asked => {
            allow(&["options", "blocks", "asked_of", "context", "recommended"])?;
            let options =
                present("options").and_then(|v| v.as_array()).ok_or_else(|| missing("options"))?;
            for o in options {
                let ok = o.get("id").and_then(|v| v.as_str()).is_some()
                    && o.get("label").and_then(|v| v.as_str()).is_some();
                if !ok {
                    return Err(props_err("options[] needs id/label".into()));
                }
            }
            if present("blocks").and_then(|v| v.as_array()).is_none() {
                return Err(missing("blocks"));
            }
            match present("asked_of").and_then(|v| v.as_str()) {
                Some("person") => {}
                Some(id) if is_uuid_str(id) => {}
                _ => return Err(props_err("asked_of must be person or a uuid".into())),
            }
            Ok(())
        }
        K::Answered => {
            allow(&["option", "note", "surface"])?;
            if present("option").and_then(|v| v.as_str()).is_none() {
                return Err(missing("option"));
            }
            check_surface(present("surface").ok_or_else(|| missing("surface"))?)?;
            Ok(())
        }
        K::Message => {
            allow(&["to", "surface"])?;
            check_to_shape(present("to").ok_or_else(|| missing("to"))?)?;
            check_surface(present("surface").ok_or_else(|| missing("surface"))?)?;
            Ok(())
        }
        K::Merged => {
            allow(&["commits", "into", "gates"])?;
            if present("commits").and_then(|v| v.as_array()).is_none() {
                return Err(missing("commits"));
            }
            if present("into").and_then(|v| v.as_str()).is_none() {
                return Err(missing("into"));
            }
            if present("gates").and_then(|v| v.as_array()).is_none() {
                return Err(missing("gates"));
            }
            Ok(())
        }
        K::Interrupted => {
            allow(&["checkpoint"])?;
            Ok(())
        }
        K::Reoffered => {
            allow(&["to_lane"])?;
            if let Some(v) = present("to_lane")
                && v.as_str().is_none_or(|s| !is_uuid_str(s))
            {
                return Err(props_err("to_lane must be a uuid".into()));
            }
            Ok(())
        }
        K::Compacted => {
            allow(&["context_tokens_before", "context_tokens_after"])?;
            Ok(())
        }
        K::HandedOver => {
            allow(&["from_lane", "from_state", "to_lane"])?;
            for key in ["from_lane", "to_lane"] {
                match present(key).and_then(|v| v.as_str()) {
                    Some(id) if is_uuid_str(id) => {}
                    _ => return Err(props_err(format!("{key} must be a uuid"))),
                }
            }
            if present("from_state").and_then(|v| v.as_str()).is_none() {
                return Err(missing("from_state"));
            }
            Ok(())
        }
        K::Started | K::Delivered | K::Read => {
            allow(&[])?;
            Ok(())
        }
    }
}

// ─── Lane refs, register modes, surfaces (Q1/Q6/Q12, settled) ───

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub message: String,
}

/// A `*lane_id*` wire value: a uuid (Q1). Format only, not the version — the
/// version is a minting discipline (phase B mints v7); the wire contract is
/// "parses as a uuid, never a code".
pub fn parse_lane_ref(s: &str) -> Result<uuid::Uuid, uuid::Error> {
    uuid::Uuid::parse_str(s)
}

/// What a `register` call does (Q6, settled).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterMode {
    /// Coordinator without `crew_id`: mints the crew (optional goal,
    /// repo_key, workspace_id ride along; the paste line carries the id out).
    MintCrew,
    /// Coordinator with `crew_id`, or contributor with `crew_id`: joins.
    /// "One coordinator per crew" is the store's check (part 2).
    JoinCrew,
}

/// Coordinator without `crew_id` mints; a contributor without one is refused.
pub fn register_mode(
    role: RelayRole,
    crew_id: Option<&str>,
) -> Result<RegisterMode, ValidationError> {
    match (role, crew_id) {
        (RelayRole::Coordinator, None) => Ok(RegisterMode::MintCrew),
        (RelayRole::Coordinator, Some(_)) | (RelayRole::Contributor, Some(_)) => {
            Ok(RegisterMode::JoinCrew)
        }
        (RelayRole::Contributor, None) => Err(ValidationError {
            message: "contributors register with crew_id; only a coordinator mints".into(),
        }),
    }
}

/// Who performs a surfaced action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Caller {
    Person,
    Lane,
}

/// Settle `surface` for a message/answer (Q12, settled): the daemon sets
/// `lane` for lane actions. Derived fail-closed edges (flagged for L0 veto):
/// a lane claiming a person surface is spoofing (refused); a person must name
/// a person surface (omitted or `lane` refused).
pub fn resolve_surface(
    caller: Caller,
    given: Option<RelaySurface>,
) -> Result<RelaySurface, ValidationError> {
    match (caller, given) {
        (Caller::Lane, None) | (Caller::Lane, Some(RelaySurface::Lane)) => Ok(RelaySurface::Lane),
        (Caller::Lane, Some(_)) => Err(ValidationError {
            message: "lanes act as surface lane; a person surface is spoofing".into(),
        }),
        (Caller::Person, Some(RelaySurface::Desktop)) => Ok(RelaySurface::Desktop),
        (Caller::Person, Some(RelaySurface::Dojo)) => Ok(RelaySurface::Dojo),
        (Caller::Person, _) => {
            Err(ValidationError { message: "the person names surface desktop|dojo".into() })
        }
    }
}

// ─── Lease math (Q7, settled) ───

/// A lane lease lives 300 s; only heartbeat renews it (no other call
/// touches the lease — `relay_auth_lane` authenticates, it does not renew).
pub const LANE_LEASE_TTL_SECS: u64 = 300;
/// An interrupted task waits 900 s for its lane, then is re-offered.
pub const REOFFER_GRACE_SECS: u64 = 900;

/// True once `now` reaches the lease's end. Clocks are fed in (tests never
/// read wall time).
pub fn lease_expired(expires_at_unix: i64, now_unix: i64) -> bool {
    expires_at_unix <= now_unix
}

/// True once the grace after the interrupt has run out.
pub fn reoffer_due(interrupted_at_unix: i64, now_unix: i64) -> bool {
    now_unix - interrupted_at_unix >= 900
}

// ─── Register guidance (F9) ───

/// Guidance schema version. Bump when the generic contract's meaning
/// changes (new routes, new TTLs); prose-only clarifications keep it.
pub const RELAY_GUIDANCE_VERSION: &str = "1";

/// The self-sufficient join-time contract: a lane that reads nothing but
/// endpoint.md plus this text can join, beat, poll, work, and report.
/// Every claim here is pinned to code (`LANE_LEASE_TTL_SECS`,
/// `REOFFER_GRACE_SECS`, `clamp_wait_timeout`, the heartbeat-only renewal
/// in `relay_heartbeat`, the Bearer scheme in `auth_lane`).
pub const RELAY_GUIDANCE_GENERIC: &str = "You joined a relay crew as a lane. The contract, in full:\n\
- Lease: your lane holds a 300s lease and ONLY heartbeat renews it. \
POST /api/relay/heartbeat {\"lane_id\": \"<you>\"} (MCP: relay_heartbeat) \
with header Authorization: Bearer <lane_token>. Beat every 60s from a loop \
you own. A lapsed lease suspends the lane; your held task is marked \
interrupted (checkpoint kept) and re-offered to a live lane after a 900s \
grace. No other call renews the lease.\n\
- Poll: GET /api/relay/tasks?lane_id=<you> (MCP: relay_get_tasks) lists your \
queue. GET /api/relay/tasks/wait?lane_id=<you>&timeout_ms= (MCP: \
relay_wait_for_task) holds the connection up to 50s (default 25s) for the \
next assignment or re-offer. Prefer wait over spinning.\n\
- Flow: the coordinator plans and assigns; you work, then report with POST \
/api/relay/tasks/{id}/report (MCP: relay_report_task). A partial report \
({\"partial\": true}) records progress and keeps the task working; a full \
report moves it to in_review and names a reviewer, whose approval lets the \
coordinator merge. Only the coordinator merges; only the named reviewer \
approves.\n\
- Resume: after a restart or context clear, POST /api/relay/reregister (MCP: \
relay_reregister) with your lane_id resumes the same lane and returns the \
task checkpoint. POST /api/relay/unregister releases the lane cleanly when \
you leave.\n\
- Silence: no transcripts in any field. Titles, summaries, notes, and \
messages carry decisions and diffs, never pasted session text.";

/// Per-family overrides, selected by `relay_guidance_for`. Each names its
/// family first (so a lane can confirm the match) and covers transport,
/// wakeups, and quirks. Families grow here as runs observe them; unknown
/// families get generic-only, never an empty block.
pub const RELAY_GUIDANCE_MUSE: &str = "muse lanes: drive the crew over MCP — run \
sensei-mcp with SENSEI_DAEMON_URL pointing at the broker and call the relay_* \
tools (relay_register mints the token and is the one call that needs no auth \
header). Keep the 60s heartbeat loop in a shell you own; if the session dies, \
reregister from a fresh shell with the saved lane_id and the checkpoint \
restores. No known quirks at guidance v1 — report yours to the coordinator.";

pub const RELAY_GUIDANCE_CLAUDE: &str = "claude lanes (claude-code included): the same \
MCP stdio shape — sensei-mcp plus SENSEI_DAEMON_URL, relay_* tools. Claude \
sessions clear context often, so save lane_id and lane_token outside the \
session (the coordinator paste line) and reregister to resume with the \
checkpoint. No known quirks at guidance v1 — report yours to the coordinator.";

pub const RELAY_GUIDANCE_OPENCODE: &str = "opencode lanes: raw HTTP is the reliable \
path — curl the broker routes with -H \"Authorization: Bearer $LANE_TOKEN\" \
(the token never goes in the query string). Beat POST /api/relay/heartbeat \
every 60s from a background loop and pick work up via tasks/wait (timeout_ms \
up to 50000) instead of spinning tasks. If your harness has an MCP bridge, \
the relay_* tools mirror the HTTP routes one-to-one. No known quirks at \
guidance v1 — report yours to the coordinator.";

pub const RELAY_GUIDANCE_FREEBUFF: &str = "freebuff lanes: raw HTTP like opencode — \
curl plus the Bearer header, a 60s heartbeat loop, tasks/wait for pickup. \
Wakeups are yours to schedule: without a beat the 300s lease lapses and the \
held task re-offers after the grace, so prefer an external loop or cron over \
in-session timers. No known quirks at guidance v1 — report yours to the \
coordinator.";

/// The register-time guidance block: versioned generic contract plus the
/// selected family override (`None` for unknown families).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayGuidance {
    pub version: &'static str,
    pub generic: &'static str,
    pub family_notes: Option<&'static str>,
}

/// Select the guidance for a register's `family`/`model`: the family wins
/// on a case-insensitive exact-or-prefix match (`claude-code` reads as
/// claude), else the model gets the same match (a model id like
/// `claude-opus-4-1` still finds its notes), else generic-only. Never
/// fails — unknown input yields `family_notes: None`, not an error.
pub fn relay_guidance_for(family: &str, model: Option<&str>) -> RelayGuidance {
    let key = guidance_family_key(family).or_else(|| model.and_then(guidance_family_key));
    let family_notes = match key {
        Some("muse") => Some(RELAY_GUIDANCE_MUSE),
        Some("claude") => Some(RELAY_GUIDANCE_CLAUDE),
        Some("opencode") => Some(RELAY_GUIDANCE_OPENCODE),
        Some("freebuff") => Some(RELAY_GUIDANCE_FREEBUFF),
        _ => None,
    };
    RelayGuidance { version: RELAY_GUIDANCE_VERSION, generic: RELAY_GUIDANCE_GENERIC, family_notes }
}

fn guidance_family_key(value: &str) -> Option<&'static str> {
    let normalized = value.trim().to_ascii_lowercase();
    ["muse", "claude", "opencode", "freebuff"].into_iter().find(|key| {
        let key: &str = key;
        normalized == key || normalized.starts_with(key)
    })
}

// ─── Reviewer pick (Q14, settled) ───

/// Pick the reviewer at first report: the least-loaded live lane that isn't
/// the author (ties break on the smallest lane id, deterministically), else
/// the coordinator — unless the coordinator is the author, when there is no
/// pick. `live` is (lane id, open review count) for the crew's live lanes;
/// liveness itself is the store's read (part 2).
pub fn pick_reviewer(
    live: &[(uuid::Uuid, u64)],
    author: &uuid::Uuid,
    coordinator: &uuid::Uuid,
) -> Option<uuid::Uuid> {
    // Least-loaded other lane, ties to the smallest id; else the
    // coordinator unless it is the author (Q14 settled — the fallback is
    // unconditional; a dead fallback reviewer is the sweeper's Q17 case).
    let mut others: Vec<&(uuid::Uuid, u64)> = live.iter().filter(|(id, _)| id != author).collect();
    others.sort_by_key(|o| (o.1, o.0));
    if let Some((id, _)) = others.first() {
        return Some(*id);
    }
    if coordinator != author { Some(*coordinator) } else { None }
}

// ─── Store errors (part 2: every op fails closed with a code) ───

/// What a relay store op rejects with. Each variant maps to one HTTP status
/// (see [`RelayError::status`]); the message carries the code and the facts,
/// never a secret. Success-shaped empties are never returned for failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayError {
    /// Unknown lane token.
    Unauthorized(String),
    /// Valid identity, wrong party (a contributor coordinating, a lane
    /// touching another lane's task, the wrong answerer).
    Forbidden(String),
    /// Unknown crew / lane / task / item / decision in scope.
    NotFound(String),
    /// The lane unregistered (it existed; `reregister` resumes it).
    Gone(String),
    /// A state or uniqueness rule refused the write (second coordinator,
    /// busy lane, wrong round, already answered).
    Conflict(String),
    /// The request or its props are malformed (bad phase, long summary,
    /// leaked secret, unknown finding class, self-review).
    Validation(String),
    /// Reserved for dōjō-sync operations that need the linked principal:
    /// raised when the machine has none. Relay routes never raise it —
    /// local relay runs on the machine-local user, signed in or not (Q19
    /// superseded, user ruling 2026-11-09).
    SignInRequired,
    /// Valid request, slice-2 work (the tracker pull behind ticketed triage).
    NotImplemented(String),
    /// The database (or the machine) failed under a valid request.
    Internal(String),
}

impl RelayError {
    /// The HTTP status the handler answers with.
    pub fn status(&self) -> u16 {
        match self {
            RelayError::Unauthorized(_) | RelayError::SignInRequired => 401,
            RelayError::Forbidden(_) => 403,
            RelayError::NotFound(_) => 404,
            RelayError::Gone(_) => 410,
            RelayError::Conflict(_) => 409,
            RelayError::Validation(_) => 422,
            RelayError::NotImplemented(_) => 501,
            RelayError::Internal(_) => 500,
        }
    }

    /// The stable machine code the screen matches on (`lane_not_found`,
    /// never a sentence).
    pub fn code(&self) -> &'static str {
        match self {
            RelayError::Unauthorized(_) => "relay_unauthorized",
            RelayError::Forbidden(_) => "relay_forbidden",
            RelayError::NotFound(_) => "relay_not_found",
            RelayError::Gone(_) => "relay_lane_gone",
            RelayError::Conflict(_) => "relay_conflict",
            RelayError::Validation(_) => "relay_invalid",
            RelayError::SignInRequired => "relay_signin_required",
            RelayError::NotImplemented(_) => "relay_not_implemented",
            RelayError::Internal(_) => "relay_internal",
        }
    }
}

// ─── Resource floors (lanes.md §8; the footer's line) ───

/// Refuse a resource grant below 25 GB free disk (the lanes.md example, as
/// the constant; flagged for L0 veto in the part-2 report).
pub const DISK_FLOOR_GB: u64 = 25;
/// Refuse a resource grant below 1 GB free swap (likewise).
pub const SWAP_FLOOR_GB: u64 = 1;
/// A resource grant lives 3600 s; heartbeat renews it (Q4, settled).
pub const RESOURCE_LEASE_TTL_SECS: u64 = 3600;

/// A floor override for resource grants (`None` at the store means the
/// `DISK_FLOOR_GB` / `SWAP_FLOOR_GB` constants; tests inject extremes so
/// the grant/refuse paths stay deterministic on any disk).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayFloors {
    pub disk_gb: u64,
    pub swap_gb: u64,
}

/// The grant decision for a resource request (Q5/Q6, settled): grant
/// unless a reported reading sits strictly below its floor; an unknown
/// reading (`None`) never refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceGrant {
    Granted,
    Refused { reason: String },
}

/// Decide a resource grant from the readings (pure; Q5/Q6): refuse iff a
/// reported reading sits strictly below its floor; unknown readings never
/// refuse. The reason names every short reading with its floor.
pub fn decide_resource_grant(
    disk_free_gb: Option<u64>,
    swap_free_gb: Option<u64>,
    floors: &RelayFloors,
) -> ResourceGrant {
    let mut short = Vec::new();
    if let Some(free) = disk_free_gb
        && free < floors.disk_gb
    {
        short.push(format!("disk free {free} GB < floor {} GB", floors.disk_gb));
    }
    if let Some(free) = swap_free_gb
        && free < floors.swap_gb
    {
        short.push(format!("swap free {free} GB < floor {} GB", floors.swap_gb));
    }
    if short.is_empty() {
        ResourceGrant::Granted
    } else {
        ResourceGrant::Refused { reason: short.join("; ") }
    }
}

// ─── Session-local time (the completed window is the Mac's day) ───

/// This Mac's IANA zone, from `/etc/localtime` (e.g. `America/Chicago`).
/// The daemon pool sets `TimeZone` to it so `relay.now`'s completed window
/// is the session-local day. Fails closed when the link is unreadable.
/// This Mac's IANA zone (`America/Chicago`), read off `/etc/localtime`.
/// Fail-closed: anything outside `zoneinfo/` — or outside the zone-name
/// alphabet — is an error, never a guess (the pool interpolates the name
/// into `SET TIME ZONE`, which takes no binds).
pub fn system_timezone() -> Result<String, String> {
    let link = std::fs::read_link("/etc/localtime")
        .map_err(|e| format!("system_timezone: cannot read /etc/localtime: {e}"))?
        .to_string_lossy()
        .into_owned();
    let zone =
        link.rsplit("zoneinfo/").next().filter(|z| *z != link).map(str::to_string).ok_or_else(
            || format!("system_timezone: /etc/localtime points outside zoneinfo: {link}"),
        )?;
    if zone.is_empty()
        || !zone.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '+'))
    {
        return Err(format!("system_timezone: not a zone name: {zone}"));
    }
    Ok(zone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Secret fixtures are built from split parts at runtime so no contiguous
    // secret-format literal appears in the SOURCE (GitHub push protection
    // would otherwise flag the test data as a real secret). Same trick as
    // `secret_scan`'s own tests.
    fn gh_token() -> String {
        ["gh", "p_"].concat() + &"a".repeat(40)
    }
    fn aws_key() -> String {
        ["AK", "IA"].concat() + &"A".repeat(16)
    }
    fn private_key_block() -> String {
        "-----BEGIN ".to_string() + "RSA PRIVATE KEY-----"
    }

    // ── Vocabulary ──

    #[test]
    fn enums_serialize_to_relay_api_strings() {
        assert_eq!(serde_json::to_string(&RelayTaskState::InReview).unwrap(), "\"in_review\"");
        assert_eq!(
            serde_json::to_string(&ReviewVerdict::ChangesRequested).unwrap(),
            "\"changes_requested\""
        );
        assert_eq!(
            serde_json::to_string(&RelayActionKind::BriefUpdated).unwrap(),
            "\"brief_updated\""
        );
        assert_eq!(serde_json::to_string(&RelayRole::Coordinator).unwrap(), "\"coordinator\"");
        assert_eq!(serde_json::to_string(&RelayLeaseState::Granted).unwrap(), "\"granted\"");
        assert_eq!(serde_json::to_string(&RelayItemKind::Finding).unwrap(), "\"finding\"");
        assert_eq!(serde_json::to_string(&RelayTicketProvider::Ado).unwrap(), "\"ado\"");
        assert_eq!(serde_json::to_string(&RelaySurface::Dojo).unwrap(), "\"dojo\"");
        assert_eq!(serde_json::to_string(&RelayFindingSeverity::High).unwrap(), "\"high\"");
        // Every state round-trips, including the side states.
        for s in [
            RelayTaskState::Assigned,
            RelayTaskState::Working,
            RelayTaskState::InReview,
            RelayTaskState::Rework,
            RelayTaskState::Approved,
            RelayTaskState::Done,
            RelayTaskState::Blocked,
            RelayTaskState::Interrupted,
            RelayTaskState::Cancelled,
        ] {
            let v = serde_json::to_string(&s).unwrap();
            assert_eq!(serde_json::from_str::<RelayTaskState>(&v).unwrap(), s);
        }
    }

    #[test]
    fn enums_reject_unknown_values() {
        assert!(serde_json::from_str::<RelayTaskState>("\"flying\"").is_err());
        // Q3 settled: no `merge` phase.
        assert!(serde_json::from_str::<RelayTaskPhase>("\"merge\"").is_err());
        assert!(serde_json::from_str::<ReviewVerdict>("\"maybe\"").is_err());
    }

    // ── State machine ──

    use RelayTaskState as S;
    use TaskEvent as E;

    #[test]
    fn transition_happy_path_with_rework_loop() {
        assert_eq!(transition(S::Assigned, E::Started), Ok(S::Working));
        assert_eq!(transition(S::Working, E::Reported), Ok(S::InReview));
        assert_eq!(transition(S::InReview, E::ReviewChangesRequested), Ok(S::Rework));
        assert_eq!(transition(S::Rework, E::Started), Ok(S::Working));
        assert_eq!(transition(S::Working, E::Reported), Ok(S::InReview));
        assert_eq!(transition(S::InReview, E::ReviewApproved), Ok(S::Approved));
        assert_eq!(transition(S::Approved, E::Merged), Ok(S::Done));
    }

    #[test]
    fn transition_rejects_skips() {
        assert_eq!(
            transition(S::Assigned, E::Reported),
            Err(TransitionError { from: S::Assigned, event: E::Reported })
        );
        assert!(transition(S::Working, E::Merged).is_err());
        assert!(transition(S::Done, E::Started).is_err());
        assert!(transition(S::Approved, E::Reported).is_err());
        assert!(transition(S::Cancelled, E::Started).is_err());
    }

    #[test]
    fn transition_rework_may_report_directly() {
        // Derived: "the next report returns it to in_review" — a lane that
        // reports from rework without an explicit re-start is accepted.
        assert_eq!(transition(S::Rework, E::Reported), Ok(S::InReview));
    }

    #[test]
    fn transition_interrupted_then_reoffered_to_assigned() {
        assert_eq!(transition(S::Working, E::LeaseLost), Ok(S::Interrupted));
        assert_eq!(transition(S::InReview, E::LeaseLost), Ok(S::Interrupted));
        assert_eq!(transition(S::Blocked, E::LeaseLost), Ok(S::Interrupted));
        // Limit exhaustion lands the same set, terminal excluded.
        assert_eq!(transition(S::Assigned, E::LimitPaused), Ok(S::Interrupted));
        assert_eq!(transition(S::Working, E::LimitPaused), Ok(S::Interrupted));
        assert_eq!(transition(S::Rework, E::LimitPaused), Ok(S::Interrupted));
        assert!(transition(S::Done, E::LimitPaused).is_err());
        assert!(transition(S::Interrupted, E::LimitPaused).is_err());
        assert_eq!(transition(S::Interrupted, E::Reoffered), Ok(S::Assigned));
        assert!(transition(S::Working, E::Reoffered).is_err());
        assert!(transition(S::Done, E::LeaseLost).is_err());
        assert!(transition(S::Interrupted, E::LeaseLost).is_err());
    }

    #[test]
    fn transition_handover_from_held_states_only() {
        // F1: working/in_review/rework hand over to assigned; every other
        // state refuses (blocked/interrupted keep their own exits;
        // assigned/approved have nothing to pull).
        for s in [S::Working, S::InReview, S::Rework] {
            assert_eq!(transition(s, E::HandedOver), Ok(S::Assigned), "{s:?} hands over");
        }
        for s in [S::Assigned, S::Approved, S::Blocked, S::Interrupted, S::Done, S::Cancelled] {
            assert!(transition(s, E::HandedOver).is_err(), "{s:?} refuses handover");
        }
    }

    #[test]
    fn transition_cancelled_from_any_non_terminal() {
        for s in [
            S::Assigned,
            S::Working,
            S::InReview,
            S::Rework,
            S::Approved,
            S::Blocked,
            S::Interrupted,
        ] {
            assert_eq!(transition(s, E::Cancelled), Ok(S::Cancelled), "{s:?} cancels");
        }
        assert!(transition(S::Done, E::Cancelled).is_err());
        assert!(transition(S::Cancelled, E::Cancelled).is_err());
    }

    #[test]
    fn transition_blocked_resumes_where_it_was() {
        assert_eq!(transition(S::Working, E::Blocked), Ok(S::Blocked));
        assert_eq!(transition(S::InReview, E::Blocked), Ok(S::Blocked));
        assert_eq!(
            transition(S::Blocked, E::Unblocked { blocked_from: S::Working }),
            Ok(S::Working)
        );
        assert_eq!(
            transition(S::Blocked, E::Unblocked { blocked_from: S::InReview }),
            Ok(S::InReview)
        );
        // Not blocked → unblock is rejected; side/terminal states are never
        // a resume target.
        assert!(transition(S::Working, E::Unblocked { blocked_from: S::Working }).is_err());
        for bad in [S::Blocked, S::Interrupted, S::Cancelled, S::Done] {
            assert!(
                transition(S::Blocked, E::Unblocked { blocked_from: bad }).is_err(),
                "{bad:?} is never a resume target"
            );
        }
        assert!(transition(S::Blocked, E::Started).is_err());
        assert!(transition(S::Done, E::Blocked).is_err());
    }

    // ── Timeout / token / guard / length ──

    // ── Register guidance (F9) ──

    #[test]
    fn guidance_selects_exact_family() {
        let g = relay_guidance_for("muse", None);
        assert_eq!(g.version, "1");
        assert!(!g.generic.is_empty());
        assert!(g.family_notes.unwrap().starts_with("muse lanes"));
    }

    #[test]
    fn guidance_selects_family_prefix_case_insensitive() {
        // `claude-code` reads as claude; so does any case variant.
        for family in ["claude-code", "Claude-Code", " CLAUDE "] {
            let g = relay_guidance_for(family, None);
            assert!(g.family_notes.unwrap().starts_with("claude lanes"), "{family}");
        }
    }

    #[test]
    fn guidance_falls_back_to_model() {
        // Unknown family, known model id: the model finds its notes.
        let g = relay_guidance_for("custom-harness", Some("claude-opus-4-1"));
        assert!(g.family_notes.unwrap().starts_with("claude lanes"));
    }

    #[test]
    fn guidance_family_beats_model() {
        let g = relay_guidance_for("muse", Some("claude-opus-4-1"));
        assert!(g.family_notes.unwrap().starts_with("muse lanes"));
    }

    #[test]
    fn guidance_unknown_is_generic_only() {
        // Documented-but-unlisted families (codex), empties, and unknown
        // models all yield a full generic block with null notes — never a
        // crash, never an empty block.
        for (family, model) in
            [("codex", None), ("", None), ("", Some("")), ("cowork", Some("gpt-x"))]
        {
            let g = relay_guidance_for(family, model);
            assert_eq!(g.version, "1");
            assert!(!g.generic.is_empty());
            assert_eq!(g.family_notes, None, "{family:?}/{model:?}");
        }
    }

    #[test]
    fn clamp_wait_timeout_defaults_and_clamps() {
        assert_eq!(clamp_wait_timeout(None), 25_000);
        assert_eq!(clamp_wait_timeout(Some(5_000)), 5_000);
        assert_eq!(clamp_wait_timeout(Some(50_000)), 50_000);
        assert_eq!(clamp_wait_timeout(Some(60_000)), 50_000);
        assert_eq!(clamp_wait_timeout(Some(0)), 1);
    }

    #[test]
    fn hash_lane_token_is_stable_hex_never_raw() {
        let h1 = hash_lane_token("lane-secret-1");
        let h2 = hash_lane_token("lane-secret-1");
        assert_eq!(h1.len(), 64);
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(h1, h2);
        assert_ne!(h1, hash_lane_token("lane-secret-2"));
        assert!(!h1.contains("lane-secret-1"));
    }

    #[test]
    fn guard_rejects_secrets_with_kinds_only() {
        let token = gh_token();
        let err = guard_markdown(&format!("see {token}")).unwrap_err();
        assert_eq!(err, vec!["github token".to_string()]);
        assert!(!format!("{err:?}").contains(&"a".repeat(40)));

        assert_eq!(guard_markdown(&aws_key()).unwrap_err(), vec!["aws access key id".to_string()]);
        assert_eq!(
            guard_markdown(&private_key_block()).unwrap_err(),
            vec!["private key block".to_string()]
        );
        // A secret-named assignment with a substantial value is rejected…
        assert!(guard_markdown("password: hunter2hunter2").is_err());
        // …but a documented env-var NAME is not a value.
        assert!(guard_markdown("api_key: SENSEI_API_KEY").is_ok());
    }

    #[test]
    fn guard_accepts_clean_brief() {
        // Q8 settled: no identifier dereference — relay text legitimately names
        // lanes, repos and paths, and stripping would corrupt briefs.
        assert!(
            guard_markdown(
                "# Diagrams header\n\nL1 owns app/src/lib/diagrams/** in sensei-hq/sensei."
            )
            .is_ok()
        );
        assert!(guard_markdown("").is_ok());
    }

    #[test]
    fn summary_len_counts_chars_not_bytes() {
        assert!(check_summary_len(&"x".repeat(600)).is_ok());
        assert_eq!(check_summary_len(&"x".repeat(601)), Err(601));
        assert!(check_summary_len(&"é".repeat(600)).is_ok());
        assert_eq!(check_summary_len(&"é".repeat(601)), Err(601));
    }

    #[test]
    fn body_is_summary_covers_every_kind() {
        use RelayActionKind as K;
        for k in [K::Reported, K::Reviewed, K::Asked, K::Answered, K::Message, K::Merged] {
            assert!(body_is_summary(k), "{k:?} is a summary");
        }
        for k in [
            K::Assigned,
            K::BriefUpdated,
            K::Started,
            K::Delivered,
            K::Read,
            K::Interrupted,
            K::Reoffered,
            K::Compacted,
        ] {
            assert!(!body_is_summary(k), "{k:?} is not a summary");
        }
    }

    // ── Props validation ──

    #[test]
    fn validate_props_requires_per_kind_keys() {
        use RelayActionKind as K;
        let full_assigned = json!({"docs": [], "owned_paths": [], "acceptance": []});
        assert!(validate_props(K::Assigned, &full_assigned).is_ok());
        let missing = json!({"docs": [], "owned_paths": []});
        let err = validate_props(K::Assigned, &missing).unwrap_err();
        assert!(err.message.contains("acceptance"), "{}", err.message);
        // An explicit null on a required key counts as missing (D83).
        let nulled = json!({"docs": [], "owned_paths": [], "acceptance": null});
        assert!(validate_props(K::Assigned, &nulled).is_err());

        assert!(
            validate_props(K::Reviewed, &json!({"verdict": "approved", "findings": []})).is_ok()
        );
        assert!(validate_props(K::Reviewed, &json!({"findings": []})).is_err());
        assert!(
            validate_props(K::Asked, &json!({"options": [], "blocks": [], "asked_of": "person"}))
                .is_ok()
        );
        assert!(validate_props(K::Asked, &json!({"options": []})).is_err());
        assert!(
            validate_props(K::Answered, &json!({"option": "db", "surface": "desktop"})).is_ok()
        );
        assert!(validate_props(K::Answered, &json!({"option": "db"})).is_err());
        assert!(
            validate_props(K::Message, &json!({"to": {"kind": "everyone"}, "surface": "desktop"}))
                .is_ok()
        );
        assert!(validate_props(K::Message, &json!({"to": {"kind": "everyone"}})).is_err());
        assert!(
            validate_props(K::Merged, &json!({"commits": [], "into": "develop", "gates": []}))
                .is_ok()
        );
        assert!(validate_props(K::Merged, &json!({"commits": []})).is_err());
    }

    #[test]
    fn validate_props_rejects_unknown_keys() {
        let err = validate_props(RelayActionKind::Reported, &json!({"bogus": 1})).unwrap_err();
        assert!(err.message.contains("bogus"), "{}", err.message);
        let err = validate_props(
            RelayActionKind::Reviewed,
            &json!({"verdict": "approved", "findings": [], "bogus": 1}),
        )
        .unwrap_err();
        assert!(err.message.contains("bogus"), "{}", err.message);
    }

    #[test]
    fn validate_props_empty_kinds_take_no_keys() {
        use RelayActionKind as K;
        for k in [K::Started, K::Delivered, K::Read] {
            assert!(validate_props(k, &json!({})).is_ok(), "{k:?} takes {{}}");
            assert!(validate_props(k, &json!({"x": 1})).is_err(), "{k:?} rejects keys");
        }
    }

    #[test]
    fn validate_props_reported_all_optional_d83() {
        // Every `reported` key is self-reported: `{}` is a valid report.
        assert!(validate_props(RelayActionKind::Reported, &json!({})).is_ok());
        assert!(
            validate_props(
                RelayActionKind::Reported,
                &json!({"look_at": null, "tokens_in": 900000, "gates": []})
            )
            .is_ok()
        );
    }

    #[test]
    fn validate_props_handed_over_pins_parties() {
        // F1: the handover event names both lanes (uuids) plus the state
        // the task left.
        assert!(
            validate_props(
                RelayActionKind::HandedOver,
                &json!({"from_lane": "01938931-0000-7000-8000-000000000001",
                        "from_state": "working",
                        "to_lane": "01938931-0000-7000-8000-000000000002"})
            )
            .is_ok()
        );
        assert!(validate_props(RelayActionKind::HandedOver, &json!({})).is_err());
        assert!(
            validate_props(
                RelayActionKind::HandedOver,
                &json!({"from_lane": "nope", "from_state": "working",
                        "to_lane": "01938931-0000-7000-8000-000000000002"})
            )
            .is_err()
        );
    }

    #[test]
    fn validate_props_reported_accepts_partial_flag() {
        // F1: `partial` is an optional bool on reported actions.
        assert!(validate_props(RelayActionKind::Reported, &json!({"partial": true})).is_ok());
        assert!(validate_props(RelayActionKind::Reported, &json!({"partial": "yes"})).is_err());
    }

    #[test]
    fn validate_props_reviewed_verdict_and_findings() {
        use RelayActionKind as K;
        assert!(validate_props(K::Reviewed, &json!({"verdict": "nope", "findings": []})).is_err());
        let bad_sev = json!({
            "verdict": "changes_requested",
            "findings": [{"severity": "critical", "class": "x", "text": "t"}]
        });
        assert!(validate_props(K::Reviewed, &bad_sev).is_err());
        let empty_class = json!({
            "verdict": "changes_requested",
            "findings": [{"severity": "high", "class": "", "text": "t"}]
        });
        assert!(validate_props(K::Reviewed, &empty_class).is_err());
        let missing_text = json!({
            "verdict": "changes_requested",
            "findings": [{"severity": "high", "class": "missing-failure-path"}]
        });
        assert!(validate_props(K::Reviewed, &missing_text).is_err());
        let good = json!({
            "verdict": "changes_requested",
            "findings": [{"severity": "high", "class": "missing-failure-path",
                          "text": "…", "file": null, "line": null}],
            "probes": 8
        });
        assert!(validate_props(K::Reviewed, &good).is_ok());
    }

    #[test]
    fn validate_props_asked_options_shape() {
        use RelayActionKind as K;
        let good = json!({
            "context": "Activity logs is waiting on this.",
            "options": [{"id": "db", "label": "In the daemon", "detail": null}],
            "recommended": "db",
            "blocks": [],
            "asked_of": "person"
        });
        assert!(validate_props(K::Asked, &good).is_ok());
        let no_label = json!({
            "options": [{"id": "db"}],
            "blocks": [],
            "asked_of": "person"
        });
        assert!(validate_props(K::Asked, &no_label).is_err());
        let empty_asked_of = json!({"options": [], "blocks": [], "asked_of": ""});
        assert!(validate_props(K::Asked, &empty_asked_of).is_err());
    }

    #[test]
    fn validate_props_interrupt_family_optional_keys() {
        use RelayActionKind as K;
        assert!(validate_props(K::Interrupted, &json!({})).is_ok());
        assert!(validate_props(K::Reoffered, &json!({"to_lane": null})).is_ok());
        assert!(validate_props(K::Compacted, &json!({"context_tokens_before": 1})).is_ok());
        assert!(validate_props(K::Interrupted, &json!({"bogus": 1})).is_err());
    }

    // ── Part 1b: settled rulings ──

    const LANE_A: &str = "01938931-0000-7000-8000-000000000001";
    const LANE_B: &str = "01938931-0000-7000-8000-000000000002";
    const LANE_C: &str = "01938931-0000-7000-8000-000000000003";

    fn uuid(s: &str) -> uuid::Uuid {
        uuid::Uuid::parse_str(s).unwrap()
    }

    #[test]
    fn transition_reassigned_advances_phase_q11() {
        // Q11 settled: no auto advance; the coordinator's re-assign moves an
        // approved task back to working (and starts a new round, part 2).
        assert_eq!(transition(S::Approved, E::Reassigned), Ok(S::Working));
        for s in [S::Assigned, S::Working, S::InReview, S::Rework, S::Done, S::Blocked] {
            assert!(transition(s, E::Reassigned).is_err(), "{s:?} + reassign");
        }
    }

    #[test]
    fn parse_lane_ref_accepts_uuids_rejects_codes_q1() {
        // Q1 settled: uuid on the wire, codes for display only. Any uuid
        // version parses — the version is a minting discipline, not the wire.
        assert!(parse_lane_ref("01938931-607b-7c71-a73b-31ddf06f228f").is_ok());
        assert!(parse_lane_ref("11111111-1111-4111-8111-111111111111").is_ok());
        assert!(parse_lane_ref("L1").is_err());
        assert!(parse_lane_ref("").is_err());
        assert!(parse_lane_ref("not-a-uuid").is_err());
    }

    #[test]
    fn register_mode_mint_join_refuse_q6() {
        use RegisterMode as M;
        use RelayRole as R;
        assert_eq!(register_mode(R::Coordinator, None), Ok(M::MintCrew));
        assert_eq!(register_mode(R::Coordinator, Some("c1")), Ok(M::JoinCrew));
        assert_eq!(register_mode(R::Contributor, Some("c1")), Ok(M::JoinCrew));
        let err = register_mode(R::Contributor, None).unwrap_err();
        assert!(err.message.contains("crew_id"), "{}", err.message);
    }

    #[test]
    fn resolve_surface_lane_and_person_q12() {
        use Caller as C;
        use RelaySurface as S;
        // The daemon sets `lane` for lane actions (Q12 settled)…
        assert_eq!(resolve_surface(C::Lane, None), Ok(S::Lane));
        assert_eq!(resolve_surface(C::Lane, Some(S::Lane)), Ok(S::Lane));
        // …and refuses the spoofing edges (derived fail-closed, L0-vetoable).
        assert!(resolve_surface(C::Lane, Some(S::Desktop)).is_err());
        assert_eq!(resolve_surface(C::Person, Some(S::Desktop)), Ok(S::Desktop));
        assert_eq!(resolve_surface(C::Person, Some(S::Dojo)), Ok(S::Dojo));
        assert!(resolve_surface(C::Person, None).is_err());
        assert!(resolve_surface(C::Person, Some(S::Lane)).is_err());
    }

    #[test]
    fn lease_math_boundaries_q7() {
        assert_eq!(LANE_LEASE_TTL_SECS, 300);
        assert_eq!(REOFFER_GRACE_SECS, 900);
        assert!(!lease_expired(1000, 999));
        assert!(lease_expired(1000, 1000));
        assert!(lease_expired(1000, 1001));
        assert!(!reoffer_due(1000, 1899));
        assert!(reoffer_due(1000, 1900));
        assert!(reoffer_due(1000, 1901));
    }

    #[test]
    fn pick_reviewer_least_loaded_not_author_q14() {
        let author = uuid(LANE_A);
        let coord = uuid(LANE_B);
        // Least-loaded other live lane wins…
        let live = vec![(uuid(LANE_B), 2), (uuid(LANE_C), 0)];
        assert_eq!(pick_reviewer(&live, &author, &coord), Some(uuid(LANE_C)));
        // …even when the author is the least loaded — never the author.
        let live = vec![(author, 0), (uuid(LANE_C), 5)];
        assert_eq!(pick_reviewer(&live, &author, &coord), Some(uuid(LANE_C)));
        // Ties break on the smallest lane id, deterministically.
        let live = vec![(uuid(LANE_C), 1), (uuid(LANE_B), 1)];
        assert_eq!(pick_reviewer(&live, &author, &coord), Some(uuid(LANE_B)));
        // No other live lane → the coordinator…
        assert_eq!(pick_reviewer(&[(author, 0)], &author, &coord), Some(coord));
        assert_eq!(pick_reviewer(&[], &author, &coord), Some(coord));
        // …unless the coordinator is the author: then no pick.
        assert_eq!(pick_reviewer(&[(author, 0)], &author, &author), None);
        assert_eq!(pick_reviewer(&[], &author, &author), None);
    }

    #[test]
    fn validate_props_message_to_kinds_q1_q2() {
        use RelayActionKind as K;
        let msg = |to: serde_json::Value| json!({"to": to, "surface": "lane"});
        assert!(validate_props(K::Message, &msg(json!({"kind": "person"}))).is_ok());
        assert!(
            validate_props(K::Message, &msg(json!({"kind": "lane", "lane_id": LANE_A}))).is_ok()
        );
        // A lane address without its id, or with a code, is refused.
        assert!(validate_props(K::Message, &msg(json!({"kind": "lane"}))).is_err());
        assert!(
            validate_props(K::Message, &msg(json!({"kind": "lane", "lane_id": "L1"}))).is_err()
        );
        // Non-lane kinds carry no lane id; unknown kinds and keys are refused.
        let desk = |to: serde_json::Value| json!({"to": to, "surface": "desktop"});
        assert!(
            validate_props(K::Message, &desk(json!({"kind": "everyone", "lane_id": LANE_A})))
                .is_err()
        );
        assert!(validate_props(K::Message, &desk(json!({"kind": "bogus"}))).is_err());
        assert!(
            validate_props(
                K::Message,
                &msg(json!({"kind": "lane", "lane_id": LANE_A, "bogus": 1}))
            )
            .is_err()
        );
    }

    #[test]
    fn validate_props_surface_values_q12() {
        use RelayActionKind as K;
        assert!(
            validate_props(
                K::Message,
                &json!({"to": {"kind": "lane", "lane_id": LANE_A}, "surface": "lane"})
            )
            .is_ok()
        );
        assert!(validate_props(K::Answered, &json!({"option": "db", "surface": "dojo"})).is_ok());
        assert!(
            validate_props(
                K::Message,
                &json!({"to": {"kind": "everyone"}, "surface": "carrier-pigeon"})
            )
            .is_err()
        );
        assert!(
            validate_props(K::Answered, &json!({"option": "db", "surface": "lane-x"})).is_err()
        );
    }

    #[test]
    fn validate_props_asked_of_person_or_uuid_q1() {
        use RelayActionKind as K;
        let asked = |of: &str| json!({"options": [], "blocks": [], "asked_of": of});
        assert!(validate_props(K::Asked, &asked("person")).is_ok());
        assert!(validate_props(K::Asked, &asked(LANE_A)).is_ok());
        assert!(validate_props(K::Asked, &asked("L1")).is_err());
    }

    // ── Part 2: errors, floors, session-local time ──

    #[test]
    fn relay_error_maps_to_status_and_code() {
        use RelayError as E;
        assert_eq!(E::Unauthorized("x".into()).status(), 401);
        assert_eq!(E::Forbidden("x".into()).status(), 403);
        assert_eq!(E::NotFound("x".into()).status(), 404);
        assert_eq!(E::Gone("x".into()).status(), 410);
        assert_eq!(E::Conflict("x".into()).status(), 409);
        assert_eq!(E::Validation("x".into()).status(), 422);
        assert_eq!(E::SignInRequired.status(), 401);
        assert_eq!(E::NotImplemented("x".into()).status(), 501);
        assert_eq!(E::Internal("x".into()).status(), 500);
        // Codes are stable snake_case tokens, one per variant.
        assert_eq!(E::Unauthorized("x".into()).code(), "relay_unauthorized");
        assert_eq!(E::Forbidden("x".into()).code(), "relay_forbidden");
        assert_eq!(E::NotFound("x".into()).code(), "relay_not_found");
        assert_eq!(E::Gone("x".into()).code(), "relay_lane_gone");
        assert_eq!(E::Conflict("x".into()).code(), "relay_conflict");
        assert_eq!(E::Validation("x".into()).code(), "relay_invalid");
        assert_eq!(E::SignInRequired.code(), "relay_signin_required");
        assert_eq!(E::NotImplemented("x".into()).code(), "relay_not_implemented");
        assert_eq!(E::Internal("x".into()).code(), "relay_internal");
    }

    #[test]
    fn resource_floors_are_lanes_md_values() {
        assert_eq!(DISK_FLOOR_GB, 25);
        assert_eq!(SWAP_FLOOR_GB, 1);
    }

    #[test]
    fn system_timezone_reads_etc_localtime() {
        // The Mac's zone, from the /etc/localtime link — the zone the
        // relay_now transaction sets, so the completed window is the
        // session-local day (the pool itself stays UTC per (B)).
        let link = std::fs::read_link("/etc/localtime").unwrap().to_string_lossy().into_owned();
        let expect = link.rsplit("zoneinfo/").next().unwrap().to_string();
        assert_eq!(system_timezone().unwrap(), expect);
        assert!(system_timezone().unwrap().contains('/'));
    }

    #[test]
    fn resource_grant_decision_matrix() {
        use super::{RelayFloors, ResourceGrant, decide_resource_grant};
        assert_eq!(super::RESOURCE_LEASE_TTL_SECS, 3600);
        let floors = RelayFloors { disk_gb: 25, swap_gb: 1 };
        // Above the floors: grant.
        assert_eq!(decide_resource_grant(Some(100), Some(8), &floors), ResourceGrant::Granted);
        // On the floor (refuse iff strictly below): grant.
        assert_eq!(decide_resource_grant(Some(25), Some(1), &floors), ResourceGrant::Granted);
        // Unknown readings never refuse (Q6).
        assert_eq!(decide_resource_grant(None, None, &floors), ResourceGrant::Granted);
        assert_eq!(decide_resource_grant(None, Some(8), &floors), ResourceGrant::Granted);
        assert_eq!(decide_resource_grant(Some(100), None, &floors), ResourceGrant::Granted);
        // Below a floor: refuse, naming the short reading and the floor.
        let ResourceGrant::Refused { reason } = decide_resource_grant(Some(10), Some(8), &floors)
        else {
            panic!("disk below floor must refuse");
        };
        assert!(reason.contains("disk"), "reason names disk: {reason}");
        assert!(reason.contains("10"), "reason names reading: {reason}");
        assert!(reason.contains("25"), "reason names floor: {reason}");
        let ResourceGrant::Refused { reason } = decide_resource_grant(Some(100), Some(0), &floors)
        else {
            panic!("swap below floor must refuse");
        };
        assert!(reason.contains("swap"), "reason names swap: {reason}");
        // Both short: one reason naming both.
        let ResourceGrant::Refused { reason } = decide_resource_grant(Some(10), Some(0), &floors)
        else {
            panic!("both below floor must refuse");
        };
        assert!(reason.contains("disk") && reason.contains("swap"), "both: {reason}");
    }
}
