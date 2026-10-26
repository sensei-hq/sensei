use crate::runs::{NewRun, Run, RunEvent, RunEventKind};

/// How a sensei workspace is named to the dōjō: its namespace on the ladder's
/// `workspace` rung, matched on the workspace's name (the namespaces' documented
/// link — "for a `workspace` namespace via the workspace of the same name"). ONE fragment,
/// spliced into the relay's queries and the sync's offer (#299), so the two
/// paths cannot file the same workspace under different slugs. Expects the
/// workspace aliased `p`; binds the namespace as `n`.
pub(crate) const WORKSPACE_NAMESPACE_JOIN: &str =
    "JOIN sensei.namespaces n ON n.scope_key = 'workspace' AND n.name = p.name";
use dojo_protocol::relay::RelayRunStatus;
use sensei_bootstrap::{
    DB_POOL_ACQUIRE_TIMEOUT_SECS, DB_POOL_IDLE_TIMEOUT_SECS, DB_POOL_MAX_CONNECTIONS,
    DB_POOL_MAX_LIFETIME_SECS, DB_POOL_MIN_CONNECTIONS,
};
use sqlx_postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// PostgreSQL store.
/// Schema is managed by `dbd apply`, not by this code.
#[derive(Clone)]
pub struct PgStore {
    pool: PgPool,
}

/// The most connections one test's pool may hold ([`PgStore::connect_test`]).
///
/// One factor of the suite's connection budget (#204): test threads × this,
/// plus the daemon's [`sensei_bootstrap::DB_POOL_MAX_CONNECTIONS`], must stay
/// under Postgres's `max_connections`. `.cargo/config.toml` caps the threads,
/// and `the_test_suite_fits_postgres_connection_budget` checks the sum.
pub(crate) const TEST_POOL_MAX_CONNECTIONS: u32 = 4;

/// One node keyed by its fqn, as [`PgStore::upsert_node_by_fqn`] writes it: a
/// REFERENCE (`def: None`) creates or returns the unresolved stub; a DEFINITION
/// (`def: Some`) creates or enriches the same node in place.
pub struct FqnNode<'a> {
    pub folder_id: &'a uuid::Uuid,
    pub fqn: &'a str,
    pub kind: &'a str,
    pub name: &'a str,
    pub language: Option<&'a str>,
    pub def: Option<FqnDef<'a>>,
}

/// Definition details for [`PgStore::upsert_node_by_fqn`] — the fields a
/// DEFINITION fills on a node a reference may have created as a stub. Passing
/// `None` for the `def` argument means "this mention is a REFERENCE": get-or-create
/// the stub and leave it unresolved. `Some` ENRICHES: flip `resolved=true` and
/// write these. `file_path` is required (a definition always has a home file);
/// external symbols with no local file are `lib_symbol` nodes (see
/// [`PgStore::upsert_lib_node_by_fqn`]).
#[derive(Debug, Clone)]
pub struct FqnDef<'a> {
    pub file_path: &'a str,
    pub signature: Option<&'a str>,
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
    pub is_exported: bool,
    pub parent_id: Option<&'a uuid::Uuid>,
}

/// Render a float slice to pgvector's text literal (`[v1,v2,...]`) so it can be
/// bound as text and cast with `$n::vector` — no pgvector crate needed. Shared
/// by `set_node_embedding` (writes) and `semantic_search_nodes` (query vector).
fn vector_literal(v: &[f32]) -> String {
    use std::fmt::Write as _;
    let mut buf = String::with_capacity(v.len() * 8 + 2);
    buf.push('[');
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            buf.push(',');
        }
        let _ = write!(buf, "{x}");
    }
    buf.push(']');
    buf
}

/// Valid `workspaces.maturity` lifecycle values — mirrors the
/// `sensei.workspace_maturity` enum in
/// `database/ddl/enum/sensei/workspace_maturity.ddl`. Used to reject an unknown
/// maturity with a clear error (and a 400 at the HTTP layer) instead of
/// leaking a raw Postgres enum-cast failure as a 500.
pub const WORKSPACE_MATURITIES: [&str; 4] = ["discovery", "active", "maintenance", "archived"];

/// Partial update for a workspace's editable identity fields (the About screen).
/// Every field is optional: `None` leaves the column unchanged (COALESCE
/// semantics), matching the About form which strips empty inputs and PUTs only
/// the fields the user touched. Text columns bind `Option<&str>`; the three
/// jsonb columns (`icon`/`stack`/`links`) bind `Option<&serde_json::Value>`;
/// `maturity` is the `sensei.workspace_maturity` enum and is validated against
/// [`WORKSPACE_MATURITIES`] before the write.
#[derive(Debug, Default, Clone)]
pub struct WorkspacePatch<'a> {
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub maturity: Option<&'a str>,
    pub client: Option<&'a str>,
    pub goal: Option<&'a str>,
    pub preferred_acp: Option<&'a str>,
    pub icon: Option<&'a serde_json::Value>,
    pub stack: Option<&'a serde_json::Value>,
    pub links: Option<&'a serde_json::Value>,
}

/// Per-table row counts from a single `PgStore::prune_activity` run (#74).
/// Everything is `u64` so callers can report a single log line without
/// per-field conversions.
#[derive(Debug, Default, Clone, Copy, serde::Serialize)]
pub struct ActivityPruneCounts {
    pub sessions: u64,
    pub turns: u64,
    pub transcript_turns: u64,
    pub assistant_events: u64,
}

/// One community to write via [`PgStore::replace_communities_for_folder`] (D4):
/// a deterministic `community_id` (1..k), a human label, its member node ids, and
/// its `god_node_ids` — the top-5 members by `degree` (D4.5), the community's hubs.
/// This is the AUTHORITATIVE payload; it is written with an honest-empty
/// `description` (`props.source = "null"`). The non-authoritative model-authored
/// description is filled in afterwards, off the terminal barrier, by
/// [`crate::indexer::community::enrich_community_descriptions`] (spec W3 fail-open).
#[derive(Debug, Clone)]
pub struct CommunityAssignment {
    pub community_id: i32,
    pub label: String,
    pub member_node_ids: Vec<uuid::Uuid>,
    pub god_node_ids: Vec<uuid::Uuid>,
}

/// One edge to (re)insert via [`PgStore::replace_edges_of_kind`] (D2). Mirrors
/// the `insert_edge` shape: a resolved edge carries `target_id`; an unresolved
/// one carries `target_name`/`target_file`.
#[derive(Debug, Clone)]
pub struct EdgeSpec {
    pub source_id: uuid::Uuid,
    pub target_id: Option<uuid::Uuid>,
    pub target_name: Option<String>,
    pub target_file: Option<String>,
}

#[derive(Clone)]
pub struct InsertMemory {
    pub workspace_id: Option<uuid::Uuid>,
    pub scope: String,
    pub scope_filter: Option<String>,
    pub mtype: String, // memory_type enum value
    pub title: String,
    pub content: String,
    pub impact: Option<String>,
    pub tags: Vec<String>,
    pub triage_signal: Option<String>,
    pub status: String, // memory_status enum value
    // Governance plane: where the rule applies (namespace) + its authority.
    pub namespace_id: Option<uuid::Uuid>,
    pub enforcement: Option<String>, // enforcement enum value; None → DB default 'recommended'
    pub origin: Option<String>,      // None → DB default 'learned'
    pub source_id: Option<uuid::Uuid>, // provenance: knowledge_sources.id for origin='federated'
    // Spine anchoring (memory-anchoring design 2026-07-18): which doc-slot this
    // memory belongs to, and — for feature-scoped slots — which feature. Both
    // nullable; None/None = unanchored.
    pub spine_slot: Option<String>, // sensei.spine_slot enum value
    pub feature: Option<String>,
}

pub struct OutcomeRow {
    pub memory_id: uuid::Uuid,
    pub session_id: Option<uuid::Uuid>,
    pub outcome: String,
    pub context: Option<String>,
}

/// Input for registering a federation endpoint.
pub struct NewKnowledgeSource {
    pub kind: String,
    pub name: String,
    pub url: String,
    pub namespace_id: Option<uuid::Uuid>,
    pub direction: String, // push | pull | both
}

/// A registered federation endpoint (row of sensei.knowledge_sources).
#[derive(Debug, Clone)]
pub struct KnowledgeSource {
    pub id: uuid::Uuid,
    pub kind: String,
    pub name: String,
    pub url: String,
    pub namespace_id: Option<uuid::Uuid>,
    pub direction: String,
    pub last_seq: i64,
    pub enabled: bool,
}

/// A `sensei.knowledge_sources` row as selected (`SOURCE_SELECT`).
pub(crate) type KnowledgeSourceRow =
    (uuid::Uuid, String, String, String, Option<uuid::Uuid>, String, i64, bool);

/// Input for recording a daemon-side Dōjō membership (`sensei.dojo_memberships`).
/// It holds no credential: its tenant calls present the bound persona's session.
pub struct NewDojoMembership {
    /// The service membership id (`dojo.memberships.id`); becomes the local PK
    /// and the value `sensei.workspaces.dojo_id` points at. Service-assigned.
    pub id: uuid::Uuid,
    pub registry_url: String,
    pub tenant_key: String,
    pub dojo_url: String,
    pub kind: String, // employer | client | community | personal
    /// Git-remote owner slugs this membership covers (lowercased) — feeds
    /// infer-at-detect auto-bind. Empty for memberships with no org coverage.
    pub org_slugs: Vec<String>,
    pub role: String,
    pub authenticated_via: String,
    pub attribution_default: String,
    pub sync_status: String,
}

