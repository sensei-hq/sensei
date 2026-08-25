//! Relay-engine (P3.4) — pure limit detection + pause-window math.
//!
//! **The 5-day-run failure this fixes.** Claude Code surfaces a rate/usage limit
//! as a **CLI TEXT message on stdout**, NOT a structured 429 (relay-engine §4.5,
//! §5). The manual run stopped on exactly one such line — *"Paused — you've hit
//! your limit. Limits will reset at 11:29 AM · View usage"* — because nothing
//! recognized it. This module recognizes that message shape and turns it into a
//! pause window; the caller ([`crate::tasks::handlers::advance_run`]) pauses the
//! run with `paused_until = reset + jitter` and the already-built
//! `resume_due_runs` scheduler auto-resumes it. **Degrade, never stop** (§5): a
//! limit is a pause, never a failure/stop.
//!
//! Everything here is **pure + unit-testable** (no clock reads, no DB, no env):
//! `now` and `jitter` are passed in so the whole module can be exercised with
//! fixed inputs. The caller supplies `chrono::Utc::now()` + a small random jitter
//! at the boundary.
//!
//! **Local-time assumption (documented, §5).** Claude Code prints the reset time
//! in the machine's **local** wall-clock (e.g. "11:29 AM"). A bare clock time has
//! no date and no zone, so we interpret it against **local `now`** (via
//! [`chrono::Local`]) — "the next time the local clock reads 11:29" — then
//! convert to `Utc` for storage. A same-clock-time-today that has already passed
//! rolls to tomorrow (limits reset forward, never backward).
//!
//! **Scope.** This is the CLI-text path from the `claude -p` drive only. The
//! gateway-routed calls (`gateway-embedded`) normalize their own rate/usage-limit
//! errors incl. reset time (relay-engine §8, change surface) — that 429 path is
//! handled elsewhere, NOT here.

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};

/// A detected limit hit parsed out of an agent's combined stdout+stderr.
///
/// `reset_at` is `Some` when a reset time could be parsed from the message and
/// `None` when a limit was clearly detected but the reset time is
/// unparseable/ambiguous — the caller then applies a capped default backoff
/// ([`resolve_paused_until`]). `reason` is a short, logical label safe to store
/// on a `run_event` detail (never raw stdout).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitHit {
    /// The parsed reset instant (UTC), or `None` if the message gave no
    /// parseable reset time.
    pub reset_at: Option<DateTime<Utc>>,
    /// Short logical reason, e.g. `"rate/usage limit reached"`. Safe for event
    /// detail — never the raw agent output.
    pub reason: String,
}

/// Case-insensitive markers that identify a Claude Code limit message. Any one
/// of these appearing in the combined output means the step was limited. Kept
/// broad (both "you've"/"you have" contractions, rate + usage + weekly-cap
/// wording) because Claude Code's exact copy varies across versions.
const LIMIT_MARKERS: [&str; 7] = [
    "you've hit your limit",
    "you have hit your limit",
    "usage limit",
    "rate limit",
    "limits will reset at",
    "limit reached",
    "reached your limit",
];

/// Scan `output` (combined stdout+stderr) for a Claude Code limit message.
///
/// Returns `Some(LimitHit)` iff one of [`LIMIT_MARKERS`] appears
/// (case-insensitive). When present, we try to extract a reset time from a
/// "reset at <TIME>" / "resets at <TIME>" phrase:
///
/// - a bare clock time ("11:29 AM" / "11:29am" / "23:29") → the **next
///   occurrence** of that local wall-clock time from `now` (rolls to tomorrow if
///   already past today);
/// - a weekday ("resets Monday") or an explicit month/day ("on Jul 20") → that
///   date (at the parsed time-of-day if one is present, else local midnight);
/// - anything unparseable/ambiguous → `reset_at: None` (caller backs off).
///
/// `now` is injected (pure): pass `chrono::Utc::now()` at the call site.
pub fn detect_limit(output: &str, now: DateTime<Utc>) -> Option<LimitHit> {
    let lower = output.to_ascii_lowercase();
    if !LIMIT_MARKERS.iter().any(|m| lower.contains(m)) {
        return None;
    }
    let reset_at = parse_reset_at(&lower, now);
    Some(LimitHit { reset_at, reason: "rate/usage limit reached".to_string() })
}

