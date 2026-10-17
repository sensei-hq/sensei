//! The Jira Cloud adapter. All Jira knowledge — host rule, board-URL shapes,
//! endpoints, auth, ticket-key matching — lives in this module. (Spec R4, R6.)

use crate::tracker::http::{InvalidBaseUrl, normalize_base_url};
use crate::tracker::sprint::SprintState;
use crate::tracker::{
    ApiToken, AuthScheme, BoardScope, ConnectParams, GetTransport, KnownTask, Link, LinkSource,
    ParamError, PrevState, SessionText, Snapshot, SprintRow, TaskRow, TrackerAdapter, TrackerError,
    TrackerKind, ValidateError, Validated,
};

/// Jira Cloud's host rule: `<label>.atlassian.net`, at least one label before it.
pub fn is_jira_cloud_host(host: &str) -> bool {
    host.strip_suffix(".atlassian.net")
        .is_some_and(|labels| !labels.is_empty() && labels.split('.').all(|l| !l.is_empty()))
}

/// A board id in canonical decimal form (`"042"` → `"42"`); `None` for zero or
/// anything that is not a `u64`.
pub fn canonical_board_id(raw: &str) -> Option<String> {
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u64>().ok().filter(|id| *id > 0).map(|id| id.to_string())
}

/// Split a pasted board address into `(base_url, board_id)`.
///
/// Accepts `…/boards/<id>[/anything]` and `…/RapidBoard.jspa?…rapidView=<id>…`.
/// The board id is read before the query is dropped; the origin goes through the
/// shared [`normalize_base_url`] with the Jira host rule.
pub fn parse_board_url(raw: &str) -> Result<(String, String), InvalidBaseUrl> {
    let url = reqwest::Url::parse(raw.trim()).map_err(|_| InvalidBaseUrl)?;
    let board_id = board_id_in(&url).ok_or(InvalidBaseUrl)?;
    let base_url = normalize_base_url(raw, is_jira_cloud_host)?;
    Ok((base_url, board_id))
}

/// The board id a Jira board address carries: the segment after `/boards/`, or
/// the classic `RapidBoard.jspa?rapidView=<id>` parameter.
fn board_id_in(url: &reqwest::Url) -> Option<String> {
    let segments: Vec<&str> = url.path_segments()?.collect();
    if let Some(i) = segments.iter().position(|s| *s == "boards") {
        return segments.get(i + 1).and_then(|s| canonical_board_id(s));
    }
    if segments.last() == Some(&"RapidBoard.jspa") {
        return url
            .query_pairs()
            .find(|(k, _)| k == "rapidView")
            .and_then(|(_, v)| canonical_board_id(&v));
    }
    None
}

/// The Jira Cloud adapter (Agile REST 1.0 + `/rest/api/3/myself`).
pub struct JiraAdapter;

/// Page size for issue listings.
pub const ISSUE_PAGE: u32 = 100;
/// Page size for sprint listings.
pub const SPRINT_PAGE: u32 = 50;

#[async_trait::async_trait]
impl TrackerAdapter for JiraAdapter {
    fn kind(&self) -> TrackerKind {
        TrackerKind::Jira
    }

    fn connect_params(&self, body: &serde_json::Value) -> Result<ConnectParams, ParamError> {
        let text =
            |k: &str| body.get(k).and_then(|v| v.as_str()).map(str::trim).filter(|v| !v.is_empty());
        let board_id_field = body.get("board_id").and_then(|v| match v {
            serde_json::Value::Number(n) => Some(n.to_string()),
            serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
            _ => None,
        });
        let separate = text("base_url").is_some() || board_id_field.is_some();
        let (base_url, board_ref) = match (text("board_url"), separate) {
            (Some(url), false) => parse_board_url(url).map_err(|_| ParamError::InvalidBaseUrl)?,
            (None, true) => {
                let base = text("base_url").ok_or(ParamError::InvalidBaseUrl)?;
                let base = normalize_base_url(base, is_jira_cloud_host)
                    .map_err(|_| ParamError::InvalidBaseUrl)?;
                let board = board_id_field
                    .as_deref()
                    .and_then(canonical_board_id)
                    .ok_or(ParamError::InvalidBaseUrl)?;
                (base, board)
            }
            // Both forms, or neither: refuse rather than guess which was meant.
            _ => return Err(ParamError::InvalidBaseUrl),
        };
        let account = text("email").ok_or(ParamError::EmailRequired)?.to_string();
        let token = text("api_token").ok_or(ParamError::TokenRequired)?.to_string();
        Ok(ConnectParams {
            base_url,
            board_ref,
            account,
            token: ApiToken(token),
            project_key: text("project_key").map(str::to_uppercase),
        })
    }

