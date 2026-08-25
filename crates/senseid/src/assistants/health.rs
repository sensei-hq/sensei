//! Pure, DB-free health vocabulary + freshness math for ACP adapters.
//! Everything here is unit-testable without a daemon, DB, or network.

use chrono::{Datelike, TimeZone, Utc, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Ok,
    Warn,
    Fail,
    Unknown,
}

impl CheckStatus {
    /// Severity for "worst-of" aggregation: Fail > Warn > Unknown > Ok.
    fn rank(self) -> u8 {
        match self {
            CheckStatus::Ok => 0,
            CheckStatus::Unknown => 1,
            CheckStatus::Warn => 2,
            CheckStatus::Fail => 3,
        }
    }
    /// The more-severe of two statuses.
    pub fn worse(self, other: CheckStatus) -> CheckStatus {
        if other.rank() > self.rank() { other } else { self }
    }
    /// Aggregate an iterator of statuses to the worst. Empty => Ok.
    pub fn worst_of<'a>(it: impl Iterator<Item = &'a CheckStatus>) -> CheckStatus {
        it.fold(CheckStatus::Ok, |acc, s| acc.worse(*s))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterCheck {
    pub id: String,
    pub label: String,
    pub status: CheckStatus,
    pub detail: Option<String>,
}

impl AdapterCheck {
    pub fn new(id: &str, label: &str, status: CheckStatus, detail: Option<String>) -> Self {
        Self { id: id.to_string(), label: label.to_string(), status, detail }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterHealth {
    pub adapter_id: String,
    pub family: String,
    pub status: CheckStatus,
    pub checks: Vec<AdapterCheck>,
    pub resolvable: bool,
}

impl AdapterHealth {
    /// Build with `status` computed as the worst of `checks`.
    pub fn new(
        adapter_id: &str,
        family: &str,
        checks: Vec<AdapterCheck>,
        resolvable: bool,
    ) -> Self {
        let status = CheckStatus::worst_of(checks.iter().map(|c| &c.status));
        Self {
            adapter_id: adapter_id.to_string(),
            family: family.to_string(),
            status,
            checks,
            resolvable,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterResolveReport {
    pub adapter_id: String,
    pub ok: bool,
    pub actions: Vec<String>,
    pub errors: Vec<String>,
}

/// Hours elapsed between two epoch-millis instants. When `exclude_weekends`,
/// any whole or partial Saturday/Sunday is removed from the elapsed total, so a
/// Friday-afternoon → Monday-morning gap counts only the working hours.
///
/// Implementation: walk the span in UTC and sum only the milliseconds that fall
/// on Mon–Fri. Coarse (1-minute step) on purpose — freshness thresholds are in
/// hours, so minute granularity is precise enough and keeps the loop cheap.
pub fn business_elapsed_hours(from_ms: i64, to_ms: i64, exclude_weekends: bool) -> f64 {
    if to_ms <= from_ms {
        return 0.0;
    }
    if !exclude_weekends {
        return (to_ms - from_ms) as f64 / 3_600_000.0;
    }
    const STEP_MS: i64 = 60_000; // 1 minute
    let mut counted_ms: i64 = 0;
    let mut t = from_ms;
    while t < to_ms {
        let dt = Utc.timestamp_millis_opt(t).single();
        let is_weekend =
            dt.map(|d| matches!(d.weekday(), Weekday::Sat | Weekday::Sun)).unwrap_or(false);
        let next = (t + STEP_MS).min(to_ms);
        if !is_weekend {
            counted_ms += next - t;
        }
        t = next;
    }
    counted_ms as f64 / 3_600_000.0
}

/// The capture-freshness check for an assistant family.
/// `last_ts` = newest hook_event ts (epoch ms) for the family, or None if the
/// daemon has never recorded one. `now_ms` = current epoch ms.
pub fn capture_freshness(
    last_ts: Option<i64>,
    now_ms: i64,
    window_hours: f64,
    exclude_weekends: bool,
) -> AdapterCheck {
    match last_ts {
        None => AdapterCheck::new(
            "events",
            "events flowing",
            CheckStatus::Warn,
            Some("never captured — no hook events recorded yet".into()),
        ),
        Some(ts) => {
            let elapsed = business_elapsed_hours(ts, now_ms, exclude_weekends);
            if elapsed <= window_hours {
                AdapterCheck::new(
                    "events",
                    "events flowing",
                    CheckStatus::Ok,
                    Some(format!("last event {:.1}h ago", elapsed)),
                )
            } else {
                AdapterCheck::new(
                    "events",
                    "events flowing",
                    CheckStatus::Fail,
                    Some(format!(
                        "stale: last event {:.1}h ago (window {}h)",
                        elapsed, window_hours
                    )),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worst_of_picks_fail_over_warn_over_ok() {
        let s = [CheckStatus::Ok, CheckStatus::Warn, CheckStatus::Fail];
        assert_eq!(CheckStatus::worst_of(s.iter()), CheckStatus::Fail);
        let s2 = [CheckStatus::Ok, CheckStatus::Warn];
        assert_eq!(CheckStatus::worst_of(s2.iter()), CheckStatus::Warn);
        let empty: [CheckStatus; 0] = [];
        assert_eq!(CheckStatus::worst_of(empty.iter()), CheckStatus::Ok);
    }

    #[test]
    fn unknown_is_worse_than_ok_but_better_than_warn() {
        assert_eq!(CheckStatus::Ok.worse(CheckStatus::Unknown), CheckStatus::Unknown);
        assert_eq!(CheckStatus::Unknown.worse(CheckStatus::Warn), CheckStatus::Warn);
    }

    #[test]
    fn adapter_health_status_is_worst_of_checks() {
        let checks = vec![
            AdapterCheck::new("a", "A", CheckStatus::Ok, None),
            AdapterCheck::new("b", "B", CheckStatus::Fail, Some("boom".into())),
        ];
        let h = AdapterHealth::new("claude-code", "claude", checks, true);
        assert_eq!(h.status, CheckStatus::Fail);
        assert_eq!(h.checks.len(), 2);
    }

    #[test]
    fn check_status_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&CheckStatus::Fail).unwrap(), "\"fail\"");
        assert_eq!(serde_json::to_string(&CheckStatus::Ok).unwrap(), "\"ok\"");
    }

    // 2026-06-12 is a Friday. 16:00Z Fri → 10:00Z Mon.
    fn ms(s: &str) -> i64 {
        chrono::DateTime::parse_from_rfc3339(s).unwrap().timestamp_millis()
    }

    #[test]
    fn business_elapsed_excludes_weekend() {
        let fri = ms("2026-06-12T16:00:00Z");
        let mon = ms("2026-06-15T10:00:00Z");
        // Wall-clock ~66h; business time = 8h (Fri 16→24) + 10h (Mon 0→10) = 18h.
        let h = business_elapsed_hours(fri, mon, true);
        assert!((h - 18.0).abs() < 0.5, "expected ~18 business hours, got {h}");
    }

    #[test]
    fn business_elapsed_full_clock_when_not_excluding() {
        let fri = ms("2026-06-12T16:00:00Z");
        let mon = ms("2026-06-15T10:00:00Z");
        let h = business_elapsed_hours(fri, mon, false);
        assert!((h - 66.0).abs() < 0.5, "expected ~66 wall-clock hours, got {h}");
    }

    #[test]
    fn business_elapsed_all_weekend_span_is_zero() {
        // 2026-06-13 is Saturday, 2026-06-14 is Sunday. A span entirely within
        // the weekend contributes no business time.
        let sat = ms("2026-06-13T12:00:00Z");
        let sun = ms("2026-06-14T12:00:00Z");
        assert_eq!(business_elapsed_hours(sat, sun, true), 0.0);
    }

    #[test]
    fn freshness_none_is_warn_never_captured() {
        let c = capture_freshness(None, ms("2026-06-15T10:00:00Z"), 24.0, true);
        assert_eq!(c.status, CheckStatus::Warn);
        assert!(c.detail.unwrap().contains("never captured"));
    }

    #[test]
    fn freshness_within_window_is_ok() {
        let now = ms("2026-06-15T10:00:00Z");
        let two_h_ago = ms("2026-06-15T08:00:00Z");
        assert_eq!(capture_freshness(Some(two_h_ago), now, 24.0, true).status, CheckStatus::Ok);
    }

    #[test]
    fn freshness_weekend_gap_stays_ok_but_full_clock_fails() {
        let mon = ms("2026-06-15T10:00:00Z");
        let fri = ms("2026-06-12T16:00:00Z");
        // 18 business hours < 24 → Ok; 66 wall-clock hours > 24 → Fail.
        assert_eq!(capture_freshness(Some(fri), mon, 24.0, true).status, CheckStatus::Ok);
        assert_eq!(capture_freshness(Some(fri), mon, 24.0, false).status, CheckStatus::Fail);
    }
}
