//! Relay-engine (P3.6) — pure watchdog state-transition core.
//!
//! **Degrade, never stop (§5).** An autonomous run must survive a hung agent
//! step. The scheduler ([`crate::tasks::watchdog_scheduler`]) reads each
//! recoverable run's status + heartbeat and asks this module *what to do*; this
//! module is the pure decision, the scheduler is the DB-writing side.
//!
//! **The escalation ladder** (relay-engine §5 "Watchdog / crashed", §Liveness):
//!
//! ```text
//!  running  ──(heartbeat stale)──▶  stalled
//!  stalled  ──(still stale, attempts < max)──▶  running   (bounded auto-recovery)
//!  stalled  ──(still stale, attempts >= max)─▶  crashed   (give up, surface + push)
//! ```
//!
//! A hung step is noticed (`Stall`), retried a bounded number of times
//! (`Recover`), and only escalated to `crashed` (`Crash`) once the bounded
//! recovery is exhausted. Real progress (a clean drive step) resets the counter
//! ([`crate::db::pg_store::PgStore::reset_run_recovery`]) so a long overnight run
//! that recovered earlier does not prematurely give up later.
//!
//! **The heartbeat-ownership invariant.** Staleness is measured against
//! `heartbeat_at`. [`crate::tasks::handlers::advance_run`] heartbeats `Running`
//! (and `Blocked`) runs every tick, but it deliberately does **NOT** touch
//! `stalled` runs — the watchdog exclusively owns them. If advance_run kept
//! heartbeating a stalled run its heartbeat would never go stale and this
//! module could never escalate it.
//!
//! Everything here is **pure + unit-testable** (no clock reads, no DB, no env):
//! `now`, `last_seen`, and `cfg` are all passed in, so the whole ladder can be
//! exercised with fixed inputs. The caller supplies `chrono::Utc::now()` +
//! [`WatchdogConfig::default`] at the boundary.

use chrono::{DateTime, Utc};
use dojo_protocol::relay::RelayRunStatus;

/// Watchdog tuning. `stale_after` is how long a run's heartbeat may go untouched
/// before it is considered stale; `max_recovery` is the bounded number of
/// auto-recovery attempts before a still-stalled run escalates to `crashed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchdogConfig {
    /// How long a run's **agent progress** may go untouched before a `running`
    /// run is a stall (Jerry: "no update in 5 min"). Measured against the newest
    /// *progress* event (not the every-tick housekeeping heartbeat).
    pub progress_stale_after: std::time::Duration,
    /// How long a `stalled` run's **heartbeat** may sit untouched before the
    /// daemon-crash recovery ladder escalates it (recover → crash). Separate from
    /// `progress_stale_after`: this is "the daemon stopped servicing the run",
    /// not "the agent went quiet".
    pub stale_after: std::time::Duration,
    /// Max bounded auto-recovery attempts before escalating to `crashed`.
    pub max_recovery: i32,
}

impl Default for WatchdogConfig {
    /// `progress_stale_after = 5 min`, `stale_after = 20 min`, `max_recovery = 3`.
    ///
    /// The 5-min progress window is the agent-stuck signal. The 20-min heartbeat
    /// `stale_after` governs the *stalled* run's daemon-crash ladder and **MUST
    /// exceed the 600s (`DRIVE_TIMEOUT`) agent-drive cap** so a legitimately
    /// in-flight drive step is never escalated mid-step. (Enabling `drive` with a
    /// sub-600s progress window would false-stall long steps — see
    /// `docs/analysis/2026-07-24-relay-stall-signal.md`; drive stays OFF for now.)
    fn default() -> Self {
        Self {
            progress_stale_after: std::time::Duration::from_secs(300),
            stale_after: std::time::Duration::from_secs(1200),
            max_recovery: 3,
        }
    }
}

/// What the watchdog should do with one run, decided purely from its status +
/// heartbeat age. The scheduler turns each variant into the matching status
/// change + `run_event` (see [`crate::tasks::watchdog_scheduler`]).
#[derive(Debug, Clone, PartialEq)]
pub enum WatchdogAction {
    /// Heartbeat fresh, or the status isn't one the watchdog acts on → no-op.
    Healthy,
    /// A `running` run whose heartbeat went stale → mark `stalled` (noticed;
    /// handed to bounded recovery on a later tick).
    Stall,
    /// A `stalled` run still stale with attempts below the cap → back to
    /// `running` (bounded retry). `attempt` is the new (incremented) count.
    Recover { attempt: i32 },
    /// A `stalled` run still stale with attempts at/over the cap → `crashed`
    /// (give up, surface). Bounded recovery is exhausted.
    Crash,
}

