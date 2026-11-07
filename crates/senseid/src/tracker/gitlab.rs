//! The GitLab adapter: gitlab.com and self-managed, Free/CE up (spec
//! `docs/spec/daemon/tracker-gitlab.md`). A board is a group (with its subgroups) or
//! a project, keyed by GitLab's numeric id. Sprints are iterations where the
//! iterations probe answers 200, milestones otherwise. Keys and links are
//! [`crate::tracker::forge`]'s. History and write-back are unsupported (§G8).

use crate::tracker::http::normalize_base_url;
use crate::tracker::sprint::SprintState;
use crate::tracker::{
    AccountRef, ApiToken, AuthScheme, BoardScope, Capabilities, ConnectParams, Discovered,
    FieldCheck, FieldValue, GetTransport, KnownTask, Link, OwnerLookup, ParamError, PrevState,
    SessionText, Snapshot, SprintRow, TaskRow, TrackerAdapter, TrackerError, TrackerKind,
    ValidateError, Validated, WriteTransport,
};

pub struct GitlabAdapter;

const API: &str = "/api/v4";
const PAGE_SIZE: usize = 100;
/// Page 201 is read only to prove 200 full pages were the end (§G7).
const MAX_PAGES: u32 = 201;
/// The token header GitLab reads (§G2).
const TOKEN_HEADER: &str = "PRIVATE-TOKEN";

/// The host rule (§G5.1): gitlab.com, or any dotted name outside the reserved
/// local suffixes. The shared `normalize_base_url` already refused IP literals,
/// ports, userinfo, plain http and a trailing dot.
pub fn is_gitlab_host(host: &str) -> bool {
    const RESERVED: [&str; 5] = [".localhost", ".local", ".internal", ".lan", ".home.arpa"];
    let host = host.to_ascii_lowercase();
    host == "gitlab.com"
        || (host.contains('.')
            && host != "localhost"
            && !RESERVED.iter().any(|s| host.ends_with(s) || host == s[1..]))
}

/// The board path typed in a GitLab URL: a leading `groups/` dropped, everything
/// from the first `/-/` on dropped, lowercased. `None` when it is not a key path.
pub fn typed_path(board_url: &str) -> Option<String> {
    let url = reqwest::Url::parse(board_url.trim()).ok()?;
    let mut segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    if segments.first() == Some(&"groups") {
        segments.remove(0);
    }
    if let Some(dash) = segments.iter().position(|s| *s == "-") {
        segments.truncate(dash);
    }
    let path = segments.join("/").to_ascii_lowercase();
    crate::tracker::forge::is_key_path(&path).then_some(path)
}

/// A path as one URL segment (`acme/platform` → `acme%2Fplatform`). Key paths hold
/// only `[A-Za-z0-9_.\-/]`, so `/` is the one character to encode.
fn encoded(path: &str) -> String {
    path.replace('/', "%2F")
}

/// Which kind of namespace a board is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoardKind {
    Project,
    Group,
}

/// A board as GitLab names it: its kind and numeric id (the stored `board_ref`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BoardId {
    kind: BoardKind,
    id: u64,
}

impl BoardId {
    fn parse(board_ref: &str) -> Option<BoardId> {
        let (kind, id) = board_ref.split_once(':')?;
        let kind = match kind {
            "project" => BoardKind::Project,
            "group" => BoardKind::Group,
            _ => return None,
        };
        Some(BoardId { kind, id: id.parse().ok()? })
    }
    fn board_ref(self) -> String {
        format!("{}:{}", self.kind_str(), self.id)
    }
    fn kind_str(self) -> &'static str {
        match self.kind {
            BoardKind::Project => "project",
            BoardKind::Group => "group",
        }
    }
    /// The board's own API path, addressed by id (§G3).
    fn api(self) -> String {
        let plural = match self.kind {
            BoardKind::Project => "projects",
            BoardKind::Group => "groups",
        };
        format!("{API}/{plural}/{}", self.id)
    }
}

/// Where sprints come from (§G3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SprintSource {
    Iteration,
    Milestone,
}

impl SprintSource {
    pub fn as_str(self) -> &'static str {
        match self {
            SprintSource::Iteration => "iteration",
            SprintSource::Milestone => "milestone",
        }
    }
}

/// The iterations probe: 200 → iterations; 403 or 404 → milestones; anything
/// else fails, so a transient failure never picks the wrong source.
async fn sprint_source(t: &dyn GetTransport, board: BoardId) -> Result<SprintSource, TrackerError> {
    match t.get_json(&format!("{}/iterations", board.api()), &[("per_page", "1".into())]).await {
        Ok(_) => Ok(SprintSource::Iteration),
        Err(TrackerError::Forbidden | TrackerError::NotFound) => Ok(SprintSource::Milestone),
        Err(e) => Err(e),
    }
}

