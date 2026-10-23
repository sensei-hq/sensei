//! Issue-tracker integrations: Jira first, behind a provider-neutral seam so
//! another tracker (GitLab) is one more adapter, not a rewrite.
//!
//! Spec: `docs/spec/daemon/tracker-jira-linkage.md`. Provider knowledge lives in
//! the adapter modules (`jira`); everything else here is provider-neutral, and
//! every SQL statement lives in `db::pg_store::trackers`.

pub mod connect;
pub mod discover;
pub mod http;
pub mod jira;
pub mod link;
pub mod runtime;
pub mod secret;
pub mod sprint;
pub mod status;
pub mod sync;
pub mod transport;
pub mod usage;

use std::collections::HashMap;

/// Which tracker a connection speaks to — the `sensei.tracker_kind` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackerKind {
    Jira,
}

impl TrackerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TrackerKind::Jira => "jira",
        }
    }
    pub fn parse(s: &str) -> Option<TrackerKind> {
        match s {
            "jira" => Some(TrackerKind::Jira),
            _ => None,
        }
    }
}

/// What a tracker's board serves in its workspace — the `sensei.tracker_level`
/// enum (D40, `docs/spec/tracker-levels.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackerLevel {
    Epic,
    Sprint,
    Task,
}

impl TrackerLevel {
    /// In the enum's declared order, which is also the order trackers list in.
    pub const ALL: [TrackerLevel; 3] =
        [TrackerLevel::Epic, TrackerLevel::Sprint, TrackerLevel::Task];

    pub fn as_str(self) -> &'static str {
        match self {
            TrackerLevel::Epic => "epic",
            TrackerLevel::Sprint => "sprint",
            TrackerLevel::Task => "task",
        }
    }
    pub fn parse(s: &str) -> Option<TrackerLevel> {
        TrackerLevel::ALL.into_iter().find(|l| l.as_str() == s)
    }
}

/// An API token. Its `Debug` never prints the value, so a token cannot reach a
/// log line through `{:?}` on any struct that carries it.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiToken(pub String);

impl std::fmt::Debug for ApiToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiToken(***)")
    }
}

/// How the shared transport authenticates, chosen by the adapter.
#[derive(Clone, PartialEq, Eq)]
pub enum AuthScheme {
    Basic { user: String, token: ApiToken },
}

impl std::fmt::Debug for AuthScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthScheme::Basic { user, .. } => write!(f, "Basic({user}, ***)"),
        }
    }
}

/// GET-only access to a tracker's API. One method, so no other verb can be sent.
#[async_trait::async_trait]
pub trait GetTransport: Send + Sync {
    async fn get_json(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<serde_json::Value, TrackerError>;
}

/// Validated connect input, in canonical form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectParams {
    pub base_url: String,
    pub board_ref: String,
    pub account: String,
    pub token: ApiToken,
    pub project_key: Option<String>,
}

/// Why connect input was refused before any network call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamError {
    InvalidBaseUrl,
    EmailRequired,
    TokenRequired,
}

impl ParamError {
    pub fn code(self) -> &'static str {
        match self {
            ParamError::InvalidBaseUrl => "invalid_base_url",
            ParamError::EmailRequired => "email_required",
            ParamError::TokenRequired => "token_required",
        }
    }
}

/// What `validate` learns about the board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validated {
    pub external_scope: String,
    pub board_name: String,
}

/// Why `validate` refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidateError {
    Tracker(TrackerError),
    BoardNotFound,
    BoardProjectMismatch,
}

/// The board a sync fetches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardScope {
    pub base_url: String,
    pub board_ref: String,
}

/// One sprint as the provider reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct SprintRow {
    pub external_id: String,
    pub seq: i64,
    pub name: String,
    pub state: sprint::SprintState,
    pub start_at: Option<chrono::DateTime<chrono::Utc>>,
    pub end_at: Option<chrono::DateTime<chrono::Utc>>,
    pub complete_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// One ticket as the provider reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRow {
    pub external_id: String,
    pub key: String,
    pub scope: Option<String>,
    pub summary: Option<String>,
    pub issue_type: Option<String>,
    pub status: Option<String>,
}

/// A complete, all-or-nothing read of a board.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    /// The board's display name, refreshed on every sync.
    pub board_name: Option<String>,
    pub sprints: Vec<SprintRow>,
    pub tasks: Vec<TaskRow>,
    /// Per sprint `external_id`: its members' `external_id`s, or `None` when the
    /// adapter skipped an immutable (already-fetched closed) sprint.
    pub members: Vec<(String, Option<Vec<String>>)>,
}

