//! Project a daemon-owned run (`activity.runs`) into the relay status snapshot.
//!
//! This is the PURE, testable core of the P1 run→relay supervision bridge: it
//! maps a [`crate::runs::Run`] (plus the timestamp of its newest cadence event)
//! onto the [`dojo_protocol::relay::RelaySessionUpdate`] wire type the phone /
//! console render. The impure part (read the run + latest event from Postgres,
//! resolve the owning membership device token, then publish via
//! [`crate::dojo::client::DojoClient::publish_session_update`]) wraps this in
//! [`crate::tasks::handlers::publish_run`].
//!
//! Contrast [`crate::dojo::relay_project`], which projects a *TodoWrite* list for
//! a session-keyed run (P2) and always reports `status = Running` with no
//! heartbeat. This module carries the run's REAL `status` (including the
//! watchdog's `stalled`) and its liveness `heartbeat_at`, so the phone can badge
//! staleness and surface a stall/crash — the supervision the run engine already
//! tracks locally, federated to Relay.
//!
//! **Zero-knowledge (D10):** only logical status crosses — the run's status,
//! progress counts, phase/feature labels, and timestamps. Never code, diffs, or
//! tool output. `goal` is the run's own short goal string (already a human label,
//! not a prompt body); the daemon set it, so it is safe to mirror.

use crate::runs::{Run, RunEvent, RunEventKind};
use dojo_protocol::relay::{RelaySegment, RelaySessionUpdate, SegmentState};

/// A short, single-line label bounded to `max` chars (ellipsized). Keeps the
/// mirrored `goal` a human phrase, never an unbounded prompt body. Shares the
/// spirit of `advance_run::short_label` but is inlined here to keep this module
/// dependency-free and independently testable.
fn bounded_label(s: &str, max: usize) -> String {
    let first = s.lines().next().unwrap_or("").trim();
    if first.chars().count() <= max {
        first.to_string()
    } else {
        let mut out: String = first.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Derive the run's phone title. Prefers the current feature (the most specific
/// "what it's doing now" label), then the current phase, then a bounded slice of
/// the goal, and finally a stable fallback so the phone never shows a blank.
pub fn run_title(run: &Run) -> String {
    if let Some(feature) = run.current_feature.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        return feature.to_string();
    }
    if let Some(phase) = run.current_phase.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        return phase.to_string();
    }
    if let Some(goal) = run.goal.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        return bounded_label(goal, 80);
    }
    "Autonomous run".to_string()
}

/// Map a run (+ its newest cadence-event timestamp + its plan→segment progress)
/// to the relay status snapshot. Pure — no DB, no clock, no env.
///
/// - `status` is the run's REAL status (mirrors the watchdog's `stalled`, a limit
///   `paused`, a hard-gate `blocked`, terminal `done`/`failed`, an unexpected
///   `crashed`) — NOT the always-`Running` of the TodoWrite path.
/// - `heartbeat_at` is the run's liveness ping; the phone badges staleness from
///   its age (no update in ~5 min ⇒ stale).
/// - `progress_done`/`progress_total` come from the projected plan segments so
///   the phone shows a real rollup for a `start_run` run (which has no
///   TodoWrite). `last_event_at` drives the "last progress N min ago" line.
///
/// `goal` is mirrored as a bounded label (never an unbounded prompt). Every
/// optional field is `None` when the run has no value for it, so the wire stays
/// minimal and the Worker upsert writes NULLs rather than empty strings.
pub fn run_to_session_update(
    run: &Run,
    last_event_at: Option<&str>,
    progress_done: i32,
    progress_total: i32,
) -> RelaySessionUpdate {
    RelaySessionUpdate {
        run_id: run.id.to_string(),
        status: run.status,
        title: run_title(run),
        goal: run
            .goal
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|g| bounded_label(g, 200)),
        progress_done,
        progress_total,
        current_phase: run
            .current_phase
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        current_feature: run
            .current_feature
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        last_event_at: last_event_at.map(str::to_string),
        paused_until: run.paused_until.clone(),
        pause_reason: run.pause_reason.clone(),
        heartbeat_at: run.heartbeat_at.clone(),
        // Set by the impure federation path (publish_run) from the run's project;
        // the pure projection has no DB to resolve the namespace slug / project row.
        project_slug: None,
        project: None,
    }
}

