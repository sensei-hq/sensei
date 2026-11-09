//! The GitHub adapter: a Project (v2) on github.com (spec
//! `docs/spec/daemon/tracker-github.md`, D92). Sprints are the iterations of the
//! project's Iteration field; tickets are its issues, from any repository. Reads
//! go through validated GraphQL queries (§H2) with the board's own token. Keys and
//! links are [`crate::tracker::forge`]'s. History and write-back are unsupported (§H6).

use crate::tracker::forge::{date, midnight, text, timestamp};
use crate::tracker::graphql::GraphqlQuery;
use crate::tracker::sprint::SprintState;
use crate::tracker::{
    AccountRef, ApiToken, AuthScheme, BoardScope, Capabilities, ConnectParams, Discovered,
    FieldCheck, FieldValue, GetTransport, KnownTask, Link, OwnerLookup, ParamError, PrevState,
    SessionText, Snapshot, SprintRow, TaskRow, TrackerAdapter, TrackerError, TrackerKind,
    ValidateError, Validated, WriteTransport,
};

pub struct GithubAdapter;

/// Every GitHub board is read here; github.com only (§H1).
pub const API_BASE: &str = "https://api.github.com";
/// The board's items, at most this many pages of 100 (§H3.6).
const MAX_ITEM_PAGES: u32 = 200;
/// An issue's project items, at most this many pages of 50 (§H4).
const MAX_PROJECT_ITEM_PAGES: u32 = 10;

const Q_VALIDATE_ORG: &str = concat!(
    include_str!("github_queries/validate_org.graphql"),
    "\n",
    include_str!("github_queries/board_fragment.graphql")
);
const Q_VALIDATE_USER: &str = concat!(
    include_str!("github_queries/validate_user.graphql"),
    "\n",
    include_str!("github_queries/board_fragment.graphql")
);
const Q_BOARD: &str = concat!(
    include_str!("github_queries/board.graphql"),
    "\n",
    include_str!("github_queries/board_fragment.graphql")
);
const Q_ITEMS: &str = include_str!("github_queries/items.graphql");
const Q_ISSUE: &str = include_str!("github_queries/issue.graphql");
/// `Q_ISSUE`'s `NOT_FOUND` paths that mean "no such repository or issue" (§H2.3).
const ISSUE_ABSENT: &[&[&str]] = &[&["repository"], &["repository", "issue"]];

/// Who owns a project: the URL segment and GraphQL root field differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerKind {
    Organization,
    User,
}

impl OwnerKind {
    fn segment(self) -> &'static str {
        match self {
            OwnerKind::Organization => "orgs",
            OwnerKind::User => "users",
        }
    }
    fn label(self) -> &'static str {
        match self {
            OwnerKind::Organization => "organization",
            OwnerKind::User => "user",
        }
    }
}

/// A GitHub login: 1–39 of `[A-Za-z0-9-]`, not starting with `-` (§H1.1).
fn is_owner(s: &str) -> bool {
    (1..=39).contains(&s.len())
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && s.as_bytes()[0].is_ascii_alphanumeric()
}

/// A repository name: 1–100 of `[A-Za-z0-9._-]` (§H4).
fn is_repo_name(s: &str) -> bool {
    (1..=100).contains(&s.len())
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// `(owner kind, owner, project number)` from a board URL (§H1.1); `None` for anything
/// that is not exactly a github.com project address.
pub fn parse_board_url(raw: &str) -> Option<(OwnerKind, String, u32)> {
    let url = reqwest::Url::parse(raw.trim()).ok()?;
    if url.scheme() != "https"
        || url.host_str()? != "github.com"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let mut segments: Vec<&str> = url.path_segments()?.collect();
    if segments.last() == Some(&"") {
        segments.pop();
    }
    let (kind, owner, number) = match segments.as_slice() {
        [kind, owner, "projects", n] | [kind, owner, "projects", n, "views", _] => {
            (*kind, *owner, *n)
        }
        _ => return None,
    };
    let kind = match kind {
        "orgs" => OwnerKind::Organization,
        "users" => OwnerKind::User,
        _ => return None,
    };
    let digits = (1..=9).contains(&number.len()) && number.bytes().all(|b| b.is_ascii_digit());
    if !is_owner(owner) || !digits {
        return None;
    }
    Some((kind, owner.to_ascii_lowercase(), number.parse().ok()?))
}

/// `(owner, repository, number)` of a forge key a GitHub board can answer (§H4): exactly
/// `owner/name#n` with GitHub's charsets and `n` within GraphQL's `Int`. Anything else
/// is never requested.
pub fn github_key(key: &str) -> Option<(String, String, i64)> {
    let (path, n) = crate::tracker::forge::parse_key(key)?;
    let (owner, name) = path.split_once('/')?;
    if !is_owner(owner) || !is_repo_name(name) || n == 0 || n > i32::MAX as u64 {
        return None;
    }
    Some((owner.to_string(), name.to_string(), n as i64))
}

/// One Iteration field of a project, as validate and the snapshot read it.
struct IterationField<'a> {
    name: &'a str,
    node: &'a serde_json::Value,
}

/// A project's iteration fields, in its order; `HttpError` when the field list was
/// cut at 100, since a hidden field could be the one in use (§H1 validate 2).
fn iteration_fields(project: &serde_json::Value) -> Result<Vec<IterationField<'_>>, TrackerError> {
    if project.pointer("/fields/pageInfo/hasNextPage").and_then(|v| v.as_bool()) != Some(false) {
        return Err(TrackerError::HttpError);
    }
    let nodes =
        project.pointer("/fields/nodes").and_then(|v| v.as_array()).ok_or(TrackerError::Decode)?;
    nodes
        .iter()
        .filter(|n| text(n, "__typename") == Some("ProjectV2IterationField"))
        .map(|n| Ok(IterationField { name: text(n, "name").ok_or(TrackerError::Decode)?, node: n }))
        .collect()
}

/// The field validate picks (§H1 validate 3): the named one; else the only one;
/// else none when there are none. `Err(names)` when that is ambiguous.
fn choose_field<'a>(
    fields: &'a [IterationField<'a>],
    named: Option<&str>,
) -> Result<Option<&'a IterationField<'a>>, Vec<String>> {
    let names = || fields.iter().map(|f| f.name.to_string()).collect::<Vec<_>>();
    match named {
        Some(n) => fields.iter().find(|f| f.name == n).map(Some).ok_or_else(names),
        None => match fields {
            [] => Ok(None),
            [only] => Ok(Some(only)),
            _ => Err(names()),
        },
    }
}

