//! Periodic-task timing, in one place.
//!
//! Twelve long-lived tasks in this daemon wake on an interval, and every one of
//! them used to build its own ticker, with eight carrying a byte-identical
//! `parse_interval`. That duplication is gone: the cadence now lives in
//! `sensei.schedules`, and `run_scheduled` is the one loop they all share.
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

/// Build a ticker for an already-resolved interval.
/// Every ticker fires immediately and then every `secs`.
///
/// There used to be a `FirstTick` choice, for the activity pruner's deliberate
/// refusal to run at boot. Deferring a worker's FIRST RUN turned out to be a
/// SCHEDULE concern rather than a poll one — the poll is capped at 60s, so
/// skipping one poll would delay by a minute, not by the interval the pruner
/// needs. `PgStore::defer_schedule_start` does it properly, and this parameter
/// had no callers left.
pub fn ticker(secs: u64) -> tokio::time::Interval {
    let mut t = tokio::time::interval(Duration::from_secs(secs));
    // A missed tick must not cause a burst of catch-up ticks. Every caller here
    // is idempotent-per-tick (a pruner, a re-scan, a metrics wave), so catching
    // up buys nothing and a stalled machine would wake to a thundering herd.
    t.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    t
}

/// Run `tick` on the schedule stored for `name`, forever.
///
/// The whole loop for a scheduled worker: wake on the poll cadence, re-read the
/// schedule, ask [`schedule::should_run`], and either run the tick and record
/// the outcome or skip with a reason. Each worker keeps its own `tick` — what to
/// do is code; when to do it is data.
///
/// The schedule is re-read EVERY poll, not cached at startup, so a user changing
/// a cadence or disabling a worker takes effect without restarting the daemon.
/// That is the whole point of making it editable.
///
/// A skip is logged at debug with its reason rather than silently: "why has this
/// not run?" has four different answers and only some are settings a user can
/// act on.
pub async fn run_scheduled<F, Fut>(pg: std::sync::Arc<PgStore>, name: &'static str, tick: F)
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    run_scheduled_inner(pg, name, false, tick).await
}

/// As [`run_scheduled`], but a never-run worker waits a full interval before its
/// first pass instead of firing at boot.
///
/// For workers whose whole point is not to run at startup — the activity pruner
/// must not reclaim sessions while capture is still re-materialising them.
pub async fn run_scheduled_deferred<F, Fut>(
    pg: std::sync::Arc<PgStore>,
    name: &'static str,
    tick: F,
) where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    run_scheduled_inner(pg, name, true, tick).await
}

async fn run_scheduled_inner<F, Fut>(
    pg: std::sync::Arc<PgStore>,
    name: &'static str,
    defer_first: bool,
    tick: F,
) where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    use crate::tasks::schedule::{Skip, poll_secs, should_run};

    // Poll cadence comes from the schedule, but a missing row must not stop the
    // loop from ever looking again — the row may be seeded by a later deploy.
    if defer_first && let Err(e) = pg.defer_schedule_start(name).await {
        tracing::warn!(worker = name, error = %e, "scheduled: could not defer the first run");
    }
    let initial = pg.load_schedule(name).await.ok().flatten();
    let poll = initial.as_ref().map_or(60, |s| poll_secs(s.interval_secs));
    let mut t = ticker(poll);

    loop {
        t.tick().await;

        let Some(stored) = pg.load_schedule(name).await.ok().flatten() else {
            tracing::debug!(worker = name, reason = ?Skip::Unscheduled, "scheduled: skipped");
            continue;
        };
        let now_utc = chrono::Utc::now();
        let now_local = chrono::Local::now().naive_local();

        if let Err(skip) = should_run(&stored.rules(), now_local, now_utc, stored.last_run_at) {
            tracing::debug!(worker = name, reason = ?skip, "scheduled: skipped");
            continue;
        }

        let outcome = tick().await;
        if let Err(e) = &outcome {
            tracing::warn!(worker = name, error = %e, "scheduled: tick failed");
        }
        if let Err(e) = pg.mark_schedule_run(name, outcome).await {
            // Non-fatal: the work happened; only the bookkeeping failed. Dying
            // here would stop a healthy worker over a write to a status column.
            tracing::warn!(worker = name, error = %e, "scheduled: could not record the run");
        }
    }
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
    #[tokio::test(start_paused = true)]
    async fn a_ticker_fires_at_boot() {
        // Every caller wants this now; deferring a first RUN is a schedule
        // concern (PgStore::defer_schedule_start), not a poll one.
        let mut t = ticker(60);
        assert!(ready_now!(t), "the first tick must fire immediately");
    }

    #[tokio::test(start_paused = true)]
    async fn a_missed_tick_does_not_burst() {
        // A laptop asleep for an hour must not wake to sixty queued ticks.
        let mut t = ticker(60);
        assert!(ready_now!(t), "boot tick");
        tokio::time::advance(Duration::from_secs(600)).await; // ten intervals missed
        assert!(ready_now!(t), "one catch-up tick is expected");
        assert!(!ready_now!(t), "but only one — MissedTickBehavior::Delay suppresses the burst");
    }
}
