//! Relay-engine (P3.6) — the run watchdog scheduler.
//!
//! A long-lived tokio task (mirroring [`crate::tasks::advance_run_scheduler`])
//! that wakes on a slow interval and keeps autonomous runs alive under the
//! **degrade-never-stop** rule (relay-engine §5 "Watchdog / crashed"):
//!
//! 1. [`PgStore::list_recoverable_runs`] returns the `running`/`stalled` runs
//!    with just the liveness fields (status, heartbeat, started_at, recovery
//!    count).
//! 2. For each, [`crate::run_watchdog::assess_run`] (pure) decides — from the
//!    heartbeat age + status + attempt count — what to do:
//!    - **Healthy** — fresh heartbeat → no-op.
//!    - **Stall** — a `running` run went stale → mark `stalled` + a `Stalled`
//!      cadence event (noticed; handed to recovery next tick).
//!    - **Recover** — a `stalled` run still stale under the attempt cap → back
//!      to `running` (heartbeat refreshed), a `Recovered` event, and an
//!      `AdvanceRun` tick so it resumes immediately.
//!    - **Crash** — a `stalled` run past the attempt cap → `crashed` + a
//!      `Crashed` event (bounded recovery exhausted; give up + surface).
//!
//! **Heartbeat ownership.** The stall signal is `heartbeat_at` age. The
//! `advance_run` handler heartbeats `running`/`blocked` runs every tick but
//! deliberately leaves `stalled` runs untouched, so this watchdog exclusively
//! owns a stalled run's lifecycle — otherwise a fresh heartbeat would mask the
//! staleness and recovery could never fire.
//!
//! **Fault tolerance:** every DB error is logged (`tracing::warn`) and the loop
//! continues — a missing `activity.runs` table, an unparseable status/timestamp,
//! or a per-run write failure never panics the scheduler or aborts the tick.
//!
//! **Zero-knowledge (D10):** run_events carry only logical status (a short note
//! + attempt count) — never stdout/stderr, diffs, or code.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use dojo_protocol::relay::RelayRunStatus;

use crate::db::pg_store::PgStore;
use crate::run_watchdog::{WatchdogAction, WatchdogConfig, assess_run};
use crate::runs::RunEventKind;
use crate::tasks::advance_run_scheduler::enqueue_advance;
use crate::tasks::queue::TaskQueue;

/// How often to sweep for stalled/crashed runs. Slow relative to the 15s
/// `AdvanceRun` cadence — staleness is a 20-minute signal, so a 60s watchdog
/// tick is a tight-enough net without churn.
const INTERVAL_SECS: u64 = 60;

/// Parse an RFC-3339 timestamp (as produced by `to_json(col)#>>'{}'`) into a
/// UTC instant. `None` on a malformed value so the caller can warn+skip rather
/// than panic.
fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc))
}

/// Apply one assessed [`WatchdogAction`] to a run: the DB-writing side of the
/// pure decision. A DB error is returned (the caller logs + continues) — never a
/// panic. Kept separate from [`tick`] so the action→effect mapping is testable
/// and the tick loop stays a thin sweep.
async fn apply_action(
    queue: &TaskQueue,
    pg: &PgStore,
    run_id: &uuid::Uuid,
    action: WatchdogAction,
    recovery_attempts: i32,
) -> Result<(), String> {
    match action {
        // Fresh (or a status the watchdog doesn't act on) — nothing to do.
        WatchdogAction::Healthy => Ok(()),

        // A running run went stale → mark it stalled so the next tick's Recover
        // ladder picks it up. Event first, then the status flip.
        WatchdogAction::Stall => {
            pg.append_run_event(
                run_id,
                RunEventKind::Stalled,
                None,
                None,
                &serde_json::json!({ "note": "no progress in a while — nudge to continue" }),
            )
            .await
            .map_err(|e| format!("append_run_event(Stalled) failed: {e}"))?;
            pg.update_run_status(run_id, RelayRunStatus::Stalled, None, None)
                .await
                .map_err(|e| format!("update_run_status(Stalled) failed: {e}"))
        }

        // Bounded auto-recovery: flip back to running (heartbeat refreshed by
        // recover_run), record the attempt, and kick an immediate AdvanceRun tick
        // so the recovered run resumes without waiting a full scheduler cycle.
        WatchdogAction::Recover { attempt } => {
            pg.recover_run(run_id, attempt)
                .await
                .map_err(|e| format!("recover_run failed: {e}"))?;
            pg.append_run_event(
                run_id,
                RunEventKind::Recovered,
                None,
                None,
                &serde_json::json!({ "attempt": attempt, "auto": true }),
            )
            .await
            .map_err(|e| format!("append_run_event(Recovered) failed: {e}"))?;
            enqueue_advance(queue, run_id).await;
            Ok(())
        }

        // Bounded recovery exhausted → crashed (terminal, surfaced). A push
        // notification is P4 (FOLLOW-UP: notify the phone on crash).
        WatchdogAction::Crash => {
            pg.append_run_event(
                run_id,
                RunEventKind::Crashed,
                None,
                None,
                &serde_json::json!({ "note": "exhausted bounded recovery", "attempts": recovery_attempts }),
            )
            .await
            .map_err(|e| format!("append_run_event(Crashed) failed: {e}"))?;
            pg.update_run_status(run_id, RelayRunStatus::Crashed, None, None)
                .await
                .map_err(|e| format!("update_run_status(Crashed) failed: {e}"))
        }
    }
}