/// The field a snapshot uses (§H3.2): only the stored one, never another.
fn stored_field<'a>(
    fields: &'a [IterationField<'a>],
    stored: Option<&str>,
) -> Result<Option<&'a IterationField<'a>>, TrackerError> {
    match stored {
        Some(name) => {
            fields.iter().find(|f| f.name == name).map(Some).ok_or(TrackerError::NotFound)
        }
        None if fields.is_empty() => Ok(None),
        None => Err(TrackerError::Decode),
    }
}

/// The settings validate and every snapshot store (§H1).
fn board_params(
    project: &serde_json::Value,
    field: Option<&str>,
) -> Result<serde_json::Map<String, serde_json::Value>, TrackerError> {
    let owner = project.get("owner").ok_or(TrackerError::Decode)?;
    let owner_kind = match text(owner, "__typename") {
        Some("Organization") => OwnerKind::Organization,
        Some("User") => OwnerKind::User,
        _ => return Err(TrackerError::Decode),
    };
    let mut m = serde_json::Map::new();
    m.insert(
        "owner".into(),
        text(owner, "login").ok_or(TrackerError::Decode)?.to_ascii_lowercase().into(),
    );
    m.insert("owner_kind".into(), owner_kind.label().into());
    m.insert(
        "number".into(),
        project.get("number").and_then(|v| v.as_u64()).ok_or(TrackerError::Decode)?.into(),
    );
    m.insert("url".into(), text(project, "url").ok_or(TrackerError::Decode)?.into());
    m.insert("iteration_field".into(), field.map(String::from).into());
    Ok(m)
}

/// The board's node id from `project:<id>`.
fn node_id(board: &BoardScope) -> Result<&str, TrackerError> {
    board.board_ref.strip_prefix("project:").filter(|s| !s.is_empty()).ok_or(TrackerError::Decode)
}

/// The stored `params.iteration_field` (§H3.2).
fn field_of(board: &BoardScope) -> Option<&str> {
    board.params.get("iteration_field").and_then(|v| v.as_str())
}