/// Whether an age is past a window. A negative/future age (clock skew) clamps to
/// fresh. A pathological (unrepresentable) window → fresh, so a bad config never
/// falsely escalates live runs.
fn is_stale(now: DateTime<Utc>, since: DateTime<Utc>, window: std::time::Duration) -> bool {
    match chrono::Duration::from_std(window) {
        Ok(w) => (now - since) > w,
        Err(_) => false,
    }
}

/// Decide what the watchdog should do with one run — pure and deterministic.
/// Two independent liveness signals, both injected so the whole ladder is
/// testable with fixed inputs:
///
/// - `last_progress` — the newest **agent-progress** event (or `started_at` when
///   the run has made none). Drives the *agent-stuck* signal.
/// - `last_heartbeat` — the run's `heartbeat_at` (or `started_at`). Drives the
///   *daemon-crash* recovery ladder for an already-`stalled` run.
///
/// Logic:
/// - `Running` + no progress within `cfg.progress_stale_after` → [`Stall`]
///   (Jerry: "no update in 5 min" — surface + nudge); else [`Healthy`].
/// - `Stalled` + heartbeat stale past `cfg.stale_after` → the daemon-crash
///   ladder: [`Recover`] while `recovery_attempts < max_recovery`, else
///   [`Crash`]. Heartbeat-fresh (the daemon is still ticking; the agent is just
///   quiet) → [`Healthy`]: it *stays* stalled, nudgeable, until the agent emits
///   progress (which revives it via the phase bridge) or a human acts.
/// - `Paused` (intentional / limit-wait — auto-resumes), `Blocked`
///   (waiting-on-human), terminal → [`Healthy`].
pub fn assess_run(
    status: RelayRunStatus,
    last_progress: DateTime<Utc>,
    last_heartbeat: DateTime<Utc>,
    now: DateTime<Utc>,
    recovery_attempts: i32,
    cfg: &WatchdogConfig,
) -> WatchdogAction {
    match status {
        // Agent-stuck signal: a running run that hasn't progressed in the window
        // is a stall (nudgeable). "Progress" excludes the every-tick housekeeping.
        RelayRunStatus::Running => {
            if is_stale(now, last_progress, cfg.progress_stale_after) {
                WatchdogAction::Stall
            } else {
                WatchdogAction::Healthy
            }
        }
        // Daemon-crash ladder: keyed on the heartbeat (which freezes once a run is
        // stalled, since advance_run stops heartbeating it). A stalled run whose
        // heartbeat is still fresh is left alone (agent quiet, daemon alive).
        RelayRunStatus::Stalled => {
            if is_stale(now, last_heartbeat, cfg.stale_after) {
                if recovery_attempts < cfg.max_recovery {
                    WatchdogAction::Recover { attempt: recovery_attempts + 1 }
                } else {
                    WatchdogAction::Crash
                }
            } else {
                WatchdogAction::Healthy
            }
        }
        RelayRunStatus::Paused
        | RelayRunStatus::Blocked
        | RelayRunStatus::Done
        | RelayRunStatus::Failed
        | RelayRunStatus::Crashed => WatchdogAction::Healthy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    /// A fixed `now` for deterministic assertions — 2026-07-17 08:00:00 UTC.
    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 17, 8, 0, 0).unwrap()
    }

    /// A timestamp `secs` seconds before `fixed_now()`.
    fn ago(secs: i64) -> DateTime<Utc> {
        fixed_now() - Duration::seconds(secs)
    }

    /// A just-now timestamp (well inside either window).
    fn fresh() -> DateTime<Utc> {
        ago(1)
    }

    fn cfg() -> WatchdogConfig {
        WatchdogConfig::default()
    }

    #[test]
    fn default_config_is_5min_progress_20min_stale_3_recoveries() {
        let c = WatchdogConfig::default();
        assert_eq!(c.progress_stale_after, std::time::Duration::from_secs(300), "5 min");
        assert_eq!(c.stale_after, std::time::Duration::from_secs(1200), "20 min");
        assert_eq!(c.max_recovery, 3);
        // The stalled-run heartbeat ladder MUST exceed the 600s drive cap so an
        // in-flight drive step is never escalated mid-step.
        assert!(c.stale_after.as_secs() > 600, "stale_after must exceed DRIVE_TIMEOUT (600s)");
    }

    // ── running: gated on PROGRESS age (agent-stuck) ────────────────────

    #[test]
    fn running_with_fresh_progress_is_healthy() {
        // Progress fresh; heartbeat age is irrelevant for a running run.
        assert_eq!(
            assess_run(RelayRunStatus::Running, fresh(), ago(9999), fixed_now(), 0, &cfg()),
            WatchdogAction::Healthy
        );
    }

    #[test]
    fn running_with_stale_progress_stalls() {
        // 6 min > 5 min progress window → Stall, even though the heartbeat is
        // fresh (the daemon is ticking — this is the bug the old heartbeat-only
        // signal missed).
        assert_eq!(
            assess_run(RelayRunStatus::Running, ago(6 * 60), fresh(), fixed_now(), 0, &cfg()),
            WatchdogAction::Stall
        );
    }

    #[test]
    fn running_progress_boundary_is_inclusive_fresh() {
        // Exactly 300s → fresh (check is `>`); 301s → stale.
        assert_eq!(
            assess_run(RelayRunStatus::Running, ago(300), fresh(), fixed_now(), 0, &cfg()),
            WatchdogAction::Healthy,
            "exact-boundary progress age is still fresh"
        );
        assert_eq!(
            assess_run(RelayRunStatus::Running, ago(301), fresh(), fixed_now(), 0, &cfg()),
            WatchdogAction::Stall,
            "one second past the progress window is stale"
        );
    }

    // ── stalled: gated on HEARTBEAT age (daemon-crash ladder) ───────────

    #[test]
    fn stalled_with_fresh_heartbeat_stays_stalled() {
        // Agent quiet (progress a day old) but the daemon is alive (heartbeat
        // fresh) → leave it stalled + nudgeable; do NOT recover/crash.
        assert_eq!(
            assess_run(RelayRunStatus::Stalled, ago(86_400), fresh(), fixed_now(), 0, &cfg()),
            WatchdogAction::Healthy
        );
    }

    #[test]
    fn stalled_and_heartbeat_stale_recovers_while_under_the_cap() {
        let hb = ago(21 * 60); // 21 min > 20 min heartbeat window
        assert_eq!(
            assess_run(RelayRunStatus::Stalled, ago(86_400), hb, fixed_now(), 0, &cfg()),
            WatchdogAction::Recover { attempt: 1 }
        );
        assert_eq!(
            assess_run(RelayRunStatus::Stalled, ago(86_400), hb, fixed_now(), 2, &cfg()),
            WatchdogAction::Recover { attempt: 3 }
        );
    }

    #[test]
    fn stalled_and_heartbeat_stale_crashes_at_and_over_the_cap() {
        let hb = ago(21 * 60);
        assert_eq!(
            assess_run(RelayRunStatus::Stalled, ago(86_400), hb, fixed_now(), 3, &cfg()),
            WatchdogAction::Crash
        );
        assert_eq!(
            assess_run(RelayRunStatus::Stalled, ago(86_400), hb, fixed_now(), 4, &cfg()),
            WatchdogAction::Crash
        );
    }

    // ── non-acted statuses → Healthy (both signals stale) ───────────────

    #[test]
    fn paused_blocked_terminal_are_healthy_even_when_stale() {
        let old = ago(24 * 60 * 60); // a full day stale on both signals
        for status in [
            RelayRunStatus::Paused,  // intentional / limit-wait — auto-resumes
            RelayRunStatus::Blocked, // waiting on a human — not a stall
            RelayRunStatus::Done,
            RelayRunStatus::Failed,
            RelayRunStatus::Crashed,
        ] {
            assert_eq!(
                assess_run(status, old, old, fixed_now(), 0, &cfg()),
                WatchdogAction::Healthy,
                "{status:?} is never escalated by the watchdog"
            );
        }
    }

    // ── clock-skew edges ────────────────────────────────────────────────

    #[test]
    fn future_timestamps_are_healthy() {
        let future = fixed_now() + Duration::hours(1);
        // Future progress (skew) → running never stalls.
        assert_eq!(
            assess_run(RelayRunStatus::Running, future, fresh(), fixed_now(), 0, &cfg()),
            WatchdogAction::Healthy
        );
        // Future heartbeat (skew) → stalled never recovers off skew.
        assert_eq!(
            assess_run(RelayRunStatus::Stalled, ago(86_400), future, fixed_now(), 0, &cfg()),
            WatchdogAction::Healthy
        );
    }
}
