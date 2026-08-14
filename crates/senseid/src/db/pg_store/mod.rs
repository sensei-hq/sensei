use std::time::Duration;
use sqlx_postgres::{PgPool, PgPoolOptions};
use sensei_bootstrap::{DB_POOL_MAX_CONNECTIONS, DB_POOL_MIN_CONNECTIONS, DB_POOL_ACQUIRE_TIMEOUT_SECS, DB_POOL_IDLE_TIMEOUT_SECS};
use dojo_protocol::relay::RelayRunStatus;
use crate::runs::{NewRun, Run, RunEvent, RunEventKind};

/// PostgreSQL store.
/// Schema is managed by `dbd apply`, not by this code.
#[derive(Clone)]
pub struct PgStore {
    pool: PgPool,
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

/// Valid `projects.maturity` lifecycle values — mirrors the
/// `sensei.project_maturity` enum in
/// `database/ddl/enum/sensei/project_maturity.ddl`. Used to reject an unknown
/// maturity with a clear error (and a 400 at the HTTP layer) instead of
/// leaking a raw Postgres enum-cast failure as a 500.
pub const PROJECT_MATURITIES: [&str; 4] = ["discovery", "active", "maintenance", "archived"];

/// Partial update for a project's editable identity fields (the About screen).
/// Every field is optional: `None` leaves the column unchanged (COALESCE
/// semantics), matching the About form which strips empty inputs and PUTs only
/// the fields the user touched. Text columns bind `Option<&str>`; the three
/// jsonb columns (`icon`/`stack`/`links`) bind `Option<&serde_json::Value>`;
/// `maturity` is the `sensei.project_maturity` enum and is validated against
/// [`PROJECT_MATURITIES`] before the write.
#[derive(Debug, Default, Clone)]
pub struct ProjectPatch<'a> {
    pub name:          Option<&'a str>,
    pub description:   Option<&'a str>,
    pub maturity:      Option<&'a str>,
    pub client:        Option<&'a str>,
    pub goal:          Option<&'a str>,
    pub preferred_acp: Option<&'a str>,
    pub icon:          Option<&'a serde_json::Value>,
    pub stack:         Option<&'a serde_json::Value>,
    pub links:         Option<&'a serde_json::Value>,
}

/// Per-table row counts from a single `PgStore::prune_activity` run (#74).
/// Everything is `u64` so callers can report a single log line without
/// per-field conversions.
#[derive(Debug, Default, Clone, Copy, serde::Serialize)]
pub struct ActivityPruneCounts {
    pub sessions:          u64,
    pub turns:             u64,
    pub transcript_turns:  u64,
    pub assistant_events:  u64,
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
    pub project_id:    Option<uuid::Uuid>,
    pub scope:         String,
    pub scope_filter:  Option<String>,
    pub mtype:         String,    // memory_type enum value
    pub title:         String,
    pub content:       String,
    pub impact:        Option<String>,
    pub tags:          Vec<String>,
    pub triage_signal: Option<String>,
    pub status:        String,    // memory_status enum value
    // Governance plane: where the rule applies (namespace) + its authority.
    pub namespace_id:  Option<uuid::Uuid>,
    pub enforcement:   Option<String>, // enforcement enum value; None → DB default 'recommended'
    pub origin:        Option<String>, // None → DB default 'learned'
    pub source_id:     Option<uuid::Uuid>, // provenance: knowledge_sources.id for origin='federated'
    // Spine anchoring (memory-anchoring design 2026-07-18): which doc-slot this
    // memory belongs to, and — for feature-scoped slots — which feature. Both
    // nullable; None/None = unanchored.
    pub spine_slot:    Option<String>, // sensei.spine_slot enum value
    pub feature:       Option<String>,
}

pub struct OutcomeRow {
    pub memory_id:  uuid::Uuid,
    pub session_id: Option<uuid::Uuid>,
    pub outcome:    String,
    pub context:    Option<String>,
}

/// Input for registering a federation endpoint.
pub struct NewKnowledgeSource {
    pub kind:           String,
    pub name:           String,
    pub url:            String,
    pub namespace_id:   Option<uuid::Uuid>,
    pub credential_ref: String,
    pub direction:      String, // push | pull | both
}

