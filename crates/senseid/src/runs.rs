//! Relay run-state model — the daemon-owned durable state of an autonomous
//! multi-phase run (`activity.runs`) plus its append-only cadence log
//! (`activity.run_events`).
//!
//! This is P3.1: pure data + the DB-facing types. No scheduler, no agent spawn,
//! no HTTP — those live in later P3 chunks. The CRUD access methods hang off
//! [`crate::db::pg_store::PgStore`].
//!
//! **Status enum reuse:** the run's `status` is
//! [`dojo_protocol::relay::RelayRunStatus`] — `sensei.run_status` and
//! `dojo.relay_run_status` share the same 7-variant string set, so we reuse the
//! one authoritative enum (with its own `as_db_str`/`from_db_str` + DDL-match
//! test) rather than duplicating it here. `run_event_kind` has no relay
//! equivalent, so [`RunEventKind`] is defined fresh with the same discipline.

use dojo_protocol::relay::RelayRunStatus;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Kind of a cadence event on a run — mirrors `sensei.run_event_kind`. Logical
/// progress only (feature shipped · gate raised · paused on limit · resumed ·
/// crashed · housekeeping · …); never diffs or raw tool output (relay-engine D10).
///
/// `as_db_str`/`from_db_str` MUST match `run_event_kind.ddl` exactly — the
/// `run_event_kind_db_strings_match_ddl` test enforces it. Note the DDL orders
/// variants alphabetically at deploy time (dbd), but the wire/`ALL` order here
/// is the *declaration* order in the DDL, which is what the test asserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunEventKind {
    PhaseStarted,
    FeatureStarted,
    FeatureDone,
    PhaseDone,
    GateRaised,
    GatePassed,
    Flagged,
    Committed,
    Pushed,
    Merged,
    Bumped,
    PausedOnLimit,
    Resumed,
    Throttled,
    Stalled,
    Crashed,
    Recovered,
    Housekeeping,
    Done,
    Failed,
}

impl RunEventKind {
    /// Every variant, in `run_event_kind.ddl` declaration order.
    pub const ALL: [RunEventKind; 20] = [
        RunEventKind::PhaseStarted,
        RunEventKind::FeatureStarted,
        RunEventKind::FeatureDone,
        RunEventKind::PhaseDone,
        RunEventKind::GateRaised,
        RunEventKind::GatePassed,
        RunEventKind::Flagged,
        RunEventKind::Committed,
        RunEventKind::Pushed,
        RunEventKind::Merged,
        RunEventKind::Bumped,
        RunEventKind::PausedOnLimit,
        RunEventKind::Resumed,
        RunEventKind::Throttled,
        RunEventKind::Stalled,
        RunEventKind::Crashed,
        RunEventKind::Recovered,
        RunEventKind::Housekeeping,
        RunEventKind::Done,
        RunEventKind::Failed,
    ];

    /// True when this event marks *agent* progress (the run actually advancing),
    /// false for the daemon's own cadence/lifecycle bookkeeping. The stall signal
    /// ([`crate::run_watchdog`]) measures "no progress in N min" against progress
    /// events only: `Housekeeping` is logged every tick, so counting it would mean
    /// a run never looks stalled — and pause/resume/throttle/watchdog markers are
    /// the daemon reacting, not the agent working.
    ///
    /// Progress = phase/feature boundaries, gates, and git milestones (commit/
    /// push/merge/bump), plus terminal done/failed/flagged. Not-progress =
    /// housekeeping tick + paused/resumed/throttled/stalled/crashed/recovered.
    pub fn is_progress(&self) -> bool {
        !matches!(
            self,
            RunEventKind::Housekeeping
                | RunEventKind::PausedOnLimit
                | RunEventKind::Resumed
                | RunEventKind::Throttled
                | RunEventKind::Stalled
                | RunEventKind::Crashed
                | RunEventKind::Recovered
        )
    }

    /// The `sensei.run_event_kind` enum string. MUST match `run_event_kind.ddl`.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            RunEventKind::PhaseStarted => "phase_started",
            RunEventKind::FeatureStarted => "feature_started",
            RunEventKind::FeatureDone => "feature_done",
            RunEventKind::PhaseDone => "phase_done",
            RunEventKind::GateRaised => "gate_raised",
            RunEventKind::GatePassed => "gate_passed",
            RunEventKind::Flagged => "flagged",
            RunEventKind::Committed => "committed",
            RunEventKind::Pushed => "pushed",
            RunEventKind::Merged => "merged",
            RunEventKind::Bumped => "bumped",
            RunEventKind::PausedOnLimit => "paused_on_limit",
            RunEventKind::Resumed => "resumed",
            RunEventKind::Throttled => "throttled",
            RunEventKind::Stalled => "stalled",
            RunEventKind::Crashed => "crashed",
            RunEventKind::Recovered => "recovered",
            RunEventKind::Housekeeping => "housekeeping",
            RunEventKind::Done => "done",
            RunEventKind::Failed => "failed",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        RunEventKind::ALL.into_iter().find(|v| v.as_db_str() == s)
    }
}

