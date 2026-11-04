//! One ticket, read live by its key (TG5, D63/D71): the tracker-agnostic read
//! behind MCP `get_ticket`. Nothing read here is stored — description, assignee
//! and acceptance criteria are shown to the assistant and dropped (D71).
// pending: tg5a — the fetch route (targets/http.rs) uses these in phase B; until
// then only the tests do.
#![allow(dead_code)]

use super::{GetTransport, TrackerError, TrackerKind};

/// Where a ticket's acceptance criteria came from, so an empty list says why
/// instead of reading as "this ticket has none".
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceCriteriaSource {
    /// Read from the tracker's configured field.
    Field,
    /// The tracker names no acceptance-criteria field
    /// (`trackers.params.acceptance_criteria_field` is unset).
    NotConfigured,
    /// The field is configured, and this ticket has no value in it.
    FieldAbsent,
}

/// A ticket as the assistant is handed it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TicketDetail {
    pub key: String,
    pub title: String,
    pub description: Option<String>,
    pub acceptance_criteria: Vec<String>,
    pub acceptance_criteria_source: AcceptanceCriteriaSource,
    pub status: Option<String>,
    pub issue_type: Option<String>,
    pub sprint: Option<String>,
    pub assignee: Option<String>,
    pub tracker: TrackerKind,
    pub url: String,
}

/// A live, read-only ticket lookup with the person's own access (D71).
#[async_trait::async_trait]
pub trait TicketReader: Send + Sync {
    /// `Ok(None)`: the tracker has no such ticket. A failed read is an `Err`,
    /// never `Ok(None)` or a placeholder ticket.
    async fn read_ticket(&self, key: &str) -> Result<Option<TicketDetail>, TrackerError>;
}

/// Jira: one issue through the board's own transport and credential.
pub struct JiraTicketReader<'a> {
    pub transport: &'a dyn GetTransport,
    /// The site, e.g. `https://acme.atlassian.net` (the ticket's URL is built on it).
    pub base_url: String,
    /// `trackers.params.acceptance_criteria_field`, e.g. `customfield_10100`.
    pub acceptance_criteria_field: Option<String>,
}

#[async_trait::async_trait]
impl TicketReader for JiraTicketReader<'_> {
    /// pending: tg5a
    async fn read_ticket(&self, _key: &str) -> Result<Option<TicketDetail>, TrackerError> {
        Err(TrackerError::UnsupportedTracker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::test_support::FakeBoard;
    use serde_json::json;

    const ISSUE: &str = "/rest/agile/1.0/issue/AUTH-7";

    fn issue(fields: serde_json::Value) -> serde_json::Value {
        json!({"id": "10007", "key": "AUTH-7", "fields": fields})
    }

    fn full_fields() -> serde_json::Value {
        json!({
            "summary": "Lock the session token to its device",
            "description": "A stolen token must not work elsewhere.",
            "status": {"name": "In Progress"},
            "issuetype": {"name": "Story"},
            "sprint": {"id": 31, "name": "Sprint 31"},
            "assignee": {"displayName": "Ada Lovelace"},
            "customfield_10100": "- the token carries the device id\n* a second device is refused\n\n  ",
        })
    }

    fn reader<'a>(board: &'a FakeBoard, ac_field: Option<&str>) -> JiraTicketReader<'a> {
        JiraTicketReader {
            transport: board,
            base_url: "https://acme.atlassian.net".into(),
            acceptance_criteria_field: ac_field.map(String::from),
        }
    }

    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn reads_one_ticket_by_key() {
        let board = FakeBoard::standard().with(ISSUE, 0, issue(full_fields()));
        let t = reader(&board, Some("customfield_10100")).read_ticket("AUTH-7").await.unwrap();
        assert_eq!(
            t,
            Some(TicketDetail {
                key: "AUTH-7".into(),
                title: "Lock the session token to its device".into(),
                description: Some("A stolen token must not work elsewhere.".into()),
                acceptance_criteria: vec![
                    "the token carries the device id".into(),
                    "a second device is refused".into(),
                ],
                acceptance_criteria_source: AcceptanceCriteriaSource::Field,
                status: Some("In Progress".into()),
                issue_type: Some("Story".into()),
                sprint: Some("Sprint 31".into()),
                assignee: Some("Ada Lovelace".into()),
                tracker: TrackerKind::Jira,
                url: "https://acme.atlassian.net/browse/AUTH-7".into(),
            })
        );
        assert_eq!(board.calls(), vec![ISSUE.to_string()], "one live read, of that issue");
    }

    /// The read asks for the fields it maps — including the acceptance-criteria
    /// field when one is configured — and nothing it does not use.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn the_read_asks_for_the_fields_it_maps() {
        let board = FakeBoard::standard().with(ISSUE, 0, issue(full_fields()));
        reader(&board, Some("customfield_10100")).read_ticket("AUTH-7").await.unwrap();
        let q = board.query_of(ISSUE).expect("asked");
        let fields = q.iter().find(|(k, _)| k == "fields").map(|(_, v)| v.clone()).unwrap();
        let mut asked: Vec<&str> = fields.split(',').collect();
        asked.sort();
        assert_eq!(
            asked,
            [
                "assignee",
                "customfield_10100",
                "description",
                "issuetype",
                "sprint",
                "status",
                "summary"
            ]
        );
    }

    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_ticket_the_tracker_does_not_have_is_none() {
        let board = FakeBoard::standard();
        assert_eq!(reader(&board, None).read_ticket("AUTH-404").await.unwrap(), None);
    }

    /// A failed read is the caller's error: never `None` (which reads as "no such
    /// ticket") and never a placeholder ticket.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_failed_read_is_an_error_not_a_missing_ticket() {
        for e in [TrackerError::Unauthorized, TrackerError::RateLimited, TrackerError::Transport] {
            let board = FakeBoard::standard().fail(ISSUE, e);
            assert_eq!(reader(&board, None).read_ticket("AUTH-7").await, Err(e), "{e:?}");
        }
    }

    /// Acceptance criteria are never invented: with no field configured the list
    /// is empty and says so; with the field configured but empty on this ticket
    /// the list is empty and says that instead.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn acceptance_criteria_say_why_they_are_empty() {
        let board = FakeBoard::standard().with(ISSUE, 0, issue(full_fields()));
        let t = reader(&board, None).read_ticket("AUTH-7").await.unwrap().unwrap();
        assert_eq!(
            (t.acceptance_criteria, t.acceptance_criteria_source),
            (vec![], AcceptanceCriteriaSource::NotConfigured)
        );

        let mut fields = full_fields();
        fields["customfield_10100"] = serde_json::Value::Null;
        let board = FakeBoard::standard().with(ISSUE, 0, issue(fields));
        let t =
            reader(&board, Some("customfield_10100")).read_ticket("AUTH-7").await.unwrap().unwrap();
        assert_eq!(
            (t.acceptance_criteria, t.acceptance_criteria_source),
            (vec![], AcceptanceCriteriaSource::FieldAbsent)
        );
    }

    /// Fields a ticket does not have are absent, never a stand-in value.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_sparse_ticket_reads_as_sparse() {
        let board = FakeBoard::standard().with(ISSUE, 0, issue(json!({"summary": "Bare"})));
        let t = reader(&board, None).read_ticket("AUTH-7").await.unwrap().unwrap();
        assert_eq!(
            (t.title.as_str(), t.description, t.status, t.issue_type, t.sprint, t.assignee),
            ("Bare", None, None, None, None, None)
        );
    }
}
