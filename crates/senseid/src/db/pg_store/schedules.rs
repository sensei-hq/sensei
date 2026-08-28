//! Reading and recording the schedules background workers run on.
//!
//! The row is the configuration (`sensei.schedules`); the decision is pure and
//! lives in [`crate::tasks::schedule`]. This module is only the I/O between
//! them, so the interesting logic stays testable without a database.

use chrono::{DateTime, NaiveTime, Utc};

use super::PgStore;

/// The `sensei.schedules` columns this module SELECTs, in order. Named so the
/// shape is stated once beside the query rather than inferred from a nine-wide
/// tuple at the call site.
type ScheduleRow = (
    String,                // name
    bool,                  // enabled
    i32,                   // interval_secs
    Option<NaiveTime>,     // window_start
    Option<NaiveTime>,     // window_end
    Option<Vec<i16>>,      // days
    Option<DateTime<Utc>>, // last_run_at
    Option<bool>,          // last_ok
    Option<String>,        // last_error
);
use crate::tasks::schedule::Schedule;

/// A schedule as stored, including the runtime state the daemon writes back.
#[derive(Debug, Clone)]
pub struct StoredSchedule {
    pub name: String,
    pub enabled: bool,
    pub interval_secs: u32,
    pub window_start: Option<NaiveTime>,
    pub window_end: Option<NaiveTime>,
    pub days: Vec<u8>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_ok: Option<bool>,
    pub last_error: Option<String>,
}

impl StoredSchedule {
    /// The pure decision input, without the runtime state.
    pub fn rules(&self) -> Schedule {
        Schedule {
            name: self.name.clone(),
            enabled: self.enabled,
            interval_secs: self.interval_secs,
            window_start: self.window_start,
            window_end: self.window_end,
            days: self.days.clone(),
        }
    }
}

impl PgStore {
    /// The schedule a worker runs on, or `None` when it has no row.
    ///
    /// `None` rather than a default: a worker with no schedule must be visibly
    /// unscheduled, not silently running on an invented cadence. The
    /// code↔table agreement test makes that state a build failure anyway, so
    /// reaching it at runtime means something is genuinely wrong.
    pub async fn load_schedule(&self, name: &str) -> Result<Option<StoredSchedule>, String> {
        let row: Option<ScheduleRow> = sqlx_core::query_as::query_as(
            "SELECT name, enabled, interval_secs, window_start, window_end, days, \
                    last_run_at, last_ok, last_error \
               FROM sensei.schedules WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("load_schedule({name}): {e}"))?;

        Ok(row.map(|(name, enabled, interval, ws, we, days, last_run, last_ok, last_err)| {
            StoredSchedule {
                name,
                enabled,
                // The CHECK guarantees > 0; clamp rather than panic if it ever
                // does not, since a scheduler must not take the daemon down.
                interval_secs: u32::try_from(interval).unwrap_or(1).max(1),
                window_start: ws,
                window_end: we,
                days: days
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|d| u8::try_from(d).ok())
                    .collect(),
                last_run_at: last_run,
                last_ok,
                last_error: last_err,
            }
        }))
    }

    /// Start a never-run worker's clock at now, so its first pass waits a full
    /// interval instead of firing at boot.
    ///
    /// `last_run_at IS NULL` means "never run", which is always due — correct for
    /// almost every worker, and wrong for the ones whose whole point is NOT to
    /// run at startup. The activity pruner must not reclaim sessions while
    /// capture is still re-materialising them; its old loop expressed that by
    /// skipping the boot tick, and a poll-level skip cannot reproduce it because
    /// the poll is capped at 60s while the delay must be a full interval.
    ///
    /// This says "the clock starts now", not "it ran" — `last_ok` and
    /// `last_error` stay NULL, so nothing claims a pass happened. Idempotent:
    /// only ever touches a row that has genuinely never run.
    pub async fn defer_schedule_start(&self, name: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.schedules SET last_run_at = now() \
              WHERE name = $1 AND last_run_at IS NULL",
        )
        .bind(name)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("defer_schedule_start({name}): {e}"))?;
        Ok(())
    }

    /// Record that a scheduled pass ran, and how it went.
    ///
    /// A success CLEARS `last_error`: a stale error sitting beside a healthy run
    /// reads as "still broken" to anyone scanning the table, which is the same
    /// mistake `mark_synced` avoids. `last_run_at` advances on BOTH outcomes —
    /// an attempt is a run, and not advancing it on failure would retry a
    /// permanently-failing worker on every single poll.
    pub async fn mark_schedule_run(
        &self,
        name: &str,
        outcome: Result<(), String>,
    ) -> Result<(), String> {
        let (ok, err) = match &outcome {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.as_str())),
        };
        sqlx_core::query::query(
            "UPDATE sensei.schedules \
                SET last_run_at = now(), last_ok = $2, last_error = $3 \
              WHERE name = $1",
        )
        .bind(name)
        .bind(ok)
        .bind(err)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("mark_schedule_run({name}): {e}"))?;
        Ok(())
    }
}