/// What the previous sync stored, so an adapter can skip immutable data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrevState {
    /// Sprint `external_id` → the state it had when its members were last fetched.
    pub members_fetched_state: HashMap<String, Option<sprint::SprintState>>,
}

/// Where a session's reference to a ticket was found (`sensei.link_source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LinkSource {
    Branch,
    Prompt,
}

impl LinkSource {
    pub fn as_str(self) -> &'static str {
        match self {
            LinkSource::Branch => "branch",
            LinkSource::Prompt => "prompt",
        }
    }
}

/// One candidate session's text, as matching needs it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionText {
    pub session_id: uuid::Uuid,
    /// Distinct branches the session's turns ran on.
    pub branches: Vec<String>,
    /// Each turn's typed prompt (capped at 4 KB on read).
    pub prompts: Vec<String>,
    pub repo_key: Option<String>,
}

/// One key a ticket answers to (its current key, or a former one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownTask {
    pub external_id: String,
    pub key: String,
    pub scope: Option<String>,
    /// This is the task's current key (not a former one).
    pub current: bool,
}

/// Tickets fetched by key (R11), and their memberships of sprints:
/// `(sprint external id, task external id)`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Discovered {
    pub tasks: Vec<TaskRow>,
    pub members: Vec<(String, String)>,
}

/// A session linked to a ticket, named by the ticket's provider id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub session_id: uuid::Uuid,
    pub task_external_id: String,
    pub source: LinkSource,
    pub matched_key: String,
    pub mentions: u32,
}

/// One tracker provider. All provider knowledge lives behind this trait; the
/// coordinator (`sync`, `connect`) is provider-neutral.
#[async_trait::async_trait]
pub trait TrackerAdapter: Send + Sync {
    fn kind(&self) -> TrackerKind;
    /// Parse and canonicalise the connect body.
    fn connect_params(&self, body: &serde_json::Value) -> Result<ConnectParams, ParamError>;
    /// The auth scheme the transport uses for this account.
    fn auth(&self, account: &str, token: &ApiToken) -> AuthScheme;
    /// Prove the credential and the board.
    async fn validate(
        &self,
        t: &dyn GetTransport,
        p: &ConnectParams,
    ) -> Result<Validated, ValidateError>;
    /// Read the whole board. Any failed request fails the whole snapshot.
    async fn fetch_snapshot(
        &self,
        t: &dyn GetTransport,
        board: &BoardScope,
        prev: &PrevState,
    ) -> Result<Snapshot, TrackerError>;
    /// Pure: which known tickets this session's text refers to.
    fn match_links(&self, session: &SessionText, known: &[KnownTask]) -> Vec<Link>;
    /// R11: pure — keys in this session's text shaped like this board's tickets (a
    /// prefix the board already uses) that match no known ticket. Every key returned
    /// is in the adapter's validated key shape.
    fn unknown_keys(&self, session: &SessionText, known: &[KnownTask]) -> Vec<String>;
    /// R11: fetch these tickets, scoped to the board. A key the board does not have
    /// is absent from the result; a request the provider rejects is `BadRequest`.
    async fn fetch_tasks(
        &self,
        t: &dyn GetTransport,
        board: &BoardScope,
        keys: &[String],
    ) -> Result<Discovered, TrackerError>;
}

/// The registered adapters — the only place a new tracker is added.
pub fn adapters() -> &'static [&'static dyn TrackerAdapter] {
    &[&jira::JiraAdapter]
}

/// The adapter for a kind, if one is registered.
pub fn adapter_for_kind(kind: TrackerKind) -> Option<&'static dyn TrackerAdapter> {
    adapters().iter().copied().find(|a| a.kind() == kind)
}

/// Why a tracker call failed — the fixed vocabulary persisted in
/// `sensei.trackers.last_error` (enum `sensei.tracker_error`) and returned by the
/// API. Never a raw error string: a transport error carries the URL, and a
/// response body can carry issue text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerError {
    Unauthorized,
    Forbidden,
    NotFound,
    RateLimited,
    HttpError,
    Transport,
    Decode,
    Redirect,
    TokenMissing,
    CredentialStore,
    UnsupportedTracker,
    Db,
    BadRequest,
}