/// A daemon-owned autonomous run of a multi-phase plan — one row of
/// `activity.runs`. Durable: survives daemon restarts (fixes the vacation run's
/// session-coupled death). `status` is authoritative; the cloud
/// `dojo.relay_sessions` mirrors it. Timestamps are RFC-3339 text (SELECTed
/// `::text`) so the API serializes them without pulling chrono into the row
/// tuple — mirrors [`crate::db::pg_store::DojoMembership`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Run {
    pub id: Uuid,
    /// `sensei.projects(id)` the run belongs to; `NULL` after the project is
    /// deleted (`on delete set null`).
    pub project_id: Option<Uuid>,
    /// Path/ref of the committed plan doc the run executes. NOT NULL, defaults
    /// to `''` in the DDL.
    pub plan_ref: String,
    pub goal: Option<String>,
    pub status: RelayRunStatus,
    /// When a limit pause auto-resumes; `None` while running. RFC-3339 text.
    pub paused_until: Option<String>,
    pub pause_reason: Option<String>,
    pub current_phase: Option<String>,
    pub current_feature: Option<String>,
    /// The cloud `dojo.relay_sessions(id)` mirroring this run — a plain uuid
    /// across the DB boundary (no cross-DB FK).
    pub dojo_session_id: Option<Uuid>,
    /// Active sub-agent cap; throttled to 1 under rate-limit pressure. NOT NULL,
    /// defaults to 1.
    pub max_concurrency: i32,
    /// RFC-3339 text. NOT NULL, defaults to `now()`.
    pub started_at: String,
    pub completed_at: Option<String>,
    pub heartbeat_at: Option<String>,
    /// RFC-3339 text. NOT NULL, defaults to `now()`.
    pub created_at: String,
    /// RFC-3339 text. NOT NULL, defaults to `now()`.
    pub updated_at: String,
}

/// Caller-supplied fields for creating a run. `id`, `status`, and every
/// timestamp are DB-defaulted (`gen_random_uuid()`, `'running'`, `now()`), so
/// they are not part of the create input. `plan_ref`/`max_concurrency` are
/// `Option` here: `None` lets the DDL default (`''` / `1`) apply.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewRun {
    pub project_id: Option<Uuid>,
    pub plan_ref: Option<String>,
    pub goal: Option<String>,
    pub dojo_session_id: Option<Uuid>,
    pub max_concurrency: Option<i32>,
    /// Git author for the run's project (resolved at the create-run handler from
    /// [`crate::git_identity::read_git_user`]); `None` leaves the columns NULL.
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    /// The authored plan graph (phases→tasks with agent/model/spec_ref + per-task
    /// state) for a run seeded via `register_plan`; `None` for ad-hoc runs. Stored
    /// as jsonb; fetched on demand (off the 16-column `RUN_SELECT`).
    pub plan_graph: Option<serde_json::Value>,
}

/// One append-only cadence event on a run — a row of `activity.run_events`.
/// `id` is a `bigserial` (hence `i64`). `detail` is the structured, stripped
/// payload (never code/diffs); it defaults to `{}` in the DDL, never null.
/// `created_at` is RFC-3339 text (SELECTed `::text`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunEvent {
    pub id: i64,
    pub run_id: Uuid,
    pub kind: RunEventKind,
    pub phase: Option<String>,
    pub feature: Option<String>,
    pub detail: serde_json::Value,
    pub created_at: String,
}

/// The run cadence events to append when a run's phase moves from `current` to
/// `new` — the pure core of the workflow→run phase bridge (an agent's
/// `update_phase` mirrored onto its active run so it streams phases→segments to
/// the relay while `drive` stays OFF).
///
/// A real transition pairs `phase_done(prev)` + `phase_started(new)` so
/// [`crate::dojo::relay_run_project`]'s `plan_events_to_segments` renders the
/// previous phase done and the new one active. Re-entering the same phase is a
/// no-op (idempotent — repeated `update_phase` calls don't duplicate segments).
pub fn phase_transition_events(current: Option<&str>, new: &str) -> Vec<(RunEventKind, String)> {
    match current {
        Some(prev) if prev == new => Vec::new(),
        Some(prev) => vec![
            (RunEventKind::PhaseDone, prev.to_string()),
            (RunEventKind::PhaseStarted, new.to_string()),
        ],
        None => vec![(RunEventKind::PhaseStarted, new.to_string())],
    }
}

