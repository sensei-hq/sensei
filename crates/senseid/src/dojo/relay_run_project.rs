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

use crate::runs::Run;
use dojo_protocol::relay::RelaySessionUpdate;

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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dojo_protocol::relay::RelayRunStatus;
    use uuid::Uuid;

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
        assert_eq!(
            run_title(&run(|r| r.current_feature = None)),
            "P1",
            "falls back to phase"
        );
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
}
