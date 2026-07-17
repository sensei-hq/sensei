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
    /// How long a heartbeat may sit untouched before the run is "stale".
    pub stale_after: std::time::Duration,
    /// Max bounded auto-recovery attempts before escalating to `crashed`.
    pub max_recovery: i32,
}

impl Default for WatchdogConfig {
    /// `stale_after = 20 min`, `max_recovery = 3`.
    ///
    /// **`stale_after` MUST exceed the 600s (`DRIVE_TIMEOUT`) agent-drive cap**
    /// ([`crate::tasks::handlers::advance_run`]) so a run whose drive step is
    /// legitimately in-flight (heartbeat frozen at drive-start for up to the
    /// full drive budget) is never falsely marked stale mid-step. 20 min is a
    /// comfortable margin over the 10-min drive cap.
    fn default() -> Self {
        Self {
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

/// Decide what the watchdog should do with one run — pure and deterministic.
///
/// `now`, `last_seen` (the run's `heartbeat_at`, or `started_at` when a run has
/// never heartbeated), `recovery_attempts`, and `cfg` are all injected so the
/// whole ladder is testable with fixed inputs.
///
/// Logic:
/// - **fresh** (`now - last_seen <= cfg.stale_after`, and negative/future
///   `last_seen` clamped to fresh) → [`WatchdogAction::Healthy`];
/// - **stale** + `Running` → [`WatchdogAction::Stall`];
/// - **stale** + `Stalled` → [`WatchdogAction::Recover`] while
///   `recovery_attempts < cfg.max_recovery`, else [`WatchdogAction::Crash`];
/// - **stale** + any other status (`Paused` = intentional, `Blocked` =
///   waiting-on-human, `Done`/`Failed`/`Crashed` = terminal) →
///   [`WatchdogAction::Healthy`] (nothing to recover).
pub fn assess_run(
    status: RelayRunStatus,
    last_seen: DateTime<Utc>,
    now: DateTime<Utc>,
    recovery_attempts: i32,
    cfg: &WatchdogConfig,
) -> WatchdogAction {
    // Heartbeat age. A negative age (last_seen in the future — clock skew) is
    // clamped to fresh: a run we just heard from (or from the future) is never
    // stale.
    let age = now - last_seen;
    let stale_after = match chrono::Duration::from_std(cfg.stale_after) {
        Ok(d) => d,
        // A pathological (unrepresentable) config duration → treat everything as
        // fresh rather than falsely escalating live runs.
        Err(_) => return WatchdogAction::Healthy,
    };
    if age <= stale_after {
        return WatchdogAction::Healthy;
    }

    // Stale. Only running/stalled runs are acted on; every other status is a
    // deliberate no-op (paused = intentional, blocked = waiting on a human,
    // done/failed/crashed = terminal, nothing to recover).
    match status {
        RelayRunStatus::Running => WatchdogAction::Stall,
        RelayRunStatus::Stalled => {
            if recovery_attempts < cfg.max_recovery {
                WatchdogAction::Recover { attempt: recovery_attempts + 1 }
            } else {
                WatchdogAction::Crash
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

    /// A heartbeat `secs` seconds before `fixed_now()`.
    fn seen_ago(secs: i64) -> DateTime<Utc> {
        fixed_now() - Duration::seconds(secs)
    }

    fn cfg() -> WatchdogConfig {
        WatchdogConfig::default()
    }

    #[test]
    fn default_config_is_20min_stale_and_3_recoveries() {
        let c = WatchdogConfig::default();
        assert_eq!(c.stale_after, std::time::Duration::from_secs(1200), "20 min");
        assert_eq!(c.max_recovery, 3);
        // MUST exceed the 600s drive cap so an in-flight drive is never falsely
        // stalled.
        assert!(c.stale_after.as_secs() > 600, "stale_after must exceed DRIVE_TIMEOUT (600s)");
    }

    // ── fresh → Healthy (regardless of status) ──────────────────────────

    #[test]
    fn fresh_heartbeat_is_healthy() {
        // Well within the 20-min window.
        let seen = seen_ago(60);
        for status in RelayRunStatus::ALL {
            assert_eq!(
                assess_run(status, seen, fixed_now(), 0, &cfg()),
                WatchdogAction::Healthy,
                "{status:?} with a fresh heartbeat is healthy"
            );
        }
    }

    // ── running + stale → Stall ─────────────────────────────────────────

    #[test]
    fn running_and_stale_stalls() {
        // 21 min > 20 min window.
        let seen = seen_ago(21 * 60);
        assert_eq!(
            assess_run(RelayRunStatus::Running, seen, fixed_now(), 0, &cfg()),
            WatchdogAction::Stall
        );
    }

    // ── stalled + stale → Recover (bounded) → Crash ─────────────────────

    #[test]
    fn stalled_and_stale_recovers_while_under_the_cap() {
        let seen = seen_ago(21 * 60);
        // attempts 0/1/2 (max 3) → Recover with the incremented attempt 1/2/3.
        assert_eq!(
            assess_run(RelayRunStatus::Stalled, seen, fixed_now(), 0, &cfg()),
            WatchdogAction::Recover { attempt: 1 }
        );
        assert_eq!(
            assess_run(RelayRunStatus::Stalled, seen, fixed_now(), 1, &cfg()),
            WatchdogAction::Recover { attempt: 2 }
        );
        assert_eq!(
            assess_run(RelayRunStatus::Stalled, seen, fixed_now(), 2, &cfg()),
            WatchdogAction::Recover { attempt: 3 }
        );
    }

    #[test]
    fn stalled_and_stale_crashes_at_and_over_the_cap() {
        let seen = seen_ago(21 * 60);
        // attempts == max (3) → Crash.
        assert_eq!(
            assess_run(RelayRunStatus::Stalled, seen, fixed_now(), 3, &cfg()),
            WatchdogAction::Crash
        );
        // attempts over the cap (4) → still Crash (never a negative-attempt
        // Recover).
        assert_eq!(
            assess_run(RelayRunStatus::Stalled, seen, fixed_now(), 4, &cfg()),
            WatchdogAction::Crash
        );
    }

    // ── non-acted statuses + stale → Healthy ────────────────────────────

    #[test]
    fn paused_blocked_terminal_are_healthy_even_when_stale() {
        let seen = seen_ago(24 * 60 * 60); // a full day stale
        for status in [
            RelayRunStatus::Paused,   // intentional pause — not a stall
            RelayRunStatus::Blocked,  // waiting on a human — not a stall
            RelayRunStatus::Done,     // terminal — nothing to do
            RelayRunStatus::Failed,   // terminal — nothing to do
            RelayRunStatus::Crashed,  // terminal — already given up
        ] {
            assert_eq!(
                assess_run(status, seen, fixed_now(), 0, &cfg()),
                WatchdogAction::Healthy,
                "{status:?} is never escalated by the watchdog"
            );
        }
    }

    // ── boundary + clock-skew edges ─────────────────────────────────────

    #[test]
    fn exactly_at_the_boundary_is_healthy() {
        // now - last_seen == stale_after exactly → fresh (the check is `<=`).
        let seen = seen_ago(20 * 60); // exactly 1200s
        assert_eq!(
            assess_run(RelayRunStatus::Running, seen, fixed_now(), 0, &cfg()),
            WatchdogAction::Healthy,
            "exact-boundary age is still fresh"
        );
    }

    #[test]
    fn one_second_past_the_boundary_is_stale() {
        let seen = seen_ago(20 * 60 + 1); // 1201s
        assert_eq!(
            assess_run(RelayRunStatus::Running, seen, fixed_now(), 0, &cfg()),
            WatchdogAction::Stall,
            "one second past the window is stale"
        );
    }

    #[test]
    fn future_last_seen_is_healthy() {
        // last_seen in the future (negative age — clock skew) clamps to fresh.
        let seen = fixed_now() + Duration::hours(1);
        assert_eq!(
            assess_run(RelayRunStatus::Running, seen, fixed_now(), 0, &cfg()),
            WatchdogAction::Healthy,
            "a future heartbeat is never stale"
        );
        // Even a stalled run with a future heartbeat is healthy (never recovers
        // off clock skew).
        assert_eq!(
            assess_run(RelayRunStatus::Stalled, seen, fixed_now(), 0, &cfg()),
            WatchdogAction::Healthy
        );
    }
}
