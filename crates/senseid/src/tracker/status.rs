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

/// A board's write-back state (spec W8), first match wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritebackState {
    Disabled,
    PassFailed,
    OwnerNotAPerson,
    OwnerLookupFailed,
    OwnerUnknown,
    NothingAssigned,
    Active,
}

impl WritebackState {
    pub fn as_str(self) -> &'static str {
        match self {
            WritebackState::Disabled => "disabled",
            WritebackState::PassFailed => "pass_failed",
            WritebackState::OwnerNotAPerson => "owner_not_a_person",
            WritebackState::OwnerLookupFailed => "owner_lookup_failed",
            WritebackState::OwnerUnknown => "owner_unknown",
            WritebackState::NothingAssigned => "nothing_assigned",
            WritebackState::Active => "active",
        }
    }
}

/// What a board's write-back state is made of (W8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WritebackFacts<'a> {
    pub enabled: bool,
    pub writeback_error: Option<&'a str>,
    pub owner_known: bool,
    pub owner_state: Option<&'a str>,
    pub owner_error: Option<&'a str>,
    pub assigned: i64,
}

/// The state, and the error code it carries (`pass_failed`, `owner_lookup_failed`).
pub fn writeback_state<'a>(f: &WritebackFacts<'a>) -> (WritebackState, Option<&'a str>) {
    use WritebackState as W;
    if !f.enabled {
        (W::Disabled, None)
    } else if let Some(code) = f.writeback_error {
        (W::PassFailed, Some(code))
    } else if f.owner_state == Some(crate::tracker::TrackerOwnerState::NotAPerson.as_str()) {
        (W::OwnerNotAPerson, None)
    } else if !f.owner_known && f.owner_error.is_some() {
        (W::OwnerLookupFailed, f.owner_error)
    } else if !f.owner_known {
        (W::OwnerUnknown, None)
    } else if f.assigned == 0 {
        (W::NothingAssigned, None)
    } else {
        (W::Active, None)
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
    fn the_writeback_state_is_the_first_that_applies() {
        use WritebackState as W;
        let active =
            WritebackFacts { enabled: true, owner_known: true, assigned: 2, ..Default::default() };
        let case = |f: WritebackFacts<'static>| writeback_state(&f);
        assert_eq!(case(active), (W::Active, None));
        assert_eq!(case(WritebackFacts { assigned: 0, ..active }), (W::NothingAssigned, None));
        let unowned = WritebackFacts { owner_known: false, ..active };
        assert_eq!(case(unowned), (W::OwnerUnknown, None));
        assert_eq!(
            case(WritebackFacts { owner_error: Some("forbidden"), ..unowned }),
            (W::OwnerLookupFailed, Some("forbidden"))
        );
        assert_eq!(
            case(WritebackFacts {
                owner_state: Some("not_a_person"),
                owner_error: Some("forbidden"),
                ..unowned
            }),
            (W::OwnerNotAPerson, None),
            "an app token outranks a lookup error"
        );
        assert_eq!(
            case(WritebackFacts {
                writeback_error: Some("db"),
                owner_state: Some("not_a_person"),
                ..unowned
            }),
            (W::PassFailed, Some("db"))
        );
        assert_eq!(
            case(WritebackFacts { enabled: false, writeback_error: Some("db"), ..active }),
            (W::Disabled, None),
            "off outranks everything"
        );
    }

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