/// Every item of a listing, `per_page=100` pages until a short one (§G7).
async fn paged(
    t: &dyn GetTransport,
    path: &str,
    query: &[(&str, &str)],
) -> Result<Vec<serde_json::Value>, TrackerError> {
    let mut out = Vec::new();
    for page in 1..=MAX_PAGES {
        let mut q: Vec<(&str, String)> = query.iter().map(|(k, v)| (*k, v.to_string())).collect();
        q.push(("per_page", PAGE_SIZE.to_string()));
        q.push(("page", page.to_string()));
        let body = t.get_json(path, &q).await?;
        let items = body.as_array().ok_or(TrackerError::Decode)?;
        if page == MAX_PAGES && !items.is_empty() {
            // More than 200 full pages: never return a partial board as whole.
            return Err(TrackerError::HttpError);
        }
        out.extend(items.iter().cloned());
        if items.len() < PAGE_SIZE {
            return Ok(out);
        }
    }
    Ok(out)
}

fn text<'a>(v: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(|x| x.as_str())
}

/// A `YYYY-MM-DD` field: `None` when absent or null, `Decode` when malformed.
fn date(v: &serde_json::Value, key: &str) -> Result<Option<chrono::NaiveDate>, TrackerError> {
    match v.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| TrackerError::Decode),
        Some(_) => Err(TrackerError::Decode),
    }
}

fn midnight(d: chrono::NaiveDate) -> chrono::DateTime<chrono::Utc> {
    d.and_hms_opt(0, 0, 0).expect("midnight exists").and_utc()
}

/// `start_at` and `end_at` (§G4.2): the due date is inclusive, the window's end
/// exclusive, so the end is the day after it.
/// A sprint's `(start_at, end_at)`.
type Window = (Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>);

fn window(v: &serde_json::Value) -> Result<Window, TrackerError> {
    let start = date(v, "start_date")?.map(midnight);
    let end = date(v, "due_date")?.map(|d| midnight(d + chrono::Duration::days(1)));
    Ok((start, end))
}

fn id(v: &serde_json::Value) -> Result<u64, TrackerError> {
    v.get("id").and_then(|x| x.as_u64()).ok_or(TrackerError::Decode)
}

/// A milestone as a sprint (§G4.2). `today` is the UTC date the state is judged on.
fn milestone_row(
    v: &serde_json::Value,
    today: chrono::NaiveDate,
) -> Result<SprintRow, TrackerError> {
    let id = id(v)?;
    let (start_at, end_at) = window(v)?;
    let state = match text(v, "state") {
        Some("closed") => SprintState::Closed,
        Some("active") if date(v, "start_date")?.is_some_and(|s| s > today) => SprintState::Future,
        Some("active") => SprintState::Active,
        _ => return Err(TrackerError::Decode),
    };
    Ok(SprintRow {
        external_id: format!("milestone:{id}"),
        seq: id as i64,
        name: text(v, "title").ok_or(TrackerError::Decode)?.to_string(),
        state,
        start_at,
        end_at,
        complete_at: None,
    })
}

/// An iteration as a sprint (§G4.2). Its `state` is an integer.
fn iteration_row(v: &serde_json::Value) -> Result<SprintRow, TrackerError> {
    let id = id(v)?;
    let (start_at, end_at) = window(v)?;
    let state = match v.get("state").and_then(|s| s.as_u64()) {
        Some(1) => SprintState::Future,
        Some(2) => SprintState::Active,
        Some(3) => SprintState::Closed,
        _ => return Err(TrackerError::Decode),
    };
    // An untitled iteration is shown by its dates, as GitLab shows it.
    let name = match text(v, "title") {
        Some(t) => t.to_string(),
        None => format!(
            "{} – {}",
            text(v, "start_date").ok_or(TrackerError::Decode)?,
            text(v, "due_date").ok_or(TrackerError::Decode)?
        ),
    };
    Ok(SprintRow {
        external_id: format!("iteration:{id}"),
        seq: id as i64,
        name,
        state,
        start_at,
        end_at,
        complete_at: None,
    })
}

fn timestamp(
    v: &serde_json::Value,
    key: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, TrackerError> {
    match v.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => chrono::DateTime::parse_from_rfc3339(s)
            .map(|t| Some(t.with_timezone(&chrono::Utc)))
            .map_err(|_| TrackerError::Decode),
        Some(_) => Err(TrackerError::Decode),
    }
}

/// An issue as a ticket (§G4.3). `base_url` is the tracker's origin.
fn task_row(v: &serde_json::Value, base_url: &str) -> Result<TaskRow, TrackerError> {
    let full =
        v.pointer("/references/full").and_then(|x| x.as_str()).ok_or(TrackerError::Decode)?;
    let (path, n) = crate::tracker::forge::parse_key(full).ok_or(TrackerError::Decode)?;
    let scope = crate::db::pg_store::normalize_repo_key(&format!("{base_url}/{path}"))
        .ok_or(TrackerError::Decode)?;
    let state = text(v, "state").map(String::from);
    let status_category = match state.as_deref() {
        Some("opened") => Some("new".to_string()),
        Some("closed") => Some("done".to_string()),
        _ => None,
    };
    let assignee = v
        .pointer("/assignees/0/id")
        .and_then(|x| x.as_u64())
        .map(|id| AccountRef::new(id.to_string()));
    Ok(TaskRow {
        external_id: id(v)?.to_string(),
        key: format!("{path}#{n}"),
        scope: Some(scope),
        summary: text(v, "title").map(String::from),
        issue_type: text(v, "issue_type").map(String::from),
        status: state,
        assignee,
        created_at: timestamp(v, "created_at")?,
        updated_at: timestamp(v, "updated_at")?,
        status_category,
        // Not asked: estimates are unsupported (§G8).
        points: None,
    })
}

