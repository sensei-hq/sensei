//! The lane-facing `get_tasks` / `wait_for_task` envelope (Q4, settled).
//!
//! Grouped response, keys in ruling order — messages, answers, reviews,
//! assignments — each newest-first. Item shapes follow CONTRACT §3; per-group
//! row order is the store's `ORDER BY` (part 2). A timed-out wait returns the
//! empty envelope (Q10).

use serde::{Deserialize, Serialize};

use super::{RelaySurface, RelayTaskPhase, RelayTicketProvider};

/// The grouped envelope. Field order is the ruling's key order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaneEnvelope {
    pub messages: Vec<EnvelopeMessage>,
    pub answers: Vec<EnvelopeAnswer>,
    pub reviews: Vec<EnvelopeReview>,
    pub assignments: Vec<EnvelopeAssignment>,
}

/// The empty envelope: a timed-out wait (Q10), or a lane with nothing queued.
pub fn empty_envelope() -> LaneEnvelope {
    todo!("relay item 2 phase B")
}

/// §3h-bis, as delivered to the recipient lane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeMessage {
    /// The message action's id.
    pub id: String,
    /// "About"; null = no particular task.
    pub task_id: Option<String>,
    /// Who wrote it, by code; null = the person (cf. `lane_code`).
    pub from_lane_code: Option<String>,
    /// `lane` when a lane sent it (Q12).
    pub surface: RelaySurface,
    /// ≤ 600 characters, plain text.
    pub text: String,
    pub created_at: String,
}

/// A decision answer addressed to the lane (Q4 `answers[]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeAnswer {
    /// The `answered` action's id.
    pub id: String,
    /// `reply_to`: the `asked` action's id.
    pub decision_id: String,
    /// The ask's task (crew-level asks carry none).
    pub task_id: Option<String>,
    pub option: String,
    /// The answer's optional note.
    pub note: Option<String>,
    /// Who answered, by code; null = the person.
    pub from_lane_code: Option<String>,
    pub created_at: String,
}

/// A review for the lane. Core fields only — identity and detail wait on Q15
/// (to-perform vs to-answer vs both).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeReview {
    pub task_id: String,
    pub phase: RelayTaskPhase,
    pub round: i32,
    pub created_at: String,
}

/// §3b, as delivered to the assignee lane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeAssignment {
    pub task_id: String,
    pub title: String,
    pub phase: RelayTaskPhase,
    pub round: i32,
    pub ticket: Option<EnvelopeTicket>,
    pub tags: Vec<String>,
    pub brief: EnvelopeBrief,
    pub acceptance: Vec<String>,
    pub owned_paths: Vec<String>,
    /// Task ids this one waits on.
    pub deps: Vec<String>,
    /// Resource names (`rust-build`, …).
    pub resources: Vec<String>,
    pub created_at: String,
}

/// The ticket's portable identity, resolved on this side (cf. `RelayTicket`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeTicket {
    pub provider: RelayTicketProvider,
    pub key: String,
    pub url: Option<String>,
}

/// §3b `brief`: frozen at assign.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeBrief {
    pub summary: String,
    pub docs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::{RelaySurface as S, RelayTaskPhase as P, RelayTicketProvider as T};

    #[test]
    #[ignore = "pending:relay-item2"]
    fn envelope_keys_serialize_in_ruling_order() {
        let json = serde_json::to_string(&empty_envelope()).unwrap();
        assert_eq!(json, r#"{"messages":[],"answers":[],"reviews":[],"assignments":[]}"#);
    }

    #[test]
    #[ignore = "pending:relay-item2"]
    fn envelope_message_round_trip() {
        let m = EnvelopeMessage {
            id: "m1".into(),
            task_id: Some("t1".into()),
            from_lane_code: None,
            surface: S::Desktop,
            text: "hi".into(),
            created_at: "2026-11-05T16:00:00Z".into(),
        };
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["from_lane_code"], serde_json::Value::Null);
        assert_eq!(v["surface"], "desktop");
        assert_eq!(serde_json::from_value::<EnvelopeMessage>(v).unwrap(), m);
        // A lane-sent message carries surface `lane` (Q12).
        let lane = EnvelopeMessage {
            id: "m2".into(),
            task_id: None,
            from_lane_code: Some("L2".into()),
            surface: S::Lane,
            text: "ack".into(),
            created_at: "2026-11-05T16:01:00Z".into(),
        };
        assert_eq!(serde_json::to_value(&lane).unwrap()["surface"], "lane");
    }

    #[test]
    #[ignore = "pending:relay-item2"]
    fn envelope_answer_round_trip() {
        let a = EnvelopeAnswer {
            id: "a1".into(),
            decision_id: "d1".into(),
            task_id: None,
            option: "db".into(),
            note: Some("go".into()),
            from_lane_code: None,
            created_at: "2026-11-05T16:02:00Z".into(),
        };
        let v = serde_json::to_value(&a).unwrap();
        assert_eq!(v["decision_id"], "d1");
        assert_eq!(v["note"], "go");
        assert_eq!(serde_json::from_value::<EnvelopeAnswer>(v).unwrap(), a);
    }

    #[test]
    #[ignore = "pending:relay-item2"]
    fn envelope_assignment_round_trip() {
        let a = EnvelopeAssignment {
            task_id: "t1".into(),
            title: "T".into(),
            phase: P::Tests,
            round: 1,
            ticket: Some(EnvelopeTicket { provider: T::Github, key: "SEN-52".into(), url: None }),
            tags: vec!["sprint 47".into()],
            brief: EnvelopeBrief { summary: "Do it.".into(), docs: vec![] },
            acceptance: vec!["green".into()],
            owned_paths: vec!["app/**".into()],
            deps: vec![],
            resources: vec!["rust-build".into()],
            created_at: "2026-11-05T16:03:00Z".into(),
        };
        let v = serde_json::to_value(&a).unwrap();
        assert_eq!(v["phase"], "tests");
        assert_eq!(v["ticket"]["provider"], "github");
        assert_eq!(serde_json::from_value::<EnvelopeAssignment>(v).unwrap(), a);
    }

    #[test]
    #[ignore = "pending:relay-item2"]
    fn envelope_review_core_shape_q15() {
        // Q15 pending: only the core all directions share is pinned here.
        let r = EnvelopeReview {
            task_id: "t1".into(),
            phase: P::Impl,
            round: 2,
            created_at: "2026-11-05T16:04:00Z".into(),
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["round"], 2);
        assert_eq!(serde_json::from_value::<EnvelopeReview>(v).unwrap(), r);
    }
}
