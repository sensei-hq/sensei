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
    let _ = (from, event);
    todo!("relay item 2 phase B")
}

// ─── wait_for_task timeout (brief §3: a wait holds at most 50 s) ───

/// Default hold when the caller omits `timeout_ms` (Q10 settled).
pub const WAIT_TIMEOUT_DEFAULT_MS: u64 = 25_000;
/// The longest hold the daemon grants.
pub const WAIT_TIMEOUT_MAX_MS: u64 = 50_000;

/// Omitted → default; otherwise clamped to 1 ms – 50 s (Q10 settled).
pub fn clamp_wait_timeout(timeout_ms: Option<u64>) -> u64 {
    let _ = timeout_ms;
    todo!("relay item 2 phase B")
}

// ─── Lane tokens ───

/// `token_hash`: SHA-256 hex of the minted token. The raw token is returned
/// once at register and never stored.
pub fn hash_lane_token(token: &str) -> String {
    let _ = token;
    todo!("relay item 2 phase B")
}

// ─── Leak guard (Q8 settled: fail-closed, kinds only) ───

/// Reject text carrying a secret, surfacing kinds only — never values.
/// Runs over `body_md`, `report_md`, and message/summary text on write.
pub fn guard_markdown(text: &str) -> Result<(), Vec<String>> {
    let _ = text;
    todo!("relay item 2 phase B")
}

// ─── Summary length (CONTRACT §3j) ───

/// Summaries are at most 600 characters.
pub const SUMMARY_MAX_CHARS: usize = 600;

/// Character count check. `Err` carries the actual count for the message.
pub fn check_summary_len(text: &str) -> Result<(), usize> {
    let _ = text;
    todo!("relay item 2 phase B")
}

/// Action kinds whose `body_md` is a summary (capped) rather than a brief or
/// a full report (uncapped). Derived reading: the `merged` note is
/// summary-like ("1–2 sentences", §3g); `assigned`/`brief_updated` bodies are
/// briefs and `report_md` is the full report.
pub fn body_is_summary(kind: RelayActionKind) -> bool {
    let _ = kind;
    todo!("relay item 2 phase B")
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
pub fn validate_props(kind: RelayActionKind, props: &serde_json::Value) -> Result<(), PropsError> {
    let _ = (kind, props);
    todo!("relay item 2 phase B")
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
    let _ = s;
    todo!("relay item 2 phase B")
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
    let _ = (role, crew_id);
    todo!("relay item 2 phase B")
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
    let _ = (caller, given);
    todo!("relay item 2 phase B")
}

// ─── Lease math (Q7, settled) ───

/// A lane lease lives 300 s; every authenticated call renews it.
pub const LANE_LEASE_TTL_SECS: u64 = 300;
/// An interrupted task waits 900 s for its lane, then is re-offered.
pub const REOFFER_GRACE_SECS: u64 = 900;

/// True once `now` reaches the lease's end. Clocks are fed in (tests never
/// read wall time).
pub fn lease_expired(expires_at_unix: i64, now_unix: i64) -> bool {
    let _ = (expires_at_unix, now_unix);
    todo!("relay item 2 phase B")
}

/// True once the grace after the interrupt has run out.
pub fn reoffer_due(interrupted_at_unix: i64, now_unix: i64) -> bool {
    let _ = (interrupted_at_unix, now_unix);
    todo!("relay item 2 phase B")
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
    let _ = (live, author, coordinator);
    todo!("relay item 2 phase B")
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
    fn transition_rework_may_report_directly() {
        // Derived: "the next report returns it to in_review" — a lane that
        // reports from rework without an explicit re-start is accepted.
        assert_eq!(transition(S::Rework, E::Reported), Ok(S::InReview));
    }

    #[test]
    #[ignore = "pending:relay-item2"]
    fn transition_interrupted_then_reoffered_to_assigned() {
        assert_eq!(transition(S::Working, E::LeaseLost), Ok(S::Interrupted));
        assert_eq!(transition(S::InReview, E::LeaseLost), Ok(S::Interrupted));
        assert_eq!(transition(S::Blocked, E::LeaseLost), Ok(S::Interrupted));
        assert_eq!(transition(S::Interrupted, E::Reoffered), Ok(S::Assigned));
        assert!(transition(S::Working, E::Reoffered).is_err());
        assert!(transition(S::Done, E::LeaseLost).is_err());
        assert!(transition(S::Interrupted, E::LeaseLost).is_err());
    }

    #[test]
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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

    #[test]
    #[ignore = "pending:relay-item2"]
    fn clamp_wait_timeout_defaults_and_clamps() {
        assert_eq!(clamp_wait_timeout(None), 25_000);
        assert_eq!(clamp_wait_timeout(Some(5_000)), 5_000);
        assert_eq!(clamp_wait_timeout(Some(50_000)), 50_000);
        assert_eq!(clamp_wait_timeout(Some(60_000)), 50_000);
        assert_eq!(clamp_wait_timeout(Some(0)), 1);
    }

    #[test]
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
    fn summary_len_counts_chars_not_bytes() {
        assert!(check_summary_len(&"x".repeat(600)).is_ok());
        assert_eq!(check_summary_len(&"x".repeat(601)), Err(601));
        assert!(check_summary_len(&"é".repeat(600)).is_ok());
        assert_eq!(check_summary_len(&"é".repeat(601)), Err(601));
    }

    #[test]
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
    fn validate_props_empty_kinds_take_no_keys() {
        use RelayActionKind as K;
        for k in [K::Started, K::Delivered, K::Read] {
            assert!(validate_props(k, &json!({})).is_ok(), "{k:?} takes {{}}");
            assert!(validate_props(k, &json!({"x": 1})).is_err(), "{k:?} rejects keys");
        }
    }

    #[test]
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
    fn transition_reassigned_advances_phase_q11() {
        // Q11 settled: no auto advance; the coordinator's re-assign moves an
        // approved task back to working (and starts a new round, part 2).
        assert_eq!(transition(S::Approved, E::Reassigned), Ok(S::Working));
        for s in [S::Assigned, S::Working, S::InReview, S::Rework, S::Done, S::Blocked] {
            assert!(transition(s, E::Reassigned).is_err(), "{s:?} + reassign");
        }
    }

    #[test]
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
    fn validate_props_asked_of_person_or_uuid_q1() {
        use RelayActionKind as K;
        let asked = |of: &str| json!({"options": [], "blocks": [], "asked_of": of});
        assert!(validate_props(K::Asked, &asked("person")).is_ok());
        assert!(validate_props(K::Asked, &asked(LANE_A)).is_ok());
        assert!(validate_props(K::Asked, &asked("L1")).is_err());
    }
}