/// The sprint `external_id` an issue currently sits in, for `source`.
fn sprint_of(v: &serde_json::Value, source: SprintSource) -> Option<String> {
    let field = source.as_str();
    let id = v.get(field)?.get("id")?.as_u64()?;
    Some(format!("{field}:{id}"))
}

/// The token's owner (§G8): a bot is not a person.
fn owner_from_user(v: &serde_json::Value) -> Result<OwnerLookup, TrackerError> {
    if v.get("bot").and_then(|b| b.as_bool()) == Some(true) {
        return Ok(OwnerLookup::NotAPerson);
    }
    Ok(OwnerLookup::Person(AccountRef::new(id(v)?.to_string())))
}

/// A board's name and current full path.
async fn board_info(
    t: &dyn GetTransport,
    board: BoardId,
) -> Result<(String, String), TrackerError> {
    let v = t.get_json(&board.api(), &[]).await?;
    let name = text(&v, "name").ok_or(TrackerError::Decode)?.to_string();
    let path = match board.kind {
        BoardKind::Project => text(&v, "path_with_namespace"),
        BoardKind::Group => text(&v, "full_path"),
    };
    Ok((name, path.ok_or(TrackerError::Decode)?.to_ascii_lowercase()))
}

/// The settings a snapshot or a validate stores (§G4.6).
fn board_params(
    full_path: &str,
    version: &str,
    source: SprintSource,
) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert("full_path".into(), full_path.into());
    m.insert("version".into(), version.into());
    m.insert("sprint_source".into(), source.as_str().into());
    m
}

async fn version(t: &dyn GetTransport) -> Result<String, TrackerError> {
    let v = t.get_json(&format!("{API}/version"), &[]).await?;
    text(&v, "version").map(String::from).ok_or(TrackerError::Decode)
}

/// Whether a key's project path is inside the board (§G6): the board's own path for
/// a project board; the group's path or below it for a group board.
fn path_in_board(path: &str, board: BoardId, board_path: &str) -> bool {
    match board.kind {
        BoardKind::Project => path == board_path,
        BoardKind::Group => path == board_path || path.starts_with(&format!("{board_path}/")),
    }
}

#[async_trait::async_trait]
impl TrackerAdapter for GitlabAdapter {
    fn kind(&self) -> TrackerKind {
        TrackerKind::Gitlab
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { history: false, writeback: false }
    }

    fn connect_params(&self, body: &serde_json::Value) -> Result<ConnectParams, ParamError> {
        let text =
            |k: &str| body.get(k).and_then(|v| v.as_str()).map(str::trim).filter(|v| !v.is_empty());
        let board_url = text("board_url").ok_or(ParamError::InvalidBaseUrl)?;
        let base_url = normalize_base_url(board_url, is_gitlab_host)
            .map_err(|_| ParamError::InvalidBaseUrl)?;
        let path = typed_path(board_url).ok_or(ParamError::InvalidBaseUrl)?;
        let host = base_url.trim_start_matches("https://");
        let confirmed = match text("confirm_host") {
            Some(c) => c.eq_ignore_ascii_case(host),
            None => host == "gitlab.com",
        };
        if !confirmed {
            return Err(ParamError::HostNotConfirmed);
        }
        let token = text("api_token").ok_or(ParamError::TokenRequired)?.to_string();
        // Both provisional: validate replaces them with the board's id and the
        // token's username (§G1).
        Ok(ConnectParams {
            base_url,
            board_ref: path,
            account: String::new(),
            token: ApiToken(token),
            project_key: None,
        })
    }

    fn auth(&self, _account: &str, token: &ApiToken) -> AuthScheme {
        AuthScheme::Header { name: TOKEN_HEADER, token: ApiToken(token.0.clone()) }
    }

    async fn owner(&self, t: &dyn GetTransport) -> Result<OwnerLookup, TrackerError> {
        owner_from_user(&t.get_json(&format!("{API}/user"), &[]).await?)
    }

