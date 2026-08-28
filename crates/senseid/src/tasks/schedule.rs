//! When a scheduled task may run — the pure decision, no clock and no database.
//!
//! A schedule is CONFIGURATION: enabled, how often, and optionally only during
//! certain hours on certain days. What the task then does is code, and which
//! names are legal is code (see [`SCHEDULABLE`]) — a schedule naming a task with
//! no implementation is a bug, not data.
//!
//! Everything here takes its inputs explicitly so the whole decision is testable
//! without waiting, without a database, and without a timezone: the caller
//! converts to local time once and passes the result in.
//!
//! Spec: docs/spec/daemon/schedules.md.

use chrono::{DateTime, Datelike, NaiveDateTime, NaiveTime, Timelike, Utc};

/// The tasks a schedule may name.
///
/// Code-side on purpose. The table is user-editable, so a typo'd or stale name
/// must be a loud failure rather than a row that silently never runs — and a
/// worker added without an entry here is exactly the drift
/// `api/handlers/scheduled_tasks.rs` warns about in its own comment. A test
/// asserts this list and `sensei.schedules` agree in both directions.
pub const SCHEDULABLE: &[&str] = &[
    "activity_prune",
    "advance_run",
    "analyzer",
    "capture_drain",
    "contribute",
    "dojo_sync",
    "index_audit",
    "library_update",
    "log_prune",
    "metrics",
    "reconcile",
    "watchdog",
];

/// One row of `sensei.schedules`, as the decision needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub name: String,
    pub enabled: bool,
    pub interval_secs: u32,
    /// Start of the allowed window. `None` (with `window_end`) = any time.
    pub window_start: Option<NaiveTime>,
    pub window_end: Option<NaiveTime>,
    /// ISO weekdays the task may run on (1 = Monday … 7 = Sunday). Empty = every
    /// day, so an unset mask does not mean "never".
    pub days: Vec<u8>,
}

/// How often to WAKE UP and re-ask whether to run — not the schedule's interval.
///
/// A daily task with an overnight window, polled daily, wakes at whatever time
/// it first woke and skips forever: its window is only open for three hours and
/// the poll never lands inside them. So a worker must wake often enough to
/// notice its window open. 60s bounds window granularity; a sub-minute interval
/// (advance_run is 15s) keeps its own cadence rather than being slowed down.
pub fn poll_secs(interval_secs: u32) -> u64 {
    u64::from(interval_secs).clamp(1, 60)
}

/// Why a task is not running right now. Returned rather than a bare bool so the
/// UI and the logs can say WHICH reason — "disabled" and "outside its window"
/// are different answers to "why has this not run?", and collapsing them into
/// one silence is the failure mode this whole slice exists to avoid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skip {
    /// `enabled = false`. On-demand paths still work.
    Disabled,
    /// The interval has not elapsed since the last run.
    NotDue,
    /// Wrong time of day.
    OutsideWindow,
    /// Wrong day of week.
    WrongDay,
    /// No `sensei.schedules` row. The code↔table agreement test makes this a
    /// build failure, so reaching it at runtime means something is genuinely
    /// wrong — and not running is the honest response.
    Unscheduled,
}

/// Is `now` inside the schedule's allowed time-of-day window?
///
/// A window whose start is AFTER its end wraps midnight: `22:00–05:00` is how
/// people say "overnight", and reading it as an empty range would mean a nightly
/// task silently never runs. Either bound unset means "any time".
pub fn within_window(now: NaiveTime, start: Option<NaiveTime>, end: Option<NaiveTime>) -> bool {
    let (Some(start), Some(end)) = (start, end) else { return true };
    if start <= end {
        // An ordinary same-day window. Inclusive of both bounds: a window of
        // 09:00–09:00 is a single instant, not an empty one.
        now >= start && now <= end
    } else {
        // Wraps midnight — inside if it is after the start OR before the end.
        now >= start || now <= end
    }
}

/// Is `weekday` (ISO 1=Mon..7=Sun) one the schedule allows? Empty = every day.
pub fn on_allowed_day(weekday: u8, days: &[u8]) -> bool {
    days.is_empty() || days.contains(&weekday)
}

/// Has `interval_secs` elapsed since the last run? Never-run is always due, so a
/// freshly seeded schedule runs on the first tick rather than waiting a full
/// interval for its first ever pass.
pub fn is_due(now_utc: DateTime<Utc>, last_run: Option<DateTime<Utc>>, interval_secs: u32) -> bool {
    match last_run {
        None => true,
        Some(prev) => (now_utc - prev).num_seconds() >= i64::from(interval_secs),
    }
}