/// A daemon-side Dōjō membership (row of `sensei.dojo_memberships`).
#[derive(Debug, Clone)]
pub struct DojoMembership {
    pub id: uuid::Uuid,
    pub registry_url: String,
    pub tenant_key: String,
    pub dojo_url: String,
    pub kind: String,
    /// Git-remote owner slugs this membership covers (lowercased). Drives
    /// infer-at-detect auto-bind (see `dojo/routing.rs::infer_binding`).
    pub org_slugs: Vec<String>,
    pub role: String,
    pub authenticated_via: String,
    pub attribution_default: String,
    pub sync_status: String,
    pub last_seq: i64,
    /// RFC-3339 text (SELECTed as `::text`) so the API can serialize it without
    /// pulling chrono into the row tuple. `None` until the first heartbeat.
    pub last_heartbeat_at: Option<String>,
    pub enabled: bool,
    /// Whose session the membership's tenant calls present (#236 step 4) —
    /// `None` until a persona is matched to it.
    pub persona: Option<MembershipPersona>,
}

/// One `DOJO_SELECT` row, in column order.
pub(crate) type DojoMembershipRow = (
    uuid::Uuid,
    String,
    String,
    String,
    String,
    Vec<String>,
    String,
    String,
    String,
    String,
    i64,
    Option<String>,
    bool,
    Option<uuid::Uuid>,
    Option<String>,
);

/// The persona a local Dōjō membership belongs to
/// (`sensei.dojo_membership_personas`, #236 step 4), with that persona's
/// session slot — `None` when the persona is signed out, which is NOT the same
/// as the membership having no persona: a bound membership never falls back to
/// its device token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipPersona {
    pub persona_id: uuid::Uuid,
    pub session_slot: Option<String>,
}

/// A row of `sensei.collective_preferences` — the single-row collective sharing
/// settings backing `GET/PUT /api/preferences/collective`. `categories` is the
/// raw jsonb toggle map; `updated_at` is RFC-3339 text (SELECTed `::text`).
#[derive(Debug, Clone)]
pub struct CollectivePrefsRow {
    pub destination: String,
    pub cadence: String,
    pub categories: serde_json::Value,
    pub attribution_default: String,
    pub updated_at: String,
}

/// A federated_memories ledger row.
#[derive(Debug, Clone)]
pub struct FederatedLink {
    pub memory_id: Option<uuid::Uuid>,
    pub remote_seq: i64,
}

/// One member memory of a share batch, in the shape the C6 upstream-contribute
/// path needs to build an artifact. Every field here goes through the
/// deterministic confidentiality strip again before it leaves the machine.
///
/// `body` and `example` are the generalised lane — a rewrite, never the raw
/// `content`. `title` is NOT: it is `memories.title` verbatim, and the strip is
/// the only thing standing between it and the wire. See the field.
#[derive(Debug, Clone)]
pub struct ShareBatchItem {
    pub memory_id: uuid::Uuid,
    /// `memories.title` VERBATIM — there is no `generalised_title`. For an
    /// `origin='learned'` memory the title is a truncated slice of the prompt
    /// that produced it, so unlike `body` this field has no rewrite behind it:
    /// only the deterministic strip runs, which removes KNOWN identifiers
    /// (workspace/client/repo/folder/session/person) and pattern-matched
    /// paths/emails/uuids/secrets — ordinary prose passes through unchanged.
    /// Generalising the title is tracked in `docs/history/plan/decisions.md`.
    pub title: String,
    /// The portable text: the `generalised_content` rewrite, or EMPTY when the
    /// memory has not been generalised. Deliberately never the raw `content` —
    /// an empty body means "nothing shareable exists yet" and the caller holds
    /// the item (`HeldNotGeneralised`) instead of shipping the local reference.
    pub body: String,
    /// The `generalised_example`: a synthetic illustration of `body`, invented by
    /// the same rewrite and containing nothing from the raw memory. `None` when
    /// the rewrite produced none — an artifact simply ships without one.
    pub example: Option<String>,
    /// `sensei.memory_type` string — drives artifact-kind mapping (pattern → the
    /// `pattern` artifact; everything else → `principle`).
    pub memory_type: String,
}

/// Snapshot needed to publish a memory to a dojo (+ namespace identity + origin/scope_key for gating).
#[derive(Debug, Clone)]
pub struct MemoryPushPayload {
    pub title: String,
    pub content: String,
    pub impact: Option<String>,
    pub enforcement: String,
    pub rule_type: String,
    pub origin: String,
    pub scope_key: String,
    pub slug: String,
    pub name: String,
}

/// Parse a git remote URL into its source-identifier tokens (owner + repo),
/// e.g. `git@github.com:acme/acme-api.git` and
/// `https://github.com/acme/acme-api` both → `["acme", "acme-api"]`. Used by
/// [`PgStore::workspace_identifiers`] to feed the confidentiality dereference
/// (C5) — both the org/owner and the repo name are source identifiers to strip.
/// Host-like segments (containing a dot) are skipped. Pure — unit-tested.
fn repo_tokens_from_remote(url: &str) -> Vec<String> {
    let segments = remote_path_segments(url);
    segments.iter().rev().take(2).map(|s| s.to_string()).collect()
}

/// Filtered path segments of a git remote URL (scheme/host/user stripped), in
/// path order — `git@github.com:acme/acme-api.git` and
/// `https://github.com/acme/acme-api` both → `["acme", "acme-api"]`. Host-like
/// segments (containing a dot or `@`) are dropped so scheme/host never survive.
/// Shared by [`repo_tokens_from_remote`] (owner+repo token set for the C5
/// dereference) and [`remote_owner_slug`] (positional owner). Pure.
fn remote_path_segments(url: &str) -> Vec<String> {
    let trimmed = url.trim().trim_end_matches(".git");
    // Drop the scheme (`https://`, `ssh://`, …) if present.
    let after_scheme = trimmed.rsplit("://").next().unwrap_or(trimmed);
    // scp-like `git@host:owner/repo` — drop the `user@host:` head.
    let path = match after_scheme.split_once(':') {
        Some((head, tail)) if head.contains('@') || head.contains('.') => tail,
        _ => after_scheme,
    };
    path.split('/')
        .filter(|s| !s.is_empty() && !s.contains('.') && !s.contains('@'))
        .map(|s| s.to_string())
        .collect()
}

/// The git-org owner slug of a remote — the path segment before the repo,
/// lowercased. `git@github.com:Sensei-HQ/sensei.git` and
/// `https://github.com/Sensei-HQ/sensei` both → `Some("sensei-hq")`; a URL with
/// no owner segment → `None`. Feeds [`PgStore::workspace_org_owners`] →
/// `dojo::routing::infer_binding` for the R3 auto-bind suggestion. Pure.
pub(crate) fn remote_owner_slug(url: &str) -> Option<String> {
    let segments = remote_path_segments(url);
    (segments.len() >= 2).then(|| segments[segments.len() - 2].to_ascii_lowercase())
}

/// A row from the `sensei.metrics` registry — the data-driven catalog of what to
/// compute (see `database/ddl/table/sensei/metrics.ddl`). Carries the descriptive
/// facets (`name`/`purpose`/`how_to_read`/`formula`) and the compute knobs
/// (`type`/`direction`/`weight`/`target`/`task_name`) the scheduler and compute
/// handlers read. `family`/`metric_type`/`direction` are the `sensei.metric_*`
/// enums surfaced as their text values; `weight`/`target` are the `numeric`
/// columns as `f64`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Metric {
    pub id: uuid::Uuid,
    pub key: String,
    pub name: String,
    pub description: String,
    pub family: String,
    pub metric_type: String,
    pub unit: Option<String>,
    pub direction: String,
    pub purpose: String,
    pub how_to_read: String,
    pub formula: String,
    pub task_name: String,
    pub weight: f64,
    pub target: Option<f64>,
    pub effective_from: chrono::NaiveDate,
    pub effective_until: Option<chrono::NaiveDate>,
}

/// Latest stored value for one metric of a workspace, with the catalog facets it is
/// read through — the shape [`PgStore::get_workspace_metrics`] returns. `value`/`props`
/// come from `sensei.workspace_metric_daily` (latest `date` per metric); `name`,
/// `metric_type`, `unit`, `direction`, `purpose`, `how_to_read` are joined from
/// `sensei.metrics`. Trend (prior/delta) is deferred to the Phase 7 endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkspaceMetricRow {
    pub metric: String,
    pub date: chrono::NaiveDate,
    pub value: f64,
    pub props: serde_json::Value,
    pub name: String,
    pub metric_type: String,
    pub unit: Option<String>,
    pub direction: String,
    pub purpose: String,
    pub how_to_read: String,
}

/// The latest weekly trend point for one metric of a workspace — the shape
/// [`PgStore::get_workspace_metric_trend`] returns. Read from
/// `sensei.workspace_metric_trend` (the weekly `lag()` over `workspace_metric_weekly`):
/// `prior`/`delta` are `None` for a metric's first period (honest-null, never a
/// fabricated 0). `direction` travels with the row so the UI can colour the delta
/// (a positive delta on a `lower_better` metric is a regression) without a
/// registry re-join.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkspaceMetricTrendRow {
    pub metric: String,
    pub period: chrono::NaiveDate,
    pub value: f64,
    pub prior: Option<f64>,
    pub delta: Option<f64>,
    pub direction: String,
}