impl TrackerError {
    /// Every variant, for exhaustiveness checks against the DDL enum.
    #[cfg(test)]
    pub const ALL: [TrackerError; 13] = [
        TrackerError::Unauthorized,
        TrackerError::Forbidden,
        TrackerError::NotFound,
        TrackerError::RateLimited,
        TrackerError::HttpError,
        TrackerError::Transport,
        TrackerError::Decode,
        TrackerError::Redirect,
        TrackerError::TokenMissing,
        TrackerError::CredentialStore,
        TrackerError::UnsupportedTracker,
        TrackerError::Db,
        TrackerError::BadRequest,
    ];

    /// The `sensei.tracker_error` enum label for this error.
    pub fn code(self) -> &'static str {
        match self {
            TrackerError::Unauthorized => "unauthorized",
            TrackerError::Forbidden => "forbidden",
            TrackerError::NotFound => "not_found",
            TrackerError::RateLimited => "rate_limited",
            TrackerError::HttpError => "http_error",
            TrackerError::Transport => "transport",
            TrackerError::Decode => "decode",
            TrackerError::Redirect => "redirect",
            TrackerError::TokenMissing => "token_missing",
            TrackerError::CredentialStore => "credential_store",
            TrackerError::UnsupportedTracker => "unsupported_tracker",
            TrackerError::Db => "db",
            TrackerError::BadRequest => "bad_request",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_code_is_a_label_of_the_ddl_enum_and_vice_versa() {
        let ddl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../database/ddl/enum/sensei/tracker_error.ddl"
        ))
        .expect("tracker_error.ddl exists");
        let body = ddl
            .split_once("as enum (")
            .and_then(|(_, rest)| rest.split_once(");"))
            .map(|(body, _)| body)
            .expect("an `as enum ( ... );` block");
        let labels: Vec<&str> = body
            .lines()
            .filter_map(|l| {
                let l = l.trim_start_matches([',', ' ']);
                l.strip_prefix('\'').and_then(|r| r.split('\'').next())
            })
            .collect();
        let codes: Vec<&str> = TrackerError::ALL.iter().map(|e| e.code()).collect();
        assert_eq!(
            labels, codes,
            "TrackerError and sensei.tracker_error must list the same labels in the same order"
        );
    }

    #[test]
    fn codes_are_snake_case_and_distinct() {
        let mut codes: Vec<&str> = TrackerError::ALL.iter().map(|e| e.code()).collect();
        assert!(codes.iter().all(|c| c.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_')));
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), TrackerError::ALL.len());
    }
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::sync::Mutex;

    /// A scripted Jira board served over [`GetTransport`]: responses keyed by
    /// `(path, startAt)`, with failure injection and a call log. Shapes follow the
    /// spec's assumed Agile REST format (claim C11 — reconciled by the F1.1 fixture).
    pub struct FakeBoard {
        pages: HashMap<(String, u32), serde_json::Value>,
        failures: HashMap<(String, Option<u32>), TrackerError>,
        calls: Mutex<Vec<String>>,
        on_call: Option<Hook>,
        delay: Option<std::time::Duration>,
        /// Board-scoped searches (R11), keyed by the exact `jql`; an unknown one
        /// finds nothing.
        jql: HashMap<String, Result<serde_json::Value, TrackerError>>,
        jql_calls: Mutex<Vec<String>>,
    }

    /// Runs on every call, before the response — e.g. to delete the tracker
    /// mid-fetch, or to inspect the pool while a fetch is in flight.
    pub type Hook = Box<
        dyn Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
            + Send
            + Sync,
    >;

    impl FakeBoard {
        pub fn empty() -> Self {
            FakeBoard {
                pages: HashMap::new(),
                failures: HashMap::new(),
                calls: Mutex::new(vec![]),
                on_call: None,
                delay: None,
                jql: HashMap::new(),
                jql_calls: Mutex::new(vec![]),
            }
        }
        /// Answer a board-scoped search for this exact `jql` with these issues.
        pub fn with_jql(mut self, jql: &str, issues: serde_json::Value) -> Self {
            self.jql.insert(
                jql.into(),
                Ok(serde_json::json!({"startAt": 0, "total": 0, "issues": issues})),
            );
            self
        }
        /// Fail a board-scoped search for this exact `jql`.
        pub fn fail_jql(mut self, jql: &str, e: TrackerError) -> Self {
            self.jql.insert(jql.into(), Err(e));
            self
        }
        /// Every `jql` asked, in order.
        /// Every path asked, in order.
        pub fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }

        pub fn jql_calls(&self) -> Vec<String> {
            self.jql_calls.lock().unwrap().clone()
        }
        pub fn with(mut self, path: &str, start_at: u32, body: serde_json::Value) -> Self {
            self.pages.insert((path.into(), start_at), body);
            self
        }
        pub fn fail(mut self, path: &str, e: TrackerError) -> Self {
            self.failures.insert((path.into(), None), e);
            self
        }
        /// Fail the `n`th page (1-based, in startAt order) of a listing.
        pub fn fail_page(mut self, path: &str, n: u32, e: TrackerError) -> Self {
            let mut starts: Vec<u32> =
                self.pages.keys().filter(|(p, _)| p == path).map(|(_, s)| *s).collect();
            starts.sort_unstable();
            self.failures.insert((path.into(), Some(starts[(n - 1) as usize])), e);
            self
        }
        pub fn on_call(mut self, hook: Hook) -> Self {
            self.on_call = Some(hook);
            self
        }
        pub fn delay(mut self, d: std::time::Duration) -> Self {
            self.delay = Some(d);
            self
        }
        pub fn was_called(&self, path: &str) -> bool {
            self.calls.lock().unwrap().iter().any(|c| c == path)
        }

        /// Board 42 of project AUTH: sprints 100 (closed) and 101 (active), three
        /// issues over two pages; 10001 in sprint 100, 10002 + 10003 in sprint 101.
        /// Another board on this site, with no sprints and no issues: it connects
        /// and syncs cleanly beside board 42.
        pub fn with_board(self, id: u32, name: &str) -> Self {
            use serde_json::json;
            self.with(
                &format!("/rest/agile/1.0/board/{id}"),
                0,
                json!({"id": id, "name": name, "location": {"projectKey": "AUTH"}}),
            )
            .with(
                &format!("/rest/agile/1.0/board/{id}/sprint"),
                0,
                json!({"startAt": 0, "isLast": true, "values": []}),
            )
            .with(
                &format!("/rest/agile/1.0/board/{id}/issue"),
                0,
                json!({"startAt": 0, "total": 0, "issues": []}),
            )
        }

        pub fn standard() -> Self {
            use serde_json::json;
            let issue = |id: &str, key: &str| {
                json!({"id": id, "key": key, "fields": {
                    "summary": format!("summary {key}"),
                    "issuetype": {"name": "Story"}, "status": {"name": "To Do"}}})
            };
            FakeBoard::empty()
                .with("/rest/api/3/myself", 0, json!({"accountId": "a1", "emailAddress": "dev@example.com"}))
                .with("/rest/agile/1.0/board/42", 0, json!({"id": 42, "name": "AUTH Scrum", "location": {"projectKey": "AUTH"}}))
                .with("/rest/agile/1.0/board/42/sprint", 0, json!({"startAt": 0, "isLast": true, "values": [
                    {"id": 100, "name": "Sprint 1", "state": "closed",
                     "startDate": "2026-09-01T00:00:00.000Z", "endDate": "2026-09-15T00:00:00.000Z",
                     "completeDate": "2026-09-15T10:00:00.000Z"},
                    {"id": 101, "name": "Sprint 2", "state": "active",
                     "startDate": "2026-10-01T00:00:00.000Z", "endDate": "2026-10-15T00:00:00.000Z"}]}))
                .with("/rest/agile/1.0/board/42/issue", 0, json!({"startAt": 0, "maxResults": 2, "total": 3,
                    "issues": [issue("10001", "auth-1"), issue("10002", "AUTH-2")]}))
                .with("/rest/agile/1.0/board/42/issue", 2, json!({"startAt": 2, "maxResults": 2, "total": 3,
                    "issues": [issue("10003", "AUTH-3")]}))
                .with("/rest/agile/1.0/board/42/sprint/100/issue", 0, json!({"startAt": 0, "total": 1,
                    "issues": [issue("10001", "AUTH-1")]}))
                .with("/rest/agile/1.0/board/42/sprint/101/issue", 0, json!({"startAt": 0, "total": 2,
                    "issues": [issue("10002", "AUTH-2"), issue("10003", "AUTH-3")]}))
        }
    }

    #[async_trait::async_trait]
    impl GetTransport for FakeBoard {
        async fn get_json(
            &self,
            path: &str,
            query: &[(&str, String)],
        ) -> Result<serde_json::Value, TrackerError> {
            self.calls.lock().unwrap().push(path.to_string());
            if let Some(hook) = &self.on_call {
                hook(path.to_string()).await;
            }
            if let Some(d) = self.delay {
                tokio::time::sleep(d).await;
            }
            if let Some((_, jql)) = query.iter().find(|(k, _)| *k == "jql") {
                self.jql_calls.lock().unwrap().push(jql.clone());
                return self.jql.get(jql).cloned().unwrap_or_else(|| {
                    Ok(serde_json::json!({"startAt": 0, "total": 0, "issues": []}))
                });
            }
            let start: u32 = query
                .iter()
                .find(|(k, _)| *k == "startAt")
                .and_then(|(_, v)| v.parse().ok())
                .unwrap_or(0);
            if let Some(e) = self
                .failures
                .get(&(path.to_string(), None))
                .or(self.failures.get(&(path.to_string(), Some(start))))
            {
                return Err(*e);
            }
            self.pages.get(&(path.to_string(), start)).cloned().ok_or(TrackerError::NotFound)
        }
    }

    /// A repository folder that belongs to workspace `p` (a fresh repository each call).
    pub async fn repo_in(pg: &crate::db::pg_store::PgStore, p: uuid::Uuid) -> uuid::Uuid {
        let suffix = format!("trk-{}", uuid::Uuid::new_v4().simple());
        let folder = crate::db::pg_store::tests::create_test_folder(pg, &suffix).await;
        pg.set_folder_workspace(&folder, &p, "root", None).await.unwrap();
        folder
    }

    /// A session anchored at `repo_folder` with one transcript turn per branch and
    /// per prompt (the shape the ingest writes: turns keyed by `client_session_id`).
    pub async fn session_with(
        pg: &crate::db::pg_store::PgStore,
        repo_folder: Option<uuid::Uuid>,
        branches: &[&str],
        prompts: &[&str],
    ) -> uuid::Uuid {
        let client = format!("trk-{}", uuid::Uuid::new_v4());
        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.sessions (repo_folder_id, client_session_id, started_at)
             VALUES ($1, $2, now()) RETURNING id",
        )
        .bind(repo_folder)
        .bind(&client)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let turns = branches
            .iter()
            .map(|b| (Some(*b), None))
            .chain(prompts.iter().map(|p| (None, Some(*p))));
        for (i, (branch, prompt)) in turns.enumerate() {
            sqlx_core::query::query(
                "INSERT INTO activity.transcript_turns
                        (source, session_id, family, turn_index, git_branch, user_text, assistant_text)
                 VALUES ('claude_code', $1, 'claude', $2, $3, $4, 'AUTH-1 AUTH-2 AUTH-3')",
            )
            .bind(&client)
            .bind(i as i32)
            .bind(branch)
            .bind(prompt)
            .execute(pg.pool())
            .await
            .unwrap();
        }
        id
    }
}