/// Project a run's plan into the relay outline (`RelaySegment[]`) — the phone's
/// Phases view for a `start_run` run, which (unlike the TodoWrite path) has no
/// per-todo segments.
///
/// The daemon has no plan-doc parser and no plan-phases table: a run's plan is
/// expressed through its cadence log (`activity.run_events`) — each phase surfaces
/// as `phase_started`/`phase_done` events carrying the `phase` label. So the
/// committed plan's phases ARE the distinct phase labels the run has advanced
/// through, in first-seen order. This projects them (newest-relevant kind wins
/// per phase) into one top-level segment per phase.
///
/// Pure — takes the run's events (any order; typically newest-first from
/// [`crate::db::pg_store::PgStore::list_run_events`]) and returns the outline in
/// first-seen (chronological) phase order with `seq` = index. Server ids are
/// `None` (the Worker upserts by session+seq). Zero-knowledge: only the `phase`
/// label (a short plan label the daemon set) becomes a title — never detail/code.
pub fn plan_events_to_segments(events: &[RunEvent]) -> Vec<RelaySegment> {
    // Sort a copy oldest-first so first-seen order is chronological regardless of
    // the caller's ordering (list_run_events is newest-first). `id` is a
    // monotonic bigserial, so it is a stable chronological key even when two
    // events share a timestamp.
    let mut ordered: Vec<&RunEvent> = events.iter().collect();
    ordered.sort_by_key(|e| e.id);

    // First-seen phase order + the strongest state seen for each phase.
    let mut phases: Vec<String> = Vec::new();
    let mut state_by_phase: std::collections::HashMap<String, SegmentState> =
        std::collections::HashMap::new();

    for e in ordered {
        let Some(phase) = e.phase.as_deref().map(str::trim).filter(|p| !p.is_empty()) else {
            continue;
        };
        if let Some(state) = phase_event_state(e.kind) {
            if !phases.iter().any(|p| p == phase) {
                phases.push(phase.to_string());
            }
            // Later events override earlier ones (a phase that started then
            // finished ends `Done`; one that later stalled ends `Blocked`).
            state_by_phase.insert(phase.to_string(), state);
        }
    }

    phases
        .into_iter()
        .enumerate()
        .map(|(i, phase)| {
            let state = state_by_phase.get(&phase).copied().unwrap_or(SegmentState::Pending);
            RelaySegment {
                id: None,
                parent_id: None,
                parent_seq: None,
                seq: i as i32,
                title: phase,
                summary: None,
                detail: None,
                state,
                is_gate: false,
                gate_severity: None,
                response_verdict: None,
                response_note: None,
                agent: None,
                model: None,
                spec_ref: None,
            }
        })
        .collect()
}

/// The segment state a phase takes from one cadence-event kind, or `None` for a
/// kind that says nothing about a phase's state (so it neither creates nor
/// changes a phase segment). A phase that only ever `phase_started` is `Active`;
/// a `phase_done` marks it `Done`; a run-level failure/stall/crash/block on a
/// phase reflects onto that phase.
fn phase_event_state(kind: RunEventKind) -> Option<SegmentState> {
    match kind {
        RunEventKind::PhaseStarted => Some(SegmentState::Active),
        RunEventKind::PhaseDone => Some(SegmentState::Done),
        RunEventKind::Failed => Some(SegmentState::Failed),
        RunEventKind::Crashed | RunEventKind::Stalled => Some(SegmentState::Blocked),
        RunEventKind::GateRaised => Some(SegmentState::Blocked),
        RunEventKind::GatePassed => Some(SegmentState::Active),
        // Feature/housekeeping/limit/etc. events don't themselves define a phase's
        // outline state (they carry a `feature`, not a phase transition).
        _ => None,
    }
}

/// Progress rollup for the session update from projected plan segments:
/// `(done, total)` where `total` = segment count and `done` = count in the
/// terminal `Done` state. Matches the TodoWrite path's rollup so the phone reads
/// both run kinds the same way.
/// Map a dōjō membership `kind` (`employer | client | community | personal`) to a
/// `dojo.project_classification` (`company | client | personal | community`) for a
/// project bound to that membership. Only `employer → company` differs; an unknown
/// / unbound kind falls to `personal` (an unbound project is the user's own). Pure.
pub fn kind_to_classification(kind: &str) -> &'static str {
    match kind {
        "employer" => "company",
        "client" => "client",
        "community" => "community",
        _ => "personal",
    }
}