/// One point in a workspace metric's time series at a chosen grain — the shape
/// [`PgStore::get_workspace_metric_series`] returns. `period` is the grain's bucket
/// start (`date` for daily; the week/month/quarter start otherwise); `value` is
/// the view's re-derived value (Σnum/Σden for ratio/pct, Σ for count/currency,
/// period-end for value/score — NEVER the mean of daily ratios). `direction`
/// travels with the row so the UI can colour the series without a registry re-join.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkspaceMetricSeriesPoint {
    pub period: chrono::NaiveDate,
    pub value: f64,
    pub direction: String,
    /// The per-datapoint explainer — the one-line "why this day's value is what it
    /// is" companion (`props->>'explainer'`), present only at DAILY grain and only
    /// once its day has been computed. `None` at coarser grains (the explainer is a
    /// per-day artifact) and for a day not yet enriched. Never fabricated.
    pub explainer: Option<String>,
}

/// A workspace metric's series at a grain bundled with the metric's `formula` — the
/// "how it's calculated" facet the metric-detail screen surfaces beside the chart.
/// The shape [`PgStore::get_workspace_metric_series`] returns. `formula` is read from
/// the `sensei.metrics` registry by key, so it is present even when `points` is
/// empty (a valid metric with no data yet) and is `None` only when the key names no
/// registered metric (honest-null, never a fabricated string).
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkspaceMetricSeries {
    pub formula: Option<String>,
    pub points: Vec<WorkspaceMetricSeriesPoint>,
}

/// One row of `sensei.metric_status`: for one repository × one registry metric,
/// should this compute, how far has it got, and if it is not current, WHY. The
/// shape [`PgStore::metric_status`] returns.
///
/// `reason_code` is a `sensei.reason_codes` key in domain `metric_computation`,
/// not a message — the human summary/detail/remedy is served ONCE per read (see
/// [`ReasonCode`]) rather than repeated on all ~1.9k rows. `cadence` is read off
/// `last_sha` by the view rather than a hardcoded group list, so it reports
/// `commit` on its own if the engine ever writes one.
///
/// `sealed_through` / `watermark_updated_at` are `None` for a metric whose group
/// has never run for the repository — an honest gap, and the state the view names
/// `never_computed`. Never a fabricated date.
/// `repo_key` is `Option`: it is the REMOTE identity, and a local-only repository
/// has no remote. Such a repository still computes metrics normally (they are keyed
/// on `repository_id`), but no dōjō can rule on them — activation is decided per
/// `repo_key`. So a null here is not missing data; it is the reason the activation
/// control does not apply, and `repository_id` is the identity to address it by.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricStatusRow {
    pub repository_id: uuid::Uuid,
    pub repo_key: Option<String>,
    pub repository_name: String,
    pub metric: String,
    pub metric_group: String,
    pub cadence: String,
    pub sealed_through: Option<chrono::NaiveDate>,
    pub last_sha: Option<String>,
    pub watermark_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub effective_from: chrono::NaiveDate,
    pub effective_until: Option<chrono::NaiveDate>,
    pub deactivated: bool,
    pub deactivated_observed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub reason_code: String,
}

/// One `sensei.reason_codes` row — the registry that answers "why didn't this
/// happen?" for one domain (see `docs/architecture/reason-codes.md`). The shape
/// [`PgStore::reason_codes`] returns.
///
/// `kind` is the axis that earns the single table: `normal` clears itself,
/// `refusal` is somebody's decision, `fault` needs attention. A `normal` code
/// carries no `remedy` and no `actor` — the DDL enforces it — so a UI can tell
/// fine from broken without a per-domain rule of its own.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReasonCode {
    pub code: String,
    pub kind: String,
    pub precedence: i16,
    pub summary: String,
    pub detail: String,
    pub remedy: Option<String>,
    pub actor: Option<String>,
}

/// The descriptive facets of ONE metric by registry key — its display `name` and
/// its `how_to_read` "what this measures" line. The shape
/// [`PgStore::get_metric_meaning`] returns, read from `sensei.metrics` by key (the
/// same by-key path [`PgStore::get_workspace_metric_series`] reads `formula`
/// through). Grounds the drill-down's per-session observation in the metric's real
/// meaning. `None` from the reader when the key names no registered metric
/// (honest-null, never a fabricated meaning).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricMeaning {
    pub name: String,
    pub how_to_read: String,
}

mod commands;
mod config;
mod dojo;
mod extensions;
pub(crate) mod folders;
mod git_history;
mod governance;
mod graph;
mod graph_push;
mod test_db;
pub use graph_push::GraphCheckout;
mod indexer;
/// Named outside this module because the MCP handler picks the direction and
/// `graph` consumes it — the caller/callee envelope and `call_coverage` must
/// agree on which side of the relation is being counted, so the choice is one
/// shared enum rather than a string each side re-interprets.
pub(crate) use graph::CallDirection;
#[cfg(test)]
pub(crate) use indexer::LibColumns;
pub(crate) use indexer::{Dropped, EdgeColumns, NodeColumns};
mod call_sites;
mod gateway_catalog;
mod library;
mod logs;
mod mcp;
mod memory;
mod metric_reads;
mod metrics;
mod neighbourhood;
mod orm_reads;
mod pruner;
mod schema_store;
pub use call_sites::{CallSiteColumns, StoredCallSite};
pub use logs::Abandonments;
pub use neighbourhood::NeighbourNode;
pub use pruner::PruneReport;
#[cfg(test)]
pub use schema_store::StoredSchemaRef;
pub use schema_store::{StoredDep, StoredSchemaEntity, UnplacedRef};
mod patterns;
mod personas;
/// Named outside this module because THREE callers now share it — the scheduled
/// check, `GET /api/auth/status`, and the sign-in callback — and the two
/// handlers must not each re-invent what a missing row means.
pub(crate) use personas::ForgeTokenRow;
mod playbook;
mod reasons;
mod repo_key;
mod runs;
mod schedules;
mod sessions;
pub(crate) mod sync;
pub(crate) mod tracker_usage;
pub(crate) mod trackers;
mod transcript;
mod workspaces;
pub(crate) use repo_key::normalize_repo_key;
pub(crate) use schedules::{SchedulePatch, StoredSchedule};

#[cfg(test)]
pub(crate) mod graph_seed;
#[cfg(test)]
mod knowledge_tests;
#[cfg(test)]
mod metrics_tests;
#[cfg(test)]
mod neighbourhood_tests;
#[cfg(test)]
mod node_tests;
#[cfg(test)]
mod pack_resolution_tests;
#[cfg(test)]
mod playbook_tests;
#[cfg(test)]
mod pruner_tests;
#[cfg(test)]
mod resolution_tests;
#[cfg(test)]
mod run_tests;
#[cfg(test)]
mod schema_store_tests;
#[cfg(test)]
mod structure_tests;
#[cfg(test)]
pub(crate) mod tests;
#[cfg(test)]
mod tool_calls_tests;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
// PgStore API surface — methods wired up incrementally; SQLx tuple return types
// are inherently verbose and adding an extra layer of type aliases would
// not improve readability at the call sites.
impl PgStore {
    /// Connect to a PostgreSQL database using the shared pool defaults from
    /// [`sensei_bootstrap`] (`DB_POOL_MIN_CONNECTIONS`, `DB_POOL_MAX_CONNECTIONS`,
    /// `DB_POOL_ACQUIRE_TIMEOUT_SECS`, `DB_POOL_IDLE_TIMEOUT_SECS`,
    /// `DB_POOL_MAX_LIFETIME_SECS`).
    ///
    /// The pool is elastic: it keeps `DB_POOL_MIN_CONNECTIONS` warm and grows to
    /// `DB_POOL_MAX_CONNECTIONS` under load. Extras are retired by
    /// `DB_POOL_MAX_LIFETIME_SECS` (an absolute age that nothing resets), which is
    /// what actually returns the pool to the warm floor — `DB_POOL_IDLE_TIMEOUT_SECS`
    /// cannot, because sqlx's FIFO idle queue rotates every connection and resets
    /// its idle clock under even a light trickle of scheduler traffic.
    pub async fn connect(database_url: &str) -> Result<Self, String> {
        Self::connect_with(database_url, DB_POOL_MIN_CONNECTIONS, DB_POOL_MAX_CONNECTIONS).await
    }