    async fn validate(
        &self,
        t: &dyn GetTransport,
        p: &ConnectParams,
    ) -> Result<Validated, ValidateError> {
        let user = t.get_json(&format!("{API}/user"), &[]).await.map_err(ValidateError::Tracker)?;
        let owner = owner_from_user(&user).map_err(ValidateError::Tracker)?;
        let account = text(&user, "username")
            .ok_or(ValidateError::Tracker(TrackerError::Decode))?
            .to_string();
        let path = encoded(&p.board_ref);
        let board = match t.get_json(&format!("{API}/projects/{path}"), &[]).await {
            Ok(v) => (BoardKind::Project, v),
            Err(TrackerError::NotFound) => {
                match t.get_json(&format!("{API}/groups/{path}"), &[]).await {
                    Ok(v) => (BoardKind::Group, v),
                    Err(TrackerError::NotFound) => return Err(ValidateError::BoardNotFound),
                    Err(e) => return Err(ValidateError::Tracker(e)),
                }
            }
            Err(e) => return Err(ValidateError::Tracker(e)),
        };
        let decode = ValidateError::Tracker(TrackerError::Decode);
        let (kind, v) = board;
        let board_id = BoardId { kind, id: id(&v).map_err(|_| decode)? };
        let name = text(&v, "name").ok_or(decode)?.to_string();
        let full_path = match kind {
            BoardKind::Project => text(&v, "path_with_namespace"),
            BoardKind::Group => text(&v, "full_path"),
        }
        .ok_or(decode)?
        .to_ascii_lowercase();
        let version = version(t).await.map_err(ValidateError::Tracker)?;
        let source = sprint_source(t, board_id).await.map_err(ValidateError::Tracker)?;
        Ok(Validated {
            external_scope: full_path.clone(),
            board_name: name,
            owner,
            account: Some(account),
            board_ref: Some(board_id.board_ref()),
            params: board_params(&full_path, &version, source),
        })
    }

    async fn fetch_snapshot(
        &self,
        t: &dyn GetTransport,
        board: &BoardScope,
        _prev: &PrevState,
    ) -> Result<Snapshot, TrackerError> {
        let id = BoardId::parse(&board.board_ref).ok_or(TrackerError::Decode)?;
        let (name, full_path) = board_info(t, id).await?;
        let version = version(t).await?;
        let source = sprint_source(t, id).await?;
        let sprints: Vec<SprintRow> = match source {
            SprintSource::Iteration => paged(
                t,
                &format!("{}/iterations", id.api()),
                &[("state", "all"), ("include_ancestors", "true")],
            )
            .await?
            .iter()
            .map(iteration_row)
            .collect::<Result<_, _>>()?,
            SprintSource::Milestone => {
                let mut q = vec![("state", "all"), ("include_ancestors", "true")];
                if id.kind == BoardKind::Group {
                    q.push(("include_descendants", "true"));
                }
                let today = chrono::Utc::now().date_naive();
                let mut seen = std::collections::HashSet::new();
                let mut rows = Vec::new();
                for m in paged(t, &format!("{}/milestones", id.api()), &q).await? {
                    if seen.insert(self::id(&m)?) {
                        rows.push(milestone_row(&m, today)?);
                    }
                }
                rows
            }
        };
        // A group's issues need `scope=all`; a project's are all of them already.
        let issue_query: &[(&str, &str)] = match id.kind {
            BoardKind::Group => &[("scope", "all"), ("state", "all")],
            BoardKind::Project => &[("state", "all")],
        };
        let issues = paged(t, &format!("{}/issues", id.api()), issue_query).await?;
        let mut tasks = Vec::with_capacity(issues.len());
        let mut members: std::collections::BTreeMap<String, Vec<String>> =
            sprints.iter().map(|s| (s.external_id.clone(), Vec::new())).collect();
        for issue in &issues {
            let task = task_row(issue, &board.base_url)?;
            if let Some(sprint) = sprint_of(issue, source)
                && let Some(ids) = members.get_mut(&sprint)
            {
                ids.push(task.external_id.clone());
            }
            tasks.push(task);
        }
        Ok(Snapshot {
            board_name: Some(name),
            sprints,
            tasks,
            members: members.into_iter().map(|(s, ids)| (s, Some(ids))).collect(),
            points_field: None,
            params_extra: board_params(&full_path, &version, source),
        })
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
        let id = BoardId::parse(&board.board_ref).ok_or(TrackerError::Decode)?;
        let (_, board_path) = board_info(t, id).await?;
        let source = sprint_source(t, id).await?;
        let mut found = Discovered::default();
        for key in keys {
            // A key outside the board is never fetched (§G6).
            let Some((path, iid)) = crate::tracker::forge::parse_key(key) else { continue };
            if !path_in_board(&path, id, &board_path) {
                continue;
            }
            let issue = match t
                .get_json(&format!("{API}/projects/{}/issues/{iid}", encoded(&path)), &[])
                .await
            {
                Ok(v) => v,
                Err(TrackerError::NotFound) => continue,
                Err(e) => return Err(e),
            };
            let in_board = match id.kind {
                BoardKind::Project => {
                    issue.get("project_id").and_then(|p| p.as_u64()) == Some(id.id)
                }
                BoardKind::Group => true, // its path was checked above
            };
            if !in_board {
                continue;
            }
            let task = task_row(&issue, &board.base_url)?;
            if let Some(sprint) = sprint_of(&issue, source) {
                found.members.push((sprint, task.external_id.clone()));
            }
            found.tasks.push(task);
        }
        Ok(found)
    }

    // ── Write-back: unsupported (§G8), unreachable behind the capability guards ──

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

#[cfg(test)]
pub(crate) mod test_support {
    use crate::tracker::{GetTransport, TrackerError};
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// One recorded or scripted answer.
    enum Answer {
        Status(u16, serde_json::Value),
        Fail(TrackerError),
    }

