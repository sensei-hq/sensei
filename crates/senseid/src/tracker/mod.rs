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
pub mod writeback;

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

/// A person's id at the provider (Jira: `accountId`). Its `Debug` never prints
/// the value, so an id cannot reach a log line through `{:?}` on any struct that
/// carries it.
#[derive(Clone, PartialEq, Eq)]
pub struct AccountRef(String);

impl AccountRef {
    pub fn new(id: impl Into<String>) -> Self {
        AccountRef(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for AccountRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AccountRef(***)")
    }
}

/// Why a tracker has no owner on record — the `sensei.tracker_owner_state` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerOwnerState {
    Unknown,
    NotAPerson,
}

impl TrackerOwnerState {
    /// In the enum's declared order.
    pub const ALL: [TrackerOwnerState; 2] =
        [TrackerOwnerState::Unknown, TrackerOwnerState::NotAPerson];

    pub fn as_str(self) -> &'static str {
        match self {
            TrackerOwnerState::Unknown => "unknown",
            TrackerOwnerState::NotAPerson => "not_a_person",
        }
    }
    pub fn parse(s: &str) -> Option<TrackerOwnerState> {
        TrackerOwnerState::ALL.into_iter().find(|o| o.as_str() == s)
    }
}

/// Who the token belongs to (spec W1): only a `Person` can be an assignee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnerLookup {
    Person(AccountRef),
    NotAPerson,
    Unknown,
}

impl OwnerLookup {
    /// The `(owner_ref, owner_state)` a tracker row stores for this lookup.
    pub fn columns(&self) -> (Option<&str>, Option<TrackerOwnerState>) {
        match self {
            OwnerLookup::Person(r) => (Some(r.as_str()), None),
            OwnerLookup::NotAPerson => (None, Some(TrackerOwnerState::NotAPerson)),
            OwnerLookup::Unknown => (None, Some(TrackerOwnerState::Unknown)),
        }
    }
}

/// Whether a sync looks the owner up before fetching (W1 backfill): no owner on
/// record, and not already known to be an app account.
pub fn needs_owner_backfill(owner_ref: Option<&str>, owner_state: Option<&str>) -> bool {
    owner_ref.is_none() && owner_state != Some(TrackerOwnerState::NotAPerson.as_str())
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

/// The verbs a write may use (spec W10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMethod {
    Post,
    Put,
}

/// Write access to a tracker's API, held only by the write-back pass (W10).
#[async_trait::async_trait]
pub trait WriteTransport: Send + Sync {
    async fn send_json(
        &self,
        method: WriteMethod,
        path: &str,
        query: &[(&str, String)],
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, TrackerError>;
}

/// One mapped field's value as written (spec W3): an integer, a 2-dp decimal, or
/// `null`, which clears the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    Int(i64),
    Dec(String),
    Null,
}

impl FieldValue {
    /// The JSON a field write sends.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            FieldValue::Int(n) => serde_json::Value::from(*n),
            FieldValue::Dec(d) => {
                d.parse::<f64>().map_or(serde_json::Value::Null, serde_json::Value::from)
            }
            FieldValue::Null => serde_json::Value::Null,
        }
    }
    /// The canonical text the hash covers (`null` for no value).
    pub fn canonical(&self) -> String {
        match self {
            FieldValue::Int(n) => n.to_string(),
            FieldValue::Dec(d) => d.clone(),
            FieldValue::Null => "null".to_string(),
        }
    }
}