/// One watchdog sweep. `now` + `cfg` are injected so a single tick is testable
/// with deterministic inputs. A list-query failure warns and returns (tolerates
/// a not-yet-deployed runs table); a per-run parse/write failure warns and skips
/// that run — the sweep never panics and never aborts on one bad row.
async fn tick(queue: &TaskQueue, pg: &PgStore, now: DateTime<Utc>, cfg: &WatchdogConfig) {
    let rows = match pg.list_recoverable_runs().await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "watchdog_scheduler: list_recoverable_runs failed");
            return;
        }
    };

    for (id, status_str, heartbeat, started_at, attempts) in rows {
        let Some(status) = RelayRunStatus::from_db_str(&status_str) else {
            tracing::warn!(run_id = %id, status = %status_str, "watchdog_scheduler: unknown run_status; skipping");
            continue;
        };
        // Daemon-liveness reference: the heartbeat, or started_at for a run that
        // has never heartbeated. Both unparseable → skip (never panic on bad data).
        let last_heartbeat =
            heartbeat.as_deref().and_then(parse_rfc3339).or_else(|| parse_rfc3339(&started_at));
        let Some(last_heartbeat) = last_heartbeat else {
            tracing::warn!(run_id = %id, "watchdog_scheduler: unparseable heartbeat/started_at; skipping");
            continue;
        };
        // Agent-progress reference: the newest non-housekeeping run event, else
        // `started_at` for a run that has made no progress yet (so a run that
        // never advanced stalls 5 min after it started). This is the "no update
        // in 5 min" signal a running run stalls on. A transient query error falls
        // back to the (fresh) heartbeat so we never false-stall on a DB hiccup.
        let started = parse_rfc3339(&started_at).unwrap_or(last_heartbeat);
        let last_progress = match pg.last_progress_at(&id).await {
            Ok(Some(ts)) => parse_rfc3339(&ts).unwrap_or(started),
            Ok(None) => started,
            Err(e) => {
                tracing::warn!(run_id = %id, error = %e, "watchdog_scheduler: last_progress_at failed; using heartbeat");
                last_heartbeat
            }
        };

        let action = assess_run(status, last_progress, last_heartbeat, now, attempts, cfg);
        if let Err(e) = apply_action(queue, pg, &id, action, attempts).await {
            // A single run's write failing must not stop the sweep over the rest.
            tracing::warn!(run_id = %id, error = %e, "watchdog_scheduler: applying action failed; skipping");
        }
    }
}

/// Spawn the watchdog for the daemon's lifetime.
pub fn spawn(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    tokio::spawn(run(queue, pg));
}

async fn run(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    let cfg = WatchdogConfig::default();
    let mut ticker = tokio::time::interval(Duration::from_secs(INTERVAL_SECS));
    loop {
        ticker.tick().await;
        tick(&queue, &pg, Utc::now(), &cfg).await;
    }
}