    /// A GitLab served from a recorded fixture set
    /// (`crates/senseid/tests/fixtures/gitlab/<set>`, recorded by
    /// `scripts/record-gitlab-fixtures.py`). A request with no recording panics, so a
    /// test can never pass on a guessed response. `group.json` and `project.json`
    /// also answer by the board's numeric id, as the adapter addresses boards after
    /// connect.
    pub struct RecordedGitlab {
        answers: HashMap<String, Answer>,
        pub calls: Mutex<Vec<String>>,
    }

    /// `path?k=v&…` with the query sorted, and `page=1` where `per_page` is given
    /// without a page.
    fn request_key(path: &str, query: &[(String, String)]) -> String {
        let mut q: Vec<(String, String)> = query.to_vec();
        if q.iter().any(|(k, _)| k == "per_page") && !q.iter().any(|(k, _)| k == "page") {
            q.push(("page".into(), "1".into()));
        }
        q.sort();
        let q: Vec<String> = q.into_iter().map(|(k, v)| format!("{k}={v}")).collect();
        if q.is_empty() { path.to_string() } else { format!("{path}?{}", q.join("&")) }
    }

    fn parse_request(request: &str) -> String {
        let (path, query) = request.split_once('?').unwrap_or((request, ""));
        let pairs: Vec<(String, String)> = query
            .split('&')
            .filter(|p| !p.is_empty())
            .map(|p| {
                let (k, v) = p.split_once('=').unwrap_or((p, ""));
                (k.to_string(), v.to_string())
            })
            .collect();
        request_key(path, &pairs)
    }

    pub fn fixture_dir(set: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gitlab").join(set)
    }

    /// A recorded response's body, by file name (`group_milestones`).
    pub fn body(set: &str, name: &str) -> serde_json::Value {
        let raw = std::fs::read_to_string(fixture_dir(set).join(format!("{name}.json"))).unwrap();
        serde_json::from_str::<serde_json::Value>(&raw).unwrap()["body"].clone()
    }

    impl RecordedGitlab {
        pub fn load(set: &str) -> Self {
            let mut answers = HashMap::new();
            for entry in std::fs::read_dir(fixture_dir(set)).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let doc: serde_json::Value =
                    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
                let status = doc["status"].as_u64().unwrap() as u16;
                let answer = || Answer::Status(status, doc["body"].clone());
                answers.insert(parse_request(doc["request"].as_str().unwrap()), answer());
                let stem = path.file_stem().unwrap().to_str().unwrap();
                if let (true, Some(id)) =
                    (matches!(stem, "group" | "project"), doc["body"]["id"].as_u64())
                {
                    answers.insert(format!("{stem}s/{id}"), answer());
                }
            }
            RecordedGitlab { answers, calls: Mutex::new(Vec::new()) }
        }

        /// Answer `request` (`groups/857/iterations?per_page=1`) with a status: for
        /// statuses a recording cannot show (a 403, a 500), never for a body shape.
        pub fn with(mut self, request: &str, status: u16, body: serde_json::Value) -> Self {
            self.answers.insert(parse_request(request), Answer::Status(status, body));
            self
        }

        /// Fail `request` with `e` (a timeout is `Transport`).
        pub fn failing(mut self, request: &str, e: TrackerError) -> Self {
            self.answers.insert(parse_request(request), Answer::Fail(e));
            self
        }

