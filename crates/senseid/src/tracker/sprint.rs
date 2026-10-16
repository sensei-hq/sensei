//! Which sprint a ticket is "in now", when several list it. (Spec R5 step 4.)

/// A sprint's lifecycle state, in the common vocabulary every adapter maps onto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SprintState {
    Future,
    Active,
    Closed,
}

/// One sprint that lists the ticket, as the ranking needs it.
#[derive(Debug, Clone, PartialEq)]
pub struct SprintRank {
    pub id: uuid::Uuid,
    pub state: SprintState,
    pub start_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Adapter-supplied creation order (Jira: the numeric sprint id).
    pub seq: i64,
    pub removed: bool,
}

/// The ticket's current sprint: among non-removed sprints, `active` > `future` >
/// `closed`; ties go to the later `start_at` (NULL last), then the higher `seq`.
pub fn pick_current_sprint(listing: &[SprintRank]) -> Option<uuid::Uuid> {
    fn state_rank(state: SprintState) -> u8 {
        match state {
            SprintState::Active => 2,
            SprintState::Future => 1,
            SprintState::Closed => 0,
        }
    }
    listing
        .iter()
        .filter(|s| !s.removed)
        // `Option` orders `None` below `Some`, which is "NULL start sorts last".
        .max_by(|a, b| {
            state_rank(a.state)
                .cmp(&state_rank(b.state))
                .then(a.start_at.cmp(&b.start_at))
                .then(a.seq.cmp(&b.seq))
        })
        .map(|s| s.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(id: u128, state: SprintState, start: Option<&str>, seq: i64, removed: bool) -> SprintRank {
        SprintRank {
            id: uuid::Uuid::from_u128(id),
            state,
            start_at: start.map(|t| {
                chrono::DateTime::parse_from_rfc3339(t).unwrap().with_timezone(&chrono::Utc)
            }),
            seq,
            removed,
        }
    }
    use SprintState::*;

    #[test]
    fn no_sprints_means_backlog() {
        assert_eq!(pick_current_sprint(&[]), None);
    }

    #[test]
    fn active_beats_future_beats_closed() {
        let closed = s(1, Closed, Some("2026-10-01T00:00:00Z"), 1, false);
        let future = s(2, Future, None, 3, false);
        let active = s(3, Active, Some("2026-10-10T00:00:00Z"), 2, false);
        assert_eq!(
            pick_current_sprint(&[closed.clone(), future.clone(), active]),
            Some(uuid::Uuid::from_u128(3))
        );
        assert_eq!(pick_current_sprint(&[closed, future]), Some(uuid::Uuid::from_u128(2)));
    }

    #[test]
    fn ties_go_to_later_start_then_higher_seq_and_null_start_sorts_last() {
        let earlier = s(1, Closed, Some("2026-09-01T00:00:00Z"), 9, false);
        let later = s(2, Closed, Some("2026-09-15T00:00:00Z"), 1, false);
        assert_eq!(pick_current_sprint(&[earlier, later.clone()]), Some(uuid::Uuid::from_u128(2)));
        let no_start = s(3, Closed, None, 99, false);
        assert_eq!(
            pick_current_sprint(&[no_start, later]),
            Some(uuid::Uuid::from_u128(2)),
            "NULL start sorts last"
        );
        let a = s(4, Future, None, 10, false);
        let b = s(5, Future, None, 11, false);
        assert_eq!(
            pick_current_sprint(&[a, b]),
            Some(uuid::Uuid::from_u128(5)),
            "equal start → higher seq"
        );
    }

    #[test]
    fn removed_sprints_never_count() {
        let closed = s(1, Closed, Some("2026-09-01T00:00:00Z"), 1, false);
        let removed_future = s(2, Future, None, 2, true);
        assert_eq!(
            pick_current_sprint(&[closed, removed_future.clone()]),
            Some(uuid::Uuid::from_u128(1))
        );
        assert_eq!(pick_current_sprint(&[removed_future]), None);
    }
}
