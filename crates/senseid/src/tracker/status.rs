//! A tracker connection's status, derived from its row. (Spec R9.)

/// What the UI and CLI show for a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerStatus {
    Pending,
    Connected,
    Stale,
    Error,
}

impl TrackerStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TrackerStatus::Pending => "pending",
            TrackerStatus::Connected => "connected",
            TrackerStatus::Stale => "stale",
            TrackerStatus::Error => "error",
        }
    }
}

/// In order: `error` if a failure is recorded; `pending` if it has never
/// succeeded; `stale` if the last success is more than `2 × interval_secs` old;
/// otherwise `connected`.
pub fn status(
    last_success_at: Option<chrono::DateTime<chrono::Utc>>,
    has_error: bool,
    interval_secs: i64,
    now: chrono::DateTime<chrono::Utc>,
) -> TrackerStatus {
    if has_error {
        return TrackerStatus::Error;
    }
    match last_success_at {
        None => TrackerStatus::Pending,
        Some(at) if now - at > chrono::Duration::seconds(2 * interval_secs) => TrackerStatus::Stale,
        Some(_) => TrackerStatus::Connected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-10-16T10:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }
    const INTERVAL: i64 = 1800;

    #[test]
    fn never_synced_is_pending() {
        assert_eq!(status(None, false, INTERVAL, now()), TrackerStatus::Pending);
    }

    #[test]
    fn a_failed_first_sync_is_error_not_pending() {
        assert_eq!(status(None, true, INTERVAL, now()), TrackerStatus::Error);
    }

    #[test]
    fn an_error_wins_over_an_old_success() {
        assert_eq!(
            status(Some(now() - Duration::seconds(10)), true, INTERVAL, now()),
            TrackerStatus::Error
        );
    }

    #[test]
    fn stale_boundary_is_strictly_greater_than_twice_the_interval() {
        let exactly = now() - Duration::seconds(2 * INTERVAL);
        assert_eq!(status(Some(exactly), false, INTERVAL, now()), TrackerStatus::Connected);
        let past = now() - Duration::seconds(2 * INTERVAL + 1);
        assert_eq!(status(Some(past), false, INTERVAL, now()), TrackerStatus::Stale);
        assert_eq!(status(Some(now()), false, INTERVAL, now()), TrackerStatus::Connected);
    }
}