    /// Connect with explicit pool bounds. The daemon uses the warm-floor defaults
    /// via [`connect`](Self::connect); tests use a tiny floorless pool (many test
    /// pools run in parallel — a warm floor of 8 each would exhaust Postgres's
    /// connection limit).
    async fn connect_with(database_url: &str, min: u32, max: u32) -> Result<Self, String> {
        let pool = PgPoolOptions::new()
            .min_connections(min)
            .max_connections(max)
            .acquire_timeout(Duration::from_secs(DB_POOL_ACQUIRE_TIMEOUT_SECS))
            .idle_timeout(Duration::from_secs(DB_POOL_IDLE_TIMEOUT_SECS))
            .max_lifetime(Duration::from_secs(DB_POOL_MAX_LIFETIME_SECS))
            // Put `extensions` on the search_path so unqualified references to
            // pgvector's `vector` type and operators (`$n::vector`, `<=>`) resolve.
            // pgvector installs into the `extensions` schema — the Supabase/dbd
            // convention declared in `database/design.yaml` — which isn't
            // on Postgres's default path. Every table this code touches is
            // schema-qualified, so this only affects extension type/operator
            // resolution (and keeps working if a DB has vector in `public`).
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    sqlx_core::query::query("SET search_path TO \"$user\", public, extensions")
                        .execute(&mut *conn)
                        .await
                        .map(|_| ())
                })
            })
            .connect(database_url)
            .await
            .map_err(|e| format!("PgStore connect: {}", e))?;
        Ok(Self { pool })
    }

    /// Connect to the test database — the ONLY constructor tests should use.
    ///
    /// A tiny floorless pool (min 0, max 4): the handler/pg_store test suites spin
    /// up many `PgStore`s in parallel, so a warm floor of
    /// `DB_POOL_MIN_CONNECTIONS` each would blow past Postgres's connection limit.
    /// That is not hypothetical — 80 of 223 pg_store tests failed to connect at
    /// default parallelism while they were still calling [`connect`](Self::connect).
    ///
    /// Defaults to `sensei_test`, the throwaway DB the monorepo convention reserves
    /// for `cargo test` and CI. NEVER default this to `sensei`: every test that
    /// inserts (e.g. `create_test_folder`) would leak into the user's production
    /// data — stray `/_test` rows showing up in the UI is a real example of how
    /// that surfaces. Override with `TEST_DATABASE_URL` for ad-hoc targets (e.g. a
    /// forked snapshot for debugging).
    pub async fn connect_test() -> Result<Self, String> {
        // Its own clone of `sensei_test`, one per test (#183) — see `test_db`.
        let url = test_db::url().await?;
        Self::connect_with(&url, 0, TEST_POOL_MAX_CONNECTIONS).await
    }

    /// The test database, or a failed test (#204).
    ///
    /// For a test that needs `sensei_test`: every place the suite runs has the
    /// database, so a connect failure is a fault to report, never a reason to
    /// pass having run nothing. Its most likely cause on a dev machine is the
    /// connection budget — the suite's pools plus the daemon's against
    /// Postgres's `max_connections` — so the message says so.
    #[cfg(test)]
    pub async fn test_store() -> Self {
        Self::connect_test().await.unwrap_or_else(|e| {
            panic!(
                "could not open this test's database: {e} — a refused or timed-out \
                 connection usually means the suite's pools plus the daemon's exhausted \
                 Postgres's max_connections (#204)"
            )
        })
    }

    /// Get a reference to the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Close the pool: stop handing out connections, then wait for the
    /// checked-out ones to come back and be closed properly.
    ///
    /// Until #212 nothing ever called this. The graceful-shutdown future in
    /// `api::server` awaited only SIGINT, while both real stop paths
    /// (`sensei stop` and `brew services stop sensei`) send SIGTERM, so the
    /// daemon was killed outright and its backends were left for Postgres to
    /// reap on its own schedule. On 2026-09-30 that left 56 orphaned backends
    /// alive with no owning process, the oldest close to three hours old,
    /// against a `max_connections` of 100 — over half the server's budget held
    /// by nothing.
    ///
    /// Idempotent: sqlx's `Pool::close` may be awaited on multiple handles
    /// concurrently. A second call resolves immediately ONCE the pool has
    /// drained; while a connection is still checked out it waits alongside the
    /// first, which is the same wait, not a new one.
    ///
    /// IT CAN WAIT FOR EVER, AND THE CALLER MUST BOUND IT. `Pool::close`
    /// returns a future that completes only when every checked-out connection
    /// has been returned, and it carries no timeout of its own. A task stuck on
    /// a long query would hold shutdown open indefinitely — and because the
    /// SIGTERM handler is process-wide and permanent, a second SIGTERM would be
    /// caught too, leaving the daemon killable only by SIGKILL. Call this inside
    /// a `tokio::time::timeout` so a stuck connection degrades to the old
    /// kill-outright behaviour instead of an unkillable process.
    pub async fn close(&self) {
        self.pool.close().await;
    }

    // ── Config ────────────────────────────────────────────────────────

    /// Columns of `activity.runs` in `Run` field order. `timestamptz` columns
    /// are projected to true RFC-3339 text via `to_json(col)#>>'{}'` (Postgres'
    /// `::text` cast is space-separated, NOT RFC-3339); this matches
    /// `chrono::to_rfc3339()` used elsewhere without pulling chrono into the row
    /// tuple. Shared by every run SELECT.
    const RUN_SELECT: &'static str = "SELECT id, workspace_id, plan_ref, goal, status::text,
                to_json(paused_until)#>>'{}',
                pause_reason, current_phase, current_feature, dojo_session_id,
                max_concurrency,
                to_json(started_at)#>>'{}',
                to_json(completed_at)#>>'{}',
                to_json(heartbeat_at)#>>'{}',
                to_json(created_at)#>>'{}',
                to_json(updated_at)#>>'{}'
           FROM activity.runs";

    /// Map a raw run row tuple to a [`Run`]. `status` arrives as text and is
    /// parsed with [`RelayRunStatus::from_db_str`]; an unknown value is a hard
    /// error (never a silent default) — the enum and the DDL must agree.
    #[allow(clippy::type_complexity)]
    fn map_run_row(
        row: (
            uuid::Uuid,
            Option<uuid::Uuid>,
            String,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<uuid::Uuid>,
            i32,
            String,
            Option<String>,
            Option<String>,
            String,
            String,
        ),
    ) -> Result<Run, String> {
        let (
            id,
            workspace_id,
            plan_ref,
            goal,
            status,
            paused_until,
            pause_reason,
            current_phase,
            current_feature,
            dojo_session_id,
            max_concurrency,
            started_at,
            completed_at,
            heartbeat_at,
            created_at,
            updated_at,
        ) = row;
        let status = RelayRunStatus::from_db_str(&status)
            .ok_or_else(|| format!("unknown run_status from DB: {status:?}"))?;
        Ok(Run {
            id,
            workspace_id,
            plan_ref,
            goal,
            status,
            paused_until,
            pause_reason,
            current_phase,
            current_feature,
            dojo_session_id,
            max_concurrency,
            started_at,
            completed_at,
            heartbeat_at,
            created_at,
            updated_at,
        })
    }

    /// Shared folder query for [`list_folders_by_workspace`] (full tree) and
    /// [`list_root_folders_by_workspace`] (repo roots only). `roots_only` gates
    /// out the `kind:'folder'` descendants at the SQL level so the compact
    /// path never materializes them.
    async fn query_folders_by_workspace(
        &self,
        workspace_id: &uuid::Uuid,
        roots_only: bool,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT f.id, f.kind::text, f.name, f.path, f.abs_path, f.role::text
             FROM sensei.folders f
             JOIN sensei.folder_workspaces fp ON fp.folder_id = f.id
             WHERE fp.workspace_id = $1
               AND ($2 = false OR f.kind::text IN ('git','standalone'))
             ORDER BY f.path",
            )
            .bind(workspace_id)
            .bind(roots_only)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(id, kind, name, path, abs, role)| {
                serde_json::json!({
                    "id": id, "kind": kind, "name": name,
                    "path": path, "abs_path": abs, "role": role,
                })
            })
            .collect())
    }

    /// Extract the source pattern id from a recommendation's `based_on` JSON
    /// (`{"patterns":[<uuid>, ...]}` — the shape the L2 generator writes via
    /// `create_recommendation_full`). Returns `None` when the key is absent,
    /// the array is empty, or the first entry isn't a uuid: a manual rec may
    /// legitimately omit provenance, and the caller treats that as a no-op
    /// rather than an error.
    fn based_on_first_pattern(based_on_json: &str) -> Option<uuid::Uuid> {
        serde_json::from_str::<serde_json::Value>(based_on_json)
            .ok()?
            .get("patterns")?
            .get(0)?
            .as_str()
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
    }

    /// Scan every doc node in a workspace's folders for backtick-wrapped
    /// identifier mentions, cross-reference against `sensei.nodes`, and
    /// materialise the results into `inference.drift_items`. Returns how
    /// many broken references were added, how many drift rows were
    /// resolved (mentions that now resolve back to a live code node),
    /// and how many doc nodes were scanned.
    ///
    /// The identifier extraction lives in `analysis::doc_drift` so it can
    /// be unit-tested without a Postgres round-trip; here we handle the
    /// per-workspace fanout and the persistence.
    /// Node kinds treated as real code symbols for drift resolution and the
    /// `symbol_names` history. One list so the drift `known` query and
    /// `record_symbol_names` can never drift apart. (Bare enum labels; Postgres
    /// coerces them against the `sensei.node_kind` column.)
    const DRIFT_SYMBOL_KINDS: &'static str = "'function','method','class','type','interface','const','module','struct','hook','component','enum','extension'";

    /// Shared read: optionally filter to a workspace, resolve `workspace_ids` → a JSON
    /// array of {id, name}. The workspaces array is aggregated as text and parsed in
    /// Rust (the codebase avoids decoding json columns directly).
    async fn query_corrections(
        &self,
        workspace_filter: Option<&uuid::Uuid>,
    ) -> Result<serde_json::Value, String> {
        let rows: Vec<(
            uuid::Uuid,
            String,
            i32,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<uuid::Uuid>,
            Option<String>,
            String,
        )> = sqlx_core::query_as::query_as(
            "SELECT c.id, c.text, c.count, c.last_seen, c.memory_id, c.suggestion,
                    COALESCE((SELECT json_agg(json_build_object('id', p.id, 'name', p.name) ORDER BY p.name)
                              FROM sensei.workspaces p WHERE p.id = ANY(c.workspace_ids)), '[]'::json)::text
               FROM inference.corrections c
              WHERE ($1::uuid IS NULL OR $1 = ANY(c.workspace_ids))
              ORDER BY c.count DESC, c.last_seen DESC NULLS LAST",
        )
        .bind(workspace_filter)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let out: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|(id, text, count, last_seen, memory_id, suggestion, workspaces_json)| {
                let workspaces: serde_json::Value = serde_json::from_str(&workspaces_json)
                    .unwrap_or_else(|_| serde_json::json!([]));
                serde_json::json!({
                    "id": id,
                    "text": text,
                    "count": count,
                    "lastSeen": last_seen.map(|t| t.to_rfc3339()),
                    "workspaces": workspaces,
                    "memoryId": memory_id,
                    "suggestion": suggestion,
                })
            })
            .collect();
        Ok(serde_json::json!({ "corrections": out }))
    }

    // ── Libraries ────────────────────────────────────────────────────

    /// Active-window predicate for `sensei.metrics`: a metric is live on
    /// `current_date` when today falls in the HALF-OPEN interval
    /// `[effective_from, effective_until)` — `effective_until` is the EXCLUSIVE
    /// last-active boundary, so a metric retired *effective today* is already
    /// inactive (its `task_name` stops being scheduled that same day). Matches the
    /// authoritative sources: `docs/spec/pipeline/metrics.md` and the
    /// `database/ddl/table/sensei/metrics.ddl` column comments. Shared by
    /// [`Self::active_metrics`] and [`Self::active_task_names`] so the two reads
    /// can't drift.
    const ACTIVE_METRIC_PREDICATE: &str = "effective_from <= current_date and (effective_until is null or effective_until > current_date)";

    /// `(phantom_id, survivor_id)` pairs — one per name-duplicate phantom that
    /// [`Self::heal_duplicate_name_workspaces`] would merge. The `= 1` guard ensures
    /// the survivor is the single folder-bearing workspace for that name
    /// (unambiguous), so a two-folder-bearing collision is excluded. Shared by the
    /// heal (which merges each) and [`Self::detect_duplicate_name_phantoms`] (which
    /// reports read-only) so both agree on exactly what a phantom is.
    async fn duplicate_name_phantom_pairs(&self) -> Result<Vec<(uuid::Uuid, uuid::Uuid)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT empty.id, keep.id
               FROM sensei.workspaces empty
               JOIN sensei.workspaces keep
                 ON keep.name = empty.name AND keep.id <> empty.id
              WHERE empty.maturity = 'discovery'
                AND NOT EXISTS (SELECT 1 FROM sensei.folder_workspaces fp WHERE fp.workspace_id = empty.id)
                AND EXISTS     (SELECT 1 FROM sensei.folder_workspaces fp WHERE fp.workspace_id = keep.id)
                AND (SELECT count(*) FROM sensei.workspaces k
                       WHERE k.name = empty.name
                         AND EXISTS (SELECT 1 FROM sensei.folder_workspaces fp WHERE fp.workspace_id = k.id)) = 1",
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }

    /// Candidate rows for [`Self::heal_nested_standalone_roots`]: each mis-scoped
    /// `standalone` root paired with the DEEPEST enclosing git repo it sits inside
    /// (DISTINCT ON + length DESC picks the closest repo, never a grandparent).
    /// `starts_with` is exact-prefix (no LIKE wildcard hazard in paths). Requires
    /// the git repo to already have a workspace so there is somewhere to attribute
    /// to. Tuple: `(standalone_id, standalone_workspace, git_id, git_workspace,
    /// git_root, git_abs_path)`. Shared by the heal (which re-absorbs each) and
    /// [`Self::detect_nested_standalone_roots`] (which reports read-only).
    /// Every `standalone` folder sitting INSIDE a git repository — mis-scoped by
    /// structure, whatever its workspace says.
    ///
    /// This used to also require `s.workspace_id IS DISTINCT FROM g.workspace_id`,
    /// using a divergent workspace as the evidence that the standalone was
    /// mis-scoped. That proxy is now obsolete and actively wrong: a folder
    /// inherits its workspace from its repo anchor at write time (#211), so a
    /// nested standalone's workspace MATCHES its enclosing repo by construction
    /// and the old predicate matched nothing. The repair silently stopped
    /// happening — caught by `audit_repairs_nested_standalone`, which asserts
    /// the kind is repaired, not just the workspace.
    ///
    /// Structure is the better rule anyway. A standalone inside a repo is
    /// mis-scoped because the repo owns that subtree, and that is true whether
    /// or not the two happen to name the same workspace.
    #[allow(clippy::type_complexity)]
    async fn nested_standalone_candidates(
        &self,
    ) -> Result<
        Vec<(uuid::Uuid, Option<uuid::Uuid>, uuid::Uuid, Option<uuid::Uuid>, uuid::Uuid, String)>,
        String,
    > {
        // BOTH workspace columns are OPTIONAL. `sole_workspace_of` is NULL whenever
        // a repository serves no workspace or two, and 4,129 folders are in that
        // state live. Decoding the enclosing repo's workspace as a bare `Uuid`
        // made ONE such repo abort the whole heal pass with a decode error, so
        // nothing anywhere was re-absorbed — the re-parenting does not need a
        // workspace at all, and only the phantom merge below does.
        sqlx_core::query_as::query_as(
            "SELECT DISTINCT ON (s.id)
                    s.id, sensei.sole_workspace_of(s.id), g.id,
                    sensei.sole_workspace_of(g.id), g.root_id, g.abs_path
               FROM sensei.folders s
               JOIN sensei.folders g
                 ON g.kind = 'git'::sensei.folder_kind
                AND s.abs_path <> g.abs_path
                AND starts_with(s.abs_path, g.abs_path || '/')
              WHERE s.kind = 'standalone'::sensei.folder_kind
              ORDER BY s.id, length(g.abs_path) DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    /// Build the FTR headline JSON shared by [`Self::get_workspace_ftr`] and
    /// [`Self::get_holistic_ftr`]. `ftr_14d` / `ftr_14d_prev` are honest-null
    /// when absent — they serialize to JSON `null`, NEVER coerced to a fabricated
    /// `0.0` a caller can't tell from a real 0%. One place so neither getter can
    /// re-introduce the fabrication.
    fn ftr_headline_json(
        ftr_14d: Option<f64>,
        ftr_14d_prev: Option<f64>,
        trend: Vec<f64>,
        sessions_7d: i64,
    ) -> serde_json::Value {
        serde_json::json!({
            "ftr14d": ftr_14d,
            "ftr14dPrev": ftr_14d_prev,
            "ftrTrend": trend,
            "sessions7d": sessions_7d,
        })
    }

    const DOJO_INBOX_SELECT: &'static str =
        "SELECT id, membership_id, kind, title, body, scope, attribution, state, note,
                applied_memory_id, received_at::text, artifact_signature
           FROM sensei.dojo_inbox";

    fn map_dojo_inbox_row(
        row: (
            uuid::Uuid,
            uuid::Uuid,
            String,
            String,
            String,
            serde_json::Value,
            serde_json::Value,
            String,
            Option<String>,
            Option<uuid::Uuid>,
            Option<String>,
            String,
        ),
    ) -> crate::collective::inbox::InboxItem {
        let (
            id,
            membership_id,
            kind,
            title,
            body,
            scope_v,
            attribution_v,
            state,
            note,
            applied_memory_id,
            received_at,
            artifact_signature,
        ) = row;
        // A malformed jsonb is logged and defaulted, never a silent panic on read.
        let scope: dojo_protocol::ArtifactScope = serde_json::from_value(scope_v).unwrap_or_else(|e| {
            tracing::warn!(error = %e, inbox = %id, "dojo inbox: scope jsonb parse failed — defaulting to empty scope");
            dojo_protocol::ArtifactScope::default()
        });
        let attribution: dojo_protocol::Attribution = serde_json::from_value(attribution_v).unwrap_or_else(|e| {
            tracing::warn!(error = %e, inbox = %id, "dojo inbox: attribution jsonb parse failed — defaulting");
            dojo_protocol::Attribution {
                mode: dojo_protocol::AttributionMode::Named,
                author: None, org: None, anonymous_id: None,
            }
        });
        crate::collective::inbox::InboxItem {
            id,
            membership_id,
            kind,
            title,
            body,
            scope,
            attribution,
            state,
            note,
            applied_memory_id,
            received_at,
            artifact_signature,
        }
    }

    /// Flip an inbox row to `applied` (guarded so it never clobbers a concurrent
    /// apply). Errors when the row is unknown or already applied.
    async fn mark_dojo_inbox_applied(
        &self,
        inbox_id: uuid::Uuid,
        memory_id: uuid::Uuid,
    ) -> Result<(), String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.dojo_inbox SET state = 'applied', applied_memory_id = $2, note = NULL, updated_at = now()
              WHERE id = $1 AND applied_memory_id IS NULL")
            .bind(inbox_id).bind(memory_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        if res.rows_affected() == 0 {
            return Err(format!("dojo inbox {inbox_id} not found or already applied"));
        }
        Ok(())
    }

    /// Wall-clock cutoff in unix-ms for `days` back — used by prune_activity's
    /// ts-based paths (assistant_events.ts is bigint ms).
    fn cutoff_millis(&self, days: i32) -> i64 {
        let secs = (chrono::Utc::now() - chrono::Duration::days(days as i64)).timestamp();
        secs.saturating_mul(1000)
    }

    // ── Raw ──────────────────────────────────────────────────────────

    fn map_dojo_row(row: DojoMembershipRow) -> DojoMembership {
        let (
            id,
            registry_url,
            tenant_key,
            dojo_url,
            kind,
            org_slugs,
            role,
            authenticated_via,
            attribution_default,
            sync_status,
            last_seq,
            last_heartbeat_at,
            enabled,
            persona_id,
            session_slot,
        ) = row;
        DojoMembership {
            id,
            registry_url,
            tenant_key,
            dojo_url,
            kind,
            org_slugs,
            role,
            authenticated_via,
            attribution_default,
            sync_status,
            last_seq,
            last_heartbeat_at,
            enabled,
            persona: persona_id.map(|persona_id| MembershipPersona { persona_id, session_slot }),
        }
    }

    /// Callers append a clause on `m.` — the persona join makes `id` and
    /// `created_at` ambiguous unqualified.
    const DOJO_SELECT: &'static str =
        "SELECT m.id, m.registry_url, m.tenant_key, m.dojo_url, m.kind, m.org_slugs, m.role,
                m.authenticated_via, m.attribution_default, m.sync_status,
                m.last_seq, m.last_heartbeat_at::text, m.enabled, mp.persona_id, p.session_slot
           FROM sensei.dojo_memberships m
           LEFT JOIN sensei.dojo_membership_personas mp ON mp.membership_id = m.id
           LEFT JOIN sensei.personas p ON p.id = mp.persona_id";

    /// Collect all folder ids belonging to a workspace, deduped.
    async fn folder_ids_for_workspace(
        &self,
        workspace_id: &uuid::Uuid,
    ) -> Result<Vec<uuid::Uuid>, String> {
        let folders = self.list_folders_by_workspace(workspace_id).await?;
        let mut ids: Vec<uuid::Uuid> =
            folders.iter().filter_map(|f| crate::api::util::json_uuid(&f["id"])).collect();
        // folders.id is the PK so dupes can't occur today, but sort+dedup keeps
        // this robust if list_folders_by_workspace ever grows a join.
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }
}

#[cfg(test)]
mod pool_lifecycle_tests {
    use super::PgStore;

    /// Shutdown runs this twice in the worst case (the signal handler and a
    /// later drop path both reaching for it), so the second call is asserted,
    /// not assumed.
    #[tokio::test]
    async fn closing_the_store_closes_the_pool_and_is_idempotent() {
        let pg = PgStore::test_store().await;

        // A live pool first, so a closed one afterwards means `close` did it
        // rather than the connection never having worked.
        assert!(!pg.pool().is_closed(), "a freshly connected pool is open");
        sqlx_core::query::query("SELECT 1")
            .execute(pg.pool())
            .await
            .expect("a live pool answers a trivial query");

        pg.close().await;

        assert!(pg.pool().is_closed(), "close() must close the sqlx pool");
        let after = sqlx_core::query::query("SELECT 1").execute(pg.pool()).await;
        assert!(after.is_err(), "a closed pool must refuse to hand out connections");

        // Idempotence: the second close must return, not panic or hang.
        pg.close().await;
        assert!(pg.pool().is_closed(), "a second close() leaves the pool closed");
    }
}

// ── The store's row types (#161) ─────────────────────────────────────
//
// Declared HERE, at the path every caller names (`pg_store::FolderRow`), not
// beside their writers: the Rust indexer does not follow `pub use`, so a
// re-exported row was a dangling reference at every use (user decision
// 2026-10-26, docs/decisions.md).

// Written by `config.rs`.
/// A change to a workspace's workflow state, as [`PgStore::upsert_workflow_state`]
/// applies it: each `Some` replaces its column, each `None` keeps the stored value.
pub struct WorkflowStatePatch<'a> {
    pub phase: Option<&'a str>,
    pub plan: Option<&'a str>,
    pub task: Option<&'a str>,
    pub issue: Option<i64>,
    pub checkpoint: Option<&'a str>,
    pub rules_hash: Option<&'a str>,
}