/// One iteration as a sprint (§H3.3). `now` decides active vs future.
fn iteration_row(
    v: &serde_json::Value,
    completed: bool,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<SprintRow, TrackerError> {
    let id = text(v, "id").ok_or(TrackerError::Decode)?;
    let start = date(v, "startDate")?.ok_or(TrackerError::Decode)?;
    let days = v
        .get("duration")
        .and_then(|d| d.as_i64())
        .filter(|d| *d > 0)
        .ok_or(TrackerError::Decode)?;
    let start_at = midnight(start);
    let state = if completed {
        SprintState::Closed
    } else if start_at > now {
        SprintState::Future
    } else {
        SprintState::Active
    };
    Ok(SprintRow {
        external_id: format!("iteration:{id}"),
        seq: (start - chrono::NaiveDate::default()).num_days(),
        name: text(v, "title").ok_or(TrackerError::Decode)?.to_string(),
        state,
        start_at: Some(start_at),
        end_at: Some(midnight(start + chrono::Duration::days(days))),
        complete_at: None,
    })
}

/// Every iteration of a field, completed ones closed (§H3.3).
fn iteration_rows(
    field: &serde_json::Value,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<SprintRow>, TrackerError> {
    let list = |p: &str| field.pointer(p).and_then(|v| v.as_array()).ok_or(TrackerError::Decode);
    let mut rows = Vec::new();
    for it in list("/configuration/iterations")? {
        rows.push(iteration_row(it, false, now)?);
    }
    for it in list("/configuration/completedIterations")? {
        rows.push(iteration_row(it, true, now)?);
    }
    Ok(rows)
}

/// An issue as a ticket (§H3.4).
fn task_row(issue: &serde_json::Value) -> Result<TaskRow, TrackerError> {
    let repo = issue.get("repository").filter(|r| !r.is_null()).ok_or(TrackerError::Decode)?;
    let name = text(repo, "nameWithOwner").ok_or(TrackerError::Decode)?.to_ascii_lowercase();
    let number = issue.get("number").and_then(|n| n.as_u64()).ok_or(TrackerError::Decode)?;
    let scope = crate::db::pg_store::repo_key::normalize_repo_key(
        text(repo, "url").ok_or(TrackerError::Decode)?,
    )
    .ok_or(TrackerError::Decode)?;
    let (status, category) = match text(issue, "state") {
        Some("OPEN") => ("open", "new"),
        Some("CLOSED") => ("closed", "done"),
        _ => return Err(TrackerError::Decode),
    };
    Ok(TaskRow {
        external_id: issue
            .get("databaseId")
            .and_then(|d| d.as_u64())
            .ok_or(TrackerError::Decode)?
            .to_string(),
        key: format!("{name}#{number}"),
        scope: Some(scope),
        summary: text(issue, "title").map(String::from),
        issue_type: issue.pointer("/issueType/name").and_then(|v| v.as_str()).map(String::from),
        status: Some(status.into()),
        assignee: issue
            .pointer("/assignees/nodes/0/databaseId")
            .and_then(|v| v.as_u64())
            .map(|id| AccountRef::new(id.to_string())),
        created_at: timestamp(issue, "createdAt")?,
        updated_at: timestamp(issue, "updatedAt")?,
        status_category: Some(category.into()),
        // Not asked: estimates are unsupported (§H6).
        points: None,
    })
}

/// The iteration an item or project item sits in, from `fieldValueByName`.
fn iteration_of(item: &serde_json::Value) -> Option<&str> {
    let v = item.get("fieldValueByName")?;
    (text(v, "__typename") == Some("ProjectV2ItemFieldIterationValue"))
        .then(|| text(v, "iterationId"))?
}

/// Pure (§H3): a snapshot from the board node, the stored field, and every items page.
fn snapshot_from(
    project: &serde_json::Value,
    stored: Option<&str>,
    item_pages: &[serde_json::Value],
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Snapshot, TrackerError> {
    let fields = iteration_fields(project)?;
    let field = stored_field(&fields, stored)?;
    let sprints = match field {
        Some(f) => iteration_rows(f.node, now)?,
        None => Vec::new(),
    };
    let mut members: std::collections::BTreeMap<String, Vec<String>> =
        sprints.iter().map(|s| (s.external_id.clone(), Vec::new())).collect();
    let mut tasks = Vec::new();
    for page in item_pages {
        let nodes = page.get("nodes").and_then(|v| v.as_array()).ok_or(TrackerError::Decode)?;
        for item in nodes {
            let is_archived =
                item.get("isArchived").and_then(|v| v.as_bool()).ok_or(TrackerError::Decode)?;
            if text(item, "type") != Some("ISSUE") || is_archived {
                continue;
            }
            let content = item.get("content").filter(|c| text(c, "__typename") == Some("Issue"));
            let task = task_row(content.ok_or(TrackerError::Decode)?)?;
            if let Some(it) = iteration_of(item) {
                members
                    .get_mut(&format!("iteration:{it}"))
                    .ok_or(TrackerError::Decode)?
                    .push(task.external_id.clone());
            }
            tasks.push(task);
        }
    }
    Ok(Snapshot {
        board_name: Some(text(project, "title").ok_or(TrackerError::Decode)?.to_string()),
        sprints,
        tasks,
        members: members.into_iter().map(|(s, ids)| (s, Some(ids))).collect(),
        points_field: None,
        params_extra: board_params(project, field.map(|f| f.name))?,
    })
}

/// The token's owner (§H6): a bot is not a person.
fn owner_from_user(v: &serde_json::Value) -> Result<OwnerLookup, TrackerError> {
    if text(v, "type") == Some("Bot") {
        return Ok(OwnerLookup::NotAPerson);
    }
    let id = v.get("id").and_then(|i| i.as_u64()).ok_or(TrackerError::Decode)?;
    Ok(OwnerLookup::Person(AccountRef::new(id.to_string())))
}

/// The board node, by id (§H3.1). A null node is a deleted project.
async fn board_node(t: &dyn GetTransport, id: &str) -> Result<serde_json::Value, TrackerError> {
    let q = GraphqlQuery::new(Q_BOARD, serde_json::json!({"id": id}))?;
    let data = t.graphql(&q).await?;
    data.get("node").filter(|n| !n.is_null()).cloned().ok_or(TrackerError::NotFound)
}

/// Every items page (§H3.4, §H3.6): never a partial board as whole.
async fn item_pages(
    t: &dyn GetTransport,
    id: &str,
    field: Option<&str>,
) -> Result<Vec<serde_json::Value>, TrackerError> {
    let mut pages = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..MAX_ITEM_PAGES {
        let q = GraphqlQuery::new(
            Q_ITEMS,
            serde_json::json!({"id": id, "field": field.unwrap_or(""), "after": after}),
        )?;
        let data = t.graphql(&q).await?;
        let items =
            data.pointer("/node/items").filter(|v| !v.is_null()).ok_or(TrackerError::NotFound)?;
        let more = items
            .pointer("/pageInfo/hasNextPage")
            .and_then(|v| v.as_bool())
            .ok_or(TrackerError::Decode)?;
        after = items.pointer("/pageInfo/endCursor").and_then(|v| v.as_str()).map(String::from);
        pages.push(items.clone());
        if !more {
            return Ok(pages);
        }
    }
    Err(TrackerError::HttpError)
}

/// One issue and the project item that puts it on this board, if any (§H4): `None`
/// for no such repository or issue, a pull-request number, and an issue off the
/// board alike.
async fn issue_on_board(
    t: &dyn GetTransport,
    board_id: &str,
    field: Option<&str>,
    (owner, name, number): (&str, &str, i64),
) -> Result<Option<(serde_json::Value, serde_json::Value)>, TrackerError> {
    let mut after: Option<String> = None;
    for _ in 0..MAX_PROJECT_ITEM_PAGES {
        let q = GraphqlQuery::new(
            Q_ISSUE,
            serde_json::json!({
                "owner": owner, "name": name, "number": number,
                "field": field.unwrap_or(""), "after": after,
            }),
        )?
        .absent_on_not_found(ISSUE_ABSENT);
        let data = t.graphql(&q).await?;
        let Some(issue) = data.pointer("/repository/issue").filter(|i| !i.is_null()) else {
            return Ok(None);
        };
        let items = issue.get("projectItems").ok_or(TrackerError::Decode)?;
        let nodes = items.get("nodes").and_then(|v| v.as_array()).ok_or(TrackerError::Decode)?;
        let on_board = nodes.iter().find(|n| {
            n.pointer("/project/id").and_then(|v| v.as_str()) == Some(board_id)
                && n.get("isArchived").and_then(|v| v.as_bool()) == Some(false)
        });
        if let Some(node) = on_board {
            return Ok(Some((issue.clone(), node.clone())));
        }
        if items.pointer("/pageInfo/hasNextPage").and_then(|v| v.as_bool()) != Some(true) {
            return Ok(None);
        }
        after = items.pointer("/pageInfo/endCursor").and_then(|v| v.as_str()).map(String::from);
    }
    Err(TrackerError::HttpError)
}

#[async_trait::async_trait]
impl TrackerAdapter for GithubAdapter {
    fn kind(&self) -> TrackerKind {
        TrackerKind::Github
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { history: false, writeback: false }
    }

    fn reads_graphql(&self) -> bool {
        true
    }

    fn connect_params(&self, body: &serde_json::Value) -> Result<ConnectParams, ParamError> {
        let text =
            |k: &str| body.get(k).and_then(|v| v.as_str()).map(str::trim).filter(|v| !v.is_empty());
        let (kind, owner, number) =
            text("board_url").and_then(parse_board_url).ok_or(ParamError::InvalidBaseUrl)?;
        let token = text("api_token").ok_or(ParamError::TokenRequired)?.to_string();
        // Both provisional: validate replaces them with the project's node id and the
        // token's login (§H1).
        Ok(ConnectParams {
            base_url: API_BASE.into(),
            board_ref: format!("{}/{owner}/{number}", kind.segment()),
            account: String::new(),
            token: ApiToken(token),
            project_key: None,
            sprint_field: text("iteration_field").map(String::from),
        })
    }

    fn auth(&self, _account: &str, token: &ApiToken) -> AuthScheme {
        AuthScheme::Header { name: "Authorization", token: ApiToken(format!("Bearer {}", token.0)) }
    }

    async fn owner(&self, t: &dyn GetTransport) -> Result<OwnerLookup, TrackerError> {
        owner_from_user(&t.get_json("/user", &[]).await?)
    }

    async fn validate(
        &self,
        t: &dyn GetTransport,
        p: &ConnectParams,
    ) -> Result<Validated, ValidateError> {
        let user = t.get_json("/user", &[]).await.map_err(ValidateError::Tracker)?;
        let owner = owner_from_user(&user).map_err(ValidateError::Tracker)?;
        let login =
            text(&user, "login").ok_or(ValidateError::Tracker(TrackerError::Decode))?.to_string();
        let decode = || ValidateError::Tracker(TrackerError::Decode);
        let mut typed = p.board_ref.splitn(3, '/');
        let (kind, login_of_board, number) = (typed.next(), typed.next(), typed.next());
        let (query, root) = match kind {
            Some("orgs") => (Q_VALIDATE_ORG, "/organization/projectV2"),
            Some("users") => (Q_VALIDATE_USER, "/user/projectV2"),
            _ => return Err(decode()),
        };
        let number: u32 = number.and_then(|n| n.parse().ok()).ok_or_else(decode)?;
        let q = GraphqlQuery::new(
            query,
            serde_json::json!({"owner": login_of_board, "number": number}),
        )
        .map_err(ValidateError::Tracker)?;
        let data = match t.graphql(&q).await {
            Ok(d) => d,
            Err(TrackerError::NotFound) => return Err(ValidateError::BoardNotFound),
            Err(e) => return Err(ValidateError::Tracker(e)),
        };
        let project =
            data.pointer(root).filter(|v| !v.is_null()).ok_or(ValidateError::BoardNotFound)?;
        let fields = iteration_fields(project).map_err(ValidateError::Tracker)?;
        let field = choose_field(&fields, p.sprint_field.as_deref())
            .map_err(ValidateError::IterationFieldAmbiguous)?;
        let params =
            board_params(project, field.map(|f| f.name)).map_err(ValidateError::Tracker)?;
        let id = text(project, "id").ok_or_else(decode)?;
        Ok(Validated {
            external_scope: format!("{}/{number}", params["owner"].as_str().ok_or_else(decode)?),
            board_name: text(project, "title").ok_or_else(decode)?.to_string(),
            owner,
            account: Some(login),
            board_ref: Some(format!("project:{id}")),
            params,
        })
    }

    async fn fetch_snapshot(
        &self,
        t: &dyn GetTransport,
        board: &BoardScope,
        _prev: &PrevState,
    ) -> Result<Snapshot, TrackerError> {
        let id = node_id(board)?;
        let project = board_node(t, id).await?;
        // The field is checked before any item is read: a missing one fails fast.
        let fields = iteration_fields(&project)?;
        let field = stored_field(&fields, field_of(board))?.map(|f| f.name.to_string());
        let pages = item_pages(t, id, field.as_deref()).await?;
        snapshot_from(&project, field.as_deref(), &pages, chrono::Utc::now())
    }

    fn match_links(&self, session: &SessionText, known: &[KnownTask]) -> Vec<Link> {
        crate::tracker::forge::match_links(session, known)
    }

    fn unknown_keys(&self, session: &SessionText, known: &[KnownTask]) -> Vec<String> {
        crate::tracker::forge::unknown_keys(session, known)
    }

    async fn fetch_tasks(
        &self,
        t: &dyn GetTransport,
        board: &BoardScope,
        keys: &[String],
    ) -> Result<Discovered, TrackerError> {
        let id = node_id(board)?;
        let mut found = Discovered::default();
        for key in keys {
            // A key that is not a GitHub key is never requested (§H4).
            let Some((owner, name, n)) = github_key(key) else { continue };
            let Some((issue, node)) =
                issue_on_board(t, id, field_of(board), (&owner, &name, n)).await?
            else {
                continue;
            };
            let task = task_row(&issue)?;
            if let Some(it) = iteration_of(&node) {
                found.members.push((format!("iteration:{it}"), task.external_id.clone()));
            }
            found.tasks.push(task);
        }
        Ok(found)
    }

    // ── Write-back: unsupported (§H6), unreachable behind the capability guards ──

    async fn find_comment(
        &self,
        _t: &dyn GetTransport,
        _task: &str,
        _owner: &AccountRef,
    ) -> Result<Option<String>, TrackerError> {
        Err(TrackerError::UnsupportedTracker)
    }
    async fn create_comment(
        &self,
        _w: &dyn WriteTransport,
        _task: &str,
        _report: &crate::tracker::writeback::Report,
    ) -> Result<String, TrackerError> {
        Err(TrackerError::UnsupportedTracker)
    }
    async fn update_comment(
        &self,
        _w: &dyn WriteTransport,
        _task: &str,
        _comment: &str,
        _report: &crate::tracker::writeback::Report,
    ) -> Result<(), TrackerError> {
        Err(TrackerError::UnsupportedTracker)
    }
    fn field_ref_shape(&self, _field_ref: &str) -> bool {
        false
    }
    async fn check_fields(
        &self,
        _t: &dyn GetTransport,
        _field_refs: &[String],
    ) -> Result<Vec<FieldCheck>, TrackerError> {
        Err(TrackerError::UnsupportedTracker)
    }
    async fn field_in_use(
        &self,
        _t: &dyn GetTransport,
        _board: &BoardScope,
        _field_ref: &str,
    ) -> Result<bool, TrackerError> {
        Err(TrackerError::UnsupportedTracker)
    }
    async fn write_fields(
        &self,
        _w: &dyn WriteTransport,
        _task: &str,
        _fields: &[(String, FieldValue)],
    ) -> Result<(), TrackerError> {
        Err(TrackerError::UnsupportedTracker)
    }
    async fn issue_exists(&self, _t: &dyn GetTransport, _task: &str) -> Result<bool, TrackerError> {
        Err(TrackerError::UnsupportedTracker)
    }
}

/// `get_ticket` for a GitHub key (§H5): one issue, read live with the board's own
/// token, and returned only when it has a non-archived item on this board's project.
/// Missing, invisible, a pull request, and off the board all read `None` alike.
pub struct GithubTicketReader<'a> {
    pub transport: &'a dyn GetTransport,
    pub board: BoardScope,
}

#[async_trait::async_trait]
impl crate::tracker::ticket::TicketReader for GithubTicketReader<'_> {
    async fn read_ticket(
        &self,
        key: &str,
    ) -> Result<Option<crate::tracker::ticket::TicketDetail>, TrackerError> {
        use crate::tracker::ticket::{AcceptanceCriteriaSource, TicketDetail};
        let Some((owner, name, n)) = github_key(key) else { return Ok(None) };
        let id = node_id(&self.board)?;
        let Some((issue, node)) =
            issue_on_board(self.transport, id, field_of(&self.board), (&owner, &name, n)).await?
        else {
            return Ok(None);
        };
        let named = |p: &str| issue.pointer(p).and_then(|v| v.as_str()).map(String::from);
        Ok(Some(TicketDetail {
            key: format!("{owner}/{name}#{n}").to_ascii_lowercase(),
            title: named("/title").ok_or(TrackerError::Decode)?,
            description: named("/body").filter(|d| !d.trim().is_empty()),
            // GitHub has no criteria field (§H5).
            acceptance_criteria: Vec::new(),
            acceptance_criteria_source: AcceptanceCriteriaSource::NotConfigured,
            status: named("/state").map(|s| s.to_ascii_lowercase()),
            issue_type: named("/issueType/name"),
            sprint: node
                .pointer("/fieldValueByName/title")
                .and_then(|v| v.as_str())
                .map(String::from),
            assignee: named("/assignees/nodes/0/login"),
            tracker: TrackerKind::Github,
            url: named("/url").ok_or(TrackerError::Decode)?,
        }))
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use crate::tracker::graphql::GraphqlQuery;
    use crate::tracker::{GetTransport, TrackerError};
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// A GitHub served from recorded fixtures (`crates/senseid/tests/fixtures/github/<set>`,
    /// recorded by `scripts/record-github-fixtures.py`). A GraphQL read is answered by its
    /// operation and variables (compared as JSON values, so key order never matters);
    /// anything unrecorded panics, so a test never passes on a guessed response. Every
    /// answer goes through the query's own envelope mapping, as the real transport does.
    pub struct RecordedGithub {
        graphql: Vec<(String, serde_json::Value, serde_json::Value)>,
        rest: HashMap<String, serde_json::Value>,
        pub calls: Mutex<Vec<String>>,
    }

    pub fn fixture_dir(set: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/github").join(set)
    }

    /// A recorded response's body (a GraphQL envelope), by set and file.
    pub fn fixture_body(set: &str, file: &str) -> serde_json::Value {
        let raw = std::fs::read_to_string(fixture_dir(set).join(format!("{file}.json"))).unwrap();
        serde_json::from_str::<serde_json::Value>(&raw).unwrap()["body"].clone()
    }

    impl RecordedGithub {
        /// The union of several fixture sets.
        pub fn load(sets: &[&str]) -> Self {
            let mut graphql = Vec::new();
            let mut rest = HashMap::new();
            for set in sets {
                for entry in std::fs::read_dir(fixture_dir(set)).unwrap() {
                    let raw = std::fs::read_to_string(entry.unwrap().path()).unwrap();
                    let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
                    if let Some(op) = doc["operation"].as_str() {
                        graphql.push((
                            op.to_string(),
                            doc["variables"].clone(),
                            doc["body"].clone(),
                        ));
                    } else {
                        rest.insert(
                            doc["request"].as_str().unwrap().to_string(),
                            doc["body"].clone(),
                        );
                    }
                }
            }
            RecordedGithub { graphql, rest, calls: Mutex::new(Vec::new()) }
        }

        /// Answer one GraphQL read with `envelope`: for what a recording cannot show
        /// (an error, a second page), never for a field's shape.
        pub fn with(
            mut self,
            operation: &str,
            variables: serde_json::Value,
            envelope: serde_json::Value,
        ) -> Self {
            self.graphql.retain(|(op, vars, _)| !(op == operation && *vars == variables));
            self.graphql.push((operation.to_string(), variables, envelope));
            self
        }

        pub fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl GetTransport for RecordedGithub {
        async fn get_json(
            &self,
            path: &str,
            _query: &[(&str, String)],
        ) -> Result<serde_json::Value, TrackerError> {
            self.calls.lock().unwrap().push(path.to_string());
            Ok(self.rest.get(path).unwrap_or_else(|| panic!("no recording for GET {path}")).clone())
        }

        async fn graphql(&self, q: &GraphqlQuery) -> Result<serde_json::Value, TrackerError> {
            let k = format!("{} {}", q.operation(), q.variables());
            self.calls.lock().unwrap().push(k.clone());
            let (_, _, envelope) = self
                .graphql
                .iter()
                .find(|(op, vars, _)| op == q.operation() && vars == q.variables())
                .unwrap_or_else(|| panic!("no recording for {k}"));
            q.read(envelope.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::RecordedGithub;
    use super::*;
    use serde_json::json;

    fn at(day: &str) -> chrono::DateTime<chrono::Utc> {
        midnight(chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").unwrap())
    }

    fn board(field: Option<&str>) -> BoardScope {
        BoardScope {
            base_url: API_BASE.into(),
            board_ref: "project:PVT_1".into(),
            params: json!({"iteration_field": field}),
        }
    }

    fn body(url: &str) -> serde_json::Value {
        json!({"board_url": url, "api_token": "github_pat_x"})
    }

    // ── E2: connect_params ──────────────────────────────────────────────────

    #[test]
    fn a_board_url_is_exactly_a_github_project_address() {
        let ok = |url: &str| GithubAdapter.connect_params(&body(url)).map(|p| p.board_ref);
        assert_eq!(
            ok("https://github.com/orgs/Sensei-HQ/projects/3/views/1").as_deref(),
            Ok("orgs/sensei-hq/3")
        );
        assert_eq!(ok("https://github.com/users/bob/projects/2/").as_deref(), Ok("users/bob/2"));
        assert_eq!(ok("https://GITHUB.COM/orgs/a/projects/1").as_deref(), Ok("orgs/a/1"));
        assert_eq!(ok("https://github.com/orgs/a/projects/1?query=x#y").as_deref(), Ok("orgs/a/1"));
        for bad in [
            "http://github.com/orgs/a/projects/1",
            "https://github.example.com/orgs/a/projects/1",
            "https://github.com.evil.com/orgs/a/projects/1",
            "https://api.github.com/orgs/a/projects/1",
            "https://u:p@github.com/orgs/a/projects/1",
            "https://github.com:444/orgs/a/projects/1",
            "https://github.com/orgs/a/projects/x",
            "https://github.com/orgs/a",
            "https://github.com/orgs/a%2fb/projects/1",
            "https://github.com/orgs/-a/projects/1",
            "https://github.com/teams/a/projects/1",
            "https://github.com/orgs/a/projects/1234567890",
        ] {
            assert_eq!(ok(bad), Err(ParamError::InvalidBaseUrl), "{bad}");
        }
        let p = GithubAdapter
            .connect_params(&json!({"board_url": "https://github.com/orgs/a/projects/1", "api_token": "t", "iteration_field": " Sprint "}))
            .unwrap();
        assert_eq!(
            (p.base_url.as_str(), p.account.as_str(), p.sprint_field.as_deref()),
            (API_BASE, "", Some("Sprint"))
        );
        assert_eq!(
            GithubAdapter
                .connect_params(&json!({"board_url": "https://github.com/orgs/a/projects/1"})),
            Err(ParamError::TokenRequired)
        );
    }

    #[test]
    fn the_token_is_sent_as_a_bearer_header() {
        let auth = GithubAdapter.auth("user1", &ApiToken("github_pat_x".into()));
        assert_eq!(
            auth,
            AuthScheme::Header {
                name: "Authorization",
                token: ApiToken("Bearer github_pat_x".into())
            }
        );
        assert!(GithubAdapter.reads_graphql());
        assert_eq!(GithubAdapter.capabilities(), Capabilities { history: false, writeback: false });
    }

    // ── E3: validate ────────────────────────────────────────────────────────

    fn typed(board_ref: &str, field: Option<&str>) -> ConnectParams {
        ConnectParams {
            base_url: API_BASE.into(),
            board_ref: board_ref.into(),
            account: String::new(),
            token: ApiToken("t".into()),
            project_key: None,
            sprint_field: field.map(String::from),
        }
    }

    #[tokio::test]
    async fn validate_names_the_project_by_node_id_and_the_token_by_login() {
        let gh = RecordedGithub::load(&["project"]);
        let v = GithubAdapter.validate(&gh, &typed("orgs/acme/2", None)).await.unwrap();
        assert_eq!(v.board_ref.as_deref(), Some("project:PVT_1"));
        assert_eq!(v.account.as_deref(), Some("user1"));
        assert_eq!((v.external_scope.as_str(), v.board_name.as_str()), ("acme/2", "Project 2"));
        assert_eq!(
            v.params["iteration_field"],
            json!("Sprint"),
            "the only field is taken and stored"
        );
        assert_eq!(v.params["owner_kind"], json!("organization"));
        assert_eq!(v.params["url"], json!("https://github.com/orgs/acme/projects/2"));
        assert!(gh.calls().len() <= 3, "{:?}", gh.calls());
        // Named and present, too.
        let named =
            GithubAdapter.validate(&gh, &typed("orgs/acme/2", Some("Sprint"))).await.unwrap();
        assert_eq!(named.params["iteration_field"], json!("Sprint"));
        // No field at all: connects, with none stored.
        let nf = RecordedGithub::load(&["project", "no-field"]);
        let v = GithubAdapter.validate(&nf, &typed("orgs/acme/3", None)).await.unwrap();
        assert_eq!(v.params["iteration_field"], json!(null));
    }

    #[tokio::test]
    async fn an_ambiguous_or_missing_field_lists_the_names() {
        let gh = RecordedGithub::load(&["project"]);
        assert_eq!(
            GithubAdapter.validate(&gh, &typed("orgs/acme/2", Some("Release"))).await,
            Err(ValidateError::IterationFieldAmbiguous(vec!["Sprint".into()]))
        );
        // Two iteration fields and no name: scripted, since the recorded project has one.
        let mut two = super::test_support::fixture_body("project", "validate_org");
        let fields = two
            .pointer_mut("/data/organization/projectV2/fields/nodes")
            .unwrap()
            .as_array_mut()
            .unwrap();
        let mut second = fields.iter().find(|f| f["name"] == json!("Sprint")).unwrap().clone();
        second["name"] = json!("Release");
        fields.push(second);
        let gh = RecordedGithub::load(&["project"]).with(
            "ValidateOrg",
            json!({"owner": "acme", "number": 2}),
            two,
        );
        assert_eq!(
            GithubAdapter.validate(&gh, &typed("orgs/acme/2", None)).await,
            Err(ValidateError::IterationFieldAmbiguous(vec!["Sprint".into(), "Release".into()]))
        );
    }

    #[tokio::test]
    async fn validate_maps_github_errors() {
        let script = |envelope| {
            RecordedGithub::load(&["project"]).with(
                "ValidateOrg",
                json!({"owner": "acme", "number": 2}),
                envelope,
            )
        };
        let err = |t: &str| json!({"data": {"organization": {"projectV2": null}}, "errors": [{"type": t, "path": ["organization", "projectV2"]}]});
        let v = |gh: RecordedGithub| async move {
            GithubAdapter.validate(&gh, &typed("orgs/acme/2", None)).await
        };
        assert_eq!(v(script(err("NOT_FOUND"))).await, Err(ValidateError::BoardNotFound));
        assert_eq!(
            v(script(err("INSUFFICIENT_SCOPES"))).await,
            Err(ValidateError::Tracker(TrackerError::Forbidden))
        );
        assert_eq!(
            v(script(json!({"data": {"organization": {"projectV2": null}}}))).await,
            Err(ValidateError::BoardNotFound)
        );
        let mut many = super::test_support::fixture_body("project", "validate_org");
        many["data"]["organization"]["projectV2"]["fields"]["pageInfo"]["hasNextPage"] =
            json!(true);
        assert_eq!(v(script(many)).await, Err(ValidateError::Tracker(TrackerError::HttpError)));
    }

    #[test]
    fn a_bot_token_is_not_a_person() {
        assert_eq!(owner_from_user(&json!({"id": 7, "type": "Bot"})), Ok(OwnerLookup::NotAPerson));
        assert_eq!(
            owner_from_user(&json!({"id": 7, "type": "User"})),
            Ok(OwnerLookup::Person(AccountRef::new("7")))
        );
    }

    // ── E6, E7, E9: the snapshot ─────────────────────────────────────────────

    #[tokio::test]
    async fn the_recorded_project_maps_to_sprints_tickets_and_membership() {
        let gh = RecordedGithub::load(&["project"]);
        let id = node_id(&board(Some("Sprint"))).unwrap().to_string();
        let project = board_node(&gh, &id).await.unwrap();
        let pages = item_pages(&gh, &id, Some("Sprint")).await.unwrap();
        // Iteration 1 starts 2026-08-31 (shifted), Iteration 2 2026-09-14; 3 is completed.
        let s = snapshot_from(&project, Some("Sprint"), &pages, at("2026-09-05")).unwrap();
        let sprint = |id: &str| {
            s.sprints.iter().find(|r| r.external_id == format!("iteration:{id}")).unwrap()
        };
        assert_eq!(sprint("it1").state, SprintState::Active);
        assert_eq!(sprint("it2").state, SprintState::Future);
        assert_eq!(sprint("it3").state, SprintState::Closed);
        assert_eq!(sprint("it1").start_at, Some(at("2026-08-31")));
        assert_eq!(sprint("it1").end_at, Some(at("2026-09-14")), "start + 14 days, exclusive");
        assert_eq!(
            sprint("it1").seq,
            (chrono::NaiveDate::from_ymd_opt(2026, 8, 31).unwrap() - chrono::NaiveDate::default())
                .num_days()
        );
        assert_eq!(sprint("it1").name, "Iteration 1");
        // Five issues; the pull request and the draft are not tickets.
        let keys: Vec<&str> = s.tasks.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "acme/platform#1",
                "acme/platform#2",
                "acme/platform#3",
                "acme/platform#4",
                "acme/core#5"
            ]
        );
        let core = s.tasks.iter().find(|t| t.key == "acme/core#5").unwrap();
        assert_eq!(core.scope.as_deref(), Some("github.com/acme/core"));
        assert_eq!(
            (core.status.as_deref(), core.status_category.as_deref()),
            (Some("open"), Some("new"))
        );
        let first = &s.tasks[0];
        assert_eq!(
            (first.status.as_deref(), first.status_category.as_deref()),
            (Some("closed"), Some("done"))
        );
        assert_eq!(
            (first.external_id.as_str(), first.issue_type.as_deref(), first.assignee.as_ref()),
            ("1001", None, None)
        );
        assert_eq!(first.summary.as_deref(), Some("Issue 1"));
        let members = |id: &str| {
            s.members
                .iter()
                .find(|(k, _)| k == &format!("iteration:{id}"))
                .unwrap()
                .1
                .clone()
                .unwrap()
        };
        assert_eq!(members("it1"), ["1002", "1005"]);
        assert_eq!(members("it3"), ["1001"]);
        assert!(
            !s.members.iter().any(|(_, ids)| ids.as_ref().unwrap().contains(&"1004".to_string())),
            "no iteration → backlog"
        );
        assert_eq!(s.params_extra["iteration_field"], json!("Sprint"));
        assert_eq!(s.points_field, None);
    }

    #[tokio::test]
    async fn a_project_without_an_iteration_field_has_no_sprints() {
        let gh = RecordedGithub::load(&["no-field"]);
        let b = BoardScope {
            board_ref: "project:PVT_3".into(),
            params: json!({"iteration_field": null}),
            ..board(None)
        };
        let s = GithubAdapter.fetch_snapshot(&gh, &b, &PrevState::default()).await.unwrap();
        assert!(s.sprints.is_empty());
        assert_eq!(s.tasks.len(), 1);
        assert!(s.members.is_empty());
    }

    #[test]
    fn archived_items_assignees_and_types_map_as_the_spec_says() {
        let issue = |n: u64| {
            json!({
                "__typename": "Issue", "databaseId": 2000 + n, "number": n, "title": "t", "body": "",
                "state": "OPEN", "url": "u", "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-02T00:00:00Z",
                "issueType": {"name": "Bug"}, "repository": {"nameWithOwner": "Acme/Platform", "url": "https://github.com/Acme/Platform"},
                "assignees": {"nodes": [{"databaseId": 9001, "login": "user1"}]},
            })
        };
        let item = |n: u64, archived: bool| json!({"id": format!("PVTI_{n}"), "type": "ISSUE", "isArchived": archived, "fieldValueByName": null, "content": issue(n)});
        let project =
            super::test_support::fixture_body("no-field", "board")["data"]["node"].clone();
        let page = json!({"pageInfo": {"hasNextPage": false, "endCursor": null}, "nodes": [item(1, false), item(2, true), {"id": "PVTI_3", "type": "REDACTED", "isArchived": false, "fieldValueByName": null, "content": null}]});
        let s = snapshot_from(&project, None, &[page], at("2026-01-03")).unwrap();
        assert_eq!(s.tasks.len(), 1, "the archived and the redacted item are not tickets");
        let t = &s.tasks[0];
        assert_eq!(t.key, "acme/platform#1", "lowercased");
        assert_eq!(t.issue_type.as_deref(), Some("Bug"));
        assert_eq!(t.assignee, Some(AccountRef::new("9001")));
        // An ISSUE item with no content is a decode error, never skipped.
        let broken = json!({"pageInfo": {"hasNextPage": false}, "nodes": [{"type": "ISSUE", "isArchived": false, "content": null}]});
        assert_eq!(
            snapshot_from(&project, None, &[broken], at("2026-01-03")).map(|_| ()),
            Err(TrackerError::Decode)
        );
    }

    // ── E8: snapshot failures ───────────────────────────────────────────────

    #[tokio::test]
    async fn the_snapshot_refuses_to_guess() {
        let gh = RecordedGithub::load(&["project"]);
        let project = board_node(&gh, "PVT_1").await.unwrap();
        let pages = item_pages(&gh, "PVT_1", Some("Sprint")).await.unwrap();
        let now = at("2026-09-05");
        // A named field that has gone: not found, never another field.
        assert_eq!(
            snapshot_from(&project, Some("Release"), &pages, now).map(|_| ()),
            Err(TrackerError::NotFound)
        );
        // None stored but the project has one: decode, never adopted silently.
        assert_eq!(
            snapshot_from(&project, None, &pages, now).map(|_| ()),
            Err(TrackerError::Decode)
        );
        // An item in an iteration the field does not list.
        let mut odd = pages[0].clone();
        odd["nodes"][1]["fieldValueByName"]["iterationId"] = json!("gone");
        assert_eq!(
            snapshot_from(&project, Some("Sprint"), &[odd], now).map(|_| ()),
            Err(TrackerError::Decode)
        );
    }

    #[tokio::test]
    async fn every_page_is_read_and_a_board_past_the_cap_fails() {
        let page = |more: bool, cursor: Option<&str>| json!({"data": {"node": {"items": {"pageInfo": {"hasNextPage": more, "endCursor": cursor}, "nodes": []}}}});
        let vars = |after: Option<&str>| json!({"id": "PVT_9", "field": "", "after": after});
        let two = RecordedGithub::load(&[]).with("Items", vars(None), page(true, Some("c1"))).with(
            "Items",
            vars(Some("c1")),
            page(false, None),
        );
        assert_eq!(item_pages(&two, "PVT_9", None).await.unwrap().len(), 2);
        let mut endless =
            RecordedGithub::load(&[]).with("Items", vars(None), page(true, Some("c1")));
        for n in 1..MAX_ITEM_PAGES {
            let (this, next) = (format!("c{n}"), format!("c{}", n + 1));
            endless = endless.with("Items", vars(Some(&this)), page(true, Some(&next)));
        }
        assert_eq!(item_pages(&endless, "PVT_9", None).await, Err(TrackerError::HttpError));
        // A failing page fails the whole read.
        let failing =
            RecordedGithub::load(&[]).with("Items", vars(None), page(true, Some("c1"))).with(
                "Items",
                vars(Some("c1")),
                json!({"errors": [{"type": "SOMETHING", "path": ["node"]}]}),
            );
        assert_eq!(item_pages(&failing, "PVT_9", None).await, Err(TrackerError::HttpError));
    }

    // ── E11: discovery and containment ──────────────────────────────────────

    #[tokio::test]
    async fn discovery_returns_only_issues_on_this_board() {
        let gh = RecordedGithub::load(&["issue-on", "issue-off", "pr", "missing-repo"]);
        let b = board(Some("Sprint"));
        let keys: Vec<String> =
            ["acme/platform#2", "acme/platform#6", "acme/platform#7", "acme/missing#8"]
                .into_iter()
                .map(String::from)
                .collect();
        let found = GithubAdapter.fetch_tasks(&gh, &b, &keys).await.unwrap();
        assert_eq!(
            found.tasks.iter().map(|t| t.key.as_str()).collect::<Vec<_>>(),
            ["acme/platform#2"]
        );
        assert_eq!(
            found.members,
            [("iteration:it1".to_string(), found.tasks[0].external_id.clone())]
        );
        // Keys GitHub cannot have are never requested.
        let gh = RecordedGithub::load(&[]);
        let shapes: Vec<String> =
            ["acme/x/y#1", "acme/platform#2147483648", "a_b/platform#1", "#3"]
                .into_iter()
                .map(String::from)
                .collect();
        assert!(GithubAdapter.fetch_tasks(&gh, &b, &shapes).await.unwrap().tasks.is_empty());
        assert!(gh.calls().is_empty(), "{:?}", gh.calls());
    }

    #[tokio::test]
    async fn containment_pages_through_project_items_and_fails_at_the_cap() {
        let vars = |after: Option<&str>| json!({"owner": "acme", "name": "platform", "number": 2, "field": "Sprint", "after": after});
        let issue_page = |items: serde_json::Value, more: bool, cursor: Option<&str>| {
            let mut v = super::test_support::fixture_body("issue-on", "issue");
            v["data"]["repository"]["issue"]["projectItems"] =
                json!({"pageInfo": {"hasNextPage": more, "endCursor": cursor}, "nodes": items});
            v
        };
        let foreign =
            json!([{"isArchived": false, "project": {"id": "PVT_99"}, "fieldValueByName": null}]);
        let ours =
            json!([{"isArchived": false, "project": {"id": "PVT_1"}, "fieldValueByName": null}]);
        let archived_ours =
            json!([{"isArchived": true, "project": {"id": "PVT_1"}, "fieldValueByName": null}]);
        let on_two = RecordedGithub::load(&[])
            .with("Issue", vars(None), issue_page(foreign.clone(), true, Some("c1")))
            .with("Issue", vars(Some("c1")), issue_page(ours, false, None));
        let key = ("acme", "platform", 2);
        assert!(issue_on_board(&on_two, "PVT_1", Some("Sprint"), key).await.unwrap().is_some());
        let archived = RecordedGithub::load(&[]).with(
            "Issue",
            vars(None),
            issue_page(archived_ours, false, None),
        );
        assert!(issue_on_board(&archived, "PVT_1", Some("Sprint"), key).await.unwrap().is_none());
        let mut endless = RecordedGithub::load(&[]).with(
            "Issue",
            vars(None),
            issue_page(foreign.clone(), true, Some("c1")),
        );
        for n in 1..MAX_PROJECT_ITEM_PAGES {
            let (this, next) = (format!("c{n}"), format!("c{}", n + 1));
            endless = endless.with(
                "Issue",
                vars(Some(&this)),
                issue_page(foreign.clone(), true, Some(&next)),
            );
        }
        assert_eq!(
            issue_on_board(&endless, "PVT_1", Some("Sprint"), key).await,
            Err(TrackerError::HttpError)
        );
    }

    // ── E12 (reader part): get_ticket's read ────────────────────────────────

    #[tokio::test]
    async fn the_ticket_reader_returns_only_a_contained_issue() {
        use crate::tracker::ticket::{AcceptanceCriteriaSource, TicketReader};
        let gh = RecordedGithub::load(&["issue-on", "issue-off", "pr", "missing-repo"]);
        let reader = GithubTicketReader { transport: &gh, board: board(Some("Sprint")) };
        let t = reader.read_ticket("Acme/Platform#2").await.unwrap().unwrap();
        assert_eq!(t.key, "acme/platform#2");
        assert_eq!(t.title, "Issue 2");
        assert_eq!(t.url, "https://github.com/acme/platform/issues/2");
        assert_eq!(t.sprint.as_deref(), Some("Iteration 1"));
        assert_eq!((t.status.as_deref(), t.tracker), (Some("open"), TrackerKind::Github));
        assert_eq!(t.acceptance_criteria_source, AcceptanceCriteriaSource::NotConfigured);
        // Off the board, a pull request and a missing repository read alike.
        for key in ["acme/platform#6", "acme/platform#7", "acme/missing#8"] {
            assert_eq!(reader.read_ticket(key).await, Ok(None), "{key}");
        }
    }
}