/// The intended pause deadline for a limit hit.
///
/// - `reset_at` present → `reset_at + jitter` (spread the herd so many runs
///   don't all resume on the same instant);
/// - `reset_at` absent (unparseable/ambiguous) → `now + default_backoff` (capped
///   retry; the next tick re-checks).
///
/// Pure: `now`/`jitter`/`default_backoff` are all params, so it is testable
/// without touching the clock.
pub fn resolve_paused_until(
    hit: &LimitHit,
    now: DateTime<Utc>,
    jitter: Duration,
    default_backoff: Duration,
) -> DateTime<Utc> {
    match hit.reset_at {
        Some(reset) => reset + jitter,
        None => now + default_backoff,
    }
}

// ── internal parsing ────────────────────────────────────────────────────────

/// Extract a reset instant from a lower-cased limit message. Tries, in order:
/// an explicit "reset(s) at <…>" phrase, then a weekday mention ("reset(s)
/// monday"), then a bare month/day date. `None` if nothing parses.
fn parse_reset_at(lower: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    // The tail after "reset at" / "resets at" / "will reset at" holds the time
    // (and possibly a date). We scan from the *last* such marker so trailing
    // copy like "· View usage" doesn't matter.
    let tail = reset_tail(lower);

    // 1. Bare clock time in the reset tail → next occurrence of that local time.
    if let Some(tail) = tail
        && let Some(t) = parse_clock_time(tail)
    {
        // If the tail also names a weekday/date, anchor to that date at time t.
        if let Some(date) = parse_weekday(tail, now).or_else(|| parse_month_day(tail, now)) {
            return local_datetime_to_utc(date, t);
        }
        return Some(next_occurrence_of_local_time(t, now));
    }

    // 2. Weekday elsewhere in the message (e.g. "resets monday") → that date at
    //    local midnight (no time-of-day given).
    if let Some(date) = parse_weekday(lower, now) {
        return local_datetime_to_utc(date, NaiveTime::from_hms_opt(0, 0, 0)?);
    }

    // 3. An explicit month/day ("on jul 20") with no time → that date, midnight.
    if let Some(date) = parse_month_day(lower, now) {
        return local_datetime_to_utc(date, NaiveTime::from_hms_opt(0, 0, 0)?);
    }

    None
}

/// Return the substring after the last "reset at" / "resets at" / "reset to"
/// (and bare "reset " / "resets ") style marker, so we parse the time that
/// follows the reset phrase rather than some unrelated number elsewhere in the
/// copy. The bare "resets "/"reset " forms cover no-"at" copy like "resets 3am".
/// The longest/latest match wins (see the `max` below), so "resets at 3pm"
/// still yields the tail "3pm", not "at 3pm".
fn reset_tail(lower: &str) -> Option<&str> {
    const MARKERS: [&str; 5] = ["reset at ", "resets at ", "reset to ", "resets ", "reset "];
    let mut best: Option<usize> = None;
    for m in MARKERS {
        if let Some(idx) = lower.rfind(m) {
            let start = idx + m.len();
            best = Some(best.map_or(start, |b| b.max(start)));
        }
    }
    best.map(|start| &lower[start..])
}