    fn auth(&self, account: &str, token: &ApiToken) -> AuthScheme {
        AuthScheme::Basic { user: account.to_string(), token: token.clone() }
    }

    async fn validate(
        &self,
        t: &dyn GetTransport,
        p: &ConnectParams,
    ) -> Result<Validated, ValidateError> {
        t.get_json("/rest/api/3/myself", &[]).await.map_err(ValidateError::Tracker)?;
        let board = match t.get_json(&board_path(&p.board_ref), &[]).await {
            Ok(b) => b,
            Err(TrackerError::NotFound) => return Err(ValidateError::BoardNotFound),
            Err(e) => return Err(ValidateError::Tracker(e)),
        };
        let board_name = board
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(ValidateError::Tracker(TrackerError::Decode))?;
        let board_project = board.pointer("/location/projectKey").and_then(|v| v.as_str());
        let mismatch = p
            .project_key
            .as_deref()
            .is_some_and(|want| board_project.map(str::to_uppercase).as_deref() != Some(want));
        if mismatch {
            return Err(ValidateError::BoardProjectMismatch);
        }
        Ok(Validated {
            // A board spanning several projects has no single key; its name is the scope shown.
            external_scope: board_project.unwrap_or(board_name).to_string(),
            board_name: board_name.to_string(),
        })
    }

    async fn fetch_snapshot(
        &self,
        t: &dyn GetTransport,
        board: &BoardScope,
        prev: &PrevState,
    ) -> Result<Snapshot, TrackerError> {
        let base = board_path(&board.board_ref);
        let board_name =
            t.get_json(&base, &[]).await?.get("name").and_then(|v| v.as_str()).map(String::from);

        let sprints: Vec<SprintRow> =
            paged(t, &format!("{base}/sprint"), "values", SPRINT_PAGE, &[])
                .await?
                .iter()
                .map(sprint_row)
                .collect::<Result<_, _>>()?;

        let tasks: Vec<TaskRow> =
            paged(t, &format!("{base}/issue"), "issues", ISSUE_PAGE, &ISSUE_FIELDS)
                .await?
                .iter()
                .map(task_row)
                .collect::<Result<_, _>>()?;

        let mut members = Vec::with_capacity(sprints.len());
        for sprint in &sprints {
            let fetched_closed = prev.members_fetched_state.get(&sprint.external_id)
                == Some(&Some(SprintState::Closed));
            if sprint.state == SprintState::Closed && fetched_closed {
                members.push((sprint.external_id.clone(), None));
                continue;
            }
            let path = format!("{base}/sprint/{}/issue", sprint.external_id);
            let ids = paged(t, &path, "issues", ISSUE_PAGE, &[("fields", "summary")])
                .await?
                .iter()
                .map(issue_id)
                .collect::<Result<Vec<_>, _>>()?;
            members.push((sprint.external_id.clone(), Some(ids)));
        }
        Ok(Snapshot { board_name, sprints, tasks, members })
    }

    fn match_links(&self, session: &SessionText, known: &[KnownTask]) -> Vec<Link> {
        match_keys(session, known)
    }
}