/// The whole decision: `Ok(())` to run, `Err(Skip)` with the reason not to.
///
/// `now_local` decides the window and the weekday (a user saying "not during my
/// working hours" means their hours); `now_utc` decides due-ness, because an
/// elapsed duration must not shift when the clocks change.
pub fn should_run(
    s: &Schedule,
    now_local: NaiveDateTime,
    now_utc: DateTime<Utc>,
    last_run: Option<DateTime<Utc>>,
) -> Result<(), Skip> {
    if !s.enabled {
        return Err(Skip::Disabled);
    }
    // Weekday before window: on a disallowed day the time of day is irrelevant,
    // and reporting "outside window" there would send someone hunting the wrong
    // setting.
    let weekday = u8::try_from(now_local.weekday().number_from_monday()).unwrap_or(1);
    if !on_allowed_day(weekday, &s.days) {
        return Err(Skip::WrongDay);
    }
    if !within_window(now_local.time(), s.window_start, s.window_end) {
        return Err(Skip::OutsideWindow);
    }
    if !is_due(now_utc, last_run, s.interval_secs) {
        return Err(Skip::NotDue);
    }
    Ok(())
}

/// A human-readable window, for the API and the UI. `None` = any time.
pub fn window_label(start: Option<NaiveTime>, end: Option<NaiveTime>) -> Option<String> {
    let (start, end) = (start?, end?);
    Some(format!("{:02}:{:02}–{:02}:{:02}", start.hour(), start.minute(), end.hour(), end.minute()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }
    /// A local datetime on a known weekday. 2026-08-31 is a Monday.
    fn monday(h: u32, m: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 8, 31).unwrap().and_hms_opt(h, m, 0).unwrap()
    }
    fn sunday(h: u32, m: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 8, 30).unwrap().and_hms_opt(h, m, 0).unwrap()
    }
    fn utc(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000 + secs, 0).unwrap()
    }
    fn sched() -> Schedule {
        Schedule {
            name: "metrics".into(),
            enabled: true,
            interval_secs: 3600,
            window_start: None,
            window_end: None,
            days: vec![],
        }
    }

    #[test]
    fn an_unset_window_means_any_time() {
        // The common case, and the dangerous default to get wrong: unset must
        // mean "always", never "never".
        assert!(within_window(t(3, 0), None, None));
        assert!(within_window(t(3, 0), Some(t(9, 0)), None), "half-set is still any time");
        assert!(within_window(t(3, 0), None, Some(t(17, 0))));
    }

    #[test]
    fn an_ordinary_window_includes_its_bounds() {
        assert!(!within_window(t(8, 59), Some(t(9, 0)), Some(t(17, 0))), "before");
        assert!(within_window(t(9, 0), Some(t(9, 0)), Some(t(17, 0))), "at the start");
        assert!(within_window(t(12, 0), Some(t(9, 0)), Some(t(17, 0))), "inside");
        assert!(within_window(t(17, 0), Some(t(9, 0)), Some(t(17, 0))), "at the end");
        assert!(!within_window(t(17, 1), Some(t(9, 0)), Some(t(17, 0))), "after");
    }

    #[test]
    fn a_window_that_wraps_midnight_covers_the_night() {
        // 22:00–05:00 is how people say "overnight". Read as an ordinary range it
        // would be empty, and a nightly task would silently never run — the
        // single most likely bug in this module.
        let (s, e) = (Some(t(22, 0)), Some(t(5, 0)));
        assert!(within_window(t(23, 0), s, e), "before midnight");
        assert!(within_window(t(0, 30), s, e), "just after midnight");
        assert!(within_window(t(4, 59), s, e), "just before the end");
        assert!(!within_window(t(12, 0), s, e), "noon is NOT in an overnight window");
        assert!(!within_window(t(21, 59), s, e), "just before it opens");
    }

    #[test]
    fn an_empty_day_mask_means_every_day() {
        // Same trap as the window: unset must not mean "never".
        assert!(on_allowed_day(1, &[]));
        assert!(on_allowed_day(7, &[]));
    }

    #[test]
    fn a_day_mask_admits_only_its_days() {
        let weekdays = [1u8, 2, 3, 4, 5];
        assert!(on_allowed_day(1, &weekdays), "Monday");
        assert!(on_allowed_day(5, &weekdays), "Friday");
        assert!(!on_allowed_day(6, &weekdays), "Saturday");
        assert!(!on_allowed_day(7, &weekdays), "Sunday");
    }

    #[test]
    fn never_run_is_always_due() {
        // A freshly seeded schedule must run on the first tick, not wait a full
        // interval for its first ever pass.
        assert!(is_due(utc(0), None, 3600));
    }

    #[test]
    fn due_only_once_the_interval_has_elapsed() {
        let last = utc(0);
        assert!(!is_due(utc(3599), Some(last), 3600), "one second short");
        assert!(is_due(utc(3600), Some(last), 3600), "exactly elapsed");
        assert!(is_due(utc(7200), Some(last), 3600), "long overdue");
    }

    #[test]
    fn disabled_beats_everything_else() {
        // And it reports DISABLED, not NotDue — "why has this not run?" has one
        // right answer and it is the one the user can act on.
        let s = Schedule { enabled: false, ..sched() };
        assert_eq!(should_run(&s, monday(12, 0), utc(0), None), Err(Skip::Disabled));
    }

    #[test]
    fn the_wrong_day_is_reported_as_the_wrong_day() {
        // Not OutsideWindow — that would send someone hunting the wrong setting.
        let s = Schedule { days: vec![1, 2, 3, 4, 5], ..sched() };
        assert_eq!(should_run(&s, sunday(12, 0), utc(0), None), Err(Skip::WrongDay));
        assert_eq!(should_run(&s, monday(12, 0), utc(0), None), Ok(()));
    }

    #[test]
    fn outside_the_window_is_distinguishable_from_not_due() {
        let s = Schedule { window_start: Some(t(9, 0)), window_end: Some(t(17, 0)), ..sched() };
        assert_eq!(should_run(&s, monday(3, 0), utc(0), None), Err(Skip::OutsideWindow));
        // Inside the window but too soon since the last run.
        assert_eq!(should_run(&s, monday(12, 0), utc(60), Some(utc(0))), Err(Skip::NotDue));
        assert_eq!(should_run(&s, monday(12, 0), utc(3600), Some(utc(0))), Ok(()));
    }

    #[test]
    fn an_overnight_window_runs_a_nightly_task_at_3am() {
        // The end-to-end shape of "prune at 3am", which is the ask this design
        // came from.
        let s = Schedule {
            name: "activity_prune".into(),
            interval_secs: 86_400,
            window_start: Some(t(2, 0)),
            window_end: Some(t(5, 0)),
            ..sched()
        };
        assert_eq!(should_run(&s, monday(3, 0), utc(0), None), Ok(()));
        assert_eq!(should_run(&s, monday(13, 0), utc(0), None), Err(Skip::OutsideWindow));
    }

    #[test]
    fn window_label_renders_or_says_nothing() {
        assert_eq!(window_label(Some(t(22, 0)), Some(t(5, 30))).as_deref(), Some("22:00–05:30"));
        assert_eq!(window_label(None, None), None);
        assert_eq!(window_label(Some(t(9, 0)), None), None, "half a window is not a window");
    }

    #[test]
    fn poll_is_capped_so_a_windowed_daily_task_can_notice_its_window() {
        // The bug this exists to prevent: polled once a day, a task with a
        // 02:00-05:00 window wakes at whatever hour it started and never lands
        // inside its window again.
        assert_eq!(poll_secs(86_400), 60, "daily must still poll every minute");
        assert_eq!(poll_secs(3600), 60, "hourly too");
        assert_eq!(poll_secs(15), 15, "a sub-minute cadence is NOT slowed to 60s");
        assert_eq!(poll_secs(60), 60);
        assert_eq!(poll_secs(0), 1, "never zero — that would busy-loop");
    }

    /// The seed datafile that ships the default schedule for every worker.
    /// Parsed rather than mocked: it is what actually reaches the database, and
    /// a cadence claim tested against a Rust constant would no longer be testing
    /// the thing that runs.
    fn seed_intervals() -> std::collections::HashMap<String, u64> {
        let path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../database/import/staging/schedules.jsonl");
        std::fs::read_to_string(path)
            .expect("the schedules seed datafile must exist")
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let v: serde_json::Value = serde_json::from_str(l).expect("each seed line is JSON");
                (
                    v["name"].as_str().expect("name").to_string(),
                    v["interval_secs"].as_u64().expect("interval_secs"),
                )
            })
            .collect()
    }

    #[test]
    fn the_seed_covers_every_schedulable_worker_with_a_usable_cadence() {
        // The datafile is the source of truth for defaults now. A worker missing
        // from it has no row, so it never runs — and the DB CHECK rejects a zero
        // interval, so a typo here fails the deploy rather than busy-looping.
        let seed = seed_intervals();
        for name in SCHEDULABLE {
            let secs = seed.get(*name).unwrap_or_else(|| {
                panic!(
                    "{name} is schedulable but missing from schedules.jsonl — it would never run"
                )
            });
            assert!(*secs > 0, "{name} has a zero interval in the seed");
        }
        assert_eq!(
            seed.len(),
            SCHEDULABLE.len(),
            "the seed has an entry for a worker that does not exist"
        );
    }

    #[test]
    fn the_seeded_cadences_match_what_each_worker_needs() {
        // These were three scattered assertions against Rust constants before the
        // cadence became data. The intent is unchanged; only the source of truth
        // moved, so they are asserted where the value now actually lives.
        let seed = seed_intervals();
        assert_eq!(
            seed["reconcile"], 300,
            "reconcile must be FREQUENT — the mtime gate makes a no-op cheap, and the \
             old hourly cadence let drift sit"
        );
        assert_eq!(
            seed["index_audit"], 86_400,
            "the audit stats every indexed file/folder — heavier than reconcile, so daily"
        );
        assert_eq!(seed["advance_run"], 15, "run advancement is user-facing latency");
        assert_eq!(seed["watchdog"], 60, "a stalled watcher must be caught within a minute");
    }

    #[test]
    fn schedulable_names_are_unique_and_sorted() {
        // A duplicate would make one entry unreachable; sorted keeps the diff
        // readable when a task is added.
        let mut sorted = SCHEDULABLE.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.as_slice(), SCHEDULABLE, "SCHEDULABLE must be sorted and duplicate-free");
    }
}