/// Parse the first clock time in `s`. Accepts, in order of specificity:
/// - `H:MM` / `HH:MM` optionally followed by am/pm ("11:29 am", "11:29am",
///   "9:05 pm"), or a 24-hour `HH:MM` ("23:29");
/// - a **bare hour** immediately (optional whitespace) followed by am/pm, with
///   **no colon or minutes** — minutes default to 0 ("3pm"→15:00, "3am"→03:00,
///   "11am"→11:00, "12am"→00:00, "12pm"→12:00, "3 pm"→15:00). This matches the
///   common Claude Code copy that prints the reset with no minutes.
///
/// A bare number with NO am/pm is NOT a time (so the "5" in "5-hour limit
/// reached" is not mistaken for a reset hour). Returns the wall-clock
/// [`NaiveTime`] (no date).
fn parse_clock_time(s: &str) -> Option<NaiveTime> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            // Read up to 2 hour digits.
            let h_start = i;
            let mut j = i;
            while j < bytes.len() && j < h_start + 2 && bytes[j].is_ascii_digit() {
                j += 1;
            }
            // Case 1: "H:MM" — a ':' then exactly 2 minute digits.
            if j < bytes.len() && bytes[j] == b':' {
                let colon = j;
                if colon + 2 < bytes.len()
                    && bytes[colon + 1].is_ascii_digit()
                    && bytes[colon + 2].is_ascii_digit()
                {
                    let hour: u32 = s[h_start..colon].parse().ok()?;
                    let min: u32 = s[colon + 1..colon + 3].parse().ok()?;
                    // Look for an am/pm suffix within the next few chars.
                    let rest = &s[colon + 3..];
                    let ampm = am_pm(rest);
                    return build_time(hour, min, ampm);
                }
            } else if let Some(ampm) = am_pm(&s[j..]) {
                // Case 2: bare hour + am/pm (no colon, no minutes). Only accept
                // when am/pm is actually present — a bare number alone is not a
                // time (avoids the "5" in "5-hour limit reached"). Minutes = 0.
                let hour: u32 = s[h_start..j].parse().ok()?;
                return build_time(hour, 0, Some(ampm));
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    None
}

/// am/pm as parsed from the text immediately after a clock time. `Some(true)` =
/// pm, `Some(false)` = am, `None` = none present (24-hour interpretation).
fn am_pm(rest: &str) -> Option<bool> {
    let trimmed = rest.trim_start();
    if trimmed.starts_with("am") || trimmed.starts_with("a.m") {
        Some(false)
    } else if trimmed.starts_with("pm") || trimmed.starts_with("p.m") {
        Some(true)
    } else {
        None
    }
}

/// Build a [`NaiveTime`] from an hour/minute and an optional am/pm. With am/pm
/// the hour is a 12-hour clock (12am→0, 12pm→12); without, it is 24-hour.
fn build_time(hour: u32, min: u32, ampm: Option<bool>) -> Option<NaiveTime> {
    if min > 59 {
        return None;
    }
    let h24 = match ampm {
        None => {
            if hour > 23 {
                return None;
            }
            hour
        }
        Some(false) => {
            // am — 12-hour clock: valid hours are 1..=12 (12am→00). Reject 0
            // and 13+ (e.g. a bare "0am"/"13am" is not a real reset time).
            if hour == 12 {
                0
            } else if (1..=11).contains(&hour) {
                hour
            } else {
                return None;
            }
        }
        Some(true) => {
            // pm — 12-hour clock: valid hours are 1..=12 (12pm→12). Reject 0
            // and 13+.
            if hour == 12 {
                12
            } else if (1..=11).contains(&hour) {
                hour + 12
            } else {
                return None;
            }
        }
    };
    NaiveTime::from_hms_opt(h24, min, 0)
}

/// The next instant at which the LOCAL wall-clock reads `t`, at or after `now`.
/// Interprets `t` in [`chrono::Local`] (Claude prints local time), and if
/// today's occurrence has already passed, rolls to tomorrow. Falls back to a
/// no-adjustment `now + 1min` only if local resolution is pathologically
/// ambiguous (DST gap/overlap on both today and tomorrow — never in practice).
//
// FOLLOW-UP: an explicit IANA zone in the message (e.g. "(America/New_York)")
// is currently IGNORED — we assume the daemon's `Local` zone. When the printed
// zone differs from the daemon's, the resolved reset can be off by the zone
// offset. A precise fix needs a tz database dependency (chrono-tz) to resolve
// the named zone; deferred (tracked in decisions.md). Not adding chrono-tz or
// parsing the zone now.
fn next_occurrence_of_local_time(t: NaiveTime, now: DateTime<Utc>) -> DateTime<Utc> {
    let local_now = now.with_timezone(&Local);
    let today = local_now.date_naive();
    // Try today, then tomorrow; pick the first occurrence that is >= now.
    for add_days in 0..=1 {
        let date = today + Duration::days(add_days);
        if let Some(utc) = local_date_time_to_utc(date, t)
            && utc >= now
        {
            return utc;
        }
    }
    // Extremely defensive fallback: a short backoff rather than a past instant.
    now + Duration::minutes(1)
}