/// A registered federation endpoint (row of sensei.knowledge_sources).
#[derive(Debug, Clone)]
pub struct KnowledgeSource {
    pub id:             uuid::Uuid,
    pub kind:           String,
    pub name:           String,
    pub url:            String,
    pub namespace_id:   Option<uuid::Uuid>,
    pub credential_ref: String,
    pub direction:      String,
    pub last_seq:       i64,
    pub enabled:        bool,
}

/// Input for registering a daemon-side Dōjō connection
/// (`sensei.dojo_memberships`). The device token is stored in the OS Keychain
/// (referenced by `credential_ref`), never here — mirrors [`NewKnowledgeSource`].
pub struct NewDojoMembership {
    /// The service membership id (`dojo.memberships.id`); becomes the local PK
    /// and the value `sensei.projects.dojo_id` points at. Service-assigned.
    pub id:                  uuid::Uuid,
    pub registry_url:        String,
    pub tenant_key:          String,
    pub dojo_url:            String,
    pub kind:                String, // employer | client | community | personal
    /// Git-remote owner slugs this membership covers (lowercased) — feeds
    /// infer-at-detect auto-bind. Empty for memberships with no org coverage.
    pub org_slugs:           Vec<String>,
    pub role:                String,
    pub authenticated_via:   String,
    pub attribution_default: String,
    pub credential_ref:      String,
    pub sync_status:         String,
}

/// A daemon-side Dōjō connection (row of `sensei.dojo_memberships`). Carries
/// `credential_ref` for C6/C7 token resolution; the API view omits it.
#[derive(Debug, Clone)]
pub struct DojoMembership {
    pub id:                  uuid::Uuid,
    pub registry_url:        String,
    pub tenant_key:          String,
    pub dojo_url:            String,
    pub kind:                String,
    /// Git-remote owner slugs this membership covers (lowercased). Drives
    /// infer-at-detect auto-bind (see `dojo/routing.rs::infer_binding`).
    pub org_slugs:           Vec<String>,
    pub role:                String,
    pub authenticated_via:   String,
    pub attribution_default: String,
    pub credential_ref:      String,
    pub sync_status:         String,
    pub last_seq:            i64,
    /// RFC-3339 text (SELECTed as `::text`) so the API can serialize it without
    /// pulling chrono into the row tuple. `None` until the first heartbeat.
    pub last_heartbeat_at:   Option<String>,
    pub enabled:             bool,
}

/// A row of `sensei.collective_preferences` — the single-row collective sharing
/// settings backing `GET/PUT /api/preferences/collective`. `categories` is the
/// raw jsonb toggle map; `updated_at` is RFC-3339 text (SELECTed `::text`).
#[derive(Debug, Clone)]
pub struct CollectivePrefsRow {
    pub destination:         String,
    pub cadence:             String,
    pub categories:          serde_json::Value,
    pub attribution_default: String,
    pub updated_at:          String,
}

/// A federated_memories ledger row.
#[derive(Debug, Clone)]
pub struct FederatedLink {
    pub memory_id:  Option<uuid::Uuid>,
    pub remote_seq: i64,
}

/// One member memory of a share batch, in the shape the C6 upstream-contribute
/// path needs to build an artifact. `body` is the portable text that will be
/// confidentiality-checked before it leaves the machine: the `generalised_content`
/// rewrite when present, else the raw `content` (the deterministic dereference
/// runs regardless — a raw content is still gated, never trusted).
#[derive(Debug, Clone)]
pub struct ShareBatchItem {
    pub memory_id:   uuid::Uuid,
    pub title:       String,
    pub body:        String,
    /// `sensei.memory_type` string — drives artifact-kind mapping (pattern → the
    /// `pattern` artifact; everything else → `principle`).
    pub memory_type: String,
}