        pub fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl GetTransport for RecordedGitlab {
        async fn get_json(
            &self,
            path: &str,
            query: &[(&str, String)],
        ) -> Result<serde_json::Value, TrackerError> {
            let path = path.strip_prefix("/api/v4/").expect("an /api/v4 path");
            let pairs: Vec<(String, String)> =
                query.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
            let request = request_key(path, &pairs);
            self.calls.lock().unwrap().push(request.clone());
            match self.answers.get(&request) {
                Some(Answer::Status(status, body)) => crate::tracker::transport::map_response(
                    &crate::tracker::transport::RawResponse {
                        status: *status,
                        retry_after: None,
                        body: serde_json::to_vec(body).unwrap(),
                    },
                ),
                Some(Answer::Fail(e)) => Err(*e),
                None => panic!("no recording for {request}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{RecordedGitlab, body};
    use super::*;

    const BASE: &str = "https://gitlab.example.com";

    fn group_board() -> BoardScope {
        BoardScope { base_url: BASE.into(), board_ref: "group:857".into() }
    }

    // ── connect_params (D2) ─────────────────────────────────────────────────

    fn connect(board_url: &str, confirm: Option<&str>) -> Result<ConnectParams, ParamError> {
        let mut body = serde_json::json!({"kind": "gitlab", "level": "sprint", "board_url": board_url, "api_token": "glpat-x"});
        if let Some(c) = confirm {
            body["confirm_host"] = c.into();
        }
        GitlabAdapter.connect_params(&body)
    }

    #[test]
    fn a_board_url_names_its_group_or_project_path() {
        let p =
            connect("https://gitlab.example.com/groups/Acme/-/boards", Some("gitlab.example.com"))
                .unwrap();
        assert_eq!((p.base_url.as_str(), p.board_ref.as_str()), (BASE, "acme"));
        let p = connect(
            "https://gitlab.example.com/acme/platform/-/issues",
            Some("GitLab.Example.com"),
        )
        .unwrap();
        assert_eq!(p.board_ref, "acme/platform");
        assert_eq!(p.account, "", "provisional until validate");
        let p = connect("https://gitlab.com/acme/platform", None).unwrap();
        assert_eq!(p.base_url, "https://gitlab.com", "gitlab.com needs no confirmation");
    }

    #[test]
    fn a_self_managed_host_must_be_confirmed() {
        assert_eq!(
            connect("https://gitlab.example.com/acme", None),
            Err(ParamError::HostNotConfirmed)
        );
        assert_eq!(
            connect("https://gitlab.example.com/acme", Some("gitlab.evil.com")),
            Err(ParamError::HostNotConfirmed)
        );
        assert_eq!(ParamError::HostNotConfirmed.code(), "host_not_confirmed");
    }

    #[test]
    fn unsafe_or_local_urls_are_refused() {
        for url in [
            "https://1.2.3.4/acme",
            "https://gitlab.example.com:8443/acme",
            "http://gitlab.example.com/acme",
            "https://localhost/acme",
            "https://x.internal/acme",
            "https://git.local/acme",
            "https://localhost./acme",
            "https://gitlab.example.com/",
        ] {
            assert_eq!(connect(url, Some("x")), Err(ParamError::InvalidBaseUrl), "{url}");
        }
        // A `..` in a URL is resolved by URL parsing before the path is read.
        let p =
            connect("https://gitlab.example.com/acme/../x", Some("gitlab.example.com")).unwrap();
        assert_eq!(p.board_ref, "x");
        let mut no_token = serde_json::json!({"board_url": "https://gitlab.com/acme"});
        assert_eq!(GitlabAdapter.connect_params(&no_token), Err(ParamError::TokenRequired));
        no_token["api_token"] = "  ".into();
        assert_eq!(GitlabAdapter.connect_params(&no_token), Err(ParamError::TokenRequired));
    }

    #[test]
    fn the_token_goes_in_its_own_header() {
        let auth = GitlabAdapter.auth("", &ApiToken("glpat-x".into()));
        assert_eq!(
            auth,
            AuthScheme::Header { name: "PRIVATE-TOKEN", token: ApiToken("glpat-x".into()) }
        );
    }

    // ── validate (D3) and the probe (D5), on the recorded CE ────────────────

    fn params(path: &str) -> ConnectParams {
        ConnectParams {
            base_url: BASE.into(),
            board_ref: path.into(),
            account: String::new(),
            token: ApiToken("glpat-x".into()),
            project_key: None,
        }
    }

    #[tokio::test]
    async fn a_group_path_validates_to_the_group_by_id() {
        let gl = RecordedGitlab::load("ce-closed-milestone");
        let v = GitlabAdapter.validate(&gl, &params("acme")).await.unwrap();
        assert_eq!(v.board_ref.as_deref(), Some("group:857"));
        assert_eq!(v.account.as_deref(), Some("group_bot"));
        assert_eq!(v.owner, OwnerLookup::NotAPerson, "a group token is a bot");
        assert_eq!(v.external_scope, "acme");
        assert_eq!(v.params["sprint_source"], "milestone", "CE answers the probe 404");
        assert_eq!(v.params["version"], "19.5.3");
        assert_eq!(v.params["full_path"], "acme");
        let calls = gl.calls();
        assert!(calls.len() <= 5, "{calls:?}");
        assert_eq!(calls[1], "projects/acme", "a path is tried as a project first");
    }

    #[tokio::test]
    async fn a_project_path_validates_to_the_project_by_id() {
        let gl = RecordedGitlab::load("ce-closed-milestone").with(
            "projects/424/iterations?per_page=1",
            404,
            serde_json::json!({}),
        );
        let v = GitlabAdapter.validate(&gl, &params("acme/backend")).await.unwrap();
        assert_eq!(v.board_ref.as_deref(), Some("project:424"));
        assert_eq!(v.external_scope, "acme/backend");
        assert_eq!(gl.calls()[1], "projects/acme%2Fbackend", "the path is one encoded segment");
    }

    #[tokio::test]
    async fn an_unknown_path_is_board_not_found() {
        let gl = RecordedGitlab::load("ce-closed-milestone")
            .with("projects/nope", 404, serde_json::json!({}))
            .with("groups/nope", 404, serde_json::json!({}));
        assert_eq!(
            GitlabAdapter.validate(&gl, &params("nope")).await,
            Err(ValidateError::BoardNotFound)
        );
    }

    #[tokio::test]
    async fn the_probe_decides_only_on_200_403_and_404() {
        let probe = "groups/857/iterations?per_page=1";
        let board = BoardId { kind: BoardKind::Group, id: 857 };
        let gl = RecordedGitlab::load("ce-closed-milestone");
        assert_eq!(sprint_source(&gl, board).await, Ok(SprintSource::Milestone), "recorded 404");
        let gl =
            RecordedGitlab::load("ce-closed-milestone").with(probe, 403, serde_json::json!({}));
        assert_eq!(sprint_source(&gl, board).await, Ok(SprintSource::Milestone));
        let gl =
            RecordedGitlab::load("ce-closed-milestone").with(probe, 200, serde_json::json!([]));
        assert_eq!(sprint_source(&gl, board).await, Ok(SprintSource::Iteration));
        let gl =
            RecordedGitlab::load("ce-closed-milestone").with(probe, 500, serde_json::json!({}));
        assert_eq!(sprint_source(&gl, board).await, Err(TrackerError::HttpError));
        let gl =
            RecordedGitlab::load("ce-closed-milestone").failing(probe, TrackerError::Transport);
        assert_eq!(sprint_source(&gl, board).await, Err(TrackerError::Transport), "a timeout");
    }

    #[tokio::test]
    async fn a_failed_probe_fails_the_snapshot() {
        let gl = RecordedGitlab::load("ce-closed-milestone")
            .failing("groups/857/iterations?per_page=1", TrackerError::Transport);
        let r = GitlabAdapter.fetch_snapshot(&gl, &group_board(), &PrevState::default()).await;
        assert_eq!(r, Err(TrackerError::Transport));
    }

    // ── snapshot (D6, D7, D9) ───────────────────────────────────────────────

    #[tokio::test]
    async fn the_recorded_group_snapshot_maps_its_milestone_and_issues() {
        let gl = RecordedGitlab::load("ce-closed-milestone");
        let s =
            GitlabAdapter.fetch_snapshot(&gl, &group_board(), &PrevState::default()).await.unwrap();
        assert_eq!(s.board_name.as_deref(), Some("acme"));
        assert_eq!(s.sprints.len(), 1);
        let sprint = &s.sprints[0];
        assert_eq!(
            (sprint.external_id.as_str(), sprint.seq, sprint.state),
            ("milestone:214", 214, SprintState::Closed)
        );
        assert_eq!(sprint.start_at.unwrap().to_rfc3339(), "2026-05-26T00:00:00+00:00");
        assert_eq!(
            sprint.end_at.unwrap().to_rfc3339(),
            "2026-06-15T00:00:00+00:00",
            "due date inclusive"
        );
        assert_eq!(s.tasks.len(), 52, "merge requests are not issues (C26)");
        assert_eq!(s.members.len(), 1);
        assert_eq!(s.members[0].0, "milestone:214");
        assert_eq!(s.members[0].1.as_ref().unwrap().len(), 32);
        let t44 = s
            .tasks
            .iter()
            .find(|t| t.key == "acme/backend#44")
            .expect("keyed by the full reference");
        assert_eq!(
            t44.scope,
            crate::db::pg_store::normalize_repo_key("git@gitlab.example.com:acme/backend.git")
        );
        assert_eq!(
            (t44.status.as_deref(), t44.status_category.as_deref()),
            (Some("opened"), Some("new"))
        );
        assert_eq!(t44.points, None, "estimates are not asked");
        assert!(s.tasks.iter().all(|t| t.issue_type.as_deref() == Some("issue")));
        assert_eq!(s.points_field, None);
        assert_eq!(s.params_extra["sprint_source"], "milestone");
        assert_eq!(s.params_extra["full_path"], "acme");
    }

    #[tokio::test]
    async fn a_board_with_no_milestones_has_no_sprints_and_every_ticket() {
        // D9: nothing is synthesised.
        let gl = RecordedGitlab::load("ce-no-milestones");
        let s =
            GitlabAdapter.fetch_snapshot(&gl, &group_board(), &PrevState::default()).await.unwrap();
        assert!(s.sprints.is_empty() && s.members.is_empty());
        assert_eq!(s.tasks.len(), 52);
    }

    #[tokio::test]
    async fn a_project_board_sees_its_groups_milestone() {
        let gl = RecordedGitlab::load("ce-closed-milestone").with(
            "projects/424/iterations?per_page=1",
            404,
            serde_json::json!({}),
        );
        let board = BoardScope { base_url: BASE.into(), board_ref: "project:424".into() };
        let s = GitlabAdapter.fetch_snapshot(&gl, &board, &PrevState::default()).await.unwrap();
        assert_eq!(
            s.sprints.iter().map(|s| s.external_id.as_str()).collect::<Vec<_>>(),
            ["milestone:214"]
        );
        assert_eq!(s.tasks.len(), 44);
        assert!(s.tasks.iter().all(|t| t.key.starts_with("acme/backend#")));
    }

    #[test]
    fn a_milestone_state_follows_its_dates() {
        let recorded = body("ce-closed-milestone", "group_milestones")[0].clone();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        assert_eq!(milestone_row(&recorded, today).unwrap().state, SprintState::Closed);
        let mut m = recorded.clone();
        m["state"] = "active".into();
        assert_eq!(milestone_row(&m, today).unwrap().state, SprintState::Active, "in its window");
        let before = chrono::NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        assert_eq!(milestone_row(&m, before).unwrap().state, SprintState::Future);
        let after = chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        assert_eq!(
            milestone_row(&m, after).unwrap().state,
            SprintState::Active,
            "an overrun stays active"
        );
        m["start_date"] = serde_json::Value::Null;
        m["due_date"] = serde_json::Value::Null;
        let undated = milestone_row(&m, today).unwrap();
        assert_eq!(
            (undated.start_at, undated.end_at, undated.state),
            (None, None, SprintState::Active),
            "kept, undated"
        );
        m["state"] = "paused".into();
        assert_eq!(milestone_row(&m, today), Err(TrackerError::Decode));
    }

    #[test]
    fn an_iteration_state_is_an_integer() {
        // No Premium recording yet: the shape is GitLab's documented one (§G4.2).
        let it = |state: serde_json::Value, title: serde_json::Value| {
            iteration_row(
                &serde_json::json!({"id": 53, "state": state, "title": title, "start_date": "2026-11-02", "due_date": "2026-11-13"}),
            )
        };
        let row = it(1.into(), "Sprint 9".into()).unwrap();
        assert_eq!(
            (row.external_id.as_str(), row.state, row.name.as_str()),
            ("iteration:53", SprintState::Future, "Sprint 9")
        );
        assert_eq!(it(2.into(), "x".into()).unwrap().state, SprintState::Active);
        assert_eq!(it(3.into(), "x".into()).unwrap().state, SprintState::Closed);
        assert_eq!(
            it("current".into(), "x".into()),
            Err(TrackerError::Decode),
            "a string is not a state"
        );
        assert_eq!(it(1.into(), serde_json::Value::Null).unwrap().name, "2026-11-02 – 2026-11-13");
        assert_eq!(
            it(1.into(), "x".into()).unwrap().end_at.unwrap().to_rfc3339(),
            "2026-11-14T00:00:00+00:00"
        );
    }

    #[test]
    fn a_closed_issue_is_done_and_its_assignee_is_by_id() {
        let mut issue = body("ce-closed-milestone", "project_issue_one");
        issue["state"] = "closed".into();
        let row = task_row(&issue, BASE).unwrap();
        assert_eq!(row.status_category.as_deref(), Some("done"));
        assert_eq!(row.assignee, Some(AccountRef::new("2")));
        issue["references"]["full"] = "Acme/Backend#44".into();
        assert_eq!(
            task_row(&issue, BASE).unwrap().key,
            "acme/backend#44",
            "the path is lowercased"
        );
        issue["references"] = serde_json::Value::Null;
        assert_eq!(task_row(&issue, BASE), Err(TrackerError::Decode));
    }

    #[test]
    fn a_bot_is_not_a_person() {
        assert_eq!(
            owner_from_user(&body("ce-closed-milestone", "user")),
            Ok(OwnerLookup::NotAPerson)
        );
        assert_eq!(
            owner_from_user(&serde_json::json!({"id": 7, "username": "dev", "bot": false})),
            Ok(OwnerLookup::Person(AccountRef::new("7")))
        );
    }

    // ── pagination (D15) ────────────────────────────────────────────────────

    struct Pages {
        page_201: usize,
    }

    #[async_trait::async_trait]
    impl GetTransport for Pages {
        async fn get_json(
            &self,
            _p: &str,
            q: &[(&str, String)],
        ) -> Result<serde_json::Value, TrackerError> {
            let page: u32 = q.iter().find(|(k, _)| *k == "page").unwrap().1.parse().unwrap();
            let n = if page <= 200 { PAGE_SIZE } else { self.page_201 };
            Ok(serde_json::Value::Array(vec![serde_json::json!({}); n]))
        }
    }

    #[tokio::test]
    async fn two_hundred_full_pages_and_an_empty_one_is_the_whole_listing() {
        assert_eq!(paged(&Pages { page_201: 0 }, "/x", &[]).await.unwrap().len(), 20_000);
        assert_eq!(paged(&Pages { page_201: 1 }, "/x", &[]).await, Err(TrackerError::HttpError));
    }

    // ── discovery (§G6) ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn discovery_fetches_only_keys_inside_the_board() {
        let issue = body("ce-closed-milestone", "project_issue_one");
        let gl = RecordedGitlab::load("ce-closed-milestone")
            .with("projects/acme%2Fbackend/issues/44", 200, issue)
            .with("projects/acme%2Fbackend/issues/999", 404, serde_json::json!({}));
        let keys = [
            "acme/backend#44".to_string(),
            "acme/backend#999".into(),
            "other/repo#1".into(),
            "acme/../x#1".into(),
        ];
        let found = GitlabAdapter.fetch_tasks(&gl, &group_board(), &keys).await.unwrap();
        assert_eq!(
            found.tasks.iter().map(|t| t.key.as_str()).collect::<Vec<_>>(),
            ["acme/backend#44"]
        );
        assert!(
            !gl.calls().iter().any(|c| c.contains("other") || c.contains("..")),
            "{:?}",
            gl.calls()
        );
    }

    #[test]
    fn the_host_rule() {
        for ok in ["gitlab.com", "gitlab.example.com", "git.acme.io"] {
            assert!(is_gitlab_host(ok), "{ok}");
        }
        for bad in [
            "localhost",
            "gitlab",
            "a.localhost",
            "git.local",
            "x.internal",
            "box.lan",
            "nas.home.arpa",
        ] {
            assert!(!is_gitlab_host(bad), "{bad}");
        }
    }
}