/// Convert a local date + time to UTC, resolving DST ambiguity to the earliest
/// valid instant. `None` only in a DST *gap* (the wall-clock time does not exist
/// that day).
fn local_date_time_to_utc(date: NaiveDate, t: NaiveTime) -> Option<DateTime<Utc>> {
    let naive = date.and_time(t);
    match Local.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => Some(dt.with_timezone(&Utc)),
        chrono::LocalResult::Ambiguous(dt, _) => Some(dt.with_timezone(&Utc)),
        chrono::LocalResult::None => None,
    }
}

/// Like [`local_date_time_to_utc`] but returns `Some` for the common case; used
/// where an explicit date was named (weekday/month-day) so we anchor to it even
/// if it isn't strictly after `now`.
fn local_datetime_to_utc(date: NaiveDate, t: NaiveTime) -> Option<DateTime<Utc>> {
    local_date_time_to_utc(date, t)
}

/// If `s` names a weekday, return the next date (local) that falls on that
/// weekday at or after `now`'s local date. "today" resolves to today; a weekly
/// cap that names the same weekday as today rolls a full week forward only when
/// the time-of-day handling upstream decides so — here we return the nearest
/// forward date INCLUDING today, and the caller anchors the time.
fn parse_weekday(s: &str, now: DateTime<Utc>) -> Option<NaiveDate> {
    let target = weekday_from_str(s)?;
    let local_today = now.with_timezone(&Local).date_naive();
    // Walk forward up to 7 days to the target weekday (0 = today).
    for add in 0..7 {
        let d = local_today + Duration::days(add);
        if d.weekday() == target {
            return Some(d);
        }
    }
    None
}

/// Map an English weekday name (full or 3-letter) found anywhere in `s` to a
/// [`Weekday`]. Longest names are checked first so "sun" in "sunday" resolves to
/// the same day either way.
fn weekday_from_str(s: &str) -> Option<Weekday> {
    const DAYS: [(&str, Weekday); 7] = [
        ("monday", Weekday::Mon),
        ("tuesday", Weekday::Tue),
        ("wednesday", Weekday::Wed),
        ("thursday", Weekday::Thu),
        ("friday", Weekday::Fri),
        ("saturday", Weekday::Sat),
        ("sunday", Weekday::Sun),
    ];
    const ABBR: [(&str, Weekday); 7] = [
        ("mon", Weekday::Mon),
        ("tue", Weekday::Tue),
        ("wed", Weekday::Wed),
        ("thu", Weekday::Thu),
        ("fri", Weekday::Fri),
        ("sat", Weekday::Sat),
        ("sun", Weekday::Sun),
    ];
    for (name, wd) in DAYS {
        if s.contains(name) {
            return Some(wd);
        }
    }
    for (name, wd) in ABBR {
        if s.contains(name) {
            return Some(wd);
        }
    }
    None
}

/// Parse an explicit month+day ("jul 20", "july 20", "on jul 20") into a
/// [`NaiveDate`], choosing the next occurrence of that month/day at or after
/// `now`'s local date (rolls to next year if already past). Year is never in
/// Claude's copy, so we infer it. Returns `None` if no month/day is present.
fn parse_month_day(s: &str, now: DateTime<Utc>) -> Option<NaiveDate> {
    let month = month_from_str(s)?;
    // Find the month token, then the first 1-2 digit day after it.
    let idx = month_token_index(s, month.0)?;
    let after = &s[idx + month.0.len()..];
    let day = first_day_number(after)?;
    let local_today = now.with_timezone(&Local).date_naive();
    let year = local_today.year();
    // This year first; if that date is before today, roll to next year.
    let candidate = NaiveDate::from_ymd_opt(year, month.1, day)?;
    if candidate >= local_today {
        Some(candidate)
    } else {
        NaiveDate::from_ymd_opt(year + 1, month.1, day)
    }
}