#[cfg(test)]
// `resume_test_guard()` is a blocking `std::sync::Mutex` held across awaits on
// purpose — see `crate::tasks::test_support::TestGate` for why an async mutex loses
// wakeups here. These are current-thread test runtimes, one per test, so
// blocking the thread costs nothing and cannot deadlock the runtime.
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::*;
    use crate::runs::NewRun;

    #[test]
    fn parse_rfc3339_roundtrips_and_rejects_garbage() {
        assert!(parse_rfc3339("2026-07-17T08:00:00Z").is_some());
        assert!(parse_rfc3339("2026-07-17T08:00:00+00:00").is_some());
        assert!(parse_rfc3339("not-a-timestamp").is_none());
        assert!(parse_rfc3339("").is_none());
    }

    /// Force a run's liveness fields directly, bypassing the normal writers, so a
    /// test can stage an "old heartbeat" / attempt count the watchdog reacts to.
    async fn force_liveness(
        pg: &PgStore,
        id: &uuid::Uuid,
        status: RelayRunStatus,
        heartbeat_age_secs: i64,
        attempts: i32,
    ) {
        // Backdate started_at alongside the heartbeat: with no progress events, the
        // stall signal's progress reference falls back to started_at, so a
        // no-progress run this old reads as agent-stale (matches "stale liveness").
        sqlx_core::query::query(
            "UPDATE activity.runs
                SET status            = $2::sensei.run_status,
                    heartbeat_at      = now() - make_interval(secs => $3),
                    started_at        = now() - make_interval(secs => $3),
                    recovery_attempts = $4
              WHERE id = $1",
        )
        .bind(id)
        .bind(status.as_db_str())
        .bind(heartbeat_age_secs as f64)
        .bind(attempts)
        .execute(pg.pool())
        .await
        .unwrap();
    }

    async fn delete_run(pg: &PgStore, id: &uuid::Uuid) {
        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(id)
            .execute(pg.pool())
            .await
            .unwrap();
    }

    /// Read `recovery_attempts` directly — it's deliberately not on the `Run`
    /// struct (the watchdog uses a lightweight query), so a test reads it raw.
    async fn recovery_attempts(pg: &PgStore, id: &uuid::Uuid) -> i32 {
        let (n,): (i32,) = sqlx_core::query_as::query_as(
            "SELECT recovery_attempts FROM activity.runs WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        n
    }

    // The DB-guarded tests share the global `activity.runs` table, and a tick is
    // a table-wide sweep. Serialize on the run-scheduler test lock so a sibling
    // scheduler test can't flip our staged run out from under an assertion.
    // (Production runs a single watchdog, so there is no such race there.)

    #[tokio::test]
    async fn tick_marks_a_stale_running_run_stalled() {
        let _guard = crate::runs::resume_test_guard();
        let Ok(pg) = PgStore::connect_test().await else {
            return;
        };
        let queue = TaskQueue::with_max_repos(16);

        let id = pg.create_run(&NewRun::default()).await.unwrap(); // running
        // 1 hour stale > 20-min default window.
        force_liveness(&pg, &id, RelayRunStatus::Running, 3600, 0).await;

        tick(&queue, &pg, Utc::now(), &WatchdogConfig::default()).await;

        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(run.status, RelayRunStatus::Stalled, "stale running run → stalled");
        let kinds: Vec<RunEventKind> =
            pg.list_run_events(&id, 10).await.unwrap().into_iter().map(|e| e.kind).collect();
        assert!(
            kinds.contains(&RunEventKind::Stalled),
            "a Stalled event was logged, got {kinds:?}"
        );

        delete_run(&pg, &id).await;
    }

    #[tokio::test]
    async fn tick_keeps_a_running_run_with_recent_progress_untouched() {
        let _guard = crate::runs::resume_test_guard();
        let Ok(pg) = PgStore::connect_test().await else {
            return;
        };
        let queue = TaskQueue::with_max_repos(16);

        let id = pg.create_run(&NewRun::default()).await.unwrap();
        // Old start (would stall via the started_at fallback) BUT a fresh progress
        // event → must stay running. Guards the last_progress_at RFC-3339 format:
        // a `::text` timestamp fails to parse and silently falls back to
        // started_at, which would false-stall a live, progressing run.
        force_liveness(&pg, &id, RelayRunStatus::Running, 3600, 0).await; // started 1h ago
        pg.append_run_event(
            &id,
            RunEventKind::PhaseStarted,
            Some("P1"),
            None,
            &serde_json::json!({}),
        )
        .await
        .unwrap(); // fresh progress, just now

        tick(&queue, &pg, Utc::now(), &WatchdogConfig::default()).await;

        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(
            run.status,
            RelayRunStatus::Running,
            "fresh progress keeps a run running despite an old start"
        );

        delete_run(&pg, &id).await;
    }

    #[tokio::test]
    async fn tick_recovers_a_stalled_run_under_the_cap() {
        let _guard = crate::runs::resume_test_guard();
        let Ok(pg) = PgStore::connect_test().await else {
            return;
        };
        let queue = TaskQueue::with_max_repos(16);

        let id = pg.create_run(&NewRun::default()).await.unwrap();
        // Stalled, 1h stale, attempts = 0 (under the max of 3) → Recover{1}.
        force_liveness(&pg, &id, RelayRunStatus::Stalled, 3600, 0).await;

        tick(&queue, &pg, Utc::now(), &WatchdogConfig::default()).await;

        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(run.status, RelayRunStatus::Running, "recovered run is running again");
        assert_eq!(recovery_attempts(&pg, &id).await, 1, "attempt counter incremented");
        // Heartbeat refreshed so it isn't instantly re-stalled next tick.
        assert!(run.heartbeat_at.is_some(), "recover refreshes the heartbeat");

        let kinds: Vec<RunEventKind> =
            pg.list_run_events(&id, 10).await.unwrap().into_iter().map(|e| e.kind).collect();
        assert!(
            kinds.contains(&RunEventKind::Recovered),
            "a Recovered event was logged, got {kinds:?}"
        );

        // An AdvanceRun tick was enqueued for the recovered run.
        let n = queue.status().await.pending;
        let mut saw = false;
        for _ in 0..n {
            let t = queue.next_task().await;
            if t.kind == crate::tasks::TaskKind::AdvanceRun && t.path == id.to_string() {
                saw = true;
            }
        }
        assert!(saw, "recover enqueues an AdvanceRun tick for the run");

        delete_run(&pg, &id).await;
    }

    #[tokio::test]
    async fn tick_crashes_a_stalled_run_at_the_cap() {
        let _guard = crate::runs::resume_test_guard();
        let Ok(pg) = PgStore::connect_test().await else {
            return;
        };
        let queue = TaskQueue::with_max_repos(16);

        let id = pg.create_run(&NewRun::default()).await.unwrap();
        // Stalled, 1h stale, attempts = 3 (== max) → Crash.
        force_liveness(&pg, &id, RelayRunStatus::Stalled, 3600, 3).await;

        tick(&queue, &pg, Utc::now(), &WatchdogConfig::default()).await;

        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(run.status, RelayRunStatus::Crashed, "exhausted recovery → crashed");
        let kinds: Vec<RunEventKind> =
            pg.list_run_events(&id, 10).await.unwrap().into_iter().map(|e| e.kind).collect();
        assert!(
            kinds.contains(&RunEventKind::Crashed),
            "a Crashed event was logged, got {kinds:?}"
        );

        delete_run(&pg, &id).await;
    }

    #[tokio::test]
    async fn tick_leaves_a_fresh_running_run_untouched() {
        let _guard = crate::runs::resume_test_guard();
        let Ok(pg) = PgStore::connect_test().await else {
            return;
        };
        let queue = TaskQueue::with_max_repos(16);

        let id = pg.create_run(&NewRun::default()).await.unwrap();
        // Fresh heartbeat (5s old) → Healthy, no change.
        force_liveness(&pg, &id, RelayRunStatus::Running, 5, 0).await;

        tick(&queue, &pg, Utc::now(), &WatchdogConfig::default()).await;

        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(run.status, RelayRunStatus::Running, "a fresh run is left running");
        assert!(
            pg.list_run_events(&id, 10).await.unwrap().is_empty(),
            "a healthy run produces no watchdog event"
        );

        delete_run(&pg, &id).await;
    }
}
