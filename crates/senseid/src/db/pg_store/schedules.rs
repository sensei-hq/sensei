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

/// That same shape as a SELECT list, so the three queries below cannot drift
/// from [`ScheduleRow`] one column at a time.
const SCHEDULE_COLUMNS: &str = "name, enabled, interval_secs, window_start, window_end, days, \
                                last_run_at, last_ok, last_error";
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

impl From<ScheduleRow> for StoredSchedule {
    fn from(row: ScheduleRow) -> Self {
        let (name, enabled, interval, ws, we, days, last_run, last_ok, last_error) = row;
        Self {
            name,
            enabled,
            // The CHECK guarantees > 0; clamp rather than panic if it ever does
            // not, since a scheduler must not take the daemon down.
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
            last_error,
        }
    }
}

/// A partial edit to a schedule: an absent field is left alone, which is what
/// makes this a PATCH rather than a replace.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchedulePatch {
    pub enabled: Option<bool>,
    pub interval_secs: Option<u32>,
    /// The window as a PAIR — `Some(None)` clears it back to "any time". A HALF
    /// window is unrepresentable here on purpose: one bound alone reads as "any
    /// time" to `within_window`, so it would silently not be the window that was
    /// asked for.
    pub window: Option<Option<(NaiveTime, NaiveTime)>>,
    /// ISO weekdays (1 = Mon … 7 = Sun). `Some(None)` clears the mask back to
    /// "every day" — an unset mask must never mean "never".
    pub days: Option<Option<Vec<u8>>>,
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
        let row: Option<ScheduleRow> = sqlx_core::query_as::query_as(&format!(
            "SELECT {SCHEDULE_COLUMNS} FROM sensei.schedules WHERE name = $1"
        ))
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("load_schedule({name}): {e}"))?;

        Ok(row.map(StoredSchedule::from))
    }

    /// Every schedule, in name order — the read behind `GET /api/tasks/scheduled`.
    ///
    /// The TABLE is the registry now. The endpoint used to serve a static Rust
    /// list whose own comment warned it would drift ("keep in step when a worker
    /// is added"); reading the rows means a worker's cadence, its enabled flag
    /// and its last outcome all come from the one place a user can edit.
    pub async fn list_schedules(&self) -> Result<Vec<StoredSchedule>, String> {
        let rows: Vec<ScheduleRow> = sqlx_core::query_as::query_as(&format!(
            "SELECT {SCHEDULE_COLUMNS} FROM sensei.schedules ORDER BY name"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("list_schedules: {e}"))?;
        Ok(rows.into_iter().map(StoredSchedule::from).collect())
    }

    /// Apply a user's edit to one schedule, returning the row as it now stands,
    /// or `None` when no such row exists.
    ///
    /// `None` rather than an upsert: the legal names are code-side
    /// ([`crate::tasks::schedule::SCHEDULABLE`]), so a row conjured by a PATCH
    /// would schedule a worker that does not exist. The caller 404s.
    ///
    /// **`modified_at` is bumped, and that is the point.** Deploy order is
    /// apply → import and `staging.import_schedules` guards on `modified_at`, so
    /// an edit that leaves it alone is silently reverted by the next
    /// `dbd deploy` — the user changes a cadence, it works, and then one day it
    /// does not. Runtime state (`last_run_at` / `last_ok` / `last_error`) is
    /// deliberately untouched: editing a schedule is not a run, and must neither
    /// claim one happened nor erase a recorded failure.
    pub async fn update_schedule(
        &self,
        name: &str,
        patch: &SchedulePatch,
    ) -> Result<Option<StoredSchedule>, String> {
        let (window_given, ws, we) = match patch.window {
            None => (false, None, None),
            Some(None) => (true, None, None),
            Some(Some((s, e))) => (true, Some(s), Some(e)),
        };
        // An empty mask is stored as NULL, the table's spelling of "every day" —
        // `days = '{}'` reads the same to the predicate but slips past the ISO
        // 1..7 CHECK, so normalising here keeps one representation in the table.
        let days: Option<Vec<i16>> = patch.days.as_ref().and_then(|d| {
            d.as_ref().filter(|v| !v.is_empty()).map(|v| v.iter().map(|&d| i16::from(d)).collect())
        });
        // Refuse an out-of-range cadence rather than clamping it: a silently
        // capped interval is a schedule the user did not ask for.
        let interval = patch
            .interval_secs
            .map(|s| {
                i32::try_from(s).map_err(|_| {
                    format!("update_schedule({name}): interval_secs {s} exceeds the column range")
                })
            })
            .transpose()?;
        let row: Option<ScheduleRow> = sqlx_core::query_as::query_as(&format!(
            "UPDATE sensei.schedules \
                SET enabled       = COALESCE($2, enabled) \
                  , interval_secs = COALESCE($3, interval_secs) \
                  , window_start  = CASE WHEN $4 THEN $5::time ELSE window_start END \
                  , window_end    = CASE WHEN $4 THEN $6::time ELSE window_end END \
                  , days          = CASE WHEN $7 THEN $8::smallint[] ELSE days END \
                  , modified_at   = now() \
              WHERE name = $1 \
          RETURNING {SCHEDULE_COLUMNS}"
        ))
        .bind(name)
        .bind(patch.enabled)
        .bind(interval)
        .bind(window_given)
        .bind(ws)
        .bind(we)
        .bind(patch.days.is_some())
        .bind(days)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("update_schedule({name}): {e}"))?;

        Ok(row.map(StoredSchedule::from))
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
