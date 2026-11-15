//! One ticket, read live by its key (TG5, D63/D71): the tracker-agnostic read
//! behind MCP `get_ticket`. Nothing read here is stored — description, assignee
//! and acceptance criteria are shown to the assistant and dropped (D71).

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

/// The fields the read maps, besides the configured acceptance-criteria field.
const JIRA_FIELDS: [&str; 6] =
    ["summary", "description", "status", "issuetype", "sprint", "assignee"];

#[async_trait::async_trait]
impl TicketReader for JiraTicketReader<'_> {
    async fn read_ticket(&self, key: &str) -> Result<Option<TicketDetail>, TrackerError> {
        let mut fields: Vec<&str> = JIRA_FIELDS.to_vec();
        if let Some(f) = self.acceptance_criteria_field.as_deref() {
            fields.push(f);
        }
        let path = format!("/rest/agile/1.0/issue/{key}");
        let issue = match self.transport.get_json(&path, &[("fields", fields.join(","))]).await {
            Ok(v) => v,
            Err(TrackerError::NotFound) => return Ok(None),
            Err(e) => return Err(e),
        };
        let f = &issue["fields"];
        let text = |v: &serde_json::Value| v.as_str().map(str::to_string);
        let named =
            |v: &serde_json::Value, k: &str| v.get(k).and_then(|n| n.as_str()).map(str::to_string);
        let key = issue["key"].as_str().unwrap_or(key).to_string();
        let title = text(&f["summary"]).ok_or(TrackerError::Decode)?;
        let (acceptance_criteria, acceptance_criteria_source) =
            match self.acceptance_criteria_field.as_deref() {
                None => (Vec::new(), AcceptanceCriteriaSource::NotConfigured),
                Some(id) => match criteria_lines(&f[id]) {
                    lines if lines.is_empty() => (lines, AcceptanceCriteriaSource::FieldAbsent),
                    lines => (lines, AcceptanceCriteriaSource::Field),
                },
            };
        Ok(Some(TicketDetail {
            url: format!("{}/browse/{key}", self.base_url.trim_end_matches('/')),
            key,
            title,
            description: text(&f["description"]).filter(|d| !d.trim().is_empty()),
            acceptance_criteria,
            acceptance_criteria_source,
            status: named(&f["status"], "name"),
            issue_type: named(&f["issuetype"], "name"),
            sprint: named(&f["sprint"], "name"),
            assignee: named(&f["assignee"], "displayName"),
            tracker: TrackerKind::Jira,
        }))
    }
}

/// The criteria in a text field, one per non-empty line, with a leading list
/// bullet (`-`, `*`, `•`) dropped. A list field is read item by item. Anything
/// else (absent, null, a shape this does not know) is no criteria — the caller
/// says `field_absent`, never invents a list.
fn criteria_lines(v: &serde_json::Value) -> Vec<String> {
    let clean = |l: &str| {
        let l = l.trim();
        l.strip_prefix(['-', '*', '•']).map(str::trim).unwrap_or(l).to_string()
    };
    let items: Vec<String> = match v {
        serde_json::Value::String(s) => s.lines().map(clean).collect(),
        serde_json::Value::Array(a) => a.iter().filter_map(|i| i.as_str()).map(clean).collect(),
        _ => Vec::new(),
    };
    items.into_iter().filter(|l| !l.is_empty()).collect()
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
    async fn a_ticket_the_tracker_does_not_have_is_none() {
        let board = FakeBoard::standard();
        assert_eq!(reader(&board, None).read_ticket("AUTH-404").await.unwrap(), None);
    }

    /// A failed read is the caller's error: never `None` (which reads as "no such
    /// ticket") and never a placeholder ticket.
    #[tokio::test]
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
    async fn a_sparse_ticket_reads_as_sparse() {
        let board = FakeBoard::standard().with(ISSUE, 0, issue(json!({"summary": "Bare"})));
        let t = reader(&board, None).read_ticket("AUTH-7").await.unwrap().unwrap();
        assert_eq!(
            (t.title.as_str(), t.description, t.status, t.issue_type, t.sprint, t.assignee),
            ("Bare", None, None, None, None, None)
        );
    }
}
