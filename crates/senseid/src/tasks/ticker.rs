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
use crate::tasks::schedule::poll_secs;

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

/// The wake-up clock of one scheduled worker, which FOLLOWS its stored cadence.
///
/// The poll is derived from `interval_secs` ([`poll_secs`]), and deriving it once
/// at boot is not enough: a worker seeded hourly polls every 60s, so a user
/// shortening it to 15s would keep waking once a minute — running on a cadence
/// neither the endpoint nor the listing admits to — until the daemon restarted.
/// Following the row is what makes the re-read each poll worth anything.
///
/// Rebuilt only on CHANGE. A fresh ticker fires immediately, so rebuilding every
/// poll would turn the loop into a spin that re-reads the schedule as fast as the
/// database can answer.
struct Poll {
    secs: u64,
    ticker: tokio::time::Interval,
}

impl Poll {
    fn new(secs: u64) -> Self {
        Self { secs, ticker: ticker(secs) }
    }

    async fn tick(&mut self) {
        self.ticker.tick().await;
    }

    /// Adopt the cadence of the row just read. `true` when the clock was rebuilt,
    /// which also means the next tick fires at once.
    fn follow(&mut self, interval_secs: u32) -> bool {
        let secs = poll_secs(interval_secs);
        if secs == self.secs {
            return false;
        }
        *self = Self::new(secs);
        true
    }
}

/// Run `tick` on the schedule stored for `name`, forever.
///
/// The whole loop for a scheduled worker: wake on the poll cadence, re-read the
/// schedule, ask [`schedule::should_run`], and either run the tick and record
/// the outcome or skip with a reason. Each worker keeps its own `tick` — what to
/// do is code; when to do it is data.
///
/// The schedule is re-read EVERY poll, not cached at startup, and the poll
/// itself follows the cadence it reads ([`Poll::follow`]), so a user changing a
/// cadence or disabling a worker takes effect without restarting the daemon —
/// within one poll of the OLD cadence, at most a minute. That is the whole point
/// of making it editable.
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
    use crate::tasks::schedule::{Skip, should_run};

    // Poll cadence comes from the schedule, but a missing row must not stop the
    // loop from ever looking again — the row may be seeded by a later deploy.
    if defer_first && let Err(e) = pg.defer_schedule_start(name).await {
        tracing::warn!(worker = name, error = %e, "scheduled: could not defer the first run");
    }
    let initial = pg.load_schedule(name).await.ok().flatten();
    let mut poll = Poll::new(initial.as_ref().map_or(60, |s| poll_secs(s.interval_secs)));

    loop {
        poll.tick().await;

        let Some(stored) = pg.load_schedule(name).await.ok().flatten() else {
            tracing::debug!(worker = name, reason = ?Skip::Unscheduled, "scheduled: skipped");
            continue;
        };
        // Before the due-check, so a shortened cadence starts applying on the
        // very poll that notices it — and a worker that had no row at boot
        // stops polling on the fallback minute once one appears.
        if poll.follow(stored.interval_secs) {
            tracing::debug!(
                worker = name,
                poll_secs = poll.secs,
                "scheduled: poll cadence changed"
            );
        }
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

    #[tokio::test(start_paused = true)]
    async fn the_poll_clock_is_rebuilt_only_when_the_cadence_changes() {
        // Hourly and daily clamp to the same 60s poll, so a clock rebuilt on
        // every read would fire immediately every time — a spin that re-reads
        // the schedule as fast as the database can answer it.
        let mut p = Poll::new(poll_secs(3600));
        assert!(ready_now!(p), "boot tick");
        assert!(!p.follow(86_400), "a cadence with the same poll must not rebuild the clock");
        assert!(!ready_now!(p), "so it keeps counting down instead of firing again");

        // The edit that used to need a restart: hourly → 15s.
        assert!(p.follow(15), "a shorter cadence rebuilds it");
        assert!(ready_now!(p), "a rebuilt clock fires at once");
        tokio::time::advance(Duration::from_secs(15)).await;
        assert!(ready_now!(p), "and then on the NEW cadence, not the boot-time minute");
    }

    /// A cadence the user SHORTENS must speed the loop up, with no restart.
    ///
    /// The endpoint answers 200 and `GET /api/tasks/scheduled` then reports the
    /// new interval, so a loop still waking on its boot-time poll would be
    /// running on a cadence nothing in the system admits to.
    ///
    /// Real seconds, not the paused clock: `should_run` reads the wall clock
    /// through `chrono`, which tokio's time control does not move, so a paused
    /// runtime would leave the poll and the due-check disagreeing. Seconds are
    /// the finest cadence the table stores, hence a test that takes a few.
    #[tokio::test]
    async fn a_shortened_cadence_takes_effect_without_a_restart() {
        use crate::db::pg_store::SchedulePatch;

        let pg = std::sync::Arc::new(PgStore::connect_test().await.unwrap());
        // `run_scheduled` names its worker with a `&'static str` because every
        // real one is a literal. A throwaway row needs a unique name, so this
        // one is leaked on purpose — a few bytes, once, in a test binary.
        let name: &'static str =
            Box::leak(crate::tasks::test_support::test_schedule_name().into_boxed_str());
        sqlx_core::query::query("INSERT INTO sensei.schedules(name, interval_secs) VALUES($1, 3)")
            .bind(name)
            .execute(pg.pool())
            .await
            .unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let worker = tokio::spawn(run_scheduled(pg.clone(), name, move || {
            let tx = tx.clone();
            async move {
                let _ = tx.send(());
                Ok(())
            }
        }));
        tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("the boot pass runs at once")
            .expect("the loop is alive");

        pg.update_schedule(name, &SchedulePatch { interval_secs: Some(1), ..Default::default() })
            .await
            .unwrap()
            .expect("the row exists");

        // Three more passes fit in seven seconds only if the loop adopted the
        // 1s cadence. Pinned to the boot poll they land at t=3/6/9 and the third
        // never arrives.
        let three_more = async {
            for _ in 0..3 {
                rx.recv().await.expect("the loop is alive");
            }
        };
        let sped_up = tokio::time::timeout(Duration::from_secs(7), three_more).await.is_ok();

        worker.abort();
        sqlx_core::query::query("DELETE FROM sensei.schedules WHERE name = $1")
            .bind(name)
            .execute(pg.pool())
            .await
            .ok();
        assert!(sped_up, "a cadence patched to 1s must not keep waking on the boot-time 3s poll");
    }
}