// Written by `folders.rs`.
/// One folder, as [`PgStore::upsert_folder`] writes it. Its repository is never
/// the caller's to name: it is inherited from the repo anchor (#211).
pub struct FolderRow<'a> {
    pub root_id: &'a uuid::Uuid,
    /// A `sensei.folder_kind` label; refreshed on conflict, since it is derived.
    pub kind: &'a str,
    pub name: &'a str,
    pub path: &'a str,
    pub abs_path: &'a str,
    /// Kept on conflict when `None`.
    pub parent_id: Option<&'a uuid::Uuid>,
    /// The workspace root this folder is a member of; assigned on conflict.
    pub workspace_root_id: Option<&'a uuid::Uuid>,
}

// Written by `governance.rs`.
/// One checker run, as [`PgStore::insert_check_run`] appends it (D-CHECKER).
pub struct CheckRunRow<'a> {
    pub folder_id: &'a uuid::Uuid,
    pub rule_statement: &'a str,
    pub checker_ref: &'a str,
    pub command: &'a str,
    /// A `sensei.check_verdict` label.
    pub verdict: &'a str,
    pub exit_code: Option<i32>,
    pub output_tail: &'a str,
}

// Written by `governance.rs`.
/// One consolidated ruleset version, as [`PgStore::insert_consolidated_ruleset`]
/// writes it.
pub struct ConsolidatedRulesetRow<'a> {
    pub scope: &'a str,
    pub version: i32,
    pub content: &'a str,
    pub conflicts: &'a serde_json::Value,
    pub model: Option<&'a str>,
    pub source_hash: &'a str,
    pub status: &'a str,
}