/// The month name token present in `s` (full or 3-letter) and its 1-based month
/// number. Full names checked first.
fn month_from_str(s: &str) -> Option<(&'static str, u32)> {
    const MONTHS: [(&str, u32); 12] = [
        ("january", 1),
        ("february", 2),
        ("march", 3),
        ("april", 4),
        ("may", 5),
        ("june", 6),
        ("july", 7),
        ("august", 8),
        ("september", 9),
        ("october", 10),
        ("november", 11),
        ("december", 12),
    ];
    const ABBR: [(&str, u32); 12] = [
        ("jan", 1),
        ("feb", 2),
        ("mar", 3),
        ("apr", 4),
        ("may", 5),
        ("jun", 6),
        ("jul", 7),
        ("aug", 8),
        ("sep", 9),
        ("oct", 10),
        ("nov", 11),
        ("dec", 12),
    ];
    for (name, num) in MONTHS {
        if s.contains(name) {
            return Some((name, num));
        }
    }
    for (name, num) in ABBR {
        if s.contains(name) {
            return Some((name, num));
        }
    }
    None
}

/// Byte index where the month token starts in `s`.
fn month_token_index(s: &str, token: &str) -> Option<usize> {
    s.find(token)
}

/// First 1-2 digit number in `s`, treated as a day-of-month (1..=31). Skips
/// leading non-digits (spaces, commas, "the ").
fn first_day_number(s: &str) -> Option<u32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && !bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let start = i;
    while i < bytes.len() && i < start + 2 && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let day: u32 = s[start..i].parse().ok()?;
    if (1..=31).contains(&day) { Some(day) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike; // for `.hour()` on the resolved local reset instant

    /// A fixed `now` for deterministic assertions — 2026-07-17 08:00:00 UTC.
    /// (Tests never call `Utc::now()`.)
    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 17, 8, 0, 0).unwrap()
    }

    // ── detect_limit ────────────────────────────────────────────────────

    #[test]
    fn detects_the_exact_five_day_run_message() {
        // The literal line that stopped the 5-day run.
        let out = "Paused — you've hit your limit. Limits will reset at 11:29 AM · View usage";
        let hit = detect_limit(out, fixed_now()).expect("must detect the 5-day-run limit line");
        assert_eq!(hit.reason, "rate/usage limit reached");
        let reset = hit.reset_at.expect("must parse a reset time from 11:29 AM");

        // The reset is the next LOCAL 11:29 (am). Compute the expected instant
        // the same way the parser does so the test is timezone-independent.
        let t = NaiveTime::from_hms_opt(11, 29, 0).unwrap();
        let expected = next_occurrence_of_local_time(t, fixed_now());
        assert_eq!(reset, expected);
        // And it must be strictly in the future relative to now.
        assert!(reset >= fixed_now(), "reset must not be in the past");
        // Within the next 24h (either today-11:29 or tomorrow-11:29).
        assert!(reset <= fixed_now() + Duration::hours(24) + Duration::minutes(1));
    }

    #[test]
    fn detects_24h_clock_variant() {
        let out = "Rate limit hit. Your limit resets at 23:29 today.";
        let hit = detect_limit(out, fixed_now()).expect("detect 24h variant");
        let reset = hit.reset_at.expect("parse 23:29");
        let t = NaiveTime::from_hms_opt(23, 29, 0).unwrap();
        assert_eq!(reset, next_occurrence_of_local_time(t, fixed_now()));
    }

    #[test]
    fn detects_weekly_cap_weekday_variant() {
        // A weekly cap that names a weekday + a time.
        let out = "You've hit your limit. Limits will reset at 11:29 AM on Monday.";
        let hit = detect_limit(out, fixed_now()).expect("detect weekday variant");
        let reset = hit.reset_at.expect("parse weekday+time");
        // 2026-07-17 is a Friday; the next Monday is 2026-07-20.
        let expected_date = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
        let expected =
            local_datetime_to_utc(expected_date, NaiveTime::from_hms_opt(11, 29, 0).unwrap())
                .unwrap();
        assert_eq!(reset, expected);
    }

    #[test]
    fn detects_explicit_month_day_variant() {
        let out = "usage limit reached — reset at 11:29 AM on Jul 20";
        let hit = detect_limit(out, fixed_now()).expect("detect month/day variant");
        let reset = hit.reset_at.expect("parse month/day+time");
        let expected_date = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
        let expected =
            local_datetime_to_utc(expected_date, NaiveTime::from_hms_opt(11, 29, 0).unwrap())
                .unwrap();
        assert_eq!(reset, expected);
    }

    #[test]
    fn normal_output_is_not_a_limit() {
        let out = "Building crate senseid...\nFinished in 4.2s. All 217 tests passed.";
        assert_eq!(detect_limit(out, fixed_now()), None);
    }

    #[test]
    fn limit_without_parseable_time_yields_none_reset() {
        // A clear limit, but no reset time to parse.
        let out = "Paused — you've hit your limit. Try again later.";
        let hit = detect_limit(out, fixed_now()).expect("still a detected limit");
        assert_eq!(hit.reset_at, None, "no parseable time → reset_at None");
        assert_eq!(hit.reason, "rate/usage limit reached");
    }

    #[test]
    fn detection_is_case_insensitive() {
        let out = "PAUSED — YOU'VE HIT YOUR LIMIT. LIMITS WILL RESET AT 11:29 AM";
        assert!(detect_limit(out, fixed_now()).is_some());
    }

    // ── next_occurrence_of_local_time ───────────────────────────────────

    #[test]
    fn next_occurrence_rolls_to_tomorrow_when_time_has_passed() {
        // Pick a local time that is *before* local-now, so it must roll forward.
        let local_now = fixed_now().with_timezone(&Local);
        let past = local_now.time() - Duration::hours(1);
        let reset = next_occurrence_of_local_time(past, fixed_now());
        assert!(reset >= fixed_now(), "a past time-of-day rolls to tomorrow");
        assert!(reset <= fixed_now() + Duration::hours(24) + Duration::minutes(1));
    }

    #[test]
    fn next_occurrence_uses_today_when_time_is_ahead() {
        let local_now = fixed_now().with_timezone(&Local);
        let ahead = local_now.time() + Duration::hours(2);
        let reset = next_occurrence_of_local_time(ahead, fixed_now());
        // Should be within today (< 24h out and >= now).
        assert!(reset >= fixed_now());
        assert!(reset < fixed_now() + Duration::hours(24));
    }

    // ── resolve_paused_until ────────────────────────────────────────────

    #[test]
    fn resolve_uses_reset_plus_jitter_when_present() {
        let reset = Utc.with_ymd_and_hms(2026, 7, 17, 11, 29, 0).unwrap();
        let hit = LimitHit { reset_at: Some(reset), reason: "x".into() };
        let jitter = Duration::seconds(30);
        let out = resolve_paused_until(&hit, fixed_now(), jitter, Duration::minutes(30));
        assert_eq!(out, reset + jitter, "reset + jitter when a reset is known");
    }

    #[test]
    fn resolve_uses_default_backoff_when_reset_absent() {
        let hit = LimitHit { reset_at: None, reason: "x".into() };
        let backoff = Duration::minutes(30);
        let out = resolve_paused_until(&hit, fixed_now(), Duration::seconds(30), backoff);
        assert_eq!(out, fixed_now() + backoff, "now + default backoff when reset unknown");
    }

    // ── clock parser edge cases ─────────────────────────────────────────

    #[test]
    fn parse_clock_time_handles_am_pm_and_24h() {
        assert_eq!(parse_clock_time("11:29 am"), NaiveTime::from_hms_opt(11, 29, 0));
        assert_eq!(parse_clock_time("11:29am"), NaiveTime::from_hms_opt(11, 29, 0));
        assert_eq!(parse_clock_time("9:05 pm"), NaiveTime::from_hms_opt(21, 5, 0));
        assert_eq!(parse_clock_time("12:00 am"), NaiveTime::from_hms_opt(0, 0, 0));
        assert_eq!(parse_clock_time("12:00 pm"), NaiveTime::from_hms_opt(12, 0, 0));
        assert_eq!(parse_clock_time("23:29"), NaiveTime::from_hms_opt(23, 29, 0));
        assert_eq!(parse_clock_time("no time here"), None);
        assert_eq!(parse_clock_time("25:99"), None, "out-of-range is rejected");
    }

    // ── bare-hour (no-colon) am/pm parsing ──────────────────────────────
    // Real Claude Code limit copy commonly prints the reset with NO minutes,
    // e.g. "resets 3am" / "reset at 3pm". These must map to :00 of that hour.

    #[test]
    fn parse_clock_time_handles_bare_hour_am_pm() {
        assert_eq!(parse_clock_time("3pm"), NaiveTime::from_hms_opt(15, 0, 0));
        assert_eq!(parse_clock_time("3am"), NaiveTime::from_hms_opt(3, 0, 0));
        assert_eq!(parse_clock_time("11am"), NaiveTime::from_hms_opt(11, 0, 0));
        assert_eq!(parse_clock_time("12am"), NaiveTime::from_hms_opt(0, 0, 0));
        assert_eq!(parse_clock_time("12pm"), NaiveTime::from_hms_opt(12, 0, 0));
        // Optional whitespace between the hour and am/pm.
        assert_eq!(parse_clock_time("3 pm"), NaiveTime::from_hms_opt(15, 0, 0));
        // Out-of-range hours with am/pm are rejected (consistent with H:MM).
        assert_eq!(parse_clock_time("0am"), None, "hour 0 with am/pm is invalid");
        assert_eq!(parse_clock_time("13pm"), None, "hour 13 with am/pm is invalid");
        // A bare hour with NO am/pm is NOT a clock time (avoids grabbing the
        // "5" in "5-hour limit reached").
        assert_eq!(parse_clock_time("5-hour limit reached"), None);
    }

    #[test]
    fn detects_bare_hour_pm_with_iana_zone() {
        // The real Claude Code copy: no minutes, trailing IANA zone (ignored).
        let out = "Claude usage limit reached. Your limit will reset at 3pm (America/New_York).";
        let hit = detect_limit(out, fixed_now()).expect("detect bare-hour pm variant");
        let reset = hit.reset_at.expect("must parse a reset time from 3pm");
        let t = NaiveTime::from_hms_opt(15, 0, 0).unwrap();
        assert_eq!(reset, next_occurrence_of_local_time(t, fixed_now()));
        // Reset is the next local 15:00 — the hour of the local wall-clock is 15.
        assert_eq!(reset.with_timezone(&Local).hour(), 15);
    }

    #[test]
    fn detects_bare_hour_am_with_bullet_separator() {
        // "5-hour limit reached ∙ resets 3am" — the "5" must NOT be mistaken for
        // the reset hour; the reset tail after "resets " is "3am".
        let out = "5-hour limit reached ∙ resets 3am";
        let hit = detect_limit(out, fixed_now()).expect("detect bare-hour am variant");
        let reset = hit.reset_at.expect("must parse a reset time from 3am");
        let t = NaiveTime::from_hms_opt(3, 0, 0).unwrap();
        assert_eq!(reset, next_occurrence_of_local_time(t, fixed_now()));
        assert_eq!(reset.with_timezone(&Local).hour(), 3);
    }

    #[test]
    fn detects_bare_hour_reset_at_variants() {
        // "resets at 11am" → 11:00 local.
        let out = "usage limit reached — resets at 11am";
        let hit = detect_limit(out, fixed_now()).expect("detect resets-at 11am");
        let reset = hit.reset_at.expect("parse 11am");
        assert_eq!(reset.with_timezone(&Local).hour(), 11);

        // "reset at 12am" → 00:00 local.
        let out = "usage limit reached — reset at 12am";
        let hit = detect_limit(out, fixed_now()).expect("detect reset-at 12am");
        let reset = hit.reset_at.expect("parse 12am");
        assert_eq!(reset.with_timezone(&Local).hour(), 0);

        // "reset at 12pm" → 12:00 local.
        let out = "usage limit reached — reset at 12pm";
        let hit = detect_limit(out, fixed_now()).expect("detect reset-at 12pm");
        let reset = hit.reset_at.expect("parse 12pm");
        assert_eq!(reset.with_timezone(&Local).hour(), 12);
    }
}
