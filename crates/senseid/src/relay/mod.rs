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

/// `RelayTaskPhase`. No `merge` phase (Q3-REC): merging is the `merged`
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

/// `RelaySurface`. Q12 may add `lane` for lane-sent messages/answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelaySurface {
    Desktop,
    Dojo,
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
    Unblocked { blocked_from: RelayTaskState },
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

/// Default hold when the caller omits `timeout_ms` (Q10-REC).
pub const WAIT_TIMEOUT_DEFAULT_MS: u64 = 25_000;
/// The longest hold the daemon grants.
pub const WAIT_TIMEOUT_MAX_MS: u64 = 50_000;

/// Omitted → default; otherwise clamped to 1 ms – 50 s (Q10-REC).
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

// ─── Leak guard (Q8-REC: fail-closed, kinds only) ───

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
pub fn validate_props(kind: RelayActionKind, props: &serde_json::Value) -> Result<(), PropsError> {
    let _ = (kind, props);
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
        // Q3-REC: no `merge` phase.
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
    fn clamp_wait_timeout_defaults_and_clamps_q10_rec() {
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
    fn guard_accepts_clean_brief_q8_rec() {
        // Q8-REC: no identifier dereference — relay text legitimately names
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
}