// Written by `graph.rs`.
/// What an edge points at. Resolved, the target IS the node: a name or file
/// beside it would be ignored, so the type cannot carry one. Unresolved, the
/// name and file are the edge's identity (`edges_unique_unresolved`).
#[derive(Clone, Copy)]
pub enum EdgeTarget<'a> {
    Node(&'a uuid::Uuid),
    Unresolved { name: Option<&'a str>, file: Option<&'a str> },
}

impl<'a> EdgeTarget<'a> {
    /// The node when there is one, else the name and file it was recorded under.
    pub fn from_parts(
        id: Option<&'a uuid::Uuid>,
        name: Option<&'a str>,
        file: Option<&'a str>,
    ) -> Self {
        match id {
            Some(id) => Self::Node(id),
            None => Self::Unresolved { name, file },
        }
    }
}

// Written by `graph.rs`.
/// One edge, as [`PgStore::insert_edge_with_props`] writes it (#161).
pub struct EdgeRow<'a> {
    pub folder_id: &'a uuid::Uuid,
    pub source_id: &'a uuid::Uuid,
    pub target: EdgeTarget<'a>,
    /// A `sensei.edge_kind` label.
    pub kind: &'a str,
    /// Merged into the stored props on conflict, never replacing them.
    pub props: &'a serde_json::Value,
}

// Written by `graph.rs`.
/// One row of `sensei.nodes`, named rather than positional (#161).
///
/// `upsert_node` took nine positional arguments and carried a comment defending
/// them: *"The arguments ARE the columns. A struct here would restate the same
/// names one indirection away without removing a single one."* The verbosity
/// claim is true and beside the point. The value is not fewer names — it is that
/// destructuring this EXHAUSTIVELY turns a field added to the producer into a
/// compile error. The positional list could not, which is how
/// `extract_return_type` ran on every rust function for months while no return
/// type reached a column: there was no ninth argument, and nothing failed.
///
/// **No `Default`, deliberately.** `..Default::default()` would relocate the
/// silence rather than remove it: a field added later would quietly take its
/// default at every existing site instead of forcing a decision. That is why
/// the defaulting wrapper this replaced (`upsert_node` → `upsert_node_ex` with
/// `is_exported = false`) is gone — a wrapper whose only job is to supply a
/// default is the same hazard with a friendlier name.
#[derive(Debug, Clone)]
pub struct NodeRow<'a> {
    pub folder_id: &'a uuid::Uuid,
    pub kind: &'a str,
    pub name: &'a str,
    /// Resolved to `nodes.file_id` by the writer, which FAILS CLOSED on a miss
    /// (R13): a node naming an untracked file is a bug in the walk, and minting
    /// a `files` row here would hide it behind a plausible id.
    pub file_path: &'a str,
    pub parent_id: Option<&'a uuid::Uuid>,
    pub signature: Option<&'a str>,
    /// Part of `nodes_unique_identity`, so moving a symbol within its file is a
    /// different node rather than an update.
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
    pub is_exported: bool,
}