#[cfg(test)]
mod seam_tests {
    use super::*;

    #[test]
    fn jira_is_registered_and_kinds_round_trip() {
        assert_eq!(adapter_for_kind(TrackerKind::Jira).map(|a| a.kind()), Some(TrackerKind::Jira));
        assert_eq!(TrackerKind::parse(TrackerKind::Jira.as_str()), Some(TrackerKind::Jira));
        assert_eq!(TrackerKind::parse("gitlab"), None, "no adapter, no kind");
    }

    #[test]
    fn every_registered_kind_is_a_label_of_the_ddl_enum() {
        let ddl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../database/ddl/enum/sensei/tracker_kind.ddl"
        ))
        .unwrap();
        for a in adapters() {
            assert!(
                ddl.contains(&format!("'{}'", a.kind().as_str())),
                "{:?} missing from tracker_kind",
                a.kind()
            );
        }
    }

    #[test]
    fn tracker_levels_are_exactly_the_ddl_enum_labels() {
        let ddl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../database/ddl/enum/sensei/tracker_level.ddl"
        ))
        .unwrap();
        let body = ddl.split("as enum (").nth(1).and_then(|b| b.split(");").next()).unwrap();
        let labels: Vec<&str> = body.lines().filter_map(|l| l.split('\'').nth(1)).collect();
        let ours: Vec<&str> = TrackerLevel::ALL.iter().map(|l| l.as_str()).collect();
        assert_eq!(labels, ours);
        for l in TrackerLevel::ALL {
            assert_eq!(TrackerLevel::parse(l.as_str()), Some(l));
        }
        assert_eq!(TrackerLevel::parse("portfolio"), None);
    }

    #[test]
    fn api_token_debug_is_redacted() {
        assert_eq!(format!("{:?}", ApiToken("SENTINEL-TOKEN-XYZ".into())), "ApiToken(***)");
    }
}
