//! Periodic-task timing, in one place.
//!
//! Twelve long-lived tasks in this daemon wake on an interval, and every one of
//! them used to build its own: read a config key, parse it, fall back on a bad
//! value, construct a `tokio::time::interval`, and decide whether the first tick
//! fires at boot. Eight carried a byte-identical `parse_interval` — same
//! `trim().parse().filter(>0).unwrap_or(default)` — each with its own
//! near-duplicate test.
//!
//! That is the *timing* concern, and it is genuinely the same everywhere. What
//! is NOT the same is what each task decides to do when it wakes: the analyzer
//! carries a watermark across ticks, the library updater dedupes across
//! libraries, reconcile overlap-guards on a specific `TaskKind`. Those are real
//! code, not configuration, so this module deliberately owns only the clock and
//! leaves the decision to the caller.
//!
//! The name is the point too: a scheduler is not a per-task thing. Nothing here
//! is named after a task, and modules that use it are named for what they
//! decide — `dojo_sync`, not `dojo_sync_scheduler`.

use std::time::Duration;

use crate::db::pg_store::PgStore;

/// Whether the first tick fires at boot or after one full interval.
///
/// `tokio::time::interval` fires immediately by default, and most callers WANT
/// that — a boot reconcile, a drain on startup, a metrics wave from a freshly
/// booted daemon. But not all: the activity pruner must not reclaim sessions the
/// instant the daemon starts, because capture is still re-materialising them and
/// pruning first would win a race it should lose.
///
/// Making that a named choice rather than a hand-written `ticker.tick().await`
/// before the loop means the decision is visible at the call site instead of
/// hidden in a comment four lines up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstTick {
    /// Fire at boot, then every interval. The common case.
    Immediate,
    /// Wait one full interval before the first run.
    AfterOneInterval,
}

/// Resolve a tick interval (seconds) from a raw config value.
///
/// Missing, unparseable, and zero all fall back to `default_secs`. Zero matters:
/// a `tokio::time::interval` of zero busy-loops, so a typo'd config value would
/// spin a core rather than fail loudly. Pure, so it is testable without a clock
/// or a database.
pub fn interval_secs(cfg: Option<String>, default_secs: u64) -> u64 {
    cfg.and_then(|v| v.trim().parse::<u64>().ok()).filter(|n| *n > 0).unwrap_or(default_secs)
}

/// Build a ticker for an already-resolved interval.
pub fn ticker(secs: u64, first: FirstTick) -> tokio::time::Interval {
    let period = Duration::from_secs(secs);
    // `interval_at` is what makes FirstTick real: starting one period out is the
    // supported way to skip the boot tick, rather than an un-awaited
    // `ticker.tick()` before the loop that a reader has to spot.
    let start = match first {
        FirstTick::Immediate => tokio::time::Instant::now(),
        FirstTick::AfterOneInterval => tokio::time::Instant::now() + period,
    };
    let mut t = tokio::time::interval_at(start, period);
    // A missed tick must not cause a burst of catch-up ticks. Every caller here
    // is idempotent-per-tick (a pruner, a re-scan, a metrics wave), so catching
    // up buys nothing and a stalled machine would wake to a thundering herd.
    t.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    t
}

/// Read the interval from config and build the ticker — the whole timing setup
/// for a periodic task, in one call.
///
/// A config-read failure is treated as "unset": the task keeps its default rather
/// than refusing to start. A periodic task that dies because the config table was
/// briefly unavailable is worse than one running at its default cadence.
pub async fn from_config(
    pg: &PgStore,
    key: &str,
    default_secs: u64,
    first: FirstTick,
) -> tokio::time::Interval {
    let secs = interval_secs(pg.get_config(key).await.ok().flatten(), default_secs);
    tracing::debug!(key, interval_secs = secs, "ticker: resolved interval");
    ticker(secs, first)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Is this tick ready RIGHT NOW? `biased` polls the tick first and falls
    /// through to an already-ready future when it is pending, so readiness is
    /// observable without waiting real time and without pulling in `futures`.
    macro_rules! ready_now {
        ($t:expr) => {
            tokio::select! {
                biased;
                _ = $t.tick() => true,
                _ = std::future::ready(()) => false,
            }
        };
    }

    #[test]
    fn interval_falls_back_on_missing_unparseable_and_zero() {
        // The three ways a config value fails to name an interval. Zero is the
        // dangerous one — it busy-loops rather than erroring.
        assert_eq!(interval_secs(None, 3600), 3600, "missing");
        assert_eq!(interval_secs(Some("nope".into()), 3600), 3600, "unparseable");
        assert_eq!(interval_secs(Some("0".into()), 3600), 3600, "zero would busy-loop");
        assert_eq!(interval_secs(Some("".into()), 3600), 3600, "empty");
        assert_eq!(interval_secs(Some("-5".into()), 3600), 3600, "negative");
    }

    #[test]
    fn interval_honours_a_valid_value_including_whitespace() {
        assert_eq!(interval_secs(Some("60".into()), 3600), 60);
        assert_eq!(interval_secs(Some("  60\n".into()), 3600), 60, "trimmed");
    }

    #[tokio::test(start_paused = true)]
    async fn immediate_fires_at_boot_and_after_one_interval_does_not() {
        // The distinction the activity pruner depends on: it must NOT prune the
        // instant the daemon starts, or it wins a race against capture that it
        // should lose.
        let mut immediate = ticker(60, FirstTick::Immediate);
        assert!(ready_now!(immediate), "Immediate must fire at boot");

        let mut delayed = ticker(60, FirstTick::AfterOneInterval);
        assert!(!ready_now!(delayed), "AfterOneInterval must NOT fire at boot");
    }

    #[tokio::test(start_paused = true)]
    async fn after_one_interval_fires_once_the_interval_elapses() {
        let mut t = ticker(60, FirstTick::AfterOneInterval);
        tokio::time::advance(Duration::from_secs(61)).await;
        assert!(ready_now!(t), "it must fire after the interval elapses");
    }

    #[tokio::test(start_paused = true)]
    async fn a_missed_tick_does_not_burst() {
        // A laptop asleep for an hour must not wake to sixty queued ticks.
        let mut t = ticker(60, FirstTick::Immediate);
        assert!(ready_now!(t), "boot tick");
        tokio::time::advance(Duration::from_secs(600)).await; // ten intervals missed
        assert!(ready_now!(t), "one catch-up tick is expected");
        assert!(!ready_now!(t), "but only one — MissedTickBehavior::Delay suppresses the burst");
    }
}