/// What the tracker says about a field a mapping names (W7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldCheck {
    pub field_ref: String,
    pub exists: bool,
    pub numeric: bool,
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
    /// Who the token belongs to (W1).
    pub owner: OwnerLookup,
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
    /// Compared with the tracker's owner on apply (W1), never stored.
    pub assignee: Option<AccountRef>,
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
    /// Who the token belongs to (W1).
    async fn owner(&self, t: &dyn GetTransport) -> Result<OwnerLookup, TrackerError>;
    /// The newest comment `owner` wrote as a write-back report on this ticket, if
    /// any: adopted instead of posting a second one (W5).
    async fn find_comment(
        &self,
        t: &dyn GetTransport,
        task_external_id: &str,
        owner: &AccountRef,
    ) -> Result<Option<String>, TrackerError>;
    /// Post the report as a new comment; its id.
    async fn create_comment(
        &self,
        w: &dyn WriteTransport,
        task_external_id: &str,
        report: &writeback::Report,
    ) -> Result<String, TrackerError>;
    /// Replace the comment's body with the report, without notifying watchers.
    async fn update_comment(
        &self,
        w: &dyn WriteTransport,
        task_external_id: &str,
        comment_ref: &str,
        report: &writeback::Report,
    ) -> Result<(), TrackerError>;
    /// Pure: whether `field_ref` is a field id this tracker can write (W7).
    fn field_ref_shape(&self, field_ref: &str) -> bool;
    /// What the tracker says about each of these fields (W7).
    async fn check_fields(
        &self,
        t: &dyn GetTransport,
        field_refs: &[String],
    ) -> Result<Vec<FieldCheck>, TrackerError>;
    /// Whether any ticket on the board already has a value in this field (W7).
    async fn field_in_use(
        &self,
        t: &dyn GetTransport,
        board: &BoardScope,
        field_ref: &str,
    ) -> Result<bool, TrackerError>;
    /// Write these fields of the ticket in one request (W6); a `Null` clears one.
    async fn write_fields(
        &self,
        w: &dyn WriteTransport,
        task_external_id: &str,
        fields: &[(String, FieldValue)],
    ) -> Result<(), TrackerError>;
    /// Whether the ticket itself still exists (W5: which object a 404 meant).
    async fn issue_exists(
        &self,
        t: &dyn GetTransport,
        task_external_id: &str,
    ) -> Result<bool, TrackerError>;
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
        let labels = test_support::ddl_enum_labels("tracker_error");
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

    #[test]
    fn owner_states_are_exactly_the_ddl_enum_labels() {
        let labels = test_support::ddl_enum_labels("tracker_owner_state");
        let ours: Vec<&str> = TrackerOwnerState::ALL.iter().map(|o| o.as_str()).collect();
        assert_eq!(labels, ours);
        for o in TrackerOwnerState::ALL {
            assert_eq!(TrackerOwnerState::parse(o.as_str()), Some(o));
        }
        assert_eq!(TrackerOwnerState::parse("person"), None);
    }

    #[test]
    fn the_design_dbml_lists_each_tracker_table_with_the_columns_of_its_ddl() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../database");
        let dbml = std::fs::read_to_string(format!("{root}/design.dbml")).unwrap();
        for table in
            ["trackers", "tracker_tasks", "sprints", "tracker_task_sprints", "tracker_fields"]
        {
            let ddl =
                std::fs::read_to_string(format!("{root}/ddl/table/sensei/{table}.ddl")).unwrap();
            assert_eq!(
                test_support::dbml_columns(&dbml, table),
                test_support::ddl_columns(&ddl, table),
                "design.dbml's sensei.{table} must list the DDL's columns, in order"
            );
        }
    }

    #[test]
    fn an_owner_lookup_maps_onto_the_stored_columns() {
        let person = OwnerLookup::Person(AccountRef::new("A1"));
        assert_eq!(person.columns(), (Some("A1"), None));
        assert_eq!(OwnerLookup::NotAPerson.columns(), (None, Some(TrackerOwnerState::NotAPerson)));
        assert_eq!(OwnerLookup::Unknown.columns(), (None, Some(TrackerOwnerState::Unknown)));
    }

    #[test]
    fn the_owner_is_looked_up_only_while_none_is_on_record_and_not_ruled_out() {
        assert!(needs_owner_backfill(None, None), "a tracker from before owners");
        assert!(needs_owner_backfill(None, Some("unknown")), "unknown is retried");
        assert!(
            !needs_owner_backfill(None, Some("not_a_person")),
            "an app token is never re-asked"
        );
        assert!(!needs_owner_backfill(Some("A1"), None));
    }

    #[test]
    fn no_account_id_reaches_a_debug_print() {
        let id = AccountRef::new("SENTINEL-ACCOUNT-ID");
        let task = TaskRow {
            external_id: "10001".into(),
            key: "AUTH-1".into(),
            scope: None,
            summary: None,
            issue_type: None,
            status: None,
            assignee: Some(AccountRef::new("SENTINEL-ACCOUNT-ID")),
        };
        let validated = Validated {
            external_scope: "AUTH".into(),
            board_name: "AUTH Scrum".into(),
            owner: OwnerLookup::Person(AccountRef::new("SENTINEL-ACCOUNT-ID")),
        };
        for printed in [format!("{id:?}"), format!("{task:?}"), format!("{validated:?}")] {
            assert!(!printed.contains("SENTINEL-ACCOUNT-ID"), "{printed}");
        }
        assert_eq!(format!("{id:?}"), "AccountRef(***)");
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
        /// Every call's path and query, in order.
        queried: Mutex<Vec<(String, Query)>>,
    }

    /// The labels of `database/ddl/enum/sensei/<name>.ddl`, in declared order.
    pub fn ddl_enum_labels(name: &str) -> Vec<String> {
        let path =
            format!("{}/../../database/ddl/enum/sensei/{name}.ddl", env!("CARGO_MANIFEST_DIR"));
        let ddl = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let body = ddl
            .split_once("as enum (")
            .and_then(|(_, rest)| rest.split_once(");"))
            .map(|(body, _)| body)
            .expect("an `as enum ( ... );` block");
        body.lines()
            .filter_map(|l| {
                let l = l.trim_start_matches([',', ' ']);
                l.strip_prefix('\'').and_then(|r| r.split('\'').next()).map(String::from)
            })
            .collect()
    }

    /// The column names of `create table … <table> ( … );`, in declared order.
    pub fn ddl_columns(ddl: &str, table: &str) -> Vec<String> {
        let start = format!("create table if not exists {table} (");
        let body = ddl
            .split_once(&start)
            .and_then(|(_, rest)| rest.split_once("\n);"))
            .map(|(body, _)| body)
            .unwrap_or_else(|| panic!("no `{start}` block"));
        body.lines()
            .map(|l| l.trim_start_matches([',', ' ']))
            .filter(|l| {
                !l.is_empty()
                    && !l.starts_with("--")
                    && !l.starts_with("unique")
                    && !l.starts_with("primary key (")
            })
            .filter_map(|l| l.split_whitespace().next().map(String::from))
            .collect()
    }

    /// The column names `database/design.dbml` lists for `sensei.<table>`, in order.
    pub fn dbml_columns(dbml: &str, table: &str) -> Vec<String> {
        let start = format!("Table \"sensei\".\"{table}\" {{\n");
        let body = dbml
            .split_once(&start)
            .map(|(_, rest)| rest.split("\n}").next().unwrap_or(""))
            .unwrap_or_else(|| panic!("no `{start}` block"));
        body.lines()
            .take_while(|l| {
                !l.trim_start().starts_with("indexes") && !l.trim_start().starts_with("Note")
            })
            .filter_map(|l| {
                l.strip_prefix("  \"").and_then(|r| r.split('"').next()).map(String::from)
            })
            .collect()
    }

    /// Drop `fields.assignee` from every issue unless the request's `fields` asks
    /// for it, as Jira does — so a read that forgets to ask sees no assignee.
    fn honour_fields(mut body: serde_json::Value, query: &[(&str, String)]) -> serde_json::Value {
        let asked = query
            .iter()
            .any(|(k, v)| *k == "fields" && v.split(',').any(|f| f.trim() == "assignee"));
        if !asked && let Some(issues) = body.get_mut("issues").and_then(|i| i.as_array_mut()) {
            for issue in issues {
                if let Some(fields) = issue.get_mut("fields").and_then(|f| f.as_object_mut()) {
                    fields.remove("assignee");
                }
            }
        }
        body
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
                queried: Mutex::new(vec![]),
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

        /// The query of the last call to `path`.
        pub fn query_of(&self, path: &str) -> Option<Vec<(String, String)>> {
            self.queried
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|(p, _)| p == path)
                .map(|(_, q)| q.clone())
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
                .with("/rest/api/3/myself", 0, json!({"accountId": "A1", "accountType": "atlassian", "emailAddress": "dev@example.com"}))
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
            self.queried.lock().unwrap().push((
                path.to_string(),
                query.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
            ));
            if let Some(hook) = &self.on_call {
                hook(path.to_string()).await;
            }
            if let Some(d) = self.delay {
                tokio::time::sleep(d).await;
            }
            if let Some((_, jql)) = query.iter().find(|(k, _)| *k == "jql") {
                self.jql_calls.lock().unwrap().push(jql.clone());
                return self
                    .jql
                    .get(jql)
                    .cloned()
                    .unwrap_or_else(|| {
                        Ok(serde_json::json!({"startAt": 0, "total": 0, "issues": []}))
                    })
                    .map(|b| honour_fields(b, query));
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
            self.pages
                .get(&(path.to_string(), start))
                .cloned()
                .map(|b| honour_fields(b, query))
                .ok_or(TrackerError::NotFound)
        }
    }

    /// A request's query, as recorded.
    pub type Query = Vec<(String, String)>;
    /// Scripted `(status, Retry-After)` replies, consumed in order.
    type Replies = std::collections::VecDeque<(u16, Option<String>)>;

    /// One write the recording backend received.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Sent {
        pub method: WriteMethod,
        pub path: String,
        pub query: Vec<(String, String)>,
        pub body: serde_json::Value,
    }

    /// A scripted write backend (spec W10): a [`RawWrite`](crate::tracker::transport::RawWrite)
    /// with a call log, so the real `PolicyTransport` (429 loop, `Retry-After`)
    /// sits in front of it. Unscripted, a POST answers 201 with a fresh `id` and a
    /// PUT answers 204.
    #[derive(Default)]
    pub struct RecordingWrites {
        sent: Mutex<Vec<Sent>>,
        script: Mutex<Vec<(WriteMethod, String, Replies)>>,
        next_id: std::sync::atomic::AtomicU32,
        on_send: Option<Hook>,
        delay: Option<std::time::Duration>,
    }

    impl RecordingWrites {
        /// Answer the next `times` sends of `method` to `path` with `status`.
        pub fn respond(self, method: WriteMethod, path: &str, status: u16, times: usize) -> Self {
            self.respond_after(method, path, status, None, times)
        }
        /// [`respond`](Self::respond) with a `Retry-After` header.
        pub fn respond_after(
            self,
            method: WriteMethod,
            path: &str,
            status: u16,
            retry_after: Option<&str>,
            times: usize,
        ) -> Self {
            let q = (0..times).map(|_| (status, retry_after.map(String::from))).collect();
            self.script.lock().unwrap().push((method, path.into(), q));
            self
        }
        /// Runs on every send, before the response.
        pub fn on_send(mut self, hook: Hook) -> Self {
            self.on_send = Some(hook);
            self
        }
        pub fn delay(mut self, d: std::time::Duration) -> Self {
            self.delay = Some(d);
            self
        }
        pub fn sent(&self) -> Vec<Sent> {
            self.sent.lock().unwrap().clone()
        }
        pub fn count(&self, method: WriteMethod) -> usize {
            self.sent.lock().unwrap().iter().filter(|s| s.method == method).count()
        }
    }

    #[async_trait::async_trait]
    impl crate::tracker::transport::RawWrite for RecordingWrites {
        async fn send(
            &self,
            method: WriteMethod,
            path: &str,
            query: &[(&str, String)],
            body: &[u8],
        ) -> Result<crate::tracker::transport::RawResponse, ()> {
            self.sent.lock().unwrap().push(Sent {
                method,
                path: path.into(),
                query: query.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
                body: serde_json::from_slice(body).unwrap_or(serde_json::Value::Null),
            });
            if let Some(hook) = &self.on_send {
                hook(path.to_string()).await;
            }
            if let Some(d) = self.delay {
                tokio::time::sleep(d).await;
            }
            let scripted = self
                .script
                .lock()
                .unwrap()
                .iter_mut()
                .find(|(m, p, q)| *m == method && p == path && !q.is_empty())
                .and_then(|(_, _, q)| q.pop_front());
            let raw = |status, retry_after, body: String| crate::tracker::transport::RawResponse {
                status,
                retry_after,
                body: body.into_bytes(),
            };
            Ok(match (scripted, method) {
                (Some((status, retry_after)), _) => raw(status, retry_after, String::new()),
                (None, WriteMethod::Post) => {
                    let id = 20000 + self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    raw(201, None, format!(r#"{{"id":"{id}"}}"#))
                }
                (None, WriteMethod::Put) => raw(204, None, String::new()),
            })
        }
    }

    /// A board issue as Jira returns it, assigned to `assignee` (`None` = unassigned).
    pub fn assigned_issue(id: &str, key: &str, assignee: Option<&str>) -> serde_json::Value {
        serde_json::json!({"id": id, "key": key, "fields": {
            "summary": format!("summary {key}"),
            "issuetype": {"name": "Story"}, "status": {"name": "To Do"},
            "assignee": assignee.map(|a| serde_json::json!({"accountId": a, "displayName": "Someone"}))}})
    }

    impl FakeBoard {
        /// The standard board (owner `A1`) with its three issues assigned as given,
        /// in order AUTH-1, AUTH-2, AUTH-3.
        pub fn with_assignees(self, assignees: [Option<&str>; 3]) -> Self {
            use serde_json::json;
            let [a1, a2, a3] = assignees;
            self.with("/rest/agile/1.0/board/42/issue", 0, json!({"startAt": 0, "maxResults": 2, "total": 3,
                "issues": [assigned_issue("10001", "auth-1", a1), assigned_issue("10002", "AUTH-2", a2)]}))
                .with("/rest/agile/1.0/board/42/issue", 2, json!({"startAt": 2, "maxResults": 2, "total": 3,
                    "issues": [assigned_issue("10003", "AUTH-3", a3)]}))
        }
        /// Write-back reads (W5) for the standard board's three issues: each has no
        /// comments yet and still exists.
        pub fn with_comment_reads(self) -> Self {
            use serde_json::json;
            ["10001", "10002", "10003"].into_iter().fold(self, |b, id| {
                b.with(&format!("/rest/api/3/issue/{id}/comment"), 0, json!({"comments": []})).with(
                    &format!("/rest/api/3/issue/{id}"),
                    0,
                    json!({"id": id}),
                )
            })
        }
        /// The standard board whose `/myself` answers `body`.
        pub fn with_myself(self, body: serde_json::Value) -> Self {
            self.with("/rest/api/3/myself", 0, body)
        }
        /// How many times `/myself` was asked.
        pub fn myself_calls(&self) -> usize {
            self.calls().iter().filter(|c| *c == "/rest/api/3/myself").count()
        }
    }

    /// `(owner_ref, owner_state, owner_error)` of a tracker row.
    pub async fn owner_of(
        pg: &crate::db::pg_store::PgStore,
        t: uuid::Uuid,
    ) -> (Option<String>, Option<String>, Option<String>) {
        sqlx_core::query_as::query_as(
            "SELECT owner_ref, owner_state::text, owner_error::text FROM sensei.trackers WHERE id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap()
    }

    /// Forget a tracker's owner, as a tracker connected before owners has none.
    pub async fn forget_owner(pg: &crate::db::pg_store::PgStore, t: uuid::Uuid) {
        sqlx_core::query::query(
            "UPDATE sensei.trackers SET owner_ref = NULL, owner_state = NULL, owner_error = NULL WHERE id = $1",
        )
        .bind(t)
        .execute(pg.pool())
        .await
        .unwrap();
    }

    /// `(external_id, assigned_to_me)` of every ticket of a tracker, by external id.
    pub async fn assigned(
        pg: &crate::db::pg_store::PgStore,
        t: uuid::Uuid,
    ) -> Vec<(String, Option<bool>)> {
        sqlx_core::query_as::query_as(
            "SELECT external_id, assigned_to_me FROM sensei.tracker_tasks WHERE tracker_id = $1 ORDER BY 1",
        )
        .bind(t)
        .fetch_all(pg.pool())
        .await
        .unwrap()
    }

    /// Rows of any `sensei` table whose text contains `needle` — proves a value
    /// was never persisted anywhere.
    pub async fn rows_containing(pg: &crate::db::pg_store::PgStore, needle: &str) -> Vec<String> {
        let tables: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT table_name::text FROM information_schema.tables
              WHERE table_schema = 'sensei' AND table_type = 'BASE TABLE' ORDER BY 1",
        )
        .fetch_all(pg.pool())
        .await
        .unwrap();
        let mut hits = vec![];
        for (table,) in tables {
            let (n,): (i64,) = sqlx_core::query_as::query_as(&format!(
                "SELECT count(*) FROM sensei.\"{table}\" r WHERE strpos(row_to_json(r)::text, $1) > 0"
            ))
            .bind(needle)
            .fetch_one(pg.pool())
            .await
            .unwrap();
            if n > 0 {
                hits.push(format!("{table}: {n}"));
            }
        }
        hits
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
        let labels = test_support::ddl_enum_labels("tracker_level");
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