pub fn segment_progress(segments: &[RelaySegment]) -> (i32, i32) {
    let total = segments.len() as i32;
    let done = segments.iter().filter(|s| s.state == SegmentState::Done).count() as i32;
    (done, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dojo_protocol::relay::RelayRunStatus;
    use uuid::Uuid;

    #[test]
    fn kind_to_classification_maps_employer_to_company_else_identity() {
        assert_eq!(kind_to_classification("employer"), "company");
        assert_eq!(kind_to_classification("client"), "client");
        assert_eq!(kind_to_classification("community"), "community");
        assert_eq!(kind_to_classification("personal"), "personal");
    }

    #[test]
    fn kind_to_classification_unknown_falls_to_personal() {
        assert_eq!(kind_to_classification(""), "personal");
        assert_eq!(kind_to_classification("bogus"), "personal");
    }

    fn run(over: impl FnOnce(&mut Run)) -> Run {
        let mut r = Run {
            id: Uuid::from_u128(0x1111),
            project_id: None,
            plan_ref: "docs/plan/relay-engine.md".into(),
            goal: Some("ship the relay supervision bridge".into()),
            status: RelayRunStatus::Running,
            paused_until: None,
            pause_reason: None,
            current_phase: Some("P1".into()),
            current_feature: Some("run→relay bridge".into()),
            dojo_session_id: None,
            max_concurrency: 1,
            started_at: "2026-07-24T10:00:00Z".into(),
            completed_at: None,
            heartbeat_at: Some("2026-07-24T10:05:00Z".into()),
            created_at: "2026-07-24T10:00:00Z".into(),
            updated_at: "2026-07-24T10:05:00Z".into(),
        };
        over(&mut r);
        r
    }

    #[test]
    fn maps_status_heartbeat_and_progress() {
        let r = run(|_| {});
        let u = run_to_session_update(&r, Some("2026-07-24T10:04:30Z"), 2, 5);
        assert_eq!(u.run_id, Uuid::from_u128(0x1111).to_string());
        assert_eq!(u.status, RelayRunStatus::Running);
        assert_eq!(u.progress_done, 2);
        assert_eq!(u.progress_total, 5);
        assert_eq!(u.last_event_at.as_deref(), Some("2026-07-24T10:04:30Z"));
        // Heartbeat is carried through from the run (the staleness signal).
        assert_eq!(u.heartbeat_at.as_deref(), Some("2026-07-24T10:05:00Z"));
        assert_eq!(u.current_phase.as_deref(), Some("P1"));
        assert_eq!(u.current_feature.as_deref(), Some("run→relay bridge"));
    }

    #[test]
    fn surfaces_the_watchdog_stalled_status() {
        // The whole point of the bridge over the TodoWrite path: a run the
        // watchdog flipped to `stalled` publishes `stalled`, not `running`.
        let r = run(|r| r.status = RelayRunStatus::Stalled);
        let u = run_to_session_update(&r, None, 0, 0);
        assert_eq!(u.status, RelayRunStatus::Stalled);
        assert!(u.status.needs_attention(), "stalled drives the 'needs you' band");
    }

    #[test]
    fn surfaces_a_limit_pause_with_reason_and_resume() {
        let r = run(|r| {
            r.status = RelayRunStatus::Paused;
            r.paused_until = Some("2026-07-24T11:29:00Z".into());
            r.pause_reason = Some("rate/usage limit reached".into());
        });
        let u = run_to_session_update(&r, None, 1, 4);
        assert_eq!(u.status, RelayRunStatus::Paused);
        assert_eq!(u.paused_until.as_deref(), Some("2026-07-24T11:29:00Z"));
        assert_eq!(u.pause_reason.as_deref(), Some("rate/usage limit reached"));
    }

    #[test]
    fn title_prefers_feature_then_phase_then_goal_then_fallback() {
        assert_eq!(run_title(&run(|_| {})), "run→relay bridge"); // feature wins
        assert_eq!(run_title(&run(|r| r.current_feature = None)), "P1", "falls back to phase");
        assert_eq!(
            run_title(&run(|r| {
                r.current_feature = None;
                r.current_phase = None;
            })),
            "ship the relay supervision bridge",
            "then the goal"
        );
        assert_eq!(
            run_title(&run(|r| {
                r.current_feature = None;
                r.current_phase = None;
                r.goal = None;
            })),
            "Autonomous run",
            "then a stable fallback (never blank)"
        );
    }

    #[test]
    fn blank_optional_fields_map_to_none_not_empty_strings() {
        let r = run(|r| {
            r.current_feature = Some("   ".into());
            r.current_phase = Some("".into());
            r.goal = Some("  ".into());
        });
        let u = run_to_session_update(&r, None, 0, 0);
        assert!(u.current_feature.is_none(), "whitespace feature → None");
        assert!(u.current_phase.is_none(), "empty phase → None");
        assert!(u.goal.is_none(), "whitespace goal → None");
        // Title still resolves (falls through to the stable fallback).
        assert_eq!(u.title, "Autonomous run");
    }

    #[test]
    fn goal_is_bounded_and_single_line() {
        let long = "x".repeat(500);
        let r = run(|r| r.goal = Some(format!("{long}\nsecond line")));
        let u = run_to_session_update(&r, None, 0, 0);
        let goal = u.goal.unwrap();
        assert!(goal.chars().count() <= 200, "goal is bounded: {}", goal.chars().count());
        assert!(!goal.contains('\n'), "goal is a single line");
        assert!(goal.ends_with('…'), "over-long goal is ellipsized");
    }

    #[test]
    fn terminal_run_mirrors_done() {
        let r = run(|r| {
            r.status = RelayRunStatus::Done;
            r.completed_at = Some("2026-07-24T12:00:00Z".into());
        });
        let u = run_to_session_update(&r, Some("2026-07-24T12:00:00Z"), 5, 5);
        assert_eq!(u.status, RelayRunStatus::Done);
        assert!(u.status.is_terminal());
    }

    // ── plan-as-run projector (d) ──────────────────────────────────────

    fn ev(id: i64, kind: RunEventKind, phase: Option<&str>, feature: Option<&str>) -> RunEvent {
        RunEvent {
            id,
            run_id: Uuid::from_u128(0x1111),
            kind,
            phase: phase.map(str::to_string),
            feature: feature.map(str::to_string),
            detail: serde_json::json!({}),
            created_at: "2026-07-24T10:00:00Z".into(),
        }
    }

    #[test]
    fn plan_segments_are_phases_in_first_seen_order() {
        // Events arrive newest-first (as list_run_events returns them); the
        // projector must still emit phases in chronological (first-seen) order.
        let events = vec![
            ev(4, RunEventKind::PhaseStarted, Some("P2"), None),
            ev(3, RunEventKind::PhaseDone, Some("P1"), None),
            ev(2, RunEventKind::FeatureDone, Some("P1"), Some("bridge")),
            ev(1, RunEventKind::PhaseStarted, Some("P1"), None),
        ];
        let segs = plan_events_to_segments(&events);
        assert_eq!(segs.len(), 2, "two distinct phases");
        assert_eq!(segs[0].title, "P1");
        assert_eq!(segs[0].seq, 0);
        assert_eq!(segs[0].state, SegmentState::Done, "P1 started then done");
        assert_eq!(segs[1].title, "P2");
        assert_eq!(segs[1].seq, 1);
        assert_eq!(segs[1].state, SegmentState::Active, "P2 started, not done");
        // Server ids are None; the Worker upserts by session+seq.
        assert!(segs.iter().all(|s| s.id.is_none() && s.parent_id.is_none() && !s.is_gate));
    }

    #[test]
    fn later_event_overrides_a_phase_state() {
        // A phase that started, then the run stalled on it → Blocked (the later
        // event wins over the earlier PhaseStarted's Active).
        let events = vec![
            ev(1, RunEventKind::PhaseStarted, Some("P1"), None),
            ev(2, RunEventKind::Stalled, Some("P1"), None),
        ];
        let segs = plan_events_to_segments(&events);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].state, SegmentState::Blocked);
    }

    #[test]
    fn events_without_a_phase_or_state_are_ignored() {
        // Feature/housekeeping events (no phase transition) and phase-less events
        // never create a segment.
        let events = vec![
            ev(1, RunEventKind::Housekeeping, None, None),
            ev(2, RunEventKind::FeatureStarted, Some("P1"), Some("x")), // feature, not a phase transition
            ev(3, RunEventKind::PhaseStarted, None, None),              // no phase label
        ];
        let segs = plan_events_to_segments(&events);
        assert!(segs.is_empty(), "no phase-transition event ⇒ no segments, got {segs:?}");
    }

    #[test]
    fn segment_progress_counts_done_over_total() {
        let events = vec![
            ev(1, RunEventKind::PhaseDone, Some("P1"), None),
            ev(2, RunEventKind::PhaseDone, Some("P2"), None),
            ev(3, RunEventKind::PhaseStarted, Some("P3"), None),
        ];
        let segs = plan_events_to_segments(&events);
        assert_eq!(segment_progress(&segs), (2, 3), "2 of 3 phases done");
        assert_eq!(segment_progress(&[]), (0, 0), "empty outline is 0/0");
    }
}