/// Jira's `match_links` (spec R6): keys of `known` found in the session's branches
/// and prompts, matched on the original bytes, ASCII case-insensitively.
pub fn match_keys(session: &SessionText, known: &[KnownTask]) -> Vec<Link> {
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    let mut by_key: HashMap<&str, Vec<&KnownTask>> = HashMap::new();
    for k in known {
        by_key.entry(k.key.as_str()).or_default().push(k);
    }
    let mut prefixes: Vec<String> = known
        .iter()
        .filter_map(|k| k.key.rsplit_once('-').map(|(p, _)| p.to_ascii_uppercase()))
        .filter(|p| !p.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    prefixes.sort_by_key(|p| std::cmp::Reverse(p.len()));
    // The (task, alias) pairs a text carries, one per occurrence.
    let tasks_in = |text: &str| -> Vec<(&str, String)> {
        keys_in(text.as_bytes(), &prefixes)
            .into_iter()
            .flat_map(|key| {
                by_key
                    .get(key.as_str())
                    .into_iter()
                    .flatten()
                    .map(move |t| (t.external_id.as_str(), key.clone()))
            })
            .collect()
    };

    // (task, source) → alias → mentions.
    let mut tally: BTreeMap<(&str, LinkSource), BTreeMap<String, u32>> = BTreeMap::new();
    let mut branch_hits: HashMap<&str, u32> = HashMap::new();
    let branches: BTreeSet<&str> = session.branches.iter().map(String::as_str).collect();
    for branch in branches {
        // A branch counts once per task, and once per alias it carries.
        let found: BTreeSet<(&str, String)> = tasks_in(branch).into_iter().collect();
        let tasks: BTreeSet<&str> = found.iter().map(|(t, _)| *t).collect();
        for t in tasks {
            *branch_hits.entry(t).or_default() += 1;
        }
        for (t, k) in found {
            *tally.entry((t, LinkSource::Branch)).or_default().entry(k).or_default() += 1;
        }
    }
    for prompt in &session.prompts {
        for (t, k) in tasks_in(prompt) {
            *tally.entry((t, LinkSource::Prompt)).or_default().entry(k).or_default() += 1;
        }
    }
    let current: HashMap<&str, &str> = known
        .iter()
        .filter(|k| k.current)
        .map(|k| (k.external_id.as_str(), k.key.as_str()))
        .collect();

    tally
        .into_iter()
        .map(|((task, source), aliases)| {
            let cur = current.get(task).copied();
            let (matched_key, _) = aliases
                .iter()
                .max_by(|(ka, na), (kb, nb)| {
                    na.cmp(nb)
                        .then_with(|| (Some(ka.as_str()) == cur).cmp(&(Some(kb.as_str()) == cur)))
                        .then_with(|| kb.cmp(ka))
                })
                .expect("a tallied task has an alias");
            let mentions = match source {
                LinkSource::Branch => branch_hits.get(task).copied().unwrap_or(0),
                LinkSource::Prompt => aliases.values().sum(),
            };
            Link {
                session_id: session.session_id,
                task_external_id: task.to_string(),
                source,
                matched_key: matched_key.clone(),
                mentions,
            }
        })
        .collect()
}

/// Every key shape `PREFIX-digits` in `text`, scanned left to right with prefixes
/// longest first; a key must not follow an ASCII letter or digit, and a matched
/// span is consumed. Comparison is ASCII-only, on the original bytes.
fn keys_in(text: &[u8], prefixes: &[String]) -> Vec<String> {
    let mut keys = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if i > 0 && text[i - 1].is_ascii_alphanumeric() {
            i += 1;
            continue;
        }
        let hit = prefixes.iter().find_map(|p| {
            let p_end = i + p.len();
            let rest = text.get(i..p_end)?;
            if !rest.eq_ignore_ascii_case(p.as_bytes()) || text.get(p_end) != Some(&b'-') {
                return None;
            }
            let digits_start = p_end + 1;
            let digits = text[digits_start..].iter().take_while(|b| b.is_ascii_digit()).count();
            (digits > 0).then(|| {
                let end = digits_start + digits;
                (format!("{p}-{}", String::from_utf8_lossy(&text[digits_start..end])), end)
            })
        });
        match hit {
            Some((key, end)) => {
                keys.push(key);
                i = end;
            }
            None => i += 1,
        }
    }
    keys
}

const ISSUE_FIELDS: [(&str, &str); 1] = [("fields", "summary,issuetype,status")];
/// A listing that never signals its end is a provider fault, not a reason to loop.
const MAX_PAGES: usize = 1000;