/// Serializes the DB-gated tests that call a *global* run UPDATE (the pg_store
/// CRUD test, the AdvanceRun scheduler tests, the watchdog scheduler tests) so
/// parallel test threads don't steal each other's runs. Production has a single
/// scheduler calling these, so there is no such race there — this is purely a
/// test-isolation guard.
///
/// See [`crate::tasks::test_support::TestGate`] for why the gate blocks rather
/// than awaiting.
#[cfg(test)]
pub(crate) fn resume_test_guard() -> std::sync::MutexGuard<'static, ()> {
    static GATE: crate::tasks::test_support::TestGate =
        crate::tasks::test_support::TestGate::new();
    GATE.enter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_transition_from_none_starts_the_phase() {
        assert_eq!(
            phase_transition_events(None, "P1"),
            vec![(RunEventKind::PhaseStarted, "P1".to_string())]
        );
    }

    #[test]
    fn phase_transition_pairs_done_and_started() {
        assert_eq!(
            phase_transition_events(Some("P0"), "P1"),
            vec![
                (RunEventKind::PhaseDone, "P0".to_string()),
                (RunEventKind::PhaseStarted, "P1".to_string()),
            ]
        );
    }

    #[test]
    fn phase_transition_same_phase_is_a_noop() {
        assert!(phase_transition_events(Some("P1"), "P1").is_empty());
    }

    #[test]
    fn is_progress_true_for_agent_advancement() {
        for k in [
            RunEventKind::PhaseStarted,
            RunEventKind::FeatureStarted,
            RunEventKind::FeatureDone,
            RunEventKind::GateRaised,
            RunEventKind::Committed,
            RunEventKind::Pushed,
            RunEventKind::Merged,
            RunEventKind::Done,
            RunEventKind::Failed,
        ] {
            assert!(k.is_progress(), "{k:?} is agent progress");
        }
    }

    #[test]
    fn is_progress_false_for_daemon_cadence_and_lifecycle() {
        for k in [
            RunEventKind::Housekeeping,
            RunEventKind::PausedOnLimit,
            RunEventKind::Resumed,
            RunEventKind::Throttled,
            RunEventKind::Stalled,
            RunEventKind::Crashed,
            RunEventKind::Recovered,
        ] {
            assert!(!k.is_progress(), "{k:?} is daemon bookkeeping, not progress");
        }
    }

    #[test]
    fn run_event_kind_db_strings_match_ddl() {
        // Declaration order of `run_event_kind.ddl`.
        let expected = [
            "phase_started",
            "feature_started",
            "feature_done",
            "phase_done",
            "gate_raised",
            "gate_passed",
            "flagged",
            "committed",
            "pushed",
            "merged",
            "bumped",
            "paused_on_limit",
            "resumed",
            "throttled",
            "stalled",
            "crashed",
            "recovered",
            "housekeeping",
            "done",
            "failed",
        ];
        let got: Vec<&str> = RunEventKind::ALL.iter().map(|v| v.as_db_str()).collect();
        assert_eq!(got, expected);
        assert_eq!(RunEventKind::ALL.len(), 20);
        for v in RunEventKind::ALL {
            assert_eq!(RunEventKind::from_db_str(v.as_db_str()), Some(v));
            // serde snake_case must agree with the DB string.
            assert_eq!(
                serde_json::to_string(&v).unwrap(),
                format!("\"{}\"", v.as_db_str())
            );
        }
        assert_eq!(RunEventKind::from_db_str("nope"), None);
    }

    #[test]
    fn run_status_reuses_relay_enum() {
        // The run's status is dojo_protocol's RelayRunStatus — the same 7
        // variants sensei.run_status declares. Sanity-check the shared strings.
        let expected = [
            "running", "paused", "stalled", "crashed", "blocked", "done", "failed",
        ];
        let got: Vec<&str> = RelayRunStatus::ALL.iter().map(|v| v.as_db_str()).collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn run_round_trips_json() {
        let run = Run {
            id: Uuid::nil(),
            project_id: Some(Uuid::nil()),
            plan_ref: "docs/plan/P3.md".into(),
            goal: Some("ship the relay engine".into()),
            status: RelayRunStatus::Running,
            paused_until: None,
            pause_reason: None,
            current_phase: Some("P3".into()),
            current_feature: None,
            dojo_session_id: None,
            max_concurrency: 1,
            started_at: "2026-07-17T10:00:00Z".into(),
            completed_at: None,
            heartbeat_at: Some("2026-07-17T10:05:00Z".into()),
            created_at: "2026-07-17T10:00:00Z".into(),
            updated_at: "2026-07-17T10:05:00Z".into(),
        };
        let js = serde_json::to_string(&run).unwrap();
        let back: Run = serde_json::from_str(&js).unwrap();
        assert_eq!(back, run);
        let v: serde_json::Value = serde_json::from_str(&js).unwrap();
        assert_eq!(v["status"], serde_json::json!("running"));
    }

    #[test]
    fn run_event_round_trips_json() {
        let ev = RunEvent {
            id: 42,
            run_id: Uuid::nil(),
            kind: RunEventKind::PausedOnLimit,
            phase: Some("P3".into()),
            feature: None,
            detail: serde_json::json!({ "reset_at": "2026-07-17T11:29:00Z" }),
            created_at: "2026-07-17T10:00:00Z".into(),
        };
        let js = serde_json::to_string(&ev).unwrap();
        let back: RunEvent = serde_json::from_str(&js).unwrap();
        assert_eq!(back, ev);
        let v: serde_json::Value = serde_json::from_str(&js).unwrap();
        assert_eq!(v["kind"], serde_json::json!("paused_on_limit"));
    }
}