// Written by `library.rs`.
/// One page of library documentation, with the identity that dedupes it.
///
/// Named rather than positional (#161): destructured exhaustively by its
/// writer, so a field added here is a compile error until someone binds it.
/// No `Default` — that would relocate the silence to the call sites.
#[derive(Debug, Clone)]
pub struct LibraryPageRow<'a> {
    pub library_id: &'a uuid::Uuid,
    pub title: &'a str,
    pub url: Option<&'a str>,
    pub local_path: Option<&'a str>,
    pub description: Option<&'a str>,
    pub content: Option<&'a str>,
    pub source_type: &'a str,
    pub component: Option<&'a str>,
    pub package_name: Option<&'a str>,
    pub version: Option<&'a str>,
}

// Written by `logs.rs`.
/// One structured log line as it is stored.
///
/// Named rather than positional (#161): destructured exhaustively by its
/// writer, so a field added here is a compile error until someone binds it.
/// No `Default` — that would relocate the silence to the call sites.
#[derive(Debug, Clone)]
pub struct LogRow<'a> {
    pub level: &'a str,
    pub running_on: &'a str,
    pub module: Option<&'a str>,
    pub logged_at: &'a str,
    pub message: &'a str,
    pub context: &'a serde_json::Value,
    pub data: &'a Option<serde_json::Value>,
    pub error: &'a Option<serde_json::Value>,
}

// Written by `logs.rs`.
/// One attempt at a task, as [`PgStore::start_task_execution`] records it
/// (`running`, until completed or failed).
pub struct TaskExecutionStart<'a> {
    /// The queue's id — it restarts at 1 with every daemon session.
    pub task_id: i64,
    pub parent_task_id: Option<i64>,
    /// A `sensei.task_execution_kind` label.
    pub task_kind: &'a str,
    pub folder_path: &'a str,
    pub path: &'a str,
    pub retry_number: i32,
}

// Written by `mcp.rs`.
/// One registered tool, as [`PgStore::upsert_assistant_tool`] writes it.
pub struct AssistantToolRow<'a> {
    pub assistant_family: &'a str,
    /// `builtin` | `mcp`.
    pub source_type: &'a str,
    pub source_key: &'a str,
    pub tool_name: &'a str,
    /// The name the harness records when the tool is called.
    pub invoked_name: &'a str,
    pub description: Option<&'a str>,
    pub server_id: Option<uuid::Uuid>,
}

// Written by `mcp.rs`.
/// One probe of an MCP server, as [`PgStore::upsert_mcp_tool_manifest`] writes it.
pub struct McpManifestRow<'a> {
    pub server_id: &'a uuid::Uuid,
    pub tools: &'a serde_json::Value,
    pub tool_count: i32,
    pub protocol_version: Option<&'a str>,
    pub server_name: Option<&'a str>,
    pub server_version: Option<&'a str>,
    /// Why the probe failed; `None` on success.
    pub error: Option<&'a str>,
}

impl<'a> McpManifestRow<'a> {
    /// A probe that failed: no tools, and the reason.
    pub fn failed(
        server_id: &'a uuid::Uuid,
        no_tools: &'a serde_json::Value,
        error: &'a str,
    ) -> Self {
        Self {
            server_id,
            tools: no_tools,
            tool_count: 0,
            protocol_version: None,
            server_name: None,
            server_version: None,
            error: Some(error),
        }
    }
}

// Written by `mcp.rs`.
/// One discovered MCP server, as [`PgStore::upsert_mcp_server`] writes it.
pub struct McpServerRow<'a> {
    /// `claude` | `zed` | `cursor` | `codex` | `opencode` | `other`.
    pub acp_family: &'a str,
    /// The key in the assistant's config, e.g. `sensei`, `postgres`.
    pub mcp_key: &'a str,
    /// `Some` for the assistant's project scope, `None` for user scope.
    pub workspace_id: Option<uuid::Uuid>,
    /// The absolute path the entry was discovered in.
    pub config_source: &'a str,
    pub command: &'a str,
    /// JSON array of args, from the config.
    pub args: &'a serde_json::Value,
    /// JSON object of env vars, from the config.
    pub env: &'a serde_json::Value,
}

// Written by `transcript.rs`.
/// Whose turns [`PgStore::upsert_transcript_turns`] writes: the same on every
/// turn of one transcript.
pub struct TranscriptSource<'a> {
    /// The capture source, e.g. `claude_code`, `zed`.
    pub source: &'a str,
    pub session_id: &'a str,
    /// The assistant family, e.g. `claude`.
    pub family: &'a str,
    pub provider: Option<&'a str>,
    pub model: Option<&'a str>,
}

// Written by `memory.rs`.
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct MemoryRow<'a> {
    pub workspace_id: Option<&'a uuid::Uuid>,
    pub scope: &'a str,
    pub scope_filter: Option<&'a str>,
    pub mem_type: &'a str,
    pub title: &'a str,
    pub content: &'a str,
    pub impact: Option<&'a str>,
    pub session_id: Option<&'a uuid::Uuid>,
    pub spine_slot: Option<&'a str>,
    pub feature: Option<&'a str>,
}

// Written by `metrics.rs`.
/// One row of `sensei.repository_metrics`, named rather than positional (#161).
///
/// The ten values this carries used to be ten positional arguments, four of them
/// string-ish and two of them adjacent `Option<&str>`. A call site read
/// `SCOPE_USER, None, None, day, GRAIN_DAILY, …`, and transposing the two `None`s
/// — `identity` for `commit_sha` — was invisible to the compiler, to the database
/// and to every test. Naming them makes that transposition unwriteable.
///
/// **No `Default`, deliberately.** `..Default::default()` at a call site is the
/// same silence in a different place: a field added later would quietly take its
/// default at every existing site instead of forcing a decision. Every field is
/// written out, every time; the cost is a longer literal and the benefit is that
/// adding a dimension is a compile error at all 30-odd call sites rather than a
/// value that stops arriving.
#[derive(Debug, Clone)]
pub struct MetricRow<'a> {
    pub metric_id: &'a uuid::Uuid,
    /// The repository the value belongs to. NOT NULL in the table — a metric row
    /// that joins to nothing is not a reading.
    pub repository_id: &'a uuid::Uuid,
    /// `sensei.metric_scope`: `user` | `repo`.
    pub scope: &'a str,
    /// The sub-key within the scope, when the metric has one. Part of the
    /// conflict target, so two identities are two rows.
    pub identity: Option<&'a str>,
    /// The commit a point-in-time reading was taken at, when it has one. Also
    /// part of the conflict target.
    pub commit_sha: Option<&'a str>,
    pub computed_on: chrono::NaiveDate,
    /// `sensei.metric_grain`: `daily` | `session`.
    pub grain: &'a str,
    pub value: f64,
    pub props: &'a serde_json::Value,
    /// `sensei.metric_source`: `measured` | `estimated` | `federated`.
    pub source: &'a str,
}

// Written by `patterns.rs`.
/// One row of `inference.recommendations` — a proposal and its evidence.
///
/// Named rather than positional (#161): destructured exhaustively by its
/// writer, so a field added here is a compile error until someone binds it.
/// No `Default` — that would relocate the silence to the call sites.
#[derive(Debug, Clone)]
pub struct RecommendationRow<'a> {
    pub workspace_id: &'a uuid::Uuid,
    pub title: &'a str,
    pub why: &'a str,
    pub impact: Option<&'a str>,
    pub action_type: &'a str,
    pub urgency: &'a str,
    pub based_on: &'a serde_json::Value,
    pub reasoning_trace_id: Option<&'a uuid::Uuid>,
    pub prompt: Option<&'a str>,
}

// Written by `playbook.rs`.
/// One row of `sensei.playbook_runs` — a classification and what produced it.
///
/// Named rather than positional (#161): destructured exhaustively by its
/// writer, so a field added here is a compile error until someone binds it.
/// No `Default` — that would relocate the silence to the call sites.
#[derive(Debug, Clone)]
pub struct PlaybookRunRow<'a> {
    pub session_id: Option<uuid::Uuid>,
    pub feature: Option<&'a str>,
    pub lifecycle: &'a str,
    pub intent: &'a str,
    pub risk: &'a str,
    pub rule_id: Option<uuid::Uuid>,
    pub playbook: &'a str,
    pub rationale: &'a str,
    pub confirmed: bool,
    pub classified_by: Option<&'a str>,
    pub model_fallback: bool,
    pub workspace_id: uuid::Uuid,
}

// Written by `sessions.rs`.
///
/// Named rather than positional (#161): destructured exhaustively by its
/// writer, so a field added here is a compile error until someone binds it.
/// No `Default` — that would relocate the silence to the call sites.
#[derive(Debug, Clone)]
pub struct HookEventRow<'a> {
    pub session_id: &'a str,
    pub assistant_family: &'a str,
    pub event_type: &'a str,
    pub tool_name: Option<&'a str>,
    pub cwd: Option<&'a str>,
    pub ts: i64,
    pub success: Option<bool>,
    pub payload: &'a serde_json::Value,
}

// Written by `sessions.rs`.
/// The measured outcome of one session, written when it closes.
///
/// Named rather than positional (#161): destructured exhaustively by its
/// writer, so a field added here is a compile error until someone binds it.
/// No `Default` — that would relocate the silence to the call sites.
#[derive(Debug, Clone)]
pub struct SessionMetricsRow<'a> {
    pub session_id: &'a uuid::Uuid,
    pub turns: i32,
    pub corrections: i32,
    pub outcome: &'a str,
    pub ftr: bool,
    /// Active time and its R12 split; `None` writes NULL (no hook events).
    pub duration_ms: Option<i64>,
    pub claude_ms: Option<i64>,
    pub user_ms: Option<i64>,
    pub module: Option<&'a str>,
    pub tool_usage: &'a serde_json::Value,
}