fn board_path(board_ref: &str) -> String {
    format!("/rest/agile/1.0/board/{board_ref}")
}

/// Every item of a `startAt`-paged listing. Stops on `isLast`, on
/// `startAt >= total`, or on an empty page.
async fn paged(
    t: &dyn GetTransport,
    path: &str,
    items_key: &str,
    page_size: u32,
    extra: &[(&str, &str)],
) -> Result<Vec<serde_json::Value>, TrackerError> {
    let mut out = Vec::new();
    let mut start: u64 = 0;
    for _ in 0..MAX_PAGES {
        let mut query: Vec<(&str, String)> =
            vec![("startAt", start.to_string()), ("maxResults", page_size.to_string())];
        query.extend(extra.iter().map(|(k, v)| (*k, v.to_string())));
        let page = t.get_json(path, &query).await?;
        let items = page.get(items_key).and_then(|v| v.as_array()).ok_or(TrackerError::Decode)?;
        if items.is_empty() {
            return Ok(out);
        }
        start += items.len() as u64;
        out.extend(items.iter().cloned());
        let last = page.get("isLast").and_then(|v| v.as_bool()) == Some(true);
        let past_total =
            page.get("total").and_then(|v| v.as_u64()).is_some_and(|total| start >= total);
        if last || past_total {
            return Ok(out);
        }
    }
    Err(TrackerError::Decode)
}

fn timestamp(
    v: &serde_json::Value,
    key: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, TrackerError> {
    match v.get(key).and_then(|x| x.as_str()) {
        None => Ok(None),
        Some(s) => chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| Some(d.with_timezone(&chrono::Utc)))
            .map_err(|_| TrackerError::Decode),
    }
}

fn sprint_row(v: &serde_json::Value) -> Result<SprintRow, TrackerError> {
    let id = v.get("id").and_then(|x| x.as_i64()).ok_or(TrackerError::Decode)?;
    let state = v
        .get("state")
        .and_then(|x| x.as_str())
        .and_then(SprintState::parse)
        .ok_or(TrackerError::Decode)?;
    Ok(SprintRow {
        external_id: id.to_string(),
        seq: id,
        name: v.get("name").and_then(|x| x.as_str()).ok_or(TrackerError::Decode)?.to_string(),
        state,
        start_at: timestamp(v, "startDate")?,
        end_at: timestamp(v, "endDate")?,
        complete_at: timestamp(v, "completeDate")?,
    })
}

fn issue_id(v: &serde_json::Value) -> Result<String, TrackerError> {
    v.get("id").and_then(|x| x.as_str()).map(String::from).ok_or(TrackerError::Decode)
}