/// Snapshot needed to publish a memory to a dojo (+ namespace identity + origin/scope_key for gating).
#[derive(Debug, Clone)]
pub struct MemoryPushPayload {
    pub title:       String,
    pub content:     String,
    pub impact:      Option<String>,
    pub enforcement: String,
    pub rule_type:   String,
    pub origin:      String,
    pub scope_key:   String,
    pub slug:        String,
    pub name:        String,
}

/// Parse a git remote URL into its source-identifier tokens (owner + repo),
/// e.g. `git@github.com:acme/acme-api.git` and
/// `https://github.com/acme/acme-api` both → `["acme", "acme-api"]`. Used by
/// [`PgStore::project_identifiers`] to feed the confidentiality dereference
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
/// no owner segment → `None`. Feeds [`PgStore::project_org_owners`] →
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
    pub id:              uuid::Uuid,
    pub key:             String,
    pub name:            String,
    pub description:     String,
    pub family:          String,
    pub metric_type:     String,
    pub unit:            Option<String>,
    pub direction:       String,
    pub purpose:         String,
    pub how_to_read:     String,
    pub formula:         String,
    pub task_name:       String,
    pub weight:          f64,
    pub target:          Option<f64>,
    pub effective_from:  chrono::NaiveDate,
    pub effective_until: Option<chrono::NaiveDate>,
}

/// Latest stored value for one metric of a project, with the catalog facets it is
/// read through — the shape [`PgStore::get_project_metrics`] returns. `value`/`props`
/// come from `sensei.project_metric_daily` (latest `date` per metric); `name`,
/// `metric_type`, `unit`, `direction`, `purpose`, `how_to_read` are joined from
/// `sensei.metrics`. Trend (prior/delta) is deferred to the Phase 7 endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectMetricRow {
    pub metric:      String,
    pub date:        chrono::NaiveDate,
    pub value:       f64,
    pub props:       serde_json::Value,
    pub name:        String,
    pub metric_type: String,
    pub unit:        Option<String>,
    pub direction:   String,
    pub purpose:     String,
    pub how_to_read: String,
}

/// The latest weekly trend point for one metric of a project — the shape
/// [`PgStore::get_project_metric_trend`] returns. Read from
/// `sensei.project_metric_trend` (the weekly `lag()` over `project_metric_weekly`):
/// `prior`/`delta` are `None` for a metric's first period (honest-null, never a
/// fabricated 0). `direction` travels with the row so the UI can colour the delta
/// (a positive delta on a `lower_better` metric is a regression) without a
/// registry re-join.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectMetricTrendRow {
    pub metric:    String,
    pub period:    chrono::NaiveDate,
    pub value:     f64,
    pub prior:     Option<f64>,
    pub delta:     Option<f64>,
    pub direction: String,
}

/// One point in a project metric's time series at a chosen grain — the shape
/// [`PgStore::get_project_metric_series`] returns. `period` is the grain's bucket
/// start (`date` for daily; the week/month/quarter start otherwise); `value` is
/// the view's re-derived value (Σnum/Σden for ratio/pct, Σ for count/currency,
/// period-end for value/score — NEVER the mean of daily ratios). `direction`
/// travels with the row so the UI can colour the series without a registry re-join.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectMetricSeriesPoint {
    pub period:    chrono::NaiveDate,
    pub value:     f64,
    pub direction: String,
    /// The per-datapoint explainer — the one-line "why this day's value is what it
    /// is" companion (`props->>'explainer'`), present only at DAILY grain and only
    /// once its day has been computed. `None` at coarser grains (the explainer is a
    /// per-day artifact) and for a day not yet enriched. Never fabricated.
    pub explainer: Option<String>,
}

/// A project metric's series at a grain bundled with the metric's `formula` — the
/// "how it's calculated" facet the metric-detail screen surfaces beside the chart.
/// The shape [`PgStore::get_project_metric_series`] returns. `formula` is read from
/// the `sensei.metrics` registry by key, so it is present even when `points` is
/// empty (a valid metric with no data yet) and is `None` only when the key names no
/// registered metric (honest-null, never a fabricated string).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectMetricSeries {
    pub formula: Option<String>,
    pub points:  Vec<ProjectMetricSeriesPoint>,
}