// Written by `sessions.rs`.
/// How a session ended, written once at completion.
///
/// Named rather than positional (#161): destructured exhaustively by its
/// writer, so a field added here is a compile error until someone binds it.
/// No `Default` — that would relocate the silence to the call sites.
#[derive(Debug, Clone)]
pub struct SessionOutcomeRow<'a> {
    pub id: &'a uuid::Uuid,
    pub outcome: &'a str,
    pub ftr: bool,
    pub turns: i32,
    pub corrections: i32,
    pub summary: Option<&'a str>,
    pub tokens_in: Option<i64>,
    pub tokens_out: Option<i64>,
}

// Written by `extensions.rs`.
/// One extension, as [`PgStore::create_extension`] inserts it.
pub struct ExtensionRow<'a> {
    /// A `sensei.extension_kind` label: plugin, skill, command, agent or hook.
    pub kind: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub content: Option<&'a str>,
    /// A `sensei.extension_scope` label: global, workspace or folder.
    pub scope: &'a str,
    /// A `sensei.extension_source` label: builtin, marketplace or local.
    pub source: &'a str,
}

// Written by `patterns.rs`.
/// One detected pattern, as [`PgStore::upsert_pattern`] writes it. Keyed by
/// `(workspace_id, name, is_anti)`; `instance_count` is derived from `instances`.
pub struct PatternRow<'a> {
    pub workspace_id: &'a uuid::Uuid,
    /// Kept on conflict when `None`.
    pub folder_id: Option<&'a uuid::Uuid>,
    pub name: &'a str,
    /// An anti-pattern rather than a pattern.
    pub is_anti: bool,
    /// Kept on conflict when `None`.
    pub confidence: Option<f64>,
    /// A JSON array of the occurrences.
    pub instances: &'a serde_json::Value,
}

// Written by `library.rs`.
/// One library and one release of it, as [`PgStore::upsert_library`] writes
/// them: `name`, `ecosystem` and `description` on `libraries`; `version`,
/// `source_type` and `base_url` on that release's `library_versions` row. A
/// `None` keeps what is stored.
pub struct LibraryRow<'a> {
    pub name: &'a str,
    /// A `sensei.library_ecosystem` label.
    pub ecosystem: &'a str,
    pub version: Option<&'a str>,
    pub description: Option<&'a str>,
    /// A `sensei.library_source_type` label.
    pub source_type: Option<&'a str>,
    pub base_url: Option<&'a str>,
}

// Written by `sessions.rs`.
/// A session's model and token usage, as [`PgStore::set_session_metadata`]
/// applies it. Each `None` keeps what is stored; `split` carries the
/// fresh/cache/reasoning breakdown and the metered cost when the source has one.
pub struct SessionMetadataPatch<'a> {
    pub provider: Option<&'a str>,
    pub model: Option<&'a str>,
    /// All input the model processed, cache included.
    pub tokens_in: Option<i64>,
    pub tokens_out: Option<i64>,
    pub split: Option<&'a crate::transcript::SessionTokens>,
}

// Written by `sessions.rs`.
/// One progress snapshot, as [`PgStore::create_snapshot`] inserts it.
pub struct SnapshotRow<'a> {
    pub session_id: &'a uuid::Uuid,
    pub folder_id: &'a uuid::Uuid,
    /// A `sensei.snapshot_kind` label: manual or checkpoint.
    pub kind: &'a str,
    pub progress: &'a str,
    pub next_step: Option<&'a str>,
    pub completed_steps: &'a [String],
}

// Written by `patterns.rs`.
/// One multi-model reasoning trace, as [`PgStore::insert_reasoning_trace`]
/// inserts it.
pub struct ReasoningTraceRow<'a> {
    pub workspace_id: Option<&'a uuid::Uuid>,
    pub trigger_event: &'a str,
    pub trigger_detail: &'a serde_json::Value,
    pub models_used: &'a [String],
    pub exchanges: &'a serde_json::Value,
    pub consensus: &'a serde_json::Value,
}

// Written by `metrics.rs`.
/// One benchmark run, as [`PgStore::create_benchmark_report`] inserts it.
pub struct BenchmarkReportRow<'a> {
    pub folder_id: Option<&'a uuid::Uuid>,
    pub run_name: &'a str,
    pub strategy: &'a str,
    pub score: Option<f64>,
    pub tokens: Option<i32>,
    pub elapsed_ms: Option<i32>,
}

// Written by `dojo.rs`.
/// One outbox entry the Dōjō accepted, as [`PgStore::outbox_mark_sent`]
/// records it (state `sent`).
pub struct OutboxSentRow<'a> {
    pub membership_id: &'a uuid::Uuid,
    pub batch_id: Option<&'a uuid::Uuid>,
    pub memory_id: Option<&'a uuid::Uuid>,
    /// The entry's identity within its membership.
    pub signature: &'a str,
    /// The Dōjō's sequence number for it.
    pub sent_seq: i64,
    /// The id the Dōjō gave it.
    pub remote_id: &'a str,
}

// Written by `patterns.rs`.
/// One cached piece of narration copy, as [`PgStore::upsert_narration_cache`]
/// writes it. Keyed by `(kind, facts_hash)`; a later write replaces it.
pub struct NarrationRow<'a> {
    pub kind: &'a str,
    /// The hash of the facts the copy describes.
    pub facts_hash: &'a str,
    pub title: &'a str,
    pub detail: &'a str,
    pub model_provider: Option<&'a str>,
    pub model_id: Option<&'a str>,
}

// Written by `patterns.rs`.
/// How a recommendation becomes a rule, as
/// [`PgStore::accept_recommendation_as_rule`] applies it.
pub struct RuleAcceptance<'a> {
    pub namespace_id: Option<&'a uuid::Uuid>,
    pub enforcement: Option<&'a str>,
    /// The governance scope the rule is filed under.
    pub gov_scope: &'a str,
    /// Replaces the recommendation's title when given.
    pub title_override: Option<&'a str>,
    /// Replaces the recommendation's body when given.
    pub body_override: Option<&'a str>,
}

// Written by `trackers.rs`.
/// One discovery pass's result for a tracker, as [`PgStore::apply_discovered`]
/// applies it. Applied only while `secret_service` is still the tracker's.
pub struct DiscoveryApply<'a> {
    pub secret_service: &'a str,
    pub discovered: &'a crate::tracker::Discovered,
    pub links: &'a [crate::tracker::Link],
    /// Keys the tracker said do not exist.
    pub not_found: &'a [String],
    pub watermark: Option<chrono::DateTime<chrono::Utc>>,
}

// Written by `folders.rs`.
/// One structural subfolder, as [`PgStore::upsert_subfolder`] writes it: a
/// `folder` (a navigable directory) or a `module` (a manifest-bearing build
/// unit). Its repository is inherited from the repo anchor, never named here.
pub struct SubfolderRow<'a> {
    pub root_id: &'a uuid::Uuid,
    /// `folder` or `module`; a path that is already a repository root keeps its kind.
    pub kind: &'a str,
    pub name: &'a str,
    pub path: &'a str,
    pub abs_path: &'a str,
    pub parent_id: Option<&'a uuid::Uuid>,
}

// Written by `transcript.rs`.
/// A transcript file's ingest cursor after its turns were written, as
/// [`PgStore::set_capture_watermark`] records it (and clears `untracked`).
pub struct CaptureWatermarkRow<'a> {
    pub source: &'a str,
    pub file_path: &'a str,
    pub session_id: Option<&'a str>,
    /// The source mtime (ns) the file was read at.
    pub mtime_ns: i64,
    /// How many turns were WRITTEN, not parsed (#125).
    pub turns: i32,
}

// Written by `dojo.rs`.
/// An outbox entry's state short of `sent`, as [`PgStore::outbox_mark_state`]
/// records it. A `sent` entry is never moved back.
pub struct OutboxStateRow<'a> {
    pub membership_id: &'a uuid::Uuid,
    pub batch_id: Option<&'a uuid::Uuid>,
    pub memory_id: Option<&'a uuid::Uuid>,
    pub signature: &'a str,
    /// `held`, `queued`, `error` or `pending`; anything else is refused.
    pub state: &'a str,
}

// Written by `mcp.rs`.
/// One installed service, as [`PgStore::upsert_service`] writes it (keyed by `name`).
pub struct ServiceRow<'a> {
    pub name: &'a str,
    pub display_name: &'a str,
    /// A `sensei.service_kind` label.
    pub kind: &'a str,
    /// A `sensei.service_protocol` label.
    pub protocol: &'a str,
    pub config: &'a serde_json::Value,
}

// Written by `workspaces.rs`.
/// What a workspace says about itself, as [`PgStore::set_workspace_identity`]
/// applies it: a `None` or empty field keeps what is stored; tags are added to.
pub struct WorkspaceIdentity<'a> {
    pub description: Option<&'a str>,
    pub client: Option<&'a str>,
    pub stack: &'a [String],
    pub tags: &'a [String],
}