fn task_row(v: &serde_json::Value) -> Result<TaskRow, TrackerError> {
    let field = |path: &str| v.pointer(path).and_then(|x| x.as_str()).map(String::from);
    Ok(TaskRow {
        external_id: issue_id(v)?,
        key: v.get("key").and_then(|x| x.as_str()).ok_or(TrackerError::Decode)?.to_uppercase(),
        scope: None,
        summary: field("/fields/summary"),
        issue_type: field("/fields/issuetype/name"),
        status: field("/fields/status/name"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jira_cloud_hosts() {
        assert!(is_jira_cloud_host("acme.atlassian.net"));
        assert!(is_jira_cloud_host("my-team.atlassian.net"));
        for h in [
            "atlassian.net",
            ".atlassian.net",
            "acme.atlassian.net.evil.com",
            "acme.atlassian.net.",
            "evilatlassian.net",
            "acme.atlassian.com",
        ] {
            assert!(!is_jira_cloud_host(h), "{h}");
        }
    }

    #[test]
    fn board_ids_are_canonical_decimal() {
        assert_eq!(canonical_board_id("42"), Some("42".into()));
        assert_eq!(canonical_board_id("042"), Some("42".into()));
        for bad in ["0", "000", "-1", "4 2", "", "x", "18446744073709551616"] {
            assert_eq!(canonical_board_id(bad), None, "{bad:?}");
        }
    }

    const SITE: &str = "https://acme.atlassian.net";

    #[test]
    fn modern_board_urls() {
        for (raw, id) in [
            ("https://acme.atlassian.net/jira/software/projects/AUTH/boards/42", "42"),
            ("https://acme.atlassian.net/jira/software/projects/AUTH/boards/042", "42"),
            (
                "https://acme.atlassian.net/jira/software/projects/AUTH/boards/42/backlog?selectedIssue=AUTH-1",
                "42",
            ),
            ("https://ACME.atlassian.net/jira/software/c/projects/AUTH/boards/7#x", "7"),
        ] {
            assert_eq!(parse_board_url(raw), Ok((SITE.into(), id.into())), "{raw}");
        }
    }

    #[test]
    fn classic_rapid_board_urls() {
        for (raw, id) in [
            ("https://acme.atlassian.net/secure/RapidBoard.jspa?rapidView=7", "7"),
            ("https://acme.atlassian.net/secure/RapidBoard.jspa?view=planning&rapidView=7", "7"),
        ] {
            assert_eq!(parse_board_url(raw), Ok((SITE.into(), id.into())), "{raw}");
        }
    }

    #[test]
    fn board_urls_without_a_usable_board_are_rejected() {
        for raw in [
            "https://acme.atlassian.net/jira/software/projects/AUTH",
            "https://acme.atlassian.net/jira/software/projects/AUTH/boards/0",
            "https://acme.atlassian.net/jira/software/projects/AUTH/boards/",
            "https://acme.atlassian.net/secure/RapidBoard.jspa?view=planning",
            "https://evil.com/jira/software/projects/AUTH/boards/42",
            "http://acme.atlassian.net/jira/software/projects/AUTH/boards/42",
        ] {
            assert_eq!(parse_board_url(raw), Err(InvalidBaseUrl), "{raw}");
        }
    }
}

#[cfg(test)]
mod adapter_tests {
    use super::*;
    use crate::tracker::sprint::SprintState;
    use crate::tracker::test_support::FakeBoard;
    use serde_json::json;

    fn params(body: serde_json::Value) -> Result<ConnectParams, ParamError> {
        JiraAdapter.connect_params(&body)
    }

    #[test]
    fn connect_params_from_a_board_url() {
        let p = params(json!({
            "board_url": "https://acme.atlassian.net/jira/software/projects/AUTH/boards/042",
            "email": "dev@example.com", "api_token": "t0k"
        }))
        .unwrap();
        assert_eq!(
            (p.base_url.as_str(), p.board_ref.as_str()),
            ("https://acme.atlassian.net", "42")
        );
        assert_eq!((p.account.as_str(), p.project_key), ("dev@example.com", None));
        assert_eq!(p.token, ApiToken("t0k".into()));
    }

    #[test]
    fn connect_params_from_separate_fields() {
        let p = params(json!({
            "base_url": "https://ACME.atlassian.net/", "board_id": "042",
            "email": "dev@example.com", "api_token": "t0k", "project_key": "auth"
        }))
        .unwrap();
        assert_eq!(
            (p.base_url.as_str(), p.board_ref.as_str()),
            ("https://acme.atlassian.net", "42")
        );
        assert_eq!(p.project_key.as_deref(), Some("AUTH"));
        let numeric = params(json!({
            "base_url": "https://acme.atlassian.net", "board_id": 7, "email": "e@x.io", "api_token": "t"
        }))
        .unwrap();
        assert_eq!(numeric.board_ref, "7");
    }

    #[test]
    fn connect_params_refusals() {
        let url = "https://acme.atlassian.net/jira/software/projects/AUTH/boards/42";
        let both = json!({"board_url": url, "base_url": "https://acme.atlassian.net", "board_id": 42,
                          "email": "e@x.io", "api_token": "t"});
        assert_eq!(params(both), Err(ParamError::InvalidBaseUrl), "both forms present");
        assert_eq!(
            params(json!({"email": "e@x.io", "api_token": "t"})),
            Err(ParamError::InvalidBaseUrl),
            "neither"
        );
        assert_eq!(
            params(
                json!({"base_url": "https://acme.atlassian.net", "board_id": 0, "email": "e@x.io", "api_token": "t"})
            ),
            Err(ParamError::InvalidBaseUrl),
            "board 0"
        );
        assert_eq!(
            params(json!({"board_url": url, "api_token": "t"})),
            Err(ParamError::EmailRequired)
        );
        assert_eq!(
            params(json!({"board_url": url, "email": "  ", "api_token": "t"})),
            Err(ParamError::EmailRequired)
        );
        assert_eq!(
            params(json!({"board_url": url, "email": "e@x.io"})),
            Err(ParamError::TokenRequired)
        );
        assert_eq!(
            params(json!({"board_url": url, "email": "e@x.io", "api_token": ""})),
            Err(ParamError::TokenRequired)
        );
    }

    #[test]
    fn the_token_never_appears_in_debug_output() {
        let p = params(json!({
            "board_url": "https://acme.atlassian.net/jira/software/projects/AUTH/boards/42",
            "email": "dev@example.com", "api_token": "SENTINEL-TOKEN-XYZ"
        }))
        .unwrap();
        let dbg = format!("{p:?} {:?}", JiraAdapter.auth(&p.account, &p.token));
        assert!(!dbg.contains("SENTINEL-TOKEN-XYZ"), "{dbg}");
    }

    fn cp(project_key: Option<&str>) -> ConnectParams {
        ConnectParams {
            base_url: "https://acme.atlassian.net".into(),
            board_ref: "42".into(),
            account: "dev@example.com".into(),
            token: ApiToken("t".into()),
            project_key: project_key.map(String::from),
        }
    }

    #[tokio::test]
    async fn validate_derives_scope_and_name_from_the_board() {
        let board = FakeBoard::standard();
        let v = JiraAdapter.validate(&board, &cp(None)).await.unwrap();
        assert_eq!(v, Validated { external_scope: "AUTH".into(), board_name: "AUTH Scrum".into() });
        assert_eq!(
            JiraAdapter.validate(&board, &cp(Some("AUTH"))).await.unwrap().external_scope,
            "AUTH"
        );
    }

    #[tokio::test]
    async fn validate_refusals() {
        let board = FakeBoard::standard();
        assert_eq!(
            JiraAdapter.validate(&board, &cp(Some("OTHER"))).await,
            Err(ValidateError::BoardProjectMismatch)
        );
        let missing =
            FakeBoard::standard().fail("/rest/agile/1.0/board/42", TrackerError::NotFound);
        assert_eq!(
            JiraAdapter.validate(&missing, &cp(None)).await,
            Err(ValidateError::BoardNotFound)
        );
        let denied = FakeBoard::standard().fail("/rest/api/3/myself", TrackerError::Unauthorized);
        assert_eq!(
            JiraAdapter.validate(&denied, &cp(None)).await,
            Err(ValidateError::Tracker(TrackerError::Unauthorized))
        );
    }

    #[tokio::test]
    async fn a_board_without_a_single_project_uses_its_name() {
        let board = FakeBoard::standard().with(
            "/rest/agile/1.0/board/42",
            0,
            json!({"id": 42, "name": "Platform"}),
        );
        assert_eq!(
            JiraAdapter.validate(&board, &cp(None)).await.unwrap().external_scope,
            "Platform"
        );
    }

    fn scope() -> BoardScope {
        BoardScope { base_url: "https://acme.atlassian.net".into(), board_ref: "42".into() }
    }

    #[tokio::test]
    async fn fetch_snapshot_pages_through_everything() {
        let board = FakeBoard::standard();
        let s = JiraAdapter.fetch_snapshot(&board, &scope(), &PrevState::default()).await.unwrap();
        let keys: Vec<&str> = s.tasks.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(keys, ["AUTH-1", "AUTH-2", "AUTH-3"], "two issue pages, keys upper-cased");
        assert_eq!(s.tasks[0].issue_type.as_deref(), Some("Story"));
        assert_eq!(s.tasks[0].status.as_deref(), Some("To Do"));
        let sprints: Vec<(&str, SprintState, i64)> =
            s.sprints.iter().map(|x| (x.external_id.as_str(), x.state, x.seq)).collect();
        assert_eq!(sprints, [("100", SprintState::Closed, 100), ("101", SprintState::Active, 101)]);
        assert!(s.sprints[0].complete_at.is_some() && s.sprints[1].start_at.is_some());
        let mut members = s.members.clone();
        members.sort();
        assert_eq!(
            members,
            [
                ("100".into(), Some(vec!["10001".into()])),
                ("101".into(), Some(vec!["10002".into(), "10003".into()]))
            ]
        );
    }

    #[tokio::test]
    async fn a_closed_sprint_fetched_after_closing_is_not_fetched_again() {
        let board = FakeBoard::standard();
        let mut prev = PrevState::default();
        prev.members_fetched_state.insert("100".into(), Some(SprintState::Closed));
        prev.members_fetched_state.insert("101".into(), Some(SprintState::Active));
        let s = JiraAdapter.fetch_snapshot(&board, &scope(), &prev).await.unwrap();
        assert!(s.members.contains(&("100".into(), None)), "closed+fetched-closed sprint skipped");
        assert!(!board.was_called("/rest/agile/1.0/board/42/sprint/100/issue"));
        assert!(
            board.was_called("/rest/agile/1.0/board/42/sprint/101/issue"),
            "open sprints always refetched"
        );
    }

    #[tokio::test]
    async fn any_failed_page_fails_the_whole_snapshot() {
        let board = FakeBoard::standard().fail_page(
            "/rest/agile/1.0/board/42/issue",
            2,
            TrackerError::HttpError,
        );
        let r = JiraAdapter.fetch_snapshot(&board, &scope(), &PrevState::default()).await;
        assert_eq!(r, Err(TrackerError::HttpError));
    }

    #[tokio::test]
    async fn an_unknown_sprint_state_is_a_decode_error() {
        let board = FakeBoard::standard().with(
            "/rest/agile/1.0/board/42/sprint",
            0,
            json!({"isLast": true, "values": [{"id": 9, "name": "X", "state": "weird"}]}),
        );
        let r = JiraAdapter.fetch_snapshot(&board, &scope(), &PrevState::default()).await;
        assert_eq!(r, Err(TrackerError::Decode));
    }
}

#[cfg(test)]
mod match_tests {
    use super::*;

    const S: uuid::Uuid = uuid::Uuid::from_u128(7);

    fn known(keys: &[(&str, &str)]) -> Vec<KnownTask> {
        // A task's first listed key is its current one.
        let mut seen = std::collections::HashSet::new();
        keys.iter()
            .map(|(id, key)| KnownTask {
                external_id: id.to_string(),
                key: key.to_string(),
                scope: None,
                current: seen.insert(*id),
            })
            .collect()
    }

    /// AUTH-420 = 1, AUTH-421 = 2, MY_PROJ-12 = 3.
    fn standard() -> Vec<KnownTask> {
        known(&[("1", "AUTH-420"), ("2", "AUTH-421"), ("3", "MY_PROJ-12")])
    }

    fn session(branches: &[&str], prompts: &[&str]) -> SessionText {
        SessionText {
            session_id: S,
            branches: branches.iter().map(|b| b.to_string()).collect(),
            prompts: prompts.iter().map(|p| p.to_string()).collect(),
            repo_key: None,
        }
    }

    /// (task, source, matched_key, mentions), sorted.
    fn found(s: &SessionText, k: &[KnownTask]) -> Vec<(String, LinkSource, String, u32)> {
        let mut v: Vec<_> = match_keys(s, k)
            .into_iter()
            .inspect(|l| assert_eq!(l.session_id, S))
            .map(|l| (l.task_external_id, l.source, l.matched_key, l.mentions))
            .collect();
        v.sort();
        v
    }

    fn prompt_keys(text: &str, k: &[KnownTask]) -> Vec<String> {
        found(&session(&[], &[text]), k).into_iter().map(|(_, _, key, _)| key).collect()
    }

    const NONE: Vec<String> = Vec::new();

    #[test]
    fn only_known_keys_link() {
        let k = standard();
        for text in ["UTF-8", "AES-256", "CVE-2026", "AUTH-999", "auth", ""] {
            assert_eq!(prompt_keys(text, &k), NONE, "{text:?}");
        }
        assert!(found(&session(&["fix/auth-420"], &["AUTH-420"]), &[]).is_empty(), "empty known");
    }

    #[test]
    fn a_branch_links_case_insensitively() {
        assert_eq!(
            found(&session(&["fix/auth-420-store"], &[]), &standard()),
            vec![("1".into(), LinkSource::Branch, "AUTH-420".into(), 1)]
        );
    }

    #[test]
    fn boundaries() {
        let k = standard();
        for text in ["AUTH-420_fix", "feature_AUTH-420", "(AUTH-420)", "AUTH-420.", "x/AUTH-420"] {
            assert_eq!(prompt_keys(text, &k), vec!["AUTH-420".to_string()], "{text:?}");
        }
        for text in ["xAUTH-420", "9AUTH-420", "AUTH-4201", "AUTH-0420", "AUTH-", "AUTH 420"] {
            assert_eq!(prompt_keys(text, &k), NONE, "{text:?}");
        }
    }

    #[test]
    fn underscore_prefixes_and_consumed_spans() {
        let k = standard();
        assert_eq!(prompt_keys("MY_PROJ-12", &k), vec!["MY_PROJ-12".to_string()]);
        let mut with_proj = k.clone();
        with_proj.extend(known(&[("4", "PROJ-12")]));
        assert_eq!(
            prompt_keys("see MY_PROJ-12", &with_proj),
            vec!["MY_PROJ-12".to_string()],
            "the consumed span never also yields PROJ-12"
        );
        assert_eq!(prompt_keys("see PROJ-12", &with_proj), vec!["PROJ-12".to_string()]);
    }

    #[test]
    fn no_unicode_folding_and_no_panics() {
        let k = known(&[("1", "KEY-1"), ("2", "AUTH-420")]);
        assert_eq!(prompt_keys("\u{212A}EY-1", &k), NONE, "the Kelvin sign is not K");
        assert_eq!(prompt_keys("ßİ AUTH-420", &k), vec!["AUTH-420".to_string()]);
        assert_eq!(
            prompt_keys("İAUTH-420ß", &k),
            vec!["AUTH-420".to_string()],
            "non-ASCII separates"
        );
        assert_eq!(prompt_keys("🙂🙂🙂 auth-", &k), NONE);
    }

    #[test]
    fn prompt_mentions_count_occurrences_and_branch_mentions_count_branches() {
        let k = standard();
        let s = session(
            &["fix/AUTH-420", "AUTH-420-retry", "main"],
            &["AUTH-420 then auth-420", "and AUTH-421"],
        );
        assert_eq!(
            found(&s, &k),
            vec![
                ("1".into(), LinkSource::Branch, "AUTH-420".into(), 2),
                ("1".into(), LinkSource::Prompt, "AUTH-420".into(), 2),
                ("2".into(), LinkSource::Prompt, "AUTH-421".into(), 1),
            ]
        );
        let twice_in_one_branch = session(&["AUTH-420-AUTH-420"], &[]);
        assert_eq!(found(&twice_in_one_branch, &k)[0].3, 1, "a branch counts once");
    }

    #[test]
    fn aliases_of_one_task_merge_into_one_link() {
        // Task 1 was AUTH-420 and is now IAM-5 (the current key is listed first).
        let k = known(&[("1", "IAM-5"), ("1", "AUTH-420")]);
        assert_eq!(
            found(&session(&["fix/auth-420"], &[]), &k),
            vec![("1".into(), LinkSource::Branch, "AUTH-420".into(), 1)],
            "the old key still links"
        );
        assert_eq!(
            found(&session(&[], &["AUTH-420 AUTH-420 IAM-5"]), &k),
            vec![("1".into(), LinkSource::Prompt, "AUTH-420".into(), 3)],
            "one link; the most-mentioned alias names it; mentions summed"
        );
        assert_eq!(
            found(&session(&[], &["AUTH-420 IAM-5"]), &k),
            vec![("1".into(), LinkSource::Prompt, "IAM-5".into(), 2)],
            "a tie goes to the current key"
        );
    }
}