/// The descriptive facets of ONE metric by registry key — its display `name` and
/// its `how_to_read` "what this measures" line. The shape
/// [`PgStore::get_metric_meaning`] returns, read from `sensei.metrics` by key (the
/// same by-key path [`PgStore::get_project_metric_series`] reads `formula`
/// through). Grounds the drill-down's per-session observation in the metric's real
/// meaning. `None` from the reader when the key names no registered metric
/// (honest-null, never a fabricated meaning).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricMeaning {
    pub name:        String,
    pub how_to_read: String,
}


mod commands;
mod config;
mod dojo;
mod extensions;
mod folders;
mod governance;
mod graph;
mod library;
mod logs;
mod mcp;
mod memory;
mod metrics;
mod patterns;
mod playbook;
mod projects;
mod runs;
mod sessions;
mod transcript;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod knowledge_tests;
#[cfg(test)]
mod run_tests;
#[cfg(test)]
mod playbook_tests;
#[cfg(test)]
mod pack_resolution_tests;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
// PgStore API surface — methods wired up incrementally; SQLx tuple return types
// are inherently verbose and adding an extra layer of type aliases would
// not improve readability at the call sites.
impl PgStore {
    /// Connect to a PostgreSQL database using the shared pool defaults from
    /// [`sensei_bootstrap`] (`DB_POOL_MIN_CONNECTIONS`, `DB_POOL_MAX_CONNECTIONS`,
    /// `DB_POOL_ACQUIRE_TIMEOUT_SECS`, `DB_POOL_IDLE_TIMEOUT_SECS`).
    ///
    /// The pool is elastic: it keeps `DB_POOL_MIN_CONNECTIONS` warm, grows to
    /// `DB_POOL_MAX_CONNECTIONS` under load, and reaps the extras back to the warm
    /// floor once they sit idle past `DB_POOL_IDLE_TIMEOUT_SECS`.
    pub async fn connect(database_url: &str) -> Result<Self, String> {
        let pool = PgPoolOptions::new()
            .min_connections(DB_POOL_MIN_CONNECTIONS)
            .max_connections(DB_POOL_MAX_CONNECTIONS)
            .acquire_timeout(Duration::from_secs(DB_POOL_ACQUIRE_TIMEOUT_SECS))
            .idle_timeout(Duration::from_secs(DB_POOL_IDLE_TIMEOUT_SECS))
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

    /// Connect to the test database. Uses TEST_DATABASE_URL or defaults to sensei_test.
    pub async fn connect_test() -> Result<Self, String> {
        let url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| format!("postgresql://localhost:{}/sensei_test", sensei_bootstrap::POSTGRES_PORT));
        Self::connect(&url).await
    }

    /// Get a reference to the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ── Config ────────────────────────────────────────────────────────

    /// Columns of `activity.runs` in `Run` field order. `timestamptz` columns
    /// are projected to true RFC-3339 text via `to_json(col)#>>'{}'` (Postgres'
    /// `::text` cast is space-separated, NOT RFC-3339); this matches
    /// `chrono::to_rfc3339()` used elsewhere without pulling chrono into the row
    /// tuple. Shared by every run SELECT.
    const RUN_SELECT: &'static str =
        "SELECT id, project_id, plan_ref, goal, status::text,
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
            uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, Option<String>,
            Option<String>, Option<String>, Option<String>, Option<uuid::Uuid>,
            i32, String, Option<String>, Option<String>, String, String,
        ),
    ) -> Result<Run, String> {
        let (
            id, project_id, plan_ref, goal, status, paused_until, pause_reason,
            current_phase, current_feature, dojo_session_id, max_concurrency,
            started_at, completed_at, heartbeat_at, created_at, updated_at,
        ) = row;
        let status = RelayRunStatus::from_db_str(&status)
            .ok_or_else(|| format!("unknown run_status from DB: {status:?}"))?;
        Ok(Run {
            id, project_id, plan_ref, goal, status, paused_until, pause_reason,
            current_phase, current_feature, dojo_session_id, max_concurrency,
            started_at, completed_at, heartbeat_at, created_at, updated_at,
        })
    }

    /// Shared folder query for [`list_folders_by_project`] (full tree) and
    /// [`list_root_folders_by_project`] (repo roots only). `roots_only` gates
    /// out the `kind:'folder'` descendants at the SQL level so the compact
    /// path never materializes them.
    async fn query_folders_by_project(&self, project_id: &uuid::Uuid, roots_only: bool) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, path, abs_path, role::text
             FROM sensei.folders
             WHERE project_id = $1
               AND ($2 = false OR kind::text IN ('git','standalone'))
             ORDER BY path"
        ).bind(project_id).bind(roots_only).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, path, abs, role)| {
            serde_json::json!({
                "id": id, "kind": kind, "name": name,
                "path": path, "abs_path": abs, "role": role,
            })
        }).collect())
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

    /// Scan every doc node in a project's folders for backtick-wrapped
    /// identifier mentions, cross-reference against `sensei.nodes`, and
    /// materialise the results into `inference.drift_items`. Returns how
    /// many broken references were added, how many drift rows were
    /// resolved (mentions that now resolve back to a live code node),
    /// and how many doc nodes were scanned.
    ///
    /// The identifier extraction lives in `analysis::doc_drift` so it can
    /// be unit-tested without a Postgres round-trip; here we handle the
    /// per-project fanout and the persistence.
    /// Node kinds treated as real code symbols for drift resolution and the
    /// `symbol_names` history. One list so the drift `known` query and
    /// `record_symbol_names` can never drift apart. (Bare enum labels; Postgres
    /// coerces them against the `sensei.node_kind` column.)
    const DRIFT_SYMBOL_KINDS: &'static str =
        "'function','method','class','type','interface','const','module','struct','hook','component','enum','extension'";

    /// Shared read: optionally filter to a project, resolve `project_ids` → a JSON
    /// array of {id, name}. The projects array is aggregated as text and parsed in
    /// Rust (the codebase avoids decoding json columns directly).
    async fn query_corrections(
        &self,
        project_filter: Option<&uuid::Uuid>,
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
                              FROM sensei.projects p WHERE p.id = ANY(c.project_ids)), '[]'::json)::text
               FROM inference.corrections c
              WHERE ($1::uuid IS NULL OR $1 = ANY(c.project_ids))
              ORDER BY c.count DESC, c.last_seen DESC NULLS LAST",
        )
        .bind(project_filter)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let out: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|(id, text, count, last_seen, memory_id, suggestion, projects_json)| {
                let projects: serde_json::Value =
                    serde_json::from_str(&projects_json).unwrap_or_else(|_| serde_json::json!([]));
                serde_json::json!({
                    "id": id,
                    "text": text,
                    "count": count,
                    "lastSeen": last_seen.map(|t| t.to_rfc3339()),
                    "projects": projects,
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
    const ACTIVE_METRIC_PREDICATE: &str =
        "effective_from <= current_date and (effective_until is null or effective_until > current_date)";

    /// `(phantom_id, survivor_id)` pairs — one per name-duplicate phantom that
    /// [`Self::heal_duplicate_name_projects`] would merge. The `= 1` guard ensures
    /// the survivor is the single folder-bearing project for that name
    /// (unambiguous), so a two-folder-bearing collision is excluded. Shared by the
    /// heal (which merges each) and [`Self::detect_duplicate_name_phantoms`] (which
    /// reports read-only) so both agree on exactly what a phantom is.
    async fn duplicate_name_phantom_pairs(&self) -> Result<Vec<(uuid::Uuid, uuid::Uuid)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT empty.id, keep.id
               FROM sensei.projects empty
               JOIN sensei.projects keep
                 ON keep.name = empty.name AND keep.id <> empty.id
              WHERE empty.maturity = 'discovery'
                AND NOT EXISTS (SELECT 1 FROM sensei.folders f WHERE f.project_id = empty.id)
                AND EXISTS     (SELECT 1 FROM sensei.folders f WHERE f.project_id = keep.id)
                AND (SELECT count(*) FROM sensei.projects k
                       WHERE k.name = empty.name
                         AND EXISTS (SELECT 1 FROM sensei.folders f WHERE f.project_id = k.id)) = 1",
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }

    /// Candidate rows for [`Self::heal_nested_standalone_roots`]: each mis-scoped
    /// `standalone` root paired with the DEEPEST enclosing git repo it sits inside
    /// (DISTINCT ON + length DESC picks the closest repo, never a grandparent).
    /// `starts_with` is exact-prefix (no LIKE wildcard hazard in paths). Requires
    /// the git repo to already have a project so there is somewhere to attribute
    /// to. Tuple: `(standalone_id, standalone_project, git_id, git_project,
    /// git_root, git_abs_path)`. Shared by the heal (which re-absorbs each) and
    /// [`Self::detect_nested_standalone_roots`] (which reports read-only).
    #[allow(clippy::type_complexity)]
    async fn nested_standalone_candidates(
        &self,
    ) -> Result<Vec<(uuid::Uuid, Option<uuid::Uuid>, uuid::Uuid, uuid::Uuid, uuid::Uuid, String)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT DISTINCT ON (s.id)
                    s.id, s.project_id, g.id, g.project_id, g.root_id, g.abs_path
               FROM sensei.folders s
               JOIN sensei.folders g
                 ON g.kind = 'git'::sensei.folder_kind
                AND g.project_id IS NOT NULL
                AND s.abs_path <> g.abs_path
                AND starts_with(s.abs_path, g.abs_path || '/')
              WHERE s.kind = 'standalone'::sensei.folder_kind
                AND s.project_id IS DISTINCT FROM g.project_id
              ORDER BY s.id, length(g.abs_path) DESC",
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }

    /// Build the FTR headline JSON shared by [`Self::get_project_ftr`] and
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
        row: (uuid::Uuid, uuid::Uuid, String, String, String, serde_json::Value, serde_json::Value,
              String, Option<String>, Option<uuid::Uuid>, Option<String>, String),
    ) -> crate::collective::inbox::InboxItem {
        let (id, membership_id, kind, title, body, scope_v, attribution_v, state, note,
             applied_memory_id, received_at, artifact_signature) = row;
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
            id, membership_id, kind, title, body, scope, attribution, state, note,
            applied_memory_id, received_at, artifact_signature,
        }
    }

    /// Flip an inbox row to `applied` (guarded so it never clobbers a concurrent
    /// apply). Errors when the row is unknown or already applied.
    async fn mark_dojo_inbox_applied(&self, inbox_id: uuid::Uuid, memory_id: uuid::Uuid) -> Result<(), String> {
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

    fn map_dojo_row(
        row: (uuid::Uuid, String, String, String, String, Vec<String>, String, String, String, String, String, i64, Option<String>, bool),
    ) -> DojoMembership {
        let (id, registry_url, tenant_key, dojo_url, kind, org_slugs, role, authenticated_via,
             attribution_default, credential_ref, sync_status, last_seq, last_heartbeat_at, enabled) = row;
        DojoMembership {
            id, registry_url, tenant_key, dojo_url, kind, org_slugs, role, authenticated_via,
            attribution_default, credential_ref, sync_status, last_seq, last_heartbeat_at, enabled,
        }
    }

    const DOJO_SELECT: &'static str =
        "SELECT id, registry_url, tenant_key, dojo_url, kind, org_slugs, role, authenticated_via,
                attribution_default, credential_ref, sync_status, last_seq,
                last_heartbeat_at::text, enabled
           FROM sensei.dojo_memberships";

    /// Collect all folder ids belonging to a project, deduped.
    async fn folder_ids_for_project(&self, project_id: &uuid::Uuid) -> Result<Vec<uuid::Uuid>, String> {
        let folders = self.list_folders_by_project(project_id).await?;
        let mut ids: Vec<uuid::Uuid> = folders
            .iter()
            .filter_map(|f| crate::api::util::json_uuid(&f["id"]))
            .collect();
        // folders.id is the PK so dupes can't occur today, but sort+dedup keeps
        // this robust if list_folders_by_project ever grows a join.
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }

}
