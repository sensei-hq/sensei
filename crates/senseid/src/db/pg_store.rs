use std::time::Duration;
use sqlx_postgres::{PgPool, PgPoolOptions};
use sensei_bootstrap::{DB_POOL_MAX_CONNECTIONS, DB_POOL_ACQUIRE_TIMEOUT_SECS, DB_POOL_IDLE_TIMEOUT_SECS};
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
}

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
// PgStore API surface — methods wired up incrementally; SQLx tuple return types
// are inherently verbose and adding an extra layer of type aliases would
// not improve readability at the call sites.
impl PgStore {
    /// Connect to a PostgreSQL database using the shared pool defaults from
    /// [`sensei_bootstrap`] (`DB_POOL_MAX_CONNECTIONS`, `DB_POOL_ACQUIRE_TIMEOUT_SECS`,
    /// `DB_POOL_IDLE_TIMEOUT_SECS`).
    pub async fn connect(database_url: &str) -> Result<Self, String> {
        let pool = PgPoolOptions::new()
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

    pub async fn get_config(&self, key: &str) -> Result<Option<String>, String> {
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT value FROM sensei.config WHERE key = $1"
        )
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    pub async fn set_config(&self, key: &str, value: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.config(key, value) VALUES($1, $2) ON CONFLICT(key) DO UPDATE SET value = EXCLUDED.value"
        )
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Scan exclusions (per watch root) ──────────────────────────────
    // Exclusions live in `folders_to_watch.excluded` — a jsonb array of relative
    // folder names/paths per root (the DDL design). `~/Developer` with
    // `excluded=["Code"]` excludes `~/Developer/Code`. An entry that is a bare
    // name matches that segment anywhere under the root; the absolute-prefix form
    // (root/entry) is precise. Adding an entry prunes the matching subtree;
    // removing one triggers a re-scan (see `update_watch_root` handler).

    /// The absolute-path exclusion prefixes for a watch root — each `excluded`
    /// entry resolved against `root_path` (`root/entry`). Consumed by the scan
    /// (to skip classification) and the watcher (to ignore events).
    pub async fn root_exclusion_prefixes(&self, root_path: &str) -> Result<Vec<String>, String> {
        // Fail closed: a DB error must NOT read as "no exclusions" — that would
        // let the scanner/watcher/grep process folders the user explicitly
        // excluded (indexing/leaking excluded content). Propagate instead.
        let row: Option<(serde_json::Value,)> = sqlx_core::query_as::query_as(
            "SELECT excluded FROM sensei.folders_to_watch WHERE path = $1"
        ).bind(root_path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        let root = root_path.trim_end_matches('/');
        Ok(row.and_then(|(v,)| v.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|e| e.as_str().map(str::to_string))
            .filter(|e| !e.is_empty())
            .map(|e| format!("{root}/{}", e.trim_start_matches('/')))
            .collect())
    }

    /// Watch root's path + its raw (relative) `excluded` list, by id — for the
    /// update handler to diff old-vs-new and prune added / re-scan removed.
    pub async fn get_watch_root(&self, id: &uuid::Uuid) -> Result<Option<(String, Vec<String>)>, String> {
        let row: Option<(String, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT path, excluded FROM sensei.folders_to_watch WHERE id = $1"
        ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(path, ex)| {
            let list = ex.as_array().map(|a| a.iter().filter_map(|e| e.as_str().map(str::to_string)).collect()).unwrap_or_default();
            (path, list)
        }))
    }

    /// Delete every folder at or under `prefix` (cascade nodes/edges/scan_state).
    /// Used when an exclusion is added; the emptied projects are then removed by
    /// [`Self::prune_empty_projects`]. `starts_with` is exact-prefix (no LIKE
    /// wildcard hazard). Returns folders deleted.
    pub async fn prune_under_prefix(&self, prefix: &str) -> Result<u64, String> {
        let p = prefix.trim_end_matches('/');
        let res = sqlx_core::query::query(
            "DELETE FROM sensei.folders f
              WHERE f.abs_path = $1 OR starts_with(f.abs_path, $1 || '/')",
        ).bind(p).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    pub async fn delete_config(&self, key: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.config WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_all_config(&self) -> Result<std::collections::HashMap<String, String>, String> {
        let rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT key, value FROM sensei.config"
        )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().collect())
    }

    // ── Collective sharing preferences (single row) ───────────────────
    //
    // One logical setting for the one local user: the table holds exactly one row
    // guarded by the `singleton` boolean PK (see collective_preferences.ddl).
    // Enum validation lives in `crate::collective::preferences` — these methods
    // only read/upsert.

    /// Read the single collective-preferences row, or `None` when unset (the API
    /// then returns conservative defaults). `categories` comes back as raw jsonb.
    pub async fn get_collective_preferences(&self) -> Result<Option<CollectivePrefsRow>, String> {
        let row: Option<(String, String, serde_json::Value, String, String)> =
            sqlx_core::query_as::query_as(
                "SELECT destination, cadence, categories, attribution_default, updated_at::text
                   FROM sensei.collective_preferences WHERE singleton = true")
            .fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(destination, cadence, categories, attribution_default, updated_at)|
            CollectivePrefsRow { destination, cadence, categories, attribution_default, updated_at }))
    }

    /// Upsert the single collective-preferences row (keys on the `singleton` PK)
    /// and return the new `updated_at`. Callers validate the enum fields first.
    pub async fn set_collective_preferences(
        &self,
        destination: &str,
        cadence: &str,
        categories: &serde_json::Value,
        attribution_default: &str,
    ) -> Result<String, String> {
        let (updated_at,): (String,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.collective_preferences
                (singleton, destination, cadence, categories, attribution_default, updated_at)
             VALUES (true, $1, $2, $3, $4, now())
             ON CONFLICT (singleton) DO UPDATE SET
                destination         = EXCLUDED.destination,
                cadence             = EXCLUDED.cadence,
                categories          = EXCLUDED.categories,
                attribution_default = EXCLUDED.attribution_default,
                updated_at          = now()
             RETURNING updated_at::text")
            .bind(destination).bind(cadence).bind(categories).bind(attribution_default)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(updated_at)
    }

    // ── Insight copy cache (insight-copy pipeline) ────────────────────

    /// Cache read for the insight-copy pipeline. Returns the persisted
    /// `(title, detail)` for `(kind, facts_hash)` and bumps `last_used_at`
    /// in the same statement so hot copy stays warm. `None` on cache miss or
    /// DB error (the caller then generates fresh or falls back to a static
    /// template). DB errors are logged, never swallowed silently.
    pub async fn get_insight_copy(&self, kind: &str, facts_hash: &str) -> Option<(String, String)> {
        let row: Result<Option<(String, String)>, _> = sqlx_core::query_as::query_as(
            "UPDATE sensei.insight_copy SET last_used_at = now() \
             WHERE kind = $1 AND facts_hash = $2 RETURNING title, detail"
        )
            .bind(kind)
            .bind(facts_hash)
            .fetch_optional(&self.pool)
            .await;
        match row {
            Ok(hit) => hit,
            Err(e) => {
                tracing::warn!(error = %e, kind, "get_insight_copy: DB error — treating as cache miss");
                None
            }
        }
    }

    /// Cache write for the insight-copy pipeline. Upserts the generated copy
    /// for `(kind, facts_hash)`; a newer generation wins on conflict and both
    /// timestamps reset. DB errors are logged and swallowed (the caller has
    /// already returned copy to the user — a failed cache write is not fatal).
    pub async fn upsert_insight_copy(
        &self,
        kind: &str,
        facts_hash: &str,
        title: &str,
        detail: &str,
        model_provider: Option<&str>,
        model_id: Option<&str>,
    ) {
        let res = sqlx_core::query::query(
            "INSERT INTO sensei.insight_copy \
               (kind, facts_hash, title, detail, model_provider, model_id) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (kind, facts_hash) DO UPDATE SET \
               title = EXCLUDED.title, detail = EXCLUDED.detail, \
               model_provider = EXCLUDED.model_provider, model_id = EXCLUDED.model_id, \
               generated_at = now(), last_used_at = now()"
        )
            .bind(kind)
            .bind(facts_hash)
            .bind(title)
            .bind(detail)
            .bind(model_provider)
            .bind(model_id)
            .execute(&self.pool)
            .await;
        if let Err(e) = res {
            tracing::warn!(error = %e, kind, "upsert_insight_copy: DB error — copy not cached");
        }
    }

    // ── Tags (controlled vocabulary) ──────────────────────────────────

    pub async fn add_tag(&self, tag: &str, category: Option<&str>) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.tags(tag, category) VALUES($1, $2) ON CONFLICT(tag) DO UPDATE SET category = EXCLUDED.category, modified_at = now()"
        )
            .bind(tag)
            .bind(category)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn remove_tag(&self, tag: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.tags WHERE tag = $1")
            .bind(tag)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn list_tags(&self) -> Result<Vec<(String, Option<String>)>, String> {
        sqlx_core::query_as::query_as("SELECT tag, category FROM sensei.tags ORDER BY tag")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn list_tags_by_category(&self, category: &str) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT tag FROM sensei.tags WHERE category = $1 ORDER BY tag"
        )
            .bind(category)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    // ── Workflow State ────────────────────────────────────────────────

    pub async fn upsert_workflow_state(
        &self, project: &str, phase: Option<&str>, plan: Option<&str>,
        task: Option<&str>, issue: Option<i64>, checkpoint: Option<&str>,
        rules_hash: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.workflow_state(project, active_phase, active_plan, active_task, active_issue, last_checkpoint, rules_hash, updated_at)
             VALUES($1, $2, $3, $4, $5, $6, $7, now())
             ON CONFLICT(project) DO UPDATE SET
               active_phase = COALESCE($2, workflow_state.active_phase),
               active_plan = COALESCE($3, workflow_state.active_plan),
               active_task = COALESCE($4, workflow_state.active_task),
               active_issue = COALESCE($5, workflow_state.active_issue),
               last_checkpoint = COALESCE($6, workflow_state.last_checkpoint),
               rules_hash = COALESCE($7, workflow_state.rules_hash),
               updated_at = now()"
        )
            .bind(project).bind(phase).bind(plan).bind(task)
            .bind(issue).bind(checkpoint).bind(rules_hash)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_workflow_state(&self, project: &str) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(
            Option<String>, Option<String>, Option<String>,
            Option<i32>, Option<String>, Option<String>, chrono::DateTime<chrono::Utc>,
        )> = sqlx_core::query_as::query_as(
            "SELECT active_phase, active_plan, active_task, active_issue, last_checkpoint, rules_hash, updated_at
             FROM sensei.workflow_state WHERE project = $1"
        )
            .bind(project)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(row.map(|(phase, plan, task, issue, checkpoint, hash, updated)| {
            serde_json::json!({
                "project": project,
                "active_phase": phase,
                "active_plan": plan,
                "active_task": task,
                "active_issue": issue,
                "last_checkpoint": checkpoint,
                "rules_hash": hash,
                "updated_at": updated.to_rfc3339(),
            })
        }))
    }

    pub async fn delete_workflow_state(&self, project: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.workflow_state WHERE project = $1")
            .bind(project)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Runs (relay engine run-state — activity.runs / activity.run_events) ─
    //
    // Durable state of an autonomous multi-phase run + its append-only cadence
    // log. `status` is `sensei.run_status` (bound via
    // `RelayRunStatus::as_db_str` cast `$N::sensei.run_status`, mirroring the
    // `$N::sensei.assistant_family` cast on `insert_assistant_event`). `kind` is
    // `sensei.run_event_kind`, bound the same way. Timestamps come back as
    // RFC-3339 `::text` (like `DojoMembership.last_heartbeat_at`). See
    // `crate::runs` for the row types.

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

    /// Create a run. `id`, `status` (`'running'`), and all timestamps are
    /// DB-defaulted; `plan_ref`/`max_concurrency` fall back to the DDL defaults
    /// (`''` / `1`) when the caller passes `None`. Returns the new run id.
    pub async fn create_run(&self, new: &NewRun) -> Result<uuid::Uuid, String> {
        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.runs
                (project_id, plan_ref, goal, dojo_session_id, max_concurrency,
                 author_name, author_email, plan_graph)
             VALUES($1, COALESCE($2, ''), $3, $4, COALESCE($5, 1), $6, $7, $8) RETURNING id"
        )
            .bind(new.project_id)
            .bind(new.plan_ref.as_deref())
            .bind(new.goal.as_deref())
            .bind(new.dojo_session_id)
            .bind(new.max_concurrency)
            .bind(new.author_name.as_deref())
            .bind(new.author_email.as_deref())
            .bind(new.plan_graph.as_ref())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// Read a run's authored plan graph (jsonb), or `None` if the run has none
    /// (ad-hoc/cadence-derived) or does not exist. Kept off the 16-column
    /// `RUN_SELECT` tuple (same reason as `run_author`) and fetched on demand:
    /// only `publish_run` (authored-segment projection) and `update_task_status`
    /// (task-state write-back) need it.
    pub async fn run_plan_graph(
        &self,
        run_id: &uuid::Uuid,
    ) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(Option<serde_json::Value>,)> = sqlx_core::query_as::query_as(
            "SELECT plan_graph FROM activity.runs WHERE id = $1",
        )
            .bind(run_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.and_then(|(g,)| g))
    }

    /// Overwrite a run's authored plan graph (jsonb). Used by `update_task_status`
    /// to persist a task's new state (read-modify-write of the graph). A no-op-safe
    /// full replace — the caller owns merging.
    pub async fn set_run_plan_graph(
        &self,
        run_id: &uuid::Uuid,
        graph: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs SET plan_graph = $2, updated_at = now() WHERE id = $1",
        )
            .bind(run_id)
            .bind(graph)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Read a run's stamped git author `(author_name, author_email)`. Kept off the
    /// wide `RUN_SELECT` tuple because sqlx caps tuple `FromRow` at 16 columns;
    /// `Run` reads stay 16-wide, and the author (a rarely-needed attribution
    /// field) is fetched on demand. `(None, None)` when the run is gone or was
    /// created without a resolvable git identity.
    pub async fn run_author(
        &self,
        run_id: &uuid::Uuid,
    ) -> Result<(Option<String>, Option<String>), String> {
        let row: Option<(Option<String>, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT author_name, author_email FROM activity.runs WHERE id = $1",
        )
            .bind(run_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.unwrap_or((None, None)))
    }

    /// Fetch one run by id, or `None` if it does not exist.
    pub async fn get_run(&self, id: &uuid::Uuid) -> Result<Option<Run>, String> {
        let row: Option<(
            uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, Option<String>,
            Option<String>, Option<String>, Option<String>, Option<uuid::Uuid>,
            i32, String, Option<String>, Option<String>, String, String,
        )> = sqlx_core::query_as::query_as(&format!("{} WHERE id = $1", Self::RUN_SELECT))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        row.map(Self::map_run_row).transpose()
    }

    /// Runs that still need the scheduler's attention — `running`, `paused`,
    /// `stalled`, or `blocked` (uses the partial `runs_active_idx`).
    /// Newest-started first. `blocked` is included so a run waiting on a gate is
    /// still ticked (heartbeat) and shown in `GET /api/runs` — otherwise it
    /// drops out of the active set and looks crashed.
    pub async fn list_active_runs(&self) -> Result<Vec<Run>, String> {
        let rows: Vec<(
            uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, Option<String>,
            Option<String>, Option<String>, Option<String>, Option<uuid::Uuid>,
            i32, String, Option<String>, Option<String>, String, String,
        )> = sqlx_core::query_as::query_as(&format!(
            "{} WHERE status IN ('running', 'paused', 'stalled', 'blocked') ORDER BY started_at DESC",
            Self::RUN_SELECT
        ))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        rows.into_iter().map(Self::map_run_row).collect()
    }

    /// The newest `running` or `stalled` run for a project, if any — the target
    /// of the workflow→run phase bridge ([`Self::advance_run_phase_for_project`]).
    /// `stalled` is included so an agent that went quiet (→ watchdog-stalled) and
    /// then resumes revives its run on the next `update_phase`. `paused`/`blocked`
    /// are excluded — a paused (limit-wait) or gate-blocked run shouldn't be
    /// silently advanced by a stray `update_phase`.
    pub async fn active_run_for_project(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Option<Run>, String> {
        let row: Option<(
            uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, Option<String>,
            Option<String>, Option<String>, Option<String>, Option<uuid::Uuid>,
            i32, String, Option<String>, Option<String>, String, String,
        )> = sqlx_core::query_as::query_as(&format!(
            "{} WHERE project_id = $1 AND status IN ('running', 'stalled') \
             ORDER BY started_at DESC LIMIT 1",
            Self::RUN_SELECT
        ))
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        row.map(Self::map_run_row).transpose()
    }

    /// Bridge a workflow phase transition onto a project's active run: append the
    /// pairing cadence events ([`crate::runs::phase_transition_events`]) and move
    /// the run's `current_phase`, so the run streams phases→segments to the relay
    /// while an agent works (`drive` stays OFF — this is status only). If the run
    /// had gone `stalled` (agent quiet), this fresh progress **revives** it to
    /// `running`. Returns the advanced run id, or `None` when there's no active
    /// run / no phase change. Best-effort: the caller logs and swallows errors so
    /// a bridge hiccup never fails the workflow-state write.
    pub async fn advance_run_phase_for_project(
        &self,
        project_id: &uuid::Uuid,
        phase: &str,
    ) -> Result<Option<uuid::Uuid>, String> {
        if phase.is_empty() {
            return Ok(None);
        }
        let Some(run) = self.active_run_for_project(project_id).await? else {
            return Ok(None);
        };
        let events = crate::runs::phase_transition_events(run.current_phase.as_deref(), phase);
        if events.is_empty() {
            return Ok(None);
        }
        // Agent progress on a stalled run = it's back → revive to running first,
        // so the appended events + the fresh heartbeat land on a running row.
        if run.status == dojo_protocol::relay::RelayRunStatus::Stalled {
            self.update_run_status(&run.id, dojo_protocol::relay::RelayRunStatus::Running, None, None)
                .await?;
            self.append_run_event(&run.id, crate::runs::RunEventKind::Recovered, Some(phase), None,
                &serde_json::json!({ "via": "update_phase", "revived": true })).await?;
        }
        let detail = serde_json::json!({ "via": "update_phase" });
        for (kind, ph) in &events {
            self.append_run_event(&run.id, *kind, Some(ph), None, &detail).await?;
        }
        self.set_run_progress(&run.id, Some(phase), run.current_feature.as_deref()).await?;
        Ok(Some(run.id))
    }

    /// The timestamp (RFC-3339 text) of a run's newest **agent-progress** event —
    /// the stall signal's reference. Excludes the daemon's cadence/lifecycle kinds
    /// (`RunEventKind::is_progress() == false`, built from the enum so it never
    /// drifts) so the every-tick `housekeeping` marker can't mask an agent stall.
    /// `None` when the run has emitted no progress event yet (caller falls back to
    /// `started_at`).
    pub async fn last_progress_at(&self, run_id: &uuid::Uuid) -> Result<Option<String>, String> {
        let excluded: Vec<String> = crate::runs::RunEventKind::ALL
            .iter()
            .filter(|k| !k.is_progress())
            .map(|k| k.as_db_str().to_string())
            .collect();
        // `to_json(...)#>>'{}'` yields RFC-3339 (the format `parse_rfc3339` and the
        // rest of RUN_SELECT use) — NOT `::text`, whose `YYYY-MM-DD HH:MM:SS-05`
        // shape fails to parse and would silently fall back to started_at.
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT to_json(created_at)#>>'{}' FROM activity.run_events
              WHERE run_id = $1 AND kind::text <> ALL($2)
              ORDER BY created_at DESC, id DESC LIMIT 1",
        )
            .bind(run_id)
            .bind(&excluded)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(|(ts,)| ts))
    }

    /// Set a run's status and (optionally) its pause fields, bumping
    /// `updated_at`. `paused_until`/`pause_reason` are written as given, so pass
    /// `None` for both to clear a pause on resume.
    pub async fn update_run_status(
        &self,
        id: &uuid::Uuid,
        status: RelayRunStatus,
        paused_until: Option<&str>,
        pause_reason: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs
                SET status       = $2::sensei.run_status,
                    paused_until = $3::timestamptz,
                    pause_reason = $4,
                    updated_at   = now()
              WHERE id = $1"
        )
            .bind(id)
            .bind(status.as_db_str())
            .bind(paused_until)
            .bind(pause_reason)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update the run's current phase/feature progress markers (+ `updated_at`).
    pub async fn set_run_progress(
        &self,
        id: &uuid::Uuid,
        phase: Option<&str>,
        feature: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs
                SET current_phase = $2, current_feature = $3, updated_at = now()
              WHERE id = $1"
        )
            .bind(id)
            .bind(phase)
            .bind(feature)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Bump the run's liveness heartbeat to `now()` (drives stall detection).
    /// Also refreshes `updated_at`.
    pub async fn touch_run_heartbeat(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs SET heartbeat_at = now(), updated_at = now() WHERE id = $1"
        )
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Persist the cloud `dojo.relay_sessions(id)` this run mirrors, so a later
    /// publish tick (and the console/app) can join the local run to its relay
    /// session. A plain uuid across the DB boundary (no cross-DB FK). Idempotent:
    /// the P1 bridge writes it once, on the first successful publish. Also bumps
    /// `updated_at`.
    pub async fn set_run_dojo_session_id(
        &self,
        id: &uuid::Uuid,
        dojo_session_id: &uuid::Uuid,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs SET dojo_session_id = $2, updated_at = now() WHERE id = $1"
        )
            .bind(id)
            .bind(dojo_session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Mark a run terminal — sets `status` (expected `Done`/`Failed`) and stamps
    /// `completed_at = now()` (+ `updated_at`).
    pub async fn complete_run(&self, id: &uuid::Uuid, status: RelayRunStatus) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs
                SET status = $2::sensei.run_status, completed_at = now(), updated_at = now()
              WHERE id = $1"
        )
            .bind(id)
            .bind(status.as_db_str())
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Append one cadence event to `activity.run_events`. `detail` is the
    /// structured, stripped payload (never code/diffs). Returns the new
    /// `bigserial` id.
    pub async fn append_run_event(
        &self,
        run_id: &uuid::Uuid,
        kind: RunEventKind,
        phase: Option<&str>,
        feature: Option<&str>,
        detail: &serde_json::Value,
    ) -> Result<i64, String> {
        let (id,): (i64,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.run_events(run_id, kind, phase, feature, detail)
             VALUES($1, $2::sensei.run_event_kind, $3, $4, $5) RETURNING id"
        )
            .bind(run_id)
            .bind(kind.as_db_str())
            .bind(phase)
            .bind(feature)
            .bind(detail)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// A run's cadence events, newest first, capped at `limit`. `kind` arrives
    /// as text and is parsed with [`RunEventKind::from_db_str`]; an unknown
    /// value is a hard error, never a silent skip.
    pub async fn list_run_events(&self, run_id: &uuid::Uuid, limit: i64) -> Result<Vec<RunEvent>, String> {
        let rows: Vec<(i64, uuid::Uuid, String, Option<String>, Option<String>, serde_json::Value, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, run_id, kind::text, phase, feature, detail,
                        to_json(created_at)#>>'{}'
                   FROM activity.run_events
                  WHERE run_id = $1
                  ORDER BY created_at DESC, id DESC
                  LIMIT $2"
            )
                .bind(run_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        rows.into_iter()
            .map(|(id, run_id, kind, phase, feature, detail, created_at)| {
                let kind = RunEventKind::from_db_str(&kind)
                    .ok_or_else(|| format!("unknown run_event_kind from DB: {kind:?}"))?;
                Ok(RunEvent { id, run_id, kind, phase, feature, detail, created_at })
            })
            .collect()
    }

    /// Flip every `paused` run whose `paused_until` has elapsed back to
    /// `running`, clearing the pause fields. The `<=` comparison runs SQL-side
    /// (`paused_until <= now()`) so we never parse RFC-3339 back into Rust just
    /// to compare clocks. Returns the ids of the runs that were resumed, so the
    /// scheduler can log a `Resumed` cadence event + kick an `AdvanceRun` tick
    /// for each. A run with `paused_until IS NULL` (an indefinite/manual pause)
    /// is never auto-resumed.
    pub async fn resume_due_runs(&self) -> Result<Vec<uuid::Uuid>, String> {
        let rows: Vec<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "UPDATE activity.runs
                SET status       = 'running'::sensei.run_status,
                    paused_until = NULL,
                    pause_reason = NULL,
                    updated_at   = now()
              WHERE status = 'paused'::sensei.run_status
                AND paused_until IS NOT NULL
                AND paused_until <= now()
             RETURNING id"
        )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    // ── Watchdog (P3.6) ────────────────────────────────────────────────

    /// The runs the watchdog can act on — `running` or `stalled` — with just the
    /// fields it needs to assess liveness: `(id, status text, heartbeat_at,
    /// started_at, recovery_attempts)`. Deliberately a lightweight query (NOT
    /// [`Self::RUN_SELECT`]/[`Self::map_run_row`]) so adding the watchdog never
    /// perturbs the `Run` row surface. Timestamps come back as RFC-3339 via the
    /// same `to_json(col)#>>'{}'` idiom as `RUN_SELECT`; `heartbeat_at` is
    /// `Option` (a run may not have heartbeated yet, so the caller falls back to
    /// `started_at`).
    pub async fn list_recoverable_runs(
        &self,
    ) -> Result<Vec<(uuid::Uuid, String, Option<String>, String, i32)>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, String, i32)> =
            sqlx_core::query_as::query_as(
                "SELECT id, status::text,
                        to_json(heartbeat_at)#>>'{}',
                        to_json(started_at)#>>'{}',
                        recovery_attempts
                   FROM activity.runs
                  WHERE status IN ('running', 'stalled')",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Bounded auto-recovery: flip a `stalled` run back to `running`, record the
    /// new attempt count, and refresh the heartbeat so the recovered run isn't
    /// immediately re-flagged stale on the next watchdog tick.
    pub async fn recover_run(&self, id: &uuid::Uuid, next_attempt: i32) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs
                SET status            = 'running'::sensei.run_status,
                    recovery_attempts = $2,
                    heartbeat_at      = now(),
                    updated_at        = now()
              WHERE id = $1",
        )
        .bind(id)
        .bind(next_attempt)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Reset the bounded-recovery counter to 0 on real progress (a clean drive
    /// step) so a long overnight run that recovered earlier doesn't prematurely
    /// give up later.
    pub async fn reset_run_recovery(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs SET recovery_attempts = 0, updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── PG Function Wrappers ───────────────────────────────────────────

    /// BM25-style keyword ranking: matches nodes by name/signature/docstring.
    pub async fn rank_bm25(&self, folder_id: &uuid::Uuid, query: &str) -> Result<Vec<(String, f64)>, String> {
        let rows: Vec<(String, f64)> = sqlx_core::query_as::query_as(
            "SELECT file_path, score FROM sensei.rank_bm25($1, $2)"
        ).bind(folder_id).bind(query)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    // ── Graph (typed wrappers) ─────────────────────────────────────────

    pub async fn merge_function(
        &self, folder_id: &uuid::Uuid, name: &str, file_path: &str,
        signature: Option<&str>, line_start: Option<i32>, line_end: Option<i32>,
        parent_id: Option<&uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node(folder_id, "function", name, file_path, parent_id, signature, line_start, line_end).await
    }

    pub async fn merge_file(
        &self, folder_id: &uuid::Uuid, name: &str, file_path: &str,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node(folder_id, "file", name, file_path, None, None, None, None).await
    }

    pub async fn merge_type(
        &self, folder_id: &uuid::Uuid, name: &str, file_path: &str,
        kind: &str, line_start: Option<i32>,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node(folder_id, kind, name, file_path, None, None, line_start, None).await
    }

    pub async fn merge_doc(
        &self, folder_id: &uuid::Uuid, name: &str, file_path: &str,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node(folder_id, "doc", name, file_path, None, None, None, None).await
    }

    pub async fn project_exists(&self, folder_id: &uuid::Uuid) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM sensei.nodes WHERE folder_id = $1)"
        ).bind(folder_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn search_functions(&self, folder_id: &uuid::Uuid, query: &str) -> Result<Vec<serde_json::Value>, String> {
        self.search_functions_scoped(std::slice::from_ref(folder_id), query).await
    }

    pub async fn search_types(&self, folder_id: &uuid::Uuid, query: &str) -> Result<Vec<serde_json::Value>, String> {
        self.search_types_scoped(std::slice::from_ref(folder_id), query).await
    }

    pub async fn count_nodes_by_kind(&self, folder_id: &uuid::Uuid) -> Result<std::collections::HashMap<String, i64>, String> {
        self.count_nodes_by_kind_scoped(std::slice::from_ref(folder_id)).await
    }

    pub async fn delete_node(&self, node_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE id = $1")
            .bind(node_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn delete_nodes_by_file(&self, folder_id: &uuid::Uuid, file_path: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE folder_id = $1 AND file_path = $2")
            .bind(folder_id).bind(file_path).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Distinct file paths (repo-relative) that a folder has indexed nodes for.
    /// Excludes `module` nodes — those record an ABSOLUTE directory path (not a
    /// file) and are re-derived structurally, so mixing them into a rel-path
    /// comparison would be wrong. Used by the reconcile's `prune_vanished` safety
    /// net to find nodes whose file no longer exists on disk.
    pub async fn list_indexed_files(&self, folder_id: &uuid::Uuid) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT file_path FROM sensei.nodes
              WHERE folder_id = $1 AND kind::text <> 'module' AND file_path <> ''"
        ).bind(folder_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(p,)| p).collect())
    }

    pub async fn clear_all_nodes(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        self.delete_nodes_by_folder(folder_id).await
    }

    // ── Repo (folders with kind='git'/'subtree') ──────────────────────

    /// Register a git repo as a folder. Equivalent to old upsert_repo_basic.
    pub async fn upsert_repo(&self, root_id: &uuid::Uuid, name: &str, abs_path: &str) -> Result<uuid::Uuid, String> {
        self.upsert_folder(root_id, "git", name, name, abs_path, None, None).await
    }

    /// Register a project root with an explicit folder kind — `git` for real
    /// repos, `standalone` for quasi-repos (non-git project roots).
    ///
    /// Unlike [`upsert_folder`]'s sticky-kind upsert, a root's git↔standalone
    /// classification is **authoritative on every scan**: a repo that lost its
    /// `.git` (now a quasi-repo) is relabelled `standalone`, and one that gained
    /// a `.git` flips back to `git`. `subtree`/`folder` kinds are never clobbered
    /// here — those are owned by subtree detection and tree materialisation.
    pub async fn upsert_repo_kind(&self, root_id: &uuid::Uuid, kind: &str, name: &str, abs_path: &str) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path)
             VALUES($1, $2::sensei.folder_kind, $3, $3, $4)
             ON CONFLICT(abs_path) DO UPDATE SET
                kind = CASE WHEN folders.kind IN ('git'::sensei.folder_kind, 'standalone'::sensei.folder_kind)
                            THEN EXCLUDED.kind ELSE folders.kind END,
                name = EXCLUDED.name,
                modified_at = now()
             RETURNING id"
        )
            .bind(root_id).bind(kind).bind(name).bind(abs_path)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Upsert a structural subfolder (`kind='folder'`) within a project, linked
    /// to its parent folder. Thin wrapper over [`Self::upsert_subfolder_kind`].
    pub async fn upsert_subfolder(
        &self, root_id: &uuid::Uuid, name: &str, path: &str, abs_path: &str,
        parent_id: Option<&uuid::Uuid>, project_id: Option<&uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_subfolder_kind(root_id, "folder", name, path, abs_path, parent_id, project_id).await
    }

    /// Upsert a structural subfolder with an explicit `kind` — `folder` (the
    /// navigable filesystem-tree row) or `workspace_member` (a monorepo member,
    /// D5a). Status is terminal (`indexed`) — these rows model the tree, not scan
    /// progress. On conflict the kind is relabelled ONLY between the two
    /// structural kinds (`folder`↔`workspace_member`); a path that is actually a
    /// (nested) project ROOT (`git`/`standalone`/`subtree`) is never reclassified.
    pub async fn upsert_subfolder_kind(
        &self, root_id: &uuid::Uuid, kind: &str, name: &str, path: &str, abs_path: &str,
        parent_id: Option<&uuid::Uuid>, project_id: Option<&uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders(root_id, kind, status, name, path, abs_path, parent_id, project_id)
             VALUES($1, $2::sensei.folder_kind, 'indexed'::sensei.folder_status, $3, $4, $5, $6, $7)
             ON CONFLICT(abs_path) DO UPDATE SET
                kind = CASE WHEN folders.kind IN ('folder'::sensei.folder_kind, 'workspace_member'::sensei.folder_kind)
                            THEN EXCLUDED.kind ELSE folders.kind END,
                name = EXCLUDED.name,
                parent_id = COALESCE(EXCLUDED.parent_id, folders.parent_id),
                project_id = COALESCE(EXCLUDED.project_id, folders.project_id),
                modified_at = now()
             RETURNING id"
        )
            .bind(root_id).bind(kind).bind(name).bind(path).bind(abs_path).bind(parent_id).bind(project_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Get a repo (folder with kind='git'/'subtree') by abs_path.
    pub async fn get_repo_by_path(&self, abs_path: &str) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, uuid::Uuid, String, String, String, Option<uuid::Uuid>, serde_json::Value, Vec<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, root_id, kind::text, name, abs_path, project_id, props, tags, modified_at FROM sensei.folders WHERE abs_path = $1"
            ).bind(abs_path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id, root_id, kind, name, abs, pid, props, tags, modified)| {
            serde_json::json!({
                "id": id, "root_id": root_id, "kind": kind, "name": name, "abs_path": abs,
                "project_id": pid, "props": props, "tags": tags,
                "modified_at": modified.to_rfc3339(),
            })
        }))
    }

    /// Get a repo by name (for backward compat with repo_id lookups).
    pub async fn get_repo_by_name(&self, name: &str) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, Option<uuid::Uuid>, serde_json::Value, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, abs_path, project_id, props, modified_at FROM sensei.folders WHERE name = $1 AND kind IN ('git'::sensei.folder_kind, 'subtree'::sensei.folder_kind, 'standalone'::sensei.folder_kind) LIMIT 1"
            ).bind(name).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id, name, abs, pid, props, modified)| {
            serde_json::json!({ "id": id, "name": name, "abs_path": abs, "project_id": pid, "props": props, "modified_at": modified.to_rfc3339() })
        }))
    }

    /// Merge into a node's `props` jsonb (D5b): used to stamp a `section` node's
    /// `level` and real `line_start` (the identity key carries a NULL line so
    /// section identity is line-independent — 0.4). Idempotent (`props || $2`).
    pub async fn set_node_props(&self, node_id: &uuid::Uuid, props: &serde_json::Value) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.nodes SET props = props || $2, modified_at = now() WHERE id = $1"
        ).bind(node_id).bind(props).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Set folder props (metadata like stack, libs, indexed_at, etc.).
    pub async fn set_folder_props(&self, folder_id: &uuid::Uuid, props: &serde_json::Value) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET props = props || $2, modified_at = now() WHERE id = $1"
        ).bind(folder_id).bind(props).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Assign a folder to a project with role/label.
    pub async fn set_folder_project(&self, folder_id: &uuid::Uuid, project_id: &uuid::Uuid, role: &str, label: Option<&str>) -> Result<(), String> {
        let props = serde_json::json!({"role": role, "label": label});
        sqlx_core::query::query(
            "UPDATE sensei.folders SET project_id = $2, props = props || $3, modified_at = now() WHERE id = $1"
        ).bind(folder_id).bind(project_id).bind(props).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update only the `role` column on a folder. Used by the Projects
    /// setup stage when the user picks a role from the dropdown — distinct
    /// from set_folder_project (which also reassigns project membership).
    pub async fn update_folder_role(&self, folder_id: &uuid::Uuid, role: Option<&str>) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET role = $2::sensei.folder_role, modified_at = now() WHERE id = $1"
        ).bind(folder_id).bind(role).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// All folders belonging to a project, ordered by path. Used to enrich
    /// /api/projects responses with folder membership so the Projects setup
    /// page can render per-folder details.
    pub async fn list_folders_by_project(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        self.query_folders_by_project(project_id, false).await
    }

    /// Repo-root folders only (`kind IN ('git','standalone')`) for a project —
    /// the compact folder set the folder-scoped `find_projects` view (`GET
    /// /api/projects?under=…`) needs. Drops the hundreds of nested
    /// `kind:'folder'` descendant rows that pushed that response past the MCP
    /// client's token cap (~72K chars for sensei). The repo roots are kept so
    /// the MCP proxy's `resolve_from_cwd_in` longest-prefix cwd→project match
    /// still resolves any deep working directory (a deep cwd still
    /// `starts_with` the repo root).
    pub async fn list_root_folders_by_project(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        self.query_folders_by_project(project_id, true).await
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

    /// Return the absolute paths of a project's indexed folders — the input the
    /// analyzer scheduler needs to enqueue `DetectCommunities` per folder. Only
    /// `indexed` folders are worth running community detection on (others have
    /// no code nodes yet).
    pub async fn get_indexed_folder_paths_for_project(&self, project_id: &uuid::Uuid) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT abs_path
             FROM sensei.folders
             WHERE project_id = $1 AND status = 'indexed'::sensei.folder_status
             ORDER BY abs_path"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(p,)| p).collect())
    }

    /// Resolve a project's canonical working-directory path — the `abs_path` of
    /// its shallowest repo-root folder (`kind IN ('git','standalone')`). Used by
    /// the relay run driver (P3.3b) to pick the cwd it spawns the agent in.
    ///
    /// Shortest `abs_path` wins so a monorepo project resolves to the repo root
    /// rather than a nested sub-package. `None` when the project has no
    /// repo-root folder (e.g. a project deleted out from under a run). The
    /// caller must still confirm the path exists on disk before spawning.
    pub async fn project_root_path(&self, project_id: &uuid::Uuid) -> Result<Option<String>, String> {
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT abs_path
             FROM sensei.folders
             WHERE project_id = $1 AND kind::text IN ('git','standalone')
             ORDER BY length(abs_path), abs_path
             LIMIT 1"
        ).bind(project_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(p,)| p))
    }

    /// Flip a folder to `indexed` and stamp `props.indexed_at`. The dedicated
    /// writer of the `indexed` status, called at the terminal community barrier
    /// (D4.1). Detected libs are folder metadata stamped separately via
    /// `set_folder_props` by the resolve/build barriers, so this need not carry
    /// them — keeping "communities computed → indexed" as the single meaning of
    /// this write.
    pub async fn mark_folder_indexed(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        let props = serde_json::json!({"indexed_at": chrono::Utc::now().to_rfc3339()});
        sqlx_core::query::query(
            "UPDATE sensei.folders SET status = 'indexed'::sensei.folder_status, props = props || $2, modified_at = now() WHERE id = $1"
        ).bind(folder_id).bind(&props).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Set a folder's lifecycle status (D6a). The general setter behind the
    /// `discovered → queued → indexing → indexed | failed` lifecycle;
    /// `mark_folder_indexed` remains the dedicated writer of `indexed` (it also
    /// stamps `props.indexed_at`). A scan marks `indexing` at start so a
    /// crash leaves a recoverable state (resume re-enqueues non-terminal folders).
    pub async fn update_folder_status(&self, folder_id: &uuid::Uuid, status: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET status = $2::sensei.folder_status, modified_at = now() WHERE id = $1"
        ).bind(folder_id).bind(status).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Read a folder's index status (`sensei.folders.status`). `None` when no
    /// such folder row exists — an honest miss, never a fabricated status. Used
    /// by the fail-closed barrier (D6d) to leave a folder with a recorded fatal
    /// failure `failed` rather than advancing it to `indexed`.
    pub async fn get_folder_status(&self, folder_id: &uuid::Uuid) -> Result<Option<String>, String> {
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT status::text FROM sensei.folders WHERE id = $1"
        ).bind(folder_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    /// Append a tag to a folder's `tags` array, idempotently (no duplicates).
    /// Used by the scan reconcile to flag a former project root that still has
    /// on-disk content but no live owner (`stale`) for the user to triage,
    /// rather than deleting content the scan can't account for.
    pub async fn tag_folder(&self, folder_id: &uuid::Uuid, tag: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders
                SET tags = array(SELECT DISTINCT unnest(tags || ARRAY[$2])),
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(folder_id)
        .bind(tag)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Remove a tag from a folder's `tags` array (no-op if absent). Pairs with
    /// [`tag_folder`] so the scan can keep a derived flag (e.g. `needs-review`)
    /// in sync — clearing it when a folder no longer qualifies.
    pub async fn untag_folder(&self, folder_id: &uuid::Uuid, tag: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET tags = array_remove(tags, $2), modified_at = now() WHERE id = $1",
        )
        .bind(folder_id)
        .bind(tag)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete a folder (cascade deletes nodes, edges, scan_state, etc.).
    pub async fn delete_repo_by_name(&self, name: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "DELETE FROM sensei.folders WHERE name = $1 AND kind IN ('git'::sensei.folder_kind, 'subtree'::sensei.folder_kind, 'standalone'::sensei.folder_kind)"
        ).bind(name).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Nodes ─────────────────────────────────────────────────────────

    /// Upsert a node (default `is_exported = false`). Thin wrapper over
    /// [`Self::upsert_node_ex`] for the many callers that don't carry visibility
    /// (file/section/rationale/module nodes, tests).
    pub async fn upsert_node(
        &self, folder_id: &uuid::Uuid, kind: &str, name: &str, file_path: &str,
        parent_id: Option<&uuid::Uuid>, signature: Option<&str>,
        line_start: Option<i32>, line_end: Option<i32>,
    ) -> Result<uuid::Uuid, String> {
        self.upsert_node_ex(folder_id, kind, name, file_path, parent_id, signature, line_start, line_end, false).await
    }

    /// Upsert a node carrying `is_exported` (the code-symbol path passes the
    /// parser's `pub`/`export` visibility). `is_exported` is written on INSERT and
    /// refreshed on the D3 upsert-then-prune conflict, so a symbol that flips
    /// pub↔private is kept current.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_node_ex(
        &self, folder_id: &uuid::Uuid, kind: &str, name: &str, file_path: &str,
        parent_id: Option<&uuid::Uuid>, signature: Option<&str>,
        line_start: Option<i32>, line_end: Option<i32>, is_exported: bool,
    ) -> Result<uuid::Uuid, String> {
        // ON CONFLICT targets nodes_unique_identity (folder_id, file_path, kind, name,
        // parent_id, line_start NULLS NOT DISTINCT). DO UPDATE keeps the row STABLE on
        // re-scans — same UUID whether just inserted or pre-existing (D3 upsert-then-
        // prune) — preserving community_id and degree. It refreshes signature/line_end,
        // and re-nulls `embedding` ONLY when the signature changed: `embed_text` is a
        // function of (kind, name, signature, file_path), and on a same-identity
        // conflict the first three-of-four are fixed by the key, so `signature` is the
        // only embed input that can change — nulling on that (and preserving it
        // otherwise) keeps embeddings fresh without a separate content_hash column.
        // `language` is derived from the file extension at write time (the single
        // shared mapping). Populating it on THIS legacy path too — every non-Rust +
        // file/section/rationale node flows through here for the whole FQN
        // transition — is what gives the same-language bare-name fallback (plan 0.8)
        // something to filter on. COALESCE on conflict backfills pre-existing rows.
        let language = crate::languages::language_for_path(file_path);
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.nodes(folder_id, kind, name, file_path, parent_id, signature, line_start, line_end, is_exported, language)
             VALUES($1, $2::sensei.node_kind, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (folder_id, file_path, kind, name, parent_id, line_start) WHERE file_path IS NOT NULL DO UPDATE
               SET signature   = EXCLUDED.signature,
                   line_end    = EXCLUDED.line_end,
                   is_exported = EXCLUDED.is_exported,
                   language    = COALESCE(EXCLUDED.language, nodes.language),
                   embedding   = CASE WHEN nodes.signature IS DISTINCT FROM EXCLUDED.signature
                                      THEN NULL ELSE nodes.embedding END,
                   modified_at = now()
             RETURNING id"
        ).bind(folder_id).bind(kind).bind(name).bind(file_path)
            .bind(parent_id).bind(signature).bind(line_start).bind(line_end).bind(is_exported).bind(language)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Get-or-create a node by its fully-qualified name (SCIP/LSIF moniker model).
    /// A REFERENCE (`def = None`) creates — or returns — an unresolved STUB
    /// (`resolved=false`, NULL `file_path`). A DEFINITION (`def = Some`) creates or
    /// ENRICHES the same `(folder_id, fqn)` node in place: flips `resolved=true` and
    /// fills `file_path`/`signature`/`line_start`/`line_end`/`is_exported`/`parent_id`.
    ///
    /// Monotone + idempotent: a reference NEVER downgrades an already-resolved node
    /// (`resolved = OLD OR NEW`; def-only columns are kept unless the incoming row is
    /// itself a definition), and re-enrichment re-nulls the embedding only when the
    /// signature changed — the same freshness rule as `upsert_node_ex`. Arbiter is
    /// the partial `nodes_unique_fqn` index, so this coexists with the line-based
    /// `nodes_unique_identity`.
    pub async fn upsert_node_by_fqn(
        &self,
        folder_id: &uuid::Uuid,
        fqn: &str,
        kind: &str,
        name: &str,
        language: Option<&str>,
        def: Option<FqnDef<'_>>,
    ) -> Result<uuid::Uuid, String> {
        let resolved = def.is_some();
        let file_path = def.as_ref().map(|d| d.file_path);
        let signature = def.as_ref().and_then(|d| d.signature);
        let line_start = def.as_ref().and_then(|d| d.line_start);
        let line_end = def.as_ref().and_then(|d| d.line_end);
        let is_exported = def.as_ref().is_some_and(|d| d.is_exported);
        let parent_id = def.as_ref().and_then(|d| d.parent_id);

        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.nodes
                 (folder_id, fqn, kind, name, language, resolved,
                  file_path, signature, line_start, line_end, is_exported, parent_id)
             VALUES($1, $2, $3::sensei.node_kind, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             ON CONFLICT (folder_id, fqn) WHERE fqn IS NOT NULL DO UPDATE
               SET resolved    = nodes.resolved OR EXCLUDED.resolved,
                   kind        = CASE WHEN EXCLUDED.resolved THEN EXCLUDED.kind ELSE nodes.kind END,
                   file_path   = COALESCE(EXCLUDED.file_path, nodes.file_path),
                   signature   = CASE WHEN EXCLUDED.resolved THEN EXCLUDED.signature ELSE nodes.signature END,
                   line_start  = CASE WHEN EXCLUDED.resolved THEN EXCLUDED.line_start ELSE nodes.line_start END,
                   line_end    = CASE WHEN EXCLUDED.resolved THEN EXCLUDED.line_end ELSE nodes.line_end END,
                   is_exported = CASE WHEN EXCLUDED.resolved THEN EXCLUDED.is_exported ELSE nodes.is_exported END,
                   parent_id   = COALESCE(EXCLUDED.parent_id, nodes.parent_id),
                   language    = COALESCE(EXCLUDED.language, nodes.language),
                   embedding   = CASE WHEN EXCLUDED.resolved
                                       AND nodes.signature IS DISTINCT FROM EXCLUDED.signature
                                      THEN NULL ELSE nodes.embedding END,
                   modified_at = now()
             RETURNING id"
        )
        .bind(folder_id).bind(fqn).bind(kind).bind(name).bind(language).bind(resolved)
        .bind(file_path).bind(signature).bind(line_start).bind(line_end).bind(is_exported).bind(parent_id)
        .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Get-or-create a first-class `lib_symbol` node for an EXTERNAL reference (a
    /// dependency's symbol), grouped under a per-package `lib_package` container so
    /// the graph shows "what we depend on and how much" (blueprint Fix 1, case 2).
    /// Both are `resolved=true` (the external symbol IS its own definition) with
    /// NULL `file_path` (no local file); the symbol's `parent_id` is its container.
    /// Owned by the referencing repo-root `folder_id` so they cascade with it.
    /// Stable ids across repeated references (arbiter = `nodes_unique_fqn`).
    pub async fn upsert_lib_node_by_fqn(
        &self,
        folder_id: &uuid::Uuid,
        fqn: &str,
        name: &str,
        package: &str,
    ) -> Result<uuid::Uuid, String> {
        // One `lib_package` container per dependency (fqn = `lib·<package>`).
        let pkg_fqn = format!("lib{}{}", crate::languages::fqn::SEP, package);
        let container: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.nodes
                 (folder_id, fqn, kind, name, resolved, props)
             VALUES($1, $2, 'lib_package'::sensei.node_kind, $3, true,
                    jsonb_build_object('package', $3::text))
             ON CONFLICT (folder_id, fqn) WHERE fqn IS NOT NULL DO UPDATE
               SET resolved = true, modified_at = now()
             RETURNING id"
        )
        .bind(folder_id).bind(&pkg_fqn).bind(package)
        .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;

        // The symbol, parented under its package container.
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.nodes
                 (folder_id, fqn, kind, name, resolved, parent_id, props)
             VALUES($1, $2, 'lib_symbol'::sensei.node_kind, $3, true, $4,
                    jsonb_build_object('package', $5::text))
             ON CONFLICT (folder_id, fqn) WHERE fqn IS NOT NULL DO UPDATE
               SET resolved    = true,
                   parent_id   = COALESCE(EXCLUDED.parent_id, nodes.parent_id),
                   props       = nodes.props || jsonb_build_object('package', $5::text),
                   modified_at = now()
             RETURNING id"
        )
        .bind(folder_id).bind(fqn).bind(name).bind(container.0).bind(package)
        .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// External dependencies referenced by a repo — one row per `lib_package` with
    /// how many of its symbols the repo actually uses (`{package, symbol_count}`).
    /// The graph-visible "what we depend on and how much".
    pub async fn list_dependencies(&self, folder_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, i64)> = sqlx_core::query_as::query_as(
            "SELECT p.name, count(s.id)
               FROM sensei.nodes p
               LEFT JOIN sensei.nodes s
                 ON s.folder_id = p.folder_id
                AND s.parent_id = p.id
                AND s.kind = 'lib_symbol'::sensei.node_kind
              WHERE p.folder_id = $1 AND p.kind = 'lib_package'::sensei.node_kind
              GROUP BY p.name
              ORDER BY count(s.id) DESC, p.name"
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(package, symbol_count)| {
            serde_json::json!({ "package": package, "symbol_count": symbol_count })
        }).collect())
    }

    pub async fn get_nodes_by_folder(&self, folder_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        self.get_nodes_scoped(std::slice::from_ref(folder_id)).await
    }

    /// Nodes in a folder that still need an embedding, restricted to the kinds
    /// worth embedding (code symbols + files + doc sections). Returns
    /// `(id, kind, name, signature, file_path)` — the fields needed to build the
    /// embedding text. Used by the `EmbedNodes` task.
    pub async fn nodes_without_embeddings(
        &self,
        folder_id: &uuid::Uuid,
        limit: i64,
    ) -> Result<Vec<(uuid::Uuid, String, String, Option<String>, String)>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, kind::text, name, signature, file_path
                   FROM sensei.nodes
                  WHERE folder_id = $1
                    AND embedding IS NULL
                    AND file_path IS NOT NULL
                    AND kind IN ('file','function','method','class','interface',
                                 'type','const','enum','enum_variant','section',
                                 'struct','component','hook','doc','extension')
                  ORDER BY file_path, line_start
                  LIMIT $2",
            )
            .bind(folder_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Store a node's vector embedding. The slice is rendered to pgvector's text
    /// form (`[v1,v2,...]`) and cast to `vector`, so no pgvector crate is needed.
    pub async fn set_node_embedding(
        &self,
        node_id: &uuid::Uuid,
        embedding: &[f32],
    ) -> Result<(), String> {
        let buf = vector_literal(embedding);
        sqlx_core::query::query("UPDATE sensei.nodes SET embedding = $1::vector WHERE id = $2")
            .bind(buf)
            .bind(node_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Semantic nearest-neighbour search over node embeddings, scoped to the
    /// given folders + node kinds. Reuses the same pgvector cosine-distance
    /// operator (`<=>`) as `find_duplicates`, ordering by ascending distance so
    /// the most semantically similar nodes come first. The query embedding is
    /// rendered with `vector_literal` and cast to `vector`, matching how
    /// `set_node_embedding` stores node vectors. Bounded by `limit` so it never
    /// materially slows the common query path. Returns
    /// `(id, name, file_path, signature, line_start)` — the fields the query
    /// handler projects into function/type hits for fusion with lexical results.
    pub async fn semantic_search_nodes(
        &self,
        folder_ids: &[uuid::Uuid],
        query_embedding: &[f32],
        kinds: &[&str],
        limit: i64,
    ) -> Result<Vec<(uuid::Uuid, String, String, Option<String>, Option<i32>)>, String> {
        if folder_ids.is_empty() || query_embedding.is_empty() || kinds.is_empty() {
            return Ok(Vec::new());
        }
        let vec_literal = vector_literal(query_embedding);
        let kind_strs: Vec<String> = kinds.iter().map(|k| k.to_string()).collect();
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<i32>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, file_path, signature, line_start
                   FROM sensei.nodes
                  WHERE folder_id = ANY($1::uuid[])
                    AND kind::text = ANY($3::text[])
                    AND embedding IS NOT NULL
                  ORDER BY embedding <=> $2::vector
                  LIMIT $4",
            )
            .bind(folder_ids)
            .bind(vec_literal)
            .bind(kind_strs)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Resolve nodes to their on-disk locations for snippet extraction, keyed by
    /// id. Returns `(id, abs_path, file_path, line_start, line_end, kind, name,
    /// signature)` — the repo's `abs_path` joined with the node `file_path` is the
    /// file to read, and the line range bounds the snippet. Missing line info
    /// falls back to line 1 (a one-line snippet). Used by `context_pack`.
    #[allow(clippy::type_complexity)]
    pub async fn node_locations(
        &self, ids: &[uuid::Uuid],
    ) -> Result<Vec<(uuid::Uuid, String, String, i32, i32, String, String, Option<String>)>, String> {
        if ids.is_empty() { return Ok(Vec::new()); }
        sqlx_core::query_as::query_as(
            "SELECT n.id, f.abs_path, n.file_path,
                    COALESCE(n.line_start, 1),
                    COALESCE(n.line_end, n.line_start, 1),
                    n.kind::text, n.name, n.signature
               FROM sensei.nodes n
               JOIN sensei.folders f ON f.id = n.folder_id
              WHERE n.id = ANY($1::uuid[])
                AND n.file_path IS NOT NULL",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    /// Find near-duplicate function/method pairs within a folder by cosine
    /// similarity on their code embeddings (HNSW `<=>` cosine distance). Each
    /// pair is returned once (`a.id < b.id`) at or above `min_similarity`,
    /// strongest first. Trivial functions (< 4 lines) are skipped — they bound
    /// the O(n²) self-join and avoid false positives from boilerplate. On-demand
    /// review query, not a hot path.
    pub async fn find_duplicates(&self, folder_id: &uuid::Uuid, min_similarity: f64, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let max_distance = 1.0 - min_similarity;
        let rows: Vec<(String, String, Option<i32>, String, String, Option<i32>, f64)> =
            sqlx_core::query_as::query_as(
                "SELECT a.name, a.file_path, a.line_start,
                        b.name, b.file_path, b.line_start,
                        1 - (a.embedding <=> b.embedding) AS similarity
                   FROM sensei.nodes a
                   JOIN sensei.nodes b
                     ON b.folder_id = a.folder_id
                    AND a.id < b.id
                    AND b.kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
                    AND b.embedding IS NOT NULL
                    AND (b.line_end - b.line_start) >= 3
                  WHERE a.folder_id = $1
                    AND a.kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
                    AND a.embedding IS NOT NULL
                    AND (a.line_end - a.line_start) >= 3
                    AND (a.embedding <=> b.embedding) <= $2
                  ORDER BY similarity DESC
                  LIMIT $3",
            )
            .bind(folder_id)
            .bind(max_distance)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(na, fa, la, nb, fb, lb, sim)| {
            serde_json::json!({
                "a": { "name": na, "file": fa, "line": la },
                "b": { "name": nb, "file": fb, "line": lb },
                "similarity": (sim * 10000.0).round() / 10000.0,
            })
        }).collect())
    }

    /// Multi-folder variant of `find_duplicates` (#54). Runs the same
    /// cosine-similarity self-join but scopes the pair search to every
    /// folder belonging to a project — so a duplicate function defined in
    /// `crates/foo/src/x.rs` and `crates/bar/src/y.rs` (both inside the
    /// same project) surfaces even though they don't share a folder_id.
    ///
    /// Pairs are restricted to `a.id < b.id` so each dyad appears once. It does
    /// NOT require `a.folder_id != b.folder_id`: in a monorepo the indexer rolls
    /// every function node up to the single repo-root folder, so a cross-folder-only
    /// filter made this always return `count:0` (masking every real duplicate).
    /// The handler uses either this OR `find_duplicates` per call (never both), so
    /// there is no double-count to guard against.
    pub async fn find_duplicates_scoped(
        &self,
        folder_ids: &[uuid::Uuid],
        min_similarity: f64,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        if folder_ids.is_empty() {
            return Ok(Vec::new());
        }
        let max_distance = 1.0 - min_similarity;
        let rows: Vec<(String, String, Option<i32>, String, String, Option<i32>, f64)> =
            sqlx_core::query_as::query_as(
                "SELECT a.name, a.file_path, a.line_start,
                        b.name, b.file_path, b.line_start,
                        1 - (a.embedding <=> b.embedding) AS similarity
                   FROM sensei.nodes a
                   JOIN sensei.nodes b
                     ON a.id < b.id
                    AND b.folder_id = ANY($1::uuid[])
                    AND b.kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
                    AND b.embedding IS NOT NULL
                    AND (b.line_end - b.line_start) >= 3
                  WHERE a.folder_id = ANY($1::uuid[])
                    AND a.kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
                    AND a.embedding IS NOT NULL
                    AND (a.line_end - a.line_start) >= 3
                    AND (a.embedding <=> b.embedding) <= $2
                  ORDER BY similarity DESC
                  LIMIT $3",
            )
            .bind(folder_ids)
            .bind(max_distance)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(na, fa, la, nb, fb, lb, sim)| {
            serde_json::json!({
                "a": { "name": na, "file": fa, "line": la },
                "b": { "name": nb, "file": fb, "line": lb },
                "similarity": (sim * 10000.0).round() / 10000.0,
            })
        }).collect())
    }

    /// Per-folder duplication stats for a project: `(folder_id, eligible_count,
    /// duplicate_count)`, one row per folder that has ≥1 eligible symbol. Powers the
    /// snapshot `duplication_ratio` metric (per-module + project).
    ///
    /// An *eligible* symbol is exactly the population [`Self::find_duplicates_scoped`]
    /// compares — a `function`/`method` node with an embedding whose body spans ≥3
    /// lines. A *duplicate* is an eligible symbol that has ≥1 near-duplicate partner
    /// anywhere in the project: another eligible symbol within cosine similarity
    /// `min_similarity` (cosine distance `<=> <= 1 - min_similarity`). BOTH members of
    /// every matched pair are counted (the join is `a.id <> b.id`, then `DISTINCT`),
    /// so `duplicate_count` is the number of distinct symbols participating in a
    /// cluster — the `duplication_ratio` numerator, with `eligible_count` its
    /// denominator. Scoped to the project via `folders.project_id` (folders with no
    /// eligible symbol cannot contribute a pair, so they are correctly absent).
    /// On-demand review query, not a hot path.
    pub async fn duplication_stats_scoped(
        &self,
        project_id: &uuid::Uuid,
        min_similarity: f64,
    ) -> Result<Vec<(uuid::Uuid, i64, i64)>, String> {
        let max_distance = 1.0 - min_similarity;
        sqlx_core::query_as::query_as(
            "WITH eligible AS (
                 SELECT n.id, n.folder_id, n.embedding
                   FROM sensei.nodes   n
                   JOIN sensei.folders f ON f.id = n.folder_id
                  WHERE f.project_id = $1
                    AND n.kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
                    AND n.embedding IS NOT NULL
                    AND (n.line_end - n.line_start) >= 3
             ),
             dup AS (
                 SELECT DISTINCT a.id, a.folder_id
                   FROM eligible a
                   JOIN eligible b
                     ON a.id <> b.id
                    AND (a.embedding <=> b.embedding) <= $2
             )
             SELECT e.folder_id                          AS folder_id
                  , count(*)::int8                       AS eligible_count
                  , count(d.id)::int8                    AS duplicate_count
               FROM eligible e
               LEFT JOIN dup d ON d.id = e.id
              GROUP BY e.folder_id",
        )
        .bind(project_id)
        .bind(max_distance)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    /// Abs paths of folders that still have embeddable nodes without an
    /// embedding. Used by the backfill endpoint to enqueue `EmbedNodes` for
    /// already-indexed folders (which a normal incremental scan won't revisit).
    pub async fn folders_with_pending_embeddings(&self) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT f.abs_path
               FROM sensei.nodes n
               JOIN sensei.folders f ON f.id = n.folder_id
              WHERE n.embedding IS NULL
                AND n.file_path IS NOT NULL
                AND n.kind IN ('file','function','method','class','interface',
                               'type','const','enum','enum_variant','section',
                               'struct','component','hook','doc','extension')",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(p,)| p).collect())
    }

    pub async fn get_nodes_by_file(&self, folder_id: &uuid::Uuid, file_path: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<uuid::Uuid>, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, parent_id, line_start FROM sensei.nodes WHERE folder_id = $1 AND file_path = $2 ORDER BY line_start"
        ).bind(folder_id).bind(file_path).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, pid, ls)| {
            serde_json::json!({ "id": id, "kind": kind, "name": name, "parent_id": pid, "line_start": ls })
        }).collect())
    }

    pub async fn delete_nodes_by_folder(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE folder_id = $1")
            .bind(folder_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn update_node_community(&self, node_id: &uuid::Uuid, community_id: i32) -> Result<(), String> {
        sqlx_core::query::query("UPDATE sensei.nodes SET community_id = $2, modified_at = now() WHERE id = $1")
            .bind(node_id).bind(community_id)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Edges ────────────────────────────────────────────────────────

    /// Insert (or upsert) an edge (D1). Edges carry an identity via two partial
    /// unique indexes, so a repeated identical insert returns the SAME row
    /// instead of duplicating. Branches on `target_id`: a resolved edge is keyed
    /// by its target node; an unresolved edge by `(target_name, target_file)`.
    /// `DO UPDATE SET modified_at = now()` (not `DO NOTHING`) so `RETURNING id`
    /// is always the surviving row's id.
    pub async fn insert_edge(
        &self, folder_id: &uuid::Uuid, source_id: &uuid::Uuid,
        target_id: Option<&uuid::Uuid>, target_name: Option<&str>,
        target_file: Option<&str>, kind: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = if let Some(tid) = target_id {
            sqlx_core::query_as::query_as(
                "INSERT INTO sensei.edges(folder_id, source_id, target_id, kind)
                 VALUES($1, $2, $3, $4::sensei.edge_kind)
                 ON CONFLICT (folder_id, source_id, target_id, kind) WHERE target_id IS NOT NULL
                   DO UPDATE SET modified_at = now()
                 RETURNING id"
            ).bind(folder_id).bind(source_id).bind(tid).bind(kind)
                .fetch_one(&self.pool).await.map_err(|e| e.to_string())?
        } else {
            sqlx_core::query_as::query_as(
                "INSERT INTO sensei.edges(folder_id, source_id, target_name, target_file, kind)
                 VALUES($1, $2, $3, $4, $5::sensei.edge_kind)
                 ON CONFLICT (folder_id, source_id, target_name, target_file, kind) WHERE target_id IS NULL
                   DO UPDATE SET modified_at = now()
                 RETURNING id"
            ).bind(folder_id).bind(source_id).bind(target_name).bind(target_file).bind(kind)
                .fetch_one(&self.pool).await.map_err(|e| e.to_string())?
        };
        Ok(row.0)
    }

    pub async fn get_callers(&self, node_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT e.id, e.source_id, e.kind::text FROM sensei.edges e WHERE e.target_id = $1 AND e.kind = 'calls'::sensei.edge_kind"
        ).bind(node_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, src, kind)| {
            serde_json::json!({ "edge_id": id, "caller_id": src, "kind": kind })
        }).collect())
    }

    pub async fn get_callees(&self, node_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, Option<uuid::Uuid>, Option<String>, String)> = sqlx_core::query_as::query_as(
            "SELECT e.id, e.target_id, e.target_name, e.kind::text FROM sensei.edges e WHERE e.source_id = $1 AND e.kind = 'calls'::sensei.edge_kind"
        ).bind(node_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, tgt, name, kind)| {
            serde_json::json!({ "edge_id": id, "callee_id": tgt, "callee_name": name, "kind": kind })
        }).collect())
    }

    /// Promote an unresolved edge to a resolved `target_id` (D1) — conflict-safe
    /// against `edges_unique_resolved`. If a resolved edge with the same
    /// `(folder_id, source_id, target_id, kind)` already exists, updating this
    /// row into it would violate the unique index; instead we MERGE — the UPDATE
    /// is guarded by a `NOT EXISTS`, and when it changes 0 rows (a dup exists, or
    /// the edge is already gone) we delete this now-redundant unresolved edge.
    ///
    /// The guard-then-delete is not one transaction, which is safe under the
    /// single-writer-per-folder invariant (W5/D6e): a folder's graph writes run as
    /// one barrier task at a time and the unique index is folder-scoped — so no
    /// concurrent resolve can race the `NOT EXISTS`.
    pub async fn resolve_edge(&self, edge_id: &uuid::Uuid, target_id: &uuid::Uuid) -> Result<(), String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.edges e
                SET target_id = $2, modified_at = now()
              WHERE e.id = $1
                AND NOT EXISTS (
                    SELECT 1 FROM sensei.edges d
                     WHERE d.folder_id = e.folder_id
                       AND d.source_id = e.source_id
                       AND d.target_id = $2
                       AND d.kind = e.kind
                       AND d.id <> e.id)"
        ).bind(edge_id).bind(target_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        if res.rows_affected() == 0 {
            // A resolved edge to the same target already exists (or this edge is
            // already gone): this unresolved edge is redundant — drop it so the
            // graph converges to the single resolved edge.
            sqlx_core::query::query("DELETE FROM sensei.edges WHERE id = $1")
                .bind(edge_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Replace a folder's entire edge set of one `kind` with `edges`, in ONE
    /// transaction (D2): DELETE every edge of that kind for the folder, then
    /// insert the current set. This makes a derived kind (e.g. `covers`) a pure
    /// function of the current tree — stale relations vanish instead of
    /// accumulating — and the single transaction means a crash can't leave the
    /// folder with a half-replaced (or empty) set: it either fully commits the
    /// new set or rolls back to the old one. Idempotent: re-running with the same
    /// set yields the same rows (the per-edge `ON CONFLICT` also absorbs a
    /// duplicate pair within the input set).
    pub async fn replace_edges_of_kind(
        &self, folder_id: &uuid::Uuid, kind: &str, edges: &[EdgeSpec],
    ) -> Result<(), String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx_core::query::query(
            "DELETE FROM sensei.edges WHERE folder_id = $1 AND kind = $2::sensei.edge_kind"
        ).bind(folder_id).bind(kind).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        for e in edges {
            if let Some(tid) = e.target_id {
                sqlx_core::query::query(
                    "INSERT INTO sensei.edges(folder_id, source_id, target_id, kind)
                     VALUES($1, $2, $3, $4::sensei.edge_kind)
                     ON CONFLICT (folder_id, source_id, target_id, kind) WHERE target_id IS NOT NULL
                       DO UPDATE SET modified_at = now()"
                ).bind(folder_id).bind(e.source_id).bind(tid).bind(kind)
                    .execute(&mut *tx).await.map_err(|e2| e2.to_string())?;
            } else {
                sqlx_core::query::query(
                    "INSERT INTO sensei.edges(folder_id, source_id, target_name, target_file, kind)
                     VALUES($1, $2, $3, $4, $5::sensei.edge_kind)
                     ON CONFLICT (folder_id, source_id, target_name, target_file, kind) WHERE target_id IS NULL
                       DO UPDATE SET modified_at = now()"
                ).bind(folder_id).bind(e.source_id).bind(e.target_name.as_deref()).bind(e.target_file.as_deref()).bind(kind)
                    .execute(&mut *tx).await.map_err(|e2| e2.to_string())?;
            }
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Prune a file's nodes that vanished from the latest parse (D3 upsert-then-
    /// prune): every node for `(folder, file_path)` whose id is NOT in `kept_ids`.
    /// First unresolve inbound edges pointing at them (clear `target_id`, KEEP
    /// `target_name` as an honest unresolved residual — the caller re-emits a
    /// resolved FQN edge when it is next processed, and a full reindex heals it;
    /// Phase 7.1 retired the `resolve_edges` re-point pass), then delete the nodes
    /// (their out-edges cascade via the `source_id` FK). One transaction. Returns
    /// nodes pruned. An empty `kept_ids` prunes ALL of the file's nodes.
    pub async fn prune_file_nodes(
        &self, folder_id: &uuid::Uuid, file_path: &str, kept_ids: &[uuid::Uuid],
    ) -> Result<u64, String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx_core::query::query(
            "UPDATE sensei.edges SET target_id = NULL, modified_at = now()
              WHERE folder_id = $1
                AND target_name IS NOT NULL
                AND target_id IN (
                    SELECT id FROM sensei.nodes
                     WHERE folder_id = $1 AND file_path = $2 AND id <> ALL($3))"
        ).bind(folder_id).bind(file_path).bind(kept_ids).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        let res = sqlx_core::query::query(
            "DELETE FROM sensei.nodes WHERE folder_id = $1 AND file_path = $2 AND id <> ALL($3)"
        ).bind(folder_id).bind(file_path).bind(kept_ids).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    /// Delete every out-edge sourced from `source_ids` in a folder (D3 per-file
    /// reconcile). A symbol that SURVIVES a re-index keeps its node id, so its
    /// stale out-edges (e.g. a call it no longer makes) aren't cascade-deleted —
    /// clear them so the caller can re-insert the current set (replace, not
    /// append). Returns rows deleted; an empty `source_ids` is a no-op.
    pub async fn delete_edges_from_sources(
        &self, folder_id: &uuid::Uuid, source_ids: &[uuid::Uuid],
    ) -> Result<u64, String> {
        if source_ids.is_empty() {
            return Ok(0);
        }
        let res = sqlx_core::query::query(
            "DELETE FROM sensei.edges WHERE folder_id = $1 AND source_id = ANY($2)"
        ).bind(folder_id).bind(source_ids).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    /// Un-resolve edges that point INTO a file's nodes: clear `target_id` while
    /// keeping `target_name`. Called before re-indexing a changed file so the
    /// inbound cross-file edges survive (they'd otherwise be cascade-deleted when
    /// the target nodes are dropped). They become an honest unresolved residual,
    /// re-pointed when the calling file is next processed (FQN edges resolve at
    /// emit — Phase 7.1 retired the resolve_edges pass). Returns edges un-resolved.
    pub async fn unresolve_edges_to_file(&self, folder_id: &uuid::Uuid, file_path: &str) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.edges SET target_id = NULL, modified_at = now()
              WHERE folder_id = $1
                AND target_id IN (SELECT id FROM sensei.nodes WHERE folder_id = $1 AND file_path = $2)
                AND target_name IS NOT NULL"
        ).bind(folder_id).bind(file_path).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    pub async fn get_edges_by_kind(&self, folder_id: &uuid::Uuid, kind: &str) -> Result<Vec<serde_json::Value>, String> {
        self.get_edges_scoped(std::slice::from_ref(folder_id), kind).await
    }

    // ── View-based graph queries ────────────────────────────────────

    /// Find callers of a function by name via the call_graph view.
    /// `scope` is resolved via [`scope_folder_ids`]: a project name/UUID expands
    /// to all of that project's folders; a bare folder name falls back to just
    /// that folder.
    pub async fn get_callers_by_name(&self, scope: &str, target: &str) -> Result<Vec<serde_json::Value>, String> {
        let folder_ids = self.scope_folder_ids(scope).await?;
        if folder_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(String, String, String, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT source_name, source_kind::text, source_file, source_line
               FROM sensei.call_graph
              WHERE folder_id = ANY($1) AND target_name = $2 AND edge_kind = 'calls'
              ORDER BY source_file, source_line LIMIT 100"
        ).bind(&folder_ids[..]).bind(target).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, kind, file, line)| {
            serde_json::json!({ "name": name, "kind": kind, "file_path": file, "line_start": line })
        }).collect())
    }

    /// Find callees of a function by name via the call_graph view.
    /// `scope` is resolved via [`scope_folder_ids`]: a project name/UUID expands
    /// to all of that project's folders; a bare folder name falls back to just
    /// that folder.
    pub async fn get_callees_by_name(&self, scope: &str, source: &str) -> Result<Vec<serde_json::Value>, String> {
        let folder_ids = self.scope_folder_ids(scope).await?;
        if folder_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(Option<String>, Option<String>, Option<String>, Option<i32>, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT target_name, target_kind::text, target_file, target_line, unresolved_target
               FROM sensei.call_graph
              WHERE folder_id = ANY($1) AND source_name = $2 AND edge_kind = 'calls'
              ORDER BY target_file, target_line LIMIT 100"
        ).bind(&folder_ids[..]).bind(source).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, kind, file, line, unresolved)| {
            let display_name = name.or(unresolved).unwrap_or_default();
            serde_json::json!({ "name": display_name, "kind": kind, "file_path": file, "line_start": line })
        }).collect())
    }

    /// Get files matching a tag via the file_tags view.
    pub async fn get_files_by_tag(&self, folder_name: &str, tag: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Vec<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, file_path, tags FROM sensei.file_tags
              WHERE folder = $1 AND $2 = ANY(tags)
              ORDER BY file_path LIMIT 200"
        ).bind(folder_name).bind(tag).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, fp, tags)| {
            serde_json::json!({ "id": id, "file_path": fp, "tags": tags })
        }).collect())
    }

    /// Get doc coverage with drift detection via the doc_coverage view.
    pub async fn get_doc_drift(&self, folder_name: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, String, String, bool)> = sqlx_core::query_as::query_as(
            "SELECT doc_name, doc_file, code_name, code_file, drifted
               FROM sensei.doc_coverage
              WHERE folder = $1
              ORDER BY drifted DESC, doc_file LIMIT 200"
        ).bind(folder_name).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(doc_name, doc_file, code_name, code_file, drifted)| {
            serde_json::json!({ "doc": doc_name, "docFile": doc_file, "code": code_name, "codeFile": code_file, "drifted": drifted })
        }).collect())
    }

    /// Count all edges across multiple folders (project-scoped variant).
    pub async fn count_edges_scoped(&self, folder_ids: &[uuid::Uuid]) -> Result<i64, String> {
        let row: (i64,) = sqlx_core::query_as::query_as(
            "SELECT COUNT(*) FROM sensei.edges WHERE folder_id = ANY($1)"
        ).bind(folder_ids).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Count all edges for a folder.
    pub async fn count_edges(&self, folder_id: &uuid::Uuid) -> Result<i64, String> {
        self.count_edges_scoped(&[*folder_id]).await
    }

    /// Delete nodes whose file_path starts with a given prefix (for folder deletion).
    pub async fn delete_nodes_by_path_prefix(&self, folder_id: &uuid::Uuid, prefix: &str) -> Result<u64, String> {
        let result = sqlx_core::query::query(
            "DELETE FROM sensei.nodes WHERE folder_id = $1 AND file_path LIKE $2 || '%'"
        ).bind(folder_id).bind(prefix).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(result.rows_affected())
    }

    /// Search libraries by name (ILIKE).
    pub async fn search_libraries(&self, query: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, name, ecosystem::text, description FROM sensei.libraries
             WHERE name ILIKE '%' || $1 || '%'
             ORDER BY name LIMIT 50"
        ).bind(query).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, eco, desc)| {
            serde_json::json!({ "id": id, "name": name, "ecosystem": eco, "description": desc })
        }).collect())
    }

    /// Get a single library by exact name.
    pub async fn get_library_by_name(&self, name: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, name, ecosystem::text, description FROM sensei.libraries
             WHERE name = $1
             ORDER BY name"
        ).bind(name).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, eco, desc)| {
            serde_json::json!({ "id": id, "name": name, "ecosystem": eco, "description": desc })
        }).collect())
    }

    /// Documentation pages for a library by name, optionally filtered to a
    /// single component. `component=None` returns every page (the handler
    /// builds the index/overview from these); `Some(c)` returns just that
    /// component's page(s). NULL-component pages (the library overview) sort
    /// first. This is what `get_lib_docs` reads — it must return the page
    /// CONTENT, not just library metadata.
    pub async fn get_library_pages(
        &self, name: &str, component: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT lp.title, lp.component, lp.description, lp.content,
                        COALESCE(lp.url, lp.local_path) AS location, lp.source_type::text
                   FROM sensei.library_pages lp
                   JOIN sensei.libraries l ON l.id = lp.library_id
                  WHERE l.name = $1
                    AND ($2::text IS NULL OR lp.component = $2)
                  ORDER BY (lp.component IS NULL) DESC, lp.component, lp.title"
            )
            .bind(name).bind(component)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(title, component, description, content, location, source_type)| {
            serde_json::json!({
                "title": title, "component": component,
                "description": description, "content": content,
                "location": location, "source": source_type,
            })
        }).collect())
    }

    /// Search library pages by title / component / content (ILIKE). Returns
    /// ranked matches with a short snippet rather than full content, so
    /// `search_lib_docs` is concise. Title/component hits rank above body hits.
    pub async fn search_library_pages(&self, query: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, Option<String>, Option<String>, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT l.name, lp.title, lp.component, lp.description,
                        left(lp.content, 400) AS snippet
                   FROM sensei.library_pages lp
                   JOIN sensei.libraries l ON l.id = lp.library_id
                  WHERE lp.title ILIKE '%' || $1 || '%'
                     OR lp.component ILIKE '%' || $1 || '%'
                     OR lp.content ILIKE '%' || $1 || '%'
                  ORDER BY (lp.title ILIKE '%' || $1 || '%') DESC,
                           (lp.component ILIKE '%' || $1 || '%') DESC,
                           l.name, lp.component
                  LIMIT 30"
            )
            .bind(query)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(library, title, component, description, snippet)| {
            serde_json::json!({
                "library": library, "title": title, "component": component,
                "description": description, "snippet": snippet,
            })
        }).collect())
    }

    /// List all sessions across all folders.
    /// List recent sessions, newest first. `range_days` (when `Some`) filters to
    /// sessions started within the last N days — powers the Observatory · Sessions
    /// digest range chips (7d/30d/90d); `None` = no time filter. `project` (when
    /// `Some`) scopes to one project. `agent` (the acp harness, e.g. "claude" /
    /// "zed") lets the digest label each row's assistant.
    pub async fn list_all_sessions(
        &self,
        limit: i64,
        range_days: Option<i64>,
        project: Option<&uuid::Uuid>,
    ) -> Result<Vec<serde_json::Value>, String> {
        // Join the project name so each session can be labelled, and return the
        // timestamps in the camelCase shape the SessionData wire type and the
        // observatory components actually read (startedAt / completedAt). `corrections`
        // powers the "Corrections" column (first-try / N× rework) per the mockup.
        type SessionRow = (
            uuid::Uuid, Option<String>, String, Option<String>, Option<String>,
            Option<bool>, i32, i32, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>,
            Option<String>,
        );
        let rows: Vec<SessionRow> = sqlx_core::query_as::query_as(
            "SELECT s.id, p.name, s.task, s.summary, s.outcome::text, s.ftr, s.turns, s.corrections,
                    s.started_at, s.completed_at, s.acp_id
             FROM activity.sessions s
             LEFT JOIN sensei.projects p ON p.id = s.project_id
             WHERE ($2::int IS NULL OR s.started_at >= now() - make_interval(days => $2::int))
               AND ($3::uuid IS NULL OR s.project_id = $3)
             ORDER BY s.started_at DESC LIMIT $1"
        ).bind(limit).bind(range_days).bind(project).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, project, task, summary, outcome, ftr, turns, corrections, started, completed, agent)| {
            serde_json::json!({
                "id": id,
                "project": project,
                "task": task,
                "summary": summary,
                "outcome": outcome,
                "ftr": ftr,
                "turns": turns,
                "corrections": corrections,
                "startedAt": started.to_rfc3339(),
                "completedAt": completed.map(|c| c.to_rfc3339()),
                "agent": agent,
            })
        }).collect())
    }

    // ── Extensions ────────────────────────────────────────────────────

    pub async fn create_extension(
        &self, kind: &str, name: &str, description: Option<&str>, content: Option<&str>,
        scope: &str, source: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.extensions(kind, name, description, content, scope, source)
             VALUES($1::sensei.extension_kind, $2, $3, $4, $5::sensei.extension_scope, $6::sensei.extension_source) RETURNING id"
        ).bind(kind).bind(name).bind(description).bind(content).bind(scope).bind(source)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn update_extension(&self, id: &uuid::Uuid, description: Option<&str>, content: Option<&str>) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.extensions SET description = COALESCE($2, description), content = COALESCE($3, content) WHERE id = $1"
        ).bind(id).bind(description).bind(content)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn list_extensions_by_kind(&self, kind: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, String, String, bool)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, description, scope::text, source::text, enabled FROM sensei.extensions WHERE kind = $1::sensei.extension_kind ORDER BY name"
        ).bind(kind).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, desc, scope, source, enabled)| {
            serde_json::json!({ "id": id, "kind": kind, "name": name, "description": desc, "scope": scope, "source": source, "enabled": enabled })
        }).collect())
    }

    pub async fn get_extension_history(&self, extension_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, i32, String, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT id, operation::text, revision, name, changed_at FROM history.past_extensions WHERE extension_id = $1 ORDER BY changed_at DESC"
        ).bind(extension_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, op, rev, name, ts)| {
            serde_json::json!({ "id": id, "operation": op, "revision": rev, "name": name, "changed_at": ts.to_rfc3339() })
        }).collect())
    }

    pub async fn delete_extension(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.extensions WHERE id = $1")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Folders ──────────────────────────────────────────────────────

    pub async fn upsert_folder(
        &self, root_id: &uuid::Uuid, kind: &str, name: &str, path: &str, abs_path: &str,
        parent_id: Option<&uuid::Uuid>, project_id: Option<&uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path, parent_id, project_id)
             VALUES($1, $2::sensei.folder_kind, $3, $4, $5, $6, $7)
             ON CONFLICT(abs_path) DO UPDATE SET name = EXCLUDED.name, project_id = COALESCE(EXCLUDED.project_id, folders.project_id), modified_at = now()
             RETURNING id"
        ).bind(root_id).bind(kind).bind(name).bind(path).bind(abs_path).bind(parent_id).bind(project_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_folders_by_root(&self, root_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, Option<uuid::Uuid>, serde_json::Value, String)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, path, abs_path, project_id, remote_urls, status::text FROM sensei.folders WHERE root_id = $1 ORDER BY path"
        ).bind(root_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, path, abs, pid, remotes, status)| {
            serde_json::json!({ "id": id, "kind": kind, "name": name, "path": path, "abs_path": abs, "project_id": pid, "remote_urls": remotes, "status": status })
        }).collect())
    }

    pub async fn delete_folder_tree(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        // CASCADE will handle children via parent_id FK
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
            .bind(folder_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// A live project root that shares a git remote URL with `remotes` — the signal
    /// that a since-vanished root was RENAMED/MOVED rather than deleted. Restricted
    /// to project-root kinds whose `abs_path` is in `live_abs` (the paths this scan
    /// just discovered) so we only ever remap onto a freshly-confirmed folder.
    /// Returns the first match's id, or `None`. Empty `remotes` → `None` (a folder
    /// with no remote can't be remote-matched). DB-only; pure lookup.
    pub async fn find_live_root_by_remote(
        &self,
        remotes: &[String],
        live_abs: &[String],
    ) -> Result<Option<uuid::Uuid>, String> {
        if remotes.is_empty() || live_abs.is_empty() {
            return Ok(None);
        }
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT f.id FROM sensei.folders f \
             WHERE f.abs_path = ANY($2) \
               AND f.kind IN ('git'::sensei.folder_kind, 'standalone'::sensei.folder_kind, 'subtree'::sensei.folder_kind) \
               AND EXISTS (SELECT 1 FROM jsonb_array_elements(f.remote_urls) e WHERE e->>'url' = ANY($1)) \
             LIMIT 1",
        )
        .bind(remotes)
        .bind(live_abs)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// Whether a folder carries history worth keeping — i.e. any `activity.sessions`
    /// row is attached to it. Drives archive-not-delete on a vanished root.
    pub async fn folder_has_sessions(&self, folder_id: &uuid::Uuid) -> Result<bool, String> {
        let row: Option<(i32,)> = sqlx_core::query_as::query_as(
            "SELECT 1 FROM activity.sessions WHERE folder_id = $1 LIMIT 1",
        )
        .bind(folder_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.is_some())
    }

    /// Re-point a renamed/moved root's history onto its new folder, then drop the
    /// now-empty old row. Order matters: (1) record the old path as an alias of the
    /// new folder so future events under old paths resolve forward; (2) move the
    /// old folder's sessions to the new folder BEFORE deleting (delete_folder_tree
    /// cascades sessions, so this must precede it); (3) delete the old husk. The
    /// alias is the durable mapping — even if a session slips through, the orphan
    /// repair re-attaches it via the alias.
    pub async fn remap_folder(
        &self,
        old_folder_id: &uuid::Uuid,
        old_abs_path: &str,
        new_folder_id: &uuid::Uuid,
    ) -> Result<(), String> {
        self.add_folder_path_alias(old_abs_path, new_folder_id, "rename").await?;
        sqlx_core::query::query(
            "UPDATE activity.sessions SET folder_id = $2 WHERE folder_id = $1",
        )
        .bind(old_folder_id)
        .bind(new_folder_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        self.delete_folder_tree(old_folder_id).await
    }

    /// Retain a vanished, history-bearing root as `archived` instead of deleting it:
    /// its directory is gone but its sessions/transcripts stay attached. The vanish
    /// prune and reconcile skip `archived` folders thereafter.
    pub async fn archive_folder(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET status = 'archived'::sensei.folder_status WHERE id = $1",
        )
        .bind(folder_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The folder registered at EXACTLY this absolute path (no alias resolution),
    /// or `None`. Distinct from [`Self::get_folder_ids_by_path`], which also follows
    /// aliases — the manual `remap` needs to know whether `old` is itself a real
    /// folder row (to re-point) versus already gone (alias-only).
    pub async fn folder_id_by_abs_path(&self, abs_path: &str) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.folders WHERE abs_path = $1 LIMIT 1",
        )
        .bind(abs_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// Write a git root's remotes (`[{name,url}]`) into `folders.remote_urls` — the
    /// producing half of git-remote rename detection, called during scan. Without
    /// this the column stays `'[]'` and [`Self::find_live_root_by_remote`] can never
    /// match, so auto-remap is inert (that was the pre-existing prod state).
    pub async fn update_folder_remotes(&self, folder_id: &uuid::Uuid, remotes: &serde_json::Value) -> Result<(), String> {
        sqlx_core::query::query("UPDATE sensei.folders SET remote_urls = $2 WHERE id = $1")
            .bind(folder_id)
            .bind(remotes)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// List folders in a non-terminal (recoverable) index state, for startup
    /// resume: `discovered` (scan ran, ProcessGitFolder hadn't started),
    /// `queued` (enqueued, not started), `indexing` (a scan was in-flight when
    /// the daemon stopped — its in-memory task was lost, D6a), and `failed`
    /// (errored, should retry). `indexed`, `deferred` (intentionally not indexed
    /// — sibling/standalone), and `archived` (directory gone) are terminal and
    /// excluded.
    ///
    /// Called once at daemon startup to rebuild the in-memory queue, which
    /// otherwise loses every task on restart. Re-enqueuing an already-running
    /// folder is deduped by the single-writer guard (`enqueue_unique`).
    pub async fn list_pending_folders(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, String, String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT id, root_id, kind::text, name, abs_path, status::text \
             FROM sensei.folders \
             WHERE status IN ('discovered'::sensei.folder_status, 'queued'::sensei.folder_status, \
                              'indexing'::sensei.folder_status, 'failed'::sensei.folder_status) \
             ORDER BY abs_path"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, root_id, kind, name, abs_path, status)| {
            serde_json::json!({
                "id": id,
                "root_id": root_id,
                "kind": kind,
                "name": name,
                "abs_path": abs_path,
                "status": status,
            })
        }).collect())
    }

    /// Count folders belonging to a project that have not yet reached a terminal
    /// index state. Returns 0 when all folders are `indexed` or `failed`.
    pub async fn count_unindexed_folders(&self, project_id: uuid::Uuid) -> Result<i64, String> {
        let row: (i64,) = sqlx_core::query_as::query_as(
            "SELECT COUNT(*) FROM sensei.folders
              WHERE project_id = $1
                AND status NOT IN ('indexed'::sensei.folder_status, 'failed'::sensei.folder_status)"
        ).bind(project_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    // ── Benchmark Reports ────────────────────────────────────────────

    pub async fn create_benchmark_report(
        &self, folder_id: Option<&uuid::Uuid>, run_name: &str, strategy: &str,
        score: Option<f64>, tokens: Option<i32>, elapsed_ms: Option<i32>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.benchmark_reports(folder_id, run_name, strategy, score, tokens, elapsed_ms) VALUES($1, $2, $3, $4, $5, $6) RETURNING id"
        ).bind(folder_id).bind(run_name).bind(strategy).bind(score).bind(tokens).bind(elapsed_ms)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_benchmark_reports(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<f64>, Option<i32>, bool, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT id, run_name, strategy, score::float8, tokens, promoted, modified_at FROM sensei.benchmark_reports ORDER BY modified_at DESC"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, strategy, score, tokens, promoted, modified)| {
            serde_json::json!({ "id": id, "run_name": name, "strategy": strategy, "score": score, "tokens": tokens, "promoted": promoted, "modified_at": modified.to_rfc3339() })
        }).collect())
    }

    // ── Views (read-only) ────────────────────────────────────────────

    pub async fn list_repositories(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT id, name, abs_path, kind::text FROM sensei.folders WHERE kind IN ('git'::sensei.folder_kind, 'subtree'::sensei.folder_kind, 'standalone'::sensei.folder_kind) ORDER BY name"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, abs_path, kind)| {
            serde_json::json!({ "id": id, "name": name, "abs_path": abs_path, "kind": kind })
        }).collect())
    }

    // ── Memories ──────────────────────────────────────────────────────

    pub async fn create_memory(
        &self, project_id: Option<&uuid::Uuid>, scope: &str, scope_filter: Option<&str>,
        mem_type: &str, title: &str, content: &str, impact: Option<&str>,
        session_id: Option<&uuid::Uuid>, spine_slot: Option<&str>, feature: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memories(project_id, scope, scope_filter, type, title, content, impact, session_id, spine_slot, feature)
             VALUES($1, $2::sensei.memory_scope, $3, $4::sensei.memory_type, $5, $6, $7, $8, $9::sensei.spine_slot, $10) RETURNING id"
        ).bind(project_id).bind(scope).bind(scope_filter).bind(mem_type)
            .bind(title).bind(content).bind(impact).bind(session_id)
            .bind(spine_slot).bind(feature)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn reinforce_memory(&self, id: &uuid::Uuid, amount: f64) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.memories SET strength = LEAST(strength + $2, 5.0), modified_at = now() WHERE id = $1"
        ).bind(id).bind(amount).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn archive_memory(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.memories SET status = 'archived'::sensei.memory_status, modified_at = now() WHERE id = $1"
        ).bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_memory(&self, id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, String, String, Option<String>, f64, String, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, project_id, scope::text, scope_filter, type::text, title, content, impact, strength::float8, status::text, modified_at FROM sensei.memories WHERE id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id, pid, scope, filter, mtype, title, content, impact, strength, status, modified)| {
            serde_json::json!({
                "id": id, "project_id": pid, "scope": scope, "scope_filter": filter,
                "type": mtype, "title": title, "content": content, "impact": impact,
                "strength": strength, "status": status, "modified_at": modified.to_rfc3339(),
            })
        }))
    }

    pub async fn list_active_memories(&self, project_id: Option<&uuid::Uuid>, scope: Option<&str>) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, String, String, String, Option<String>, f64)> = match (project_id, scope) {
            (Some(pid), Some(s)) => sqlx_core::query_as::query_as(
                "SELECT id, scope::text, scope_filter, type::text, title, content, impact, strength::float8
                 FROM sensei.memories WHERE status = 'active' AND strength >= 1.0 AND (project_id = $1 OR project_id IS NULL) AND scope = $2::sensei.memory_scope
                 ORDER BY strength DESC"
            ).bind(pid).bind(s).fetch_all(&self.pool).await,
            (Some(pid), None) => sqlx_core::query_as::query_as(
                "SELECT id, scope::text, scope_filter, type::text, title, content, impact, strength::float8
                 FROM sensei.memories WHERE status = 'active' AND strength >= 1.0 AND (project_id = $1 OR project_id IS NULL)
                 ORDER BY strength DESC"
            ).bind(pid).fetch_all(&self.pool).await,
            _ => sqlx_core::query_as::query_as(
                "SELECT id, scope::text, scope_filter, type::text, title, content, impact, strength::float8
                 FROM sensei.memories WHERE status = 'active' AND strength >= 1.0 AND project_id IS NULL
                 ORDER BY strength DESC"
            ).fetch_all(&self.pool).await,
        }.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, scope, filter, mtype, title, content, impact, strength)| {
            serde_json::json!({ "id": id, "scope": scope, "scope_filter": filter, "type": mtype, "title": title, "content": content, "impact": impact, "strength": strength })
        }).collect())
    }

    /// In-force adopted memories across ALL projects — powers the Observatory ·
    /// Today adopted lane. Same in-force filter as [`Self::list_active_memories`]
    /// (`status='active'`, `strength>=1.0`) but not scoped to a single
    /// project/global namespace.
    pub async fn list_active_memories_global(&self, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, f64, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT m.id, m.title, m.scope::text, m.impact, m.strength::float8, m.modified_at
             FROM sensei.memories m
             WHERE m.status = 'active' AND m.strength >= 1.0
             ORDER BY m.strength DESC, m.modified_at DESC
             LIMIT $1"
        ).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, scope, impact, strength, modified)| {
            serde_json::json!({ "id": id, "title": title, "scope": scope,
                                "impact": impact, "strength": strength,
                                "modified_at": modified.to_rfc3339() })
        }).collect())
    }

    // ── Memory Examples ──────────────────────────────────────────────

    pub async fn add_memory_example(&self, memory_id: &uuid::Uuid, node_id: &str, is_good: bool, note: Option<&str>) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memory_examples(memory_id, node_id, is_good, note) VALUES($1, $2, $3, $4) RETURNING id"
        ).bind(memory_id).bind(node_id).bind(is_good).bind(note)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_memory_examples(&self, memory_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, bool, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, node_id, is_good, note FROM sensei.memory_examples WHERE memory_id = $1"
        ).bind(memory_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, nid, good, note)| {
            serde_json::json!({ "id": id, "node_id": nid, "is_good": good, "note": note })
        }).collect())
    }

    // ── Memory Evidence ──────────────────────────────────────────────

    /// Attach one piece of evidence to a memory: a session where it was learned/
    /// confirmed (`session_id = Some`), OR a save-time source note (`session_id =
    /// None`, e.g. a file:line / test / run ref supplied with the memory).
    pub async fn add_memory_evidence(&self, memory_id: &uuid::Uuid, session_id: Option<&uuid::Uuid>, note: Option<&str>) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memory_evidence(memory_id, session_id, note) VALUES($1, $2, $3) RETURNING id"
        ).bind(memory_id).bind(session_id).bind(note)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_memory_evidence(&self, memory_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, Option<uuid::Uuid>, Option<String>, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT id, session_id, note, modified_at FROM sensei.memory_evidence WHERE memory_id = $1 ORDER BY modified_at"
        ).bind(memory_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, sid, note, modified)| {
            serde_json::json!({ "id": id, "session_id": sid, "note": note, "modified_at": modified.to_rfc3339() })
        }).collect())
    }

    // ── Memory Links ─────────────────────────────────────────────────

    pub async fn link_memories(&self, parent_id: &uuid::Uuid, child_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.memory_links(parent_id, child_id) VALUES($1, $2) ON CONFLICT DO NOTHING"
        ).bind(parent_id).bind(child_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_memory_children(&self, parent_id: &uuid::Uuid) -> Result<Vec<uuid::Uuid>, String> {
        let rows: Vec<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT child_id FROM sensei.memory_links WHERE parent_id = $1"
        ).bind(parent_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn get_memory_parent(&self, child_id: &uuid::Uuid) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT parent_id FROM sensei.memory_links WHERE child_id = $1"
        ).bind(child_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    // ── Recommendations (inference) ──────────────────────────────────

    pub async fn create_recommendation(
        &self, project_id: &uuid::Uuid, title: &str, why: &str, action_type: &str, urgency: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.recommendations(project_id, title, why, action_type, urgency)
             VALUES($1, $2, $3, $4, $5::sensei.recommendation_urgency) RETURNING id"
        ).bind(project_id).bind(title).bind(why).bind(action_type).bind(urgency)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
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

    /// Accept a `pending` recommendation and carry out the action it stands for.
    ///
    /// Guards the transition to `accepted` at the `pending` state so a
    /// double-click / stale UI can't push an already-decided rec back to
    /// accepted — errors (verbatim) when the row is missing or already decided,
    /// which the HTTP handler maps to 409. Because the guard fires at most once,
    /// the action side effect below runs at most once too.
    ///
    /// A `promote_pattern` rec advances its source pattern's lifecycle to `rule`
    /// (`based_on.patterns[0]`), which the Patterns read path then renders as an
    /// `adopted` pattern. Non-atomic by design: the status flip and the
    /// lifecycle advance are two autocommit statements rather than one
    /// transaction, because reusing `promote_pattern` (DRY) precludes enrolling
    /// it in a caller-side tx without duplicating its SQL. The pending-guard
    /// already makes re-promotion impossible, so the only failure window —
    /// status flipped, promote failed — is a logged inconsistency (surfaced at
    /// error level), never a double-write.
    pub async fn accept_recommendation(&self, id: &uuid::Uuid) -> Result<(), String> {
        let row: Option<(String, String)> = sqlx_core::query_as::query_as(
            "UPDATE inference.recommendations
                SET status = 'accepted'::sensei.recommendation_status,
                    acted_at = now()
              WHERE id = $1 AND status = 'pending'
          RETURNING action_type, based_on::text"
        ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        let Some((action_type, based_on)) = row else {
            return Err("recommendation not found or already decided".into());
        };

        // A missing/empty/non-uuid `based_on.patterns[0]` yields None here, so a
        // promote_pattern rec with no provenance short-circuits to a no-op.
        if action_type == "promote_pattern"
            && let Some(pattern_id) = Self::based_on_first_pattern(&based_on)
            && let Err(e) = self.promote_pattern(&pattern_id, "rule").await
        {
            // The status flip already committed; the guard blocks a
            // retry-promotion, so log loudly rather than swallow — the
            // rec IS accepted, only the lifecycle advance was lost.
            tracing::error!(
                error = %e, recommendation = %id, pattern = %pattern_id,
                "accept_recommendation: pattern promotion failed after status flip"
            );
        }
        Ok(())
    }

    /// Move a `pending` recommendation to `dismissed` (the reject terminal —
    /// the enum uses `dismissed`, not `rejected`). Same shape as accept:
    /// idempotency-guarded so a stale UI can't clobber a real decision.
    pub async fn reject_recommendation(&self, id: &uuid::Uuid) -> Result<(), String> {
        let result = sqlx_core::query::query(
            "UPDATE inference.recommendations
                SET status = 'dismissed'::sensei.recommendation_status,
                    acted_at = now()
              WHERE id = $1 AND status = 'pending'"
        ).bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        if result.rows_affected() == 0 {
            return Err("recommendation not found or already decided".into());
        }
        Ok(())
    }

    pub async fn measure_recommendation(&self, id: &uuid::Uuid, verdict: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE inference.recommendations SET verdict = $2::sensei.recommendation_verdict, measured_at = now() WHERE id = $1"
        ).bind(id).bind(verdict).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn list_recommendations(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT id, title, why, urgency::text, status::text, verdict::text FROM inference.recommendations WHERE project_id = $1 ORDER BY urgency::text"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, why, urg, status, verdict)| {
            serde_json::json!({ "id": id, "title": title, "why": why, "urgency": urg, "status": status, "verdict": verdict })
        }).collect())
    }

    /// Insert a recommendation with provenance (#69 L2 generator). `based_on`
    /// links the L1/L2 artifacts reasoned over (`{patterns,memories,corrections}`),
    /// distinct from raw session/file `evidence`. Used for idempotency.
    pub async fn create_recommendation_full(
        &self, project_id: &uuid::Uuid, title: &str, why: &str, impact: Option<&str>,
        action_type: &str, urgency: &str, based_on: &serde_json::Value,
        reasoning_trace_id: Option<&uuid::Uuid>, prompt: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.recommendations(project_id, title, why, impact, action_type, urgency, based_on, reasoning_trace_id, prompt)
             VALUES($1, $2, $3, $4, $5, $6::sensei.recommendation_urgency, $7::jsonb, $8, $9) RETURNING id"
        ).bind(project_id).bind(title).bind(why).bind(impact).bind(action_type).bind(urgency)
            .bind(based_on.to_string()).bind(reasoning_trace_id).bind(prompt)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// True if any recommendation for `project_id` already cites `pattern_id` in
    /// `based_on.patterns`. The L2 generator's idempotency guard.
    pub async fn recommendation_exists_for_pattern(
        &self, project_id: &uuid::Uuid, pattern_id: &uuid::Uuid,
    ) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(
               SELECT 1 FROM inference.recommendations
                WHERE project_id = $1 AND based_on->'patterns' @> to_jsonb($2::text)
             )"
        ).bind(project_id).bind(pattern_id.to_string())
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    // ── Recommendation ranking (ranking.rs) ──────────────────────────

    /// Pending recs for a project with the scoring factors joined from their
    /// source patterns (`based_on.patterns` → `detected_patterns`): returns
    /// `(id, action_type, urgency, avg_confidence, max_recurrence)`. A rec with
    /// no joinable pattern yields `avg_confidence = None`, `max_recurrence = 0`.
    pub async fn get_pending_recs_for_ranking(
        &self, project_id: &uuid::Uuid,
    ) -> Result<Vec<(uuid::Uuid, String, String, Option<f64>, i32)>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<f64>, i32)> = sqlx_core::query_as::query_as(
            "SELECT r.id, r.action_type, r.urgency::text,
                    avg(dp.confidence)::float8 AS avg_conf,
                    COALESCE(max(dp.instance_count), 0)::int4 AS max_recur
               FROM inference.recommendations r
               LEFT JOIN LATERAL jsonb_array_elements_text(
                     CASE WHEN jsonb_typeof(r.based_on->'patterns') = 'array'
                          THEN r.based_on->'patterns' ELSE '[]'::jsonb END
                   ) AS pid(v) ON true
               LEFT JOIN inference.detected_patterns dp ON dp.id = pid.v::uuid
              WHERE r.project_id = $1 AND r.status = 'pending'
              GROUP BY r.id, r.action_type, r.urgency",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Clear the focal flag across a project (before a fresh ranking pass marks a
    /// new one) so a previously-focal rec that has since been acted on or
    /// out-scored never stays flagged.
    pub async fn clear_project_focal(&self, project_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE inference.recommendations SET focal = false WHERE project_id = $1 AND focal",
        )
        .bind(project_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Persist a rec's computed `score` + `focal`, mirroring the factor
    /// breakdown into `based_on.score_factors` for explainability.
    pub async fn set_recommendation_rank(
        &self, id: &uuid::Uuid, score: f64, focal: bool, factors: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE inference.recommendations
                SET score = $2::float8::numeric(5,2),
                    focal = $3,
                    based_on = jsonb_set(based_on, '{score_factors}', $4::jsonb, true)
              WHERE id = $1",
        )
        .bind(id)
        .bind(score)
        .bind(focal)
        .bind(factors.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Communities (inference) ───────────────────────────────────────

    pub async fn upsert_community(&self, folder_id: &uuid::Uuid, community_id: i32, label: &str, node_count: i32) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.communities(folder_id, community_id, label, node_count)
             VALUES($1, $2, $3, $4)
             ON CONFLICT(folder_id, community_id) DO UPDATE SET label = EXCLUDED.label, node_count = EXCLUDED.node_count, modified_at = now()
             RETURNING id"
        ).bind(folder_id).bind(community_id).bind(label).bind(node_count)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Recompute `nodes.degree` for every node in a folder (D4.5) — the in+out
    /// count of edges incident to the node (source, plus resolved target). Run at
    /// the start of the `DetectCommunities` terminal barrier (Phase 7.1 moved it
    /// there from the retired `ResolveEdges` pass) so degree is fresh before it
    /// ranks each community's god nodes. Edgeless nodes are set to 0 (not left
    /// stale/NULL), so a symbol that lost its last edge on a re-scan reflects it.
    pub async fn recompute_degrees_for_folder(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.nodes n
                SET degree = COALESCE(d.deg, 0), modified_at = now()
               FROM (SELECT id FROM sensei.nodes WHERE folder_id = $1) an
               LEFT JOIN (
                   SELECT node_id, count(*)::int AS deg FROM (
                       SELECT source_id AS node_id FROM sensei.edges WHERE folder_id = $1
                       UNION ALL
                       SELECT target_id AS node_id FROM sensei.edges WHERE folder_id = $1 AND target_id IS NOT NULL
                   ) inc GROUP BY node_id
               ) d ON d.node_id = an.id
              WHERE n.id = an.id"
        ).bind(folder_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Set `is_test` for every node of a file (folder-scoped) to the file's
    /// test-ness (`languages::is_test_path`). `is_test` is a FILE-level property —
    /// all of a file's nodes (file/symbol/section/rationale/fqn-def) share it — so
    /// this runs once per file after emit rather than threading a param through
    /// every upsert. Guarded by `IS DISTINCT FROM` so a steady-state re-scan
    /// changes 0 rows (cheap) while a test↔prod rename flips them. `lib_symbol`/
    /// `lib_package` nodes (file_path NULL) are never matched (external deps aren't
    /// test). Returns rows changed.
    pub async fn set_nodes_is_test_for_file(
        &self, folder_id: &uuid::Uuid, file_path: &str, is_test: bool,
    ) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.nodes SET is_test = $3, modified_at = now()
              WHERE folder_id = $1 AND file_path = $2 AND is_test IS DISTINCT FROM $3"
        ).bind(folder_id).bind(file_path).bind(is_test).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    /// Replace a folder's ENTIRE community assignment in one transaction (D4):
    /// delete its community rows, clear every node's `community_id`, then insert
    /// the new communities and set their members' `community_id`. This makes
    /// `inference.communities` + `nodes.community_id` a pure function of the
    /// current graph — no stale community rows, no stranded/orphaned
    /// `community_id`s (invariant 5) — and atomic (a crash can't leave a
    /// half-assigned folder). An empty `communities` just clears the folder.
    pub async fn replace_communities_for_folder(
        &self, folder_id: &uuid::Uuid, communities: &[CommunityAssignment],
    ) -> Result<(), String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx_core::query::query("DELETE FROM inference.communities WHERE folder_id = $1")
            .bind(folder_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        sqlx_core::query::query(
            "UPDATE sensei.nodes SET community_id = NULL, modified_at = now()
              WHERE folder_id = $1 AND community_id IS NOT NULL"
        ).bind(folder_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        for c in communities {
            // Authoritative write: description honest-empty (`props.source='null'`);
            // enrich_community_descriptions fills real prose later, off-barrier.
            sqlx_core::query::query(
                "INSERT INTO inference.communities(folder_id, community_id, label, node_count, god_node_ids, description, props)
                 VALUES($1, $2, $3, $4, $5, NULL, '{\"source\":\"null\"}'::jsonb)"
            ).bind(folder_id).bind(c.community_id).bind(&c.label).bind(c.member_node_ids.len() as i32)
                .bind(&c.god_node_ids)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            if !c.member_node_ids.is_empty() {
                sqlx_core::query::query(
                    "UPDATE sensei.nodes SET community_id = $2, modified_at = now()
                      WHERE folder_id = $1 AND id = ANY($3)"
                ).bind(folder_id).bind(c.community_id).bind(&c.member_node_ids)
                    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            }
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn list_communities(&self, folder_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, i32)> = sqlx_core::query_as::query_as(
            "SELECT id, label, node_count FROM inference.communities WHERE folder_id = $1 ORDER BY node_count DESC"
        ).bind(folder_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, label, count)| {
            serde_json::json!({ "id": id, "label": label, "node_count": count })
        }).collect())
    }

    /// Communities across ALL folders of a project scope (one query). Communities
    /// are stored per-folder and the repo root usually owns them, so a caller must
    /// aggregate over every scope folder — a single-folder lookup (a leaf) misses
    /// them (the #G5a `get_communities` bug).
    pub async fn list_communities_scoped(&self, folder_ids: &[uuid::Uuid]) -> Result<Vec<serde_json::Value>, String> {
        if folder_ids.is_empty() { return Ok(vec![]); }
        let rows: Vec<(uuid::Uuid, String, i32)> = sqlx_core::query_as::query_as(
            "SELECT id, label, node_count FROM inference.communities WHERE folder_id = ANY($1) ORDER BY node_count DESC"
        ).bind(folder_ids).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, label, count)| {
            serde_json::json!({ "id": id, "label": label, "node_count": count })
        }).collect())
    }

    /// Communities across a project scope with LIVE membership counts (7.3): the
    /// `node_count` is computed from the real `nodes.community_id` join, not the
    /// denormalized `communities.node_count` — so the overview reflects the
    /// current graph (a node whose community changed since the last detect is
    /// counted where it actually is now). Also carries `god_node_ids`. Ordered by
    /// live count desc. This is what turns the flat "scattered circles" overview
    /// into one sized by real per-community membership.
    pub async fn list_communities_live_scoped(&self, folder_ids: &[uuid::Uuid]) -> Result<Vec<serde_json::Value>, String> {
        if folder_ids.is_empty() { return Ok(vec![]); }
        let rows: Vec<(uuid::Uuid, Option<String>, i64, Vec<uuid::Uuid>)> = sqlx_core::query_as::query_as(
            "SELECT c.id, c.label, count(n.id) AS live_count, c.god_node_ids
               FROM inference.communities c
               LEFT JOIN sensei.nodes n
                 ON n.folder_id = c.folder_id AND n.community_id = c.community_id
              WHERE c.folder_id = ANY($1)
              GROUP BY c.id, c.label, c.god_node_ids
              ORDER BY live_count DESC, c.id"
        ).bind(folder_ids).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, label, count, gods)| {
            serde_json::json!({ "id": id, "label": label.unwrap_or_default(), "node_count": count, "god_node_ids": gods })
        }).collect())
    }

    /// The folder's communities with their `god_node_ids`, largest first — the
    /// input to description enrichment (D4.5). Bounded by `limit` so a huge cold
    /// repo enriches only its most significant clusters per detect run.
    pub async fn list_communities_with_god_nodes(
        &self, folder_id: &uuid::Uuid, limit: i64,
    ) -> Result<Vec<(i32, String, i32, Vec<uuid::Uuid>)>, String> {
        let rows: Vec<(i32, Option<String>, i32, Vec<uuid::Uuid>)> = sqlx_core::query_as::query_as(
            "SELECT community_id, label, node_count, god_node_ids
               FROM inference.communities
              WHERE folder_id = $1
              ORDER BY node_count DESC, community_id
              LIMIT $2"
        ).bind(folder_id).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(cid, label, n, gods)| (cid, label.unwrap_or_default(), n, gods)).collect())
    }

    /// `(id, name, kind)` for a set of node ids — builds community description
    /// facts from the god-node hubs. Empty input is a no-op.
    pub async fn get_node_name_kind(
        &self, ids: &[uuid::Uuid],
    ) -> Result<Vec<(uuid::Uuid, String, String)>, String> {
        if ids.is_empty() { return Ok(vec![]); }
        let rows: Vec<(uuid::Uuid, String, String)> = sqlx_core::query_as::query_as(
            "SELECT id, name, kind::text FROM sensei.nodes WHERE id = ANY($1)"
        ).bind(ids).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Stamp a community's model-authored `description` + its provenance
    /// (`props.source`), replacing the honest-empty placeholder from the
    /// authoritative write (D4.5). Only called on a successful insight-copy
    /// generation — a failure leaves the honest-empty NULL/`'null'` as written.
    pub async fn set_community_description(
        &self, folder_id: &uuid::Uuid, community_id: i32, description: &str, source: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE inference.communities
                SET description = $3,
                    props = props || jsonb_build_object('source', $4::text),
                    modified_at = now()
              WHERE folder_id = $1 AND community_id = $2"
        ).bind(folder_id).bind(community_id).bind(description).bind(source)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Reasoning Traces (inference) ─────────────────────────────────

    pub async fn insert_reasoning_trace(
        &self, project_id: Option<&uuid::Uuid>, trigger_event: &str, trigger_detail: &serde_json::Value,
        models_used: &[String], exchanges: &serde_json::Value, consensus: &serde_json::Value,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.reasoning_traces(project_id, trigger_event, trigger_detail, models_used, exchanges, consensus) VALUES($1, $2, $3, $4, $5, $6) RETURNING id"
        ).bind(project_id).bind(trigger_event).bind(trigger_detail).bind(models_used).bind(exchanges).bind(consensus)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// True if a reasoning trace for `project_id` already carries this
    /// finding-set `signature` (in `trigger_detail`). The consolidation tier's
    /// idempotency guard — keeps the LLM call from re-firing on the same signals.
    pub async fn reasoning_trace_exists_with_signature(&self, project_id: &uuid::Uuid, signature: &str) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM inference.reasoning_traces WHERE project_id = $1 AND trigger_detail->>'signature' = $2)"
        ).bind(project_id).bind(signature).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Impact reports (#70 read-path): recommendations that have been acted on
    /// or carry a consolidation trace, joined to that trace. Powers the
    /// Observatory Impact view (before/after FTR + the MOE-style reasoning).
    pub async fn get_project_impact(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, Option<f64>, Option<f64>, serde_json::Value, Option<Vec<String>>, Option<serde_json::Value>)> =
            sqlx_core::query_as::query_as(
                "SELECT r.id, r.title, r.action_type, r.status::text, r.verdict::text,
                        r.baseline_ftr::float8, r.current_ftr::float8, r.props,
                        t.models_used, t.consensus
                   FROM inference.recommendations r
                   LEFT JOIN inference.reasoning_traces t ON t.id = r.reasoning_trace_id
                  WHERE r.project_id = $1
                    AND (r.reasoning_trace_id IS NOT NULL OR r.verdict <> 'pending'::sensei.recommendation_verdict)
                  ORDER BY r.measured_at DESC NULLS LAST"
            ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, action_type, status, verdict, baseline, current, props, models, consensus)| {
            // The reasoning field carries the honest single-verdict JSON to the
            // UI: `{headline, body, modelsUsed: string[], suggestedRevision}`
            // when measure has populated it, or null when the rec has no trace
            // yet. HONEST SINGLE VERDICT (#109 audit): no fabricated consensus
            // tally or per-model panelist verdicts — there is one FTR-delta
            // verdict, and `modelsUsed` lists the models that actually ran.
            let reasoning = consensus.map(|synth| {
                // The honest synth is marked by `headline` — flow it straight through.
                if synth.get("headline").is_some() {
                    synth
                } else {
                    // Legacy/old-shape trace (e.g. `{conclusion}` from the retired
                    // consensus path). Surface the REAL model names from the trace;
                    // never fabricate per-model roles/notes/verdicts.
                    let conclusion = synth.get("conclusion")
                        .and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let names = models.clone().unwrap_or_default();
                    serde_json::json!({
                        "headline":          if conclusion.is_empty() { "Reasoning captured (no narrative)".into() } else { conclusion },
                        "body":              serde_json::Value::Null,
                        "modelsUsed":        names,
                        "suggestedRevision": serde_json::Value::Null,
                    })
                }
            });

            serde_json::json!({
                "id": id, "title": title, "actionType": action_type, "status": status,
                "verdict": verdict, "baselineFtr": baseline, "currentFtr": current,
                "ftrDelta": match (current, baseline) { (Some(c), Some(b)) => Some(((c - b) * 1000.0).round() / 1000.0), _ => None },
                "props": props,
                "reasoning": reasoning,
            })
        }).collect())
    }

    // ── Tool insights cache (T2 Slice D) ─────────────────────────────────

    /// Append a snapshot row for one tool. Called by the
    /// `AggregateToolInsights` task once per tool per tick. Historical rows
    /// stay in place so a follow-up trend chart can walk `computed_at` back
    /// in time.
    pub async fn insert_tool_insight(
        &self,
        tool_name: &str,
        metrics: &serde_json::Value,
        signal: Option<&crate::api::handlers::tool_signals::Signal>,
    ) -> Result<(), String> {
        use crate::tasks::handlers::tool_insights::variant_str;
        let (variant, title, detail) = match signal {
            Some(s) => (Some(variant_str(s.variant)), Some(s.title.as_str()), Some(s.detail.as_str())),
            None => (None, None, None),
        };
        sqlx_core::query::query(
            "INSERT INTO sensei.tool_insights
                (tool_name, metrics, signal_variant, signal_title, signal_detail)
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(tool_name)
        .bind(metrics)
        .bind(variant)
        .bind(title)
        .bind(detail)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Read the latest cached insight per tool. Returns `(tool_name,
    /// computed_at, metrics, signal_variant, signal_title, signal_detail)`
    /// tuples ordered by variant priority (warn > opportunity > unused >
    /// win > null) so the caller can render them straight through.
    pub async fn get_latest_tool_insights(
        &self,
    ) -> Result<Vec<serde_json::Value>, String> {
        // DISTINCT ON (tool_name) with ORDER BY tool_name, computed_at DESC
        // is the compact "latest row per tool" trick — Postgres picks the
        // first tuple per group. Wrapped in an outer SELECT so we can add
        // the variant-priority ordering the endpoint expects.
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            String,                                // tool_name
            chrono::DateTime<chrono::Utc>,         // computed_at
            serde_json::Value,                     // metrics
            Option<String>,                        // variant
            Option<String>,                        // title
            Option<String>,                        // detail
        )> = sqlx_core::query_as::query_as(
            "SELECT tool_name, computed_at, metrics,
                    signal_variant, signal_title, signal_detail
               FROM (
                 SELECT DISTINCT ON (tool_name)
                        tool_name, computed_at, metrics,
                        signal_variant, signal_title, signal_detail
                   FROM sensei.tool_insights
                  ORDER BY tool_name, computed_at DESC
               ) latest
              ORDER BY CASE signal_variant
                         WHEN 'warn'        THEN 0
                         WHEN 'opportunity' THEN 1
                         WHEN 'unused'      THEN 2
                         WHEN 'win'         THEN 3
                         ELSE 4
                       END,
                       computed_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(tool_name, computed_at, metrics, variant, title, detail)| {
            serde_json::json!({
                "toolName":   tool_name,
                "computedAt": computed_at.to_rfc3339(),
                "metrics":    metrics,
                "variant":    variant,
                "title":      title,
                "detail":     detail,
            })
        }).collect())
    }

    // ── Doc-drift scan (T3 Slice 2.3) ────────────────────────────────────

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

    /// Upsert the current code-symbol names into the global `symbol_names`
    /// registry (monotonic — never prunes). The doc-drift scan reads this history
    /// to tell a REMOVED symbol (real drift) from an identifier that was never a
    /// symbol (prose/config — not drift). Returns the number of names recorded.
    pub async fn record_symbol_names(&self) -> Result<u64, String> {
        let sql = format!(
            "INSERT INTO sensei.symbol_names (name)
             SELECT DISTINCT name FROM sensei.nodes
              WHERE kind IN ({kinds}) AND name <> ''
             ON CONFLICT (name) DO UPDATE SET last_seen = now()",
            kinds = Self::DRIFT_SYMBOL_KINDS
        );
        let res = sqlx_core::query::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    pub async fn scan_project_doc_drift(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<serde_json::Value, String> {
        use crate::analysis::doc_drift::{
            extract_identifier_mentions, extract_mention_from_detail, is_broken_drift,
        };

        // 1. Load every doc node in this project along with its folder id
        //    and the absolute path so we can read the file content off disk.
        //    `n.content` is intentionally not stored for doc nodes today —
        //    the file remains the source of truth — so this scan reads the
        //    on-disk content each pass. Capped at 500 docs per run so a
        //    heavy project doesn't stall the request.
        #[allow(clippy::type_complexity)]
        let doc_rows: Vec<(uuid::Uuid, uuid::Uuid, String, String)> = sqlx_core::query_as::query_as(
            "SELECT n.id, n.folder_id, f.abs_path, n.file_path
               FROM sensei.nodes n
               JOIN sensei.folders f ON f.id = n.folder_id
              WHERE f.project_id = $1
                AND n.kind = 'doc'
              LIMIT 500"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let scanned_docs = doc_rows.len();

        // 2. Load known code identifier names into a set once, so the
        //    per-mention lookup is a HashSet::contains (cheap) rather than a DB
        //    round-trip per mention. Two deliberate widenings vs the original
        //    (which over-fired ~all mentions as broken):
        //    - ALL code-symbol kinds, not just 7 — the old whitelist predated
        //      struct/enum/hook/component/extension, so real project symbols of
        //      those kinds were wrongly flagged.
        //    - GLOBAL, not per-project — a doc legitimately references its
        //      indexed dependencies' symbols (e.g. a rokkit component). Those
        //      resolve to a real node in another project, so their mention is
        //      not drift. (Cross-project name collisions can mask a removed
        //      same-named symbol — an accepted precision tradeoff to kill the
        //      dependency-reference false positives.)
        let code_names: Vec<(String,)> = sqlx_core::query_as::query_as(&format!(
            "SELECT DISTINCT name FROM sensei.nodes WHERE kind IN ({kinds})",
            kinds = Self::DRIFT_SYMBOL_KINDS
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let mut known: std::collections::HashSet<String> = code_names.into_iter().map(|(n,)| n).collect();

        // Also treat DB schema identifiers as known: docs legitimately reference
        // table / column / view names and enum labels (`project_id`, `created_at`,
        // `tool_usage_stats`, `assistant_family`), which are real identifiers, not
        // drift — but they are never indexed as code-symbol nodes. Own-schemas
        // only (skip pg_catalog / information_schema noise).
        let schema_names: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT table_name AS name FROM information_schema.tables
              WHERE table_schema IN ('sensei','inference','activity','governance','staging')
             UNION
             SELECT column_name FROM information_schema.columns
              WHERE table_schema IN ('sensei','inference','activity','governance','staging')
             UNION
             SELECT e.enumlabel
               FROM pg_enum e JOIN pg_type t ON t.oid = e.enumtypid
               JOIN pg_namespace ns ON ns.oid = t.typnamespace
              WHERE ns.nspname IN ('sensei','inference','activity','governance')"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        known.extend(schema_names.into_iter().map(|(n,)| n));

        // 2b. Refresh the symbol-name history (monotonic upsert of the current
        //     symbols) so a symbol removed since a prior scan stays "known to have
        //     existed", then load the full history. The drift gate flags a mention
        //     ONLY when it was a real symbol (in `ever_symbols`) and no longer
        //     resolves (`known`) — so identifiers that were never symbols (enum
        //     variants, serde camelCase fields, string-dispatched tool names) are
        //     not drift. This is what removes the ~408 false positives.
        if let Err(e) = self.record_symbol_names().await {
            tracing::warn!(error = %e, "scan_project_doc_drift: record_symbol_names failed — history not refreshed this pass");
        }
        let ever_rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT name FROM sensei.symbol_names"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let ever_symbols: std::collections::HashSet<String> =
            ever_rows.into_iter().map(|(n,)| n).collect();

        // 3. Fan the mentions out per doc, inserting `broken` drift rows for
        //    mentions that were a real symbol and no longer resolve. We check for
        //    an existing broken row via a subquery to avoid duplicates.
        let mut new_broken: i64 = 0;
        for (doc_id, folder_id, abs_path, file_path) in &doc_rows {
            // Read the doc content off disk. Unreadable files (deleted,
            // permission denied) silently skip — we never fail the whole
            // scan for one bad file.
            let full_path = std::path::Path::new(abs_path).join(file_path);
            let content = match std::fs::read_to_string(&full_path) {
                Ok(text) => text,
                Err(_) => continue,
            };
            let mentions = extract_identifier_mentions(&content);
            for mention in mentions {
                // Flag only names that WERE a real symbol and no longer resolve.
                if !is_broken_drift(&mention, &known, &ever_symbols) {
                    continue;
                }
                let detail = format!("Mentions `{mention}` which is not in the code.");
                // Skip if we already logged this same drift signal.
                let existing: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
                    "SELECT id FROM inference.drift_items
                      WHERE doc_node_id = $1 AND detail = $2 AND resolved_at IS NULL
                      LIMIT 1"
                )
                .bind(doc_id)
                .bind(&detail)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
                if existing.is_some() {
                    continue;
                }
                // The `doc_node_id -> code_node_id` invariant requires a code
                // node — the DDL enforces NOT NULL on the FK. Use the doc node
                // as a self-reference so the FK stays satisfied without a
                // dedicated "unresolved" sentinel. Callers rely on
                // `code_node_id` matching `doc_node_id` to mean "broken".
                sqlx_core::query::query(
                    "INSERT INTO inference.drift_items
                        (folder_id, doc_node_id, code_node_id, status, detail)
                     VALUES ($1, $2, $2, 'broken', $3)"
                )
                .bind(folder_id)
                .bind(doc_id)
                .bind(&detail)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
                new_broken += 1;
            }
        }

        // 4. Resolve any existing broken rows whose mention now RESOLVES —
        //    the doc got fixed or the code got added since the last scan.
        //    We re-parse each open row's detail to recover the mention name.
        let open_rows: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT di.id, di.detail
               FROM inference.drift_items di
               JOIN sensei.folders f ON f.id = di.folder_id
              WHERE f.project_id = $1
                AND di.status = 'broken'
                AND di.resolved_at IS NULL"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let mut resolved: i64 = 0;
        for (drift_id, detail) in open_rows {
            // Clear an open row once it's no longer drift: the code came back /
            // the doc was fixed (now in `known`) OR the mention was never a real
            // symbol (absent from history) — the false-positive backlog.
            if let Some(mention) = extract_mention_from_detail(&detail)
                && !is_broken_drift(&mention, &known, &ever_symbols)
            {
                sqlx_core::query::query(
                    "UPDATE inference.drift_items
                        SET status = 'current', resolved_at = now()
                      WHERE id = $1"
                )
                .bind(drift_id)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
                resolved += 1;
            }
        }

        Ok(serde_json::json!({
            "scannedDocs": scanned_docs,
            "newBroken":   new_broken,
            "resolved":    resolved,
        }))
    }

    // ── Service registry + per-project scoping (T2 Slice B) ──────────────

    /// List every installed service, joined with the given project's per-scope
    /// override so the UI can render enabled/disabled state without a second
    /// round-trip. `enabled_for_project` reads from the scoped override when
    /// present, otherwise falls back to the global row's `enabled`, otherwise
    /// defaults to `true` (installed services are on by default).
    pub async fn list_services_with_project_scope(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            uuid::Uuid,     // id
            String,         // name
            String,         // display_name
            Option<String>, // publisher
            String,         // protocol
            String,         // kind
            Option<String>, // summary
            i32,            // tools_count
            bool,           // verified
            bool,           // installed
            Option<bool>,   // scoped_enabled
            Option<bool>,   // global_enabled
        )> = sqlx_core::query_as::query_as(
            "SELECT s.id, s.name, s.display_name, s.publisher,
                    s.protocol::text, s.kind::text, s.summary, s.tools_count,
                    s.verified, s.installed,
                    (SELECT enabled FROM sensei.service_projects sp
                      WHERE sp.service_id = s.id AND sp.project_id = $1) AS scoped_enabled,
                    (SELECT enabled FROM sensei.service_projects sp
                      WHERE sp.service_id = s.id AND sp.project_id IS NULL) AS global_enabled
               FROM sensei.services s
              WHERE s.installed = true
              ORDER BY s.display_name"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, display_name, publisher, protocol, kind, summary,
                                    tools_count, verified, installed, scoped_enabled, global_enabled)| {
            // Effective enable: scoped override wins, then global row, then default true.
            let enabled_for_project = scoped_enabled.or(global_enabled).unwrap_or(true);
            serde_json::json!({
                "id":                 id,
                "name":               name,
                "displayName":        display_name,
                "publisher":          publisher,
                "protocol":           protocol,
                "kind":               kind,
                "summary":            summary,
                "toolsCount":         tools_count,
                "verified":           verified,
                "installed":          installed,
                "enabledForProject":  enabled_for_project,
                "scopedEnabled":      scoped_enabled,
                "globalEnabled":      global_enabled,
            })
        }).collect())
    }

    /// Upsert the per-project scope row for a service. `project_id = None`
    /// writes the global scope. Idempotent — repeat calls flip the enabled
    /// flag and bump `modified_at`.
    pub async fn set_service_project_scope(
        &self,
        service_id: &uuid::Uuid,
        project_id: Option<&uuid::Uuid>,
        enabled: bool,
    ) -> Result<(), String> {
        // Partial-unique indexes on (service_id) WHERE project_id IS NULL and
        // (service_id, project_id) WHERE project_id IS NOT NULL guarantee at
        // most one row per scope, so an UPDATE-first fallback is enough
        // without needing INSERT ... ON CONFLICT (which can't target a
        // partial unique index without extra hints in Postgres).
        let updated = if let Some(pid) = project_id {
            sqlx_core::query::query(
                "UPDATE sensei.service_projects
                    SET enabled = $1, modified_at = now()
                  WHERE service_id = $2 AND project_id = $3"
            )
            .bind(enabled)
            .bind(service_id)
            .bind(pid)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            sqlx_core::query::query(
                "UPDATE sensei.service_projects
                    SET enabled = $1, modified_at = now()
                  WHERE service_id = $2 AND project_id IS NULL"
            )
            .bind(enabled)
            .bind(service_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        };

        if updated.rows_affected() == 0 {
            sqlx_core::query::query(
                "INSERT INTO sensei.service_projects (service_id, project_id, enabled)
                 VALUES ($1, $2, $3)"
            )
            .bind(service_id)
            .bind(project_id)
            .bind(enabled)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Per-tool aggregation for a project's Instruments screen.
    /// Joins `session_tool_calls` back to `activity.sessions` on
    /// `client_session_id`, filters by `session.project_id`, and computes
    /// call count, error count, avg duration, and FTR (fraction of sessions
    /// that used the tool AND completed FTR).
    pub async fn get_project_mcp_tool_stats(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, i64, i64, Option<f64>, Option<f64>, Option<chrono::DateTime<chrono::Utc>>)> =
            sqlx_core::query_as::query_as(
                "WITH scoped AS (
                     SELECT stc.tool_name,
                            stc.success,
                            stc.duration_ms,
                            stc.started_at,
                            s.ftr
                       FROM sensei.session_tool_calls stc
                       JOIN activity.sessions s
                         ON s.client_session_id = stc.session_id
                      WHERE s.project_id = $1
                 )
                 SELECT tool_name,
                        count(*)::bigint                                                          AS calls,
                        count(*) FILTER (WHERE success IS FALSE)::bigint                          AS errors,
                        avg(duration_ms)::float8                                                  AS avg_duration_ms,
                        (count(*) FILTER (WHERE ftr IS TRUE)::float8
                            / NULLIF(count(*) FILTER (WHERE ftr IS NOT NULL), 0))                 AS ftr,
                        max(started_at)                                                           AS last_used_at
                   FROM scoped
                  GROUP BY tool_name
                  ORDER BY calls DESC, tool_name ASC"
            )
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(tool_name, calls, errors, avg_dur, ftr, last_used_at)| {
            serde_json::json!({
                "toolName":      tool_name,
                "calls":         calls,
                "errors":        errors,
                "avgDurationMs": avg_dur,
                "ftr":           ftr,
                "lastUsedAt":    last_used_at.map(|t| t.to_rfc3339()),
            })
        }).collect())
    }

    // ── Manual impact-verdict log (T3 Slice 3) ─────────────────────────────

    /// List manual impact-verdict entries for a project, newest first.
    /// Optional `verdict` filter narrows to one lifecycle stage (`pending`,
    /// `success`, `mixed`, `failure`).
    pub async fn list_impact_verdicts(
        &self,
        project_id: &uuid::Uuid,
        verdict: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, session_id, title, note, verdict::text, created_at, decided_at
                   FROM sensei.impact_verdicts
                  WHERE project_id = $1
                    AND ($2::text IS NULL OR verdict::text = $2)
                  ORDER BY created_at DESC
                  LIMIT 200"
            )
            .bind(project_id)
            .bind(verdict)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, session_id, title, note, verdict, created_at, decided_at)| {
            serde_json::json!({
                "id":         id,
                "sessionId":  session_id,
                "title":      title,
                "note":       note,
                "verdict":    verdict,
                "createdAt":  created_at.to_rfc3339(),
                "decidedAt":  decided_at.map(|t| t.to_rfc3339()),
            })
        }).collect())
    }

    /// Log a new impact entry. Verdict defaults to `pending`; the caller
    /// hits `set_impact_verdict_outcome` later to record the outcome.
    pub async fn create_impact_verdict(
        &self,
        project_id: &uuid::Uuid,
        title: &str,
        note: Option<&str>,
        session_id: Option<&uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        if title.trim().is_empty() {
            return Err("title required".into());
        }
        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.impact_verdicts (project_id, title, note, session_id)
             VALUES ($1, $2, $3, $4) RETURNING id"
        )
        .bind(project_id)
        .bind(title)
        .bind(note)
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// Assign a terminal verdict (`success` | `mixed` | `failure`) to a
    /// pending impact log entry. Stamps `decided_at = now()`. Errors when
    /// the entry doesn't exist or has already been decided.
    pub async fn set_impact_verdict_outcome(
        &self,
        verdict_id: &uuid::Uuid,
        outcome: &str,
        note: Option<&str>,
    ) -> Result<(), String> {
        if !matches!(outcome, "success" | "mixed" | "failure") {
            return Err(format!("invalid verdict {outcome}"));
        }
        let result = sqlx_core::query::query(
            "UPDATE sensei.impact_verdicts
                SET verdict = $1::sensei.impact_verdict,
                    note = COALESCE($2, note),
                    decided_at = now()
              WHERE id = $3
                AND verdict = 'pending'"
        )
        .bind(outcome)
        .bind(note)
        .bind(verdict_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        if result.rows_affected() == 0 {
            return Err("verdict not found or already decided".into());
        }
        Ok(())
    }

    pub async fn get_reasoning_traces_by_project(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Vec<String>, serde_json::Value, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT id, trigger_event, models_used, exchanges, consensus FROM inference.reasoning_traces WHERE project_id = $1"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, trigger, models, exchanges, consensus)| {
            serde_json::json!({ "id": id, "trigger_event": trigger, "models_used": models, "exchanges": exchanges, "consensus": consensus })
        }).collect())
    }

    // ── Folders to Watch ───────────────────────────────────────────────

    pub async fn add_watch_root(&self, path: &str, name: &str, excluded: &serde_json::Value) -> Result<uuid::Uuid, String> {
        // On conflict, PRESERVE the existing `excluded` — exclusions are managed
        // by `update_watch_root` (the roots API), and a re-scan passes `[]`, which
        // must never wipe the user's exclusions. `excluded` here is the seed for
        // the first insert only.
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders_to_watch(path, name, excluded) VALUES($1, $2, $3)
             ON CONFLICT(path) DO UPDATE SET name = EXCLUDED.name, modified_at = now()
             RETURNING id"
        ).bind(path).bind(name).bind(excluded)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_watch_roots(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, serde_json::Value, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, path, name, status::text, excluded, modified_at FROM sensei.folders_to_watch ORDER BY path"
            ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, path, name, status, excluded, modified)| {
            serde_json::json!({ "id": id, "path": path, "name": name, "status": status, "excluded": excluded, "modified_at": modified.to_rfc3339() })
        }).collect())
    }

    /// The watch root that contains `path` — the exact root, or the nearest
    /// ancestor root when `path` sits inside one. Lets `scan_root` reuse an
    /// existing top-level root instead of registering a redundant sub-root (watch
    /// roots stay top-level; a change resolves to its repo via
    /// [`Self::repo_root_for_path`]). `None` when `path` is under no watch root.
    pub async fn enclosing_watch_root(&self, path: &str) -> Result<Option<(uuid::Uuid, String)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT id, path FROM sensei.folders_to_watch
              WHERE $1 = path OR $1 LIKE path || '/%'
              ORDER BY length(path) DESC LIMIT 1"
        ).bind(path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())
    }

    pub async fn update_watch_status(&self, id: &uuid::Uuid, status: &str) -> Result<(), String> {
        sqlx_core::query::query("UPDATE sensei.folders_to_watch SET status = $2::sensei.watch_status, modified_at = now() WHERE id = $1")
            .bind(id).bind(status).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update a watch root's name and/or exclusions. Passing `None` for a
    /// field leaves it unchanged; passing `Some(x)` replaces the current
    /// value with `x`. Path is deliberately not mutable — a rename would
    /// need to remove + re-add (folders_to_watch.path is UNIQUE and the
    /// materialised sensei.folders subtree references it). #41.
    pub async fn update_watch_root(
        &self,
        id: &uuid::Uuid,
        name: Option<&str>,
        excluded: Option<&serde_json::Value>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders_to_watch
                SET name     = COALESCE($2, name),
                    excluded = COALESCE($3, excluded),
                    modified_at = now()
              WHERE id = $1"
        )
            .bind(id).bind(name).bind(excluded)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn remove_watch_root(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Scan State ───────────────────────────────────────────────────

    pub async fn upsert_scan_state(&self, folder_id: &uuid::Uuid, file_path: &str, mtime: i64, content_hash: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.scan_state(folder_id, file_path, mtime, content_hash) VALUES($1, $2, $3, $4)
             ON CONFLICT(folder_id, file_path) DO UPDATE SET mtime = EXCLUDED.mtime, content_hash = EXCLUDED.content_hash, indexed_at = now(), modified_at = now()"
        ).bind(folder_id).bind(file_path).bind(mtime).bind(content_hash)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_stale_files(&self, folder_id: &uuid::Uuid, current_files: &[(String, i64)]) -> Result<Vec<String>, String> {
        // Return files where mtime has changed
        let mut stale = Vec::new();
        for (path, mtime) in current_files {
            let row: Option<(i64,)> = sqlx_core::query_as::query_as(
                "SELECT mtime FROM sensei.scan_state WHERE folder_id = $1 AND file_path = $2"
            ).bind(folder_id).bind(path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
            match row {
                None => stale.push(path.clone()), // new file
                Some((old_mtime,)) if old_mtime != *mtime => stale.push(path.clone()),
                _ => {}
            }
        }
        Ok(stale)
    }

    pub async fn delete_scan_state(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.scan_state WHERE folder_id = $1")
            .bind(folder_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// All scan-state fingerprints for a folder as `(file_path, mtime,
    /// content_hash)`. Loaded once per scan so the indexer can run the two-tier
    /// change-detection (cheap mtime gate → content-hash gate) entirely in
    /// memory instead of N per-file queries. The `content_hash` lets a re-scan
    /// short-circuit a *touched-but-identical* file (mtime drifted, bytes same)
    /// without reindexing it. See [`crate::tasks::handlers::scan_logic::plan_reindex`].
    pub async fn list_scan_state_full(&self, folder_id: &uuid::Uuid) -> Result<Vec<(String, i64, String)>, String> {
        let rows: Vec<(String, i64, String)> = sqlx_core::query_as::query_as(
            "SELECT file_path, mtime, content_hash FROM sensei.scan_state WHERE folder_id = $1"
        ).bind(folder_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// All scan-state fingerprints for a folder as `(file_path, mtime)`. A thin
    /// projection over [`Self::list_scan_state_full`] for callers that only need
    /// the mtime (removed-file diff, tests).
    pub async fn list_scan_state(&self, folder_id: &uuid::Uuid) -> Result<Vec<(String, i64)>, String> {
        Ok(self.list_scan_state_full(folder_id).await?
            .into_iter().map(|(p, m, _)| (p, m)).collect())
    }

    /// Drop a single file's scan-state row (used when a file no longer exists on
    /// disk, e.g. it was deleted or removed by a branch switch).
    pub async fn delete_scan_state_file(&self, folder_id: &uuid::Uuid, file_path: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.scan_state WHERE folder_id = $1 AND file_path = $2")
            .bind(folder_id).bind(file_path).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Services ─────────────────────────────────────────────────────

    pub async fn upsert_service(&self, name: &str, display_name: &str, kind: &str, protocol: &str, config: &serde_json::Value) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.services(name, display_name, kind, protocol, config) VALUES($1, $2, $3::sensei.service_kind, $4::sensei.service_protocol, $5)
             ON CONFLICT(name) DO UPDATE SET display_name = EXCLUDED.display_name, config = EXCLUDED.config, modified_at = now()
             RETURNING id"
        ).bind(name).bind(display_name).bind(kind).bind(protocol).bind(config)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn list_services(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, bool, serde_json::Value)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, display_name, kind::text, protocol::text, installed, config FROM sensei.services ORDER BY name"
            ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, dn, kind, proto, inst, config)| {
            serde_json::json!({ "id": id, "name": name, "display_name": dn, "kind": kind, "protocol": proto, "installed": inst, "config": config })
        }).collect())
    }

    pub async fn delete_service(&self, name: &str) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.services WHERE name = $1")
            .bind(name).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Snapshots (activity) ─────────────────────────────────────────

    pub async fn create_snapshot(
        &self, session_id: &uuid::Uuid, folder_id: &uuid::Uuid, kind: &str,
        progress: &str, next_step: Option<&str>, completed_steps: &[String],
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.snapshots(session_id, folder_id, kind, progress_summary, next_step_hint, completed_steps) VALUES($1, $2, $3::sensei.snapshot_kind, $4, $5, $6) RETURNING id"
        ).bind(session_id).bind(folder_id).bind(kind).bind(progress).bind(next_step).bind(completed_steps)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn get_latest_snapshot(&self, session_id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, Option<String>, Vec<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, kind::text, progress_summary, next_step_hint, completed_steps, created_at FROM activity.snapshots WHERE session_id = $1 ORDER BY created_at DESC LIMIT 1"
            ).bind(session_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id, kind, progress, next, steps, ts)| {
            serde_json::json!({ "id": id, "kind": kind, "progress_summary": progress, "next_step_hint": next, "completed_steps": steps, "created_at": ts.to_rfc3339() })
        }))
    }

    // ── Detected Patterns (inference) ──────────────────────────────────

    /// Upsert a detected pattern at PROJECT scope (#82). `folder_id` is
    /// preserved as an optional locus pointer for file/folder-scoped signals
    /// (churn); it is not part of the uniqueness key. Passing the same
    /// (project_id, name, is_anti_pattern) with a different folder_id
    /// updates the same row and overwrites the locus — that's the desired
    /// merge behaviour when a single file's pattern shows up across sibling
    /// folders inside the project.
    pub async fn upsert_pattern(
        &self, project_id: &uuid::Uuid, folder_id: Option<&uuid::Uuid>,
        name: &str, is_anti: bool,
        confidence: Option<f64>, instances: &serde_json::Value,
    ) -> Result<uuid::Uuid, String> {
        let count = instances.as_array().map(|a| a.len() as i32).unwrap_or(0);
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.detected_patterns(project_id, folder_id, name, is_anti_pattern, confidence, instance_count, instances)
             VALUES($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT(project_id, name, is_anti_pattern) DO UPDATE SET
               folder_id = COALESCE(EXCLUDED.folder_id, detected_patterns.folder_id),
               confidence = COALESCE(EXCLUDED.confidence, detected_patterns.confidence),
               instance_count = EXCLUDED.instance_count,
               instances = EXCLUDED.instances,
               modified_at = now()
             RETURNING id"
        ).bind(project_id).bind(folder_id).bind(name).bind(is_anti).bind(confidence).bind(count).bind(instances)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn promote_pattern(&self, id: &uuid::Uuid, lifecycle: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE inference.detected_patterns SET lifecycle = $2::sensei.pattern_lifecycle, modified_at = now() WHERE id = $1"
        ).bind(id).bind(lifecycle)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn list_patterns_by_folder(&self, folder_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, String, bool, Option<f64>, i32, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, family, lifecycle::text, is_anti_pattern, confidence::float8, instance_count, modified_at
                 FROM inference.detected_patterns WHERE folder_id = $1 ORDER BY instance_count DESC"
            ).bind(folder_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, family, lc, anti, conf, count, modified)| {
            serde_json::json!({
                "id": id, "name": name, "family": family, "lifecycle": lc,
                "is_anti_pattern": anti, "confidence": conf, "instance_count": count,
                "modified_at": modified.to_rfc3339(),
            })
        }).collect())
    }

    /// Patterns a symbol participates in — FILE-level membership. `detected_patterns`
    /// records file `instances`, not per-symbol members, so we match the symbol's
    /// file: resolve nodes named `symbol` in the project's folders, then return the
    /// project's patterns whose `instances[].file` is that node's file. `nodes.file_path`
    /// is repo-RELATIVE and `instances[].file` is ABSOLUTE, so the match is an
    /// equality-or-path-suffix. `[]` when the symbol's file is in no pattern
    /// (honest-empty, NOT the old always-null mask that read a nonexistent `members`).
    pub async fn patterns_for_symbol(
        &self, project_id: &uuid::Uuid, folder_ids: &[uuid::Uuid], symbol: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        if folder_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows: Vec<(uuid::Uuid, String, Option<String>, String, bool, Option<f64>, i32)> =
            sqlx_core::query_as::query_as(
                "SELECT DISTINCT p.id, p.name, p.family, p.lifecycle::text, p.is_anti_pattern, p.confidence::float8, p.instance_count
                   FROM inference.detected_patterns p
                  WHERE p.project_id = $1
                    AND EXISTS (
                        SELECT 1 FROM sensei.nodes n
                        JOIN jsonb_array_elements(p.instances) e
                          ON (e->>'file' = n.file_path OR e->>'file' LIKE '%/' || n.file_path)
                        WHERE n.folder_id = ANY($2::uuid[]) AND n.name = $3 AND n.file_path <> ''
                    )
                  ORDER BY p.instance_count DESC",
            )
            .bind(project_id).bind(folder_ids).bind(symbol)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, family, lc, anti, conf, count)| {
            serde_json::json!({
                "id": id, "name": name, "family": family, "lifecycle": lc,
                "is_anti_pattern": anti, "confidence": conf, "instance_count": count,
            })
        }).collect())
    }

    /// Read a project's detected patterns for the L2 generator: `(id, folder_id,
    /// folder_label, name, is_anti_pattern, instance_count, instances_json_text)`.
    /// `instances` is returned as text (parsed by the caller) to avoid a sqlx
    /// json-feature dependency.
    ///
    /// Attribution matches L1 (`derive_signals`): patterns belong to a project
    /// via the **folders that have sessions for that project** (`sessions.project_id`),
    /// not `folders.project_id` — the two can diverge, and L1 keys off the
    /// session path, so the generator must read the same set.
    pub async fn get_patterns_for_generation(
        &self, project_id: &uuid::Uuid,
    ) -> Result<Vec<(uuid::Uuid, uuid::Uuid, String, String, bool, i32, String)>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, String, String, bool, i32, String)> =
            sqlx_core::query_as::query_as(
                "SELECT dp.id, dp.folder_id, COALESCE(f.name, ''), dp.name, dp.is_anti_pattern, dp.instance_count, dp.instances::text
                   FROM inference.detected_patterns dp
                   JOIN sensei.folders f ON f.id = dp.folder_id
                  WHERE dp.folder_id IN (
                          SELECT DISTINCT folder_id FROM activity.sessions
                           WHERE project_id = $1 AND folder_id IS NOT NULL
                        )
                  ORDER BY dp.instance_count DESC, dp.id"
            ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    // ── Corrections aggregation (#65 step 5) ─────────────────────────────────

    /// All captured user prompts across every project: (project_id, project_name,
    /// session_id, ts_ms, prompt). Ordered by ts so the handler's clustering seeds
    /// on the earliest member. The handler filters to corrections.
    pub async fn get_all_user_prompts(
        &self,
    ) -> Result<Vec<(uuid::Uuid, String, String, i64, String)>, String> {
        let rows: Vec<(uuid::Uuid, String, String, i64, String)> = sqlx_core::query_as::query_as(
            "SELECT s.project_id, COALESCE(p.name, ''), ae.session_id, ae.ts, ae.payload->>'prompt'
               FROM activity.assistant_events ae
               JOIN activity.sessions s ON s.client_session_id = ae.session_id
               JOIN sensei.projects p ON p.id = s.project_id
              WHERE ae.event_type = 'UserPromptSubmit'
                AND ae.payload->>'prompt' IS NOT NULL
                AND s.project_id IS NOT NULL
              ORDER BY ae.ts",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Active memories offered to the corrections summarizer for linking: (id,
    /// title). Bounded; most-recent first.
    pub async fn get_learned_memories_for_matching(
        &self,
    ) -> Result<Vec<(uuid::Uuid, String)>, String> {
        let rows: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT id, title FROM sensei.memories
              WHERE status = 'active'
              ORDER BY created_at DESC
              LIMIT 100",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Upsert one aggregated correction, keyed by its stable `signature` (so `id`
    /// stays constant across re-derivations).
    pub async fn upsert_correction(
        &self,
        row: &crate::corrections::CorrectionRow,
    ) -> Result<uuid::Uuid, String> {
        let r: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO inference.corrections
                (signature, text, suggestion, count, project_ids, last_seen, memory_id, instances, modified_at)
             VALUES($1, $2, $3, $4, $5, $6, $7, $8, now())
             ON CONFLICT(signature) DO UPDATE SET
               text = EXCLUDED.text,
               suggestion = EXCLUDED.suggestion,
               count = EXCLUDED.count,
               project_ids = EXCLUDED.project_ids,
               last_seen = EXCLUDED.last_seen,
               memory_id = EXCLUDED.memory_id,
               instances = EXCLUDED.instances,
               modified_at = now()
             RETURNING id",
        )
        .bind(&row.signature)
        .bind(&row.text)
        .bind(&row.suggestion)
        .bind(row.count)
        .bind(&row.project_ids)
        .bind(row.last_seen)
        .bind(row.memory_id)
        .bind(&row.instances)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(r.0)
    }

    /// Delete corrections whose signature is not in `keep`. With an empty slice
    /// this clears the table (no corrections currently recur). Returns row count.
    pub async fn delete_corrections_not_in(&self, keep: &[String]) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "DELETE FROM inference.corrections WHERE signature <> ALL($1)",
        )
        .bind(keep)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    /// Global corrections list (camelCase, projects resolved to {id, name}).
    pub async fn list_corrections(&self) -> Result<serde_json::Value, String> {
        self.query_corrections(None).await
    }

    /// Corrections touching a specific project.
    pub async fn list_corrections_for_project(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<serde_json::Value, String> {
        self.query_corrections(Some(project_id)).await
    }

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

    pub async fn upsert_library(
        &self, name: &str, ecosystem: &str, version: Option<&str>,
        description: Option<&str>, source_type: Option<&str>, base_url: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.libraries(name, ecosystem, version, description, source_type, base_url)
             VALUES($1, $2::sensei.library_ecosystem, $3, $4, $5::sensei.library_source_type, $6)
             ON CONFLICT(ecosystem, name) DO UPDATE SET
               version = COALESCE(EXCLUDED.version, libraries.version),
               description = COALESCE(EXCLUDED.description, libraries.description),
               source_type = COALESCE(EXCLUDED.source_type, libraries.source_type),
               base_url = COALESCE(EXCLUDED.base_url, libraries.base_url),
               modified_at = now()
             RETURNING id"
        ).bind(name).bind(ecosystem).bind(version).bind(description).bind(source_type).bind(base_url)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Refresh a library's source pointer (`source_type` + `base_url`) BY id — for
    /// a re-index that resolves the row via its uuid rather than by
    /// `(ecosystem, name)`. NEVER changes `ecosystem`: that is half the
    /// `upsert_library` conflict key and the row's identity, and clobbering it is
    /// exactly the phantom-row bug this avoids. `base_url` is COALESCE'd so a
    /// missing value doesn't wipe the stored one.
    pub async fn update_library_source(
        &self, id: &uuid::Uuid, source_type: &str, base_url: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.libraries
                SET source_type = $2::sensei.library_source_type,
                    base_url = COALESCE($3, base_url),
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(id).bind(source_type).bind(base_url)
        .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Library capabilities (workstream D): skills/agents a library provides ──
    // Two writers coexist in one table, keyed by `source` ('manifest' | 'generated').

    /// Manifest-authoritative replace of a library's `source`-scoped capabilities:
    /// delete this library's rows for `source`, then re-insert — so a skill/agent
    /// REMOVED from a manifest disappears on re-ingest. One transaction. Mirrors
    /// [`Self::replace_folder_commands`]. `version_range` is the manifest's applies-to
    /// range (same for all rows). Only entries with a resolved `body` are persisted
    /// (a path/body-less entry is dropped upstream at ingest — no fabrication).
    /// Returns (skills, agents) written.
    pub async fn replace_library_capabilities(
        &self,
        library_id: &uuid::Uuid,
        source: &str,
        version_range: Option<&str>,
        skills: &[crate::libraries::manifest::ProvidedSkill],
        agents: &[crate::libraries::manifest::ProvidedAgent],
    ) -> Result<(u32, u32), String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx_core::query::query("DELETE FROM sensei.library_skills WHERE library_id = $1 AND source = $2")
            .bind(library_id).bind(source).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        sqlx_core::query::query("DELETE FROM sensei.library_agents WHERE library_id = $1 AND source = $2")
            .bind(library_id).bind(source).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        let mut ns = 0u32;
        for s in skills.iter().filter(|s| s.body.is_some()) {
            sqlx_core::query::query(
                "INSERT INTO sensei.library_skills(library_id, name, focus, body, source, source_path, version_range)
                 VALUES($1,$2,$3,$4,$5,$6,$7)
                 ON CONFLICT(library_id, name) DO UPDATE SET
                   focus=EXCLUDED.focus, body=EXCLUDED.body, source=EXCLUDED.source,
                   source_path=EXCLUDED.source_path, version_range=EXCLUDED.version_range, modified_at=now()"
            ).bind(library_id).bind(&s.name).bind(&s.focus).bind(s.body.as_deref()).bind(source).bind(s.path.as_deref()).bind(version_range)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            ns += 1;
        }
        let mut na = 0u32;
        for a in agents.iter().filter(|a| a.body.is_some()) {
            sqlx_core::query::query(
                "INSERT INTO sensei.library_agents(library_id, name, focus, body, source, source_path, version_range)
                 VALUES($1,$2,$3,$4,$5,$6,$7)
                 ON CONFLICT(library_id, name) DO UPDATE SET
                   focus=EXCLUDED.focus, body=EXCLUDED.body, source=EXCLUDED.source,
                   source_path=EXCLUDED.source_path, version_range=EXCLUDED.version_range, modified_at=now()"
            ).bind(library_id).bind(&a.name).bind(&a.focus).bind(a.body.as_deref()).bind(source).bind(a.path.as_deref()).bind(version_range)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            na += 1;
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok((ns, na))
    }

    /// Skills a library provides, by library NAME. Enum-free; errors propagate.
    pub async fn list_library_skills(&self, library: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, Option<String>, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT s.name, s.focus, s.body, s.source, s.version_range
               FROM sensei.library_skills s JOIN sensei.libraries l ON l.id = s.library_id
              WHERE l.name = $1 ORDER BY s.focus"
        ).bind(library).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, focus, body, source, vr)| {
            serde_json::json!({ "name": name, "focus": focus, "body": body, "source": source, "version_range": vr })
        }).collect())
    }

    /// One skill of a library by `focus`. `focus` is NOT unique (uniqueness is on
    /// name), so this takes the most-recent match via `LIMIT 1` — never a multi-row
    /// error. `None` on a genuine miss (handler → 404), `Err` on failure.
    pub async fn get_library_skill(&self, library: &str, focus: &str) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(String, String, Option<String>, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT s.name, s.focus, s.body, s.source, s.version_range
               FROM sensei.library_skills s JOIN sensei.libraries l ON l.id = s.library_id
              WHERE l.name = $1 AND s.focus = $2 ORDER BY s.modified_at DESC LIMIT 1"
        ).bind(library).bind(focus).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(name, focus, body, source, vr)| {
            serde_json::json!({ "name": name, "focus": focus, "body": body, "source": source, "version_range": vr })
        }))
    }

    /// Review agents a library provides, by library NAME.
    pub async fn list_library_agents(&self, library: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, Option<String>, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT a.name, a.focus, a.body, a.source, a.version_range
               FROM sensei.library_agents a JOIN sensei.libraries l ON l.id = a.library_id
              WHERE l.name = $1 ORDER BY a.focus"
        ).bind(library).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, focus, body, source, vr)| {
            serde_json::json!({ "name": name, "focus": focus, "body": body, "source": source, "version_range": vr })
        }).collect())
    }

    /// The library skills/agents to SUGGEST for a project, from the libraries it
    /// depends on — REUSES `project_libraries_resolved` (the same view
    /// [`Self::get_project_libraries`] reads) joined to the capability tables. Backs
    /// the recommender enrichment. Returns `{suggested_skills, suggested_agents}`.
    pub async fn list_project_library_capabilities(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        let skills: Vec<(String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT pl.name, s.name, s.focus
               FROM sensei.project_libraries_resolved pl
               JOIN sensei.library_skills s ON s.library_id = pl.id
              WHERE (pl.scoped_project_id = $1 OR pl.scoped_project_id IS NULL) AND pl.enabled = true
              ORDER BY pl.name, s.focus"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        let agents: Vec<(String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT pl.name, a.name, a.focus
               FROM sensei.project_libraries_resolved pl
               JOIN sensei.library_agents a ON a.library_id = pl.id
              WHERE (pl.scoped_project_id = $1 OR pl.scoped_project_id IS NULL) AND pl.enabled = true
              ORDER BY pl.name, a.focus"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "suggested_skills": skills.into_iter().map(|(lib, name, focus)| serde_json::json!({ "library": lib, "name": name, "focus": focus })).collect::<Vec<_>>(),
            "suggested_agents": agents.into_iter().map(|(lib, name, focus)| serde_json::json!({ "library": lib, "name": name, "focus": focus })).collect::<Vec<_>>(),
        }))
    }

    // ── Library update detection (workstream F, v0) ────────────────────────────

    /// Library pins per project, for the update scheduler: joins referenced_libraries
    /// (the folder's pinned `version_used`) → folders (project) → libraries. Returns
    /// `(library_id, name, ecosystem, local_path, project_id, version_used, base_url,
    /// source_type)`; only rows with a project and a non-empty pin. `base_url` +
    /// `source_type` let the apply arm rebuild the re-index `task.url` fail-closed.
    pub async fn list_library_project_pins(
        &self,
    ) -> Result<Vec<(uuid::Uuid, String, String, Option<String>, uuid::Uuid, String, Option<String>, Option<String>)>, String> {
        let rows = sqlx_core::query_as::query_as(
            "SELECT l.id, l.name, l.ecosystem::text, l.local_path, f.project_id, rl.version_used, l.base_url, l.source_type::text
               FROM sensei.referenced_libraries rl
               JOIN sensei.libraries l ON l.id = rl.library_id
               JOIN sensei.folders f ON f.id = rl.folder_id
              WHERE f.project_id IS NOT NULL AND rl.version_used IS NOT NULL AND rl.version_used <> ''",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Cache the latest-known version + check time for a library in `libraries.props`
    /// (the TTL guard against re-hitting registries every tick). No schema change.
    pub async fn set_library_latest_cache(&self, library_id: &uuid::Uuid, latest: &str, checked_at_unix: i64) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.libraries
                SET props = coalesce(props, '{}'::jsonb)
                          || jsonb_build_object('latest_version', $2::text, 'latest_checked_at', $3::bigint)
              WHERE id = $1",
        )
        .bind(library_id).bind(latest).bind(checked_at_unix)
        .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The cached `(latest_version, latest_checked_at_unix)` from `libraries.props`,
    /// if both are present.
    pub async fn get_library_latest_cache(&self, library_id: &uuid::Uuid) -> Result<Option<(String, i64)>, String> {
        let row: Option<(Option<String>, Option<i64>)> = sqlx_core::query_as::query_as(
            "SELECT props->>'latest_version', (props->>'latest_checked_at')::bigint FROM sensei.libraries WHERE id = $1",
        )
        .bind(library_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.and_then(|(v, t)| match (v, t) {
            (Some(v), Some(t)) => Some((v, t)),
            _ => None,
        }))
    }

    /// Stamp the "docs applied at version" marker in `libraries.props` after a
    /// CONFIRMED, non-empty re-index (F v1 auto-apply). Mirrors
    /// [`Self::set_library_latest_cache`]'s single-statement jsonb merge — no
    /// schema change. Only ever written on success, so it never fabricates
    /// "applied".
    pub async fn set_library_docs_applied(&self, library_id: &uuid::Uuid, version: &str, applied_at_unix: i64) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.libraries
                SET props = coalesce(props, '{}'::jsonb)
                          || jsonb_build_object('docs_applied_version', $2::text, 'docs_applied_at', $3::bigint)
              WHERE id = $1",
        )
        .bind(library_id).bind(version).bind(applied_at_unix)
        .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The `docs_applied_version` marker from `libraries.props`, if present — the
    /// gate that stops the scheduler re-applying an already-applied version.
    pub async fn get_library_docs_applied(&self, library_id: &uuid::Uuid) -> Result<Option<String>, String> {
        let row: Option<(Option<String>,)> = sqlx_core::query_as::query_as(
            "SELECT props->>'docs_applied_version' FROM sensei.libraries WHERE id = $1",
        )
        .bind(library_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.and_then(|(v,)| v))
    }

    /// True if a recommendation already flags this project's update of `library_id`
    /// to `to_version` at the given security tier. `is_security` discriminates the
    /// tier so a prior/dismissed non-security notify can't suppress a later security
    /// flag (and vice-versa). Mirrors [`Self::recommendation_exists_for_pattern`],
    /// keyed on the library payload in `based_on`.
    pub async fn pending_library_update_exists(&self, project_id: &uuid::Uuid, library_id: &uuid::Uuid, to_version: &str, is_security: bool) -> Result<bool, String> {
        // The is_security discriminator: a row's tier is `based_on.is_security`
        // (absent/false = non-security). COALESCE the missing key to false so a
        // legacy notify (no key) reads as non-security, and only a same-tier row
        // matches — a non-security notify can't dedup-suppress a security flag.
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(
               SELECT 1 FROM inference.recommendations
                WHERE project_id = $1 AND action_type = 'library_update'
                  AND based_on->'library_update' @> jsonb_build_object('library_id', $2::text, 'to_version', $3::text)
                  AND COALESCE((based_on->'library_update'->>'is_security')::boolean, false) = $4)",
        )
        .bind(project_id).bind(library_id.to_string()).bind(to_version).bind(is_security)
        .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn get_library(&self, id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, Option<String>, Option<String>, i32, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, ecosystem::text, version, description, page_count, modified_at FROM sensei.libraries WHERE id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(row.map(|(id, name, eco, ver, desc, pages, modified)| {
            serde_json::json!({
                "id": id, "name": name, "ecosystem": eco, "version": ver,
                "description": desc, "page_count": pages, "modified_at": modified.to_rfc3339(),
            })
        }))
    }

    pub async fn list_libraries(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, i32)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, ecosystem::text, version, page_count FROM sensei.libraries ORDER BY name"
            ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, eco, ver, pages)| {
            serde_json::json!({ "id": id, "name": name, "ecosystem": eco, "version": ver, "page_count": pages })
        }).collect())
    }

    /// List libraries joined with their folder usage. Returns one row per
    /// library with `repos` (folder names that reference it) and `repoCount`.
    /// Drives `GET /api/libs` for the setup wizard so the Libraries page can
    /// render ecosystem + version + usage without a second round-trip.
    pub async fn list_libraries_with_usage(
        &self,
        scope_folder_name: Option<&str>,
        scope_project_id: Option<&uuid::Uuid>,
        min_repos: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        // Aggregate by library, joining via referenced_libraries to count and
        // list distinct folder names. The optional scopes filter the *folders*
        // counted (not the library), so a lib appears only if some in-scope
        // folder references it.
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<String>, i32, i64, Vec<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT l.id, l.name, l.ecosystem::text, l.version, l.description, l.page_count,
                        COUNT(DISTINCT rl.folder_id)::bigint AS repo_count,
                        COALESCE(array_agg(DISTINCT f.name ORDER BY f.name), ARRAY[]::text[]) AS repos
                   FROM sensei.libraries l
                   JOIN sensei.referenced_libraries rl ON rl.library_id = l.id
                   JOIN sensei.folders f ON f.id = rl.folder_id
                  WHERE l.kind = 'detected'::sensei.library_kind
                    AND ($1::text     IS NULL OR f.name = $1)
                    AND ($2::uuid     IS NULL OR f.project_id = $2)
                  GROUP BY l.id, l.name, l.ecosystem, l.version, l.description, l.page_count
                 HAVING COUNT(DISTINCT rl.folder_id) >= $3
                  ORDER BY repo_count DESC, l.name"
            )
            .bind(scope_folder_name)
            .bind(scope_project_id)
            .bind(min_repos)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, eco, ver, desc, pages, repo_count, repos)| {
            serde_json::json!({
                "id": id, "name": name, "ecosystem": eco, "version": ver,
                "description": desc, "pageCount": pages,
                "repoCount": repo_count, "repos": repos,
            })
        }).collect())
    }

    pub async fn delete_library(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.libraries WHERE id = $1")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn upsert_library_page(
        &self, library_id: &uuid::Uuid, title: &str, url: Option<&str>,
        local_path: Option<&str>, description: Option<&str>, content: Option<&str>,
        source_type: &str, component: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.library_pages(library_id, title, url, local_path, description, content, source_type, component, fetched_at)
             VALUES($1, $2, $3, $4, $5, $6, $7::sensei.library_source_type, $8, now())
             ON CONFLICT(library_id, title) DO UPDATE SET
               url = COALESCE(EXCLUDED.url, library_pages.url),
               local_path = COALESCE(EXCLUDED.local_path, library_pages.local_path),
               description = COALESCE(EXCLUDED.description, library_pages.description),
               content = COALESCE(EXCLUDED.content, library_pages.content),
               component = COALESCE(EXCLUDED.component, library_pages.component),
               fetched_at = now(), modified_at = now()
             RETURNING id"
        ).bind(library_id).bind(title).bind(url).bind(local_path).bind(description).bind(content).bind(source_type).bind(component)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn update_library_page_count(&self, library_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.libraries SET page_count = (SELECT count(*) FROM sensei.library_pages WHERE library_id = $1), modified_at = now() WHERE id = $1"
        ).bind(library_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Upsert a folder → library edge with optional `version_used` and `props`.
    ///
    /// `props` is merged (`||`) with any existing row's props, so callers can
    /// stack tags across passes without clobbering earlier metadata. Pass
    /// `None` for a props-free upsert.
    ///
    /// Typical `props` shape: `{"local_source": "../actions", "protocol": "link"}`
    /// for a dep declared via `link:` / `workspace:` / `file:` / Cargo `path=`.
    pub async fn upsert_referenced_library(
        &self,
        folder_id: &uuid::Uuid,
        library_id: &uuid::Uuid,
        version: Option<&str>,
        props: Option<serde_json::Value>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.referenced_libraries(folder_id, library_id, version_used, props)
             VALUES($1, $2, $3, COALESCE($4, '{}'::jsonb))
             ON CONFLICT(folder_id, library_id) DO UPDATE SET
               version_used = COALESCE(EXCLUDED.version_used, referenced_libraries.version_used),
               props = referenced_libraries.props || EXCLUDED.props,
               modified_at = now()"
        ).bind(folder_id).bind(library_id).bind(version).bind(props)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Upsert a project → project edge into `sensei.project_dependencies`.
    ///
    /// Called from `extract_deps` when a `link:` / `workspace:` / `file:` /
    /// `path=` dep resolves to a sibling folder that belongs to a DIFFERENT
    /// project than the declaring folder. Idempotent on the composite PK
    /// `(from_project_id, to_project_id, from_folder_id, source_manifest)`.
    pub async fn upsert_project_dependency(
        &self,
        from_project_id: &uuid::Uuid,
        to_project_id: &uuid::Uuid,
        from_folder_id: &uuid::Uuid,
        source_protocol: &str,
        source_manifest: &str,
        resolved_target: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.project_dependencies
                (from_project_id, to_project_id, from_folder_id, source_protocol, source_manifest, resolved_target)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (from_project_id, to_project_id, from_folder_id, source_manifest) DO UPDATE SET
               source_protocol = EXCLUDED.source_protocol,
               resolved_target = EXCLUDED.resolved_target,
               modified_at = now()"
        )
            .bind(from_project_id)
            .bind(to_project_id)
            .bind(from_folder_id)
            .bind(source_protocol)
            .bind(source_manifest)
            .bind(resolved_target)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Roll a folder-level library reference up to a project-level association
    /// (sensei.project_libraries), scoped to `project_id`. `referenced_libraries`
    /// is folder-grained; `project_libraries` is the project↔library M2M the
    /// indexer owns and which `project_libraries_resolved` (the Projects screen)
    /// reads. Idempotent and non-destructive: `ON CONFLICT DO NOTHING` preserves
    /// any user edits to `enabled`/`props` on re-scan.
    pub async fn upsert_project_library(
        &self, library_id: &uuid::Uuid, project_id: &uuid::Uuid,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.project_libraries(library_id, project_id)
             VALUES($1, $2)
             ON CONFLICT (library_id, project_id) WHERE project_id IS NOT NULL DO NOTHING"
        ).bind(library_id).bind(project_id)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Verdict measurement ────────────────────────────────────────────

    /// Recompute FTR deltas for accepted recommendations with pending verdict.
    /// Compares current 14-day FTR against baseline_ftr snapshot at time of acceptance.
    /// Returns number of recommendations updated.
    pub async fn measure_pending_verdicts(&self) -> Result<i64, String> {
        // Per-row measure so we can also compose the MOE consensus JSON
        // the Observatory Impact panel renders. The classification rule
        // is the same ±0.05 FTR band the old bulk UPDATE used; kept in
        // `crate::verdicts::Verdict::from_ftr_delta` so it's testable.
        //
        // Two-phase per rec:
        //   1. UPDATE the rec (verdict / current_ftr / measured_at).
        //   2. Insert or update the linked reasoning_trace's `consensus`
        //      JSON with the synth helper. If the rec has no trace yet,
        //      we mint one with trigger_event = 'verdict_measurement'
        //      and link it back onto the rec.
        //
        // Failures in the reasoning-trace write are logged but don't
        // abort the whole batch — verdict measurement is best-effort by
        // design (the scheduler retries every full-refresh window).
        type Row = (
            uuid::Uuid, Option<uuid::Uuid>, f64, f64,
            Option<Vec<String>>, String,
        );
        let rows: Vec<Row> = sqlx_core::query_as::query_as(
            "WITH current AS (
               SELECT r.id AS rec_id,
                      AVG(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)::float8 AS current_ftr
                 FROM inference.recommendations r
                 JOIN activity.sessions s ON s.project_id = r.project_id
                                         AND s.started_at > r.acted_at
                WHERE r.status = 'accepted'
                  AND r.verdict = 'pending'
                  AND r.acted_at < now() - interval '3 days'
                  AND s.outcome IS NOT NULL
                GROUP BY r.id
                HAVING COUNT(*) >= 3
             )
             SELECT r.id,
                    r.reasoning_trace_id,
                    COALESCE(r.baseline_ftr, 0)::float8,
                    c.current_ftr,
                    t.models_used,
                    r.based_on::text
               FROM inference.recommendations r
               JOIN current c ON c.rec_id = r.id
          LEFT JOIN inference.reasoning_traces t ON t.id = r.reasoning_trace_id"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        if rows.is_empty() { return Ok(0); }

        let mut updated: i64 = 0;
        for (rec_id, trace_id, baseline_ftr, current_ftr, models_used_opt, based_on) in rows {
            let verdict = crate::verdicts::Verdict::from_ftr_delta(current_ftr - baseline_ftr);
            let models_used = models_used_opt.unwrap_or_default();
            let consensus = crate::verdicts::synthesize_reasoning(
                verdict, baseline_ftr, current_ftr, &models_used,
            );

            let upd = sqlx_core::query::query(
                "UPDATE inference.recommendations
                    SET verdict     = $2::sensei.recommendation_verdict,
                        current_ftr = $3,
                        measured_at = now()
                  WHERE id = $1 AND verdict = 'pending'"
            )
            .bind(rec_id)
            .bind(verdict.as_wire())
            .bind(current_ftr)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;

            // The `verdict = 'pending'` guard makes the flip win exactly once, so a
            // concurrent scheduler tick can't measure (or challenge) the same rec
            // twice. `rows_affected == 0` means another tick already claimed it.
            if upd.rows_affected() == 0 { continue; }
            updated += 1;

            // Learning-loop feedback: an accepted rec whose FTR REGRESSED after
            // acceptance discredits the memory that spawned it. Challenge (weaken)
            // that source memory through the existing memory_outcome pipeline — the
            // `memory_outcome_apply` trigger does the strength/status math. This
            // fires at most once per rec (the atomic pending→negative flip above is
            // the transition signal). Non-fatal: a challenge-write failure must not
            // abort verdict measurement.
            if verdict == crate::verdicts::Verdict::Negative
                && let Err(e) = self.challenge_source_memory_for_rec(&rec_id, &based_on).await
            {
                tracing::warn!(error = %e, rec = %rec_id, "measure_pending_verdicts: challenge source memory failed");
            }
            // Positive mirror: an FTR improvement vindicates the source memory —
            // reinforce it (bumps reinforced_count/strength, drives the promotion
            // ladder). Same once-per-rec transition signal; non-fatal.
            if verdict == crate::verdicts::Verdict::Positive
                && let Err(e) = self.reinforce_source_memory_for_rec(&rec_id, &based_on).await
            {
                tracing::warn!(error = %e, rec = %rec_id, "measure_pending_verdicts: reinforce source memory failed");
            }

            match trace_id {
                Some(id) => {
                    if let Err(e) = sqlx_core::query::query(
                        "UPDATE inference.reasoning_traces SET consensus = $2 WHERE id = $1"
                    ).bind(id).bind(&consensus).execute(&self.pool).await {
                        tracing::warn!(error = %e, rec = %rec_id, trace = %id, "measure_pending_verdicts: consensus update failed");
                    }
                }
                None => {
                    match sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
                        "INSERT INTO inference.reasoning_traces
                            (trigger_event, trigger_detail, models_used, consensus)
                         VALUES ($1, $2, $3, $4)
                         RETURNING id"
                    )
                    .bind("verdict_measurement")
                    .bind(serde_json::json!({ "recId": rec_id, "verdict": verdict.as_wire() }))
                    .bind::<Vec<String>>(models_used)
                    .bind(&consensus)
                    .fetch_one(&self.pool).await
                    {
                        Ok((new_trace_id,)) => {
                            if let Err(e) = sqlx_core::query::query(
                                "UPDATE inference.recommendations SET reasoning_trace_id = $2 WHERE id = $1"
                            ).bind(rec_id).bind(new_trace_id).execute(&self.pool).await {
                                tracing::warn!(error = %e, rec = %rec_id, trace = %new_trace_id, "measure_pending_verdicts: relink failed");
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, rec = %rec_id, "measure_pending_verdicts: mint trace failed");
                        }
                    }
                }
            }
        }

        Ok(updated)
    }

    // ── Observatory views ──────────────────────────────────────────────

    /// Daily FTR sparkline. Re-sourced from the daily `ftr` rows in
    /// `sensei.project_metric_daily` (metric='ftr') — the single FTR source of
    /// truth: `ftr_rate` = the stored `value` (num/den), `session_count` =
    /// `props.denominator`. Per-project rows read straight through (one row per
    /// day). The holistic (no project filter) branch POOLS the parts per day —
    /// Σnumerator / Σdenominator — so it stays session-weighted and consistent
    /// with every other rollup, honouring the `project_metrics` invariant that
    /// ratios re-derive from their parts (never an average-of-averages). Response
    /// shape unchanged (`{day, ftr_rate, session_count}`);
    /// `props.correction_count`/`avg_turns` are carried in the store but were
    /// never part of this getter's shape, so they stay unexposed.
    pub async fn get_ftr_daily(&self, project_id: Option<&uuid::Uuid>, days: i32) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(chrono::NaiveDate, Option<f64>, Option<i64>)> = if let Some(pid) = project_id {
            sqlx_core::query_as::query_as(
                "SELECT d.date, d.value::float8, (d.props->>'denominator')::int8
                   FROM sensei.project_metric_daily d
                  WHERE d.metric = 'ftr' AND d.project_id = $1 AND d.date >= (current_date - $2::int)
                  ORDER BY d.date"
            ).bind(pid).bind(days).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
        } else {
            sqlx_core::query_as::query_as(
                "SELECT d.date,
                        (SUM((d.props->>'numerator')::float8) / NULLIF(SUM((d.props->>'denominator')::float8), 0))::float8 AS ftr_rate,
                        SUM((d.props->>'denominator')::int8)::int8 AS session_count
                   FROM sensei.project_metric_daily d
                  WHERE d.metric = 'ftr' AND d.date >= (current_date - $1::int)
                  GROUP BY d.date ORDER BY d.date"
            ).bind(days).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
        };
        Ok(rows.into_iter().map(|(day, ftr, count)| {
            serde_json::json!({ "day": day.to_string(), "ftr_rate": ftr.unwrap_or(0.0), "session_count": count.unwrap_or(0) })
        }).collect())
    }

    pub async fn get_hotspots(&self, project_id: &uuid::Uuid, days: i32) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, i64, i64)> = sqlx_core::query_as::query_as(
            "SELECT folder, file_path, edit_count, correction_count
             FROM sensei.project_hotspots
             WHERE project_id = $1 AND last_event_at >= (now() - ($2::int || ' days')::interval)
             ORDER BY (edit_count + correction_count) DESC LIMIT 20"
        ).bind(project_id).bind(days).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(folder, path, edits, corrections)| {
            serde_json::json!({ "folder": folder, "file_path": path, "edit_count": edits, "correction_count": corrections })
        }).collect())
    }

    pub async fn get_quality_signals(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        let row: Option<(f64, Option<f64>, i64, Option<f64>)> = sqlx_core::query_as::query_as(
            "SELECT ftr_7d::float8, pattern_compliance::float8, open_drift_count, test_pass_rate::float8
             FROM sensei.project_quality_signals WHERE project_id = $1"
        ).bind(project_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(match row {
            Some((ftr, compliance, drift, tests)) => serde_json::json!({
                "ftr_7d": ftr, "pattern_compliance": compliance,
                "open_drift_count": drift, "test_pass_rate": tests
            }),
            None => serde_json::json!({
                "ftr_7d": 0, "pattern_compliance": null, "open_drift_count": 0, "test_pass_rate": null
            }),
        })
    }

    pub async fn get_tool_usage_stats(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, i64, i64, Option<f64>, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT tool_name, call_count, error_count, avg_duration_ms::float8, last_used_at
             FROM sensei.tool_usage_stats ORDER BY call_count DESC LIMIT 50"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, calls, errors, dur, last)| {
            serde_json::json!({ "tool_name": name, "call_count": calls, "error_count": errors,
                                "avg_duration_ms": dur, "last_used_at": last.to_rfc3339() })
        }).collect())
    }

    pub async fn get_library_usage(&self, library_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, Option<uuid::Uuid>, Option<String>, i64)> = sqlx_core::query_as::query_as(
            "SELECT library_name, folder, project_id, version_used, unresolved_import_count
             FROM sensei.library_usage WHERE library_id = $1 ORDER BY folder"
        ).bind(library_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name, folder, pid, ver, imports)| {
            serde_json::json!({ "library_name": name, "folder": folder, "project_id": pid,
                                "version_used": ver, "import_count": imports })
        }).collect())
    }

    pub async fn get_pending_recommendations(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<String>, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT id, urgency::text, title, why, impact, evidence
             FROM inference.recommendations
             WHERE project_id = $1 AND status = 'pending'
             ORDER BY CASE urgency WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, id
             LIMIT 10"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, urgency, title, why, impact, evidence)| {
            serde_json::json!({ "id": id, "urgency": urgency, "title": title,
                                "why": why, "impact": impact, "evidence": evidence })
        }).collect())
    }

    /// Highest-priority pending recommendations across all projects — powers the
    /// Observatory · Today hero + insight strip. Mirrors
    /// [`Self::get_pending_recommendations`] without the project filter.
    pub async fn get_pending_recommendations_global(&self, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<String>, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT id, urgency::text, title, why, impact, evidence
             FROM inference.recommendations
             WHERE status = 'pending'
             ORDER BY CASE urgency WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, id
             LIMIT $1"
        ).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, urgency, title, why, impact, evidence)| {
            serde_json::json!({ "id": id, "urgency": urgency, "title": title,
                                "why": why, "impact": impact, "evidence": evidence })
        }).collect())
    }

    // ── Insights (Learnings Triage) aggregator sources (#Slot 5) ──────────
    // Each carries `project_id` so the UI can call the per-project
    // accept/reject action; `project` is None for the cross-project view.

    /// Pending recommendations + their project name, ordered high→low urgency.
    /// Capped: the triage screen shows the highest-urgency first (Now/Soon are
    /// complete; low-urgency Settled recs beyond the cap fall off the shelf).
    pub async fn get_insights_recommendations(&self, project: Option<&uuid::Uuid>) -> Result<Vec<serde_json::Value>, String> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(uuid::Uuid, String, String, String, Option<String>, serde_json::Value, Option<uuid::Uuid>, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT r.id, r.urgency::text, r.title, r.why, r.impact, r.evidence, r.project_id, p.name
                 FROM inference.recommendations r
                 LEFT JOIN sensei.projects p ON p.id = r.project_id
                 WHERE r.status = 'pending' AND ($1::uuid IS NULL OR r.project_id = $1)
                 ORDER BY CASE r.urgency WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, r.id
                 LIMIT 200"
            ).bind(project).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        // No silent caps: the Insights board shows the top-200 pending recs by
        // urgency; if the cap is hit, lower-urgency recs are not surfaced. Log it
        // so the truncation is observable (a "showing N of M" UI hint is a follow-up).
        if rows.len() >= 200 {
            tracing::warn!(returned = rows.len(),
                "get_insights_recommendations hit the 200-row cap — lower-urgency pending recs are not surfaced on the triage board");
        }
        Ok(rows.into_iter().map(|(id, urgency, title, why, impact, evidence, project_id, name)| {
            serde_json::json!({ "id": id, "urgency": urgency, "title": title, "why": why,
                                "impact": impact, "evidence": evidence,
                                "project_id": project_id, "name": name })
        }).collect())
    }

    /// Memories eligible for the triage screen: proposed, in-force, or violated
    /// (non-archived). Column assignment happens in `crate::insights`.
    pub async fn get_insights_memories(&self, project: Option<&uuid::Uuid>) -> Result<Vec<serde_json::Value>, String> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, i32, Option<f64>, String, Option<uuid::Uuid>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, status::text, title, content, violated_count, strength::float8, scope::text, project_id
                 FROM sensei.memories
                 WHERE ($1::uuid IS NULL OR project_id = $1)
                   AND ( status IN ('proposed','active','reinforced','battle_tested')
                         OR (violated_count > 0 AND status != 'archived') )
                 ORDER BY strength DESC NULLS LAST
                 LIMIT 100"
            ).bind(project).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, status, title, content, violated_count, strength, scope, project_id)| {
            serde_json::json!({ "id": id, "status": status, "title": title, "content": content,
                                "violated_count": violated_count, "strength": strength,
                                "scope": scope, "project_id": project_id })
        }).collect())
    }

    /// Suggested + rule patterns for the triage screen (anti-patterns excluded).
    pub async fn get_insights_patterns(&self, project: Option<&uuid::Uuid>) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, String, i32, Option<uuid::Uuid>)> =
            sqlx_core::query_as::query_as(
                "SELECT dp.id, dp.name, dp.family, dp.lifecycle::text, dp.instance_count, f.project_id
                 FROM inference.detected_patterns dp
                 JOIN sensei.folders f ON f.id = dp.folder_id
                 WHERE dp.lifecycle IN ('suggested','rule') AND NOT dp.is_anti_pattern
                   AND ($1::uuid IS NULL OR f.project_id = $1)
                 ORDER BY dp.instance_count DESC
                 LIMIT 100"
            ).bind(project).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, family, lifecycle, instance_count, project_id)| {
            serde_json::json!({ "id": id, "name": name, "family": family, "lifecycle": lifecycle,
                                "instance_count": instance_count, "project_id": project_id })
        }).collect())
    }

    /// Top recurring corrections by count → the Now column. `project` scopes via
    /// the `project_ids` array membership.
    pub async fn get_insights_corrections(&self, project: Option<&uuid::Uuid>, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, i32)> = sqlx_core::query_as::query_as(
            "SELECT id, text, suggestion, count
             FROM inference.corrections
             WHERE ($1::uuid IS NULL OR $1 = ANY(project_ids))
             ORDER BY count DESC LIMIT $2"
        ).bind(project).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, text, suggestion, count)| {
            serde_json::json!({ "id": id, "text": text, "suggestion": suggestion, "count": count })
        }).collect())
    }

    pub async fn get_adopted_teachings(&self, project_id: &uuid::Uuid, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, i32, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT dp.id, dp.name, dp.family, dp.instance_count, dp.modified_at
             FROM inference.detected_patterns dp
             JOIN sensei.folders f ON f.id = dp.folder_id
             WHERE f.project_id = $1 AND dp.lifecycle = 'rule' AND NOT dp.is_anti_pattern
             ORDER BY dp.modified_at DESC LIMIT $2"
        ).bind(project_id).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, family, count, modified)| {
            serde_json::json!({ "id": id, "name": name, "family": family,
                                "instance_count": count, "modified_at": modified.to_rfc3339() })
        }).collect())
    }

    // ── Sessions (activity) ────────────────────────────────────────────

    pub async fn create_session(&self, folder_id: &uuid::Uuid, task: &str, acp_id: Option<&str>) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.sessions(folder_id, task, acp_id) VALUES($1, $2, $3) RETURNING id"
        ).bind(folder_id).bind(task).bind(acp_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn complete_session(
        &self, id: &uuid::Uuid, outcome: &str, ftr: bool,
        turns: i32, corrections: i32,
        summary: Option<&str>, tokens_in: Option<i32>, tokens_out: Option<i32>,
    ) -> Result<(), String> {
        // summary/tokens are COALESCE'd so a caller that omits them doesn't wipe a
        // previously-set value; these columns exist on activity.sessions and were
        // being silently dropped (the MCP schema advertised them).
        sqlx_core::query::query(
            "UPDATE activity.sessions SET outcome = $2::sensei.session_outcome, ftr = $3, turns = $4, corrections = $5, \
             summary = COALESCE($6, summary), tokens_in = COALESCE($7, tokens_in), tokens_out = COALESCE($8, tokens_out), \
             completed_at = now() WHERE id = $1"
        ).bind(id).bind(outcome).bind(ftr).bind(turns).bind(corrections)
            .bind(summary).bind(tokens_in).bind(tokens_out)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Nearest-ancestor folder for an absolute path: the folder whose `abs_path`
    /// is the path itself or its closest parent. Attributes a hook event (which
    /// carries a `cwd`) to the indexed folder it ran in. `None` when uncovered.
    pub async fn find_folder_for_path(
        &self, path: &str,
    ) -> Result<Option<(uuid::Uuid, Option<uuid::Uuid>)>, String> {
        // Nearest ancestor over current abs_paths AND former paths (aliases), so a
        // hook cwd recorded under an old path (pre-rename) still attributes to the
        // current folder. A live abs_path and an alias of equal length tie-break to
        // the live folder (abs_path row sorts first).
        let row: Option<(uuid::Uuid, Option<uuid::Uuid>)> = sqlx_core::query_as::query_as(
            "SELECT id, project_id FROM (
                 SELECT id, project_id, abs_path AS p, 1 AS live FROM sensei.folders
                 UNION ALL
                 SELECT f.id, f.project_id, a.alias_abs_path AS p, 0 AS live
                   FROM sensei.folder_path_aliases a
                   JOIN sensei.folders f ON f.id = a.folder_id
             ) c
             WHERE $1 = c.p OR $1 LIKE c.p || '/%'
             ORDER BY length(c.p) DESC, c.live DESC
             LIMIT 1"
        ).bind(path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Resolve a filesystem path to its owning INDEXED REPO ROOT — the nearest
    /// `git`/`standalone`/`subtree` ancestor folder — returning `(abs_path,
    /// project_id)`. The file watcher uses this to enqueue incremental tasks
    /// against the correct repo (a change in `~/Dev/kavach/src/x.ts` resolves to
    /// the kavach repo root), exactly as the full scan shapes ProcessFile.
    /// Structural subdirs (workspace_member / folder) are skipped so the one-owner
    /// repo root wins. `None` when the path is under no indexed repo.
    pub async fn repo_root_for_path(
        &self, path: &str,
    ) -> Result<Option<(String, Option<uuid::Uuid>)>, String> {
        let row: Option<(String, Option<uuid::Uuid>)> = sqlx_core::query_as::query_as(
            "SELECT abs_path, project_id FROM sensei.folders
              WHERE kind IN ('git','standalone','subtree')
                AND ($1 = abs_path OR $1 LIKE abs_path || '/%')
              ORDER BY length(abs_path) DESC
              LIMIT 1"
        ).bind(path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Find-or-create the `activity.sessions` row for an assistant
    /// `client_session_id`, attributing it to `folder_id`/`project_id`. Marks it
    /// completed when `is_end` (Stop / SessionEnd). Idempotent per
    /// client_session_id so every hook event of a session folds into one row (#31).
    pub async fn record_session_event(
        &self, client_session_id: &str, folder_id: &uuid::Uuid,
        project_id: Option<&uuid::Uuid>, family: &str, is_end: bool,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.sessions (client_session_id, folder_id, project_id, acp_id, completed_at)
             VALUES ($1, $2, $3, $4, CASE WHEN $5 THEN now() ELSE NULL END)
             ON CONFLICT (client_session_id) WHERE client_session_id IS NOT NULL
             DO UPDATE SET
               completed_at = CASE WHEN $5 THEN now() ELSE activity.sessions.completed_at END,
               project_id   = COALESCE(activity.sessions.project_id, EXCLUDED.project_id)
             RETURNING id"
        ).bind(client_session_id).bind(folder_id).bind(project_id).bind(family).bind(is_end)
         .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Resolve `activity.sessions.id` (observatory UUID) → `client_session_id`
    /// (the string the assistant's hook writer stamps on every
    /// `activity.assistant_events` row). The Replay endpoint (#84 Slice C)
    /// needs this because `assistant_events.session_id` is the client id,
    /// not the UUID.
    pub async fn get_session_client_id(&self, id: &uuid::Uuid) -> Result<Option<String>, String> {
        let row: Option<(Option<String>,)> = sqlx_core::query_as::query_as(
            "SELECT client_session_id FROM activity.sessions WHERE id = $1"
        ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.and_then(|(c,)| c))
    }

    pub async fn get_session(&self, id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, uuid::Uuid, String, Option<String>, Option<String>, Option<bool>, i32, i32, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, folder_id, task, acp_id, outcome::text, ftr, turns, corrections, started_at, completed_at FROM activity.sessions WHERE id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(row.map(|(id, fid, task, acp, outcome, ftr, turns, corr, started, completed)| {
            serde_json::json!({
                "id": id, "folder_id": fid, "task": task, "acp_id": acp,
                "outcome": outcome, "ftr": ftr, "turns": turns, "corrections": corr,
                "started_at": started.to_rfc3339(),
                "completed_at": completed.map(|t| t.to_rfc3339()),
            })
        }))
    }

    pub async fn list_sessions_by_folder(&self, folder_id: &uuid::Uuid, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, Option<bool>, i32, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, task, outcome::text, ftr, corrections, started_at FROM activity.sessions WHERE folder_id = $1 ORDER BY started_at DESC LIMIT $2"
            ).bind(folder_id).bind(limit).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, task, outcome, ftr, corr, started)| {
            serde_json::json!({ "id": id, "task": task, "outcome": outcome, "ftr": ftr, "corrections": corr, "started_at": started.to_rfc3339() })
        }).collect())
    }

    // ── Assistant events ───────────────────────────────────────────────

    /// Insert a hook event payload into activity.assistant_events.
    /// session_id is the assistant's string session ID (not a DB UUID).
    /// assistant_family identifies the source (claude, cursor, zed, …); defaults to 'claude'.
    pub async fn insert_hook_event(
        &self,
        session_id: &str,
        assistant_family: &str,
        event_type: &str,
        tool_name: Option<&str>,
        cwd: Option<&str>,
        ts: i64,
        success: Option<bool>,
        payload: &serde_json::Value,
    ) -> Result<i64, String> {
        let row: (i64,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.assistant_events \
             (session_id, family, event_type, tool_name, cwd, ts, success, payload) \
             VALUES($1, $2::sensei.assistant_family, $3, $4, $5, $6, $7, $8) RETURNING id"
        )
        .bind(session_id)
        .bind(assistant_family)
        .bind(event_type)
        .bind(tool_name)
        .bind(cwd)
        .bind(ts)
        .bind(success)
        .bind(payload)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Insert a hook event only if an identical one isn't already stored, and
    /// return the new id (`None` when it was a duplicate). Used by the capture
    /// drain ([`crate::tasks::capture_drain`]) to import dead-lettered events
    /// without twinning a row the daemon already committed in the rare
    /// "curl timed out after the insert succeeded" race. Dedup is on the payload
    /// (identical on both the live POST and the fallback line) so it holds even
    /// though the two paths stamp `ts` independently.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_hook_event_if_absent(
        &self,
        session_id: &str,
        assistant_family: &str,
        event_type: &str,
        tool_name: Option<&str>,
        cwd: Option<&str>,
        ts: i64,
        success: Option<bool>,
        payload: &serde_json::Value,
    ) -> Result<Option<i64>, String> {
        let row: Option<(i64,)> = sqlx_core::query_as::query_as(
            "INSERT INTO activity.assistant_events \
             (session_id, family, event_type, tool_name, cwd, ts, success, payload) \
             SELECT $1, $2::sensei.assistant_family, $3, $4, $5, $6, $7, $8 \
             WHERE NOT EXISTS ( \
               SELECT 1 FROM activity.assistant_events \
               WHERE session_id = $1 AND event_type = $3 \
                 AND tool_name IS NOT DISTINCT FROM $4 AND payload = $8 \
             ) RETURNING id",
        )
        .bind(session_id)
        .bind(assistant_family)
        .bind(event_type)
        .bind(tool_name)
        .bind(cwd)
        .bind(ts)
        .bind(success)
        .bind(payload)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    /// Newest hook_event timestamp (epoch ms) for an assistant family, or None
    /// when the daemon has never recorded one for it. `assistant_family` is a
    /// Postgres enum, so bind with the explicit cast.
    pub async fn latest_hook_event_ts(&self, family: &str) -> Result<Option<i64>, String> {
        let row: (Option<i64>,) = sqlx_core::query_as::query_as(
            "SELECT max(ts) FROM activity.assistant_events WHERE family = $1::sensei.assistant_family"
        )
        .bind(family)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// All assistant events for one session (by its string `session_id`),
    /// oldest-first, projected to the fields session enrichment reads (#66).
    pub async fn get_hook_events_for_session(&self, client_session_id: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, Option<String>, i64, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT event_type, tool_name, ts, payload FROM activity.assistant_events
             WHERE session_id = $1 ORDER BY ts"
        ).bind(client_session_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(event_type, tool_name, ts, payload)| {
            serde_json::json!({ "event_type": event_type, "tool_name": tool_name, "ts": ts, "payload": payload })
        }).collect())
    }

    /// Same as [`get_hook_events_for_session`] but also returns the DB row id
    /// so the verdict classifier (#90) can reference each `PostToolUse` by
    /// its `activity.assistant_events.id`.
    pub async fn get_hook_events_for_session_with_id(
        &self,
        client_session_id: &str,
    ) -> Result<Vec<(i64, String, Option<String>, i64, serde_json::Value)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT id, event_type, tool_name, ts, payload FROM activity.assistant_events
             WHERE session_id = $1 ORDER BY ts"
        ).bind(client_session_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }

    /// The most recent `TodoWrite` event for a session: its `(payload, cwd)`.
    /// `None` when the session has no `TodoWrite` yet. Feeds the relay
    /// segment-publish path (P2) — `payload` holds the todo list
    /// (`payload.tool_input.todos`, projected by [`crate::dojo::relay_project`]);
    /// `cwd` names the working folder for the run title. Reads the jsonb column
    /// straight into `serde_json::Value` (same pattern as
    /// [`Self::get_hook_events_for_session`]).
    pub async fn latest_todowrite(
        &self,
        session_id: &str,
    ) -> Result<Option<(serde_json::Value, Option<String>)>, String> {
        let row: Option<(serde_json::Value, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT payload, cwd FROM activity.assistant_events
             WHERE session_id = $1 AND tool_name = 'TodoWrite' ORDER BY ts DESC LIMIT 1"
        ).bind(session_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Upsert a batch of tool-call verdicts (#90). Idempotent: repeated calls
    /// with a new heuristic just refresh the row (`ON CONFLICT (event_id) DO
    /// UPDATE`). Returns the number of rows written.
    pub async fn upsert_verdicts_batch(
        &self,
        rows: &[(String, i64, Option<String>, &'static str, f32, String)],
    ) -> Result<usize, String> {
        if rows.is_empty() { return Ok(0); }
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        for (session_id, event_id, tool_name, verdict, confidence, reason) in rows {
            sqlx_core::query::query(
                "INSERT INTO sensei.tool_call_verdicts \
                    (session_id, event_id, tool_name, verdict, confidence, reason, classified_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, now()) \
                 ON CONFLICT (event_id) DO UPDATE SET \
                    tool_name = EXCLUDED.tool_name, \
                    verdict = EXCLUDED.verdict, \
                    confidence = EXCLUDED.confidence, \
                    reason = EXCLUDED.reason, \
                    classified_at = now()"
            )
            .bind(session_id)
            .bind(event_id)
            .bind(tool_name)
            .bind(*verdict)
            .bind(*confidence)
            .bind(reason)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(rows.len())
    }

    /// Distinct session ids that still need verdict classification: sessions
    /// with a `PostToolUse` event inside the window that have no rows in
    /// `sensei.tool_call_verdicts` yet. Feeds the scheduled classifier
    /// (`ClassifyPendingVerdicts`) so the Health-tab aggregate reflects every
    /// session, not just the ones whose Replay tab was opened.
    ///
    /// Unclassified-only is a cheap gap-fill: it bounds the per-tick cost so we
    /// don't re-classify the whole corpus each scheduler tick. Correctness for
    /// anything already classified is covered by `upsert_verdicts_batch`'s
    /// idempotent upsert.
    ///
    /// `assistant_events.ts` is epoch millis (bigint), so the window cutoff is
    /// computed in millis — mirrors `get_tools_health`'s 14-day `PostToolUse`
    /// window; the parametrised-days form mirrors `get_verdict_split_per_tool`.
    /// Session-less events (`session_id = ''`) are excluded — they'd otherwise
    /// collapse every unattached event into one pseudo-session.
    pub async fn unclassified_verdict_sessions(&self, window_days: i32) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT h.session_id
               FROM activity.assistant_events h
              WHERE h.event_type = 'PostToolUse'
                AND h.session_id <> ''
                AND h.ts >= (extract(epoch from now() - ($1::int || ' days')::interval) * 1000)::bigint
                AND NOT EXISTS (
                    SELECT 1 FROM sensei.tool_call_verdicts v WHERE v.session_id = h.session_id
                )"
        )
        .bind(window_days)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(s,)| s).collect())
    }

    // ── #84 Track 2 Slice B — MCP tool manifest cache ─────────────────────

    /// Read the cached tool manifest for a server. `None` when nothing has
    /// been probed yet.
    pub async fn get_mcp_tool_manifest(
        &self,
        server_id: &uuid::Uuid,
    ) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, serde_json::Value, i32, chrono::DateTime<chrono::Utc>, i32, Option<String>, Option<String>, Option<String>, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, tools, tool_count, probed_at, ttl_seconds, error,
                        protocol_version, server_name, server_version
                   FROM sensei.mcp_tool_manifests
                  WHERE server_id = $1"
            )
            .bind(server_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(row.map(|(id, tools, tool_count, probed_at, ttl, error, pv, sn, sv)| serde_json::json!({
            "id":                id,
            "server_id":         server_id,
            "tools":             tools,
            "tool_count":        tool_count,
            "probed_at":         probed_at.to_rfc3339(),
            "ttl_seconds":       ttl,
            "error":             error,
            "protocol_version":  pv,
            "server_name":       sn,
            "server_version":    sv,
            "age_seconds":       (chrono::Utc::now() - probed_at).num_seconds(),
        })))
    }

    /// Upsert a probed manifest. Uses `server_id UNIQUE` on the table so a
    /// re-probe overwrites in place.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_mcp_tool_manifest(
        &self,
        server_id: &uuid::Uuid,
        tools: &serde_json::Value,
        tool_count: i32,
        protocol_version: Option<&str>,
        server_name: Option<&str>,
        server_version: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.mcp_tool_manifests
                (server_id, tools, tool_count, probed_at, protocol_version, server_name, server_version, error)
             VALUES ($1, $2, $3, now(), $4, $5, $6, $7)
             ON CONFLICT (server_id) DO UPDATE SET
                tools            = EXCLUDED.tools,
                tool_count       = EXCLUDED.tool_count,
                probed_at        = now(),
                protocol_version = EXCLUDED.protocol_version,
                server_name      = EXCLUDED.server_name,
                server_version   = EXCLUDED.server_version,
                error            = EXCLUDED.error"
        )
        .bind(server_id).bind(tools).bind(tool_count)
        .bind(protocol_version).bind(server_name).bind(server_version).bind(error)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Full server row for the probe orchestrator — command, args, env,
    /// enabled state — keyed by id.
    pub async fn get_mcp_server_by_id(
        &self,
        id: &uuid::Uuid,
    ) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, String, Option<uuid::Uuid>, String, String, serde_json::Value, serde_json::Value, bool, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, acp_family, mcp_key, scope, project_id, config_source, command, args, env, enabled, connection_state
                   FROM sensei.mcp_servers WHERE id = $1"
            )
            .bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(row.map(|(id, family, key, scope, pid, source, cmd, args, env, enabled, state)| serde_json::json!({
            "id": id, "acp_family": family, "mcp_key": key, "scope": scope,
            "project_id": pid, "config_source": source, "command": cmd,
            "args": args, "env": env, "enabled": enabled, "connection_state": state,
        })))
    }

    // ── #84 Track 2 Slice D — Health tab per-tool verdict split ───────────

    /// Per-tool verdict counts (`used` / `partial` / `ignored`) over the
    /// last N days. Feeds the Health tab's "usage split %" via
    /// `aggregate_tool_insights` (#84 T2 Slice D). Zero-row tools that
    /// still appear in `tool_usage_stats` land with all-zero counts on the
    /// caller side; this method returns only tools that have at least one
    /// classified verdict in the window.
    pub async fn get_verdict_split_per_tool(
        &self,
        days: i32,
    ) -> Result<Vec<(String, i64, i64, i64)>, String> {
        let rows: Vec<(String, i64, i64, i64)> = sqlx_core::query_as::query_as(
            "SELECT COALESCE(tool_name, '') AS tool_name,
                    count(*) FILTER (WHERE verdict = 'used')::bigint    AS used,
                    count(*) FILTER (WHERE verdict = 'partial')::bigint AS partial,
                    count(*) FILTER (WHERE verdict = 'ignored')::bigint AS ignored
               FROM sensei.tool_call_verdicts
              WHERE classified_at > now() - ($1::int || ' days')::interval
              GROUP BY tool_name
              HAVING count(*) > 0"
        )
        .bind(days)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    // ── #83 T1 commands surface — project_commands writer + reader ────────

    /// Replace the set of discovered commands for a folder. Delete + insert
    /// in one transaction so a fresh scan atomically supersedes whatever
    /// was there before. Returns the number of rows inserted.
    pub async fn replace_folder_commands(
        &self,
        folder_id: &uuid::Uuid,
        ecosystem: &str,
        source_file: Option<&str>,
        commands: &[(String, String, Option<&str>)],
    ) -> Result<usize, String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        sqlx_core::query::query(
            "DELETE FROM sensei.project_commands WHERE folder_id = $1 AND ecosystem = $2"
        ).bind(folder_id).bind(ecosystem).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        for (raw_name, command_line, category) in commands {
            sqlx_core::query::query(
                "INSERT INTO sensei.project_commands
                    (folder_id, raw_name, command_line, category, ecosystem, source_file)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (folder_id, raw_name) DO UPDATE SET
                    command_line  = EXCLUDED.command_line,
                    category      = EXCLUDED.category,
                    ecosystem     = EXCLUDED.ecosystem,
                    source_file   = EXCLUDED.source_file,
                    discovered_at = now()"
            )
            .bind(folder_id).bind(raw_name).bind(command_line).bind(category).bind(ecosystem).bind(source_file)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(commands.len())
    }

    /// All commands for a project — union across its folders. `category`
    /// filter is applied server-side so callers can ask for just `test` or
    /// `build` without pulling everything. Ordered by category (nulls last)
    /// then raw_name for stable UI display.
    pub async fn get_project_commands(
        &self,
        project_id: &uuid::Uuid,
        category: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(i64, uuid::Uuid, String, String, String, Option<String>, String, Option<String>, chrono::DateTime<chrono::Utc>)> =
            if let Some(cat) = category {
                sqlx_core::query_as::query_as(
                    "SELECT c.id, c.folder_id, f.name, c.raw_name, c.command_line, c.category, c.ecosystem, c.source_file, c.discovered_at
                       FROM sensei.project_commands c
                       JOIN sensei.folders f ON f.id = c.folder_id
                      WHERE f.project_id = $1 AND c.category = $2
                      ORDER BY c.category NULLS LAST, c.raw_name"
                ).bind(project_id).bind(cat).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
            } else {
                sqlx_core::query_as::query_as(
                    "SELECT c.id, c.folder_id, f.name, c.raw_name, c.command_line, c.category, c.ecosystem, c.source_file, c.discovered_at
                       FROM sensei.project_commands c
                       JOIN sensei.folders f ON f.id = c.folder_id
                      WHERE f.project_id = $1
                      ORDER BY c.category NULLS LAST, c.raw_name"
                ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
            };

        // G10 command bias: mark the user's preferred tool per capability.
        let prefs = self.command_preferences("user").await?;
        let mut out = rows.into_iter().map(|(id, folder_id, folder_name, raw_name, command_line, category, ecosystem, source_file, discovered_at)| {
            serde_json::json!({
                "id":            id,
                "folder_id":     folder_id,
                "folder_name":   folder_name,
                "raw_name":      raw_name,
                "command_line":  command_line,
                "category":      category,
                "ecosystem":     ecosystem,
                "source_file":   source_file,
                "discovered_at": discovered_at.to_rfc3339(),
                "preferred":     crate::adapters::manifest::command_matches_preference(
                                     category.as_deref(), &raw_name, &command_line, &prefs),
            })
        }).collect::<Vec<_>>();

        // G10: rank the preferred tool first within each category (stable), so a
        // caller that takes "the test command" gets the biased one. NULL category
        // sorts last (matching the SQL `NULLS LAST`).
        out.sort_by(|a, b| {
            let key = |v: &serde_json::Value| {
                let c = v["category"].as_str();
                (c.is_none(), c.unwrap_or("").to_string())
            };
            key(a).cmp(&key(b))
                .then_with(|| b["preferred"].as_bool().unwrap_or(false)
                              .cmp(&a["preferred"].as_bool().unwrap_or(false)))
                .then_with(|| a["raw_name"].as_str().unwrap_or("")
                              .cmp(b["raw_name"].as_str().unwrap_or("")))
        });
        Ok(out)
    }

    /// User/dojo capability→preferred-tool preferences for a scope, as a
    /// capability→token map. Backs the `get_commands` bias (G10). Fail-open: an
    /// error yields an empty map (no bias) rather than failing the command read.
    pub async fn command_preferences(&self, scope: &str) -> Result<std::collections::HashMap<String, String>, String> {
        // Fail closed: a DB error must not read as an empty preference map — that
        // would silently ignore the user's real tool bias and fall back to
        // defaults (a governance fail-open). See the #109 audit.
        let rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT capability, preferred FROM sensei.dojo_preferences WHERE scope = $1",
        )
        .bind(scope)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("command_preferences: {e}"))?;
        Ok(rows.into_iter().collect())
    }

    /// Upsert a capability→preferred-tool bias for a scope (`user` today; a Dōjō
    /// can later set org/team scopes that override it). One row per (scope,
    /// capability).
    pub async fn upsert_command_preference(
        &self, scope: &str, capability: &str, preferred: &str, note: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.dojo_preferences (scope, capability, preferred, note, updated_at)
             VALUES ($1, $2, $3, $4, now())
             ON CONFLICT (scope, capability) DO UPDATE
               SET preferred = EXCLUDED.preferred, note = EXCLUDED.note, updated_at = now()",
        )
        .bind(scope).bind(capability).bind(preferred).bind(note)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("upsert_command_preference: {e}"))?;
        Ok(())
    }

    // ── #84 Track 2 Slice C — Replay tab session timeline ─────────────────

    /// Session timeline for the Replay tab (#84 T2 Slice C). Same
    /// paired-call shape as [`get_session_tool_calls`], but also joins
    /// `sensei.tool_call_verdicts` (#90) on the underlying PostToolUse
    /// event id so each row carries the usage verdict.
    ///
    /// The existing view [`sensei.session_tool_calls`] keys on the
    /// PreToolUse event id (call_id); verdicts are keyed on the
    /// PostToolUse event id. This query recomputes both directly against
    /// `activity.assistant_events` so we can LEFT JOIN verdicts on the
    /// PostToolUse id without changing either the view or the verdicts
    /// table.
    pub async fn get_session_replay_timeline(
        &self,
        session_id: &str,
        limit: i32,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(
            i64,                                              // pre_id (call_id)
            Option<i64>,                                      // post_id
            String,                                           // tool_name
            String,                                           // family
            serde_json::Value,                                // request
            Option<serde_json::Value>,                        // response
            Option<bool>,                                     // success
            i64,                                              // pre_ts
            Option<i64>,                                      // post_ts
            Option<i64>,                                      // duration_ms
            Option<String>,                                   // verdict
            Option<f32>,                                      // confidence
            Option<String>,                                   // reason
        )> = sqlx_core::query_as::query_as(
            "WITH pre AS (
                SELECT session_id, family::text AS family, tool_name,
                       id AS pre_id, ts AS pre_ts, payload AS request,
                       row_number() OVER (
                           PARTITION BY session_id, tool_name
                           ORDER BY ts, id
                       ) AS seq
                  FROM activity.assistant_events
                 WHERE event_type = 'PreToolUse'
                   AND tool_name IS NOT NULL
                   AND session_id = $1
            ),
            post AS (
                SELECT session_id, tool_name,
                       id AS post_id, ts AS post_ts,
                       payload AS response, success,
                       row_number() OVER (
                           PARTITION BY session_id, tool_name
                           ORDER BY ts, id
                       ) AS seq
                  FROM activity.assistant_events
                 WHERE event_type = 'PostToolUse'
                   AND tool_name IS NOT NULL
                   AND session_id = $1
            )
            SELECT pre.pre_id, post.post_id, pre.tool_name, pre.family,
                   pre.request, post.response, post.success,
                   pre.pre_ts, post.post_ts,
                   CASE WHEN post.post_ts IS NULL THEN NULL
                        ELSE GREATEST(post.post_ts - pre.pre_ts, 0)
                   END AS duration_ms,
                   v.verdict, v.confidence, v.reason
              FROM pre
              LEFT JOIN post ON pre.session_id = post.session_id
                            AND pre.tool_name  = post.tool_name
                            AND pre.seq        = post.seq
              LEFT JOIN sensei.tool_call_verdicts v ON v.event_id = post.post_id
             ORDER BY pre.pre_ts ASC
             LIMIT $2"
        )
        .bind(session_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(
            pre_id, post_id, tool_name, family, request, response, success,
            pre_ts, post_ts, duration_ms, verdict, confidence, reason,
        )| {
            serde_json::json!({
                "callId":         pre_id,
                "postEventId":    post_id,
                "toolName":       tool_name,
                "family":         family,
                "request":        request,
                "response":       response,
                "success":        success,
                "startedAtMs":    pre_ts,
                "completedAtMs":  post_ts,
                "durationMs":     duration_ms,
                "inFlight":       post_ts.is_none(),
                "verdict":        verdict,    // null when unclassified
                "confidence":     confidence,
                "verdictReason":  reason,
            })
        }).collect())
    }

    // ── #84 Track 2 Slice A — mcp_servers ─────────────────────────────────

    /// Upsert a discovered MCP server row (#84). The uniqueness key is
    /// `(acp_family, mcp_key, scope, project_id)`; existing rows have
    /// `command`/`args`/`env`/`config_source`/`last_seen_at` refreshed, but
    /// `enabled` is preserved (a user's manual toggle survives a re-scan).
    ///
    /// Args:
    /// - `acp_family`  — 'claude' | 'zed' | 'cursor' | 'codex' | 'opencode' | 'other'
    /// - `mcp_key`     — key in the ACP config, e.g. 'sensei', 'postgres'
    /// - `project_id`  — Some(uuid) for project-scope, None for user-scope
    /// - `config_source` — absolute path where discovered
    /// - `command`     — the mcp entry's `command`
    /// - `args`        — JSON array of args (from the config)
    /// - `env`         — JSON object of env vars (from the config)
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_mcp_server(
        &self,
        acp_family: &str,
        mcp_key: &str,
        project_id: Option<uuid::Uuid>,
        config_source: &str,
        command: &str,
        args: &serde_json::Value,
        env: &serde_json::Value,
    ) -> Result<uuid::Uuid, String> {
        let scope = if project_id.is_some() { "project" } else { "user" };
        // Partial unique indexes mean the ON CONFLICT target differs for
        // user vs project scope; the cleanest cross-cutting pattern is
        // "try INSERT, on failure UPDATE by lookup". Use a plain lookup +
        // conditional insert inside a transaction so a concurrent scan
        // can't race us into a duplicate.
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        let existing: Option<(uuid::Uuid,)> = if let Some(pid) = project_id {
            sqlx_core::query_as::query_as(
                "SELECT id FROM sensei.mcp_servers
                  WHERE acp_family = $1 AND mcp_key = $2
                    AND scope = 'project' AND project_id = $3"
            ).bind(acp_family).bind(mcp_key).bind(pid)
            .fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?
        } else {
            sqlx_core::query_as::query_as(
                "SELECT id FROM sensei.mcp_servers
                  WHERE acp_family = $1 AND mcp_key = $2
                    AND scope = 'user'"
            ).bind(acp_family).bind(mcp_key)
            .fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?
        };

        let id = if let Some((existing_id,)) = existing {
            sqlx_core::query::query(
                "UPDATE sensei.mcp_servers
                    SET config_source = $2,
                        command       = $3,
                        args          = $4,
                        env           = $5,
                        last_seen_at  = now()
                  WHERE id = $1"
            )
            .bind(existing_id).bind(config_source).bind(command).bind(args).bind(env)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            existing_id
        } else {
            let (new_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
                "INSERT INTO sensei.mcp_servers
                    (acp_family, mcp_key, scope, project_id, config_source, command, args, env)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 RETURNING id"
            )
            .bind(acp_family).bind(mcp_key).bind(scope).bind(project_id)
            .bind(config_source).bind(command).bind(args).bind(env)
            .fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;
            new_id
        };

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// List MCP servers. `project_id = None` returns user-scope rows; a
    /// concrete project returns the union of user-scope + that project's
    /// project-scope rows (the Instruments Playground shows both). Ordered
    /// by family, then key.
    pub async fn list_mcp_servers(
        &self,
        project_id: Option<uuid::Uuid>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<uuid::Uuid>, String, String, serde_json::Value, serde_json::Value, bool, String, Option<String>, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> =
            if let Some(pid) = project_id {
                sqlx_core::query_as::query_as(
                    "SELECT id, acp_family, mcp_key, scope, project_id, config_source, command, args, env, enabled, connection_state, last_error, last_seen_at, discovered_at
                       FROM sensei.mcp_servers
                      WHERE scope = 'user' OR project_id = $1
                      ORDER BY acp_family, mcp_key"
                ).bind(pid).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
            } else {
                sqlx_core::query_as::query_as(
                    "SELECT id, acp_family, mcp_key, scope, project_id, config_source, command, args, env, enabled, connection_state, last_error, last_seen_at, discovered_at
                       FROM sensei.mcp_servers
                      WHERE scope = 'user'
                      ORDER BY acp_family, mcp_key"
                ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?
            };

        Ok(rows.into_iter().map(|r| serde_json::json!({
            "id": r.0, "acp_family": r.1, "mcp_key": r.2, "scope": r.3,
            "project_id": r.4, "config_source": r.5, "command": r.6,
            "args": r.7, "env": r.8, "enabled": r.9,
            "connection_state": r.10, "last_error": r.11,
            "last_seen_at": r.12.to_rfc3339(),
            "discovered_at": r.13.to_rfc3339(),
        })).collect())
    }

    /// Toggle `enabled` for an MCP server. Returns the new state, or `None`
    /// if the id doesn't exist.
    pub async fn set_mcp_server_enabled(
        &self,
        id: &uuid::Uuid,
        enabled: bool,
    ) -> Result<Option<bool>, String> {
        let row: Option<(bool,)> = sqlx_core::query_as::query_as(
            "UPDATE sensei.mcp_servers
                SET enabled = $2,
                    connection_state = CASE WHEN $2 THEN connection_state ELSE 'disabled' END
              WHERE id = $1
          RETURNING enabled"
        ).bind(id).bind(enabled).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(e,)| e))
    }

    /// Delete rows the current scan did NOT touch — servers that no longer
    /// appear in any ACP config. Compares against `not_seen_before` so a
    /// row scanned after the cutoff survives. Returns the number of rows
    /// pruned. Called at the end of `discover_mcp_servers`.
    pub async fn prune_stale_mcp_servers(
        &self,
        not_seen_before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "DELETE FROM sensei.mcp_servers WHERE last_seen_at < $1"
        ).bind(not_seen_before).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    // ── Unified tool inventory (assistant_tools) + Instruments · Health grid ──

    /// Wipe the inventory — the capture repopulates from scratch so tools that
    /// vanished from a source don't linger.
    pub async fn clear_assistant_tools(&self) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.assistant_tools")
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Upsert one registered tool (idempotent on the (family, source_type,
    /// source_key, tool_name) unique index).
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_assistant_tool(
        &self, assistant_family: &str, source_type: &str, source_key: &str,
        tool_name: &str, invoked_name: &str, description: Option<&str>,
        server_id: Option<uuid::Uuid>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.assistant_tools
                (assistant_family, source_type, source_key, tool_name, invoked_name, description, server_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT (assistant_family, source_type, source_key, tool_name)
             DO UPDATE SET invoked_name = EXCLUDED.invoked_name,
                           description  = EXCLUDED.description,
                           server_id    = EXCLUDED.server_id,
                           updated_at   = now()"
        ).bind(assistant_family).bind(source_type).bind(source_key)
         .bind(tool_name).bind(invoked_name).bind(description).bind(server_id)
         .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Distinct built-in (non-MCP) tool names observed in usage — the harness's
    /// built-in catalog (no canonical list exists, so observed usage IS the
    /// registry: Bash, Read, Edit, Task, Skill, …).
    pub async fn distinct_builtin_tool_names(&self) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT tool_name FROM sensei.tool_usage_stats
              WHERE tool_name NOT LIKE 'mcp__%' ORDER BY tool_name"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    /// Usage-observed MCP prefixes → their bare tool names. Powers the bridge
    /// that maps a probed server to its harness usage key.
    pub async fn usage_mcp_prefix_tools(
        &self,
    ) -> Result<std::collections::HashMap<String, std::collections::HashSet<String>>, String> {
        let rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT split_part(tool_name,'__',2) AS prefix,
                    split_part(tool_name,'__',3) AS bare
               FROM sensei.tool_usage_stats
              WHERE tool_name LIKE 'mcp__%'"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        let mut map: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        for (p, b) in rows { map.entry(p).or_default().insert(b); }
        Ok(map)
    }

    /// Set an MCP server's connection state (after a probe attempt).
    pub async fn set_mcp_server_connection_state(
        &self, id: &uuid::Uuid, state: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.mcp_servers SET connection_state = $2, last_seen_at = now() WHERE id = $1"
        ).bind(id).bind(state).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The Instruments · Health L1 grid — one row per tool source. Unions the
    /// inventory (registered known) with usage-observed MCP sources not yet in
    /// the inventory (registered unknown → null share, never a fabricated bar).
    pub async fn get_tools_health(&self) -> Result<Vec<serde_json::Value>, String> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(String, String, String, Option<i64>, Option<uuid::Uuid>, i64, i64, Option<String>)> =
            sqlx_core::query_as::query_as(
            // `evt` is the real 14-day usage window: one row per tool with its
            // PostToolUse count over the last 14 days. `assistant_events.ts` is
            // epoch MILLIS (bigint), so the cutoff is computed in millis too.
            // tool_usage_stats is an all-time view — never use it for the window.
            "WITH evt AS (
               SELECT h.tool_name AS tool_name, count(*)::bigint AS calls_14d
                 FROM activity.assistant_events h
                WHERE h.event_type = 'PostToolUse' AND h.tool_name IS NOT NULL
                  AND h.ts >= (extract(epoch from now() - interval '14 days') * 1000)::bigint
                GROUP BY h.tool_name ),
             reg AS (
               SELECT assistant_family, source_type, source_key,
                      count(*)::bigint AS registered,
                      (array_agg(server_id) FILTER (WHERE server_id IS NOT NULL))[1] AS server_id
                 FROM sensei.assistant_tools
                GROUP BY assistant_family, source_type, source_key ),
             inv AS (
               SELECT at.assistant_family, at.source_type, at.source_key,
                      count(DISTINCT e.tool_name)::bigint AS invoked_14d,
                      coalesce(sum(e.calls_14d),0)::bigint AS calls_14d
                 FROM sensei.assistant_tools at
                 JOIN evt e ON e.tool_name = at.invoked_name
                GROUP BY at.assistant_family, at.source_type, at.source_key ),
             uncovered AS (
               SELECT 'claude'::text AS assistant_family, 'mcp'::text AS source_type,
                      split_part(e.tool_name,'__',2) AS source_key,
                      count(DISTINCT e.tool_name)::bigint AS invoked_14d,
                      coalesce(sum(e.calls_14d),0)::bigint AS calls_14d
                 FROM evt e
                WHERE e.tool_name LIKE 'mcp__%'
                  AND NOT EXISTS (SELECT 1 FROM sensei.assistant_tools at WHERE at.invoked_name = e.tool_name)
                GROUP BY split_part(e.tool_name,'__',2) )
             SELECT r.assistant_family, r.source_type, r.source_key,
                    r.registered, r.server_id,
                    coalesce(i.invoked_14d,0)::bigint, coalesce(i.calls_14d,0)::bigint,
                    s.connection_state
               FROM reg r
               LEFT JOIN inv i ON i.assistant_family=r.assistant_family
                              AND i.source_type=r.source_type AND i.source_key=r.source_key
               LEFT JOIN sensei.mcp_servers s ON s.id = r.server_id
             UNION ALL
             SELECT assistant_family, source_type, source_key,
                    NULL::bigint, NULL::uuid, invoked_14d, calls_14d, NULL::text
               FROM uncovered
             ORDER BY 7 DESC"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(family, stype, skey, registered, server_id, invoked, calls, conn)| {
            let connected = match stype.as_str() {
                "builtin" => true,
                _ => conn.as_deref() == Some("connected"),
            };
            let share = registered.filter(|r| *r > 0).map(|r| invoked as f64 / r as f64);
            serde_json::json!({
                "assistant_family": family,
                "source_type": stype,
                "source_key": skey,
                "name": crate::tool_discovery::pretty_source_name(&stype, &skey),
                "connected": connected,
                "connection_state": conn,
                "server_id": server_id,
                "tools_registered": registered,
                "tools_invoked_14d": invoked,
                "calls_14d": calls,
                "share_invoked": share,
            })
        }).collect())
    }

    /// All verdicts for one session, ordered by the underlying event ts.
    /// Consumed by the Replay tab's timeline read path (#84 / #90).
    pub async fn get_verdicts_for_session(
        &self,
        client_session_id: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(i64, Option<String>, String, f32, Option<String>, chrono::DateTime<chrono::Utc>, i64)> =
            sqlx_core::query_as::query_as(
                "SELECT v.event_id, v.tool_name, v.verdict, v.confidence, v.reason,
                        v.classified_at, ae.ts
                   FROM sensei.tool_call_verdicts v
                   JOIN activity.assistant_events ae ON ae.id = v.event_id
                  WHERE v.session_id = $1
               ORDER BY ae.ts"
            )
            .bind(client_session_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(event_id, tool_name, verdict, confidence, reason, classified_at, ts)| {
            serde_json::json!({
                "event_id":       event_id,
                "tool_name":      tool_name,
                "verdict":        verdict,
                "confidence":     confidence,
                "reason":         reason,
                "classified_at":  classified_at.to_rfc3339(),
                "ts":             ts,
            })
        }).collect())
    }

    /// Session-level summary of verdicts — the counts by outcome. Cheap to
    /// project into a StatBlock on the Replay/Health tab.
    pub async fn get_verdict_summary_for_session(
        &self,
        client_session_id: &str,
    ) -> Result<serde_json::Value, String> {
        let rows: Vec<(String, i64)> = sqlx_core::query_as::query_as(
            "SELECT verdict, count(*)::bigint FROM sensei.tool_call_verdicts
              WHERE session_id = $1 GROUP BY verdict"
        )
        .bind(client_session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let mut used = 0i64;
        let mut partial = 0i64;
        let mut ignored = 0i64;
        for (v, n) in rows {
            match v.as_str() {
                "used" => used = n,
                "partial" => partial = n,
                "ignored" => ignored = n,
                _ => {}
            }
        }
        let total = used + partial + ignored;
        Ok(serde_json::json!({
            "used":    used,
            "partial": partial,
            "ignored": ignored,
            "total":   total,
        }))
    }

    /// `(session uuid, client_session_id)` for a project's sessions that NEED
    /// (re)enrichment — never analyzed (`analyzed_at IS NULL`), or with
    /// assistant_events newer than the last analysis. Lets the scheduler skip
    /// unchanged sessions so enrichment cost scales with NEW activity, not total
    /// history (#67 incremental).
    pub async fn get_project_sessions_needing_enrichment(&self, project_id: &uuid::Uuid) -> Result<Vec<(uuid::Uuid, String, uuid::Uuid)>, String> {
        let rows: Vec<(uuid::Uuid, String, uuid::Uuid)> = sqlx_core::query_as::query_as(
            "SELECT s.id, s.client_session_id, s.folder_id FROM activity.sessions s
             WHERE s.project_id = $1 AND s.client_session_id IS NOT NULL
               AND (s.analyzed_at IS NULL
                    OR EXISTS (SELECT 1 FROM activity.assistant_events e
                               WHERE e.session_id = s.client_session_id
                                 AND to_timestamp(e.ts / 1000.0) > s.analyzed_at))"
        ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// `(project_id, latest_session_activity)` for every project with attributed
    /// sessions — drives the analyzer scheduler's "what changed since last run"
    /// check (#67).
    pub async fn get_projects_with_session_activity(&self) -> Result<Vec<(uuid::Uuid, chrono::DateTime<chrono::Utc>)>, String> {
        let rows: Vec<(uuid::Uuid, chrono::DateTime<chrono::Utc>)> = sqlx_core::query_as::query_as(
            "SELECT project_id, max(GREATEST(started_at, COALESCE(completed_at, started_at)))
             FROM activity.sessions WHERE project_id IS NOT NULL GROUP BY project_id"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Per-(folder, file) re-edit churn across a project's sessions — the
    /// SignalDeriver's rework anti-pattern source (#68, L1). Tool failures aren't
    /// captured, so churn (the same file edited many times in ONE session) is the
    /// "result needed follow-ups" signal. Returns `(folder_id, file,
    /// max_session_edits, total_edits)` only for files whose busiest single
    /// session reaches `min_session_edits`.
    /// `folders = Some(ids)` scopes derivation to just those folders (incremental
    /// re-derive — only folders touched by newly-enriched sessions); `None` =
    /// the whole project (full refresh / on-demand).
    pub async fn get_file_churn_stats(&self, project_id: &uuid::Uuid, min_session_edits: i64, folders: Option<&[uuid::Uuid]>) -> Result<Vec<(uuid::Uuid, String, i64, i64)>, String> {
        let rows: Vec<(uuid::Uuid, String, i64, i64)> = sqlx_core::query_as::query_as(
            "WITH per_session AS (
                 SELECT s.folder_id,
                        ae.payload->'tool_input'->>'file_path' AS file,
                        ae.session_id,
                        count(*) AS edits
                 FROM activity.assistant_events ae
                 JOIN activity.sessions s ON s.client_session_id = ae.session_id
                 WHERE s.project_id = $1
                   AND ($3::uuid[] IS NULL OR s.folder_id = ANY($3))
                   AND ae.event_type = 'PostToolUse'
                   AND ae.tool_name IN ('Edit', 'Write', 'MultiEdit')
                   AND ae.payload->'tool_input'->>'file_path' IS NOT NULL
                 GROUP BY s.folder_id, ae.payload->'tool_input'->>'file_path', ae.session_id
             )
             SELECT folder_id, file,
                    max(edits)::bigint AS max_session_edits,
                    sum(edits)::bigint AS total_edits
             FROM per_session
             GROUP BY folder_id, file
             HAVING max(edits) >= $2"
        ).bind(project_id).bind(min_session_edits).bind(folders.map(<[uuid::Uuid]>::to_vec)).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// User-prompt text across a project's sessions — the SignalDeriver's
    /// correction / rule-candidate source (#68, L1). Returns `(folder_id,
    /// session_id, prompt)` for every UserPromptSubmit carrying prompt text.
    /// `folders = Some(ids)` scopes to those folders (incremental re-derive);
    /// `None` = whole project.
    pub async fn get_project_prompts(&self, project_id: &uuid::Uuid, folders: Option<&[uuid::Uuid]>) -> Result<Vec<(uuid::Uuid, String, String)>, String> {
        let rows: Vec<(uuid::Uuid, String, String)> = sqlx_core::query_as::query_as(
            "SELECT s.folder_id, ae.session_id, ae.payload->>'prompt'
             FROM activity.assistant_events ae
             JOIN activity.sessions s ON s.client_session_id = ae.session_id
             WHERE s.project_id = $1
               AND ($2::uuid[] IS NULL OR s.folder_id = ANY($2))
               AND ae.event_type = 'UserPromptSubmit'
               AND ae.payload->>'prompt' IS NOT NULL
             ORDER BY ae.ts"
        ).bind(project_id).bind(folders.map(<[uuid::Uuid]>::to_vec)).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    // ── Transcript backfill (#73) ────────────────────────────────────────────

    /// Upsert parsed transcript turns for a (source, session). Idempotent by
    /// (source, session_id, turn_index). Returns the number of rows written.
    pub async fn upsert_transcript_turns(
        &self, source: &str, session_id: &str, family: &str,
        provider: Option<&str>, model: Option<&str>,
        turns: &[crate::transcript::TranscriptTurn],
    ) -> Result<u32, String> {
        let mut n = 0u32;
        for t in turns {
            let char_count = t.assistant_text.chars().count() as i32;
            sqlx_core::query::query(
                "INSERT INTO activity.transcript_turns
                    (source, session_id, family, provider, model, turn_index, user_text, assistant_text, char_count, started_at)
                 VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                 ON CONFLICT(source, session_id, turn_index) DO UPDATE SET
                   provider       = EXCLUDED.provider,
                   model          = EXCLUDED.model,
                   user_text      = EXCLUDED.user_text,
                   assistant_text = EXCLUDED.assistant_text,
                   char_count     = EXCLUDED.char_count,
                   started_at     = EXCLUDED.started_at"
            )
            .bind(source).bind(session_id).bind(family).bind(provider).bind(model).bind(t.turn_index)
            .bind(&t.user_text).bind(&t.assistant_text).bind(char_count).bind(t.started_at)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
            n += 1;
        }
        Ok(n)
    }

    /// Last-ingested mtime (ns) for a transcript file, or None if never seen.
    pub async fn get_transcript_cursor(&self, source: &str, file_path: &str) -> Result<Option<i64>, String> {
        let row: Option<(i64,)> = sqlx_core::query_as::query_as(
            "SELECT last_mtime_ns FROM activity.transcript_cursor WHERE source = $1 AND file_path = $2"
        ).bind(source).bind(file_path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    /// Advance the ingest cursor for a transcript file (idempotent upsert).
    pub async fn set_transcript_cursor(
        &self, source: &str, file_path: &str, session_id: Option<&str>, mtime_ns: i64, turns: i32,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO activity.transcript_cursor
                (source, file_path, session_id, last_mtime_ns, turns_ingested, updated_at)
             VALUES($1, $2, $3, $4, $5, now())
             ON CONFLICT(source, file_path) DO UPDATE SET
               session_id     = EXCLUDED.session_id,
               last_mtime_ns  = EXCLUDED.last_mtime_ns,
               turns_ingested = EXCLUDED.turns_ingested,
               updated_at     = now()"
        ).bind(source).bind(file_path).bind(session_id).bind(mtime_ns).bind(turns)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Historical-bootstrap import (#75) ────────────────────────────────────

    /// Resolve `(folder_id, project_id)` for a repo path — the importer's
    /// project mapping from a transcript's cwd. Matches the folder whose current
    /// `abs_path` is the path, OR (fallback) whose `folder_path_aliases` includes it
    /// — so a transcript recorded under an OLD path (before a rename/move) still
    /// resolves to the current folder + project. A live abs_path match wins over an
    /// alias. None if the path isn't a tracked folder or a known former path.
    pub async fn get_folder_ids_by_path(&self, abs_path: &str) -> Result<Option<(uuid::Uuid, Option<uuid::Uuid>)>, String> {
        let row: Option<(uuid::Uuid, Option<uuid::Uuid>)> = sqlx_core::query_as::query_as(
            "SELECT f.id, f.project_id FROM sensei.folders f
             WHERE f.abs_path = $1
                OR f.id = (SELECT folder_id FROM sensei.folder_path_aliases WHERE alias_abs_path = $1)
             ORDER BY (f.abs_path = $1) DESC
             LIMIT 1"
        ).bind(abs_path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Register a former absolute path for a folder (`folder_path_aliases`), so a
    /// transcript/hook cwd recorded at the old path still resolves to this folder +
    /// its project after a rename/move. Idempotent (re-registering an alias updates
    /// its reason). `reason` is `rename` (explicit) or `detected` (git-remote match).
    pub async fn add_folder_path_alias(
        &self,
        alias_abs_path: &str,
        folder_id: &uuid::Uuid,
        reason: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.folder_path_aliases (alias_abs_path, folder_id, reason)
             VALUES ($1, $2, $3)
             ON CONFLICT (alias_abs_path) DO UPDATE SET folder_id = EXCLUDED.folder_id, reason = EXCLUDED.reason",
        )
        .bind(alias_abs_path)
        .bind(folder_id)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Metrics: value store + active registry (Phase 3) ──────────────────

    /// Upsert one `sensei.project_metrics` row, keyed on its identity
    /// `(metric_id, project_id, folder_id, session_id, computed_on, grain)` — the
    /// `project_metrics_identity` unique index (`nulls not distinct`, so a
    /// project-scope / daily-grain row's null `folder_id`/`session_id` collide
    /// rather than duplicate). A re-run with the same identity BACKFILLS in place —
    /// updates `value`, `props`, `source` and bumps `modified_at` — so the compute
    /// tasks are idempotent. Returns the row id. `grain` is the
    /// `sensei.metric_grain` enum (`daily`|`session`); `source` the
    /// `sensei.metric_source` enum (`measured`|`estimated`).
    ///
    /// `project_metrics_identity` is a unique INDEX, not a named constraint, so the
    /// conflict target is the column list — Postgres infers the arbiter index
    /// (honouring its `nulls not distinct`); `ON CONFLICT ON CONSTRAINT <name>`
    /// would not resolve against an index.
    pub async fn upsert_project_metric(
        &self,
        metric_id:   &uuid::Uuid,
        project_id:  &uuid::Uuid,
        folder_id:   Option<&uuid::Uuid>,
        session_id:  Option<&uuid::Uuid>,
        computed_on: chrono::NaiveDate,
        grain:       &str,
        value:       f64,
        props:       &serde_json::Value,
        source:      &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.project_metrics
                (metric_id, project_id, folder_id, session_id, computed_on, grain, value, props, source)
             VALUES ($1, $2, $3, $4, $5, $6::sensei.metric_grain, $7::float8::numeric, $8, $9::sensei.metric_source)
             ON CONFLICT (metric_id, project_id, folder_id, session_id, computed_on, grain) DO UPDATE
                SET value       = EXCLUDED.value,
                    props       = EXCLUDED.props,
                    source      = EXCLUDED.source,
                    modified_at = now()
             RETURNING id",
        )
        .bind(metric_id)
        .bind(project_id)
        .bind(folder_id)
        .bind(session_id)
        .bind(computed_on)
        .bind(grain)
        .bind(value)
        .bind(props)
        .bind(source)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

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

    /// The ACTIVE metric registry: rows that live on `current_date` (see
    /// [`Self::ACTIVE_METRIC_PREDICATE`]) — retired (past/at `effective_until`) and
    /// not-yet-effective (future `effective_from`) rows are excluded. Drives the
    /// scheduler and the compute handlers.
    pub async fn active_metrics(&self) -> Result<Vec<Metric>, String> {
        let sql = format!(
            "SELECT id, key, name, description, family::text, type::text, unit, direction::text,
                    purpose, how_to_read, formula, task_name, weight::float8, target::float8,
                    effective_from, effective_until
               FROM sensei.metrics
              WHERE {}
              ORDER BY key",
            Self::ACTIVE_METRIC_PREDICATE,
        );
        let rows: Vec<(
            uuid::Uuid, String, String, String, String, String, Option<String>, String,
            String, String, String, String, f64, Option<f64>, chrono::NaiveDate, Option<chrono::NaiveDate>,
        )> = sqlx_core::query_as::query_as(&sql)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(
                id, key, name, description, family, metric_type, unit, direction,
                purpose, how_to_read, formula, task_name, weight, target, effective_from, effective_until,
            )| Metric {
                id, key, name, description, family, metric_type, unit, direction,
                purpose, how_to_read, formula, task_name, weight, target, effective_from, effective_until,
            })
            .collect())
    }

    /// Distinct `task_name`s over the ACTIVE metric registry — the set of compiled
    /// TaskKinds the scheduler must dispatch. Same active-window filter as
    /// [`Self::active_metrics`].
    pub async fn active_task_names(&self) -> Result<Vec<String>, String> {
        let sql = format!(
            "SELECT DISTINCT task_name FROM sensei.metrics WHERE {} ORDER BY task_name",
            Self::ACTIVE_METRIC_PREDICATE,
        );
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(&sql)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(t,)| t).collect())
    }

    /// `key → metric_id` for every ACTIVE metric whose `task_name` matches — the
    /// map a per-group compute handler resolves its metrics through (shared by all
    /// six base groups so each doesn't re-implement the filter-by-task_name loop).
    /// Built on [`Self::active_metrics`] (same active-window filter), so a retired /
    /// not-yet-effective / unseeded metric is simply ABSENT from the map and the
    /// caller skips it (`ids.get(key)` → `None`) — an inactive metric is never
    /// computed. Propagates the read error; never masks it.
    pub async fn active_metric_ids(
        &self,
        task_name: &str,
    ) -> Result<std::collections::HashMap<String, uuid::Uuid>, String> {
        Ok(self
            .active_metrics()
            .await?
            .into_iter()
            .filter(|m| m.task_name == task_name)
            .map(|m| (m.key, m.id))
            .collect())
    }

    /// Resolve an absolute folder path to its `(folder_id, project_id)` for
    /// project-scoped metric attribution. Thin wrapper over
    /// [`Self::get_folder_ids_by_path`] (matches `folders.abs_path`, then falls
    /// back to a `folder_path_aliases.alias_abs_path`) that additionally REQUIRES a
    /// project: a folder not yet attached to a project can't own a project-scoped
    /// metric, so it resolves to `None` — an honest miss, never a fabricated
    /// project id. Reuses the shared resolver rather than duplicating the
    /// folders/alias SQL.
    pub async fn resolve_folder_by_path(
        &self,
        folder_path: &str,
    ) -> Result<Option<(uuid::Uuid, uuid::Uuid)>, String> {
        Ok(self
            .get_folder_ids_by_path(folder_path)
            .await?
            .and_then(|(folder_id, project_id)| project_id.map(|p| (folder_id, p))))
    }

    /// Latest stored value per metric for a project, with the catalog facets it is
    /// read through — reads `sensei.project_metric_daily` (project-scope daily
    /// rows) and keeps the newest `date` per metric (`DISTINCT ON`), joining
    /// `sensei.metrics` for name/type/unit/direction/purpose/how_to_read. Empty
    /// when the project has no daily rows yet (honest-empty, not a failure). Trend
    /// (prior/delta over `project_metric_trend`) is deferred to the Phase 7
    /// endpoint.
    pub async fn get_project_metrics(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<ProjectMetricRow>, String> {
        let rows: Vec<(
            String, chrono::NaiveDate, f64, serde_json::Value, String, String,
            Option<String>, String, String, String,
        )> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT ON (d.metric)
                    d.metric, d.date, d.value::float8, d.props,
                    m.name, m.type::text, m.unit, m.direction::text, m.purpose, m.how_to_read
               FROM sensei.project_metric_daily d
               JOIN sensei.metrics m ON m.key = d.metric
              WHERE d.project_id = $1
              ORDER BY d.metric, d.date DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(
                metric, date, value, props, name, metric_type, unit, direction, purpose, how_to_read,
            )| ProjectMetricRow {
                metric, date, value, props, name, metric_type, unit, direction, purpose, how_to_read,
            })
            .collect())
    }

    /// Latest weekly trend point per metric for a project — reads
    /// `sensei.project_metric_trend` (the weekly `lag()` view), keeping the newest
    /// `period` per metric (`DISTINCT ON`). Powers the trend arrow on the project
    /// metrics endpoint: `prior`/`delta` are `None` for a metric with a single
    /// weekly period (honest-null, never a fabricated 0). Empty when the project
    /// has no daily rows yet (honest-empty, not a failure). Propagates the read
    /// error; never masks it.
    pub async fn get_project_metric_trend(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<ProjectMetricTrendRow>, String> {
        let rows: Vec<(String, chrono::NaiveDate, f64, Option<f64>, Option<f64>, String)> =
            sqlx_core::query_as::query_as(
                "SELECT DISTINCT ON (metric)
                        metric, period, value::float8, prior::float8, delta::float8, direction::text
                   FROM sensei.project_metric_trend
                  WHERE project_id = $1
                  ORDER BY metric, period DESC",
            )
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(metric, period, value, prior, delta, direction)| ProjectMetricTrendRow {
                metric, period, value, prior, delta, direction,
            })
            .collect())
    }

    /// The time series for ONE metric of a project at a chosen `grain`, read from
    /// the matching roll-up view: `daily` → `project_metric_daily` (raw stored
    /// values); `weekly`/`monthly`/`quarterly` → the roll-up view that re-derives
    /// each period from sums (Σnum/Σden for ratio/pct — NEVER the mean of daily
    /// ratios). `grain` MUST be one of `daily`|`weekly`|`monthly`|`quarterly`; any
    /// other value is an `Err` (the caller 400s) rather than a silent default that
    /// would mismeasure. An unknown metric key — or a project with no rows — yields
    /// an empty series (honest-empty, not a failure). Propagates the read error;
    /// never masks it.
    pub async fn get_project_metric_series(
        &self,
        project_id: &uuid::Uuid,
        key: &str,
        grain: &str,
    ) -> Result<Vec<ProjectMetricSeriesPoint>, String> {
        // The view + its period column are chosen from a fixed allowlist keyed on
        // the validated grain — no user-supplied string ever reaches the SQL, so
        // the `format!` is injection-safe.
        let (view, period_col) = match grain {
            "daily"     => ("sensei.project_metric_daily",     "date"),
            "weekly"    => ("sensei.project_metric_weekly",    "period"),
            "monthly"   => ("sensei.project_metric_monthly",   "period"),
            "quarterly" => ("sensei.project_metric_quarterly", "period"),
            other => return Err(format!("invalid grain: {other:?}")),
        };
        let sql = format!(
            "SELECT {period_col} AS period, value::float8, direction::text
               FROM {view}
              WHERE project_id = $1 AND metric = $2
              ORDER BY {period_col}",
        );
        let rows: Vec<(chrono::NaiveDate, f64, String)> = sqlx_core::query_as::query_as(&sql)
            .bind(project_id)
            .bind(key)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(period, value, direction)| ProjectMetricSeriesPoint { period, value, direction })
            .collect())
    }

    /// Re-attach orphaned sessions: `activity.assistant_events` rows whose session
    /// no longer has an `activity.sessions` row (its folder was cascade-deleted on a
    /// repo delete/rename, but the events — session-id-keyed, no FK — survived). For
    /// each, recreate the session row, resolving `folder_id` from the session's cwd
    /// via [`find_folder_for_path`] (alias-aware, so a renamed repo's history
    /// re-attaches to the current folder + project). Sessions whose cwd still doesn't
    /// resolve (no folder, no alias) are left orphaned. Idempotent — a session that
    /// already has a row isn't reprocessed. Returns the number repaired.
    pub async fn repair_orphaned_sessions(&self) -> Result<u32, String> {
        // All distinct cwds per orphaned session. We try them MOST-SPECIFIC (longest)
        // first: a renamed subdir (`…/monorepo/docs`, aliased to the new repo) is a
        // deeper — and thus stronger — signal than a still-live parent (`…/strategos`)
        // that would otherwise shadow it and misattribute. find_folder_for_path is
        // alias-aware, so the longest matching path wins via its alias.
        let orphans: Vec<(String, Vec<String>, String)> = sqlx_core::query_as::query_as(
            "SELECT e.session_id,
                    COALESCE(array_agg(DISTINCT e.cwd) FILTER (WHERE e.cwd IS NOT NULL), '{}') AS cwds,
                    (array_agg(e.family::text))[1] AS family
               FROM activity.assistant_events e
              WHERE e.session_id <> ''
                AND NOT EXISTS (
                    SELECT 1 FROM activity.sessions s WHERE s.client_session_id = e.session_id)
              GROUP BY e.session_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let mut repaired = 0u32;
        for (session_id, mut cwds, family) in orphans {
            cwds.sort_by_key(|c| std::cmp::Reverse(c.len())); // most-specific first
            let mut resolved = None;
            for cwd in &cwds {
                if let Ok(Some(fp)) = self.find_folder_for_path(cwd).await {
                    resolved = Some(fp);
                    break;
                }
            }
            let Some((folder_id, project_id)) = resolved else {
                continue; // no cwd resolves (no folder, no alias) — leave orphaned
            };
            if self
                .record_session_event(&session_id, &folder_id, project_id.as_ref(), &family, true)
                .await
                .is_ok()
            {
                repaired += 1;
            }
        }
        Ok(repaired)
    }

    /// True if a session already has captured/imported events — the dedup guard
    /// so the importer never double-counts a live-captured (or already-imported) session.
    pub async fn session_has_events(&self, client_session_id: &str) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM activity.assistant_events WHERE session_id = $1)"
        ).bind(client_session_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Mark a session as synthesized from a historical transcript (#75) and set
    /// its real historical start/end from the transcript timestamps (so it
    /// doesn't masquerade as "today" in the FTR/quality time windows).
    pub async fn set_session_history(&self, client_session_id: &str, started_ms: i64, completed_ms: i64) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.sessions SET backfilled = true,
                 started_at   = to_timestamp($2::float8 / 1000.0),
                 completed_at = to_timestamp($3::float8 / 1000.0)
             WHERE client_session_id = $1"
        ).bind(client_session_id).bind(started_ms).bind(completed_ms)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Set the inference `provider` + `model` that ran a session (captured from
    /// the transcript at synthesis, #75). Idempotent. Powers effectiveness-by-model.
    pub async fn set_session_model(
        &self, client_session_id: &str, provider: &str, model: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.sessions SET provider = $2, model = $3 WHERE client_session_id = $1",
        )
        .bind(client_session_id)
        .bind(provider)
        .bind(model)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Effectiveness aggregated by model: per (provider, model), how many
    /// enriched sessions, the First-Try Rate, average corrections, and average
    /// turns. The cross-model comparison the multi-model corpus (Zed + Claude)
    /// unlocks. Ordered by session volume.
    pub async fn get_model_effectiveness(&self) -> Result<Vec<serde_json::Value>, String> {
        // Raw per-(provider, raw-model) SUMS; folded by canonical model in Rust
        // (re-weighting FTR) so label variants aggregate — see model_insight.
        let rows: Vec<(Option<String>, String, i64, i64, i64, i64)> = sqlx_core::query_as::query_as(
            "SELECT provider, model,
                    count(*) AS sessions,
                    count(*) FILTER (WHERE ftr)::int8 AS ftr_sessions,
                    sum(corrections)::int8 AS corrections,
                    sum(turns)::int8 AS turns
               FROM activity.sessions
              WHERE model IS NOT NULL AND analyzed_at IS NOT NULL
              GROUP BY provider, model",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let raw = rows
            .into_iter()
            .map(|(provider, model, sessions, ftr, corr, turns)| {
                (provider.unwrap_or_default(), model, sessions, ftr, corr, turns)
            })
            .collect();
        Ok(crate::model_insight::fold_effectiveness(raw))
    }

    /// Per-(provider, canonical-model) FTR over a project's enriched, model-tagged
    /// sessions — the input to the model-effectiveness recommendation. Label
    /// variants are folded to a canonical model (model_insight::fold_model_stats).
    pub async fn get_project_model_stats(
        &self, project_id: &uuid::Uuid,
    ) -> Result<Vec<crate::model_insight::ModelStat>, String> {
        let rows: Vec<(Option<String>, String, i64, i64)> = sqlx_core::query_as::query_as(
            "SELECT provider, model, count(*) AS sessions,
                    count(*) FILTER (WHERE ftr)::int8 AS ftr_sessions
               FROM activity.sessions
              WHERE project_id = $1 AND model IS NOT NULL AND analyzed_at IS NOT NULL
              GROUP BY provider, model",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let raw = rows
            .into_iter()
            .map(|(provider, model, sessions, ftr)| (provider.unwrap_or_default(), model, sessions, ftr))
            .collect();
        Ok(crate::model_insight::fold_model_stats(raw))
    }

    /// True if a pending recommendation already proposes `model` for this project
    /// (the model-insight generator's idempotency guard).
    pub async fn model_recommendation_exists(
        &self, project_id: &uuid::Uuid, model: &str,
    ) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(
               SELECT 1 FROM inference.recommendations
                WHERE project_id = $1 AND status = 'pending'
                  AND based_on->>'recommended_model' = $2
             )",
        )
        .bind(project_id)
        .bind(model)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Write enrichment metrics onto a session (#66). Sets the derived fields
    /// and merges `tool_usage` into `props` — deliberately does NOT touch
    /// `completed_at` (owned by the hook-stream session derivation, #31).
    pub async fn update_session_metrics(
        &self, session_id: &uuid::Uuid, turns: i32, corrections: i32, outcome: &str,
        ftr: bool, duration_ms: i64, module: Option<&str>, tool_usage: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.sessions
                SET outcome = $2::sensei.session_outcome, ftr = $3, turns = $4,
                    corrections = $5, duration = make_interval(secs => $6::float8 / 1000.0),
                    module = $7, analyzed_at = now(),
                    props = props || jsonb_build_object('tool_usage', $8::jsonb)
              WHERE id = $1"
        ).bind(session_id).bind(outcome).bind(ftr).bind(turns).bind(corrections)
            .bind(duration_ms).bind(module).bind(tool_usage)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Persist a session's retrospective summary — but only when the row has no
    /// summary yet (NULL or blank). `activity.sessions.summary` may be authored
    /// by the assistant at checkpoint time; this fills the (large) gap of empty
    /// summaries with the analyzer-derived narrative without ever clobbering an
    /// existing one. Idempotent and safe to re-run — a populated summary is left
    /// untouched. Called from `analyze::enrich_session` on each analysis pass.
    pub async fn set_session_summary_if_empty(&self, session_id: &uuid::Uuid, summary: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.sessions SET summary = $2
              WHERE id = $1 AND (summary IS NULL OR btrim(summary) = '')"
        ).bind(session_id).bind(summary).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Replace a session's per-turn rows (#66). Deletes the session's existing
    /// turns and re-inserts from a JSON array `[{turn_number, segment,
    /// started_ms, ended_ms, duration_ms, is_correction, triage_signal,
    /// tool_calls}]` — ms epochs/durations are converted to timestamptz/interval
    /// here. Idempotent (delete + reinsert), so re-enrichment never duplicates.
    pub async fn replace_session_turns(&self, session_id: &uuid::Uuid, turns: &serde_json::Value) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM activity.turns WHERE session_id = $1")
            .bind(session_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        sqlx_core::query::query(
            "INSERT INTO activity.turns
               (session_id, turn_number, segment, started_at, ended_at, duration, is_correction, triage_signal, tool_calls)
             SELECT $1, (t->>'turn_number')::int, (t->>'segment')::int,
                    to_timestamp((t->>'started_ms')::bigint / 1000.0),
                    to_timestamp((t->>'ended_ms')::bigint / 1000.0),
                    make_interval(secs => (t->>'duration_ms')::bigint / 1000.0),
                    (t->>'is_correction')::bool, t->>'triage_signal', (t->>'tool_calls')::int
             FROM jsonb_array_elements($2::jsonb) t"
        ).bind(session_id).bind(turns).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Projects ──────────────────────────────────────────────────────

    pub async fn create_project(&self, name: &str, description: Option<&str>, client: Option<&str>) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.projects(name, description, client) VALUES($1, $2, $3) RETURNING id"
        ).bind(name).bind(description).bind(client)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Race-safe get-or-create of a project by name — the scan-time assignment
    /// path ([`crate::tasks::handlers::process_git_folder`]) calls this instead
    /// of a bare SELECT-then-INSERT. That earlier pattern raced across the
    /// concurrent scan workers: two folders resolving to the same project name
    /// both saw "no such project" and both called [`Self::create_project`],
    /// minting a second same-name row — the 0-folder "phantom" project that then
    /// made name resolution ambiguous.
    ///
    /// A transaction-scoped advisory lock keyed on the name serializes only
    /// concurrent creators of the SAME name (distinct names hash to distinct
    /// keys and never contend), closing the select-then-insert window WITHOUT a
    /// `UNIQUE(name)` constraint — which would be wrong, since two DIFFERENT
    /// repos may legitimately share a name (a project's identity is its folder
    /// path, not its name).
    ///
    /// When the name already has rows, the folder-bearing one is preferred, so a
    /// pre-existing phantom is never adopted over the real project (the phantom
    /// is pruned separately by [`Self::heal_duplicate_name_projects`]). Returns
    /// `(id, created)`; `created` is true only when a new row was minted, letting
    /// the caller emit its `project_add` event exactly once.
    pub async fn get_or_create_project_by_name(&self, name: &str) -> Result<(uuid::Uuid, bool), String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        // Serialize concurrent creators of this exact name. The lock is
        // transaction-scoped (auto-released on commit/rollback); hashtext maps
        // the name into the advisory key space.
        sqlx_core::query::query("SELECT pg_advisory_xact_lock(hashtext($1)::int8)")
            .bind(name)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;

        // Prefer the folder-bearing row so a not-yet-healed phantom is never
        // adopted over the real project; `id` is the stable tiebreak.
        let existing: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT p.id FROM sensei.projects p
              WHERE p.name = $1
              ORDER BY (SELECT count(*) FROM sensei.folders f WHERE f.project_id = p.id) DESC, p.id
              LIMIT 1",
        ).bind(name).fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;

        if let Some((id,)) = existing {
            tx.commit().await.map_err(|e| e.to_string())?;
            return Ok((id, false));
        }

        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.projects(name) VALUES($1) RETURNING id",
        ).bind(name).fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok((id, true))
    }

    /// Update a project's derived identity props (from README frontmatter or
    /// best-guess). Only overwrites description/client when provided; replaces
    /// stack only when a non-empty stack is given; unions tags.
    pub async fn set_project_identity(
        &self,
        id: &uuid::Uuid,
        description: Option<&str>,
        client: Option<&str>,
        stack: &[String],
        tags: &[String],
    ) -> Result<(), String> {
        let stack_json = serde_json::json!(stack);
        let tags_vec: Vec<String> = tags.to_vec();
        sqlx_core::query::query(
            "UPDATE sensei.projects
                SET description = COALESCE($2, description),
                    client      = COALESCE($3, client),
                    stack       = CASE WHEN jsonb_array_length($4) > 0 THEN $4 ELSE stack END,
                    tags        = array(SELECT DISTINCT unnest(tags || $5)),
                    modified_at = now()
              WHERE id = $1",
        )
        .bind(id)
        .bind(description)
        .bind(client)
        .bind(&stack_json)
        .bind(&tags_vec)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Overwrite a project's `icon` jsonb with a deterministically inferred icon
    /// ([[pipeline/project-icon]]). The caller guards against clobbering an
    /// author choice; this setter just persists the value.
    pub async fn set_project_icon(
        &self,
        id: &uuid::Uuid,
        icon: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.projects SET icon = $2, modified_at = now() WHERE id = $1",
        )
        .bind(id)
        .bind(icon)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get-or-create a namespace instance by (scope_key, slug). Returns its id.
    pub async fn upsert_namespace(
        &self,
        scope_key: &str,
        name: &str,
        slug: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.namespaces(scope_key, name, slug)
             VALUES($1, $2, $3)
             ON CONFLICT (scope_key, slug) DO UPDATE SET name = EXCLUDED.name, modified_at = now()
             RETURNING id",
        )
        .bind(scope_key)
        .bind(name)
        .bind(slug)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Link a folder (repo) to a namespace it belongs to. Idempotent.
    pub async fn link_folder_namespace(
        &self,
        folder_id: &uuid::Uuid,
        namespace_id: &uuid::Uuid,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.folder_namespaces(folder_id, namespace_id)
             VALUES($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(folder_id)
        .bind(namespace_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Merge icon metadata onto a folder (icons column is jsonb {emoji,devicon,custom}).
    pub async fn set_folder_icons(
        &self,
        folder_id: &uuid::Uuid,
        icons: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET icons = icons || $2, modified_at = now() WHERE id = $1",
        )
        .bind(folder_id)
        .bind(icons)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Self-healing reconcile: tag discovery projects with no member folders as
    /// `orphaned` (for the user to resolve), and clear the tag from any that
    /// regained folders. Never deletes. Returns rows changed.
    pub async fn mark_orphaned_projects(&self) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.projects p
                SET tags = CASE
                      WHEN NOT EXISTS (SELECT 1 FROM sensei.folders f WHERE f.project_id = p.id)
                        THEN array(SELECT DISTINCT unnest(p.tags || ARRAY['orphaned']))
                      ELSE array_remove(p.tags, 'orphaned')
                    END,
                    modified_at = now()
              WHERE p.maturity = 'discovery'
                AND ((NOT EXISTS (SELECT 1 FROM sensei.folders f WHERE f.project_id = p.id))
                     <> ('orphaned' = ANY(p.tags)))",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    /// Self-healing reconcile: DELETE `discovery` projects that hold nothing — no
    /// folders, no sessions, no learned artifacts (recommendations / memories).
    /// These are phantom rows left when a promoted crate/subfolder was later
    /// reconciled away (pre-#101 residue: names like `logger`, `senseid`,
    /// `gateway-embedded`). `mark_orphaned_projects` only tags them; this removes
    /// the provably-empty ones so they never reach the UI. A project mid-scan has
    /// its git/standalone folder already, so it never matches.
    ///
    /// `grace_secs` guards `modified_at`: scan reconcile passes 60 so a project
    /// just created but whose folder is still being attached in a concurrent step
    /// isn't deleted mid-population (also fixes a shared-test-DB FK race). A
    /// *deliberate* caller — the exclusion handler, which already deleted the
    /// subtree's folders — passes 0: those projects are provably orphaned, not
    /// in-flight, and a boot re-scan may have freshly bumped their `modified_at`.
    /// Returns rows deleted.
    pub async fn prune_empty_projects(&self, grace_secs: i32) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "DELETE FROM sensei.projects p
              WHERE p.maturity = 'discovery'
                AND p.modified_at < now() - make_interval(secs => $1)
                AND NOT EXISTS (SELECT 1 FROM sensei.folders f        WHERE f.project_id = p.id)
                AND NOT EXISTS (SELECT 1 FROM activity.sessions s     WHERE s.project_id = p.id)
                AND NOT EXISTS (SELECT 1 FROM inference.recommendations r WHERE r.project_id = p.id)
                AND NOT EXISTS (SELECT 1 FROM sensei.memories m       WHERE m.project_id = p.id)",
        )
        .bind(grace_secs)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    pub async fn get_project(&self, id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        #[allow(clippy::type_complexity)]
        let row: Option<(uuid::Uuid, String, Option<String>, Option<String>, String, Option<String>, serde_json::Value, serde_json::Value, serde_json::Value, Vec<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, description, client, maturity::text, goal, icon, stack, links, tags, modified_at FROM sensei.projects WHERE id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(row.map(|(id, name, desc, client, maturity, goal, icon, stack, links, tags, modified)| {
            serde_json::json!({
                "id": id, "name": name, "description": desc, "client": client,
                "maturity": maturity, "goal": goal, "icon": icon, "stack": stack, "links": links,
                "tags": tags, "modified_at": modified.to_rfc3339(),
            })
        }))
    }

    /// Top pending recommendation for a project — highest urgency, then id —
    /// including `default_acp` for the Overview hero's "send to {acp}" action.
    /// `None` when the project has no pending recommendation.
    pub async fn get_top_recommendation(&self, project_id: &uuid::Uuid) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, String, serde_json::Value, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, title, why, evidence, default_acp
             FROM inference.recommendations
             WHERE project_id = $1 AND status = 'pending'
             ORDER BY CASE urgency WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, id
             LIMIT 1"
        ).bind(project_id).fetch_optional(&self.pool).await
            .map_err(|e| { tracing::error!(error = %e, "get_top_recommendation failed"); e.to_string() })?;
        Ok(row.map(|(id, title, why, evidence, default_acp)| serde_json::json!({
            "id": id, "title": title, "why": why, "evidence": evidence, "defaultAcp": default_acp,
        })))
    }

    /// Overview stat scalars for a project in one round trip: active (non-
    /// archived) memory count, 7-day session + corrected counts, and open
    /// doc-drift + distinct-referenced-doc counts.
    ///
    /// `readyToShare` / `toMerge` are DERIVED from existing columns (no invented
    /// status — [[pipeline/memory]] defines a scope *ladder*, not new statuses):
    /// - `readyToShare` = established memories (status active/reinforced/
    ///   battle_tested) whose `scope` is narrower than the widest rung
    ///   (`global`) — i.e. promotable up the ladder (project→…→global).
    /// - `toMerge` = memories that share a normalized `title` with at least one
    ///   other memory in the project (dedup candidates). There is no signature
    ///   column, so a case/whitespace-folded title is the merge-candidate proxy.
    pub async fn get_project_overview_stats(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        let (mem_total, sessions_7d, sessions_7d_corrected, drift_open, referenced_docs, ready_to_share, to_merge):
            (i64, i64, i64, i64, i64, i64, i64) = sqlx_core::query_as::query_as(
            "SELECT
               (SELECT count(*) FROM sensei.memories
                  WHERE project_id = $1 AND status != 'archived'),
               (SELECT count(*) FROM activity.sessions
                  WHERE project_id = $1 AND started_at > now() - interval '7 days'),
               (SELECT count(*) FROM activity.sessions
                  WHERE project_id = $1 AND started_at > now() - interval '7 days' AND corrections > 0),
               (SELECT count(*) FROM sensei.project_drift
                  WHERE project_id = $1 AND status::text IN ('drifted','broken')),
               (SELECT count(DISTINCT di.doc_node_id) FROM inference.drift_items di
                  JOIN sensei.folders f ON f.id = di.folder_id WHERE f.project_id = $1),
               (SELECT count(*) FROM sensei.memories
                  WHERE project_id = $1
                    AND status::text IN ('active','reinforced','battle_tested')
                    AND scope::text <> 'global'),
               (SELECT coalesce(sum(c), 0)::bigint FROM (
                    SELECT count(*) AS c FROM sensei.memories
                      WHERE project_id = $1 AND status != 'archived'
                      GROUP BY lower(btrim(title)) HAVING count(*) > 1) g)"
        ).bind(project_id).fetch_one(&self.pool).await
            .map_err(|e| { tracing::error!(error = %e, "get_project_overview_stats failed"); e.to_string() })?;
        Ok(serde_json::json!({
            "sessions7d": sessions_7d,
            "sessions7dCorrected": sessions_7d_corrected,
            "memories": { "total": mem_total, "readyToShare": ready_to_share, "toMerge": to_merge },
            "docDrift": { "open": drift_open, "referencedDocs": referenced_docs },
        }))
    }

    /// Recent sessions for a project with the folder role they ran in (the
    /// multi-repo membership chip). Newest first, capped at `limit`. Duration
    /// and relative time are formatted client-side from the ISO timestamps, the
    /// same as the shared RecentSessions component.
    pub async fn list_recent_project_sessions_with_role(&self, project_id: &uuid::Uuid, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(uuid::Uuid, Option<String>, Option<bool>, i32, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT s.id, s.task, s.ftr, s.corrections, s.started_at, s.completed_at, f.role::text
                 FROM activity.sessions s
                 LEFT JOIN sensei.folders f ON f.id = s.folder_id
                 WHERE s.project_id = $1
                 ORDER BY s.started_at DESC LIMIT $2"
            ).bind(project_id).bind(limit).fetch_all(&self.pool).await
            .map_err(|e| { tracing::error!(error = %e, "list_recent_project_sessions_with_role failed"); e.to_string() })?;
        Ok(rows.into_iter().map(|(id, task, ftr, corrections, started, completed, role)| {
            serde_json::json!({
                "id": id,
                "title": task,
                "ftr": ftr,
                "corrections": corrections,
                "startedAt": started.to_rfc3339(),
                "completedAt": completed.map(|t| t.to_rfc3339()),
                "role": role,
            })
        }).collect())
    }

    pub async fn get_project_by_name(&self, name: &str) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, String, Option<String>, Option<String>, String, Option<String>, serde_json::Value, serde_json::Value, Vec<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, description, client, maturity::text, goal, stack, links, tags, modified_at FROM sensei.projects WHERE name = $1"
            ).bind(name).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(row.map(|(id, name, desc, client, maturity, goal, stack, links, tags, modified)| {
            serde_json::json!({
                "id": id, "name": name, "description": desc, "client": client,
                "maturity": maturity, "goal": goal, "stack": stack, "links": links,
                "tags": tags, "modified_at": modified.to_rfc3339(),
            })
        }))
    }

    pub async fn list_projects(&self) -> Result<Vec<serde_json::Value>, String> {
        self.list_projects_under(None).await
    }

    /// Like [`list_projects`], optionally scoped to a folder. When `under` is
    /// `Some(path)`, only projects that own at least one folder whose `abs_path`
    /// is `path` itself OR lives beneath `path` are returned — path-boundary-safe,
    /// so a sibling `path-other` is NOT matched (the boundary test is
    /// `left(abs_path, len(path)+1) = path || '/'`, never a raw `LIKE` prefix
    /// that would let `_`/`%` in the path act as wildcards). `None` returns every
    /// project (unchanged behavior). The path is a bound parameter — never
    /// interpolated.
    pub async fn list_projects_under(
        &self,
        under: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        // Additive: also returns icon/stack/vision(goal) plus repos_count /
        // libs_count / last_session_at / sessions7d so the Projects index can
        // render its card + list layouts without a per-project fanout. Existing
        // consumers (e.g. the Today loader) keep working — nothing is removed.
        // repos_count counts only real repos (folders.kind git|standalone), NOT
        // the ~10k nested `folder` rows.
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            uuid::Uuid, String, Option<String>, Option<String>, String, Vec<String>,
            chrono::DateTime<chrono::Utc>, Option<serde_json::Value>, Option<serde_json::Value>,
            Option<String>, Option<uuid::Uuid>, i64, i64, Option<chrono::DateTime<chrono::Utc>>, i64,
        )> = sqlx_core::query_as::query_as(
                "SELECT p.id, p.name, p.description, p.client, p.maturity::text, p.tags, p.modified_at,
                        p.icon, p.stack, p.goal, p.dojo_id,
                        (SELECT count(*) FROM sensei.folders f
                          WHERE f.project_id = p.id AND f.kind::text IN ('git','standalone'))::bigint AS repos_count,
                        (SELECT count(*) FROM sensei.project_libraries pl
                          WHERE pl.project_id = p.id)::bigint AS libs_count,
                        (SELECT max(s.started_at) FROM activity.sessions s WHERE s.project_id = p.id) AS last_session_at,
                        (SELECT count(*) FROM activity.sessions s
                          WHERE s.project_id = p.id AND s.started_at > now() - interval '7 days')::bigint AS sessions7d
                 FROM sensei.projects p
                 WHERE $1::text IS NULL OR EXISTS (
                          SELECT 1 FROM sensei.folders f
                           WHERE f.project_id = p.id
                             AND (f.abs_path = $1::text
                               OR left(f.abs_path, length($1::text) + 1) = $1::text || '/'))
                 ORDER BY p.name"
            ).bind(under).fetch_all(&self.pool).await
            .map_err(|e| { tracing::error!(error = %e, "list_projects failed"); e.to_string() })?;

        Ok(rows.into_iter().map(|(id, name, desc, client, maturity, tags, modified, icon, stack, vision, dojo_id, repos_count, libs_count, last_session_at, sessions7d)| {
            serde_json::json!({
                "id": id, "name": name, "description": desc, "client": client,
                "maturity": maturity, "tags": tags, "modified_at": modified.to_rfc3339(),
                "icon": icon, "stack": stack, "vision": vision,
                "dojo_id": dojo_id,
                "repos_count": repos_count, "libs_count": libs_count,
                "last_session_at": last_session_at.map(|t| t.to_rfc3339()),
                "sessions7d": sessions7d,
            })
        }).collect())
    }

    /// Partial-update a project's editable identity fields. Omitted (`None`)
    /// fields are left untouched via COALESCE, so a lossless patch from the
    /// About form only overwrites the columns the user actually edited. An
    /// unknown `maturity` is rejected up front (before the DB round trip)
    /// rather than allowed to fail as a raw Postgres enum-cast error.
    pub async fn update_project(&self, id: &uuid::Uuid, patch: &ProjectPatch<'_>) -> Result<(), String> {
        if let Some(m) = patch.maturity
            && !PROJECT_MATURITIES.contains(&m)
        {
            return Err(format!(
                "invalid maturity '{m}': expected one of {PROJECT_MATURITIES:?}"
            ));
        }
        sqlx_core::query::query(
            "UPDATE sensei.projects SET
                 name          = COALESCE($2, name),
                 description   = COALESCE($3, description),
                 maturity      = COALESCE($4::sensei.project_maturity, maturity),
                 client        = COALESCE($5, client),
                 goal          = COALESCE($6, goal),
                 preferred_acp = COALESCE($7, preferred_acp),
                 icon          = COALESCE($8, icon),
                 stack         = COALESCE($9, stack),
                 links         = COALESCE($10, links),
                 modified_at   = now()
             WHERE id = $1"
        )
        .bind(id)
        .bind(patch.name)
        .bind(patch.description)
        .bind(patch.maturity)
        .bind(patch.client)
        .bind(patch.goal)
        .bind(patch.preferred_acp)
        .bind(patch.icon)
        .bind(patch.stack)
        .bind(patch.links)
        .execute(&self.pool).await
        .map_err(|e| { tracing::error!(error = %e, "update_project failed"); e.to_string() })?;
        Ok(())
    }

    pub async fn delete_project(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Merge one project into another (#41). All source-project folders +
    /// sessions + memories are reassigned to `target`, then the source
    /// project row is deleted; ON DELETE CASCADE cleans up the derived rows
    /// (detected_patterns, recommendations, reasoning_traces,
    /// impact_verdicts, memory_share_batches, service_projects,
    /// project_dependencies edges terminating at the source).
    ///
    /// Derived signals (patterns/recommendations) are dropped and
    /// regenerated by the analyzer on the next tick over the merged corpus
    /// — that's why we don't try to hand-merge them here (the unique keys
    /// on those tables would need collision handling that isn't worth the
    /// code for a delete-and-rederive path). User-authored memories DO
    /// survive because `memories.project_id` is nullable + non-unique;
    /// they simply move under the target.
    ///
    /// Runs inside a transaction so a mid-way failure doesn't leave the
    /// merge half-done. Refuses `source == target` (no-op guarded up front)
    /// and errors if either project id doesn't exist.
    pub async fn merge_projects(
        &self,
        source: &uuid::Uuid,
        target: &uuid::Uuid,
    ) -> Result<(), String> {
        if source == target {
            return Err("merge_projects: source and target must differ".into());
        }
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        // Verify both projects exist.
        let (exists,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT COUNT(*) FROM sensei.projects WHERE id = ANY($1::uuid[])"
        )
            .bind([*source, *target].as_slice())
            .fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;
        if exists != 2 {
            return Err(format!(
                "merge_projects: expected source + target to exist, found {exists} of 2"
            ));
        }

        // Reassign the data-source rows. Order: folders first (they define
        // the corpus), then sessions, then memories (user-authored — must
        // survive the merge). Derived tables are left for CASCADE to trim.
        for stmt in [
            "UPDATE sensei.folders    SET project_id = $2 WHERE project_id = $1",
            "UPDATE activity.sessions SET project_id = $2 WHERE project_id = $1",
            "UPDATE sensei.memories   SET project_id = $2 WHERE project_id = $1",
        ] {
            sqlx_core::query::query(stmt)
                .bind(source).bind(target)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }

        // CASCADE deletes derived rows (detected_patterns / recommendations /
        // reasoning_traces / impact_verdicts / memory_share_batches /
        // service_projects / project_dependencies edges at either end).
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(source)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Deterministic self-heal for name-duplicate phantom projects: when a name
    /// is shared by EXACTLY ONE folder-bearing project (the survivor) and one or
    /// more 0-folder `discovery` projects (phantoms — an earlier
    /// select-then-insert race minted them; now prevented by
    /// [`Self::get_or_create_project_by_name`]), each phantom is merged into the
    /// survivor via [`Self::merge_projects`] (folders/sessions/memories
    /// reassigned, derived rows CASCADE-trimmed — no FK row is left orphaned).
    /// Idempotent: once the phantoms are gone the candidate query returns
    /// nothing, so a re-run is a no-op.
    ///
    /// Deliberately conservative — a name shared by TWO folder-bearing projects
    /// (two different repos at different paths that happen to share a name) is
    /// LEFT ALONE: those are legitimately distinct projects (identity = path,
    /// not name) and must never be merged. All-empty same-name groups are also
    /// left untouched for [`Self::mark_orphaned_projects`] to tag. Returns the
    /// number of phantoms merged away.
    pub async fn heal_duplicate_name_projects(&self) -> Result<u64, String> {
        let pairs = self.duplicate_name_phantom_pairs().await?;

        let mut healed = 0u64;
        for (phantom, survivor) in pairs {
            match self.merge_projects(&phantom, &survivor).await {
                Ok(()) => {
                    healed += 1;
                    tracing::info!(phantom = %phantom, survivor = %survivor,
                        "heal_duplicate_name_projects: merged 0-folder phantom into folder-bearing survivor");
                }
                Err(e) => tracing::warn!(phantom = %phantom, survivor = %survivor, error = %e,
                    "heal_duplicate_name_projects: merge failed"),
            }
        }
        Ok(healed)
    }

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

    /// Read-only detection counterpart to [`Self::heal_duplicate_name_projects`]:
    /// the phantom project ids that WOULD be merged into their folder-bearing
    /// survivor. Shares the candidate query; performs no mutation. Used by the
    /// index integrity audit's read-only (`doctor`) pass.
    pub async fn detect_duplicate_name_phantoms(&self) -> Result<Vec<uuid::Uuid>, String> {
        Ok(self.duplicate_name_phantom_pairs().await?.into_iter().map(|(phantom, _)| phantom).collect())
    }

    /// Self-heal Bug 3: re-absorb a `standalone` project root that was
    /// mis-scoped INSIDE an existing git repo (e.g. a moved `crates/*` sub-crate
    /// registered as its own project instead of a folder of the monorepo). For
    /// each standalone folder nested under a git-repo folder that belongs to a
    /// DIFFERENT project:
    ///
    /// 1. Its own (repo-relative-to-itself) nodes are dropped — the enclosing
    ///    repo re-indexes the subtree with repo-relative paths on its next
    ///    `ProcessGitFolder`, so no duplicate nodes survive. (Node deletion
    ///    cascades edges; it does NOT touch `activity.sessions`, which key on
    ///    `folder_id` — those are preserved.)
    /// 2. The folder row is re-classified `kind='folder'`, re-parented under the
    ///    repo, and re-pointed at the repo's project — so its code attributes to
    ///    the repo's project, exactly like `crates/hive-mind` used to.
    /// 3. When the mis-scoped project then lives ENTIRELY inside the repo, it is
    ///    folded into the repo's project via [`Self::merge_projects`] (moving any
    ///    sessions/memories, CASCADE-trimming derived rows, deleting the phantom).
    ///    A phantom that also owns unrelated folders elsewhere is left for
    ///    [`Self::mark_orphaned_projects`] rather than dragged in.
    ///
    /// Idempotent — once re-absorbed the candidate query returns nothing.
    /// Returns the number of roots re-absorbed.
    pub async fn heal_nested_standalone_roots(&self) -> Result<u64, String> {
        let pairs = self.nested_standalone_candidates().await?;

        let mut healed = 0u64;
        for (s_id, s_pid, g_id, g_pid, g_root, g_abs) in pairs {
            // 1. Drop the mis-scoped root's own nodes (repo re-indexes the subtree).
            if let Err(e) = self.delete_nodes_by_folder(&s_id).await {
                tracing::warn!(folder = %s_id, error = %e, "heal_nested_standalone_roots: delete_nodes_by_folder failed");
                continue;
            }
            // 2. Re-classify as a folder of the enclosing repo's project, under
            //    the repo's watch root (it may have been registered under another).
            if let Err(e) = sqlx_core::query::query(
                "UPDATE sensei.folders
                    SET kind = 'folder'::sensei.folder_kind,
                        parent_id = $2, project_id = $3, root_id = $4, modified_at = now()
                  WHERE id = $1",
            ).bind(s_id).bind(g_id).bind(g_pid).bind(g_root).execute(&self.pool).await {
                tracing::warn!(folder = %s_id, error = %e, "heal_nested_standalone_roots: re-attribute failed");
                continue;
            }
            // 3. Fold the phantom project into the repo's project when it lives
            //    entirely inside the repo (the folder above was already re-pointed
            //    to g_pid, so it no longer counts against s_pid).
            if let Some(s_pid) = s_pid.filter(|p| *p != g_pid) {
                let outside: (i64,) = sqlx_core::query_as::query_as(
                    "SELECT count(*) FROM sensei.folders
                      WHERE project_id = $1 AND NOT starts_with(abs_path, $2 || '/')",
                ).bind(s_pid).bind(&g_abs).fetch_one(&self.pool).await.unwrap_or((1,));
                if outside.0 == 0 {
                    match self.merge_projects(&s_pid, &g_pid).await {
                        Ok(()) => tracing::info!(phantom = %s_pid, survivor = %g_pid,
                            "heal_nested_standalone_roots: merged phantom project into enclosing repo's project"),
                        Err(e) => tracing::warn!(phantom = %s_pid, survivor = %g_pid, error = %e,
                            "heal_nested_standalone_roots: merge_projects failed"),
                    }
                } else {
                    tracing::info!(phantom = %s_pid, outside = outside.0,
                        "heal_nested_standalone_roots: phantom has folders outside the repo — left for orphan-tagging");
                }
            }
            healed += 1;
            tracing::info!(folder = %s_id, project = %g_pid, "heal_nested_standalone_roots: re-absorbed nested standalone root");
        }
        Ok(healed)
    }

    /// Enforce one-node-one-owner: a `folder`-kind (structural) subfolder must
    /// not carry a node that the project's canonical ROOT owner
    /// (git/standalone/subtree) already holds. Deletes each such duplicate —
    /// TWIN-GUARDED by identical `kind` AND a path-suffix match on BOTH
    /// `file_path` and `name`: the root node's repo-relative value must end in
    /// the structural node's subfolder-relative value (`right(...)` suffix, no
    /// LIKE wildcard hazard). For code symbols the `name` suffix collapses to an
    /// exact match (symbol names have no `/`, so the `'/' ||` separator can't
    /// spuriously match); for `file`/`module` nodes (where `name` is itself a
    /// path) the suffix catches the differing subfolder prefix. A node held
    /// UNIQUELY under a structural folder is therefore never removed — only
    /// proven duplicates are. Self-heals the pre-fix double-index residue (#101:
    /// members promoted to second index owners on 2026-07-13) and, run every
    /// scan, prevents future accumulation. Scoped to `root_id`. Edges cascade
    /// with the deleted nodes. Returns rows pruned.
    pub async fn dedup_structural_folder_nodes(&self, root_id: &uuid::Uuid) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "DELETE FROM sensei.nodes s
               USING sensei.folders sf
              WHERE s.folder_id = sf.id
                AND sf.kind IN ('folder'::sensei.folder_kind, 'workspace_member'::sensei.folder_kind)
                AND sf.root_id = $1
                AND EXISTS (
                  SELECT 1
                    FROM sensei.nodes g
                    JOIN sensei.folders gf ON gf.id = g.folder_id
                   WHERE gf.project_id = sf.project_id
                     AND gf.kind IN ('git'::sensei.folder_kind,
                                     'standalone'::sensei.folder_kind,
                                     'subtree'::sensei.folder_kind)
                     AND g.kind = s.kind
                     AND (g.name = s.name
                          OR right(g.name, char_length(s.name) + 1)
                             = ('/' || s.name))
                     AND (g.file_path = s.file_path
                          OR right(g.file_path, char_length(s.file_path) + 1)
                             = ('/' || s.file_path)))",
        )
        .bind(root_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("dedup_structural_folder_nodes: {e}"))?;
        Ok(res.rows_affected())
    }

    /// Populate framework-pattern tags on file nodes so `get_patterns` /
    /// `get_file_tags` return real files (they read `sensei.file_tags`, a view
    /// over `nodes.tags` for `kind='file'` — previously always empty).
    ///
    /// The signal is the classifier's own node kinds: a file is tagged with the
    /// framework kinds of the symbols it contains (`hook`, `component`). This
    /// reuses the existing per-node classification — no separate tagger — and
    /// recomputes the full set per file (so a file that loses its last hook is
    /// cleared), scoped to one watch root. Runs in the scan reconcile. Returns
    /// file nodes whose tags changed.
    pub async fn tag_file_nodes_by_framework_kind(&self, root_id: &uuid::Uuid) -> Result<u64, String> {
        // Tag each `file` node with the framework roles it plays, so `get_patterns`
        // / `get_file_tags` answer "which files are components / hooks / routes /
        // middleware". Two signals, merged into `tags` and recomputed each scan
        // (self-correcting — adds AND removes):
        //   • symbol-kind — the `hook`/`component` node-kinds the classifier emits
        //     for symbols the file contains.
        //   • file-role (path convention, per-framework) — a whole file that *is* a
        //     route or middleware. `route`/`middleware` aren't node kinds, they are
        //     file-level roles, so a path convention is the right per-adapter
        //     detector: SvelteKit `+page`/`+layout`/`+server`/`+error` + Next
        //     `page`/`route` → `route`; SvelteKit `hooks.{server,client}` + Next
        //     `middleware` → `middleware`.
        // A CTE computes the desired tag set once per file; only rows whose set
        // actually changes are written (idempotent, accurate `rows_affected`).
        let res = sqlx_core::query::query(
            r"WITH desired AS (
                SELECT fn.id,
                       COALESCE((
                         SELECT array_agg(DISTINCT tag ORDER BY tag) FROM (
                           SELECT s.kind::text AS tag
                             FROM sensei.nodes s
                            WHERE s.folder_id = fn.folder_id
                              AND s.file_path = fn.file_path
                              AND s.kind IN ('hook','component')
                           UNION ALL
                           SELECT 'route'
                            WHERE fn.file_path ~ '(^|/)\+(page|layout|server|error)\.'
                               OR fn.file_path ~ '(^|/)(page|route)\.(tsx?|jsx?)$'
                           UNION ALL
                           SELECT 'middleware'
                            WHERE fn.file_path ~ '(^|/)hooks\.(server|client)\.(tsx?|jsx?)$'
                               OR fn.file_path ~ '(^|/)hooks\.(tsx?|jsx?)$'
                               OR fn.file_path ~ '(^|/)middleware\.(tsx?|jsx?)$'
                         ) src
                       ), '{}') AS tags
                  FROM sensei.nodes fn
                  JOIN sensei.folders f ON f.id = fn.folder_id
                 WHERE f.root_id = $1
                   AND fn.kind = 'file'
              )
              UPDATE sensei.nodes n
                 SET tags = d.tags
                FROM desired d
               WHERE n.id = d.id
                 AND n.tags IS DISTINCT FROM d.tags",
        )
        .bind(root_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("tag_file_nodes_by_framework_kind: {e}"))?;
        Ok(res.rows_affected())
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

    /// Read-only detection counterpart to [`Self::heal_nested_standalone_roots`]:
    /// the abs_paths of standalone roots currently mis-scoped inside a git repo
    /// (what the heal WOULD re-absorb). Shares the candidate query; performs no
    /// mutation. Used by the index integrity audit's read-only (`doctor`) pass.
    pub async fn detect_nested_standalone_roots(&self) -> Result<Vec<String>, String> {
        Ok(self.nested_standalone_candidates().await?.into_iter().map(|c| c.5).collect())
    }

    pub async fn get_project_libraries(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        // Query the resolved view directly — it already joins libraries internally.
        // Extended for T3 Slice 1.5: pull `page_count` (indexed docs marker) and
        // `local_path` (workspace / local-source marker) so the Libraries page
        // can render "wrapped by sensei" and "local source" badges without a
        // second round-trip.
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            uuid::Uuid, String, String, Option<String>, bool,
            serde_json::Value, String, i32, Option<String>,
        )> = sqlx_core::query_as::query_as(
                "SELECT id, name, ecosystem::text, description, enabled,
                        project_props, scope, page_count, local_path
                 FROM sensei.project_libraries_resolved
                 WHERE (scoped_project_id = $1 OR scoped_project_id IS NULL)
                   AND enabled = true
                 ORDER BY scope DESC, name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, ecosystem, desc, enabled, props, scope, page_count, local_path)| {
            serde_json::json!({
                "id":            id,
                "name":          name,
                "ecosystem":     ecosystem,
                "description":   desc,
                "enabled":       enabled,
                "project_props": props,
                "scope":         scope,
                "hasDocs":       page_count > 0,
                "pageCount":     page_count,
                "localSource":   local_path,
            })
        }).collect())
    }

    /// List libraries pinned to different versions across folders of a project.
    ///
    /// Reads `sensei.project_library_version_conflicts` — excludes local-
    /// protocol deps so only registry-version drift surfaces. Returns one row
    /// per conflicting (project, library) pair with the distinct versions and
    /// the folders where each version was seen.
    pub async fn list_project_library_version_conflicts(
        &self, project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Vec<String>, Vec<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT library_id, library_name, ecosystem, versions, folders
                   FROM sensei.project_library_version_conflicts
                  WHERE project_id = $1
                  ORDER BY library_name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(lib_id, name, ecosystem, versions, folders)| {
            serde_json::json!({
                "library_id": lib_id,
                "library_name": name,
                "ecosystem": ecosystem,
                "versions": versions,
                "folders": folders,
            })
        }).collect())
    }

    /// List outgoing project → project edges for a project.
    ///
    /// Returns one row per edge with the target project's name joined in.
    /// Sorted by target project name for stable UI ordering.
    pub async fn list_project_dependencies(
        &self, project_id: &uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, uuid::Uuid, String, String, Option<String>, String)> =
            sqlx_core::query_as::query_as(
                "SELECT to_p.id, to_p.name, pd.from_folder_id, pd.source_protocol,
                        pd.source_manifest, pd.resolved_target, from_f.name
                   FROM sensei.project_dependencies pd
                   JOIN sensei.projects to_p   ON to_p.id   = pd.to_project_id
                   JOIN sensei.folders  from_f ON from_f.id = pd.from_folder_id
                  WHERE pd.from_project_id = $1
                  ORDER BY to_p.name, from_f.name, pd.source_manifest"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(to_id, to_name, from_folder_id, protocol, manifest, target, from_folder_name)| {
            serde_json::json!({
                "to_project_id": to_id,
                "to_project_name": to_name,
                "from_folder_id": from_folder_id,
                "from_folder": from_folder_name,
                "source_protocol": protocol,
                "source_manifest": manifest,
                "resolved_target": target,
            })
        }).collect())
    }

    pub async fn get_project_extensions(&self, project_id: &uuid::Uuid, kind_filter: Option<&[&str]>) -> Result<Vec<serde_json::Value>, String> {
        // Query the resolved view directly — it already joins extensions internally
        let rows: Vec<(uuid::Uuid, String, String, bool, serde_json::Value, String)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, kind::text, enabled, project_props, scope
                 FROM sensei.project_extensions_resolved
                 WHERE (scoped_project_id = $1 OR scoped_project_id IS NULL)
                   AND enabled = true
                 ORDER BY scope DESC, name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter()
            .filter(|(_, _, kind, _, _, _)| {
                kind_filter.is_none_or(|f| f.contains(&kind.as_str()))
            })
            .map(|(id, name, kind, enabled, props, scope)| {
                serde_json::json!({
                    "id": id, "name": name, "kind": kind,
                    "enabled": enabled, "project_props": props, "scope": scope,
                })
            }).collect())
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

    pub async fn get_project_ftr(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        // Headline re-derived from the daily `ftr` rows in
        // `sensei.project_metric_daily` (metric='ftr') — the single FTR source of
        // truth. `ftr14d` reuses [`Self::get_project_ftr_rate`] (same 14d Σnum/Σden)
        // so the window can't drift between the two; `ftr14dPrev` is the same
        // pooled ratio over the prior-14d window and `sessions7d` is Σdenominator
        // over 7d. Scoped to the analyzed base (`outcome is not null`, the store's
        // denominator); `nullif(...,0)` keeps an empty window honest-null.
        let ftr_14d = self.get_project_ftr_rate(project_id).await?;
        let (ftr_14d_prev, sessions_7d): (Option<f64>, i64) =
            sqlx_core::query_as::query_as(
                "SELECT
                   (sum((props->>'numerator')::float8) FILTER (WHERE date > current_date - 28 AND date <= current_date - 14)
                      / nullif(sum((props->>'denominator')::float8) FILTER (WHERE date > current_date - 28 AND date <= current_date - 14), 0))::float8,
                   coalesce(sum((props->>'denominator')::int8) FILTER (WHERE date > current_date - 7), 0)::int8
                 FROM sensei.project_metric_daily
                 WHERE metric = 'ftr' AND project_id = $1"
            ).bind(project_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;

        // 14-day daily trend array — reads activity.sessions directly, filtered to
        // the SAME analyzed base as the headline (`outcome is not null`) so the
        // last trend point agrees with `ftr14d` for a day with in-flight sessions.
        let daily: Vec<(chrono::NaiveDate, Option<f64>)> =
            sqlx_core::query_as::query_as(
                "SELECT date_trunc('day', started_at)::date AS day,
                        AVG(CASE WHEN ftr THEN 1.0 ELSE 0.0 END)::float8 AS daily_ftr
                 FROM activity.sessions
                 WHERE project_id = $1 AND outcome IS NOT NULL AND started_at > now() - interval '14d'
                 GROUP BY day ORDER BY day"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let trend: Vec<f64> = daily.into_iter().map(|(_, v)| v.unwrap_or(0.0)).collect();

        Ok(Self::ftr_headline_json(ftr_14d, ftr_14d_prev, trend, sessions_7d))
    }

    /// The project's 14-day session-weighted FTR — Σ(`props.numerator`) /
    /// Σ(`props.denominator`) over the daily `ftr` rows in
    /// `sensei.project_metric_daily` (metric='ftr'), the single FTR source of
    /// truth. Same 14d window and derivation as [`Self::get_project_ftr`]'s
    /// `ftr14d` (which calls this), so both agree. Returns `None` when the project
    /// has no `ftr` rows in the window — honest-absent, NEVER a fabricated `0`.
    /// Shared by the legacy `/api/metrics/{project}` route and the MCP
    /// `get_metrics` tool so those surfaces report the same number.
    pub async fn get_project_ftr_rate(&self, project_id: &uuid::Uuid) -> Result<Option<f64>, String> {
        let row: (Option<f64>,) = sqlx_core::query_as::query_as(
            "SELECT (sum((props->>'numerator')::float8)
                       / nullif(sum((props->>'denominator')::float8), 0))::float8
               FROM sensei.project_metric_daily
              WHERE metric = 'ftr' AND project_id = $1 AND date > current_date - 14"
        ).bind(project_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Holistic First-Try-Right rollup across all sessions — powers the
    /// Observatory · Today header. Mirrors [`Self::get_project_ftr`] without the
    /// project filter: the 14d / prior-14d headline is session-weighted
    /// (fraction of FTR-scored sessions, honest-null when there are none), and
    /// the trend is a fixed 14 calendar-day array (0-filled on empty days) so the
    /// sparkline always has 14 points.
    pub async fn get_holistic_ftr(&self) -> Result<serde_json::Value, String> {
        let row: (Option<f64>, Option<f64>, i64) = sqlx_core::query_as::query_as(
            "SELECT
               (avg(CASE WHEN ftr THEN 1.0 ELSE 0.0 END)
                  FILTER (WHERE ftr IS NOT NULL AND started_at > now() - interval '14 days'))::float8,
               (avg(CASE WHEN ftr THEN 1.0 ELSE 0.0 END)
                  FILTER (WHERE ftr IS NOT NULL
                          AND started_at <= now() - interval '14 days'
                          AND started_at >  now() - interval '28 days'))::float8,
               count(*) FILTER (WHERE started_at > now() - interval '7 days')
             FROM activity.sessions"
        ).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        let (ftr_14d, ftr_14d_prev, sessions_7d) = row;

        // Exactly 14 calendar-day trend points, oldest → newest, 0-filled on
        // days with no FTR-scored session.
        let daily: Vec<(chrono::NaiveDate, Option<f64>)> = sqlx_core::query_as::query_as(
            "SELECT d::date,
                    (SELECT avg(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)::float8
                       FROM activity.sessions s
                      WHERE date_trunc('day', s.started_at)::date = d::date
                        AND s.ftr IS NOT NULL)
             FROM generate_series(current_date - 13, current_date, interval '1 day') d
             ORDER BY d"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        let trend: Vec<f64> = daily.into_iter().map(|(_, v)| v.unwrap_or(0.0)).collect();

        Ok(Self::ftr_headline_json(ftr_14d, ftr_14d_prev, trend, sessions_7d))
    }

    pub async fn get_project_drift(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        // `expected_signature` and `actual_signature` power the Traceability
        // detail drawer's Expected-vs-Actual diff. Both are nullable — `broken`
        // rows carry no `actual`, `drifted` carries both, `current` may carry
        // neither depending on how the detector wrote the row.
        type DriftRow = (
            uuid::Uuid, String, Option<String>, Option<String>, Option<String>,
            chrono::DateTime<chrono::Utc>,
        );
        let rows: Vec<DriftRow> = sqlx_core::query_as::query_as(
                "SELECT id, status::text, detail, expected_signature, actual_signature, detected_at
                 FROM sensei.project_drift WHERE project_id = $1
                 ORDER BY detected_at DESC LIMIT 200"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let total = rows.len();
        let drifted = rows.iter().filter(|r| r.1 == "drifted").count();
        let broken = rows.iter().filter(|r| r.1 == "broken").count();
        let items: Vec<_> = rows.into_iter().map(|(id, status, detail, expected, actual, detected_at)| {
            serde_json::json!({
                "id": id, "status": status, "detail": detail,
                "expectedSignature": expected,
                "actualSignature":   actual,
                "detectedAt": detected_at.to_rfc3339(),
            })
        }).collect();

        Ok(serde_json::json!({ "items": items, "total": total, "drifted": drifted, "broken": broken }))
    }

    pub async fn get_project_patterns(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        // Project baseline FTR — average First-Try-Right across the project's
        // FTR-scored sessions. `ftrDelta` per pattern is its folder's FTR minus this.
        let project_ftr_row: (Option<f64>,) = sqlx_core::query_as::query_as(
            "SELECT avg(CASE WHEN ftr THEN 1.0 ELSE 0.0 END)::float8
             FROM activity.sessions WHERE project_id = $1 AND ftr IS NOT NULL"
        ).bind(project_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        let project_ftr: Option<f64> = project_ftr_row.0;

        // Each pattern + its folder's average FTR (locus signal).
        // confidence is nullable (correction-prone / rule-candidate patterns set
        // no confidence) — decode as Option to avoid a NULL→f64 decode failure.
        // description / example / enforcement are exposed here (previously
        // dropped) so the Patterns screen can render the guidance the analyzer
        // captured with each pattern (T3 Slice 2.1).
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            uuid::Uuid,      // id
            String,          // name
            Option<String>,  // family
            bool,            // is_anti_pattern
            String,          // lifecycle
            Option<f64>,     // confidence
            i32,             // instance_count
            Option<f64>,     // folder_ftr
            Option<String>,  // description
            Option<String>,  // example
            Option<String>,  // enforcement
        )> = sqlx_core::query_as::query_as(
                "SELECT pp.id, pp.name, pp.family, pp.is_anti_pattern, pp.lifecycle::text,
                        pp.confidence::float8, pp.instance_count,
                        (SELECT avg(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)::float8
                           FROM activity.sessions s
                          WHERE s.folder_id = pp.folder_id AND s.ftr IS NOT NULL) AS folder_ftr,
                        pp.description, pp.example, pp.enforcement
                 FROM sensei.project_patterns pp WHERE pp.project_id = $1
                 ORDER BY pp.is_anti_pattern, pp.name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let (followed, anti): (Vec<_>, Vec<_>) = rows.into_iter().partition(|r| !r.3);
        let map_row = |(id, name, family, is_anti, lifecycle, confidence, count, folder_ftr, description, example, enforcement): (uuid::Uuid, String, Option<String>, bool, String, Option<f64>, i32, Option<f64>, Option<String>, Option<String>, Option<String>)| {
            let kind = crate::pattern_effectiveness::pattern_kind(is_anti, &lifecycle);
            let ftr_delta = crate::pattern_effectiveness::ftr_delta(folder_ftr, project_ftr);
            serde_json::json!({
                "id":            id,
                "name":          name,
                "family":        family,
                "isAntiPattern": is_anti,
                "lifecycle":     lifecycle,
                "confidence":    confidence,
                "instanceCount": count,
                "kind":          kind,
                "ftrDelta":      ftr_delta,
                "description":   description,
                "example":       example,
                "enforcement":   enforcement,
            })
        };
        Ok(serde_json::json!({
            "followed": followed.into_iter().map(map_row).collect::<Vec<_>>(),
            "antiPatterns": anti.into_iter().map(map_row).collect::<Vec<_>>(),
        }))
    }

    pub async fn get_project_memories(&self, project_id: &uuid::Uuid) -> Result<serde_json::Value, String> {
        // `strength` is `real` (Postgres 4-byte float) — sqlx decodes it as
        // `f32`, not `f64`. A mismatched decode-target quietly failed the
        // whole query and made the endpoint 500. `last_relevant_at` is
        // likewise nullable for freshly minted memories that haven't been
        // reinforced or violated, so decode as Option so a NULL doesn't
        // fail the row.
        //
        // `content`, `impact`, and the two counts power the Memory Anatomy
        // detail drawer (What / Because / Consequence + evidence). Cheap
        // to project — all existing columns on `sensei.memories`.
        // `generalised` / `generalised_content` power the ready-to-share lane:
        // the flag says sensei has rewritten this memory project-agnostic, and
        // `generalised_content` carries that portable rewrite (null until then).
        type MemRow = (
            uuid::Uuid, String, String, String, String, Option<String>,
            f32, i32, i32, String, Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            bool, Option<String>,
        );
        let rows: Vec<MemRow> = sqlx_core::query_as::query_as(
                "SELECT id, title, type::text, status::text, content, impact,
                        strength, reinforced_count, violated_count,
                        scope::text, scope_filter, last_relevant_at,
                        generalised, generalised_content
                 FROM sensei.memories WHERE project_id = $1
                 ORDER BY last_relevant_at DESC NULLS LAST LIMIT 100"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let total = rows.len();
        let active: Vec<_> = rows.into_iter()
            .filter(|r| r.3 == "active")
            .map(|(id, title, typ, status, content, impact, strength, reinforced, violated, scope, scope_filter, last, generalised, generalised_content)| {
                serde_json::json!({
                    "id": id, "title": title, "type": typ, "status": status,
                    "content": content, "impact": impact,
                    "strength": strength,
                    "reinforcedCount": reinforced,
                    "violatedCount": violated,
                    "scope": scope, "scopeFilter": scope_filter,
                    "lastRelevantAt": last.map(|t| t.to_rfc3339()),
                    "generalised": generalised,
                    "generalisedContent": generalised_content,
                })
            }).collect();

        Ok(serde_json::json!({ "active": active, "total": total }))
    }

    /// Return the paired PreToolUse / PostToolUse timeline for an assistant
    /// session, ordered by call start. Each row carries the request payload,
    /// the response payload (null when the call is still in-flight or the
    /// PostToolUse was dropped), the success flag, and duration_ms. Backed
    /// by the `sensei.session_tool_calls` view — see its DDL for the
    /// pairing rule.
    pub async fn get_session_tool_calls(
        &self,
        session_id: &str,
        limit: i32,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(
            i64,                                            // call_id
            String,                                         // tool_name
            String,                                         // family
            serde_json::Value,                              // request
            Option<serde_json::Value>,                      // response
            Option<bool>,                                   // success
            i64,                                            // started_at_ms
            Option<i64>,                                    // completed_at_ms
            Option<i64>,                                    // duration_ms
            chrono::DateTime<chrono::Utc>,                  // started_at
            Option<chrono::DateTime<chrono::Utc>>,          // completed_at
        )> = sqlx_core::query_as::query_as(
            "SELECT call_id, tool_name, family::text, request, response, success,
                    started_at_ms, completed_at_ms, duration_ms,
                    started_at, completed_at
               FROM sensei.session_tool_calls
              WHERE session_id = $1
              ORDER BY started_at_ms ASC
              LIMIT $2"
        )
        .bind(session_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(
            call_id, tool_name, family, request, response, success,
            started_at_ms, completed_at_ms, duration_ms,
            started_at, completed_at,
        )| {
            serde_json::json!({
                "callId":         call_id,
                "toolName":       tool_name,
                "family":         family,
                "request":        request,
                "response":       response,
                "success":        success,
                "startedAtMs":    started_at_ms,
                "completedAtMs":  completed_at_ms,
                "durationMs":     duration_ms,
                "startedAt":      started_at.to_rfc3339(),
                "completedAt":    completed_at.map(|t| t.to_rfc3339()),
                "inFlight":       completed_at_ms.is_none(),
            })
        }).collect())
    }

    /// List memory-share batches for a project, newest first. `only_status`
    /// filters to a single lifecycle stage (`proposed`, `approved`, …); pass
    /// `None` to include every stage.
    pub async fn list_memory_share_batches(
        &self,
        project_id: &uuid::Uuid,
        only_status: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>, i64)> =
            sqlx_core::query_as::query_as(
                "SELECT b.id, b.status::text, b.note, b.created_at, b.decided_at,
                        (SELECT count(*) FROM sensei.memory_share_batch_members m WHERE m.batch_id = b.id)::bigint
                   FROM sensei.memory_share_batches b
                  WHERE b.project_id = $1
                    AND ($2::text IS NULL OR b.status::text = $2)
                  ORDER BY b.created_at DESC
                  LIMIT 200"
            )
            .bind(project_id)
            .bind(only_status)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, status, note, created_at, decided_at, member_count)| {
            serde_json::json!({
                "id":          id,
                "status":      status,
                "note":        note,
                "createdAt":   created_at.to_rfc3339(),
                "decidedAt":   decided_at.map(|t| t.to_rfc3339()),
                "memberCount": member_count,
            })
        }).collect())
    }

    /// Create a new `proposed` memory-share batch with the given memory ids.
    /// Rejects an empty member list — a batch with nothing to share is a
    /// caller-side bug. Returns the new batch id on success.
    pub async fn create_memory_share_batch(
        &self,
        project_id: &uuid::Uuid,
        memory_ids: &[uuid::Uuid],
        note: Option<&str>,
    ) -> Result<uuid::Uuid, String> {
        if memory_ids.is_empty() {
            return Err("memory_ids must be non-empty".into());
        }
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        let (batch_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memory_share_batches (project_id, note)
             VALUES ($1, $2) RETURNING id"
        )
        .bind(project_id)
        .bind(note)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        // ON CONFLICT DO NOTHING is intentional — the composite PK guards
        // against duplicate members when a caller passes the same id twice.
        sqlx_core::query::query(
            "INSERT INTO sensei.memory_share_batch_members (batch_id, memory_id)
             SELECT $1, unnest($2::uuid[])
             ON CONFLICT DO NOTHING"
        )
        .bind(batch_id)
        .bind(memory_ids)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(batch_id)
    }

    /// Set a memory-share batch's terminal status. Accepts `approved`,
    /// `rejected`, or `withdrawn`. `approved` / `rejected` stamp
    /// `decided_at = now()`; `withdrawn` clears it (the batch was never
    /// decided). Errors when the batch is missing or already decided.
    pub async fn set_memory_share_batch_status(
        &self,
        batch_id: &uuid::Uuid,
        new_status: &str,
        note: Option<&str>,
    ) -> Result<(), String> {
        if !matches!(new_status, "approved" | "rejected" | "withdrawn") {
            return Err(format!("invalid status {new_status}"));
        }
        let decided_at_sql = if new_status == "withdrawn" { "NULL" } else { "now()" };
        let sql = format!(
            "UPDATE sensei.memory_share_batches
                SET status = $1::sensei.memory_share_batch_status,
                    note = COALESCE($2, note),
                    decided_at = {decided_at_sql}
              WHERE id = $3
                AND status = 'proposed'"
        );
        let result = sqlx_core::query::query(&sql)
            .bind(new_status)
            .bind(note)
            .bind(batch_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        if result.rows_affected() == 0 {
            return Err("batch not found or already decided".into());
        }
        Ok(())
    }

    // ── Dōjō upstream contribute (C6) ─────────────────────────────────

    /// Load a share batch's `(project_id, status, member items)` for the C6
    /// upstream-contribute path. `status` is returned so the caller can enforce
    /// "only `approved` batches contribute". Each item's `body` is the
    /// `generalised_content` rewrite when present, else the raw `content`.
    pub async fn batch_share_items(
        &self,
        batch_id: &uuid::Uuid,
    ) -> Result<Option<(uuid::Uuid, String, Vec<ShareBatchItem>)>, String> {
        let head: Option<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT project_id, status::text FROM sensei.memory_share_batches WHERE id = $1")
            .bind(batch_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        let Some((project_id, status)) = head else { return Ok(None); };

        let rows: Vec<(uuid::Uuid, String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT m.id, m.title,
                    COALESCE(NULLIF(btrim(m.generalised_content), ''), m.content),
                    m.type::text
               FROM sensei.memory_share_batch_members mm
               JOIN sensei.memories m ON m.id = mm.memory_id
              WHERE mm.batch_id = $1
              ORDER BY m.title")
            .bind(batch_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let items = rows.into_iter()
            .map(|(memory_id, title, body, memory_type)| ShareBatchItem { memory_id, title, body, memory_type })
            .collect();
        Ok(Some((project_id, status, items)))
    }

    /// The membership a project is bound to (`sensei.projects.dojo_id`), or `None`
    /// when the project is unbound / unknown. The routing anchor for C6.
    pub async fn project_bound_membership(&self, project_id: &uuid::Uuid) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(Option<uuid::Uuid>,)> = sqlx_core::query_as::query_as(
            "SELECT dojo_id FROM sensei.projects WHERE id = $1")
            .bind(project_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.and_then(|(d,)| d))
    }

    /// The oldest `approved` share batch that still has at least one member memory
    /// with no `sent` outbox row — i.e. work the daemon still owes a Dōjō. Powers
    /// `GET /api/share-review/next-batch`. Returns `(batch_id, project_id,
    /// decided_at)`.
    pub async fn next_unsent_approved_batch(
        &self,
    ) -> Result<Option<(uuid::Uuid, uuid::Uuid, Option<String>)>, String> {
        let row: Option<(uuid::Uuid, uuid::Uuid, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT b.id, b.project_id, b.decided_at::text
               FROM sensei.memory_share_batches b
              WHERE b.status = 'approved'
                AND EXISTS (
                  SELECT 1 FROM sensei.memory_share_batch_members mm
                   WHERE mm.batch_id = b.id
                     AND NOT EXISTS (
                       SELECT 1 FROM sensei.dojo_outbox o
                        WHERE o.memory_id = mm.memory_id AND o.state = 'sent'))
              ORDER BY b.decided_at ASC NULLS LAST
              LIMIT 1")
            .fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// The stable local contributor key (a machine-local secret that NEVER leaves
    /// the machine — only its rotated hash does, via [`crate::collective::anonymize`]).
    /// Get-or-create in `sensei.config` under `collective.contributor_key`.
    pub async fn get_or_create_contributor_key(&self) -> Result<String, String> {
        const KEY: &str = "collective.contributor_key";
        if let Some(v) = self.get_config(KEY).await?
            && !v.trim().is_empty()
        {
            return Ok(v);
        }
        let fresh = uuid::Uuid::new_v4().to_string();
        self.set_config(KEY, &fresh).await?;
        Ok(fresh)
    }

    /// Has this artifact `signature` already been published to `membership_id`?
    /// The pre-send dedup check — a retry after a federation drop skips a row
    /// already `sent` rather than double-publishing.
    pub async fn outbox_already_sent(&self, membership_id: &uuid::Uuid, signature: &str) -> Result<bool, String> {
        let row: Option<(bool,)> = sqlx_core::query_as::query_as(
            "SELECT state = 'sent' FROM sensei.dojo_outbox WHERE membership_id = $1 AND signature = $2")
            .bind(membership_id).bind(signature).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(b,)| b).unwrap_or(false))
    }

    /// Record a successful publish (idempotent on the `(membership_id, signature)`
    /// dedup key — a repeat send just refreshes the assigned seq/id).
    pub async fn outbox_mark_sent(
        &self, membership_id: &uuid::Uuid, batch_id: Option<&uuid::Uuid>,
        memory_id: Option<&uuid::Uuid>, signature: &str, sent_seq: i64, remote_id: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.dojo_outbox
                (membership_id, batch_id, memory_id, signature, state, sent_seq, remote_id, last_attempt_at)
             VALUES ($1,$2,$3,$4,'sent',$5,$6,now())
             ON CONFLICT (membership_id, signature) DO UPDATE SET
               state = 'sent', batch_id = EXCLUDED.batch_id, memory_id = EXCLUDED.memory_id,
               sent_seq = EXCLUDED.sent_seq, remote_id = EXCLUDED.remote_id,
               last_attempt_at = now(), updated_at = now()")
            .bind(membership_id).bind(batch_id).bind(memory_id).bind(signature).bind(sent_seq).bind(remote_id)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Record a non-sent outbox state (`held` | `queued` | `error`). Never
    /// downgrades an already-`sent` row (the `WHERE` guard), so a late held/queued
    /// signal can't erase a successful publish.
    pub async fn outbox_mark_state(
        &self, membership_id: &uuid::Uuid, batch_id: Option<&uuid::Uuid>,
        memory_id: Option<&uuid::Uuid>, signature: &str, state: &str,
    ) -> Result<(), String> {
        if !matches!(state, "held" | "queued" | "error" | "pending") {
            return Err(format!("invalid outbox state {state}"));
        }
        sqlx_core::query::query(
            "INSERT INTO sensei.dojo_outbox
                (membership_id, batch_id, memory_id, signature, state, last_attempt_at)
             VALUES ($1,$2,$3,$4,$5,now())
             ON CONFLICT (membership_id, signature) DO UPDATE SET
               state = EXCLUDED.state, batch_id = EXCLUDED.batch_id, memory_id = EXCLUDED.memory_id,
               last_attempt_at = now(), updated_at = now()
             WHERE sensei.dojo_outbox.state <> 'sent'")
            .bind(membership_id).bind(batch_id).bind(memory_id).bind(signature).bind(state)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Dōjō downstream inbox (C7) — the DOWNSTREAM twin of the outbox above ──

    /// Mirror one pulled artifact into `sensei.dojo_inbox` as `pending`, deduped
    /// by `(membership_id, artifact_signature)`. Returns `true` when a NEW row was
    /// inserted, `false` when the artifact was already present in any state — so a
    /// re-pull is idempotent. scope/attribution ride as JSON text cast to jsonb
    /// (no sqlx json feature needed on the bind side).
    pub async fn upsert_dojo_inbox(&self, row: &crate::collective::inbox::InboxRow) -> Result<bool, String> {
        let scope = serde_json::to_string(&row.scope).map_err(|e| e.to_string())?;
        let attribution = serde_json::to_string(&row.attribution).map_err(|e| e.to_string())?;
        let inserted: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.dojo_inbox
                (membership_id, artifact_seq, artifact_signature, remote_id, kind, title, body, scope, attribution)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8::jsonb,$9::jsonb)
             ON CONFLICT (membership_id, artifact_signature) DO NOTHING
             RETURNING id")
            .bind(row.membership_id).bind(row.artifact_seq).bind(&row.signature).bind(&row.remote_id)
            .bind(&row.kind).bind(&row.title).bind(&row.body).bind(&scope).bind(&attribution)
            .fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(inserted.is_some())
    }

    /// Advance a membership's downstream pull cursor
    /// (`sensei.dojo_memberships.last_seq`).
    pub async fn set_dojo_pull_cursor(&self, membership_id: uuid::Uuid, cursor: i64) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.dojo_memberships SET last_seq = $2, updated_at = now() WHERE id = $1")
            .bind(membership_id).bind(cursor).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
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

    /// Load one inbox item by id.
    pub async fn get_dojo_inbox(&self, inbox_id: uuid::Uuid) -> Result<Option<crate::collective::inbox::InboxItem>, String> {
        let row: Option<(uuid::Uuid, uuid::Uuid, String, String, String, serde_json::Value, serde_json::Value,
                         String, Option<String>, Option<uuid::Uuid>, Option<String>, String)> =
            sqlx_core::query_as::query_as(&format!("{} WHERE id = $1", Self::DOJO_INBOX_SELECT))
            .bind(inbox_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(Self::map_dojo_inbox_row))
    }

    /// The daemon's downstream inbox across all memberships, ordered for the
    /// Upgrades list (pinned first, then newest; muted hidden unless
    /// `include_muted`). Reuses [`crate::collective::inbox::order_and_filter`] so
    /// the ordering contract has a single home.
    pub async fn list_dojo_inbox(&self, include_muted: bool) -> Result<Vec<crate::collective::inbox::InboxItem>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, String, String, String, serde_json::Value, serde_json::Value,
                       String, Option<String>, Option<uuid::Uuid>, Option<String>, String)> =
            sqlx_core::query_as::query_as(&format!("{} ORDER BY received_at DESC", Self::DOJO_INBOX_SELECT))
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        let items: Vec<_> = rows.into_iter().map(Self::map_dojo_inbox_row).collect();
        Ok(crate::collective::inbox::order_and_filter(items, include_muted))
    }

    /// Resolve a local project by name → its id (scope-match for a project-scoped
    /// artifact). `None` = no such project on this install.
    pub async fn resolve_project_by_name(&self, name: String) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.projects WHERE name = $1 LIMIT 1")
            .bind(&name).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// Land an Applied principle/pattern: insert the memory (reusing
    /// [`Self::insert_memory`] — the shared memory-insert, never reimplemented)
    /// and flip the inbox row to `applied` + `applied_memory_id`. On a failed
    /// mark, the just-inserted memory is compensatingly deleted so a retry cannot
    /// double-land (the two writes are not one transaction because the insert goes
    /// through the shared helper; the compensating delete preserves idempotency).
    pub async fn land_dojo_inbox_memory(&self, inbox_id: uuid::Uuid, m: &InsertMemory) -> Result<uuid::Uuid, String> {
        let memory_id = self.insert_memory(m).await?;
        if let Err(e) = self.mark_dojo_inbox_applied(inbox_id, memory_id).await {
            if let Err(de) = sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
                .bind(memory_id).execute(&self.pool).await
            {
                tracing::error!(error = %de, memory = %memory_id, "dojo inbox: compensating memory delete failed after mark-applied error");
            }
            return Err(e);
        }
        Ok(memory_id)
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

    /// Record why an Apply did not land (deferred kind / scope mismatch). The item
    /// stays `pending`.
    pub async fn set_dojo_inbox_note(&self, inbox_id: uuid::Uuid, note: String) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.dojo_inbox SET note = $2, updated_at = now() WHERE id = $1")
            .bind(inbox_id).bind(&note).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Set an inbox item's state (mute → `muted`, pin → `pinned`). Returns `false`
    /// when the id is unknown (drives a 404). Never lands anything.
    pub async fn set_dojo_inbox_state(&self, inbox_id: uuid::Uuid, state: &str) -> Result<bool, String> {
        if !matches!(state, "pending" | "applied" | "muted" | "pinned") {
            return Err(format!("invalid dojo_inbox state {state}"));
        }
        let res = sqlx_core::query::query(
            "UPDATE sensei.dojo_inbox SET state = $2, updated_at = now() WHERE id = $1")
            .bind(inbox_id).bind(state).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn ensure_test_project(&self, name: &str) -> Result<uuid::Uuid, String> {
        // Namespace fixtures under `_test:` so leaked rows are identifiable
        // (and filterable by the Projects screen) and never masquerade as real
        // projects. Find-or-create by name so repeated test runs reuse one row
        // instead of minting a fresh UUID each call (#34). Each fixture name is
        // owned by a single test, so the SELECT-then-INSERT is race-free here.
        let name = format!("_test:{name}");
        if let Some(row) = sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM sensei.projects WHERE name = $1"
        ).bind(&name).fetch_optional(&self.pool).await.map_err(|e| e.to_string())? {
            return Ok(row.0);
        }
        let id = uuid::Uuid::new_v4();
        sqlx_core::query::query(
            "INSERT INTO sensei.projects (id, name) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING"
        ).bind(id).bind(&name)
         .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub async fn insert_memory(&self, m: &InsertMemory) -> Result<uuid::Uuid, String> {
        let id: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memories
                (project_id, scope, scope_filter, type, title, content, impact,
                 tags, triage_signal, status, namespace_id, enforcement, origin, source_id,
                 spine_slot, feature)
             VALUES ($1, $2::sensei.memory_scope, $3, $4::sensei.memory_type, $5, $6, $7,
                     $8, $9, $10::sensei.memory_status, $11,
                     COALESCE($12::sensei.enforcement, 'recommended'::sensei.enforcement),
                     COALESCE($13, 'learned'), $14, $15::sensei.spine_slot, $16)
             RETURNING id"
        )
            .bind(m.project_id)
            .bind(&m.scope).bind(&m.scope_filter)
            .bind(&m.mtype).bind(&m.title).bind(&m.content).bind(&m.impact)
            .bind(&m.tags).bind(&m.triage_signal).bind(&m.status)
            .bind(m.namespace_id).bind(&m.enforcement).bind(&m.origin).bind(m.source_id)
            .bind(&m.spine_slot).bind(&m.feature)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(id.0)
    }

    /// Set the learnings-anatomy `category` on a memory (correctness/convention/
    /// pattern/preference). Separate from `insert_memory` so the existing
    /// callers (API, federation) need no change (#69).
    pub async fn set_memory_category(&self, id: &uuid::Uuid, category: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.memories SET category = $2::sensei.memory_category WHERE id = $1"
        ).bind(id).bind(category).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Inputs for the L3 maturity signal (#71): `(enriched_session_count,
    /// has_insights)`. `has_insights` is true once the analyzer has produced any
    /// recommendation or learned memory for the project.
    pub async fn get_project_maturity_inputs(&self, project_id: &uuid::Uuid) -> Result<(i64, bool), String> {
        let row: (i64, bool) = sqlx_core::query_as::query_as(
            "SELECT
               (SELECT count(*) FROM activity.sessions WHERE project_id = $1 AND analyzed_at IS NOT NULL),
               (EXISTS(SELECT 1 FROM inference.recommendations WHERE project_id = $1)
                OR EXISTS(SELECT 1 FROM sensei.memories WHERE project_id = $1 AND origin = 'learned'))"
        ).bind(project_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Aggregate maturity inputs across all sessions/projects — powers the
    /// Observatory · Today maturity gate. Mirrors
    /// [`Self::get_project_maturity_inputs`] without the project filter.
    pub async fn get_global_maturity_inputs(&self) -> Result<(i64, bool), String> {
        let row: (i64, bool) = sqlx_core::query_as::query_as(
            "SELECT
               (SELECT count(*) FROM activity.sessions WHERE analyzed_at IS NOT NULL),
               (EXISTS(SELECT 1 FROM inference.recommendations)
                OR EXISTS(SELECT 1 FROM sensei.memories WHERE origin = 'learned'))"
        ).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// True if a learned memory already sources `source_id` (a detected-pattern
    /// id). The L2 generator's idempotency guard for memories.
    pub async fn memory_exists_with_source(&self, source_id: &uuid::Uuid) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM sensei.memories WHERE source_id = $1 AND origin = 'learned')"
        ).bind(source_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Fetch the learned memory that sources `source_id` (a detected-pattern id),
    /// if any. Companion to [`Self::memory_exists_with_source`] — returns the id
    /// so a caller can act on the memory (e.g. record a challenge outcome when a
    /// recommendation built on the same pattern later regresses).
    pub async fn memory_id_by_source(&self, source_id: &uuid::Uuid) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.memories WHERE source_id = $1 AND origin = 'learned' LIMIT 1"
        ).bind(source_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// Challenge (weaken) the learned memory that sourced a now-regressed
    /// recommendation. Resolves `based_on.patterns[0]` → the convention memory
    /// (`source_id = pattern`), then records ONE `'violated'` memory_outcome so
    /// the `memory_outcome_apply` trigger does the actual strength/status math —
    /// no hand-rolled weakening here, DRY with the outcome pipeline.
    ///
    /// Idempotent: the outcome `context` carries a `rec:<id>` marker and the write
    /// is gated on that marker not already existing, so a rec that is somehow
    /// re-measured never penalises the same memory twice. Returns `Ok(true)` when
    /// a fresh violation was recorded, `Ok(false)` for the no-op paths (the rec
    /// has no source memory, was already challenged for this rec, or the memory is
    /// archived/rejected).
    pub async fn challenge_source_memory_for_rec(
        &self, rec_id: &uuid::Uuid, based_on_json: &str,
    ) -> Result<bool, String> {
        // A missing/empty/non-uuid `patterns[0]` → manual rec / no provenance → no-op.
        let Some(pattern_id) = Self::based_on_first_pattern(based_on_json) else {
            return Ok(false);
        };
        let Some(memory_id) = self.memory_id_by_source(&pattern_id).await? else {
            return Ok(false); // the rec's pattern never spawned a memory
        };
        let marker = format!("rec:{rec_id} regression");
        // Idempotency guard: skip if this rec already challenged this memory.
        let already: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM sensei.memory_outcomes
                            WHERE memory_id = $1 AND outcome = 'violated' AND context = $2)"
        ).bind(memory_id).bind(&marker).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        if already.0 {
            return Ok(false);
        }
        // The memory_outcome_apply trigger applies strength -= 0.7 and moves the
        // memory to challenged/archived. record_outcomes_batch skips archived/
        // rejected memories, so an empty `skipped` means a violation landed.
        let skipped = self.record_outcomes_batch(&[OutcomeRow {
            memory_id,
            session_id: None,
            outcome: "violated".to_string(),
            context: Some(marker),
        }]).await?;
        Ok(skipped.is_empty())
    }

    /// Learning-loop feedback (positive side): an accepted rec whose FTR IMPROVED
    /// after acceptance vindicates the memory that spawned it. Reinforce that
    /// source memory through the same `memory_outcome` pipeline the challenge path
    /// uses — recording an `applied` outcome fires the `memory_outcome_apply`
    /// trigger, which bumps `reinforced_count`, raises `strength`, and drives the
    /// promotion ladder (active → reinforced → battle_tested). This is the bridge
    /// that lets a proven recommendation promote its memory (closes G1→G2). Fires
    /// at most once per rec (idempotency marker). Mirror of
    /// [`Self::challenge_source_memory_for_rec`].
    pub async fn reinforce_source_memory_for_rec(
        &self, rec_id: &uuid::Uuid, based_on_json: &str,
    ) -> Result<bool, String> {
        // A missing/empty/non-uuid `patterns[0]` → manual rec / no provenance → no-op.
        let Some(pattern_id) = Self::based_on_first_pattern(based_on_json) else {
            return Ok(false);
        };
        let Some(memory_id) = self.memory_id_by_source(&pattern_id).await? else {
            return Ok(false); // the rec's pattern never spawned a memory
        };
        let marker = format!("rec:{rec_id} confirmed");
        // Idempotency guard: skip if this rec already reinforced this memory.
        let already: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM sensei.memory_outcomes
                            WHERE memory_id = $1 AND outcome = 'applied' AND context = $2)"
        ).bind(memory_id).bind(&marker).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        if already.0 {
            return Ok(false);
        }
        // The memory_outcome_apply trigger applies strength += 0.5 (capped 5.0),
        // reinforced_count += 1, and promotes to battle_tested at strength >= 4.0
        // with violated_count = 0.
        let skipped = self.record_outcomes_batch(&[OutcomeRow {
            memory_id,
            session_id: None,
            outcome: "applied".to_string(),
            context: Some(marker),
        }]).await?;
        Ok(skipped.is_empty())
    }

    /// Promote a proven memory to a higher (broader) scope: copy it as a
    /// `proposed` memory on `target_namespace_id` with `origin='promoted'` and
    /// `source_id` pointing back at the original. The copy lands in the triage
    /// queue — accepting it (set_memory_status proposed→active) is the approval
    /// gate, so a promotion never auto-applies at the new scope. Only an
    /// established source (active/reinforced/battle_tested) is promotable;
    /// returns Ok(None) otherwise. `enforcement` overrides the source's when set.
    pub async fn promote_memory(
        &self,
        source_id: uuid::Uuid,
        target_namespace_id: Option<uuid::Uuid>,
        enforcement: Option<&str>,
    ) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memories
                (project_id, scope, scope_filter, type, title, content, impact, tags,
                 status, namespace_id, enforcement, origin, source_id)
             SELECT project_id, scope, scope_filter, type, title, content, impact, tags,
                    'proposed'::sensei.memory_status,
                    $2,
                    COALESCE($3::sensei.enforcement, enforcement),
                    'promoted', $1
               FROM sensei.memories
              WHERE id = $1
                AND status IN ('active'::sensei.memory_status,
                               'reinforced'::sensei.memory_status,
                               'battle_tested'::sensei.memory_status)
             RETURNING id"
        )
            .bind(source_id)
            .bind(target_namespace_id)
            .bind(enforcement)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// Persist the project-agnostic rewrite of a memory and flag it
    /// ready-to-share. Sets `generalised_content`, `generalised = true`, and
    /// bumps `modified_at`. Returns the id when a row was updated, `None` when
    /// no memory matched. Never panics — a DB error surfaces as `Err` for the
    /// caller to log; the caller only sets the flag on success (never fabricated).
    pub async fn set_memory_generalisation(
        &self,
        id: uuid::Uuid,
        generalised: &str,
    ) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "UPDATE sensei.memories
                SET generalised_content = $2
                  , generalised         = true
                  , modified_at         = now()
              WHERE id = $1
              RETURNING id"
        )
            .bind(id)
            .bind(generalised)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// Memories that have proven themselves (`battle_tested`) and have not
    /// already been promoted — the candidates a UI surfaces for "promote to a
    /// broader scope".
    pub async fn list_promotion_candidates(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<uuid::Uuid>, String)> =
            sqlx_core::query_as::query_as(
                "SELECT m.id, m.title, m.content, m.namespace_id, m.enforcement::text
                   FROM sensei.memories m
                  WHERE m.status = 'battle_tested'::sensei.memory_status
                    AND NOT EXISTS (
                          SELECT 1 FROM sensei.memories c WHERE c.source_id = m.id
                    )
                  ORDER BY m.strength DESC, m.modified_at DESC",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, content, ns, enforcement)| {
            serde_json::json!({ "id": id, "title": title, "content": content,
                "namespace_id": ns, "enforcement": enforcement })
        }).collect())
    }

    pub async fn list_memories(
        &self,
        project_id: Option<uuid::Uuid>,
        status:     Option<&str>,
        scope:      Option<&str>,
        limit:      i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, String, String,
                       Option<String>, f64, String, i32, i32,
                       Option<chrono::DateTime<chrono::Utc>>, Vec<String>, Option<String>,
                       chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, project_id, scope::text, scope_filter, type::text, title, content,
                        impact, strength::float8, status::text, reinforced_count, violated_count,
                        last_relevant_at, tags, triage_signal, modified_at
                   FROM sensei.memories
                  WHERE ($1::uuid IS NULL OR project_id = $1)
                    AND ($2::text IS NULL OR status::text = $2)
                    AND ($3::text IS NULL OR scope::text = $3)
                  ORDER BY strength DESC, last_relevant_at DESC NULLS LAST, modified_at DESC
                  LIMIT $4"
            )
            .bind(project_id).bind(status).bind(scope).bind(limit)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|r| serde_json::json!({
            "id":               r.0,
            "project_id":       r.1,
            "scope":            r.2,
            "scope_filter":     r.3,
            "type":             r.4,
            "title":            r.5,
            "content":          r.6,
            "impact":           r.7,
            "strength":         r.8,
            "status":           r.9,
            "applied_count":    r.10,
            "violated_count":   r.11,
            "last_relevant_at": r.12.map(|t| t.to_rfc3339()),
            "tags":             r.13,
            "triage_signal":    r.14,
            "modified_at":      r.15.to_rfc3339(),
        })).collect())
    }

    /// Transition a memory's status, only when its current status is in `from_states`.
    /// Returns the new status if the transition happened, None if no row matched.
    pub async fn set_memory_status(
        &self,
        memory_id: uuid::Uuid,
        to_status: &str,
        from_states: &[&str],
    ) -> Result<Option<String>, String> {
        let from_owned: Vec<String> = from_states.iter().map(|s| s.to_string()).collect();
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "UPDATE sensei.memories
                SET status      = $1::sensei.memory_status
                  , modified_at = now()
              WHERE id = $2
                AND status::text = ANY($3)
              RETURNING status::text"
        )
            .bind(to_status).bind(memory_id).bind(&from_owned)
            .fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    /// Rolling 7-day telemetry for one memory: `(loaded, followed, skipped)`.
    /// - `loaded`   = load events in `activity.memory_loads` (injected into context)
    /// - `followed` = `memory_outcomes` with outcome `applied` (used in output)
    /// - `skipped`  = `memory_outcomes` with outcome `ignored` (loaded but discarded)
    ///
    /// `consulted`/`violated` are deliberately NOT folded into followed/skipped.
    /// One round-trip via scalar subqueries (loads and outcomes live in different
    /// tables) — fewer round-trips than three separate readers.
    pub async fn memory_telemetry_7d(&self, memory_id: uuid::Uuid) -> Result<(i64, i64, i64), String> {
        let row: (i64, i64, i64) = sqlx_core::query_as::query_as(
            "SELECT
               (SELECT count(*) FROM activity.memory_loads
                 WHERE memory_id = $1 AND loaded_at   > now() - interval '7 days'),
               (SELECT count(*) FROM sensei.memory_outcomes
                 WHERE memory_id = $1 AND outcome = 'applied' AND recorded_at > now() - interval '7 days'),
               (SELECT count(*) FROM sensei.memory_outcomes
                 WHERE memory_id = $1 AND outcome = 'ignored' AND recorded_at > now() - interval '7 days')"
        )
            .bind(memory_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Full memory detail bundle: row + evidence + examples + recent outcomes.
    pub async fn get_memory_detail(&self, id: uuid::Uuid) -> Result<serde_json::Value, String> {
        let row: Option<(uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, String, String,
                         Option<String>, f64, String, i32, i32,
                         Option<chrono::DateTime<chrono::Utc>>, Vec<String>, Option<String>,
                         chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, project_id, scope::text, scope_filter, type::text, title, content,
                        impact, strength::float8, status::text, reinforced_count, violated_count,
                        last_relevant_at, tags, triage_signal, modified_at
                   FROM sensei.memories WHERE id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        let r = row.ok_or_else(|| format!("memory {id} not found"))?;
        // category + created_at fetched separately (the main row tuple is at
        // sqlx's 16-element FromRow limit).
        let (category, created_at): (Option<String>, chrono::DateTime<chrono::Utc>) =
            sqlx_core::query_as::query_as(
                "SELECT category::text, created_at FROM sensei.memories WHERE id = $1"
            ).bind(id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        let memory = serde_json::json!({
            "id":               r.0,
            "project_id":       r.1,
            "scope":            r.2,
            "scope_filter":     r.3,
            "type":             r.4,
            "title":            r.5,
            "content":          r.6,
            "impact":           r.7,
            "strength":         r.8,
            "status":           r.9,
            "applied_count":    r.10,
            "violated_count":   r.11,
            "last_relevant_at": r.12.map(|t| t.to_rfc3339()),
            "tags":             r.13,
            "triage_signal":    r.14,
            "modified_at":      r.15.to_rfc3339(),
            "category":         category,
            "created_at":       created_at.to_rfc3339(),
        });

        // Related memories (the anatomy "related" links — both directions).
        let related: Vec<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT child_id  FROM sensei.memory_links WHERE parent_id = $1
             UNION
             SELECT parent_id FROM sensei.memory_links WHERE child_id = $1"
        ).bind(id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        // Evidence — table has: session_id, note, modified_at (no url column).
        let evidence: Vec<(Option<uuid::Uuid>, Option<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT session_id, note, modified_at
                   FROM sensei.memory_evidence
                  WHERE memory_id = $1
                  ORDER BY modified_at DESC"
            ).bind(id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        // Examples — table has: node_id, is_good (non-nullable), note. No is_bad column.
        let examples: Vec<(Option<String>, bool, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT node_id, is_good, note
                   FROM sensei.memory_examples
                  WHERE memory_id = $1"
            ).bind(id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        // Last 20 outcomes
        let outcomes: Vec<(String, Option<uuid::Uuid>, Option<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT outcome::text, session_id, context, recorded_at
                   FROM sensei.memory_outcomes
                  WHERE memory_id = $1
                  ORDER BY recorded_at DESC
                  LIMIT 20"
            ).bind(id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        // Rolling 7-day telemetry ("did injected memory help?"): loaded / followed
        // / skipped. Additive to the lifetime applied_count/violated_count on the
        // memory row above.
        let (loaded_7d, followed_7d, skipped_7d) = self.memory_telemetry_7d(id).await?;

        Ok(serde_json::json!({
            "memory":   memory,
            "loaded_last_7d":   loaded_7d,
            "followed_last_7d": followed_7d,
            "skipped_last_7d":  skipped_7d,
            "evidence": evidence.into_iter().map(|(session_id, note, ts)|
                serde_json::json!({ "session_id": session_id, "note": note, "recorded_at": ts.to_rfc3339() })
            ).collect::<Vec<_>>(),
            "examples": examples.into_iter().map(|(node, is_good, note)|
                serde_json::json!({ "node_id": node, "is_good": is_good, "note": note })
            ).collect::<Vec<_>>(),
            "outcomes": outcomes.into_iter().map(|(outcome, sess, ctx, ts)|
                serde_json::json!({ "outcome": outcome, "session_id": sess, "context": ctx, "recorded_at": ts.to_rfc3339() })
            ).collect::<Vec<_>>(),
            "related": related.into_iter().map(|(rid,)| rid).collect::<Vec<_>>(),
        }))
    }

    /// Assemble a blended context blob: project-scoped + stack-scoped + global memories.
    /// Only active/reinforced/battle_tested/challenged memories are included.
    /// Governance Tier-1 resolution: the active rules that apply to a repo,
    /// ordered strongest-first. A rule applies when it sits on one of the repo's
    /// member namespaces (`folder_namespaces`), on an always-on `general`/`user`
    /// scope, is genuinely global (unscoped **and** not tied to a project —
    /// `namespace_id IS NULL AND project_id IS NULL`), or is a project-tied
    /// learned convention for **this repo's own project** (`namespace_id IS NULL
    /// AND project_id = the folder's project`). The last clause is what keeps a
    /// project's learned principle scoped to that project instead of bleeding
    /// into every repo's always-on `general` set: an unscoped memory carrying a
    /// `project_id` is that project's convention, not a global rule. Ordering is
    /// the two-axis precedence — enforcement desc (mandatory first), then scope
    /// level desc (most-specific first), then strength. Structuring (dedup +
    /// mandatory-lock) is done by `crate::governance::structure_ruleset` so it
    /// stays pure.
    pub async fn resolve_rules_raw(&self, folder_id: &uuid::Uuid) -> Result<Vec<crate::governance::RawRule>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT m.id, m.title, m.content, m.impact, m.enforcement::text,
                        COALESCE(n.scope_key,
                                 CASE WHEN m.project_id IS NOT NULL THEN 'project' ELSE 'general' END) AS scope,
                        n.name AS namespace
                   FROM sensei.memories m
                   LEFT JOIN sensei.namespaces n ON n.id = m.namespace_id
                   LEFT JOIN sensei.scopes s ON s.key = n.scope_key
                  WHERE m.status IN ('active'::sensei.memory_status,
                                     'reinforced'::sensei.memory_status,
                                     'battle_tested'::sensei.memory_status)
                    AND ( (m.namespace_id IS NULL AND m.project_id IS NULL)
                          OR n.scope_key IN ('general', 'user')
                          OR m.namespace_id IN (
                                SELECT namespace_id FROM sensei.folder_namespaces WHERE folder_id = $1
                          )
                          OR ( m.namespace_id IS NULL
                               AND m.project_id = (SELECT project_id FROM sensei.folders WHERE id = $1) ) )
                  ORDER BY m.enforcement DESC,
                           COALESCE(n.level, s.level, 0) DESC,
                           m.strength DESC",
            )
            .bind(folder_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, content, impact, enforcement, scope, namespace)| {
            crate::governance::RawRule {
                id: id.to_string(), title, content, impact, enforcement, scope, namespace,
            }
        }).collect())
    }

    /// The LOCAL authoritative raw ruleset for a folder — resolved memories
    /// ([`Self::resolve_rules_raw`]) plus adopted LOCAL rule-pack rules
    /// ([`Self::resolve_local_pack_raws`]), memories strongest-first then packs.
    /// This is the offline constitution the editor resolves; the dōjō
    /// constitution federation composes it into a preview. Fails closed on either
    /// read — a DB error must never silently drop governance. The remote Dōjō pack
    /// fold-in is layered by the api-handler resolver (needs `AppState` + network),
    /// not here, so a task-context federation stays offline.
    pub async fn resolve_repo_raw_local(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<crate::governance::RawRule>, String> {
        let mut raw = self.resolve_rules_raw(folder_id).await?;
        raw.extend(self.resolve_local_pack_raws(Some(folder_id)).await?);
        Ok(raw)
    }

    /// The folder (repo) a run's project maps to
    /// (`sensei.folders.project_id = activity.runs.project_id`). Lets the
    /// constitution federation resolve the run's ruleset. `None` when the run has
    /// no project or no folder is indexed for it.
    pub async fn run_folder_id(&self, run_id: &uuid::Uuid) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT f.id
               FROM activity.runs r
               JOIN sensei.folders f ON f.project_id = r.project_id
              WHERE r.id = $1
              LIMIT 1",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// The rules of rule packs adopted at a folder's namespaces (or at the
    /// always-on general/user scopes) resolved from the LOCAL `sensei.rule_packs`
    /// replica (D-LOCAL-PACKS) — offline, in tandem with the remote Dōjō fold-in.
    /// Pass `Some(folder)` for a repo's ruleset; pass `None` for the always-on
    /// GLOBAL set (`~/.sensei/rules.md`), where a NULL bind makes the folder
    /// clause match nothing, leaving only the general/user adoptions.
    /// Effective tier is never-weaken: an adoption override can only RAISE a rule's
    /// enforcement, never lower it (ranked in SQL so the enum's storage order does
    /// not matter). Maps to `RawRule`: scope = the GOVERNANCE scope the pack was
    /// ADOPTED at (the adoption namespace's `scope_key` — general/user/project/…, as
    /// `resolve_rules_raw` does for memories), NOT the pack's own area/category, so
    /// the constitution ladder groups pack rules on the same scope axis as memories;
    /// namespace = the pack source.
    pub async fn resolve_local_pack_raws(
        &self,
        folder_id: Option<&uuid::Uuid>,
    ) -> Result<Vec<crate::governance::RawRule>, String> {
        let rows: Vec<(String, String, String, Option<String>, String, String, String)> =
            sqlx_core::query_as::query_as(
                "SELECT r.id::text, r.statement, r.body, r.rationale,
                        CASE WHEN a.enforcement IS NULL THEN r.enforcement::text
                             WHEN (CASE a.enforcement::text WHEN 'advisory' THEN 1 WHEN 'recommended' THEN 2 WHEN 'required' THEN 3 WHEN 'mandatory' THEN 4 ELSE 0 END)
                                > (CASE r.enforcement::text WHEN 'advisory' THEN 1 WHEN 'recommended' THEN 2 WHEN 'required' THEN 3 WHEN 'mandatory' THEN 4 ELSE 0 END)
                             THEN a.enforcement::text ELSE r.enforcement::text END,
                        COALESCE(n.scope_key, 'general'),
                        p.source
                   FROM sensei.rule_pack_adoptions a
                   JOIN sensei.rule_packs p      ON p.id = a.pack_id
                   JOIN sensei.rule_pack_rules r ON r.pack_id = p.id
                   LEFT JOIN sensei.namespaces n ON n.id = a.namespace_id
                  WHERE a.namespace_id IN (
                            SELECT namespace_id FROM sensei.folder_namespaces WHERE folder_id = $1)
                     OR a.namespace_id IN (
                            SELECT id FROM sensei.namespaces WHERE scope_key IN ('general', 'user'))
                  ORDER BY r.ordinal",
            )
            .bind(folder_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(id, title, content, impact, enforcement, scope, source)| {
                crate::governance::RawRule {
                    id,
                    title,
                    content,
                    impact,
                    enforcement,
                    scope,
                    namespace: if source.is_empty() { None } else { Some(source) },
                }
            })
            .collect())
    }

    /// The checker-backed rules that govern a folder (D-CHECKER): adopted pack
    /// rules with `verification = 'checker'` and a non-empty `checker_ref`,
    /// resolved from the same two planes as [`Self::resolve_local_pack_raws`] (the
    /// folder's namespaces plus the always-on general/user adoptions). Returns
    /// `(rule_statement, checker_ref)` — the statement is the stable handle, the
    /// checker_ref the canonical command verb to run.
    pub async fn resolve_local_checker_rules(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<(String, String)>, String> {
        let rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT r.statement, r.checker_ref
               FROM sensei.rule_pack_adoptions a
               JOIN sensei.rule_packs p      ON p.id = a.pack_id
               JOIN sensei.rule_pack_rules r ON r.pack_id = p.id
              WHERE r.verification = 'checker'
                AND r.checker_ref IS NOT NULL AND r.checker_ref <> ''
                AND ( a.namespace_id IN (
                          SELECT namespace_id FROM sensei.folder_namespaces WHERE folder_id = $1)
                      OR a.namespace_id IN (
                          SELECT id FROM sensei.namespaces WHERE scope_key IN ('general', 'user')) )
              ORDER BY r.statement",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// The command line a repo runs for a canonical command verb (`lint` | `test`
    /// | `build` | …), from the manifest-discovered `project_commands`. `None`
    /// when the repo has no command in that category. Used to map a checker rule's
    /// `checker_ref` to a runnable command.
    pub async fn project_command_for(
        &self,
        folder_id: &uuid::Uuid,
        category: &str,
    ) -> Result<Option<String>, String> {
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT command_line FROM sensei.project_commands
              WHERE folder_id = $1 AND category = $2
              ORDER BY discovered_at DESC LIMIT 1",
        )
        .bind(folder_id)
        .bind(category)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(c,)| c))
    }

    /// Append a checker run to `rule_check_runs` (D-CHECKER).
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_check_run(
        &self,
        folder_id: &uuid::Uuid,
        rule_statement: &str,
        checker_ref: &str,
        command: &str,
        verdict: &str,
        exit_code: Option<i32>,
        output_tail: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_check_runs
                (folder_id, rule_statement, checker_ref, command, verdict, exit_code, output_tail)
             VALUES ($1, $2, $3, $4, $5::sensei.check_verdict, $6, $7)",
        )
        .bind(folder_id)
        .bind(rule_statement)
        .bind(checker_ref)
        .bind(command)
        .bind(verdict)
        .bind(exit_code)
        .bind(output_tail)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The governance scope ladder — `(key, name, level)` ordered most-general
    /// first (ascending level). Feeds the constitution endpoint, which groups a
    /// repo's resolved rules into one rung per scope.
    pub async fn list_scopes(&self) -> Result<Vec<(String, String, i32)>, String> {
        let rows: Vec<(String, String, i32)> = sqlx_core::query_as::query_as(
            "SELECT key, name, level FROM sensei.scopes ORDER BY level",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Resolve a user's effective behavioural stance for a repo: the most-specific
    /// namespace stance on the `sensei.scopes` ladder wins, falling back to the
    /// user's namespace-less default, then to the enum defaults (via
    /// [`crate::stance::pick_stance`]). `folder_id` is optional — with `None` (the
    /// repo isn't indexed / unknown) only the user's default row is a candidate.
    /// Daemon-local (D-STANCE-SCOPE): stance drives the local session, never a
    /// tenant-shared value.
    pub async fn resolve_stance(
        &self,
        user_key: &str,
        folder_id: Option<&uuid::Uuid>,
    ) -> Result<crate::stance::ResolvedStance, String> {
        // Candidate rows: the user's namespace-less default (level NULL) plus any
        // stance bound to a namespace this folder belongs to. The pure
        // pick_stance applies precedence, so SQL only needs to gather + tag level.
        let rows: Vec<(Option<i32>, String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT s.level, st.autonomy::text, st.sharing::text, st.review::text
               FROM sensei.stances st
               LEFT JOIN sensei.namespaces n ON n.id = st.namespace_id
               LEFT JOIN sensei.scopes s ON s.key = n.scope_key
              WHERE st.user_key = $1
                AND ( st.namespace_id IS NULL
                      OR st.namespace_id IN (
                            SELECT namespace_id FROM sensei.folder_namespaces
                             WHERE folder_id = $2 ) )",
        )
        .bind(user_key)
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let candidates: Vec<crate::stance::StanceCandidate> = rows
            .into_iter()
            .map(|(level, autonomy, sharing, review)| crate::stance::StanceCandidate {
                level,
                autonomy,
                sharing,
                review,
            })
            .collect();
        Ok(crate::stance::pick_stance(&candidates))
    }

    /// Upsert a user's stance at a scope and return the new `updated_at`.
    /// `namespace_id = None` writes the user's default row (namespace-less);
    /// `Some(ns)` writes the stance for that scope namespace. The default row and
    /// the scoped rows have different uniqueness (a partial unique index on
    /// `user_key` where `namespace_id IS NULL` vs. the `(user_key, namespace_id)`
    /// composite), so the conflict target differs by branch. Callers validate the
    /// enum fields first (via [`crate::stance::StanceInput`]).
    pub async fn upsert_stance(
        &self,
        user_key: &str,
        namespace_id: Option<&uuid::Uuid>,
        autonomy: &str,
        sharing: &str,
        review: &str,
    ) -> Result<String, String> {
        let sql = if namespace_id.is_some() {
            "INSERT INTO sensei.stances (user_key, namespace_id, autonomy, sharing, review, updated_at)
             VALUES ($1, $2, $3::sensei.stance_autonomy, $4::sensei.stance_sharing, $5::sensei.stance_review, now())
             ON CONFLICT (user_key, namespace_id) DO UPDATE SET
                autonomy = EXCLUDED.autonomy, sharing = EXCLUDED.sharing,
                review = EXCLUDED.review, updated_at = now()
             RETURNING updated_at::text"
        } else {
            // The default row: NULLs are distinct under the composite unique, so
            // target the partial unique index (user_key where namespace_id IS NULL).
            "INSERT INTO sensei.stances (user_key, namespace_id, autonomy, sharing, review, updated_at)
             VALUES ($1, $2, $3::sensei.stance_autonomy, $4::sensei.stance_sharing, $5::sensei.stance_review, now())
             ON CONFLICT (user_key) WHERE namespace_id IS NULL DO UPDATE SET
                autonomy = EXCLUDED.autonomy, sharing = EXCLUDED.sharing,
                review = EXCLUDED.review, updated_at = now()
             RETURNING updated_at::text"
        };
        let (updated_at,): (String,) = sqlx_core::query_as::query_as(sql)
            .bind(user_key)
            .bind(namespace_id)
            .bind(autonomy)
            .bind(sharing)
            .bind(review)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(updated_at)
    }

    /// Resolve a repo's namespace at a governance scope — e.g. "this repo's
    /// `project` namespace" or "its `organization` namespace". Used when
    /// authoring a rule so the caller can say "scope this to the project" and we
    /// attach the right namespace_id from the repo's memberships. Returns None
    /// for always-on scopes (`general`/`user`) or when the repo has no namespace
    /// at that scope.
    /// A folder's namespace memberships as `(scope_key, slug)` pairs — the stable
    /// cross-DB identity the Dōjō `rules/resolved` endpoint matches on (the daemon
    /// and Dōjō have separate namespace uuids). Excludes the always-on
    /// general/user scopes (no namespace row). Used to fold adopted-pack rules
    /// into `get_rules`.
    pub async fn folder_namespace_pairs(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<(String, String)>, String> {
        let rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT n.scope_key, n.slug
               FROM sensei.folder_namespaces fn
               JOIN sensei.namespaces n ON n.id = fn.namespace_id
              WHERE fn.folder_id = $1",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// The project a folder belongs to, or `None` (unattributed folder).
    pub async fn folder_project_id(&self, folder_id: &uuid::Uuid) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(Option<uuid::Uuid>,)> =
            sqlx_core::query_as::query_as("SELECT project_id FROM sensei.folders WHERE id = $1")
                .bind(folder_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        Ok(row.and_then(|(pid,)| pid))
    }

    pub async fn namespace_for_folder_scope(&self, folder_id: &uuid::Uuid, scope_key: &str) -> Result<Option<uuid::Uuid>, String> {
        if matches!(scope_key, "general" | "user") {
            return Ok(None); // always-on scopes are unscoped (namespace_id NULL)
        }
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT n.id
               FROM sensei.folder_namespaces fn
               JOIN sensei.namespaces n ON n.id = fn.namespace_id
              WHERE fn.folder_id = $1 AND n.scope_key = $2
              LIMIT 1",
        )
        .bind(folder_id)
        .bind(scope_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// The slug of a run's project namespace (`sensei.namespaces` scope=project),
    /// or None when the run has no project or no project-scope namespace. Fed to
    /// the relay federation so the Worker can open the caller's billing seat on
    /// this project (proof the user is actively using sensei there).
    pub async fn run_project_slug(&self, run_id: &uuid::Uuid) -> Result<Option<String>, String> {
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT n.slug
               FROM activity.runs r
               JOIN sensei.folders f ON f.project_id = r.project_id
               JOIN sensei.folder_namespaces fn ON fn.folder_id = f.id
               JOIN sensei.namespaces n ON n.id = fn.namespace_id
              WHERE r.id = $1 AND n.scope_key = 'project'
              LIMIT 1",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(slug,)| slug))
    }

    /// The run's project as `(slug, name)` for the dōjō `dojo.projects` display row:
    /// the project-scope namespace slug (as [`run_project_slug`]) plus the project's
    /// display name (`sensei.projects.name`). Both are the user's own project
    /// metadata, federated as-is. `None` when the run has no bound project namespace.
    pub async fn run_project_info(
        &self,
        run_id: &uuid::Uuid,
    ) -> Result<Option<(String, String)>, String> {
        let row: Option<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT n.slug, p.name
               FROM activity.runs r
               JOIN sensei.projects p ON p.id = r.project_id
               JOIN sensei.folders f ON f.project_id = r.project_id
               JOIN sensei.folder_namespaces fn ON fn.folder_id = f.id
               JOIN sensei.namespaces n ON n.id = fn.namespace_id
              WHERE r.id = $1 AND n.scope_key = 'project'
              LIMIT 1",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// The global, repo-independent ruleset: rules at the always-on `general`
    /// and `user` scopes plus genuinely-global unscoped rules (`namespace_id IS
    /// NULL AND project_id IS NULL`). These apply everywhere and are what the
    /// daemon materializes into `~/.sensei/rules.md`. A project-tied unscoped
    /// memory (a learned convention with a `project_id`) is that project's, not
    /// global, so it is deliberately excluded here — it surfaces only via
    /// [`Self::resolve_rules_raw`] for its own repo. Same ordering as
    /// `resolve_rules_raw` but with no folder dimension.
    pub async fn resolve_global_rules(&self) -> Result<Vec<crate::governance::RawRule>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT m.id, m.title, m.content, m.impact, m.enforcement::text,
                        COALESCE(n.scope_key, 'general') AS scope,
                        n.name AS namespace
                   FROM sensei.memories m
                   LEFT JOIN sensei.namespaces n ON n.id = m.namespace_id
                   LEFT JOIN sensei.scopes s ON s.key = n.scope_key
                  WHERE m.status IN ('active'::sensei.memory_status,
                                     'reinforced'::sensei.memory_status,
                                     'battle_tested'::sensei.memory_status)
                    AND ( (m.namespace_id IS NULL AND m.project_id IS NULL)
                          OR n.scope_key IN ('general', 'user') )
                  ORDER BY m.enforcement DESC,
                           COALESCE(n.level, s.level, 0) DESC,
                           m.strength DESC",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, content, impact, enforcement, scope, namespace)| {
            crate::governance::RawRule {
                id: id.to_string(), title, content, impact, enforcement, scope, namespace,
            }
        }).collect())
    }

    // ── Governance Tier-2: consolidated (LLM-merged, approved) rulesets ──

    /// Next version number for a scope's consolidated ruleset (max+1, or 1).
    pub async fn next_ruleset_version(&self, scope: &str) -> Result<i32, String> {
        let row: (Option<i32>,) = sqlx_core::query_as::query_as(
            "SELECT max(version) FROM sensei.consolidated_rulesets WHERE scope = $1",
        ).bind(scope).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0.unwrap_or(0) + 1)
    }

    /// The source_hash of a scope's most recent consolidation (any status), so a
    /// re-merge can be skipped when the Tier-1 input is unchanged.
    pub async fn latest_ruleset_source_hash(&self, scope: &str) -> Result<Option<String>, String> {
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT source_hash FROM sensei.consolidated_rulesets WHERE scope = $1 ORDER BY version DESC LIMIT 1",
        ).bind(scope).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(h,)| h))
    }

    /// Insert a new consolidated ruleset version.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_consolidated_ruleset(
        &self, scope: &str, version: i32, content: &str, conflicts: &serde_json::Value,
        model: Option<&str>, source_hash: &str, status: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.consolidated_rulesets
                (scope, version, content, conflicts, model, source_hash, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        )
            .bind(scope).bind(version).bind(content).bind(conflicts)
            .bind(model).bind(source_hash).bind(status)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Fetch a scope's consolidated ruleset: the row with `status` when given
    /// (e.g. "approved"), else the latest version.
    pub async fn get_consolidated_ruleset(&self, scope: &str, status: Option<&str>) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(uuid::Uuid, i32, String, serde_json::Value, Option<String>, String)> = match status {
            Some(s) => sqlx_core::query_as::query_as(
                "SELECT id, version, content, conflicts, model, status FROM sensei.consolidated_rulesets
                  WHERE scope = $1 AND status = $2 ORDER BY version DESC LIMIT 1",
            ).bind(scope).bind(s).fetch_optional(&self.pool).await,
            None => sqlx_core::query_as::query_as(
                "SELECT id, version, content, conflicts, model, status FROM sensei.consolidated_rulesets
                  WHERE scope = $1 ORDER BY version DESC LIMIT 1",
            ).bind(scope).fetch_optional(&self.pool).await,
        }.map_err(|e| e.to_string())?;
        Ok(row.map(|(id, version, content, conflicts, model, status)| serde_json::json!({
            "id": id, "version": version, "content": content,
            "conflicts": conflicts, "model": model, "status": status,
        })))
    }

    /// Approve a consolidated ruleset: supersede the scope's prior approved
    /// version, then mark this one approved. Returns (scope, content).
    pub async fn approve_consolidated_ruleset(&self, id: &uuid::Uuid) -> Result<Option<(String, String)>, String> {
        sqlx_core::query::query(
            "UPDATE sensei.consolidated_rulesets SET status = 'superseded'
              WHERE status = 'approved'
                AND scope = (SELECT scope FROM sensei.consolidated_rulesets WHERE id = $1)
                AND id <> $1",
        ).bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        let row: Option<(String, String)> = sqlx_core::query_as::query_as(
            "UPDATE sensei.consolidated_rulesets SET status = 'approved' WHERE id = $1 RETURNING scope, content",
        ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// Active memories anchored to (slot[, feature]) for a project. `feature=None`
    /// matches project-scope (feature IS NULL); `Some(f)` matches that feature.
    pub async fn list_memories_for_slot(
        &self, project_id: &uuid::Uuid, slot: &str, feature: Option<&str>, limit: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, title, content, feature FROM sensei.memories
                  WHERE status='active' AND project_id = $1
                    AND spine_slot = $2::sensei.spine_slot
                    AND feature IS NOT DISTINCT FROM $3
                  ORDER BY strength DESC, modified_at DESC LIMIT $4"
            ).bind(project_id).bind(slot).bind(feature).bind(limit)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, title, content, feature)|
            serde_json::json!({ "id": id, "title": title, "content": content, "feature": feature })
        ).collect())
    }

    pub async fn assemble_context(
        &self,
        project_id: uuid::Uuid,
        stack_ids:  &[String],
        tags:       Option<&[String]>,
        limit:      i64,
        slot:       Option<(&str, Option<&str>)>,
    ) -> Result<serde_json::Value, String> {
        let allowed = ["active", "reinforced", "battle_tested", "challenged"];
        let allowed_owned: Vec<String> = allowed.iter().map(|s| s.to_string()).collect();
        let stack_owned: Vec<String> = stack_ids.to_vec();
        let tags_owned: Option<Vec<String>> = tags.map(|t| t.to_vec());

        let rows: Vec<(uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, String, String,
                       Option<String>, f64, String, i32, i32,
                       Option<chrono::DateTime<chrono::Utc>>, Vec<String>, Option<String>,
                       chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, project_id, scope::text, scope_filter, type::text, title, content,
                        impact, strength::float8, status::text, reinforced_count, violated_count,
                        last_relevant_at, tags, triage_signal, modified_at
                   FROM sensei.memories
                  WHERE status::text = ANY($1)
                    AND (
                           project_id = $2
                        OR (scope = 'stack'  AND scope_filter = ANY($3))
                        OR  scope = 'global'
                    )
                    AND ($4::text[] IS NULL OR tags && $4)
                  ORDER BY strength DESC, last_relevant_at DESC NULLS LAST, modified_at DESC
                  LIMIT $5"
            )
            .bind(&allowed_owned).bind(project_id).bind(&stack_owned)
            .bind(&tags_owned).bind(limit)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        // Telemetry: log one memory_loads row per delivered memory ("did injected
        // memory help?" — loads here vs applied/ignored outcomes there). The status
        // filter above already excludes archived/rejected, so these are the same
        // memories record_outcomes_batch would accept. NON-FATAL: this is the hot
        // context-delivery path — a logging failure must warn and continue, never
        // block or error the returned context.
        let memory_ids: Vec<uuid::Uuid> = rows.iter().map(|r| r.0).collect();
        if !memory_ids.is_empty() {
            let logged = sqlx_core::query::query(
                // `FOR SHARE` pins each referenced memory row for the duration of
                // this insert, so a concurrent DELETE (which cascades to memory_loads)
                // blocks until we commit rather than racing the FK check. The CTE
                // also filters to memories that still exist — a memory already gone
                // is simply not logged, never a whole-batch FK abort.
                "WITH existing AS (
                     SELECT id FROM sensei.memories WHERE id = ANY($1::uuid[]) FOR SHARE
                 )
                 INSERT INTO activity.memory_loads (memory_id, project_id, source)
                 SELECT id, $2, 'get_layered_context' FROM existing"
            )
                .bind(&memory_ids).bind(project_id)
                .execute(&self.pool).await;
            if let Err(e) = logged {
                tracing::warn!(error = %e, count = memory_ids.len(),
                    "assemble_context: failed to log memory loads (non-fatal — context still delivered)");
            }
        }

        let mut memories: Vec<serde_json::Value> = rows.into_iter().map(|r| serde_json::json!({
            "id":               r.0,
            "scope":            r.2,
            "scope_filter":     r.3,
            "type":             r.4,
            "title":            r.5,
            "content":          r.6,
            "impact":           r.7,
            "strength":         r.8,
            "applied_count":    r.10,
            "violated_count":   r.11,
            "last_relevant_at": r.12.map(|t| t.to_rfc3339()),
            "tags":             r.13,
            "updated_at":       r.15.to_rfc3339(),
        })).collect();

        // Slot hint: lead the bundle with slot-anchored memories, deduped against
        // the general blend above (a slot-anchored memory that also matched the
        // scope/tag blend must not appear twice).
        if let Some((s, feature)) = slot {
            let anchored = self.list_memories_for_slot(&project_id, s, feature, limit).await?;
            if !anchored.is_empty() {
                let anchored_ids: std::collections::HashSet<String> = anchored.iter()
                    .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                    .collect();
                memories.retain(|m| {
                    m["id"].as_str().map(|id| !anchored_ids.contains(id)).unwrap_or(true)
                });
                let mut led = anchored;
                led.append(&mut memories);
                memories = led;
            }
        }

        // Version = max modified_at across the set (stable identifier for cache validation).
        let version = memories.iter()
            .filter_map(|m| m["updated_at"].as_str().map(|s| s.to_string()))
            .max()
            .unwrap_or_default();
        let cache_until = (chrono::Utc::now() + chrono::Duration::minutes(5)).to_rfc3339();

        Ok(serde_json::json!({
            "version":     version,
            "memories":    memories,
            "cache_until": cache_until,
        }))
    }

    /// Insert a batch of outcomes. Skips rows whose target memory is archived or rejected.
    pub async fn record_outcomes_batch(
        &self,
        rows: &[OutcomeRow],
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut skipped: Vec<serde_json::Value> = Vec::new();
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        for r in rows {
            // Check current status first.
            let status: Option<(String,)> = sqlx_core::query_as::query_as(
                "SELECT status::text FROM sensei.memories WHERE id = $1"
            ).bind(r.memory_id).fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
            let Some((s,)) = status else {
                skipped.push(serde_json::json!({"memory_id": r.memory_id, "reason": "not_found"}));
                continue;
            };
            if s == "archived" || s == "rejected" {
                skipped.push(serde_json::json!({"memory_id": r.memory_id, "reason": format!("status_{s}")}));
                continue;
            }
            sqlx_core::query::query(
                "INSERT INTO sensei.memory_outcomes (memory_id, session_id, outcome, context)
                 VALUES ($1, $2, $3::sensei.memory_outcome, $4)"
            )
                .bind(r.memory_id).bind(r.session_id).bind(&r.outcome).bind(&r.context)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(skipped)
    }

    /// Return the list of stack identifiers for a project.
    /// The `sensei.projects.stack` column is JSONB and may be an array of strings,
    /// an object with a recognisable array key, or absent — all cases return `[]`.
    pub async fn get_project_stack_ids(&self, project_id: &uuid::Uuid) -> Result<Vec<String>, String> {
        let row: Option<(serde_json::Value,)> = sqlx_core::query_as::query_as(
            "SELECT stack FROM sensei.projects WHERE id = $1"
        ).bind(project_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;

        let Some((stack_json,)) = row else { return Ok(vec![]); };

        // The stack jsonb may be an array of strings, an object with a "languages" key,
        // or empty. Be permissive: accept array-of-strings OR object-with-arrays, return [].
        match &stack_json {
            serde_json::Value::Array(arr) => {
                Ok(arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            }
            serde_json::Value::Object(obj) => {
                // Try common keys: languages, ids, items.
                for key in &["languages", "ids", "items"] {
                    if let Some(serde_json::Value::Array(arr)) = obj.get(*key) {
                        return Ok(arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());
                    }
                }
                // No recognizable shape — return empty (no stack blending).
                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }

    pub async fn get_project_repos(&self, project_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        // Only project ROOTS are repos. `kind='folder'` (navigable subfolder tree)
        // AND `kind='workspace_member'` (monorepo members, D5a) are the structural
        // tree, NOT separate repos — listing them makes a single-repo monorepo with
        // N members render as an N+1-repo "multi-repo" project (#62). The data is
        // correct; this read path was projecting the subfolder tree as repos.
        let rows: Vec<(uuid::Uuid, String, String, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, abs_path, kind::text FROM sensei.folders
                 WHERE project_id = $1 AND kind::text NOT IN ('folder', 'workspace_member') ORDER BY name"
            ).bind(project_id)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, path, kind)| {
            serde_json::json!({ "id": id, "name": name, "path": path, "kind": kind })
        }).collect())
    }

    pub async fn list_sessions_by_project(&self, project_id: &uuid::Uuid, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        // Extended shape (T3 Slice 1.4): the Sessions screen needs model,
        // provider, turns, corrections, and completed_at so the row can
        // render date / model / turns / corrections / FTR / outcome
        // side-by-side per the mockup. `outcome` is nullable while a
        // session is still in-flight — decode it Option to keep the query
        // resilient.
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            uuid::Uuid,                                     // id
            String,                                         // task
            Option<bool>,                                   // ftr
            Option<String>,                                 // outcome
            chrono::DateTime<chrono::Utc>,                  // started_at
            Option<chrono::DateTime<chrono::Utc>>,          // completed_at
            i32,                                            // turns
            i32,                                            // corrections
            Option<String>,                                 // provider
            Option<String>,                                 // model
        )> = sqlx_core::query_as::query_as(
                "SELECT id, task, ftr, outcome::text, started_at, completed_at,
                        turns, corrections, provider, model
                 FROM activity.sessions WHERE project_id = $1
                 ORDER BY started_at DESC LIMIT $2"
            ).bind(project_id).bind(limit)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, task, ftr, outcome, started, completed, turns, corrections, provider, model)| {
            serde_json::json!({
                "id":           id,
                "task":         task,
                "ftr":          ftr,
                "outcome":      outcome,
                "startedAt":    started.to_rfc3339(),
                "completedAt":  completed.map(|t| t.to_rfc3339()),
                "turns":        turns,
                "corrections":  corrections,
                "provider":     provider,
                "model":        model,
            })
        }).collect())
    }

    pub async fn get_project_recommendations(&self, project_id: &uuid::Uuid, status: Option<&str>) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<String>, String, Option<String>,
                        Option<f64>, Option<f64>, Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>,
                        Option<f64>, bool, String)> =
            sqlx_core::query_as::query_as(
                // `action_type` powers the Upgrades screen's installable filter; it is
                // `not null` on the table and mirrors the impact serializer's `actionType`.
                "SELECT id, title, urgency::text, status::text, verdict::text, why, impact,
                        baseline_ftr::float8, current_ftr::float8, acted_at, measured_at,
                        score::float8, focal, action_type
                 FROM inference.recommendations WHERE project_id = $1
                   AND ($2::text IS NULL OR status::text = $2)
                 ORDER BY focal DESC, score DESC NULLS LAST,
                          CASE urgency WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, id
                 LIMIT 50"
            ).bind(project_id).bind(status)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, title, urgency, status, verdict, why, impact, baseline, current, acted, measured, score, focal, action_type)| {
            serde_json::json!({
                "id": id, "title": title, "urgency": urgency, "status": status, "verdict": verdict,
                "why": why, "impact": impact, "actionType": action_type,
                "baseline_ftr": baseline, "current_ftr": current,
                "acted_at": acted.map(|t| t.to_rfc3339()), "measured_at": measured.map(|t| t.to_rfc3339()),
                "score": score, "focal": focal,
            })
        }).collect())
    }

    // ── Index Errors ──────────────────────────────────────────────────

    pub async fn log_index_error(
        &self, folder_id: &uuid::Uuid, file_path: &str, error: &str,
        adapter: Option<&str>, phase: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.index_errors(folder_id, file_path, error, adapter, phase) VALUES($1, $2, $3, $4, $5)"
        )
            .bind(folder_id).bind(file_path).bind(error).bind(adapter).bind(phase)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_index_errors(&self, folder_id: Option<&uuid::Uuid>) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<String>, chrono::DateTime<chrono::Utc>)> = match folder_id {
            Some(fid) => sqlx_core::query_as::query_as(
                "SELECT folder_id, file_path, error, adapter, phase, created_at FROM sensei.index_errors WHERE folder_id = $1 ORDER BY created_at DESC"
            ).bind(fid).fetch_all(&self.pool).await,
            None => sqlx_core::query_as::query_as(
                "SELECT folder_id, file_path, error, adapter, phase, created_at FROM sensei.index_errors ORDER BY created_at DESC LIMIT 200"
            ).fetch_all(&self.pool).await,
        }.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(fid, fp, err, adapter, phase, ts)| {
            serde_json::json!({
                "folder_id": fid, "file_path": fp, "error": err,
                "adapter": adapter, "phase": phase, "created_at": ts.to_rfc3339(),
            })
        }).collect())
    }

    pub async fn clear_index_errors(&self, folder_id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("DELETE FROM sensei.index_errors WHERE folder_id = $1")
            .bind(folder_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete `public.logs` rows older than `days` days. The task logger writes
    /// two rows per task, so large scans add hundreds of thousands of rows;
    /// this enforces a retention window. Returns the number of rows removed.
    pub async fn prune_logs(&self, days: i32) -> Result<u64, String> {
        let r = sqlx_core::query::query(
            "DELETE FROM public.logs WHERE logged_at < now() - (interval '1 day' * $1)"
        )
            .bind(days)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(r.rows_affected())
    }

    /// Prune raw activity older than `days` days, respecting the analyzer's
    /// value-extraction guard (#74) AND the capture-before-reclaim guard
    /// (2026-08-12 retention decision):
    ///
    /// - Sessions are eligible only when `analyzed_at IS NOT NULL` AND
    ///   `started_at < now() - days` — a session whose insights the analyzer
    ///   never derived is kept even if it is old (would lose signal).
    /// - AND (capture-before-reclaim) the session's day must EITHER already be
    ///   captured in the durable metric store — an `EXISTS` daily
    ///   `sensei.project_metrics` row for the session's project (via
    ///   `folders.project_id`) with `computed_on = date_trunc('day',
    ///   s.started_at)::date` — OR the session must be older than a hard
    ///   backstop (`backstop_days`) so nothing lingers forever if metrics never
    ///   compute. Backfilled history is thus durable regardless of
    ///   prune/compute ordering: a day's sessions are only reclaimed once that
    ///   day's snapshot exists (or the backstop forces it).
    /// - The eligible sessions' \`activity.turns\` cascade (FK ON DELETE
    ///   CASCADE) so `turns` deletes are counted via a preflight
    ///   `COUNT(*) WHERE session_id IN (…)` for observability.
    /// - `activity.transcript_turns` and `activity.assistant_events` key
    ///   session-scoped rows off `client_session_id` (text), NOT the
    ///   session uuid — no FK, so we DELETE by matching that column.
    /// - Session-less assistant_events (never attached to a session; still
    ///   valuable for global tool-usage stats via ts) are pruned by ts alone
    ///   when they're older than the cutoff — same window, but they don't
    ///   need the analyzed-only guard.
    ///
    /// Derived signals (\`inference.detected_patterns\` /
    /// \`inference.recommendations\` / \`inference.reasoning_traces\` /
    /// \`sensei.memories\`) are NEVER touched — they are the distilled value
    /// that survives raw-event pruning.
    ///
    /// Ordering respects FKs: children first (transcript_turns / assistant_events
    /// keyed by client_session_id), then sessions (which cascades turns).
    pub async fn prune_activity(&self, days: i32, backstop_days: i32, day_keyed_groups: &[&str]) -> Result<ActivityPruneCounts, String> {
        // Owned copy for the text[] bind (sqlx encodes `&[String]`, not `&[&str]`).
        let day_keyed_owned: Vec<String> = day_keyed_groups.iter().map(|g| g.to_string()).collect();
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        // (1) Snapshot eligible sessions once — used for every child delete
        //     so we don't re-scan the guard SQL four times. Capture-before-
        //     reclaim: a session is eligible only when its day is already
        //     captured by a DAY-KEYED (delivery) metric in sensei.project_metrics
        //     (daily grain, same project via the folder join, and the metric's
        //     task_name is one of the day-keyed groups) OR it is older than the
        //     hard backstop. Scoping to day-keyed metrics is load-bearing:
        //     forward-only SNAPSHOT computers stamp a grain='daily' row on their
        //     own day every run, so an unscoped EXISTS would let a session be
        //     reclaimed before its delivery metric ever computed.
        let eligible: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT s.id, s.client_session_id
               FROM activity.sessions s
               JOIN sensei.folders f ON f.id = s.folder_id
              WHERE s.analyzed_at IS NOT NULL
                AND s.started_at < now() - (interval '1 day' * $1)
                AND (EXISTS (SELECT 1
                               FROM sensei.project_metrics pm
                               JOIN sensei.metrics m ON m.id = pm.metric_id
                              WHERE pm.project_id = f.project_id
                                AND pm.grain = 'daily'
                                AND pm.computed_on = date_trunc('day', s.started_at)::date
                                AND m.task_name = ANY($3))
                     OR s.started_at < now() - (interval '1 day' * $2))"
        )
            .bind(days)
            .bind(backstop_days)
            .bind(&day_keyed_owned)
            .fetch_all(&mut *tx).await.map_err(|e| e.to_string())?;
        if eligible.is_empty() {
            // Even with no eligible sessions, orphan assistant_events by ts
            // are still a valid target.
            let cutoff_ms = self.cutoff_millis(days);
            let ae = sqlx_core::query::query(
                // NOT EXISTS instead of NOT IN because sessions.client_session_id
            // is nullable — a NULL in the NOT IN subquery poisons the whole
            // predicate under ANSI three-valued logic.
            "DELETE FROM activity.assistant_events ae
              WHERE ae.ts < $1
                AND (ae.session_id = ''
                     OR NOT EXISTS (
                        SELECT 1 FROM activity.sessions s
                         WHERE s.client_session_id = ae.session_id))"
            )
                .bind(cutoff_ms)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            tx.commit().await.map_err(|e| e.to_string())?;
            return Ok(ActivityPruneCounts { assistant_events: ae.rows_affected(), ..Default::default() });
        }
        let session_uuids: Vec<uuid::Uuid> = eligible.iter().map(|(u, _)| *u).collect();
        let client_ids:    Vec<String>     = eligible.iter().map(|(_, c)| c.clone()).collect();

        // (2) Count turns that will cascade on the session delete — for the
        //     log line; the DELETE itself happens via CASCADE below.
        let turns_count: (i64,) = sqlx_core::query_as::query_as(
            "SELECT COUNT(*) FROM activity.turns WHERE session_id = ANY($1::uuid[])"
        )
            .bind(&session_uuids)
            .fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;

        // (3) transcript_turns keyed by client_session_id (text, no FK).
        let tt = sqlx_core::query::query(
            "DELETE FROM activity.transcript_turns WHERE session_id = ANY($1::text[])"
        )
            .bind(&client_ids)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;

        // (4) assistant_events for the same client_session_ids.
        let ae_session = sqlx_core::query::query(
            "DELETE FROM activity.assistant_events WHERE session_id = ANY($1::text[])"
        )
            .bind(&client_ids)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;

        // (5) sessions — cascades turns.
        let sess = sqlx_core::query::query(
            "DELETE FROM activity.sessions WHERE id = ANY($1::uuid[])"
        )
            .bind(&session_uuids)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;

        // (6) Session-less orphan assistant_events by ts. Runs after the
        //     session-scoped prune so we don't double-count.
        let cutoff_ms = self.cutoff_millis(days);
        let ae_orphan = sqlx_core::query::query(
            "DELETE FROM activity.assistant_events WHERE ts < $1
               AND (session_id = '' OR session_id NOT IN
                    (SELECT client_session_id FROM activity.sessions))"
        )
            .bind(cutoff_ms)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;

        Ok(ActivityPruneCounts {
            sessions:         sess.rows_affected(),
            turns:            turns_count.0.max(0) as u64,
            transcript_turns: tt.rows_affected(),
            assistant_events: ae_session.rows_affected() + ae_orphan.rows_affected(),
        })
    }

    /// Wall-clock cutoff in unix-ms for `days` back — used by prune_activity's
    /// ts-based paths (assistant_events.ts is bigint ms).
    fn cutoff_millis(&self, days: i32) -> i64 {
        let secs = (chrono::Utc::now() - chrono::Duration::days(days as i64)).timestamp();
        secs.saturating_mul(1000)
    }

    // ── Raw ──────────────────────────────────────────────────────────

    /// Execute a parameterized query returning unresolved edges.
    pub async fn execute_raw_query(&self, sql: &str, folder_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, uuid::Uuid, Option<String>, String)> = sqlx_core::query_as::query_as(sql)
            .bind(folder_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, src, tgt_name, kind)| {
            serde_json::json!({ "id": id, "source_id": src, "target_name": tgt_name, "kind": kind })
        }).collect())
    }

    /// Execute a raw SQL statement.
    pub async fn execute_raw(&self, sql: &str) -> Result<(), String> {
        sqlx_core::query::query(sql)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("PgStore execute_raw: {}", e))?;
        Ok(())
    }

    // ── Logging (public.logs) ───────────────────────────────────────

    /// Insert a structured log entry into public.logs (kavach pattern).
    pub async fn insert_log(
        &self,
        level: &str,
        running_on: &str,
        logged_at: &str,
        message: &str,
        context: &serde_json::Value,
        data: &Option<serde_json::Value>,
        error: &Option<serde_json::Value>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO public.logs(level, running_on, logged_at, message, context, data, error)
             VALUES($1, $2, $3::timestamptz, $4, $5, $6, $7)"
        )
        .bind(level)
        .bind(running_on)
        .bind(logged_at)
        .bind(message)
        .bind(context)
        .bind(data)
        .bind(error)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("insert_log: {}", e))?;
        Ok(())
    }

    /// Read structured log rows from `public.logs` for the Observatory · Logs
    /// screen. All filters are optional (`None` = no constraint) and fully
    /// parameterized — never string-interpolated. Rows come back newest-first
    /// (`logged_at DESC`), capped at `limit`.
    ///
    /// - `level`   → exact match on the indexed `level` column.
    /// - `source`  → exact match on the indexed `running_on` column (which
    ///   component wrote the log: daemon / cli / mcp / app).
    /// - `module`  → exact match on the indexed `context->>'module'` bucket
    ///   (finer source: scanner / watcher / analyzer / scheduler / …).
    /// - `since`   → lower bound on the indexed `logged_at` timestamp.
    pub async fn query_logs(
        &self,
        level: Option<&str>,
        source: Option<&str>,
        module: Option<&str>,
        since: Option<chrono::DateTime<chrono::Utc>>,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        type LogRow = (
            uuid::Uuid,
            String,
            String,
            chrono::DateTime<chrono::Utc>,
            Option<String>,
            serde_json::Value,
            Option<serde_json::Value>,
            Option<serde_json::Value>,
        );
        let rows: Vec<LogRow> = sqlx_core::query_as::query_as(
            "SELECT id, level, running_on, logged_at, message, context, data, error
             FROM public.logs
             WHERE ($1::text IS NULL OR level = $1)
               AND ($2::text IS NULL OR running_on = $2)
               AND ($3::text IS NULL OR context->>'module' = $3)
               AND ($4::timestamptz IS NULL OR logged_at >= $4)
             ORDER BY logged_at DESC
             LIMIT $5",
        )
        .bind(level)
        .bind(source)
        .bind(module)
        .bind(since)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("query_logs: {}", e))?;

        Ok(rows
            .into_iter()
            .map(|(id, level, running_on, logged_at, message, context, data, error)| {
                serde_json::json!({
                    "id": id,
                    "level": level,
                    "source": running_on,
                    "logged_at": logged_at.to_rfc3339(),
                    "message": message,
                    "context": context,
                    "data": data,
                    "error": error,
                })
            })
            .collect())
    }

    // ── Task Executions (activity.task_executions) ──────────────────

    /// Insert a running task execution record. Returns the row UUID.
    /// `retry_number` is the task's attempt count (0 = first attempt), persisted
    /// so bounded retries (D6c) are observable on the logs/health screen.
    pub async fn start_task_execution(
        &self,
        task_id: i64,
        parent_task_id: Option<i64>,
        task_kind: &str,
        folder_path: &str,
        path: &str,
        retry_number: i32,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.task_executions(task_id, parent_task_id, task_kind, folder_path, path, status, retry_number)
             VALUES($1, $2, $3, $4, $5, 'running', $6) RETURNING id"
        )
        .bind(task_id)
        .bind(parent_task_id)
        .bind(task_kind)
        .bind(folder_path)
        .bind(path)
        .bind(retry_number)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("start_task_execution: {}", e))?;
        Ok(row.0)
    }

    /// Mark a task execution as completed.
    pub async fn complete_task_execution(
        &self,
        id: &uuid::Uuid,
        items_processed: i32,
        duration_ms: i32,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.task_executions
                SET status = 'completed', items_processed = $2, duration_ms = $3, completed_at = now()
              WHERE id = $1"
        )
        .bind(id)
        .bind(items_processed)
        .bind(duration_ms)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("complete_task_execution: {}", e))?;
        Ok(())
    }

    /// Mark a task execution as failed.
    pub async fn fail_task_execution(
        &self,
        id: &uuid::Uuid,
        duration_ms: i32,
        error_message: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.task_executions
                SET status = 'failed', duration_ms = $2, error_message = $3, completed_at = now()
              WHERE id = $1"
        )
        .bind(id)
        .bind(duration_ms)
        .bind(error_message)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("fail_task_execution: {}", e))?;
        Ok(())
    }

    /// Boot reconcile (D6b/W2): terminate task-execution rows still `running`
    /// from a prior daemon session. `task_id` resets per session and the queue
    /// is in-memory, so a `running` row whose `started_at` precedes this
    /// session's start can never complete — its worker died with the process.
    /// Mark those `failed` (a terminal state) with a completion time and an
    /// explanatory `error_message`, so `status='running'` reflects only live
    /// work. Rows started at/after `session_start` (this session's own
    /// in-flight tasks) are left untouched. Idempotent. Returns rows reconciled.
    pub async fn reconcile_orphaned_task_executions(
        &self,
        session_start: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE activity.task_executions
                SET status = 'failed',
                    error_message = 'orphaned: daemon restarted while task was running',
                    completed_at = now()
              WHERE status = 'running' AND started_at < $1"
        )
        .bind(session_start)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("reconcile_orphaned_task_executions: {}", e))?;
        Ok(res.rows_affected())
    }

    // ── Knowledge Sources (federation endpoints) ──────────────────────

    pub async fn create_knowledge_source(&self, s: &NewKnowledgeSource) -> Result<uuid::Uuid, String> {
        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.knowledge_sources(kind, name, url, namespace_id, credential_ref, direction)
             VALUES($1,$2,$3,$4,$5,$6) RETURNING id")
            .bind(&s.kind).bind(&s.name).bind(&s.url).bind(s.namespace_id).bind(&s.credential_ref).bind(&s.direction)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub async fn list_knowledge_sources(&self) -> Result<Vec<KnowledgeSource>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, Option<uuid::Uuid>, String, String, i64, bool)> =
            sqlx_core::query_as::query_as(
                "SELECT id, kind, name, url, namespace_id, credential_ref, direction, last_seq, enabled
                   FROM sensei.knowledge_sources ORDER BY created_at")
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id,kind,name,url,namespace_id,credential_ref,direction,last_seq,enabled)|
            KnowledgeSource { id,kind,name,url,namespace_id,credential_ref,direction,last_seq,enabled }).collect())
    }

    pub async fn get_knowledge_source(&self, id: &uuid::Uuid) -> Result<Option<KnowledgeSource>, String> {
        let row: Option<(uuid::Uuid, String, String, String, Option<uuid::Uuid>, String, String, i64, bool)> =
            sqlx_core::query_as::query_as(
                "SELECT id, kind, name, url, namespace_id, credential_ref, direction, last_seq, enabled
                   FROM sensei.knowledge_sources WHERE id = $1")
            .bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(id,kind,name,url,namespace_id,credential_ref,direction,last_seq,enabled)|
            KnowledgeSource { id,kind,name,url,namespace_id,credential_ref,direction,last_seq,enabled }))
    }

    pub async fn set_source_cursor(&self, id: &uuid::Uuid, last_seq: i64) -> Result<(), String> {
        sqlx_core::query::query("UPDATE sensei.knowledge_sources SET last_seq = $2 WHERE id = $1")
            .bind(id).bind(last_seq).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn delete_knowledge_source(&self, id: &uuid::Uuid) -> Result<bool, String> {
        let res = sqlx_core::query::query("DELETE FROM sensei.knowledge_sources WHERE id = $1")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    // ── Dōjō connections (daemon-side membership mirror) ───────────────
    //
    // Local mirror of the Dōjōs this install is connected to (Fork 1: the
    // authoritative dojo.memberships row lives in the Dōjō service DB). Mirrors
    // the knowledge_sources CRUD discipline; the credential lives in the OS
    // Keychain (credential_ref), never in these rows.

    /// Insert a Dōjō connection with the service-assigned `id` as the PK.
    pub async fn create_dojo_membership(&self, m: &NewDojoMembership) -> Result<uuid::Uuid, String> {
        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.dojo_memberships
                (id, registry_url, tenant_key, dojo_url, kind, org_slugs, role,
                 authenticated_via, attribution_default, credential_ref, sync_status)
             VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id")
            .bind(m.id).bind(&m.registry_url).bind(&m.tenant_key).bind(&m.dojo_url)
            .bind(&m.kind).bind(&m.org_slugs).bind(&m.role).bind(&m.authenticated_via)
            .bind(&m.attribution_default).bind(&m.credential_ref).bind(&m.sync_status)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(id)
    }

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

    pub async fn list_dojo_memberships(&self) -> Result<Vec<DojoMembership>, String> {
        let rows: Vec<(uuid::Uuid, String, String, String, String, Vec<String>, String, String, String, String, String, i64, Option<String>, bool)> =
            sqlx_core::query_as::query_as(&format!("{} ORDER BY created_at", Self::DOJO_SELECT))
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(Self::map_dojo_row).collect())
    }

    pub async fn get_dojo_membership(&self, id: &uuid::Uuid) -> Result<Option<DojoMembership>, String> {
        let row: Option<(uuid::Uuid, String, String, String, String, Vec<String>, String, String, String, String, String, i64, Option<String>, bool)> =
            sqlx_core::query_as::query_as(&format!("{} WHERE id = $1", Self::DOJO_SELECT))
            .bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(Self::map_dojo_row))
    }

    /// Replace a membership's `org_slugs` (the git-remote owners it covers) —
    /// the org-tagging edit. Slugs are stored as given; callers normalise
    /// (lowercase/trim/dedup) upstream. Returns `false` if the id is unknown.
    pub async fn set_dojo_membership_orgs(&self, id: &uuid::Uuid, org_slugs: &[String]) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.dojo_memberships SET org_slugs = $2, updated_at = now() WHERE id = $1")
            .bind(id).bind(org_slugs).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// Update a connection's sync status. Returns `false` if unknown.
    pub async fn set_dojo_sync_status(&self, id: &uuid::Uuid, sync_status: &str) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.dojo_memberships SET sync_status = $2, updated_at = now() WHERE id = $1")
            .bind(id).bind(sync_status).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn delete_dojo_membership(&self, id: &uuid::Uuid) -> Result<bool, String> {
        let res = sqlx_core::query::query("DELETE FROM sensei.dojo_memberships WHERE id = $1")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// Bind (or, with `None`, unbind) a project to a Dōjō membership by setting
    /// `sensei.projects.dojo_id`. Returns `false` if the project is unknown.
    pub async fn bind_project_to_dojo(
        &self, project_id: &uuid::Uuid, membership_id: Option<&uuid::Uuid>,
    ) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.projects SET dojo_id = $2, modified_at = now() WHERE id = $1")
            .bind(project_id).bind(membership_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// Projects bound to a membership (`projects.dojo_id = id`) — the
    /// connections pane's "bound projects" strip.
    pub async fn projects_bound_to_dojo(&self, membership_id: &uuid::Uuid) -> Result<Vec<(uuid::Uuid, String)>, String> {
        let rows: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT id, name FROM sensei.projects WHERE dojo_id = $1 ORDER BY name")
            .bind(membership_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// The distinct git-remote owner slugs across a project's folders (lowercased,
    /// first-seen order) — e.g. a project whose repos are `github.com/sensei-hq/*`
    /// yields `["sensei-hq"]`. Feeds `dojo::routing::infer_binding` for the R3
    /// auto-bind suggestion. Reads `sensei.folders.remote_urls`; DB-only.
    pub async fn project_org_owners(&self, project_id: &uuid::Uuid) -> Result<Vec<String>, String> {
        let folders: Vec<(serde_json::Value,)> = sqlx_core::query_as::query_as(
            "SELECT remote_urls FROM sensei.folders WHERE project_id = $1")
            .bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        let mut owners: Vec<String> = Vec::new();
        for (remotes,) in folders {
            if let Some(arr) = remotes.as_array() {
                for r in arr {
                    if let Some(url) = r.get("url").and_then(serde_json::Value::as_str)
                        && let Some(owner) = remote_owner_slug(url)
                        && !owners.contains(&owner)
                    {
                        owners.push(owner);
                    }
                }
            }
        }
        Ok(owners)
    }

    /// Gather every KNOWN sensitive identifier for a project into a
    /// [`crate::dojo::attribution::ProjectIdentifiers`] — the deterministic
    /// client-work dereference (C5) needs these to strip source references before
    /// anything leaves the machine. Reads only; the strip itself is DB-free.
    ///
    /// Sources: `sensei.projects.{name, client}`, `sensei.folders.{name,
    /// abs_path, remote_urls}` (repo name + git owner/repo parsed from remotes),
    /// and `activity.sessions.{id, client_session_id}`.
    pub async fn project_identifiers(
        &self, project_id: &uuid::Uuid,
    ) -> Result<crate::dojo::attribution::ProjectIdentifiers, String> {
        use crate::dojo::attribution::ProjectIdentifiers;

        let proj: Option<(String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT name, client FROM sensei.projects WHERE id = $1")
            .bind(project_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        let (project_name, client_name) = match proj {
            Some((name, client)) => (Some(name), client),
            None => (None, None),
        };

        let folders: Vec<(String, String, serde_json::Value)> = sqlx_core::query_as::query_as(
            "SELECT name, abs_path, remote_urls FROM sensei.folders WHERE project_id = $1")
            .bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let mut repo_names: Vec<String> = Vec::new();
        let mut folder_paths: Vec<String> = Vec::new();
        for (name, abs_path, remotes) in folders {
            if !name.trim().is_empty() {
                repo_names.push(name);
            }
            if !abs_path.trim().is_empty() {
                folder_paths.push(abs_path);
            }
            if let Some(arr) = remotes.as_array() {
                for r in arr {
                    if let Some(url) = r.get("url").and_then(serde_json::Value::as_str) {
                        repo_names.extend(repo_tokens_from_remote(url));
                    }
                }
            }
        }

        let sessions: Vec<(uuid::Uuid, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT id, client_session_id FROM activity.sessions WHERE project_id = $1")
            .bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        let mut session_ids: Vec<String> = Vec::new();
        for (id, csid) in sessions {
            session_ids.push(id.to_string());
            if let Some(c) = csid.filter(|c| !c.trim().is_empty()) {
                session_ids.push(c);
            }
        }

        for v in [&mut repo_names, &mut folder_paths, &mut session_ids] {
            v.sort();
            v.dedup();
        }

        Ok(ProjectIdentifiers {
            project_name,
            client_name,
            repo_names,
            folder_paths,
            session_ids,
            // No reliable structured person-name source in the schema yet; C6 can
            // enrich this from session/transcript metadata if one lands.
            person_names: Vec::new(),
        })
    }

    // ── Federation ledger ─────────────────────────────────────────────

    pub async fn namespace_is_shareable(&self, namespace_id: &uuid::Uuid) -> Result<bool, String> {
        let row: Option<(bool,)> = sqlx_core::query_as::query_as(
            "SELECT s.shareable FROM sensei.namespaces n JOIN sensei.scopes s ON s.key = n.scope_key
              WHERE n.id = $1")
            .bind(namespace_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(b,)| b).unwrap_or(false))
    }

    pub async fn upsert_federated_memory(
        &self, source_id: &uuid::Uuid, remote_rule_id: &uuid::Uuid,
        content_hash: &str, memory_id: Option<&uuid::Uuid>, remote_seq: i64,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.federated_memories(knowledge_source_id, remote_rule_id, content_hash, memory_id, remote_seq)
             VALUES($1,$2,$3,$4,$5)
             ON CONFLICT(knowledge_source_id, remote_rule_id) DO UPDATE SET
               content_hash = EXCLUDED.content_hash,
               memory_id = COALESCE(EXCLUDED.memory_id, sensei.federated_memories.memory_id),
               remote_seq = EXCLUDED.remote_seq, synced_at = now()")
            .bind(source_id).bind(remote_rule_id).bind(content_hash).bind(memory_id).bind(remote_seq)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn find_federated_memory(
        &self, source_id: &uuid::Uuid, remote_rule_id: &uuid::Uuid,
    ) -> Result<Option<FederatedLink>, String> {
        let row: Option<(Option<uuid::Uuid>, i64)> = sqlx_core::query_as::query_as(
            "SELECT memory_id, remote_seq FROM sensei.federated_memories
              WHERE knowledge_source_id = $1 AND remote_rule_id = $2")
            .bind(source_id).bind(remote_rule_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(memory_id, remote_seq)| FederatedLink { memory_id, remote_seq }))
    }

    /// Retire a federated memory (tombstone pulled from upstream). Only archives
    /// federated-origin rows, so a locally-authored/promoted memory is never force-archived.
    pub async fn archive_federated_memory(&self, memory_id: &uuid::Uuid) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.memories SET status = 'archived'::sensei.memory_status
              WHERE id = $1 AND origin = 'federated'")
            .bind(memory_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// Fields to build a PublishedRule for a memory + its namespace identity.
    /// None if the memory has no namespace (unscoped).
    pub async fn memory_push_payload(&self, memory_id: &uuid::Uuid)
        -> Result<Option<MemoryPushPayload>, String> {
        let row: Option<(String, String, Option<String>, String, String, String, String, String, String)> =
            sqlx_core::query_as::query_as(
            "SELECT m.title, m.content, m.impact, m.enforcement::text, m.type::text, m.origin,
                    n.scope_key, n.slug, n.name
               FROM sensei.memories m JOIN sensei.namespaces n ON n.id = m.namespace_id
              WHERE m.id = $1")
            .bind(memory_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(title, content, impact, enforcement, rule_type, origin, scope_key, slug, name)|
            MemoryPushPayload { title, content, impact, enforcement, rule_type, origin, scope_key, slug, name }))
    }

    // ── Scoped query helpers (#60) ─────────────────────────────────────

    /// Resolve a scope identifier (project name, project UUID, or folder name)
    /// to the set of folder ids to query.  A project expands to ALL its folders
    /// (children included).  A bare folder name that has a project expands to
    /// that project's folders; a folder with no project falls back to just
    /// itself.  Returns an empty Vec if nothing matches.
    ///
    /// Resolution order:
    ///   1. `ident` matches a project by name → all that project's folder ids.
    ///   2. `ident` is a valid UUID + project with that id exists → its folders.
    ///   3. `ident` matches a repo/folder by name → if folder has a project_id,
    ///      return that project's folders; else return `[folder.id]`.
    ///   4. No match → empty Vec.
    ///
    /// Note: a bare child-folder name (kind='folder') is not resolvable here —
    /// `get_repo_by_name` only matches git/subtree/standalone roots — so it falls
    /// through to the empty Vec. Callers pass a project name/UUID or a repo name.
    pub async fn scope_folder_ids(&self, ident: &str) -> Result<Vec<uuid::Uuid>, String> {
        // (1) Try project name lookup first.
        if let Some(proj) = self.get_project_by_name(ident).await? {
            let pid = crate::api::util::json_uuid(&proj["id"])
                .ok_or_else(|| format!("scope_folder_ids: project row missing id for '{}'", ident))?;
            return self.folder_ids_for_project(&pid).await;
        }

        // (2) Try parsing ident as a UUID and look up the project directly.
        if let Ok(uid) = uuid::Uuid::parse_str(ident)
            && self.get_project(&uid).await?.is_some()
        {
            return self.folder_ids_for_project(&uid).await;
        }

        // (3) Try folder/repo lookup by name.
        if let Some(folder) = self.get_repo_by_name(ident).await? {
            let fid = crate::api::util::json_uuid(&folder["id"])
                .ok_or_else(|| format!("scope_folder_ids: folder row missing id for '{}'", ident))?;
            if let Some(pid) = crate::api::util::json_uuid(&folder["project_id"]) {
                return self.folder_ids_for_project(&pid).await;
            }
            return Ok(vec![fid]);
        }

        // (4) No match.
        Ok(vec![])
    }

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

    /// Repo-root abs_paths (git / subtree / standalone folders) among the given
    /// scope folder ids — the directories a content grep should walk. Distinct
    /// and path-ordered for determinism. Structural (workspace_member / doc /
    /// component / hook) folders are excluded so we walk repo roots, not subdirs.
    pub async fn scope_repo_roots(&self, ids: &[uuid::Uuid]) -> Result<Vec<String>, String> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT abs_path FROM sensei.folders
             WHERE id = ANY($1)
               AND kind IN ('git'::sensei.folder_kind, 'subtree'::sensei.folder_kind, 'standalone'::sensei.folder_kind)
             ORDER BY abs_path",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(p,)| p).collect())
    }

    // ── Project-scoped query variants (#60) ───────────────────────────

    /// Search functions across multiple folders (project-scoped variant).
    pub async fn search_functions_scoped(&self, folder_ids: &[uuid::Uuid], query: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, name, file_path, signature, line_start FROM sensei.nodes
             WHERE folder_id = ANY($1) AND kind IN ('function'::sensei.node_kind, 'method'::sensei.node_kind)
             AND file_path IS NOT NULL
             AND (name ILIKE '%' || $2 || '%' OR signature ILIKE '%' || $2 || '%')
             ORDER BY name LIMIT 50"
        ).bind(folder_ids).bind(query).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, fp, sig, line)| {
            serde_json::json!({ "id": id, "name": name, "file_path": fp, "signature": sig, "line_start": line })
        }).collect())
    }

    /// Search types across multiple folders (project-scoped variant).
    pub async fn search_types_scoped(&self, folder_ids: &[uuid::Uuid], query: &str) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, Option<i32>)> = sqlx_core::query_as::query_as(
            "SELECT id, name, file_path, line_start FROM sensei.nodes
             WHERE folder_id = ANY($1) AND kind IN ('class'::sensei.node_kind, 'struct'::sensei.node_kind, 'interface'::sensei.node_kind, 'enum'::sensei.node_kind, 'type'::sensei.node_kind)
             AND file_path IS NOT NULL
             AND name ILIKE '%' || $2 || '%'
             ORDER BY name LIMIT 50"
        ).bind(folder_ids).bind(query).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, name, fp, line)| {
            serde_json::json!({ "id": id, "name": name, "file_path": fp, "line_start": line })
        }).collect())
    }

    /// Count nodes by kind across multiple folders (project-scoped variant).
    pub async fn count_nodes_by_kind_scoped(&self, folder_ids: &[uuid::Uuid]) -> Result<std::collections::HashMap<String, i64>, String> {
        let rows: Vec<(String, i64)> = sqlx_core::query_as::query_as(
            "SELECT kind::text, COUNT(*) FROM sensei.nodes WHERE folder_id = ANY($1) GROUP BY kind"
        ).bind(folder_ids).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().collect())
    }

    /// Get all nodes across multiple folders (project-scoped variant).
    pub async fn get_nodes_scoped(&self, folder_ids: &[uuid::Uuid]) -> Result<Vec<serde_json::Value>, String> {
        // file_path is Option: reference stubs + lib_symbol nodes have none. The
        // whole-graph projection must decode them without erroring (they serialize
        // to a null file_path); NULLs sort last under ORDER BY file_path.
        // `fqn`/`resolved` are projected (7.2) so the Atlas can key symbols by
        // moniker and distinguish enriched defs from reference stubs. `fqn` is NULL
        // for pre-FQN/legacy rows; `resolved` is NOT NULL (defaults false).
        let rows: Vec<(uuid::Uuid, String, String, Option<String>, Option<uuid::Uuid>, Option<i32>, Option<i32>, Option<i32>, Option<i32>, uuid::Uuid, Option<String>, Option<String>, bool, bool)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, name, file_path, parent_id, line_start, line_end, degree, community_id, folder_id, language, fqn, resolved, is_test FROM sensei.nodes WHERE folder_id = ANY($1) ORDER BY file_path, line_start, parent_id, id"
        ).bind(folder_ids).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, name, fp, pid, ls, le, degree, community_id, folder_id, language, fqn, resolved, is_test)| {
            serde_json::json!({ "id": id, "kind": kind, "name": name, "file_path": fp, "parent_id": pid, "line_start": ls, "line_end": le, "degree": degree, "community_id": community_id, "folder_id": folder_id, "language": language, "fqn": fqn, "resolved": resolved, "is_test": is_test })
        }).collect())
    }

    /// Folder rows for a set of folder ids (7.2) — the structural skeleton of the
    /// `/tree` endpoint: `kind`/`role`/`parent_id` drive the folder hierarchy
    /// (repo root → sub-projects/subtrees → subfolders) that the node subtrees
    /// hang off.
    pub async fn get_folders_scoped(&self, folder_ids: &[uuid::Uuid]) -> Result<Vec<serde_json::Value>, String> {
        if folder_ids.is_empty() { return Ok(vec![]); }
        let rows: Vec<(uuid::Uuid, String, Option<String>, String, String, Option<uuid::Uuid>)> = sqlx_core::query_as::query_as(
            "SELECT id, kind::text, role::text, name, abs_path, parent_id FROM sensei.folders
              WHERE id = ANY($1) ORDER BY abs_path"
        ).bind(folder_ids).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, kind, role, name, abs_path, parent_id)| {
            serde_json::json!({ "id": id, "kind": kind, "role": role, "name": name, "abs_path": abs_path, "parent_id": parent_id })
        }).collect())
    }

    /// Get edges by kind across multiple folders (project-scoped variant).
    pub async fn get_edges_scoped(&self, folder_ids: &[uuid::Uuid], kind: &str) -> Result<Vec<serde_json::Value>, String> {
        self.get_edges_scoped_kinds(folder_ids, &[kind]).await
    }

    /// Get edges of ANY of `kinds` across multiple folders (7.1) — the graph
    /// layout set is `calls,imports,extends` (+`implements` once emitted), not the
    /// single `calls` the node view used to fetch. Each row carries its `kind` so
    /// the client can style/overlay per relationship type.
    pub async fn get_edges_scoped_kinds(&self, folder_ids: &[uuid::Uuid], kinds: &[&str]) -> Result<Vec<serde_json::Value>, String> {
        let kinds_owned: Vec<String> = kinds.iter().map(|k| k.to_string()).collect();
        let rows: Vec<(uuid::Uuid, uuid::Uuid, Option<uuid::Uuid>, Option<String>, String)> = sqlx_core::query_as::query_as(
            "SELECT id, source_id, target_id, target_name, kind::text FROM sensei.edges
              WHERE folder_id = ANY($1) AND kind::text = ANY($2)"
        ).bind(folder_ids).bind(&kinds_owned).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id, src, tgt, name, kind)| {
            serde_json::json!({ "id": id, "source_id": src, "target_id": tgt, "target_name": name, "kind": kind })
        }).collect())
    }

    // ── Gateway fallback chains + role assignments ──────────────────────
    //
    // Reads and writes for the Model Assignments wizard step. The DDL
    // model puts an optional `role` on `gateway.fallback_chains` (unique
    // when set); a chain-with-a-role IS the role assignment. Utility
    // chains (consensus-*) keep role=null and stay invisible to the
    // wizard.

    /// Return every active chain with its ordered model list. The wizard
    /// reads this to build the per-role picker; the settings page reuses
    /// it for the "which chain serves which role" table.
    pub async fn list_chains_with_models(&self) -> Result<Vec<serde_json::Value>, String> {
        // One round trip: chain metadata + JSON-aggregated members ordered
        // by sequence_order. Sqlx decodes the aggregate directly; the null
        // JSON coalesce keeps chains with no models rendering as `[]`
        // instead of the row disappearing.
        type ChainRow = (
            uuid::Uuid, String, String, Option<String>, Option<String>,
            i32, bool, serde_json::Value,
        );
        let rows: Vec<ChainRow> = sqlx_core::query_as::query_as(
            "SELECT fc.id,
                    fc.name,
                    fc.capability::text,
                    fc.role::text,
                    fc.description,
                    fc.max_fallback_attempts,
                    fc.is_active,
                    COALESCE(
                        (SELECT jsonb_agg(
                                    jsonb_build_object(
                                        'memberId',      fcm.id,
                                        'sequenceOrder', fcm.sequence_order,
                                        'modelName',     m.name,
                                        'routerName',    r.id::text
                                    ) ORDER BY fcm.sequence_order
                                )
                           FROM gateway.fallback_chain_models fcm
                           JOIN gateway.routers r ON r.id = fcm.router_id
                           JOIN gateway.models  m ON m.id = fcm.model_id
                          WHERE fcm.chain_id = fc.id AND fcm.is_active),
                        '[]'::jsonb) AS models
               FROM gateway.fallback_chains fc
              WHERE fc.is_active
              ORDER BY fc.sequence, fc.name"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|(id, name, capability, role, description, max_attempts, is_active, models)| {
            serde_json::json!({
                "id":                 id,
                "name":               name,
                "capability":         capability,
                "role":               role,
                "description":        description,
                "maxFallbackAttempts": max_attempts,
                "isActive":           is_active,
                "models":             models,
            })
        }).collect())
    }

    /// Assign (or clear) the sensei inference role a chain serves. The
    /// `role` column carries a unique-when-set index — writing a role
    /// that another chain already owns returns a database error the
    /// caller can map to a 409. Pass `None` to unassign.
    pub async fn set_chain_role(
        &self,
        chain_id: &uuid::Uuid,
        role: Option<&str>,
    ) -> Result<(), String> {
        // Cast at bind time so `None` writes SQL NULL, not the empty
        // string. `modified_at` updates so downstream diff-based reads
        // see the change.
        let result = sqlx_core::query::query(
            "UPDATE gateway.fallback_chains
                SET role = $2::sensei.inference_role,
                    modified_at = now()
              WHERE id = $1"
        )
        .bind(chain_id)
        .bind(role)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        if result.rows_affected() == 0 {
            return Err("chain not found".into());
        }
        Ok(())
    }

    // ── Chain member editing (add / remove / move) ───────────────────
    //
    // Members of a chain are rows in `gateway.fallback_chain_models`,
    // ordered by `sequence_order`. The (chain_id, sequence_order) pair
    // is unique — so writes must maintain contiguous ordering, and
    // moves happen through temporary shifts to dodge the constraint.

    /// List the models that a chain COULD use — everything with a
    /// matching capability, in any router, minus the models already
    /// present in the chain. Each row carries the model + its router
    /// so the picker can render provider chips per the mockup.
    pub async fn list_available_models_for_chain(&self, chain_id: &uuid::Uuid) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT m.id, m.name, m.full_name, r.id, r.name
               FROM gateway.models m
               JOIN gateway.models_in_router mir ON mir.model_id = m.id
               JOIN gateway.routers r ON r.id = mir.router_id
              WHERE m.capabilities @> ARRAY[(
                  SELECT fc.capability FROM gateway.fallback_chains fc WHERE fc.id = $1
              )]::sensei.model_capability[]
                AND NOT EXISTS (
                    SELECT 1 FROM gateway.fallback_chain_models fcm
                     WHERE fcm.chain_id = $1
                       AND fcm.model_id = m.id
                       AND fcm.router_id = r.id
                )
              ORDER BY r.name, m.full_name"
        ).bind(chain_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(mid, name, full, rid, rname)| {
            serde_json::json!({
                "modelId":    mid,
                "modelName":  name,
                "fullName":   full,
                "routerId":   rid,
                "routerName": rname,
            })
        }).collect())
    }

    /// Append a model to the end of a chain's ordered list. Returns the
    /// new row id and the assigned sequence_order so the caller can
    /// update its optimistic UI. Fails with a helpful message when the
    /// (model_id, router_id) pair isn't reachable via `models_in_router`.
    pub async fn add_chain_model(
        &self,
        chain_id: &uuid::Uuid,
        model_id: &uuid::Uuid,
        router_id: &uuid::Uuid,
    ) -> Result<(uuid::Uuid, i32), String> {
        // Guard: chain must exist.
        let (chain_exists,): (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM gateway.fallback_chains WHERE id = $1)"
        ).bind(chain_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        if !chain_exists {
            return Err("chain not found".into());
        }

        // Guard: the (model_id, router_id) pair must be reachable via
        // models_in_router. This is what the FK check would tell us,
        // but a clearer message helps the wizard render a useful error.
        let (reachable,): (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(
                SELECT 1 FROM gateway.models_in_router
                 WHERE model_id = $1 AND router_id = $2
             )"
        ).bind(model_id).bind(router_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        if !reachable {
            return Err("model is not reachable via this router".into());
        }

        // Next sequence_order = max + 1 (or 1 for an empty chain). The
        // unique(chain_id, sequence_order) index catches any race; on a
        // conflict we surface as-is.
        let (row_id, seq): (uuid::Uuid, i32) = sqlx_core::query_as::query_as(
            "INSERT INTO gateway.fallback_chain_models (chain_id, router_id, model_id, sequence_order)
             SELECT $1, $2, $3, COALESCE(MAX(sequence_order), 0) + 1
               FROM gateway.fallback_chain_models
              WHERE chain_id = $1
             RETURNING id, sequence_order"
        )
        .bind(chain_id).bind(router_id).bind(model_id)
        .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;

        Ok((row_id, seq))
    }

    /// Remove a chain-model row by id and compact the sequence so the
    /// remaining rows stay contiguous (1, 2, 3, …). Fails if the row
    /// isn't in the given chain — surfaces as 404 upstream.
    pub async fn remove_chain_model(
        &self,
        chain_id: &uuid::Uuid,
        member_id: &uuid::Uuid,
    ) -> Result<(), String> {
        // Two-step in a single transaction so the compaction sees the
        // deletion. The unique(chain_id, sequence_order) constraint
        // enforces the contiguous invariant.
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        let (removed_seq,): (Option<i32>,) = sqlx_core::query_as::query_as(
            "DELETE FROM gateway.fallback_chain_models
              WHERE id = $1 AND chain_id = $2
              RETURNING (sequence_order)::int"
        ).bind(member_id).bind(chain_id).fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?
         .map(|(s,)| (Some(s),)).unwrap_or((None,));

        let Some(seq) = removed_seq else {
            return Err("chain member not found".into());
        };

        // Compact: shift everyone above the removed slot down by one.
        // The unique index would collide if we did a single-step
        // decrement, so we bump the shifted rows to a negative range
        // first, then normalise.
        sqlx_core::query::query(
            "UPDATE gateway.fallback_chain_models
                SET sequence_order = -sequence_order
              WHERE chain_id = $1 AND sequence_order > $2"
        ).bind(chain_id).bind(seq).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        sqlx_core::query::query(
            "UPDATE gateway.fallback_chain_models
                SET sequence_order = -sequence_order - 1
              WHERE chain_id = $1 AND sequence_order < 0"
        ).bind(chain_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Swap a chain-model with its neighbour above (direction = -1) or
    /// below (direction = +1). No-op at boundaries. Returns Ok(false)
    /// when no swap happened so the caller can distinguish "hit
    /// boundary" from "wrote".
    pub async fn move_chain_model(
        &self,
        chain_id: &uuid::Uuid,
        member_id: &uuid::Uuid,
        direction: i32,
    ) -> Result<bool, String> {
        if direction != -1 && direction != 1 {
            return Err("direction must be -1 (up) or +1 (down)".into());
        }

        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        // Find the current sequence_order (also confirms membership).
        let cur: Option<(i32,)> = sqlx_core::query_as::query_as(
            "SELECT sequence_order FROM gateway.fallback_chain_models
              WHERE id = $1 AND chain_id = $2"
        ).bind(member_id).bind(chain_id).fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
        let Some((cur_seq,)) = cur else {
            return Err("chain member not found".into());
        };

        let target_seq = cur_seq + direction;
        if target_seq < 1 {
            return Ok(false); // Already at top.
        }

        // Locate the neighbour to swap with. If none exists at target
        // (member is last row), also a boundary.
        let neighbour: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM gateway.fallback_chain_models
              WHERE chain_id = $1 AND sequence_order = $2"
        ).bind(chain_id).bind(target_seq).fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
        let Some((neighbour_id,)) = neighbour else {
            return Ok(false); // Already at bottom.
        };

        // Three-step swap to dodge the unique(chain_id, sequence_order)
        // index: park the mover at a negative slot, move the neighbour
        // into the mover's old slot, then land the mover.
        sqlx_core::query::query(
            "UPDATE gateway.fallback_chain_models
                SET sequence_order = -$1
              WHERE id = $2"
        ).bind(cur_seq).bind(member_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        sqlx_core::query::query(
            "UPDATE gateway.fallback_chain_models
                SET sequence_order = $1
              WHERE id = $2"
        ).bind(cur_seq).bind(neighbour_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        sqlx_core::query::query(
            "UPDATE gateway.fallback_chain_models
                SET sequence_order = $1
              WHERE id = $2"
        ).bind(target_seq).bind(member_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(true)
    }

    // ── Front-door intake: playbooks / rules / guide / runs ────────────

    pub async fn list_playbooks(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, String, String, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT name, title, when_to_use, opening_tone, method_ref
               FROM sensei.playbooks WHERE enabled ORDER BY name"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(name,title,wtu,tone,mref)| serde_json::json!({
            "name":name,"title":title,"when_to_use":wtu,"opening_tone":tone,"method_ref":mref
        })).collect())
    }

    /// Fetch a single playbook by name (any enabled state), for enriching a
    /// recommendation response with its `opening_tone` + `when_to_use`.
    pub async fn get_playbook(&self, name: &str) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(String, String, String, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT name, title, when_to_use, opening_tone, method_ref
               FROM sensei.playbooks WHERE name = $1"
        ).bind(name).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(name, title, wtu, tone, mref)| serde_json::json!({
            "name": name, "title": title, "when_to_use": wtu, "opening_tone": tone, "method_ref": mref
        })))
    }

    pub async fn list_intake_guide(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(String, Option<String>, String, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT kind, axis, prompt, help FROM sensei.intake_guide WHERE enabled
              ORDER BY (kind='frame') DESC, axis"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(kind,axis,prompt,help)| serde_json::json!({
            "kind":kind,"axis":axis,"prompt":prompt,"help":help
        })).collect())
    }

    /// Returns the rule set as pure `crate::playbook::Rule`s (ready for the resolver).
    pub async fn list_playbook_rules(&self) -> Result<Vec<crate::playbook::Rule>, String> {
        use crate::playbook::{Rule, Lifecycle, Intent, Risk};
        let rows: Vec<(uuid::Uuid, String, Option<String>, Option<String>, Option<String>, String, String, i32, i32)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, match_lifecycle::text, match_intent::text, match_risk::text,
                        playbook, rationale, priority, coalesce(base_priority, priority)
                   FROM sensei.playbook_rules WHERE enabled ORDER BY priority DESC"
            ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id,name,lf,it,rk,pb,rat,pri,base_pri)| Rule {
            id: Some(id), name,
            match_lifecycle: lf.as_deref().and_then(Lifecycle::parse),
            match_intent:    it.as_deref().and_then(Intent::parse),
            match_risk:      rk.as_deref().and_then(Risk::parse),
            playbook: pb, rationale: rat, priority: pri, base_priority: base_pri,
        }).collect())
    }

    /// Snapshot the session's outcome onto confirmed, not-yet-attributed runs. Returns rows updated.
    pub async fn attribute_playbook_outcomes(&self) -> Result<u64, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.playbook_run pr
                SET outcome = s.outcome::text, outcome_ftr = s.ftr
               FROM activity.sessions s
              WHERE pr.session_id = s.id AND pr.confirmed
                AND pr.outcome IS NULL AND s.outcome IS NOT NULL"
        ).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }

    pub async fn playbook_combo_stats(&self) -> Result<Vec<crate::playbook::ComboPlaybookStat>, String> {
        use crate::playbook::{ComboPlaybookStat, Lifecycle, Intent, Risk};
        let rows: Vec<(String, String, String, String, i64, f64)> = sqlx_core::query_as::query_as(
            "SELECT lifecycle::text, intent::text, risk::text, playbook,
                    count(*)::int8, avg(outcome_ftr::int)::float8
               FROM sensei.playbook_run
              WHERE confirmed AND outcome_ftr IS NOT NULL
              GROUP BY lifecycle, intent, risk, playbook"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().filter_map(|(l,i,r,pb,n,ftr)| Some(ComboPlaybookStat {
            lifecycle: Lifecycle::parse(&l)?, intent: Intent::parse(&i)?, risk: Risk::parse(&r)?,
            playbook: pb, n, ftr_rate: ftr,
        })).collect())
    }

    /// Confirmed+attributed sample size + FTR rate for one exact (lifecycle, intent,
    /// risk, playbook) combo — the auto-select-on-trust gate's evidence lookup.
    /// Auto-select trust for a `(lifecycle, intent, risk, playbook)` combo,
    /// scoped to ONE project. A playbook run always happens in a project, so
    /// trust is "does this playbook earn FTR in THIS project" — never a global
    /// average across unrelated projects (which would auto-select on the wrong
    /// signal). Returns `(n confirmed+attributed runs, avg FTR)` for the combo
    /// within `project_id`.
    pub async fn playbook_combo_trust(
        &self, lifecycle: &str, intent: &str, risk: &str, playbook: &str, project_id: &uuid::Uuid,
    ) -> Result<(i64, f64), String> {
        let row: (i64, f64) = sqlx_core::query_as::query_as(
            "SELECT count(*)::int8, coalesce(avg(outcome_ftr::int)::float8, 0.0)
               FROM sensei.playbook_run
              WHERE confirmed AND outcome_ftr IS NOT NULL
                AND lifecycle=$1::sensei.chunk_lifecycle AND intent=$2::sensei.chunk_intent
                AND risk=$3::sensei.chunk_risk AND playbook=$4 AND project_id=$5"
        ).bind(lifecycle).bind(intent).bind(risk).bind(playbook).bind(project_id)
         .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row)
    }

    /// `classified_by` records how the axes were derived (e.g. "manual",
    /// a gateway model id, or "heuristic") and `model_fallback`
    /// flags whether the local-model path fell back to the heuristic —
    /// both feed the §9 measurement of local-model usefulness.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_playbook_run(
        &self, session_id: Option<uuid::Uuid>, feature: Option<&str>,
        lifecycle: &str, intent: &str, risk: &str,
        rule_id: Option<uuid::Uuid>, playbook: &str, rationale: &str, confirmed: bool,
        classified_by: Option<&str>, model_fallback: bool, project_id: uuid::Uuid,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.playbook_run
               (session_id, feature, lifecycle, intent, risk, rule_id, playbook, rationale, confirmed,
                classified_by, model_fallback, project_id)
             VALUES ($1,$2,$3::sensei.chunk_lifecycle,$4::sensei.chunk_intent,$5::sensei.chunk_risk,$6,$7,$8,$9,$10,$11,$12)
             RETURNING id"
        ).bind(session_id).bind(feature).bind(lifecycle).bind(intent).bind(risk)
         .bind(rule_id).bind(playbook).bind(rationale).bind(confirmed)
         .bind(classified_by).bind(model_fallback).bind(project_id)
         .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Whether `session_id` has a *confirmed* playbook_run — the gate the
    /// nudge hook (`POST /hook/nudge`) uses to decide whether to suggest
    /// `/sensei:intake`. A session with no confirmed run yet is nudged;
    /// one that already confirmed a playbook is left alone.
    pub async fn session_has_confirmed_run(&self, session_id: &uuid::Uuid) -> Result<bool, String> {
        let row: (bool,) = sqlx_core::query_as::query_as(
            "SELECT exists(SELECT 1 FROM sensei.playbook_run WHERE session_id = $1 AND confirmed)"
        ).bind(session_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Apply a §9 `learn()` plan: reweight existing rules' `priority` in place
    /// (off their immutable `base_priority`), and UPSERT proposed new rules as
    /// `source='learned', enabled=false` (invisible to the resolver's
    /// `list_playbook_rules` until accepted). Upsert targets the learned
    /// partial-unique index so re-running the same plan is idempotent.
    pub async fn apply_learn_plan(&self, plan: &crate::playbook::LearnPlan) -> Result<(), String> {
        for (id, new_priority) in &plan.reweights {
            sqlx_core::query::query("UPDATE sensei.playbook_rules SET priority = $2 WHERE id = $1")
                .bind(id).bind(new_priority).execute(&self.pool).await.map_err(|e| e.to_string())?;
        }
        for p in &plan.proposals {
            sqlx_core::query::query(
                "INSERT INTO sensei.playbook_rules
                   (name, match_lifecycle, match_intent, match_risk, playbook, rationale,
                    priority, base_priority, enabled, source)
                 VALUES ($1, $2::sensei.chunk_lifecycle, $3::sensei.chunk_intent, $4::sensei.chunk_risk,
                         $5, $6, $7, $7, false, 'learned')
                 ON CONFLICT (match_lifecycle, match_intent, match_risk, playbook)
                   WHERE source='learned'
                 DO UPDATE SET rationale = excluded.rationale, priority = excluded.priority, base_priority = excluded.priority"
            )
            .bind(format!("learned: {}", p.playbook))
            .bind(p.lifecycle.as_str()).bind(p.intent.as_str()).bind(p.risk.as_str())
            .bind(&p.playbook).bind(&p.rationale).bind(p.priority)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Pending §9 learned-rule proposals (`source='learned' AND NOT enabled`) —
    /// invisible to the resolver until accepted. Backs the accept-path list
    /// endpoint/MCP tool (Task 5) and is exercised directly by the T4 apply test.
    pub async fn list_playbook_rule_proposals(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, Option<String>, Option<String>, String, String, i32, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, name, match_lifecycle::text, match_intent::text, match_risk::text,
                        playbook, rationale, priority, created_at
                   FROM sensei.playbook_rules WHERE source='learned' AND NOT enabled ORDER BY created_at DESC"
            ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id,name,lf,it,rk,pb,rat,pri,created)| serde_json::json!({
            "id": id, "name": name, "match_lifecycle": lf, "match_intent": it, "match_risk": rk,
            "playbook": pb, "rationale": rat, "priority": pri, "created_at": created,
        })).collect())
    }

    /// Accept a §9 learned-rule proposal: flip it `enabled=true` so the resolver's
    /// `list_playbook_rules` (which filters `WHERE enabled`) picks it up. Scoped to
    /// `source='learned'` — never flips a builtin/manual rule via this path.
    ///
    /// Returns `Ok(true)` only when a row actually flipped; `Ok(false)` when no
    /// matching learned proposal exists (unknown id, or a builtin/manual rule) — so
    /// the caller can 404 instead of fabricating `{accepted}` for a no-op UPDATE.
    pub async fn accept_playbook_rule(&self, id: &uuid::Uuid) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.playbook_rules SET enabled=true WHERE id=$1 AND source='learned'"
        ).bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// FTR by `classified_by` (+ `model_fallback`) — measures whether the local
    /// gateway model's chunk classification is actually useful vs. the heuristic
    /// fallback (§9 model-stats read).
    pub async fn playbook_model_stats(&self) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(Option<String>, Option<bool>, i64, f64)> = sqlx_core::query_as::query_as(
            "SELECT classified_by, model_fallback, count(*)::int8, avg(outcome_ftr::int)::float8
               FROM sensei.playbook_run WHERE confirmed AND outcome_ftr IS NOT NULL
              GROUP BY classified_by, model_fallback ORDER BY count(*) DESC"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(cb,mf,n,ftr)| serde_json::json!({
            "classified_by": cb, "model_fallback": mf, "n": n, "ftr_rate": ftr
        })).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx_core::query_as::query_as;

    /// Test DB URL. Defaults to `sensei_test` — the throwaway DB the
    /// monorepo convention reserves for `cargo test` and CI. NEVER default
    /// to `sensei`: every test that inserts (e.g. `create_test_folder`)
    /// would leak into the user's production data, and the `/_test` row
    /// from earlier runs is a real example of how that surfaces in the UI.
    /// Override with `TEST_DATABASE_URL` for ad-hoc targets (e.g. a forked
    /// snapshot for debugging).
    fn test_db_url() -> String {
        std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| format!("postgresql://localhost:{}/sensei_test", sensei_bootstrap::POSTGRES_PORT))
    }

    #[tokio::test]
    async fn connect_to_pg() {
        let store = PgStore::connect(&test_db_url()).await.unwrap();
        let row: (i32,) = query_as("SELECT 1")
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(row.0, 1);
    }

    #[tokio::test]
    async fn execute_raw_works() {
        let store = PgStore::connect(&test_db_url()).await.unwrap();
        store.execute_raw("SELECT 1").await.unwrap();
    }

    #[tokio::test]
    async fn schema_exists() {
        let store = PgStore::connect(&test_db_url()).await.unwrap();
        let row: (bool,) = query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'sensei')"
        )
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert!(row.0, "sensei schema must exist — run `dbd apply` first");
    }

    // ── Config tests ───────────────────────────────────────────────

    async fn pg_store() -> PgStore {
        PgStore::connect(&test_db_url()).await.unwrap()
    }

    /// Generate a unique key prefix for test isolation.
    fn tkey(test: &str, key: &str) -> String {
        format!("_test:{}:{}", test, key)
    }

    #[tokio::test]
    async fn config_set_and_get() {
        let s = pg_store().await;
        let k = tkey("set_get", "theme");
        s.set_config(&k, "dark").await.unwrap();
        assert_eq!(s.get_config(&k).await.unwrap(), Some("dark".into()));
        s.delete_config(&k).await.unwrap(); // cleanup
    }

    #[tokio::test]
    async fn config_get_missing_returns_none() {
        let s = pg_store().await;
        assert_eq!(s.get_config("_test:missing:nonexistent").await.unwrap(), None);
    }

    #[tokio::test]
    async fn config_set_overwrites() {
        let s = pg_store().await;
        let k = tkey("overwrite", "k");
        s.set_config(&k, "v1").await.unwrap();
        s.set_config(&k, "v2").await.unwrap();
        assert_eq!(s.get_config(&k).await.unwrap(), Some("v2".into()));
        s.delete_config(&k).await.unwrap();
    }

    #[tokio::test]
    async fn config_delete() {
        let s = pg_store().await;
        let k = tkey("delete", "k");
        s.set_config(&k, "v").await.unwrap();
        s.delete_config(&k).await.unwrap();
        assert_eq!(s.get_config(&k).await.unwrap(), None);
    }

    #[tokio::test]
    async fn config_delete_nonexistent_is_noop() {
        let s = pg_store().await;
        s.delete_config("_test:noop:nope").await.unwrap();
    }

    #[tokio::test]
    async fn config_get_all() {
        let s = pg_store().await;
        let k1 = tkey("getall", "a");
        let k2 = tkey("getall", "b");
        s.set_config(&k1, "1").await.unwrap();
        s.set_config(&k2, "2").await.unwrap();
        let all = s.get_all_config().await.unwrap();
        assert_eq!(all[&k1], "1");
        assert_eq!(all[&k2], "2");
        s.delete_config(&k1).await.unwrap();
        s.delete_config(&k2).await.unwrap();
    }

    // ── Task executions — boot reconcile (D6b) ────────────────────────

    #[tokio::test]
    async fn reconcile_orphaned_task_executions_terminates_only_prior_session_running() {
        // D6b: on boot, a `running` task_execution row left over from a dead
        // daemon session (started before this session's start time) can never
        // complete — its in-memory task is gone. Reconcile flips it to a
        // terminal `failed`; a row from THIS session (started at/after the
        // cutoff) and an already-terminal row are both left untouched.
        let s = pg_store().await;
        let fp = format!("/_test/reconcile/{}", uuid::Uuid::new_v4());

        // A — orphaned: running, started well before the cutoff (prior session).
        let a = s.start_task_execution(1, None, "ProcessFile", &fp, "a", 0).await.unwrap();
        sqlx_core::query::query(
            "UPDATE activity.task_executions SET started_at = now() - interval '2 hours' WHERE id = $1")
            .bind(a).execute(s.pool()).await.unwrap();
        // B — this session: running, started at now() (after the cutoff).
        let b = s.start_task_execution(2, None, "ProcessFile", &fp, "b", 0).await.unwrap();
        // C — already terminal from a prior session: must not be re-touched.
        let c = s.start_task_execution(3, None, "ProcessFile", &fp, "c", 0).await.unwrap();
        sqlx_core::query::query(
            "UPDATE activity.task_executions SET status = 'completed', started_at = now() - interval '2 hours' WHERE id = $1")
            .bind(c).execute(s.pool()).await.unwrap();

        // Cutoff sits between the prior-session rows (−2h) and this session (now).
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(1);
        // D — boundary: running, started EXACTLY at the cutoff. The sweep is
        // exclusive (`started_at < cutoff`), so a row at session_start belongs
        // to this session and must be left running — locks the `<` vs `<=` line.
        let d = s.start_task_execution(4, None, "ProcessFile", &fp, "d", 0).await.unwrap();
        sqlx_core::query::query(
            "UPDATE activity.task_executions SET started_at = $2 WHERE id = $1")
            .bind(d).bind(cutoff).execute(s.pool()).await.unwrap();

        let n = s.reconcile_orphaned_task_executions(cutoff).await.unwrap();
        assert!(n >= 1, "at least the one orphaned running row is reconciled, got {n}");

        let (a_status, a_completed, a_err): (String, Option<chrono::DateTime<chrono::Utc>>, Option<String>) =
            query_as("SELECT status, completed_at, error_message FROM activity.task_executions WHERE id = $1")
                .bind(a).fetch_one(s.pool()).await.unwrap();
        assert_eq!(a_status, "failed", "orphaned running row is marked failed");
        assert!(a_completed.is_some(), "reconciled row gets a completed_at");
        assert!(a_err.is_some(), "reconciled row records why it was terminated");

        let (b_status, b_completed): (String, Option<chrono::DateTime<chrono::Utc>>) =
            query_as("SELECT status, completed_at FROM activity.task_executions WHERE id = $1")
                .bind(b).fetch_one(s.pool()).await.unwrap();
        assert_eq!(b_status, "running", "this session's in-flight row is left running");
        assert!(b_completed.is_none(), "this session's row keeps a null completed_at");

        let (c_status,): (String,) =
            query_as("SELECT status FROM activity.task_executions WHERE id = $1")
                .bind(c).fetch_one(s.pool()).await.unwrap();
        assert_eq!(c_status, "completed", "an already-terminal row is not re-touched");

        let (d_status,): (String,) =
            query_as("SELECT status FROM activity.task_executions WHERE id = $1")
                .bind(d).fetch_one(s.pool()).await.unwrap();
        assert_eq!(d_status, "running", "a row exactly at the cutoff is this session's — left running");

        // Cleanup.
        sqlx_core::query::query("DELETE FROM activity.task_executions WHERE folder_path = $1")
            .bind(&fp).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn start_task_execution_records_retry_number() {
        // D6c: a re-driven task carries its attempt count, and the execution
        // record must persist it (`task_executions.retry_number`, currently
        // always 0) so retries are observable on the logs/health screen.
        let s = pg_store().await;
        let fp = format!("/_test/retrynum/{}", uuid::Uuid::new_v4());
        let id = s.start_task_execution(77, None, "ProcessFile", &fp, "a.rs", 2).await.unwrap();

        let (rn,): (i32,) = query_as("SELECT retry_number FROM activity.task_executions WHERE id = $1")
            .bind(id).fetch_one(s.pool()).await.unwrap();
        assert_eq!(rn, 2, "the recorded retry_number matches the attempt");

        sqlx_core::query::query("DELETE FROM activity.task_executions WHERE folder_path = $1")
            .bind(&fp).execute(s.pool()).await.unwrap();
    }

    // ── Scan exclusions (per watch root) ──────────────────────────────

    #[tokio::test]
    async fn root_exclusion_prefixes_resolves_relative_entries_against_root() {
        let s = pg_store().await;
        let uniq = uuid::Uuid::new_v4();
        let root = format!("/_test/exroot/{uniq}");
        let id = s.add_watch_root(&root, "ex", &serde_json::json!(["Code", "archive/old"])).await.unwrap();
        let mut prefixes = s.root_exclusion_prefixes(&root).await.unwrap();
        prefixes.sort();
        assert_eq!(prefixes, vec![format!("{root}/Code"), format!("{root}/archive/old")]);
        // get_watch_root round-trips the raw relative list.
        let (path, ex) = s.get_watch_root(&id).await.unwrap().unwrap();
        assert_eq!(path, root);
        assert!(ex.contains(&"Code".to_string()));
        s.remove_watch_root(&id).await.ok();
    }

    #[tokio::test]
    async fn prune_under_prefix_deletes_subtree_keeps_siblings() {
        // Exclusion prune: every folder at or under the prefix is deleted; a
        // sibling that only shares the prefix string is kept (boundary-safe).
        let s = pg_store().await;
        let uniq = uuid::Uuid::new_v4();
        let root_path = format!("/_test/prune_prefix/{uniq}");
        let root_id = s.add_watch_root(&root_path, "prune_prefix_root", &serde_json::json!([])).await.unwrap();

        let code = format!("{root_path}/Code");
        let inside = format!("{code}/archive/repo");
        let sibling = format!("{root_path}/Coder"); // shares the "Code" prefix string
        let code_fid = s.upsert_repo_kind(&root_id, "git", "Code", &code).await.unwrap();
        s.upsert_repo_kind(&root_id, "git", "repo", &inside).await.unwrap();
        let sib_fid = s.upsert_repo_kind(&root_id, "git", "Coder", &sibling).await.unwrap();

        let deleted = s.prune_under_prefix(&code).await.unwrap();
        assert_eq!(deleted, 2, "the prefix folder and its descendant are deleted");

        for (fid, alive, msg) in [(code_fid, 0, "prefix folder deleted"), (sib_fid, 1, "sibling kept")] {
            let (n,): (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.folders WHERE id=$1")
                .bind(fid).fetch_one(s.pool()).await.unwrap();
            assert_eq!(n, alive, "{msg}");
        }
        // cleanup
        s.prune_under_prefix(&root_path).await.unwrap();
    }

    #[tokio::test]
    async fn prune_empty_projects_grace_protects_fresh() {
        // A just-created empty `discovery` project must survive a grace>0 prune —
        // its folder may still be attaching in a concurrent step. (The grace=0
        // path is deliberate/global and unsafe to exercise in the shared test DB,
        // so it's covered by the exclusion handler in production, not here.)
        let s = pg_store().await;
        let fresh = s.create_project(&format!("_test:grace-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        s.prune_empty_projects(60).await.unwrap();
        let (alive,): (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.projects WHERE id=$1")
            .bind(fresh).fetch_one(s.pool()).await.unwrap();
        assert_eq!(alive, 1, "grace protects a fresh empty project");
        // Direct cleanup (not a grace=0 global prune, which would hit sibling tests).
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id=$1").bind(fresh).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn tag_file_nodes_by_framework_kind_aggregates_symbol_kinds() {
        // G5b: a file node gets tagged with the framework kinds of the symbols it
        // contains, so `get_patterns`/`get_file_tags` return real files. A file
        // with no framework symbols stays untagged; a stale tag is cleared.
        let s = pg_store().await;
        let uniq = uuid::Uuid::new_v4();
        let root_id = s.add_watch_root(&format!("/_test/tag/{uniq}"), "tag_root", &serde_json::json!([])).await.unwrap();
        let fid = s.upsert_repo_kind(&root_id, "git", "repo", &format!("/_test/tag/{uniq}/repo")).await.unwrap();

        // A .svelte file that defines a component and uses a hook.
        let widget = s.upsert_node(&fid, "file", "Widget.svelte", "src/Widget.svelte", None, None, None, None).await.unwrap();
        s.upsert_node(&fid, "component", "Widget", "src/Widget.svelte", None, None, None, None).await.unwrap();
        s.upsert_node(&fid, "hook", "effect", "src/Widget.svelte", None, None, None, None).await.unwrap();
        // A plain file with only a function → no framework tag.
        let util = s.upsert_node(&fid, "file", "util.rs", "src/util.rs", None, None, None, None).await.unwrap();
        s.upsert_node(&fid, "function", "helper", "src/util.rs", None, None, None, None).await.unwrap();

        // File-role by path convention (no symbols needed): SvelteKit routes +
        // middleware, and a Next-style middleware file.
        let page = s.upsert_node(&fid, "file", "+page.svelte", "src/routes/blog/+page.svelte", None, None, None, None).await.unwrap();
        let endpoint = s.upsert_node(&fid, "file", "+server.ts", "src/routes/api/+server.ts", None, None, None, None).await.unwrap();
        let hooks = s.upsert_node(&fid, "file", "hooks.server.ts", "src/hooks.server.ts", None, None, None, None).await.unwrap();
        let mw = s.upsert_node(&fid, "file", "middleware.ts", "middleware.ts", None, None, None, None).await.unwrap();

        let changed = s.tag_file_nodes_by_framework_kind(&root_id).await.unwrap();
        assert!(changed >= 1, "at least the component/hook file is tagged");

        let (widget_tags,): (Vec<String>,) = sqlx_core::query_as::query_as("SELECT tags FROM sensei.nodes WHERE id=$1")
            .bind(widget).fetch_one(s.pool()).await.unwrap();
        assert_eq!(widget_tags, vec!["component".to_string(), "hook".to_string()], "file tagged with its symbol kinds (sorted)");
        let (util_tags,): (Vec<String>,) = sqlx_core::query_as::query_as("SELECT tags FROM sensei.nodes WHERE id=$1")
            .bind(util).fetch_one(s.pool()).await.unwrap();
        assert!(util_tags.is_empty(), "a file with no framework symbols stays untagged");

        // File-role tags come from the path convention alone.
        let pool_ref = s.pool();
        let tags_of = |id: uuid::Uuid| async move {
            let (t,): (Vec<String>,) = sqlx_core::query_as::query_as("SELECT tags FROM sensei.nodes WHERE id=$1")
                .bind(id).fetch_one(pool_ref).await.unwrap();
            t
        };
        assert_eq!(tags_of(page).await, vec!["route".to_string()], "+page.svelte → route");
        assert_eq!(tags_of(endpoint).await, vec!["route".to_string()], "+server.ts → route");
        assert_eq!(tags_of(hooks).await, vec!["middleware".to_string()], "hooks.server.ts → middleware");
        assert_eq!(tags_of(mw).await, vec!["middleware".to_string()], "middleware.ts → middleware");

        // Idempotent: a second run changes nothing.
        assert_eq!(s.tag_file_nodes_by_framework_kind(&root_id).await.unwrap(), 0, "no-op on re-run");

        // cleanup
        let pool = s.pool();
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE folder_id=$1").bind(fid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id=$1").bind(root_id).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id=$1").bind(root_id).execute(pool).await.ok();
    }

    /// Create a unique test folder for FK tests. Uses suffix for isolation.
    async fn create_test_folder(s: &PgStore, suffix: &str) -> uuid::Uuid {
        use sqlx_core::query_as::query_as;
        s.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) ON CONFLICT DO NOTHING"
        ).await.unwrap();
        let abs_path = format!("/_test/{}", suffix);
        let row: (uuid::Uuid,) = query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path) VALUES('00000000-0000-0000-0000-000000000001', 'git'::sensei.folder_kind, $1, $1, $2) ON CONFLICT(abs_path) DO UPDATE SET name = EXCLUDED.name RETURNING id"
        ).bind(suffix).bind(&abs_path).fetch_one(s.pool()).await.unwrap();
        row.0
    }

    /// Create a unique (project, folder) pair for FK tests that need both,
    /// wiring the folder to the project. Used by the pattern tests since
    /// detected_patterns is project-scoped (#82) and needs a non-null
    /// project_id, while `list_patterns_by_folder` still keys on folder.
    async fn create_test_project_and_folder(s: &PgStore, suffix: &str) -> (uuid::Uuid, uuid::Uuid) {
        let pid = s.create_project(&format!("_test:{}", suffix), None, None).await.unwrap();
        let fid = create_test_folder(s, suffix).await;
        sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
            .bind(pid).bind(fid)
            .execute(s.pool()).await.unwrap();
        (pid, fid)
    }

    // ── Dōjō confidentiality: project identifiers (C5) ─────────────────

    #[test]
    fn repo_tokens_from_remote_parses_ssh_and_https() {
        assert_eq!(
            repo_tokens_from_remote("git@github.com:acme/acme-api.git"),
            vec!["acme-api".to_string(), "acme".to_string()]
        );
        assert_eq!(
            repo_tokens_from_remote("https://github.com/acme/acme-api"),
            vec!["acme-api".to_string(), "acme".to_string()]
        );
        // Host-like and empty segments are skipped.
        assert!(repo_tokens_from_remote("https://example.com").is_empty());
    }

    #[test]
    fn remote_owner_slug_extracts_lowercased_owner() {
        // ssh + https, mixed case → the owner (segment before the repo), lowercased.
        assert_eq!(remote_owner_slug("git@github.com:Sensei-HQ/sensei.git").as_deref(), Some("sensei-hq"));
        assert_eq!(remote_owner_slug("https://github.com/Sensei-HQ/sensei").as_deref(), Some("sensei-hq"));
        assert_eq!(remote_owner_slug("https://gitlab.com/acme/api.git").as_deref(), Some("acme"));
        // No owner segment / unparseable → None.
        assert_eq!(remote_owner_slug("https://example.com"), None);
        assert_eq!(remote_owner_slug(""), None);
    }

    #[tokio::test]
    async fn suggest_binding_infers_from_git_owner_then_stops_once_bound() {
        let Ok(s) = PgStore::connect_test().await else { return; };
        let suffix = format!("suggestbind_{}", uuid::Uuid::new_v4());
        let (pid, fid) = create_test_project_and_folder(&s, &suffix).await;
        // The project's repo is owned by "Acme" (mixed case in the remote).
        sqlx_core::query::query("UPDATE sensei.folders SET remote_urls = $2 WHERE id = $1")
            .bind(fid)
            .bind(serde_json::json!([{ "name": "origin", "url": "git@github.com:Acme/widget.git" }]))
            .execute(s.pool()).await.unwrap();
        assert_eq!(s.project_org_owners(&pid).await.unwrap(), vec!["acme".to_string()], "owner parsed + lowercased");

        // No membership connected yet → no suggestion.
        assert!(crate::dojo::memberships::suggest_binding(&s, &pid).await.unwrap().is_none());

        // Connect a client membership covering "acme".
        let mid = uuid::Uuid::new_v4();
        s.create_dojo_membership(&NewDojoMembership {
            id: mid, registry_url: "http://localhost:7755".into(), tenant_key: "github/acme".into(),
            dojo_url: "http://localhost:7755/github/acme".into(), kind: "client".into(),
            org_slugs: vec!["acme".into()],
            role: "contributor".into(), authenticated_via: "device_code".into(),
            attribution_default: "anonymous".into(),
            credential_ref: format!("dojo-{}", uuid::Uuid::new_v4()), sync_status: "healthy".into(),
        }).await.unwrap();

        // Now it suggests that membership, explaining which owner matched.
        let sug = crate::dojo::memberships::suggest_binding(&s, &pid).await.unwrap().expect("a suggestion");
        assert_eq!(sug.membership_id, mid);
        assert_eq!(sug.kind, "client");
        assert_eq!(sug.matched_slug, "acme");
        assert_eq!(sug.tenant_key, "github/acme");

        // Once the project is bound, the chip no longer applies.
        assert!(s.bind_project_to_dojo(&pid, Some(&mid)).await.unwrap());
        assert!(crate::dojo::memberships::suggest_binding(&s, &pid).await.unwrap().is_none());

        s.bind_project_to_dojo(&pid, None).await.unwrap();
        s.delete_dojo_membership(&mid).await.unwrap();
    }

    #[tokio::test]
    async fn project_identifiers_gathers_names_paths_repos_and_sessions() {
        let s = pg_store().await;
        let suffix = format!("projident_{}", uuid::Uuid::new_v4());
        let (pid, fid) = create_test_project_and_folder(&s, &suffix).await;
        // Set the client + a git remote so the parser has something to chew on.
        sqlx_core::query::query(
            "UPDATE sensei.projects SET client = $2 WHERE id = $1",
        )
        .bind(pid)
        .bind("Acme Corp")
        .execute(s.pool())
        .await
        .unwrap();
        sqlx_core::query::query(
            "UPDATE sensei.folders SET remote_urls = $2 WHERE id = $1",
        )
        .bind(fid)
        .bind(serde_json::json!([{ "name": "origin", "url": "git@github.com:acme/acme-api.git" }]))
        .execute(s.pool())
        .await
        .unwrap();
        // A session for the project, with a client_session_id.
        let csid = format!("cs-{suffix}");
        s.record_session_event(&csid, &fid, Some(&pid), "claude", true).await.unwrap();

        let ids = s.project_identifiers(&pid).await.unwrap();
        assert_eq!(ids.project_name.as_deref(), Some(format!("_test:{suffix}").as_str()));
        assert_eq!(ids.client_name.as_deref(), Some("Acme Corp"));
        assert!(ids.repo_names.iter().any(|r| r == "acme-api"), "repo from remote missing: {:?}", ids.repo_names);
        assert!(ids.repo_names.iter().any(|r| r == "acme"), "owner from remote missing: {:?}", ids.repo_names);
        assert!(ids.folder_paths.iter().any(|p| p.contains(&suffix)), "folder path missing: {:?}", ids.folder_paths);
        assert!(ids.session_ids.iter().any(|sid| sid == &csid), "client_session_id missing: {:?}", ids.session_ids);
        // The observatory session UUID is also present.
        assert!(ids.session_ids.len() >= 2, "expected uuid + client_session_id: {:?}", ids.session_ids);

        // Cleanup (project delete cascades folders/sessions via FKs).
        s.delete_project(&pid).await.ok();
    }

    // ── PG Function tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn rank_bm25_returns_results() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("bm25_{}", uuid::Uuid::new_v4())).await;
        s.upsert_node(&fid, "function", "authenticate_user", "src/auth.rs", None, Some("fn authenticate_user(token: &str)"), Some(1), Some(20)).await.unwrap();
        s.upsert_node(&fid, "function", "validate_email", "src/validation.rs", None, Some("fn validate_email(email: &str)"), Some(1), Some(10)).await.unwrap();
        let results = s.rank_bm25(&fid, "authenticate").await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "src/auth.rs");
        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn rank_bm25_empty_folder() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("bm25_empty_{}", uuid::Uuid::new_v4())).await;
        let results = s.rank_bm25(&fid, "anything").await.unwrap();
        assert!(results.is_empty());
    }

    // ── Nodes + Edges tests ────────────────────────────────────────────

    #[tokio::test]
    async fn node_upsert_and_query() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("node_{}", uuid::Uuid::new_v4())).await;
        let file_id = s.upsert_node(&fid, "file", "main.rs", "src/main.rs", None, None, None, None).await.unwrap();
        let fn_id = s.upsert_node(&fid, "function", "main", "src/main.rs", Some(&file_id), Some("fn main()"), Some(1), Some(10)).await.unwrap();
        let nodes = s.get_nodes_by_folder(&fid).await.unwrap();
        assert_eq!(nodes.len(), 2);
        let by_file = s.get_nodes_by_file(&fid, "src/main.rs").await.unwrap();
        assert_eq!(by_file.len(), 2);
        s.delete_nodes_by_folder(&fid).await.unwrap();
        assert_eq!(s.get_nodes_by_folder(&fid).await.unwrap().len(), 0);
        let _ = (file_id, fn_id);
    }

    #[tokio::test]
    async fn upsert_persists_doc_and_symbol_kinds() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("kinds_{}", uuid::Uuid::new_v4())).await;
        // Each of these failed the enum cast before the fix and was dropped.
        for (kind, name, path) in [
            ("doc", "README", "README.md"),
            ("struct", "Point", "src/geo.rs"),
            ("component", "Button", "src/Button.svelte"),
            ("hook", "useState", "src/Button.svelte"),
            ("extension", "review", "marketplace/commands/review.md"),
        ] {
            s.upsert_node(&fid, kind, name, path, None, None, Some(1), Some(2))
                .await
                .unwrap_or_else(|e| panic!("upsert {kind} failed: {e}"));
        }
        let kinds = s.count_nodes_by_kind(&fid).await.unwrap();
        for kind in ["doc", "struct", "component", "hook", "extension"] {
            assert_eq!(kinds.get(kind), Some(&1), "missing {kind} node");
        }
        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn doc_nodes_are_embeddable() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("embed_{}", uuid::Uuid::new_v4())).await;
        s.upsert_node(&fid, "doc", "README", "README.md", None, None, Some(1), Some(2))
            .await.unwrap();
        let pending = s.nodes_without_embeddings(&fid, 100).await.unwrap();
        assert!(
            pending.iter().any(|(_, kind, name, _, _)| kind == "doc" && name == "README"),
            "doc node not returned by nodes_without_embeddings"
        );
        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn semantic_search_nodes_ranks_by_cosine() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("sem_{}", uuid::Uuid::new_v4())).await;

        // Two function nodes whose *names* share no keyword with the query.
        // 384-dim (matches the vector(384) column). `alpha` points along dim 0,
        // `beta` along dim 1 — orthogonal, so the query vector's direction alone
        // decides ranking (purely semantic, no lexical overlap).
        let dim = 384usize;
        let mut e_alpha = vec![0.0f32; dim];
        e_alpha[0] = 1.0;
        let mut e_beta = vec![0.0f32; dim];
        e_beta[1] = 1.0;

        let id_alpha = s.upsert_node(&fid, "function", "alpha", "a.rs", None, None, Some(1), Some(9)).await.unwrap();
        let id_beta = s.upsert_node(&fid, "function", "beta", "b.rs", None, None, Some(1), Some(9)).await.unwrap();
        s.set_node_embedding(&id_alpha, &e_alpha).await.unwrap();
        s.set_node_embedding(&id_beta, &e_beta).await.unwrap();

        // Query vector leans toward alpha's direction.
        let mut query = vec![0.0f32; dim];
        query[0] = 0.9;
        query[1] = 0.1;

        let hits = s
            .semantic_search_nodes(&[fid], &query, &["function", "method"], 10)
            .await
            .unwrap();

        let names: Vec<&str> = hits.iter().map(|(_, name, ..)| name.as_str()).collect();
        assert!(names.contains(&"alpha") && names.contains(&"beta"), "both nodes should surface, got {names:?}");
        assert_eq!(names.first(), Some(&"alpha"), "alpha is the closest by cosine — must rank first, got {names:?}");

        // A kind filter that matches neither node returns nothing.
        let none = s.semantic_search_nodes(&[fid], &query, &["class"], 10).await.unwrap();
        assert!(none.is_empty(), "kind filter should exclude functions, got {none:?}");

        // Empty inputs are cheap no-ops, never a query.
        assert!(s.semantic_search_nodes(&[], &query, &["function"], 10).await.unwrap().is_empty());
        assert!(s.semantic_search_nodes(&[fid], &[], &["function"], 10).await.unwrap().is_empty());

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn edge_insert_and_query() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("edge_{}", uuid::Uuid::new_v4())).await;
        let fn_a = s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();
        let fn_b = s.upsert_node(&fid, "function", "b", "b.rs", None, None, Some(1), Some(5)).await.unwrap();
        s.insert_edge(&fid, &fn_a, Some(&fn_b), None, None, "calls").await.unwrap();
        let callers = s.get_callers(&fn_b).await.unwrap();
        assert_eq!(callers.len(), 1);
        assert_eq!(callers[0]["caller_id"], fn_a.to_string());
        let callees = s.get_callees(&fn_a).await.unwrap();
        assert_eq!(callees.len(), 1);
        let by_kind = s.get_edges_by_kind(&fid, "calls").await.unwrap();
        assert_eq!(by_kind.len(), 1);
        s.delete_nodes_by_folder(&fid).await.unwrap(); // cascades edges
    }

    #[tokio::test]
    async fn insert_edge_is_idempotent() {
        // D1: edges have identity — inserting the same edge twice returns the
        // SAME id and adds no second row, for both resolved and unresolved edges.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("edgeidem_{}", uuid::Uuid::new_v4())).await;
        let a = s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();
        let b = s.upsert_node(&fid, "function", "b", "b.rs", None, None, Some(1), Some(5)).await.unwrap();

        // Resolved edge: a repeated identical insert upserts to the same row.
        let e1 = s.insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap();
        let e2 = s.insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap();
        assert_eq!(e1, e2, "a repeated resolved edge returns the same id");
        assert_eq!(s.get_edges_by_kind(&fid, "calls").await.unwrap().len(), 1, "no duplicate resolved edge");

        // Unresolved edge: a repeated insert (same source, target_name, kind)
        // upserts to the same row (nulls-not-distinct target_file).
        let u1 = s.insert_edge(&fid, &a, None, Some("ext_fn"), None, "calls").await.unwrap();
        let u2 = s.insert_edge(&fid, &a, None, Some("ext_fn"), None, "calls").await.unwrap();
        assert_eq!(u1, u2, "a repeated unresolved edge returns the same id");
        assert_eq!(s.get_edges_by_kind(&fid, "calls").await.unwrap().len(), 2,
            "one resolved (a→b) + one unresolved (a→ext_fn), no dupes");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn resolve_edge_merges_into_existing_resolved_edge() {
        // D1: promoting an unresolved edge to a target that already has a resolved
        // edge from the same (source, kind) must MERGE (delete the loser), not
        // throw a unique violation against edges_unique_resolved.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("resolvemerge_{}", uuid::Uuid::new_v4())).await;
        let a = s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();
        let b = s.upsert_node(&fid, "function", "b", "b.rs", None, None, Some(1), Some(5)).await.unwrap();

        s.insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap(); // resolved a→b
        let u = s.insert_edge(&fid, &a, None, Some("b"), None, "calls").await.unwrap(); // unresolved a→"b"
        // The resolved and unresolved partial indexes are DISJOINT: both edges
        // coexist (no collision on insert) until resolution merges them.
        assert_eq!(s.get_edges_by_kind(&fid, "calls").await.unwrap().len(), 2,
            "resolved a→b and unresolved a→\"b\" coexist as two rows");

        s.resolve_edge(&u, &b).await.unwrap(); // collides with a→b → merge

        assert_eq!(s.get_edges_by_kind(&fid, "calls").await.unwrap().len(), 1,
            "the redundant edge is merged away, not duplicated");
        let exists: (bool,) = query_as("SELECT EXISTS(SELECT 1 FROM sensei.edges WHERE id=$1)")
            .bind(u).fetch_one(s.pool()).await.unwrap();
        assert!(!exists.0, "the loser unresolved edge is deleted");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn replace_communities_for_folder_kills_stale_and_orphans() {
        // D4 invariant 5: the per-folder replace DELETEs stale community rows,
        // CLEARs every node's community_id, then writes the new set — no orphaned
        // rows, no stranded community_ids. Per-folder, sum(node_count) equals the
        // count of nodes actually carrying a community_id.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("comm_{}", uuid::Uuid::new_v4())).await;
        let a = s.upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2)).await.unwrap();
        let b = s.upsert_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(3), Some(4)).await.unwrap();
        let c = s.upsert_node(&fid, "function", "c", "a.rs", None, Some("()"), Some(5), Some(6)).await.unwrap();

        // Stale prior state: community 99 + nodes a & c assigned to it.
        s.upsert_community(&fid, 99, "stale", 2).await.unwrap();
        s.update_node_community(&a, 99).await.unwrap();
        s.update_node_community(&c, 99).await.unwrap();

        // Replace with a single community {1: [a, b]} — c must be orphaned out.
        s.replace_communities_for_folder(&fid, &[
            CommunityAssignment { community_id: 1, label: "new".into(), member_node_ids: vec![a, b], god_node_ids: vec![a] },
        ]).await.unwrap();

        assert_eq!(s.list_communities(&fid).await.unwrap().len(), 1, "stale community 99 is gone");
        let cid = |id: uuid::Uuid| {
            let s = &s;
            async move {
                let (v,): (Option<i32>,) = query_as("SELECT community_id FROM sensei.nodes WHERE id=$1")
                    .bind(id).fetch_one(s.pool()).await.unwrap();
                v
            }
        };
        assert_eq!(cid(a).await, Some(1), "a assigned to the new community");
        assert_eq!(cid(b).await, Some(1), "b assigned to the new community");
        assert_eq!(cid(c).await, None, "c's stale community_id is cleared (orphan removed)");

        // Per-folder integrity: claimed node_count == real nodes carrying a community_id.
        let (claimed,): (i64,) = query_as("SELECT COALESCE(sum(node_count),0) FROM inference.communities WHERE folder_id=$1")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
        let (real,): (i64,) = query_as("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND community_id IS NOT NULL")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(claimed, real, "claimed == real (invariant 5)");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn detect_communities_assigns_deterministic_ids_by_natural_key() {
        // D4 invariant 2: community_id is DETERMINISTIC — communities are ranked
        // 1..k by the natural key (file_path, line_start, …) of their smallest
        // member, so an identical graph always yields identical ids. Two disjoint
        // triangles ⇒ two communities; the one holding the earliest node is #1.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("commdet_{}", uuid::Uuid::new_v4())).await;
        let mut n = std::collections::HashMap::new();
        for (name, line) in [("a", 10), ("b", 20), ("c", 30), ("d", 40), ("e", 50), ("f", 60)] {
            let id = s.upsert_node(&fid, "function", name, "a.rs", None, Some("()"), Some(line), Some(line + 1)).await.unwrap();
            n.insert(name, id);
        }
        // Two disjoint triangles: {a,b,c} and {d,e,f} (resolved calls).
        for (x, y) in [("a","b"),("b","c"),("c","a"),("d","e"),("e","f"),("f","d")] {
            s.insert_edge(&fid, &n[x], Some(&n[y]), None, None, "calls").await.unwrap();
        }

        let read_ids = |s: &PgStore, n: &std::collections::HashMap<&str, uuid::Uuid>| {
            let ids: Vec<uuid::Uuid> = ["a","b","c","d","e","f"].iter().map(|k| n[*k]).collect();
            let pool = s.pool().clone();
            async move {
                let mut out = Vec::new();
                for id in ids {
                    let (v,): (Option<i32>,) = query_as("SELECT community_id FROM sensei.nodes WHERE id=$1")
                        .bind(id).fetch_one(&pool).await.unwrap();
                    out.push(v);
                }
                out
            }
        };

        crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();
        let first: Vec<Option<i32>> = read_ids(&s, &n).await;
        // {a,b,c} share a community; {d,e,f} share another; triangle-a (earliest
        // natural key) is community 1, triangle-d is 2.
        assert_eq!(&first[0..3], &[Some(1), Some(1), Some(1)], "earliest triangle is community 1");
        assert_eq!(&first[3..6], &[Some(2), Some(2), Some(2)], "later triangle is community 2");

        // Invariant 5 after a REAL detect run (not just the hand-built replace):
        // claimed sum(node_count) == real nodes carrying a community_id.
        let (claimed,): (i64,) = query_as("SELECT COALESCE(sum(node_count),0) FROM inference.communities WHERE folder_id=$1")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
        let (real,): (i64,) = query_as("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND community_id IS NOT NULL")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(claimed, real, "per-folder claimed == real after detect_communities (invariant 5)");

        // Re-running over the identical graph yields the identical assignment.
        crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();
        let second: Vec<Option<i32>> = read_ids(&s, &n).await;
        assert_eq!(first, second, "identical graph ⇒ identical community ids (deterministic)");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn detect_communities_clears_stale_on_empty_folder() {
        // D4 invariant 5: running detection on a folder that has become empty
        // clears its stale community rows (the nodes.is_empty() → replace([]) path).
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("commempty_{}", uuid::Uuid::new_v4())).await;
        s.upsert_community(&fid, 1, "stale", 3).await.unwrap();
        assert_eq!(s.list_communities(&fid).await.unwrap().len(), 1, "seeded a stale community");

        // No nodes exist for this folder → detection must clear the stale rows.
        crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();
        assert!(s.list_communities(&fid).await.unwrap().is_empty(),
            "an empty folder's stale communities are cleared");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn community_coverage_full_singletons_inherit_file_community() {
        // D4.4: every node gets a community_id. A file's symbols with NO call/
        // import edge still land in a community via `parent_id` containment
        // (they cluster under the file), and any residual singleton inherits its
        // enclosing file community — so coverage is ~100% (invariant 5), not just
        // the nodes that happen to carry a resolved semantic edge.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("commcov_{}", uuid::Uuid::new_v4())).await;
        // A file with a struct + two methods, and NO edges between any of them.
        let file = s.upsert_node(&fid, "file", "widget.rs", "src/widget.rs", None, None, Some(1), Some(99)).await.unwrap();
        s.upsert_node(&fid, "struct", "Widget", "src/widget.rs", Some(&file), None, Some(2), Some(2)).await.unwrap();
        s.upsert_node(&fid, "method", "new", "src/widget.rs", Some(&file), Some("() -> Self"), Some(3), Some(5)).await.unwrap();
        s.upsert_node(&fid, "method", "render", "src/widget.rs", Some(&file), Some("(&self)"), Some(7), Some(20)).await.unwrap();

        crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
        let (covered,): (i64,) = query_as("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND community_id IS NOT NULL")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(total, 4, "seeded 4 nodes");
        assert_eq!(covered, total, "every node carries a community_id (singletons inherit the file community)");

        // per-folder integrity still holds with the broadened coverage.
        let (claimed,): (i64,) = query_as("SELECT COALESCE(sum(node_count),0) FROM inference.communities WHERE folder_id=$1")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(claimed, covered, "claimed == real (invariant 5)");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn community_adjacency_includes_extends() {
        // D4.4: the adjacency set is broadened to calls,imports,extends,references
        // (the dead `implements` is dropped). Two classes in DIFFERENT files
        // (so `parent_id` containment does NOT group them) linked only by an
        // `extends` edge land in the SAME community — before D4b, `extends` was
        // ignored and they would be separate singletons.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("commext_{}", uuid::Uuid::new_v4())).await;
        let base = s.upsert_node(&fid, "class", "Base", "src/base.rs", None, Some("class Base"), Some(1), Some(5)).await.unwrap();
        let derived = s.upsert_node(&fid, "class", "Derived", "src/derived.rs", None, Some("class Derived"), Some(1), Some(5)).await.unwrap();
        s.insert_edge(&fid, &derived, Some(&base), None, None, "extends").await.unwrap();

        crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

        let cid = |id: uuid::Uuid| {
            let pool = s.pool().clone();
            async move {
                let (v,): (Option<i32>,) = query_as("SELECT community_id FROM sensei.nodes WHERE id=$1")
                    .bind(id).fetch_one(&pool).await.unwrap();
                v
            }
        };
        let cb = cid(base).await;
        let cd = cid(derived).await;
        assert!(cb.is_some(), "extends-linked class carries a community");
        assert_eq!(cb, cd, "extends-linked classes share a community (broadened adjacency)");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn recompute_degrees_counts_incident_edges() {
        // D4.5: nodes.degree = in+out count of edges incident to the node (source,
        // plus resolved target). An edgeless node is set to 0, not left NULL.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("degree_{}", uuid::Uuid::new_v4())).await;
        let hub = s.upsert_node(&fid, "function", "hub", "a.rs", None, Some("()"), Some(1), Some(2)).await.unwrap();
        let a = s.upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(3), Some(4)).await.unwrap();
        let b = s.upsert_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(5), Some(6)).await.unwrap();
        let lonely = s.upsert_node(&fid, "function", "lonely", "a.rs", None, Some("()"), Some(7), Some(8)).await.unwrap();
        s.insert_edge(&fid, &a, Some(&hub), None, None, "calls").await.unwrap(); // a→hub
        s.insert_edge(&fid, &b, Some(&hub), None, None, "calls").await.unwrap(); // b→hub

        s.recompute_degrees_for_folder(&fid).await.unwrap();

        let deg = |id: uuid::Uuid| {
            let pool = s.pool().clone();
            async move {
                let (d,): (Option<i32>,) = query_as("SELECT degree FROM sensei.nodes WHERE id=$1")
                    .bind(id).fetch_one(&pool).await.unwrap();
                d
            }
        };
        assert_eq!(deg(hub).await, Some(2), "hub is the resolved target of 2 calls");
        assert_eq!(deg(a).await, Some(1), "a is the source of 1 call");
        assert_eq!(deg(b).await, Some(1), "b is the source of 1 call");
        assert_eq!(deg(lonely).await, Some(0), "an edgeless node has degree 0, not NULL");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn god_node_ids_are_top_by_degree() {
        // D4.5: a community's god_node_ids are its highest-degree members (top-5),
        // read from nodes.degree; the hub ranks first.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("godnode_{}", uuid::Uuid::new_v4())).await;
        let hub = s.upsert_node(&fid, "function", "hub", "a.rs", None, Some("()"), Some(1), Some(2)).await.unwrap();
        let a = s.upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(3), Some(4)).await.unwrap();
        let b = s.upsert_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(5), Some(6)).await.unwrap();
        let c = s.upsert_node(&fid, "function", "c", "a.rs", None, Some("()"), Some(7), Some(8)).await.unwrap();
        // a→hub, b→hub, c→hub, a→b (calls). Degrees: hub=3, a=2, b=2, c=1 → one
        // community {hub,a,b,c}; hub is the clear hub.
        s.insert_edge(&fid, &a, Some(&hub), None, None, "calls").await.unwrap();
        s.insert_edge(&fid, &b, Some(&hub), None, None, "calls").await.unwrap();
        s.insert_edge(&fid, &c, Some(&hub), None, None, "calls").await.unwrap();
        s.insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap();

        s.recompute_degrees_for_folder(&fid).await.unwrap();
        crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

        let (god,): (Vec<uuid::Uuid>,) = query_as(
            "SELECT god_node_ids FROM inference.communities WHERE folder_id=$1 ORDER BY community_id LIMIT 1")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(god.first(), Some(&hub), "the highest-degree node is the first god node");
        assert!(god.contains(&hub), "hub is a god node");
        assert!(god.len() <= 5, "at most 5 god nodes per community");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn community_description_authoritative_write_is_honest_null() {
        // D4.5 never-fabricate: the authoritative detection write leaves every
        // community's description NULL with props.source='null' — honest-empty,
        // NEVER a static template. (Model prose is stamped later, off-barrier, by
        // enrich_community_descriptions.) The Done-gate keys on
        // props.source ∈ {'insight-copy','null'}.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("commdesc_{}", uuid::Uuid::new_v4())).await;
        let a = s.upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2)).await.unwrap();
        let b = s.upsert_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(3), Some(4)).await.unwrap();
        s.insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap();

        crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

        let rows: Vec<(Option<String>, serde_json::Value)> = query_as(
            "SELECT description, props FROM inference.communities WHERE folder_id=$1")
            .bind(fid).fetch_all(s.pool()).await.unwrap();
        assert!(!rows.is_empty(), "at least one community was written");
        for (desc, props) in &rows {
            assert_eq!(*desc, None, "description is honest-NULL without a gateway");
            let source = props.get("source").and_then(|v| v.as_str());
            assert_eq!(source, Some("null"), "props.source records the honest-empty provenance");
            assert_ne!(source, Some("template"), "never a templated description (never-fabricate)");
        }

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn graph_nodes_returns_community_and_structural_edges() {
        // 7.1: get_nodes_scoped exposes community_id, and get_edges_scoped_kinds
        // returns the full layout set calls,imports,extends — NOT just calls, and
        // NOT overlay kinds like covers.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("gscope_{}", uuid::Uuid::new_v4())).await;
        let a = s.upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2)).await.unwrap();
        let b = s.upsert_node(&fid, "class", "B", "a.rs", None, Some("()"), Some(3), Some(4)).await.unwrap();
        sqlx_core::query::query("UPDATE sensei.nodes SET community_id=5 WHERE id=$1").bind(a).execute(s.pool()).await.unwrap();
        s.insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap();
        s.insert_edge(&fid, &a, None, Some("lib"), None, "imports").await.unwrap();
        s.insert_edge(&fid, &b, Some(&a), None, None, "extends").await.unwrap();
        s.insert_edge(&fid, &a, Some(&b), None, None, "covers").await.unwrap(); // overlay — excluded

        let nodes = s.get_nodes_scoped(&[fid]).await.unwrap();
        let a_node = nodes.iter().find(|n| n["name"] == "a").unwrap();
        assert_eq!(a_node["community_id"].as_i64(), Some(5), "get_nodes_scoped exposes community_id");

        let edges = s.get_edges_scoped_kinds(&[fid], &["calls", "imports", "extends"]).await.unwrap();
        let kinds: std::collections::HashSet<&str> = edges.iter().filter_map(|e| e["kind"].as_str()).collect();
        assert_eq!(edges.len(), 3, "exactly the 3 layout edges (covers excluded)");
        assert!(kinds.contains("calls") && kinds.contains("imports") && kinds.contains("extends"), "all 3 layout kinds present: {kinds:?}");
        assert!(!kinds.contains("covers"), "covers (overlay) is not a layout edge");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn communities_info_uses_live_membership() {
        // 7.3: list_communities_live_scoped counts from the real nodes.community_id
        // join, NOT the denormalized communities.node_count — so a stale count
        // doesn't drive the overview.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("livecomm_{}", uuid::Uuid::new_v4())).await;
        let n1 = s.upsert_node(&fid, "function", "n1", "a.rs", None, Some("()"), Some(1), Some(2)).await.unwrap();
        let n2 = s.upsert_node(&fid, "function", "n2", "a.rs", None, Some("()"), Some(3), Some(4)).await.unwrap();
        let n3 = s.upsert_node(&fid, "function", "n3", "a.rs", None, Some("()"), Some(5), Some(6)).await.unwrap();
        // Community 1 has 2 live members, community 2 has 1 — but seed a STALE count.
        s.upsert_community(&fid, 1, "c1", 99).await.unwrap(); // stale node_count = 99
        s.upsert_community(&fid, 2, "c2", 0).await.unwrap();  // stale node_count = 0
        sqlx_core::query::query("UPDATE sensei.nodes SET community_id=1 WHERE id = ANY($1)")
            .bind(vec![n1, n2]).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("UPDATE sensei.nodes SET community_id=2 WHERE id=$1")
            .bind(n3).execute(s.pool()).await.unwrap();

        let live = s.list_communities_live_scoped(&[fid]).await.unwrap();
        let count_of = |label: &str| live.iter()
            .find(|c| c["label"] == label)
            .and_then(|c| c["node_count"].as_i64());
        assert_eq!(count_of("c1"), Some(2), "c1 sized by 2 LIVE members, not the stale 99");
        assert_eq!(count_of("c2"), Some(1), "c2 sized by 1 LIVE member, not the stale 0");
        // Ordered by live count desc → c1 first.
        assert_eq!(live.first().and_then(|c| c["label"].as_str()), Some("c1"));

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn upsert_node_at_same_line_keeps_id_and_renulls_embedding_on_sig_change() {
        // D3: a re-upsert at the SAME identity (line_start is part of the key)
        // keeps the id — preserving community_id and degree. The embedding is
        // PRESERVED when the signature is unchanged, and RE-NULLED (re-embed) when
        // the signature changed — signature being the only embed input that can
        // change on a same-identity conflict. A DIFFERENT line is a new identity.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("nodeid_{}", uuid::Uuid::new_v4())).await;
        let id1 = s.upsert_node(&fid, "function", "foo", "a.rs", None, Some("fn foo(x: i32)"), Some(10), Some(20)).await.unwrap();

        // Simulate a prior enrich + embed pass on this node.
        let zeros = format!("[{}]", vec!["0"; 384].join(","));
        sqlx_core::query::query("UPDATE sensei.nodes SET community_id = 7, degree = 3, embedding = $2::vector WHERE id = $1")
            .bind(id1).bind(&zeros).execute(s.pool()).await.unwrap();

        // Re-upsert SAME line, SAME signature, only line_end grew → id kept, all preserved.
        let id2 = s.upsert_node(&fid, "function", "foo", "a.rs", None, Some("fn foo(x: i32)"), Some(10), Some(25)).await.unwrap();
        assert_eq!(id1, id2, "a re-upsert at the same identity keeps its id");
        let (community, degree, has_emb): (Option<i32>, Option<i32>, bool) = query_as(
            "SELECT community_id, degree, embedding IS NOT NULL FROM sensei.nodes WHERE id = $1")
            .bind(id1).fetch_one(s.pool()).await.unwrap();
        assert_eq!((community, degree, has_emb), (Some(7), Some(3), true),
            "community_id/degree/embedding preserved when signature is unchanged");

        // Re-upsert SAME line, CHANGED signature → id kept, community kept, embedding RE-NULLED.
        let id3 = s.upsert_node(&fid, "function", "foo", "a.rs", None, Some("fn foo(x: i64)"), Some(10), Some(25)).await.unwrap();
        assert_eq!(id1, id3, "same identity (line) keeps the id even when signature changes");
        let (community2, has_emb2): (Option<i32>, bool) = query_as(
            "SELECT community_id, embedding IS NOT NULL FROM sensei.nodes WHERE id = $1")
            .bind(id1).fetch_one(s.pool()).await.unwrap();
        assert_eq!(community2, Some(7), "community_id is still preserved");
        assert!(!has_emb2, "embedding is re-nulled for re-embedding when the signature changed");

        // A DIFFERENT line is a new identity ⇒ a new node (a moved symbol churns).
        let id4 = s.upsert_node(&fid, "function", "foo", "a.rs", None, Some("fn foo(x: i32)"), Some(99), Some(105)).await.unwrap();
        assert_ne!(id1, id4, "a different line_start is a new identity (moved symbol re-mints until D5c nesting)");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn upsert_node_by_fqn_merges_ref_and_def() {
        // FQN get-or-create (SCIP/LSIF moniker model): a REFERENCE creates an
        // unresolved stub (resolved=false, NULL file_path); a later DEFINITION
        // with the same (folder_id, fqn) returns the SAME id, flips resolved=true
        // and fills file/line/signature; a second reference shares the one node
        // and never downgrades the resolved definition. No "unresolved" state —
        // the node exists from its first mention.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("fqn_{}", uuid::Uuid::new_v4())).await;
        let fqn = "rust·senseid·widget·Widget·new";

        // 1. Reference-first → a stub.
        let stub = s.upsert_node_by_fqn(&fid, fqn, "method", "new", Some("rust"), None).await.unwrap();
        let (resolved, fp): (bool, Option<String>) =
            query_as("SELECT resolved, file_path FROM sensei.nodes WHERE id=$1")
            .bind(stub).fetch_one(s.pool()).await.unwrap();
        assert!(!resolved, "a reference-first node is an unresolved stub");
        assert_eq!(fp, None, "a stub has no known file");

        // 2. The definition enriches the SAME node in place.
        let def = s.upsert_node_by_fqn(&fid, fqn, "method", "new", Some("rust"),
            Some(FqnDef { file_path: "src/widget.rs", signature: Some("fn new() -> Self"),
                          line_start: Some(10), line_end: Some(12), is_exported: true, parent_id: None })
        ).await.unwrap();
        assert_eq!(stub, def, "the definition get-or-creates the SAME node as the reference");
        let (resolved2, fp2, sig, ls, exported): (bool, Option<String>, Option<String>, Option<i32>, bool) =
            query_as("SELECT resolved, file_path, signature, line_start, is_exported FROM sensei.nodes WHERE id=$1")
            .bind(def).fetch_one(s.pool()).await.unwrap();
        assert!(resolved2, "the node is resolved once its definition is seen");
        assert_eq!(fp2.as_deref(), Some("src/widget.rs"));
        assert_eq!(sig.as_deref(), Some("fn new() -> Self"));
        assert_eq!(ls, Some(10));
        assert!(exported, "the definition's is_exported is written");

        // 3. A second reference shares the one node and does NOT downgrade it.
        let ref2 = s.upsert_node_by_fqn(&fid, fqn, "method", "new", Some("rust"), None).await.unwrap();
        assert_eq!(ref2, def, "a later reference resolves to the same node");
        let (still_resolved, still_fp): (bool, Option<String>) =
            query_as("SELECT resolved, file_path FROM sensei.nodes WHERE id=$1")
            .bind(def).fetch_one(s.pool()).await.unwrap();
        assert!(still_resolved, "a reference must not downgrade an already-resolved node");
        assert_eq!(still_fp.as_deref(), Some("src/widget.rs"), "a reference must not clear the definition's file");

        // Exactly one node for this fqn.
        let (n,): (i64,) = query_as("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND fqn=$2")
            .bind(fid).bind(fqn).fetch_one(s.pool()).await.unwrap();
        assert_eq!(n, 1, "ref + def + ref = exactly one node");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn lib_node_by_fqn() {
        // An external reference get-or-creates a first-class `lib_symbol` node:
        // resolved=true (the external symbol IS its own definition — nothing to
        // enrich), NULL file_path (no local file), grouped by package in props.
        // Stable id across repeated references.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("lib_{}", uuid::Uuid::new_v4())).await;
        let fqn = "lib·serde_json·serde_json·from_str";

        let a = s.upsert_lib_node_by_fqn(&fid, fqn, "from_str", "serde_json").await.unwrap();
        let (kind, resolved, fp, pkg): (String, bool, Option<String>, Option<String>) = query_as(
            "SELECT kind::text, resolved, file_path, props->>'package' FROM sensei.nodes WHERE id=$1")
            .bind(a).fetch_one(s.pool()).await.unwrap();
        assert_eq!(kind, "lib_symbol");
        assert!(resolved, "a lib symbol is its own definition — resolved");
        assert_eq!(fp, None, "a lib symbol has no local file");
        assert_eq!(pkg.as_deref(), Some("serde_json"), "grouped by package in props");

        // A second reference to the same external fqn shares the one node.
        let b = s.upsert_lib_node_by_fqn(&fid, fqn, "from_str", "serde_json").await.unwrap();
        assert_eq!(a, b, "repeated external references share one lib node");
        let (n,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='lib_symbol'::sensei.node_kind")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(n, 1);

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn graph_nodes_and_tree_expose_fqn_and_containers() {
        // Phase 7.2: the graph/nodes projection (get_nodes_scoped) carries `fqn` +
        // `resolved` so the Atlas can key symbols by moniker, and /tree (build_tree)
        // nests the Phase-5 type/module containers (file → type → method) via
        // parent_id. These are the two projections the retrieval endpoints delegate to.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("retr_{}", uuid::Uuid::new_v4())).await;

        // file → struct container → method(fqn), nested by parent_id (Phase 5 shape).
        let file_id = s.upsert_node(&fid, "file", "lib.rs", "src/lib.rs", None, None, Some(1), Some(9)).await.unwrap();
        let type_id = s.upsert_node(&fid, "struct", "Widget", "src/lib.rs", Some(&file_id), Some("struct Widget"), Some(2), Some(6)).await.unwrap();
        let method_fqn = "rust·pkg·lib·Widget·render";
        let method_id = s.upsert_node_by_fqn(&fid, method_fqn, "method", "render", Some("rust"),
            Some(super::FqnDef { file_path: "src/lib.rs", signature: Some("fn render(&self)"),
                line_start: Some(3), line_end: Some(5), is_exported: true, parent_id: Some(&type_id) })).await.unwrap();

        // ── graph/nodes exposes fqn + resolved ──
        let nodes = s.get_nodes_scoped(&[fid]).await.unwrap();
        let method = nodes.iter().find(|n| n["name"] == "render").expect("method node in projection");
        assert_eq!(crate::api::util::json_uuid(&method["id"]), Some(method_id), "same method node");
        assert_eq!(method["fqn"].as_str(), Some(method_fqn), "get_nodes_scoped projects the node's fqn");
        assert_eq!(method["resolved"].as_bool(), Some(true), "and its resolved flag");
        assert_eq!(method["is_test"].as_bool(), Some(false), "and its is_test flag (default false here)");

        // ── /tree nests the type container → method (Phase 5 parent_id) ──
        let folders = s.get_folders_scoped(&[fid]).await.unwrap();
        let tree = crate::api::handlers::codebase::build_tree_pub(&folders, &nodes);
        let files = tree["tree"][0]["nodes"].as_array().expect("folder root nodes");
        let file = files.iter().find(|n| n["name"] == "lib.rs").expect("file node under folder");
        let widget = file["children"].as_array().unwrap().iter()
            .find(|n| n["name"] == "Widget").expect("type container nested under file");
        assert_eq!(widget["kind"], "struct", "the type container carries its kind");
        let render = widget["children"].as_array().unwrap().iter()
            .find(|n| n["name"] == "render").expect("method nested under the type container");
        assert_eq!(render["kind"], "method", "the method nests under the type container (Phase 5)");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn two_same_name_stubs_do_not_merge() {
        // Phase 3.0: with the partial identity index (`where file_path is not null`),
        // reference stubs (file_path NULL) are governed ONLY by nodes_unique_fqn.
        // Two references with the same simple name but DIFFERENT fqns must stay two
        // distinct nodes — the false-merge this rebuild exists to kill. (Under the
        // old non-partial identity constraint these would collide on
        // (folder, NULL, kind, name, NULL, NULL).)
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("stubmerge_{}", uuid::Uuid::new_v4())).await;
        let a = s.upsert_node_by_fqn(&fid, "rust·pkg·a·A·foo", "method", "foo", Some("rust"), None).await.unwrap();
        let b = s.upsert_node_by_fqn(&fid, "rust·pkg·b·B·foo", "method", "foo", Some("rust"), None).await.unwrap();
        assert_ne!(a, b, "same simple name, different fqn → two distinct stub nodes");
        let (n,): (i64,) = query_as("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND resolved=false")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(n, 2, "both stubs coexist under the fqn index");
        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn legacy_upsert_sets_language_from_extension() {
        // Every node written via the legacy upsert_node/_ex path (all non-Rust +
        // file/section/rationale nodes, for the whole FQN transition) must carry
        // `language` derived from its file extension — otherwise the same-language
        // bare-name fallback filter (plan 0.8) has nothing to match on. Compound
        // extensions resolve too (.svelte.ts → typescript).
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("lang_{}", uuid::Uuid::new_v4())).await;
        let cases = [
            ("src/a.rs",        "function", Some("fn f()"), "rust"),
            ("pkg/b.py",        "function", Some("def g()"), "python"),
            ("app/c.svelte.ts", "function", None,           "typescript"), // compound ext
            ("docs/e.md",       "doc",      None,           "markdown"),
        ];
        for (path, kind, sig, want) in cases {
            let id = s.upsert_node(&fid, kind, "n", path, None, sig, Some(1), Some(2)).await.unwrap();
            let (lang,): (Option<String>,) = query_as("SELECT language FROM sensei.nodes WHERE id=$1")
                .bind(id).fetch_one(s.pool()).await.unwrap();
            assert_eq!(lang.as_deref(), Some(want), "{path} → language {want}");
        }

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn node_locations_tolerates_stub_rows() {
        // file_path is now nullable (reference stubs + lib_symbol nodes have none).
        // node_locations decodes file_path as a required String, so a stub id among
        // the requested ids must NOT error the whole fetch — the stub (no location)
        // is simply omitted while the real node still resolves.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("nodeloc_{}", uuid::Uuid::new_v4())).await;
        let real = s.upsert_node(&fid, "function", "real", "a.rs", None, Some("fn real()"), Some(3), Some(9)).await.unwrap();
        let stub = s.upsert_node_by_fqn(&fid, "rust·pkg·m·Missing·gone", "method", "gone", Some("rust"), None).await.unwrap();

        let locs = s.node_locations(&[real, stub]).await.unwrap();
        assert_eq!(locs.len(), 1, "the stub (NULL file_path) is omitted, not an error");
        assert_eq!(locs[0].0, real, "the real node still resolves");
        assert_eq!(locs[0].2, "a.rs", "with its file_path");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn prune_file_nodes_deletes_vanished_and_unresolves_inbound() {
        // D3: a symbol that vanished from the parse is pruned; an inbound
        // cross-file edge to it is UNRESOLVED (target_id→NULL, target_name kept),
        // not cascade-deleted (invariant 3). A kept node is untouched.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("prune_{}", uuid::Uuid::new_v4())).await;
        let keep = s.upsert_node(&fid, "function", "keep", "a.rs", None, Some("()"), Some(1), Some(5)).await.unwrap();
        let gone = s.upsert_node(&fid, "function", "gone", "a.rs", None, Some("()"), Some(6), Some(9)).await.unwrap();
        let caller = s.upsert_node(&fid, "function", "caller", "b.rs", None, Some("()"), Some(1), Some(3)).await.unwrap();
        // A resolved inbound edge b.rs::caller → a.rs::gone, carrying target_name.
        let e = s.insert_edge(&fid, &caller, None, Some("gone"), None, "calls").await.unwrap();
        s.resolve_edge(&e, &gone).await.unwrap();

        // Re-index of a.rs keeps only `keep`.
        let pruned = s.prune_file_nodes(&fid, "a.rs", &[keep]).await.unwrap();
        assert_eq!(pruned, 1, "the vanished `gone` node is pruned");

        let (keep_exists,): (bool,) = query_as("SELECT EXISTS(SELECT 1 FROM sensei.nodes WHERE id=$1)")
            .bind(keep).fetch_one(s.pool()).await.unwrap();
        assert!(keep_exists, "the surviving node is untouched");
        let (gone_exists,): (bool,) = query_as("SELECT EXISTS(SELECT 1 FROM sensei.nodes WHERE id=$1)")
            .bind(gone).fetch_one(s.pool()).await.unwrap();
        assert!(!gone_exists, "the vanished node is deleted");
        // The inbound edge survived, unresolved (target_id NULL, target_name kept).
        let (tid, tname): (Option<uuid::Uuid>, Option<String>) = query_as(
            "SELECT target_id, target_name FROM sensei.edges WHERE id = $1")
            .bind(e).fetch_one(s.pool()).await.unwrap();
        assert_eq!(tid, None, "inbound edge to the pruned node is unresolved, not deleted");
        assert_eq!(tname.as_deref(), Some("gone"), "target_name is kept for re-resolution");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn delete_edges_from_sources_clears_a_files_out_edges() {
        // D3 per-file reconcile: a surviving symbol's stale out-edges are cleared
        // before re-inserting the current set (they don't cascade — the node
        // lives). Only edges FROM the given sources are removed.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("outedge_{}", uuid::Uuid::new_v4())).await;
        let a = s.upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(5)).await.unwrap();
        let b = s.upsert_node(&fid, "function", "b", "b.rs", None, Some("()"), Some(1), Some(5)).await.unwrap();
        s.insert_edge(&fid, &a, None, Some("x"), None, "calls").await.unwrap();       // a's out-edge
        s.insert_edge(&fid, &b, None, Some("y"), None, "calls").await.unwrap();       // b's out-edge (must survive)

        let n = s.delete_edges_from_sources(&fid, &[a]).await.unwrap();
        assert_eq!(n, 1, "only a's out-edge is deleted");
        assert_eq!(s.get_edges_by_kind(&fid, "calls").await.unwrap().len(), 1, "b's out-edge survives");
        // Empty sources is a cheap no-op.
        assert_eq!(s.delete_edges_from_sources(&fid, &[]).await.unwrap(), 0);

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn replace_edges_of_kind_swaps_the_full_set() {
        // D2: replace_edges_of_kind removes STALE edges of a kind and inserts the
        // current set atomically — the "replaced, not appended" guarantee that
        // makes a derived kind (covers) a pure function of the current tree.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("replkind_{}", uuid::Uuid::new_v4())).await;
        let doc = s.upsert_node(&fid, "doc", "d", "d.md", None, None, None, None).await.unwrap();
        let f1 = s.upsert_node(&fid, "file", "f1", "f1.rs", None, None, None, None).await.unwrap();
        let f2 = s.upsert_node(&fid, "file", "f2", "f2.rs", None, None, None, None).await.unwrap();

        // A STALE covers edge doc→f1 (as if f1 was the covered file last scan).
        s.insert_edge(&fid, &doc, Some(&f1), None, None, "covers").await.unwrap();
        assert_eq!(s.get_edges_by_kind(&fid, "covers").await.unwrap().len(), 1);

        // Replace the covers set with {doc→f2}: the stale doc→f1 must vanish.
        s.replace_edges_of_kind(&fid, "covers", &[
            EdgeSpec { source_id: doc, target_id: Some(f2), target_name: None, target_file: None },
        ]).await.unwrap();

        let (tid,): (Option<uuid::Uuid>,) = query_as(
            "SELECT target_id FROM sensei.edges WHERE folder_id=$1 AND kind='covers'::sensei.edge_kind")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(s.get_edges_by_kind(&fid, "covers").await.unwrap().len(), 1,
            "exactly the current set — stale edge removed, not appended");
        assert_eq!(tid, Some(f2), "the surviving covers edge is the new target");

        // Replacing with an EMPTY set clears the kind for the folder.
        s.replace_edges_of_kind(&fid, "covers", &[]).await.unwrap();
        assert!(s.get_edges_by_kind(&fid, "covers").await.unwrap().is_empty(),
            "an empty set clears every edge of the kind");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn replace_edges_of_kind_handles_unresolved_edges() {
        // The unresolved branch (target_id=None) — the path the per-file reconcile
        // (D3) will use. Replaces by (target_name, target_file).
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("replun_{}", uuid::Uuid::new_v4())).await;
        let a = s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();
        s.insert_edge(&fid, &a, None, Some("old"), None, "calls").await.unwrap(); // stale unresolved a→"old"

        s.replace_edges_of_kind(&fid, "calls", &[
            EdgeSpec { source_id: a, target_id: None, target_name: Some("new".into()), target_file: Some("x.rs".into()) },
        ]).await.unwrap();

        let (name, file): (Option<String>, Option<String>) = query_as(
            "SELECT target_name, target_file FROM sensei.edges WHERE folder_id=$1 AND kind='calls'::sensei.edge_kind")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
        assert_eq!((name.as_deref(), file.as_deref()), (Some("new"), Some("x.rs")),
            "unresolved edge replaced by (target_name, target_file)");
        assert_eq!(s.get_edges_by_kind(&fid, "calls").await.unwrap().len(), 1, "stale unresolved edge removed");
        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn replace_edges_of_kind_is_atomic_and_rolls_back_on_failure() {
        // The "one transaction" guarantee: if an insert in the batch fails (a bad
        // source_id → FK violation), the whole replace rolls back — the OLD set is
        // intact, never half-deleted (no zero-covers window).
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("replatomic_{}", uuid::Uuid::new_v4())).await;
        let doc = s.upsert_node(&fid, "doc", "d", "d.md", None, None, None, None).await.unwrap();
        let f1 = s.upsert_node(&fid, "file", "f1", "f1.rs", None, None, None, None).await.unwrap();
        s.insert_edge(&fid, &doc, Some(&f1), None, None, "covers").await.unwrap();

        // A batch whose second edge has a bogus source_id (no such node) → the
        // FK on edges.source_id fails the insert mid-batch.
        let bogus = uuid::Uuid::new_v4();
        let res = s.replace_edges_of_kind(&fid, "covers", &[
            EdgeSpec { source_id: doc, target_id: Some(f1), target_name: None, target_file: None },
            EdgeSpec { source_id: bogus, target_id: Some(f1), target_name: None, target_file: None },
        ]).await;
        assert!(res.is_err(), "a bad edge fails the replace");

        assert_eq!(s.get_edges_by_kind(&fid, "covers").await.unwrap().len(), 1,
            "the DELETE rolled back with the failed insert — old set intact, not half-deleted");
        let (tid,): (Option<uuid::Uuid>,) = query_as(
            "SELECT target_id FROM sensei.edges WHERE folder_id=$1 AND kind='covers'::sensei.edge_kind")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(tid, Some(f1), "the surviving edge is the original (rollback)");
        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn insert_edge_unresolved_dedups_by_target_file() {
        // D1: the unresolved identity is (folder, source, target_name, target_file,
        // kind). Same target_name in DIFFERENT files are distinct edges; same
        // name + same file (incl. nulls-not-distinct) upserts to one row. This is
        // the whole point of the target_file column — a same-named symbol in two
        // files must not collapse to one edge.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("edgetf_{}", uuid::Uuid::new_v4())).await;
        let a = s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();

        let e1 = s.insert_edge(&fid, &a, None, Some("helper"), Some("x.rs"), "calls").await.unwrap();
        let e2 = s.insert_edge(&fid, &a, None, Some("helper"), Some("y.rs"), "calls").await.unwrap();
        assert_ne!(e1, e2, "same target_name in different files are distinct unresolved edges");
        assert_eq!(s.get_edges_by_kind(&fid, "calls").await.unwrap().len(), 2, "two distinct rows");

        let e3 = s.insert_edge(&fid, &a, None, Some("helper"), Some("x.rs"), "calls").await.unwrap();
        assert_eq!(e1, e3, "same (target_name, target_file) upserts to the same row");
        assert_eq!(s.get_edges_by_kind(&fid, "calls").await.unwrap().len(), 2, "no new row on re-insert");

        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn resolve_edge_second_call_is_safe() {
        // resolve_edge is idempotent: resolving the same edge to the same target
        // twice must be a safe no-op (one edge), not a unique-violation throw.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("resolve2x_{}", uuid::Uuid::new_v4())).await;
        let a = s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();
        let b = s.upsert_node(&fid, "function", "b", "b.rs", None, None, Some(1), Some(5)).await.unwrap();
        let u = s.insert_edge(&fid, &a, None, Some("b"), None, "calls").await.unwrap();

        s.resolve_edge(&u, &b).await.unwrap();
        s.resolve_edge(&u, &b).await.unwrap(); // second call — must not throw

        assert_eq!(s.get_edges_by_kind(&fid, "calls").await.unwrap().len(), 1,
            "resolving twice keeps exactly one edge");
        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn resolve_edge_updates_in_place_when_no_conflict() {
        // The common case: no existing resolved dup → the unresolved edge is
        // updated in place to the resolved target (not deleted).
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("resolveok_{}", uuid::Uuid::new_v4())).await;
        let a = s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();
        let b = s.upsert_node(&fid, "function", "b", "b.rs", None, None, Some(1), Some(5)).await.unwrap();
        let u = s.insert_edge(&fid, &a, None, Some("b"), None, "calls").await.unwrap();

        s.resolve_edge(&u, &b).await.unwrap();

        let (tid,): (Option<uuid::Uuid>,) = query_as("SELECT target_id FROM sensei.edges WHERE id=$1")
            .bind(u).fetch_one(s.pool()).await.unwrap();
        assert_eq!(tid, Some(b), "the edge is resolved in place to the target");
        s.delete_nodes_by_folder(&fid).await.unwrap();
    }

    // ── Extensions tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn extension_create_and_list() {
        let s = pg_store().await;
        let name = format!("_test:ext_{}", uuid::Uuid::new_v4());
        let id = s.create_extension("skill", &name, Some("test skill"), Some("# content"), "global", "local").await.unwrap();
        let skills = s.list_extensions_by_kind("skill").await.unwrap();
        assert!(skills.iter().any(|e| e["name"] == name));
        s.delete_extension(&id).await.unwrap();
    }

    #[tokio::test]
    async fn extension_historize_trigger() {
        let s = pg_store().await;
        let name = format!("_test:ext_hist_{}", uuid::Uuid::new_v4());
        let id = s.create_extension("skill", &name, Some("v1"), None, "global", "local").await.unwrap();
        s.update_extension(&id, Some("v2"), None).await.unwrap();
        let history = s.get_extension_history(&id).await.unwrap();
        assert!(history.len() >= 2, "historize trigger should create INSERT + UPDATE entries");
        s.delete_extension(&id).await.unwrap();
    }

    // ── Folders tests ────────────────────────────────────────────────

    #[tokio::test]
    async fn folder_upsert_and_list() {
        let s = pg_store().await;
        let path = format!("/_test/folder_root_{}", uuid::Uuid::new_v4());
        let rid = s.add_watch_root(&path, "test_root", &serde_json::json!([])).await.unwrap();
        let fid = s.upsert_folder(&rid, "git", "myrepo", "myrepo", &format!("{}/myrepo", path), None, None).await.unwrap();
        let folders = s.list_folders_by_root(&rid).await.unwrap();
        assert!(folders.iter().any(|f| f["name"] == "myrepo"));
        s.delete_folder_tree(&fid).await.unwrap();
        s.remove_watch_root(&rid).await.unwrap();
    }

    #[tokio::test]
    async fn list_pending_folders_returns_only_non_terminal_status() {
        let s = pg_store().await;
        let root_path = format!("/_test/pending_resume_{}", uuid::Uuid::new_v4().simple());
        let rid = s.add_watch_root(&root_path, "pending_root", &serde_json::json!([])).await.unwrap();

        // Seed one folder per status. Default is 'discovered'; the rest are
        // forced with an explicit UPDATE because upsert_folder has no status
        // parameter and `mark_folder_indexed` is the only writer of `indexed`.
        for (status, suffix) in [
            ("discovered", "a"),
            ("queued",     "b"),
            ("indexing",   "c"),
            ("indexed",    "d"),
            ("failed",     "e"),
            ("deferred",   "f"),
            ("archived",   "g"),
        ] {
            let name = format!("repo_{}", suffix);
            let abs_path = format!("{}/{}", root_path, name);
            let fid = s.upsert_folder(&rid, "git", &name, &name, &abs_path, None, None).await.unwrap();
            s.update_folder_status(&fid, status).await.unwrap();
        }

        let rows = s.list_pending_folders().await.unwrap();
        let ours: Vec<_> = rows.iter()
            .filter(|r| r["abs_path"].as_str().unwrap_or("").starts_with(&root_path))
            .collect();

        // Recoverable = non-terminal. `discovered`/`queued` never started;
        // `indexing`/`failed` are a scan interrupted mid-flight or errored —
        // its in-memory task was lost on restart (D6a marks `indexing` at scan
        // start), so resume MUST re-enqueue them. `indexed`/`deferred`/`archived`
        // are terminal and never resumed.
        let statuses: std::collections::BTreeSet<&str> = ours.iter()
            .map(|r| r["status"].as_str().unwrap())
            .collect();
        assert_eq!(
            statuses,
            std::collections::BTreeSet::from(["discovered", "queued", "indexing", "failed"]),
            "expected discovered+queued+indexing+failed, got {:?}", statuses
        );

        // Resume needs enough info to enqueue ProcessGitFolder: id, kind, abs_path.
        for r in &ours {
            assert!(r["id"].is_string(),       "row missing id: {:?}", r);
            assert!(r["kind"].is_string(),     "row missing kind: {:?}", r);
            assert!(r["abs_path"].is_string(), "row missing abs_path: {:?}", r);
        }

        // cleanup — removing the watch root cascades to folders.
        s.remove_watch_root(&rid).await.unwrap();
    }

    #[tokio::test]
    async fn update_folder_status_round_trips() {
        // D6a: the folder-status lifecycle needs a production setter — before
        // this, only `indexed` (mark_folder_indexed) and `archived` were
        // writable, so a scan could never record that it had started.
        let s = pg_store().await;
        let root_path = format!("/_test/status_{}", uuid::Uuid::new_v4().simple());
        let rid = s.add_watch_root(&root_path, "status_root", &serde_json::json!([])).await.unwrap();
        let fid = s.upsert_folder(&rid, "git", "r", "r", &format!("{root_path}/r"), None, None).await.unwrap();

        s.update_folder_status(&fid, "indexing").await.unwrap();

        let (status,): (String,) = sqlx_core::query_as::query_as(
            "SELECT status::text FROM sensei.folders WHERE id = $1"
        ).bind(fid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(status, "indexing", "update_folder_status writes the enum value");

        s.remove_watch_root(&rid).await.unwrap();
    }

    #[tokio::test]
    async fn get_folder_status_reads_back_status_and_is_none_for_missing() {
        // D6d: the fail-closed barrier reads folder status to decide whether to
        // mark `indexed`. A missing folder must be honest-`None`, never an error
        // or a fabricated status.
        let s = pg_store().await;
        let root_path = format!("/_test/getstatus_{}", uuid::Uuid::new_v4().simple());
        let rid = s.add_watch_root(&root_path, "getstatus_root", &serde_json::json!([])).await.unwrap();
        let fid = s.upsert_folder(&rid, "git", "r", "r", &format!("{root_path}/r"), None, None).await.unwrap();

        s.update_folder_status(&fid, "failed").await.unwrap();
        assert_eq!(s.get_folder_status(&fid).await.unwrap().as_deref(), Some("failed"),
            "reads back the written status");
        assert_eq!(s.get_folder_status(&uuid::Uuid::new_v4()).await.unwrap(), None,
            "a missing folder is None, not an error");

        s.remove_watch_root(&rid).await.unwrap();
    }

    // ── Benchmark Reports tests ──────────────────────────────────────

    #[tokio::test]
    async fn benchmark_create_and_list() {
        let s = pg_store().await;
        let id = s.create_benchmark_report(None, "_test:bench", "strategy_a", Some(95.5), Some(1000), Some(5000)).await.unwrap();
        let reports = s.list_benchmark_reports().await.unwrap();
        assert!(reports.iter().any(|r| r["run_name"] == "_test:bench"));
        sqlx_core::query::query("DELETE FROM sensei.benchmark_reports WHERE id = $1").bind(id).execute(s.pool()).await.unwrap();
    }

    // ── Views tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn repositories_view() {
        let s = pg_store().await;
        // list_repositories returns git+subtree folders
        let repos = s.list_repositories().await.unwrap();
        // Just verify it doesn't error — content depends on seeded data
        // Just verify the query succeeds — content depends on seeded data
        let _ = repos;
    }

    // ── Memories tests ─────────────────────────────────────────────────

    #[tokio::test]
    async fn memory_create_and_get() {
        let s = pg_store().await;
        let id = s.create_memory(None, "global", None, "decision", "_test:mem_create", "Always use TDD", Some("Bugs ship to prod"), None, None, None).await.unwrap();
        let m = s.get_memory(&id).await.unwrap().unwrap();
        assert_eq!(m["title"], "_test:mem_create");
        assert_eq!(m["scope"], "global");
        assert_eq!(m["strength"], 1.0);
        assert_eq!(m["status"], "active");
        // cleanup via historize trigger test
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(id).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn create_memory_persists_spine_slot_and_feature() {
        let s = pg_store().await;
        let pid = s.create_project(&format!("_test:mem_slot-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let id = s.create_memory(Some(&pid), "project", None, "decision", "t", "c", None, None,
            Some("decisions"), Some("auth")).await.unwrap();
        let row: (Option<String>, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT spine_slot::text, feature FROM sensei.memories WHERE id = $1"
        ).bind(id).fetch_one(s.pool()).await.unwrap();
        assert_eq!(row, (Some("decisions".into()), Some("auth".into())));
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(id).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn memory_reinforce() {
        let s = pg_store().await;
        let id = s.create_memory(None, "global", None, "pattern", "_test:mem_reinforce", "rule", None, None, None, None).await.unwrap();
        s.reinforce_memory(&id, 1.0).await.unwrap();
        s.reinforce_memory(&id, 1.0).await.unwrap();
        let m = s.get_memory(&id).await.unwrap().unwrap();
        assert_eq!(m["strength"], 3.0); // 1.0 + 1.0 + 1.0
        // Cap at 5.0
        s.reinforce_memory(&id, 10.0).await.unwrap();
        let m = s.get_memory(&id).await.unwrap().unwrap();
        assert_eq!(m["strength"], 5.0);
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(id).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn memory_archive() {
        let s = pg_store().await;
        let id = s.create_memory(None, "global", None, "question", "_test:mem_archive", "open q", None, None, None, None).await.unwrap();
        s.archive_memory(&id).await.unwrap();
        let m = s.get_memory(&id).await.unwrap().unwrap();
        assert_eq!(m["status"], "archived");
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(id).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn memory_list_active() {
        let s = pg_store().await;
        let id1 = s.create_memory(None, "global", None, "decision", "_test:mem_list_a", "rule a", None, None, None, None).await.unwrap();
        let id2 = s.create_memory(None, "global", None, "decision", "_test:mem_list_b", "rule b", None, None, None, None).await.unwrap();
        let active = s.list_active_memories(None, Some("global")).await.unwrap();
        assert!(active.iter().any(|m| m["title"] == "_test:mem_list_a"));
        assert!(active.iter().any(|m| m["title"] == "_test:mem_list_b"));
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)").bind(&[id1, id2][..]).execute(s.pool()).await.unwrap();
    }

    // ── Memory Examples tests ────────────────────────────────────────

    #[tokio::test]
    async fn memory_example_add_and_list() {
        let s = pg_store().await;
        let mid = s.create_memory(None, "global", None, "pattern", "_test:mem_ex", "rule", None, None, None, None).await.unwrap();
        s.add_memory_example(&mid, "fn:auth_handler", true, Some("canonical auth")).await.unwrap();
        s.add_memory_example(&mid, "fn:inline_auth", false, Some("avoid inline")).await.unwrap();
        let examples = s.list_memory_examples(&mid).await.unwrap();
        assert_eq!(examples.len(), 2);
        assert!(examples.iter().any(|e| e["is_good"] == true));
        assert!(examples.iter().any(|e| e["is_good"] == false));
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(mid).execute(s.pool()).await.unwrap();
    }

    // ── Memory Evidence tests ────────────────────────────────────────

    #[tokio::test]
    async fn memory_evidence_add_and_list() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("mem_ev_{}", uuid::Uuid::new_v4())).await;
        let sid = s.create_session(&fid, "test", None).await.unwrap();
        let mid = s.create_memory(None, "global", None, "decision", "_test:mem_ev", "rule", None, None, None, None).await.unwrap();
        s.add_memory_evidence(&mid, Some(&sid), Some("user corrected twice")).await.unwrap();
        // A save-time source note carries no session_id (nullable).
        s.add_memory_evidence(&mid, None, Some("crates/x.rs:42")).await.unwrap();
        let evidence = s.list_memory_evidence(&mid).await.unwrap();
        assert_eq!(evidence.len(), 2);
        assert!(evidence.iter().any(|e| e["session_id"].is_null() && e["note"] == "crates/x.rs:42"),
            "the session-less source note round-trips with a null session_id");
        assert_eq!(evidence[0]["note"], "user corrected twice");
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(mid).execute(s.pool()).await.unwrap();
    }

    // ── Memory Links tests ───────────────────────────────────────────

    #[tokio::test]
    async fn memory_links_parent_child() {
        let s = pg_store().await;
        let parent = s.create_memory(None, "global", None, "decision", "_test:mem_parent", "combined", None, None, None, None).await.unwrap();
        let child1 = s.create_memory(None, "global", None, "decision", "_test:mem_child1", "original 1", None, None, None, None).await.unwrap();
        let child2 = s.create_memory(None, "global", None, "decision", "_test:mem_child2", "original 2", None, None, None, None).await.unwrap();
        s.link_memories(&parent, &child1).await.unwrap();
        s.link_memories(&parent, &child2).await.unwrap();
        let children = s.get_memory_children(&parent).await.unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(s.get_memory_parent(&child1).await.unwrap(), Some(parent));
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
            .bind(&[parent, child1, child2][..]).execute(s.pool()).await.unwrap();
    }

    // ── Recommendations tests ────────────────────────────────────────

    #[tokio::test]
    async fn recommendation_lifecycle() {
        let s = pg_store().await;
        let pid = s.create_project("_test:rec_proj", None, None).await.unwrap();
        let rid = s.create_recommendation(&pid, "_test:rec", "reduces corrections", "promote_pattern", "high").await.unwrap();
        s.accept_recommendation(&rid).await.unwrap();
        s.measure_recommendation(&rid, "positive").await.unwrap();
        let recs = s.list_recommendations(&pid).await.unwrap();
        let r = recs.iter().find(|r| r["title"] == "_test:rec").unwrap();
        assert_eq!(r["status"], "accepted");
        assert_eq!(r["verdict"], "positive");
        sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id = $1").bind(rid).execute(s.pool()).await.unwrap();
        s.delete_project(&pid).await.unwrap();
    }

    // Gap 1 fix — the reject terminal is `dismissed` in the enum (not
    // `rejected`), and both accept + reject must only fire from `pending`.
    // Locks the enum-value contract so a future rename can't silently
    // break the UI action buttons.
    #[tokio::test]
    async fn recommendation_reject_writes_dismissed_and_guards_at_pending() {
        let s = pg_store().await;
        let pid = s.create_project("_test:rec_reject_proj", None, None).await.unwrap();
        let rid = s.create_recommendation(&pid, "_test:rej", "why", "revise_rule", "low").await.unwrap();

        s.reject_recommendation(&rid).await.unwrap();
        let recs = s.list_recommendations(&pid).await.unwrap();
        let r = recs.iter().find(|r| r["title"] == "_test:rej").unwrap();
        assert_eq!(r["status"], "dismissed", "reject writes the `dismissed` enum terminal");

        // Second reject on the same rec is a no-op guarded at `pending`;
        // pg_store must return an error rather than clobber the decision.
        let err = s.reject_recommendation(&rid).await.expect_err("guard fires on already-decided");
        assert!(err.contains("already decided") || err.contains("not found"),
                "guard error text: {err}");

        sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id = $1").bind(rid).execute(s.pool()).await.unwrap();
        s.delete_project(&pid).await.unwrap();
    }

    // ── Accept-driven pattern promotion ──────────────────────────────
    // Accepting a `promote_pattern` rec advances its source pattern's
    // lifecycle to `rule` (the read path renders it `adopted`). The action
    // is store-owned so it stays single-call-site + unit-testable.

    /// Seed a (project, folder, pattern, promote_pattern rec) fixture. The rec's
    /// `based_on.patterns[0]` cites the pattern (unless `cite_pattern` is false,
    /// exercising the defensive no-op path). Returns (proj, folder, pattern, rec).
    async fn seed_promote_fixture(
        s: &PgStore, suffix: &str, action_type: &str, cite_pattern: bool,
    ) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid, uuid::Uuid) {
        let (proj_id, fid) = create_test_project_and_folder(s, suffix).await;
        let pat_id = s
            .upsert_pattern(&proj_id, Some(&fid), "_test:rule-candidates", false, None, &serde_json::json!([]))
            .await
            .unwrap();
        // suggested is the seeded lifecycle default; assert the precondition so a
        // schema default change can't make the promotion test vacuously pass.
        let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
        let seeded = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
        assert_eq!(seeded["lifecycle"], "suggested", "pattern starts at suggested");

        let based_on = if cite_pattern {
            serde_json::json!({ "patterns": [pat_id] })
        } else {
            serde_json::json!({ "patterns": [] })
        };
        let rid = s
            .create_recommendation_full(&proj_id, "_test:promote", "why", None, action_type, "medium", &based_on, None, None)
            .await
            .unwrap();
        (proj_id, fid, pat_id, rid)
    }

    async fn cleanup_promote_fixture(s: &PgStore, proj_id: &uuid::Uuid, pat_id: &uuid::Uuid, rid: &uuid::Uuid) {
        sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id = $1").bind(rid).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1").bind(pat_id).execute(s.pool()).await.unwrap();
        s.delete_project(proj_id).await.unwrap();
    }

    /// Pure extractor: only a well-formed `patterns[0]` uuid comes back; a
    /// missing key, empty array, or non-uuid is `None` (the no-op signal).
    #[test]
    fn based_on_first_pattern_parses_uuid_and_defends() {
        let id = uuid::Uuid::new_v4();
        let good = serde_json::json!({ "patterns": [id] }).to_string();
        assert_eq!(PgStore::based_on_first_pattern(&good), Some(id));
        assert_eq!(PgStore::based_on_first_pattern("{}"), None);
        assert_eq!(PgStore::based_on_first_pattern(r#"{"patterns":[]}"#), None);
        assert_eq!(PgStore::based_on_first_pattern(r#"{"patterns":["not-a-uuid"]}"#), None);
        assert_eq!(PgStore::based_on_first_pattern("not json"), None);
    }

    /// (1) Accepting a promote_pattern rec advances the cited pattern to `rule`.
    #[tokio::test]
    async fn accept_promote_pattern_advances_lifecycle_to_rule() {
        let s = pg_store().await;
        let suffix = format!("accept_promote_{}", uuid::Uuid::new_v4());
        let (proj_id, fid, pat_id, rid) = seed_promote_fixture(&s, &suffix, "promote_pattern", true).await;

        s.accept_recommendation(&rid).await.unwrap();

        let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
        let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
        assert_eq!(p["lifecycle"], "rule", "accepting a promote_pattern rec advances the pattern to rule");

        cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
    }

    /// (2) A non-promote action (e.g. write_skill) leaves the pattern untouched.
    #[tokio::test]
    async fn accept_non_promote_leaves_pattern_untouched() {
        let s = pg_store().await;
        let suffix = format!("accept_writeskill_{}", uuid::Uuid::new_v4());
        let (proj_id, fid, pat_id, rid) = seed_promote_fixture(&s, &suffix, "write_skill", true).await;

        s.accept_recommendation(&rid).await.unwrap();

        let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
        let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
        assert_eq!(p["lifecycle"], "suggested", "a non-promote action must not advance the pattern");

        cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
    }

    /// (3) A promote_pattern rec with no cited pattern accepts as a no-op —
    /// returns Ok, the pattern is unchanged, and nothing panics.
    #[tokio::test]
    async fn accept_promote_pattern_without_provenance_is_noop() {
        let s = pg_store().await;
        let suffix = format!("accept_noprov_{}", uuid::Uuid::new_v4());
        let (proj_id, fid, pat_id, rid) = seed_promote_fixture(&s, &suffix, "promote_pattern", false).await;

        s.accept_recommendation(&rid).await.expect("empty provenance accepts without error");

        let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
        let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
        assert_eq!(p["lifecycle"], "suggested", "no cited pattern → nothing to promote");

        cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
    }

    /// (4) The pending-guard holds: a second accept errors, and the promotion
    /// fired exactly once (the pattern is still `rule`, not re-touched into error).
    #[tokio::test]
    async fn accept_promote_pattern_guard_fires_promotion_once() {
        let s = pg_store().await;
        let suffix = format!("accept_guard_{}", uuid::Uuid::new_v4());
        let (proj_id, fid, pat_id, rid) = seed_promote_fixture(&s, &suffix, "promote_pattern", true).await;

        s.accept_recommendation(&rid).await.unwrap();
        let err = s.accept_recommendation(&rid).await.expect_err("second accept is guarded at pending");
        assert!(err.contains("already decided") || err.contains("not found"), "guard error text: {err}");

        let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
        let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
        assert_eq!(p["lifecycle"], "rule", "promotion fired once; the guarded re-accept is inert");

        cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
    }

    /// (5) Read-path: after accept, get_project_patterns surfaces the pattern
    /// with kind='adopted' (pattern_kind maps lifecycle='rule' → adopted).
    #[tokio::test]
    async fn accept_promote_pattern_reads_back_as_adopted() {
        let s = pg_store().await;
        let suffix = format!("accept_adopted_{}", uuid::Uuid::new_v4());
        let (proj_id, _fid, pat_id, rid) = seed_promote_fixture(&s, &suffix, "promote_pattern", true).await;

        s.accept_recommendation(&rid).await.unwrap();

        let view = s.get_project_patterns(&proj_id).await.unwrap();
        let followed = view["followed"].as_array().expect("followed array");
        let p = followed.iter().find(|p| p["id"] == pat_id.to_string()).expect("promoted pattern in followed set");
        assert_eq!(p["kind"], "adopted", "lifecycle='rule' reads back as adopted");
        assert_eq!(p["lifecycle"], "rule");

        cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
    }

    // ── Verdict regression → challenge the source memory ────────────────
    // When an accepted rec's FTR REGRESSES after acceptance, the memory that
    // spawned it (via based_on.patterns[0] → memories.source_id) is challenged
    // (weakened) through the existing memory_outcome pipeline.

    /// Seed a (project, folder, pattern, learned memory sourced by the pattern)
    /// fixture. The memory starts at 1.0 + `strength_bump` so a single violation
    /// (−0.7) challenges rather than archives it. Returns (proj, folder, pat, mem).
    async fn seed_pattern_and_sourced_memory(
        s: &PgStore, suffix: &str, strength_bump: f64,
    ) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid, uuid::Uuid) {
        let (proj_id, fid) = create_test_project_and_folder(s, suffix).await;
        let pat_id = s
            .upsert_pattern(&proj_id, Some(&fid), "_test:regressed-rule", false, None, &serde_json::json!([]))
            .await
            .unwrap();
        // Convention memory sourced by the pattern — mirrors the rule-candidates generator.
        let mem = InsertMemory {
            project_id: Some(proj_id),
            scope: "project".to_string(),
            scope_filter: None,
            mtype: "convention".to_string(),
            title: format!("_test:regressed-memory:{suffix}"),
            content: "always foo".to_string(),
            impact: None,
            tags: Vec::new(),
            triage_signal: None,
            status: "active".to_string(),
            namespace_id: None,
            enforcement: None,
            origin: Some("learned".to_string()),
            source_id: Some(pat_id),
            spine_slot: None, feature: None,
        };
        let mem_id = s.insert_memory(&mem).await.unwrap();
        if strength_bump > 0.0 {
            s.reinforce_memory(&mem_id, strength_bump).await.unwrap();
        }
        (proj_id, fid, pat_id, mem_id)
    }

    /// Extend the memory fixture with an accepted+regressed promote_pattern rec:
    /// acted 4 days ago at baseline FTR 0.9, then ≥3 post-acceptance sessions that
    /// all fail (ftr=false) so the measured current FTR is 0.0 → a negative verdict.
    async fn seed_regressed_rec_with_memory(
        s: &PgStore, suffix: &str,
    ) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid, uuid::Uuid) {
        let (proj_id, fid, pat_id, mem_id) = seed_pattern_and_sourced_memory(s, suffix, 2.0).await;
        let based_on = serde_json::json!({ "patterns": [pat_id] });
        let rec_id = s
            .create_recommendation_full(&proj_id, "_test:regressed-rec", "why", None, "promote_pattern", "medium", &based_on, None, None)
            .await
            .unwrap();
        sqlx_core::query::query(
            "UPDATE inference.recommendations
                SET status = 'accepted'::sensei.recommendation_status,
                    acted_at = now() - interval '4 days',
                    baseline_ftr = 0.900
              WHERE id = $1"
        ).bind(rec_id).execute(s.pool()).await.unwrap();
        for _ in 0..3 {
            sqlx_core::query::query(
                "INSERT INTO activity.sessions (folder_id, project_id, outcome, ftr, started_at)
                 VALUES ($1, $2, 'corrected'::sensei.session_outcome, false, now())"
            ).bind(fid).bind(proj_id).execute(s.pool()).await.unwrap();
        }
        (proj_id, pat_id, mem_id, rec_id)
    }

    async fn cleanup_regressed_fixture(s: &PgStore, proj_id: &uuid::Uuid) {
        // Sessions FK to the folder (persisted), not the project, so drop them
        // explicitly; delete_project cascades recs, memories(+outcomes), patterns.
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE project_id = $1")
            .bind(proj_id).execute(s.pool()).await.ok();
        s.delete_project(proj_id).await.ok();
    }

    async fn violated_count(s: &PgStore, mem_id: &uuid::Uuid) -> i64 {
        let row: (i64,) = query_as(
            "SELECT count(*) FROM sensei.memory_outcomes WHERE memory_id = $1 AND outcome = 'violated'"
        ).bind(mem_id).fetch_one(s.pool()).await.unwrap();
        row.0
    }

    /// Full round-trip: measuring a regressed rec flips its verdict to negative
    /// AND challenges the source memory exactly once — re-measuring is inert
    /// (the rec is no longer pending, so the transition never re-fires).
    #[tokio::test]
    async fn measure_regressed_rec_challenges_source_memory_once() {
        let s = pg_store().await;
        let suffix = format!("regress_challenge_{}", uuid::Uuid::new_v4());
        let (proj_id, _pat_id, mem_id, _rec_id) = seed_regressed_rec_with_memory(&s, &suffix).await;

        let m0 = s.get_memory(&mem_id).await.unwrap().unwrap();
        assert!((m0["strength"].as_f64().unwrap() - 3.0).abs() < 1e-6, "seed strength 3.0");
        assert_eq!(m0["status"], "active", "memory starts active");

        s.measure_pending_verdicts().await.unwrap();

        let recs = s.list_recommendations(&proj_id).await.unwrap();
        let r = recs.iter().find(|r| r["title"] == "_test:regressed-rec").unwrap();
        assert_eq!(r["verdict"], "negative", "FTR dropped 0.9→0.0 → negative verdict");

        assert_eq!(violated_count(&s, &mem_id).await, 1, "one violation recorded for the source memory");
        let m1 = s.get_memory(&mem_id).await.unwrap().unwrap();
        assert!((m1["strength"].as_f64().unwrap() - 2.3).abs() < 1e-6, "trigger dropped strength 3.0→2.3");
        assert_eq!(m1["status"], "challenged", "trigger moved memory to challenged");

        // Re-measure: the rec is no longer pending → not re-measured → no second hit.
        s.measure_pending_verdicts().await.unwrap();
        assert_eq!(violated_count(&s, &mem_id).await, 1, "idempotent: no second violation on re-run");
        let m2 = s.get_memory(&mem_id).await.unwrap().unwrap();
        assert!((m2["strength"].as_f64().unwrap() - 2.3).abs() < 1e-6, "strength did not drop again");

        cleanup_regressed_fixture(&s, &proj_id).await;
    }

    /// Method-level idempotency: the `rec:<id>` context marker gates the write, so
    /// challenging the same memory for the same rec twice records only one violation.
    #[tokio::test]
    async fn challenge_source_memory_for_rec_is_idempotent_per_rec() {
        let s = pg_store().await;
        let suffix = format!("challenge_idem_{}", uuid::Uuid::new_v4());
        let (proj_id, _fid, pat_id, mem_id) = seed_pattern_and_sourced_memory(&s, &suffix, 2.0).await;
        let based_on = serde_json::json!({ "patterns": [pat_id] }).to_string();
        let rec_id = uuid::Uuid::new_v4();

        assert!(s.challenge_source_memory_for_rec(&rec_id, &based_on).await.unwrap(), "first challenge records a violation");
        assert_eq!(violated_count(&s, &mem_id).await, 1);
        let m1 = s.get_memory(&mem_id).await.unwrap().unwrap();
        assert!((m1["strength"].as_f64().unwrap() - 2.3).abs() < 1e-6, "strength 3.0→2.3");
        assert_eq!(m1["status"], "challenged");

        // Same rec again → no-op, no second violation, strength unchanged.
        assert!(!s.challenge_source_memory_for_rec(&rec_id, &based_on).await.unwrap(), "second challenge for same rec is a no-op");
        assert_eq!(violated_count(&s, &mem_id).await, 1, "still exactly one violation");
        let m2 = s.get_memory(&mem_id).await.unwrap().unwrap();
        assert!((m2["strength"].as_f64().unwrap() - 2.3).abs() < 1e-6, "strength did not drop again");

        cleanup_regressed_fixture(&s, &proj_id).await;
    }

    #[tokio::test]
    async fn reinforce_source_memory_for_rec_bumps_promotes_and_is_idempotent() {
        // The G1→G2 bridge: a positive verdict reinforces the source memory via an
        // `applied` outcome, and the memory_outcome_apply trigger promotes it up
        // the ladder. Seed strength 3.6 (active) → one applied → 4.1 (≥4.0, no
        // violations) → battle_tested. Second call for the same rec is a no-op.
        let s = pg_store().await;
        let suffix = format!("reinforce_{}", uuid::Uuid::new_v4());
        let (proj_id, _fid, pat_id, mem_id) = seed_pattern_and_sourced_memory(&s, &suffix, 2.6).await;
        let based_on = serde_json::json!({ "patterns": [pat_id] }).to_string();
        let rec_id = uuid::Uuid::new_v4();

        let (before,): (i64,) = sqlx_core::query_as::query_as("SELECT reinforced_count::bigint FROM sensei.memories WHERE id=$1")
            .bind(mem_id).fetch_one(s.pool()).await.unwrap();

        assert!(s.reinforce_source_memory_for_rec(&rec_id, &based_on).await.unwrap(), "first reinforce records an applied outcome");
        let (strength, status, count): (f64, String, i64) = sqlx_core::query_as::query_as(
            "SELECT strength::float8, status::text, reinforced_count::bigint FROM sensei.memories WHERE id=$1"
        ).bind(mem_id).fetch_one(s.pool()).await.unwrap();
        assert!((strength - 4.1).abs() < 1e-6, "strength 3.6→4.1, got {strength}");
        assert_eq!(status, "battle_tested", "promoted once strength >= 4.0 with no violations");
        assert_eq!(count, before + 1, "reinforced_count bumped once");

        // Same rec again → no-op (idempotency marker), count unchanged.
        assert!(!s.reinforce_source_memory_for_rec(&rec_id, &based_on).await.unwrap(), "second reinforce for same rec is a no-op");
        let (count2,): (i64,) = sqlx_core::query_as::query_as("SELECT reinforced_count::bigint FROM sensei.memories WHERE id=$1")
            .bind(mem_id).fetch_one(s.pool()).await.unwrap();
        assert_eq!(count2, before + 1, "reinforced_count unchanged on idempotent re-run");

        cleanup_regressed_fixture(&s, &proj_id).await;
    }

    /// A rec with no resolvable source memory is a clean no-op (not an error):
    /// a pattern that never spawned a memory, and empty/absent provenance.
    #[tokio::test]
    async fn challenge_source_memory_for_rec_no_source_memory_is_noop() {
        let s = pg_store().await;
        let suffix = format!("challenge_nomem_{}", uuid::Uuid::new_v4());
        let (proj_id, fid) = create_test_project_and_folder(&s, &suffix).await;
        // Pattern with NO sourced memory.
        let pat_id = s
            .upsert_pattern(&proj_id, Some(&fid), "_test:orphan-rule", false, None, &serde_json::json!([]))
            .await
            .unwrap();
        let rec_id = uuid::Uuid::new_v4();

        let based_on = serde_json::json!({ "patterns": [pat_id] }).to_string();
        assert!(!s.challenge_source_memory_for_rec(&rec_id, &based_on).await.unwrap(), "no memory sources this pattern → no-op");
        // Empty / absent provenance → no-op, no panic.
        assert!(!s.challenge_source_memory_for_rec(&rec_id, r#"{"patterns":[]}"#).await.unwrap());
        assert!(!s.challenge_source_memory_for_rec(&rec_id, "{}").await.unwrap());
        assert!(s.memory_id_by_source(&pat_id).await.unwrap().is_none(), "sanity: pattern has no learned memory");

        sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1").bind(pat_id).execute(s.pool()).await.ok();
        s.delete_project(&proj_id).await.ok();
    }

    // ── Gateway chains / role assignments tests ─────────────────────

    /// list_chains_with_models returns every active chain, correctly
    /// projects the `role` column, and JSON-aggregates member models in
    /// sequence order. Seeded prod rows drive the shape assertions —
    /// only a chain the test itself creates is deleted at teardown.
    #[tokio::test]
    async fn list_chains_with_models_returns_role_and_ordered_members() {
        let s = pg_store().await;
        let chains = s.list_chains_with_models().await.unwrap();
        assert!(!chains.is_empty(), "seed data should include at least one chain");

        // `reasoning` seeds to role=inference; `embed` to role=embedding.
        let reasoning = chains.iter().find(|c| c["name"] == "reasoning")
            .expect("seed data should include reasoning chain");
        assert_eq!(reasoning["role"], "inference");

        let embed = chains.iter().find(|c| c["name"] == "embed")
            .expect("seed data should include embed chain");
        assert_eq!(embed["role"], "embedding");

        // Utility chain — consensus-proposer — must NOT carry a role.
        let proposer = chains.iter().find(|c| c["name"] == "consensus-proposer")
            .expect("seed data should include consensus-proposer");
        assert!(proposer["role"].is_null(), "utility chains stay unassigned");

        // Members are JSON-aggregated in `sequence_order`.
        let members = reasoning["models"].as_array().expect("models is an array");
        if members.len() >= 2 {
            let first  = members[0]["sequenceOrder"].as_i64().unwrap();
            let second = members[1]["sequenceOrder"].as_i64().unwrap();
            assert!(first < second, "members are ordered by sequence_order asc");
        }
    }

    /// set_chain_role writes the role, clears it on None, and rejects a
    /// role that another chain already owns (the unique-when-set index).
    /// Runs against a scratch chain so seed rows stay intact.
    #[tokio::test]
    async fn set_chain_role_writes_clears_and_rejects_duplicate() {
        let s = pg_store().await;

        // Create a scratch chain with capability=reasoning so it can carry
        // a role at all.
        let scratch_name = format!("_test:chain_{}", uuid::Uuid::new_v4());
        let (scratch_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO gateway.fallback_chains (name, capability, description, is_active)
             VALUES ($1, 'reasoning'::sensei.model_capability, 'scratch', true)
             RETURNING id"
        ).bind(&scratch_name).fetch_one(s.pool()).await.unwrap();

        // Write voice (unassigned by seed) → row now carries the role.
        s.set_chain_role(&scratch_id, Some("voice")).await.unwrap();
        let row: (Option<String>,) = sqlx_core::query_as::query_as(
            "SELECT role::text FROM gateway.fallback_chains WHERE id = $1"
        ).bind(scratch_id).fetch_one(s.pool()).await.unwrap();
        assert_eq!(row.0.as_deref(), Some("voice"));

        // Clear → back to null.
        s.set_chain_role(&scratch_id, None).await.unwrap();
        let row: (Option<String>,) = sqlx_core::query_as::query_as(
            "SELECT role::text FROM gateway.fallback_chains WHERE id = $1"
        ).bind(scratch_id).fetch_one(s.pool()).await.unwrap();
        assert!(row.0.is_none());

        // Taking a role already owned by another chain (seed: reasoning ↔
        // inference) must fail — surfaces as a duplicate-key DB error the
        // caller can map to a 409 CONFLICT.
        let err = s.set_chain_role(&scratch_id, Some("inference")).await
            .expect_err("unique index rejects a second inference chain");
        assert!(err.contains("duplicate") || err.contains("unique"),
                "expected uniqueness violation, got: {err}");

        // Unknown chain id → not-found error, not a silent no-op.
        let ghost = uuid::Uuid::new_v4();
        let err = s.set_chain_role(&ghost, Some("voice")).await
            .expect_err("missing row must error");
        assert!(err.contains("not found"), "expected not-found error, got: {err}");

        // Teardown — remove the scratch chain.
        sqlx_core::query::query("DELETE FROM gateway.fallback_chains WHERE id = $1")
            .bind(scratch_id).execute(s.pool()).await.unwrap();
    }

    /// End-to-end chain-model editing: add → move → remove → compact.
    /// Runs against a scratch chain so seed rows stay intact.
    #[tokio::test]
    async fn chain_model_editing_add_move_remove_compacts_sequence() {
        let s = pg_store().await;

        // Scratch chain, capability=chat so any chat model matches.
        let scratch_name = format!("_test:mchain_{}", uuid::Uuid::new_v4());
        let (chain_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO gateway.fallback_chains (name, capability, description, is_active)
             VALUES ($1, 'chat'::sensei.model_capability, 'scratch', true)
             RETURNING id"
        ).bind(&scratch_name).fetch_one(s.pool()).await.unwrap();

        // Pick three (model, router) pairs with capability=chat.
        let pairs: Vec<(uuid::Uuid, uuid::Uuid)> = sqlx_core::query_as::query_as(
            "SELECT m.id, mir.router_id
               FROM gateway.models m
               JOIN gateway.models_in_router mir ON mir.model_id = m.id
              WHERE m.capabilities @> ARRAY['chat'::sensei.model_capability]
              LIMIT 3"
        ).fetch_all(s.pool()).await.unwrap();
        assert!(pairs.len() >= 2, "test needs at least 2 chat-capable (model, router) pairs; got {}", pairs.len());

        // Add — sequence_order starts at 1 and advances.
        let (row_a, seq_a) = s.add_chain_model(&chain_id, &pairs[0].0, &pairs[0].1).await.unwrap();
        let (row_b, seq_b) = s.add_chain_model(&chain_id, &pairs[1].0, &pairs[1].1).await.unwrap();
        assert_eq!(seq_a, 1);
        assert_eq!(seq_b, 2);

        // Move A down (swap with B).
        let moved = s.move_chain_model(&chain_id, &row_a, 1).await.unwrap();
        assert!(moved, "A should swap with B");
        let (seq_a_now,): (i32,) = sqlx_core::query_as::query_as(
            "SELECT sequence_order FROM gateway.fallback_chain_models WHERE id = $1"
        ).bind(row_a).fetch_one(s.pool()).await.unwrap();
        assert_eq!(seq_a_now, 2, "A now sits at position 2");
        let (seq_b_now,): (i32,) = sqlx_core::query_as::query_as(
            "SELECT sequence_order FROM gateway.fallback_chain_models WHERE id = $1"
        ).bind(row_b).fetch_one(s.pool()).await.unwrap();
        assert_eq!(seq_b_now, 1, "B moved into A's old slot");

        // Move B up past the top — boundary, no-op.
        let moved = s.move_chain_model(&chain_id, &row_b, -1).await.unwrap();
        assert!(!moved, "top boundary should return false");

        // Remove B — A should compact to sequence_order 1.
        s.remove_chain_model(&chain_id, &row_b).await.unwrap();
        let (seq_a_final,): (i32,) = sqlx_core::query_as::query_as(
            "SELECT sequence_order FROM gateway.fallback_chain_models WHERE id = $1"
        ).bind(row_a).fetch_one(s.pool()).await.unwrap();
        assert_eq!(seq_a_final, 1, "A compacts after B removal");

        // Not-found errors surface, not silent no-ops.
        let ghost = uuid::Uuid::new_v4();
        let err = s.remove_chain_model(&chain_id, &ghost).await.expect_err("remove missing must error");
        assert!(err.contains("not found"), "expected not-found, got: {err}");
        let err = s.move_chain_model(&chain_id, &ghost, 1).await.expect_err("move missing must error");
        assert!(err.contains("not found"), "expected not-found, got: {err}");

        // Available list: chain has 1 model, so all others with matching
        // capability are available (excludes the row we still have).
        let available = s.list_available_models_for_chain(&chain_id).await.unwrap();
        assert!(!available.is_empty(), "at least one chat model should be available after removing B");

        // Bad direction rejected with a clear message.
        let err = s.move_chain_model(&chain_id, &row_a, 2).await.expect_err("direction 2 must reject");
        assert!(err.contains("-1") || err.contains("+1"), "expected direction hint, got: {err}");

        // Teardown.
        sqlx_core::query::query("DELETE FROM gateway.fallback_chains WHERE id = $1")
            .bind(chain_id).execute(s.pool()).await.unwrap();
    }

    // ── Communities tests ────────────────────────────────────────────

    #[tokio::test]
    async fn community_upsert_and_list() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("comm_{}", uuid::Uuid::new_v4())).await;
        let cid = s.upsert_community(&fid, 1, "_test:auth_cluster", 3).await.unwrap();
        let comms = s.list_communities(&fid).await.unwrap();
        assert!(comms.iter().any(|c| c["label"] == "_test:auth_cluster" && c["node_count"] == 3));
        sqlx_core::query::query("DELETE FROM inference.communities WHERE id = $1").bind(cid).execute(s.pool()).await.unwrap();
    }

    // ── Reasoning Traces tests ───────────────────────────────────────

    #[tokio::test]
    async fn reasoning_trace_insert_and_get() {
        let s = pg_store().await;
        let pid = s.create_project("_test:rt_proj", None, None).await.unwrap();
        let tid = s.insert_reasoning_trace(
            Some(&pid), "pattern_emerging", &serde_json::json!({}), &["gemma4:27b".into()],
            &serde_json::json!([{"model":"gemma4","role":"proposer","content":"analyze"}]),
            &serde_json::json!({"conclusion":"adopt adapter pattern","confidence":0.9}),
        ).await.unwrap();
        let traces = s.get_reasoning_traces_by_project(&pid).await.unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0]["consensus"]["confidence"], 0.9);
        assert_eq!(traces[0]["trigger_event"], "pattern_emerging");
        sqlx_core::query::query("DELETE FROM inference.reasoning_traces WHERE id = $1").bind(tid).execute(s.pool()).await.unwrap();
        s.delete_project(&pid).await.unwrap();
    }

    // ── Folders to Watch tests ─────────────────────────────────────────

    #[tokio::test]
    async fn watch_root_add_and_list() {
        let s = pg_store().await;
        let path = format!("/_test/watch_{}", uuid::Uuid::new_v4());
        let id = s.add_watch_root(&path, "test_root", &serde_json::json!(["node_modules"])).await.unwrap();
        let roots = s.list_watch_roots().await.unwrap();
        assert!(roots.iter().any(|r| r["path"] == path));
        s.remove_watch_root(&id).await.unwrap();
    }

    #[tokio::test]
    async fn watch_root_update_status() {
        let s = pg_store().await;
        let path = format!("/_test/watch_status_{}", uuid::Uuid::new_v4());
        let id = s.add_watch_root(&path, "test", &serde_json::json!([])).await.unwrap();
        s.update_watch_status(&id, "watching").await.unwrap();
        let roots = s.list_watch_roots().await.unwrap();
        let r = roots.iter().find(|r| r["path"] == path).unwrap();
        assert_eq!(r["status"], "watching");
        s.remove_watch_root(&id).await.unwrap();
    }

    // ── Scan State tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn scan_state_upsert_and_stale() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("scan_{}", uuid::Uuid::new_v4())).await;
        s.upsert_scan_state(&fid, "src/main.rs", 1000, "hash1").await.unwrap();
        // Same mtime = not stale
        let stale = s.get_stale_files(&fid, &[("src/main.rs".into(), 1000)]).await.unwrap();
        assert!(stale.is_empty());
        // Changed mtime = stale
        let stale = s.get_stale_files(&fid, &[("src/main.rs".into(), 2000)]).await.unwrap();
        assert_eq!(stale, vec!["src/main.rs"]);
        // New file = stale
        let stale = s.get_stale_files(&fid, &[("src/new.rs".into(), 1000)]).await.unwrap();
        assert_eq!(stale, vec!["src/new.rs"]);
        s.delete_scan_state(&fid).await.unwrap();
    }

    // ── Services tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn service_upsert_and_list() {
        let s = pg_store().await;
        let name = format!("_test:svc_{}", uuid::Uuid::new_v4());
        let id = s.upsert_service(&name, "Test MCP", "data", "mcp", &serde_json::json!({"url":"http://localhost"})).await.unwrap();
        let svcs = s.list_services().await.unwrap();
        assert!(svcs.iter().any(|sv| sv["name"] == name));
        s.delete_service(&name).await.unwrap();
        let _ = id;
    }

    // ── Snapshots tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn snapshot_create_and_get_latest() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("snap_{}", uuid::Uuid::new_v4())).await;
        let sid = s.create_session(&fid, "snapshot test", None).await.unwrap();
        s.create_snapshot(&sid, &fid, "manual", "Step 1 done", Some("Do step 2"), &["Step 1".into()]).await.unwrap();
        s.create_snapshot(&sid, &fid, "checkpoint", "Step 2 done", None, &["Step 1".into(), "Step 2".into()]).await.unwrap();
        let latest = s.get_latest_snapshot(&sid).await.unwrap().unwrap();
        assert_eq!(latest["progress_summary"], "Step 2 done");
        assert_eq!(latest["kind"], "checkpoint");
        assert_eq!(latest["completed_steps"].as_array().unwrap().len(), 2);
    }

    // ── Detected Patterns tests ────────────────────────────────────────

    #[tokio::test]
    async fn pattern_upsert_and_list() {
        let s = pg_store().await;
        let suffix = format!("pat_upsert_{}", uuid::Uuid::new_v4());
        let (proj_id, fid) = create_test_project_and_folder(&s, &suffix).await;
        let instances = serde_json::json!([{"file":"src/lib.rs","line":10},{"file":"src/main.rs","line":20}]);
        let pat_id = s.upsert_pattern(&proj_id, Some(&fid), "_test:Adapter", false, Some(0.85), &instances).await.unwrap();
        let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
        assert!(patterns.iter().any(|p| p["name"] == "_test:Adapter" && p["instance_count"] == 2));
        // cleanup
        sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
            .bind(pat_id).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(proj_id).execute(s.pool()).await.ok();
    }

    #[tokio::test]
    async fn pattern_promote() {
        let s = pg_store().await;
        let suffix = format!("pat_promote_{}", uuid::Uuid::new_v4());
        let (proj_id, fid) = create_test_project_and_folder(&s, &suffix).await;
        let pat_id = s.upsert_pattern(&proj_id, Some(&fid), "_test:Factory", false, None, &serde_json::json!([])).await.unwrap();
        s.promote_pattern(&pat_id, "rule").await.unwrap();
        let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
        let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
        assert_eq!(p["lifecycle"], "rule");
        sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
            .bind(pat_id).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(proj_id).execute(s.pool()).await.ok();
    }

    #[tokio::test]
    async fn pattern_upsert_updates_existing() {
        let s = pg_store().await;
        let suffix = format!("pat_dup_{}", uuid::Uuid::new_v4());
        let (proj_id, fid) = create_test_project_and_folder(&s, &suffix).await;
        let id1 = s.upsert_pattern(&proj_id, Some(&fid), "_test:Singleton", false, Some(0.5), &serde_json::json!([{"file":"a.rs"}])).await.unwrap();
        let id2 = s.upsert_pattern(&proj_id, Some(&fid), "_test:Singleton", false, Some(0.9), &serde_json::json!([{"file":"a.rs"},{"file":"b.rs"}])).await.unwrap();
        assert_eq!(id1, id2); // same row updated
        let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
        let p = patterns.iter().find(|p| p["name"] == "_test:Singleton").unwrap();
        assert_eq!(p["instance_count"], 2);
        sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
            .bind(id1).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(proj_id).execute(s.pool()).await.ok();
    }

    #[tokio::test]
    async fn pattern_upsert_merges_across_folders_in_same_project() {
        // #82: patterns are project-scoped. Two folders in the same project
        // upserting the same pattern name collapse into a single row — the
        // second upsert updates the first row's instances/folder_id locus.
        let s = pg_store().await;
        let suffix = format!("pat_project_scope_{}", uuid::Uuid::new_v4());
        let (proj_id, fid_a) = create_test_project_and_folder(&s, &suffix).await;
        let fid_b = create_test_folder(&s, &format!("{}_b", suffix)).await;
        sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
            .bind(proj_id).bind(fid_b)
            .execute(s.pool()).await.unwrap();

        let id_a = s.upsert_pattern(&proj_id, Some(&fid_a), "_test:Shared", false, None, &serde_json::json!([{"file":"a.rs"}])).await.unwrap();
        let id_b = s.upsert_pattern(&proj_id, Some(&fid_b), "_test:Shared", false, None, &serde_json::json!([{"file":"b.rs"},{"file":"b2.rs"}])).await.unwrap();
        assert_eq!(id_a, id_b, "same (project_id, name) must merge into one row");

        // The row's instances reflect the latest upsert; folder_id follows too.
        let (count, locus): (i32, uuid::Uuid) = sqlx_core::query_as::query_as(
            "SELECT instance_count, folder_id FROM inference.detected_patterns WHERE id = $1"
        ).bind(id_a).fetch_one(s.pool()).await.unwrap();
        assert_eq!(count, 2);
        assert_eq!(locus, fid_b, "folder_id is the latest upsert's locus");

        // cleanup
        sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
            .bind(id_a).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(proj_id).execute(s.pool()).await.ok();
    }

    // ── Project merge tests (#41) ──────────────────────────────────────

    #[tokio::test]
    async fn merge_projects_moves_folders_sessions_memories_and_deletes_source() {
        let s = pg_store().await;
        let suffix = format!("merge_{}", uuid::Uuid::new_v4());
        let (src, src_folder) = create_test_project_and_folder(&s, &format!("{}_src", suffix)).await;
        let (tgt, _tgt_folder) = create_test_project_and_folder(&s, &format!("{}_tgt", suffix)).await;

        // Seed a memory attributed to the source project so we can prove it
        // survives the merge (only its project_id shifts).
        let mem_id: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memories(project_id, scope, type, title, content, origin)
             VALUES($1, 'project'::sensei.memory_scope, 'convention'::sensei.memory_type, $2, 'body', 'user')
             RETURNING id"
        ).bind(src).bind(format!("_test:merge_memory_{}", uuid::Uuid::new_v4()))
            .fetch_one(s.pool()).await.unwrap();

        s.merge_projects(&src, &tgt).await.unwrap();

        // Source project row is gone.
        let src_exists: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM sensei.projects WHERE id = $1)"
        ).bind(src).fetch_one(s.pool()).await.unwrap();
        assert!(!src_exists.0, "source project should be deleted after merge");

        // The source's folder now lives under the target project.
        let (folder_project,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT project_id FROM sensei.folders WHERE id = $1"
        ).bind(src_folder).fetch_one(s.pool()).await.unwrap();
        assert_eq!(folder_project, Some(tgt), "folder should be reassigned to target");

        // The memory survived and points at the target.
        let (mem_project,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT project_id FROM sensei.memories WHERE id = $1"
        ).bind(mem_id.0).fetch_one(s.pool()).await.unwrap();
        assert_eq!(mem_project, Some(tgt), "user-authored memory should follow to target");

        // cleanup
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
            .bind(mem_id.0).execute(s.pool()).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(tgt).execute(s.pool()).await.ok();
    }

    #[tokio::test]
    async fn merge_projects_rejects_self_merge() {
        let s = pg_store().await;
        let (pid, _fid) = create_test_project_and_folder(&s, &format!("selfmerge_{}", uuid::Uuid::new_v4())).await;
        let err = s.merge_projects(&pid, &pid).await.unwrap_err();
        assert!(err.contains("must differ"), "got: {err}");
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(pid).execute(s.pool()).await.ok();
    }

    #[tokio::test]
    async fn merge_projects_errors_on_missing_ids() {
        let s = pg_store().await;
        let ghost = uuid::Uuid::new_v4();
        let (real, _fid) = create_test_project_and_folder(&s, &format!("mergemiss_{}", uuid::Uuid::new_v4())).await;
        let err = s.merge_projects(&ghost, &real).await.unwrap_err();
        assert!(err.contains("expected source + target to exist"), "got: {err}");
        // The real project is untouched.
        let exists: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM sensei.projects WHERE id = $1)"
        ).bind(real).fetch_one(s.pool()).await.unwrap();
        assert!(exists.0);
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(real).execute(s.pool()).await.ok();
    }

    // ── Bug 3: re-absorb a standalone root mis-scoped inside a git repo ────

    #[tokio::test]
    async fn heal_nested_standalone_roots_reabsorbs_and_removes_phantom() {
        let s = pg_store().await;
        let uniq = uuid::Uuid::new_v4();
        let root_path = format!("/_test/heal_nested/{uniq}");
        let root_id = s.add_watch_root(&root_path, "heal_nested_root", &serde_json::json!([])).await.unwrap();

        // A git repo (like the sensei monorepo) with its own project.
        let repo_abs = format!("{root_path}/repo");
        let repo_pid = s.create_project(&format!("_test:heal_repo_{uniq}"), None, None).await.unwrap();
        let repo_fid = s.upsert_repo_kind(&root_id, "git", "repo", &repo_abs).await.unwrap();
        sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
            .bind(repo_pid).bind(repo_fid).execute(s.pool()).await.unwrap();

        // A sub-crate INSIDE the repo, mis-scoped as its own standalone project
        // (the Bug 3 phantom). Give it a node so we can prove its nodes are dropped.
        let crate_abs = format!("{repo_abs}/crates/dojo-mind");
        let phantom_pid = s.create_project(&format!("_test:heal_phantom_{uniq}"), None, None).await.unwrap();
        let crate_fid = s.upsert_repo_kind(&root_id, "standalone", "dojo-mind", &crate_abs).await.unwrap();
        sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
            .bind(phantom_pid).bind(crate_fid).execute(s.pool()).await.unwrap();
        let node_id = s.upsert_node(&crate_fid, "struct", "DojoStore", "src/store.rs", None, None, None, None).await.unwrap();

        // Heal.
        let healed = s.heal_nested_standalone_roots().await.unwrap();
        assert!(healed >= 1, "the nested standalone root should be re-absorbed");

        // The nested root is now a folder of the repo's project, parented to the repo.
        let (kind, pid, parent): (String, Option<uuid::Uuid>, Option<uuid::Uuid>) =
            sqlx_core::query_as::query_as("SELECT kind::text, project_id, parent_id FROM sensei.folders WHERE id = $1")
                .bind(crate_fid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(kind, "folder", "mis-scoped standalone should be re-classified as a folder");
        assert_eq!(pid, Some(repo_pid), "should now belong to the enclosing repo's project");
        assert_eq!(parent, Some(repo_fid), "should be parented under the enclosing repo");

        // Its own nodes were dropped (the repo re-indexes the subtree).
        let (node_exists,): (bool,) = sqlx_core::query_as::query_as("SELECT EXISTS(SELECT 1 FROM sensei.nodes WHERE id = $1)")
            .bind(node_id).fetch_one(s.pool()).await.unwrap();
        assert!(!node_exists, "the mis-scoped root's own nodes should be pruned");

        // The phantom project (lived entirely inside the repo) is gone.
        let (phantom_exists,): (bool,) = sqlx_core::query_as::query_as("SELECT EXISTS(SELECT 1 FROM sensei.projects WHERE id = $1)")
            .bind(phantom_pid).fetch_one(s.pool()).await.unwrap();
        assert!(!phantom_exists, "the phantom project should be merged away");

        // Idempotent for THIS test's rows: a second run leaves the phantom merged
        // away. The returned count is GLOBAL — db-gated tests share `sensei_test`
        // and other tests (e.g. the index-audit suite) may seed nested-standalone
        // rows concurrently — so assert on our own row, not the global count.
        s.heal_nested_standalone_roots().await.unwrap();
        let (phantom_gone_after_rerun,): (bool,) = sqlx_core::query_as::query_as("SELECT NOT EXISTS(SELECT 1 FROM sensei.projects WHERE id = $1)")
            .bind(phantom_pid).fetch_one(s.pool()).await.unwrap();
        assert!(phantom_gone_after_rerun, "re-run leaves the phantom merged away (idempotent)");

        // cleanup
        s.delete_folder_tree(&repo_fid).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(repo_pid).execute(s.pool()).await.ok();
        s.remove_watch_root(&root_id).await.ok();
    }

    #[tokio::test]
    async fn list_indexed_files_excludes_modules_and_empties() {
        let s = pg_store().await;
        let uniq = uuid::Uuid::new_v4();
        let root_path = format!("/_test/idx_files/{uniq}");
        let root_id = s.add_watch_root(&root_path, "idx_files_root", &serde_json::json!([])).await.unwrap();
        let repo_abs = format!("{root_path}/repo");
        let fid = s.upsert_repo_kind(&root_id, "git", "repo", &repo_abs).await.unwrap();

        s.upsert_node(&fid, "file", "a.rs", "a.rs", None, None, None, None).await.unwrap();
        s.upsert_node(&fid, "struct", "B", "b.rs", None, None, None, None).await.unwrap();
        // A module node records an ABSOLUTE dir path — must be excluded so it never
        // pollutes the rel-path comparison in prune_vanished.
        s.upsert_node(&fid, "module", "src", &format!("{repo_abs}/src"), None, None, None, None).await.unwrap();

        let mut files = s.list_indexed_files(&fid).await.unwrap();
        files.sort();
        assert_eq!(files, vec!["a.rs".to_string(), "b.rs".to_string()], "only real (rel) file paths, no module");

        s.delete_folder_tree(&fid).await.ok();
        s.remove_watch_root(&root_id).await.ok();
    }

    // ── Activity pruner tests (#74) ────────────────────────────────────

    #[tokio::test]
    async fn prune_activity_keeps_unanalyzed_sessions_even_when_old() {
        let s = pg_store().await;
        let suffix = format!("prune_keep_unanalyzed_{}", uuid::Uuid::new_v4());
        let (_pid, fid) = create_test_project_and_folder(&s, &suffix).await;
        let csid = format!("{}-csid", suffix);
        let sid = s.record_session_event(&csid, &fid, None, "claude", true).await.unwrap();
        // Age the session past the cutoff but leave analyzed_at NULL.
        sqlx_core::query::query(
            "UPDATE activity.sessions SET started_at = now() - interval '90 days' WHERE id = $1"
        ).bind(sid).execute(s.pool()).await.unwrap();

        // Other tests may seed analyzed sessions, so the global count is not
        // useful — verify OUR session specifically survives. The analyzed-only
        // guard keeps it regardless of the capture-before-reclaim backstop
        // (backstop=60 here), so this assertion is unaffected by that guard.
        let day_keyed = crate::tasks::handlers::metrics::planner::day_keyed_task_names();
        s.prune_activity(30, 60, &day_keyed).await.unwrap();

        let exists: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM activity.sessions WHERE id = $1)"
        ).bind(sid).fetch_one(s.pool()).await.unwrap();
        assert!(exists.0, "unanalyzed session must survive prune");

        sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
            .bind(sid).execute(s.pool()).await.ok();
    }

    #[tokio::test]
    async fn prune_activity_deletes_analyzed_sessions_past_cutoff_and_children() {
        let s = pg_store().await;
        let suffix = format!("prune_del_{}", uuid::Uuid::new_v4());
        let (pid, fid) = create_test_project_and_folder(&s, &suffix).await;
        let csid = format!("{}-csid", suffix);
        let sid = s.record_session_event(&csid, &fid, None, "claude", true).await.unwrap();
        // Age to 40 days: past the 30-day retention window but INSIDE the 60-day
        // backstop, so the ONLY thing that makes it prune-eligible is the
        // capture path — its day must already exist in sensei.project_metrics.
        // (Previously aged 90 days, which pruned unconditionally; now the test
        // exercises capture-before-reclaim directly.)
        sqlx_core::query::query(
            "UPDATE activity.sessions
                SET started_at = date_trunc('day', now() - interval '40 days'),
                    analyzed_at = now() - interval '39 days'
              WHERE id = $1"
        ).bind(sid).execute(s.pool()).await.unwrap();
        // Seed a covering daily project_metrics row for the session's day so the
        // capture-before-reclaim guard is satisfied (the durable snapshot exists).
        let day40: (chrono::NaiveDate,) = sqlx_core::query_as::query_as(
            "SELECT (date_trunc('day', now() - interval '40 days'))::date"
        ).fetch_one(s.pool()).await.unwrap();
        let ftr_id: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.metrics WHERE key = 'ftr'"
        ).fetch_one(s.pool()).await.unwrap();
        s.upsert_project_metric(&ftr_id.0, &pid, None, None, day40.0, "daily", 1.0,
            &serde_json::json!({}), "measured").await.unwrap();
        // Seed a child transcript_turn keyed on client_session_id (no FK).
        sqlx_core::query::query(
            "INSERT INTO activity.transcript_turns(session_id, source, turn_index, assistant_text)
             VALUES ($1, 'claude', 0, 'hello')"
        ).bind(&csid).execute(s.pool()).await.unwrap();
        // Seed a hook event under the same client_session_id.
        s.insert_hook_event(&csid, "claude", "UserPromptSubmit", None, None, 1000, None,
            &serde_json::json!({"prompt": "hi"})).await.unwrap();

        // Counts include any leftover analyzed+old data from other tests, so
        // don't assert exact numbers — assert OUR session (and its child
        // rows) are gone after the prune. backstop=60 > 40d age, so the capture
        // row (not the backstop) is what enables the prune. The covering row is
        // an `ftr` (session_outcomes = DAY-KEYED) metric, so it still counts as
        // captured under the scoped guard.
        let day_keyed = crate::tasks::handlers::metrics::planner::day_keyed_task_names();
        s.prune_activity(30, 60, &day_keyed).await.unwrap();

        let exists: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM activity.sessions WHERE id = $1)"
        ).bind(sid).fetch_one(s.pool()).await.unwrap();
        assert!(!exists.0, "analyzed + old session must be pruned");

        let tt: (i64,) = sqlx_core::query_as::query_as(
            "SELECT COUNT(*) FROM activity.transcript_turns WHERE session_id = $1"
        ).bind(&csid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(tt.0, 0, "transcript_turns keyed on this session must be gone");
    }

    /// Capture-before-reclaim (2026-08-12 retention decision): an analyzed
    /// session past retention is reclaimable ONLY when its day is captured by a
    /// DAY-KEYED (delivery) metric in sensei.project_metrics OR it is older than
    /// the hard backstop. This proves all four arms with retention=30, backstop=60
    /// on four same-project sessions dated: 40d + captured by a day-keyed `ftr`
    /// row (prune), 45d + uncaptured (KEEP), 50d + covered ONLY by a forward-only
    /// snapshot `duplication_ratio` row (KEEP — a snapshot row must NOT mark the
    /// day captured), 90d + uncaptured (prune via backstop).
    #[tokio::test]
    async fn prune_activity_captures_before_reclaim() {
        let s = pg_store().await;
        let suffix = format!("prune_cbr_{}", uuid::Uuid::new_v4());
        let (pid, fid) = create_test_project_and_folder(&s, &suffix).await;
        let ftr_id: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.metrics WHERE key = 'ftr'"
        ).fetch_one(s.pool()).await.unwrap();

        // Helper: an analyzed session dated `age_days` ago in this folder.
        async fn aged_session(s: &PgStore, fid: &uuid::Uuid, suffix: &str, tag: &str, age_days: i32) -> uuid::Uuid {
            let csid = format!("{suffix}-{tag}");
            let sid = s.record_session_event(&csid, fid, None, "claude", true).await.unwrap();
            sqlx_core::query::query(
                "UPDATE activity.sessions
                    SET started_at = date_trunc('day', now() - (interval '1 day' * $2)),
                        analyzed_at = now() - (interval '1 day' * ($2 - 1))
                  WHERE id = $1"
            ).bind(sid).bind(age_days).execute(s.pool()).await.unwrap();
            sid
        }

        // (a) captured + past retention, inside backstop → PRUNED via capture. The
        //     covering row is `ftr` = session_outcomes, a DAY-KEYED metric, so it
        //     satisfies the scoped capture guard.
        let captured = aged_session(&s, &fid, &suffix, "captured", 40).await;
        let day40: (chrono::NaiveDate,) = sqlx_core::query_as::query_as(
            "SELECT (date_trunc('day', now() - interval '40 days'))::date"
        ).fetch_one(s.pool()).await.unwrap();
        s.upsert_project_metric(&ftr_id.0, &pid, None, None, day40.0, "daily", 1.0,
            &serde_json::json!({}), "measured").await.unwrap();

        // (b) uncaptured + past retention, inside backstop → KEPT. A DIFFERENT
        //     day (45d) than the captured one so its day is genuinely uncovered
        //     (capture is scoped per project-day, not per session).
        let uncaptured = aged_session(&s, &fid, &suffix, "uncaptured", 45).await;

        // (c) covered ONLY by a FORWARD-ONLY snapshot metric (`duplication_ratio`,
        //     task_name='duplication'), past retention, inside backstop → KEPT.
        //     This is the load-bearing case for the scoped guard: a snapshot
        //     computer stamps a grain='daily' row on its own day on every run, so
        //     an UNscoped EXISTS would treat this day as "captured" and reclaim the
        //     session before its DELIVERY (day-keyed) metric ever computed —
        //     reintroducing the data loss. The scoped guard requires a day-keyed
        //     metric, so this session stays until one lands (or the backstop).
        let snapshot_only = aged_session(&s, &fid, &suffix, "snaponly", 50).await;
        let day50: (chrono::NaiveDate,) = sqlx_core::query_as::query_as(
            "SELECT (date_trunc('day', now() - interval '50 days'))::date"
        ).fetch_one(s.pool()).await.unwrap();
        let dup_id: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.metrics WHERE key = 'duplication_ratio'"
        ).fetch_one(s.pool()).await.unwrap();
        s.upsert_project_metric(&dup_id.0, &pid, None, None, day50.0, "daily", 0.4,
            &serde_json::json!({}), "measured").await.unwrap();

        // (d) uncaptured + past the backstop (90d > 60) → PRUNED via backstop.
        let past_backstop = aged_session(&s, &fid, &suffix, "backstop", 90).await;

        let day_keyed = crate::tasks::handlers::metrics::planner::day_keyed_task_names();
        s.prune_activity(30, 60, &day_keyed).await.unwrap();

        async fn alive(s: &PgStore, sid: uuid::Uuid) -> bool {
            let r: (bool,) = sqlx_core::query_as::query_as(
                "SELECT EXISTS(SELECT 1 FROM activity.sessions WHERE id = $1)"
            ).bind(sid).fetch_one(s.pool()).await.unwrap();
            r.0
        }
        assert!(!alive(&s, captured).await, "day captured by a day-keyed metric → pruned");
        assert!(alive(&s, uncaptured).await, "uncaptured day inside backstop → kept (durable until snapshot exists)");
        assert!(alive(&s, snapshot_only).await,
            "day covered ONLY by a forward-only snapshot metric is NOT captured → kept (a snapshot row must not mask the missing delivery metric)");
        assert!(!alive(&s, past_backstop).await, "uncaptured but past backstop → pruned so nothing lingers forever");

        // Clean up the survivors + their covering metric rows.
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = ANY($1::uuid[])")
            .bind(vec![uncaptured, snapshot_only]).execute(s.pool()).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid).execute(s.pool()).await.ok();
    }

    #[tokio::test]
    async fn prune_activity_prunes_orphan_events_by_ts() {
        let s = pg_store().await;
        // Insert an assistant_event with no matching session and old ts.
        let old_ts: i64 = (chrono::Utc::now() - chrono::Duration::days(90)).timestamp() * 1000;
        let orphan_csid = format!("orphan_prune_{}", uuid::Uuid::new_v4());
        s.insert_hook_event(&orphan_csid, "claude", "PostToolUse", Some("Read"), None, old_ts, None,
            &serde_json::json!({})).await.unwrap();

        // prune_activity's returned count is GLOBAL across the shared test DB — a
        // sibling db-gated test may prune this row concurrently, so don't assert on
        // the count. The per-row check below deterministically proves our orphan
        // (unique csid) was pruned. Session-less orphan events are pruned by ts
        // alone (no capture-before-reclaim guard), so the backstop + day-keyed args
        // are inert.
        let day_keyed = crate::tasks::handlers::metrics::planner::day_keyed_task_names();
        s.prune_activity(30, 60, &day_keyed).await.unwrap();

        let orphaned: (i64,) = sqlx_core::query_as::query_as(
            "SELECT COUNT(*) FROM activity.assistant_events WHERE session_id = $1"
        ).bind(&orphan_csid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(orphaned.0, 0);
    }

    // ── Corrections aggregation tests ──────────────────────────────────

    #[tokio::test]
    async fn correction_upsert_is_idempotent_by_signature() {
        let s = pg_store().await;
        let p = uuid::Uuid::new_v4();
        let sig = format!("corr-test-{}", uuid::Uuid::new_v4());
        let row = crate::corrections::CorrectionRow {
            signature: sig.clone(),
            text: "Use $state for reactive locals".into(),
            suggestion: Some("Reinforce the svelte5 memory".into()),
            count: 3,
            project_ids: vec![p],
            last_seen: chrono::Utc::now(),
            memory_id: None,
            instances: serde_json::json!([{"session_id": "s1", "ts": 1, "prompt": "use $state"}]),
        };
        let id1 = s.upsert_correction(&row).await.unwrap();
        let mut row2 = row.clone();
        row2.count = 4;
        let id2 = s.upsert_correction(&row2).await.unwrap();
        assert_eq!(id1, id2, "same signature updates the same row");

        let global = s.list_corrections().await.unwrap();
        let found = global["corrections"].as_array().unwrap().iter()
            .find(|c| c["id"] == id1.to_string()).unwrap().clone();
        assert_eq!(found["count"], 4);
        assert_eq!(found["text"], "Use $state for reactive locals");

        // the project-scoped read exercises the `$1 = ANY(project_ids)` filter.
        let scoped = s.list_corrections_for_project(&p).await.unwrap();
        assert!(
            scoped["corrections"].as_array().unwrap().iter().any(|c| c["id"] == id1.to_string()),
            "per-project read returns a correction tagged with that project"
        );

        // prune keeping our signature → the row survives.
        s.delete_corrections_not_in(std::slice::from_ref(&sig)).await.unwrap();
        let kept = s.list_corrections().await.unwrap();
        assert!(
            kept["corrections"].as_array().unwrap().iter().any(|c| c["id"] == id1.to_string()),
            "a kept signature survives the prune"
        );

        // prune excluding our signature → the row is deleted (also leaves the
        // test DB clean — this clears the derived corrections table).
        s.delete_corrections_not_in(&["corr-nope".to_string()]).await.unwrap();
        let after = s.list_corrections().await.unwrap();
        assert!(
            !after["corrections"].as_array().unwrap().iter().any(|c| c["id"] == id1.to_string()),
            "a signature not in the keep set is pruned"
        );
    }

    // ── Libraries tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn library_upsert_and_get() {
        let s = pg_store().await;
        let id = s.upsert_library("_test:tokio", "cargo", Some("1.0"), Some("async runtime"), None, None).await.unwrap();
        let lib = s.get_library(&id).await.unwrap().unwrap();
        assert_eq!(lib["name"], "_test:tokio");
        assert_eq!(lib["ecosystem"], "cargo");
        assert_eq!(lib["version"], "1.0");
        s.delete_library(&id).await.unwrap();
    }

    #[tokio::test]
    async fn upsert_project_dependency_is_idempotent_and_stores_all_columns() {
        // 1a Step 5: project → project edges must be idempotent on the
        // composite PK (from_project, to_project, from_folder, source_manifest)
        // and must preserve source_protocol and resolved_target across upserts.
        let s = pg_store().await;
        let from_pid = s.ensure_test_project(&format!("dep-from-{}", uuid::Uuid::new_v4())).await.unwrap();
        let to_pid   = s.ensure_test_project(&format!("dep-to-{}", uuid::Uuid::new_v4())).await.unwrap();
        let from_fid = create_test_folder(&s, &format!("pd-{}", uuid::Uuid::new_v4())).await;

        // First upsert
        s.upsert_project_dependency(
            &from_pid, &to_pid, &from_fid, "link", "package.json", Some("../actions"),
        ).await.unwrap();
        // Repeat with a different resolved_target — same PK, so this must
        // update in place (last-writer wins on non-key columns).
        s.upsert_project_dependency(
            &from_pid, &to_pid, &from_fid, "link", "package.json", Some("../actions-renamed"),
        ).await.unwrap();

        use sqlx_core::query_as::query_as;
        let rows: Vec<(String, Option<String>)> = query_as(
            "SELECT source_protocol, resolved_target
               FROM sensei.project_dependencies
              WHERE from_project_id = $1 AND to_project_id = $2 AND from_folder_id = $3"
        ).bind(from_pid).bind(to_pid).bind(from_fid)
         .fetch_all(s.pool()).await.unwrap();

        assert_eq!(rows.len(), 1, "composite PK must dedupe two upserts");
        assert_eq!(rows[0].0, "link", "protocol preserved");
        assert_eq!(rows[0].1.as_deref(), Some("../actions-renamed"), "target updated in place");

        // Cleanup
        sqlx_core::query::query("DELETE FROM sensei.project_dependencies WHERE from_folder_id = $1")
            .bind(from_fid).execute(s.pool()).await.unwrap();
        s.delete_project(&from_pid).await.ok();
        s.delete_project(&to_pid).await.ok();
    }

    #[tokio::test]
    async fn version_conflicts_view_flags_multi_version_pins_and_excludes_local() {
        // 1a Step 7-8: two folders in the same project pin the same library
        // at DIFFERENT versions → surfaces as a conflict. A third row tagged
        // local_source (as if declared via link:) with a different version
        // must NOT contribute to the conflict.
        let s = pg_store().await;
        let suffix = uuid::Uuid::new_v4();
        let pid = s.ensure_test_project(&format!("vc-{suffix}")).await.unwrap();
        let lib = s.upsert_library(&format!("_test:vc-lib-{suffix}"), "npm", Some("1.0"), None, None, None).await.unwrap();

        // Two folders in the same project, different versions.
        let fid_a = create_test_folder(&s, &format!("vc-a-{suffix}")).await;
        let fid_b = create_test_folder(&s, &format!("vc-b-{suffix}")).await;
        // Attach folders to the project.
        sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id IN ($2, $3)")
            .bind(pid).bind(fid_a).bind(fid_b)
            .execute(s.pool()).await.unwrap();

        s.upsert_referenced_library(&fid_a, &lib, Some("1.2.0"), None).await.unwrap();
        s.upsert_referenced_library(&fid_b, &lib, Some("1.3.0"), None).await.unwrap();

        // Third folder pins a local-source variant. This must be excluded.
        let fid_local = create_test_folder(&s, &format!("vc-local-{suffix}")).await;
        sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
            .bind(pid).bind(fid_local).execute(s.pool()).await.unwrap();
        s.upsert_referenced_library(
            &fid_local, &lib, Some("workspace-42"),
            Some(serde_json::json!({"local_source": "../lib"})),
        ).await.unwrap();

        let rows = s.list_project_library_version_conflicts(&pid).await.unwrap();
        assert_eq!(rows.len(), 1, "one lib with two registry-version pins → one row");
        let r = &rows[0];
        assert_eq!(r["library_id"].as_str().unwrap(), lib.to_string());
        let versions: Vec<String> = r["versions"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap().to_string()).collect();
        assert_eq!(versions, vec!["1.2.0".to_string(), "1.3.0".to_string()],
                   "versions must be sorted + distinct; workspace-42 excluded because local_source is tagged");

        // Cleanup — library FK cascades referenced_libraries; then delete
        // project (folders cascade because project_id set null).
        s.delete_library(&lib).await.unwrap();
        s.delete_project(&pid).await.ok();
    }

    #[tokio::test]
    async fn list_project_dependencies_joins_target_name_and_folder() {
        // 1a Step 6: the list endpoint returns each outgoing edge with the
        // TARGET project's name and the source folder's name joined in.
        let s = pg_store().await;
        let suffix = uuid::Uuid::new_v4();
        let from_pid = s.ensure_test_project(&format!("lpd-from-{suffix}")).await.unwrap();
        let to_pid   = s.ensure_test_project(&format!("lpd-to-{suffix}")).await.unwrap();
        let from_fid = create_test_folder(&s, &format!("lpd-fid-{suffix}")).await;

        s.upsert_project_dependency(
            &from_pid, &to_pid, &from_fid, "link", "package.json", Some("../actions"),
        ).await.unwrap();

        let deps = s.list_project_dependencies(&from_pid).await.unwrap();

        assert_eq!(deps.len(), 1);
        let d = &deps[0];
        assert_eq!(d["to_project_id"].as_str().unwrap(), to_pid.to_string());
        assert!(d["to_project_name"].as_str().unwrap().starts_with("_test:lpd-to-"),
                "target project name must be joined in");
        assert!(d["from_folder"].as_str().unwrap().starts_with("lpd-fid-"),
                "source folder name must be joined in");
        assert_eq!(d["source_protocol"], "link");
        assert_eq!(d["source_manifest"], "package.json");
        assert_eq!(d["resolved_target"], "../actions");

        // Reverse direction returns empty
        let none = s.list_project_dependencies(&to_pid).await.unwrap();
        assert!(none.is_empty(), "target project has no outgoing edges");

        sqlx_core::query::query("DELETE FROM sensei.project_dependencies WHERE from_folder_id = $1")
            .bind(from_fid).execute(s.pool()).await.unwrap();
        s.delete_project(&from_pid).await.ok();
        s.delete_project(&to_pid).await.ok();
    }

    #[tokio::test]
    async fn upsert_project_dependency_rejects_self_edges() {
        // 1a Step 5: DDL check constraint (from_project_id <> to_project_id)
        // must reject self-edges at the write path.
        let s = pg_store().await;
        let pid = s.ensure_test_project(&format!("self-{}", uuid::Uuid::new_v4())).await.unwrap();
        let fid = create_test_folder(&s, &format!("self-fid-{}", uuid::Uuid::new_v4())).await;

        let err = s.upsert_project_dependency(
            &pid, &pid, &fid, "path", "Cargo.toml", Some("."),
        ).await;

        assert!(err.is_err(), "self-edge must be rejected");
        assert!(err.unwrap_err().contains("check"), "err message must reference the check constraint");

        s.delete_project(&pid).await.ok();
    }

    #[tokio::test]
    async fn upsert_referenced_library_merges_props() {
        // 1a Step 3: props must accumulate across passes, not overwrite. A
        // first pass tags {"local_source": "../actions"} for a link:/path=
        // dep; a later pass adding {"pinned": true} must produce the merged
        // {"local_source": "../actions", "pinned": true}.
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("refprops_{}", uuid::Uuid::new_v4())).await;
        let lib = s.upsert_library(&format!("_test:refprops-{}", uuid::Uuid::new_v4()), "npm", Some("1.0"), None, None, None).await.unwrap();

        s.upsert_referenced_library(
            &fid, &lib, Some("1.0"),
            Some(serde_json::json!({"local_source": "../actions"})),
        ).await.unwrap();

        s.upsert_referenced_library(
            &fid, &lib, Some("1.0"),
            Some(serde_json::json!({"pinned": true})),
        ).await.unwrap();

        use sqlx_core::query_as::query_as;
        let (props,): (serde_json::Value,) = query_as(
            "SELECT props FROM sensei.referenced_libraries WHERE folder_id = $1 AND library_id = $2"
        ).bind(fid).bind(lib).fetch_one(s.pool()).await.unwrap();

        assert_eq!(props["local_source"], "../actions", "first pass tag must persist");
        assert_eq!(props["pinned"], true, "second pass tag must merge in");

        // Cleanup — library delete cascades referenced_libraries via FK.
        s.delete_library(&lib).await.unwrap();
    }

    #[tokio::test]
    async fn project_library_promotion_shows_in_resolved_and_is_idempotent() {
        // #30: referenced_libraries (folder-grained) must roll up to
        // project_libraries so detected libs — incl. scoped @rokkit/* — show in
        // project_libraries_resolved (the Projects screen). Was never populated.
        let s = pg_store().await;
        let pid = s.ensure_test_project("proj-lib-promo").await.unwrap();
        let lib = s.upsert_library("_test:@rokkit/core", "npm", Some("1.2"), None, None, None).await.unwrap();
        // Promote twice — must be idempotent (no error, no duplicate row).
        s.upsert_project_library(&lib, &pid).await.unwrap();
        s.upsert_project_library(&lib, &pid).await.unwrap();
        let libs = s.get_project_libraries(&pid).await.unwrap();
        let hits = libs.iter().filter(|l| l["name"] == "_test:@rokkit/core").count();
        assert_eq!(hits, 1, "promoted scoped lib should appear exactly once in resolved view; got {libs:?}");
        s.delete_library(&lib).await.unwrap(); // FK CASCADE removes the project_libraries row
    }

    #[tokio::test]
    async fn ensure_test_project_is_namespaced_and_idempotent() {
        // #34: test fixtures must not accrete a new row per run, nor look like
        // real projects. Reuse one `_test:`-namespaced row per name.
        let s = pg_store().await;
        let a = s.ensure_test_project("dup-check").await.unwrap();
        let b = s.ensure_test_project("dup-check").await.unwrap();
        assert_eq!(a, b, "repeated ensure_test_project must reuse one row, not create a new one");
        let proj = s.get_project(&a).await.unwrap().unwrap();
        assert_eq!(proj["name"], "_test:dup-check", "test projects must be _test:-namespaced");
        s.delete_project(&a).await.ok();
    }

    #[tokio::test]
    async fn find_folder_for_path_returns_nearest_ancestor() {
        // #31: a hook's cwd (often a subdir) must resolve to its indexed folder.
        let s = pg_store().await;
        let fid = create_test_folder(&s, "sess-nearest").await; // abs_path /_test/sess-nearest
        assert_eq!(s.find_folder_for_path("/_test/sess-nearest/src/auth").await.unwrap()
            .map(|(id, _)| id), Some(fid), "subdir cwd resolves to ancestor folder");
        assert_eq!(s.find_folder_for_path("/_test/sess-nearest").await.unwrap()
            .map(|(id, _)| id), Some(fid), "exact path resolves too");
        assert_eq!(s.find_folder_for_path("/_test/nonexistent-xyz/deep").await.unwrap(), None,
            "uncovered path resolves to nothing");
    }

    /// The folder a repaired/created session row points at (test assertion helper).
    async fn session_row_folder(s: &PgStore, client_session_id: &str) -> Option<uuid::Uuid> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT folder_id FROM activity.sessions WHERE client_session_id = $1",
        )
        .bind(client_session_id)
        .fetch_optional(&s.pool)
        .await
        .unwrap();
        row.map(|(f,)| f)
    }

    // The repair operates on ALL orphaned sessions in the DB, and sensei_test is
    // persistent + tests run in parallel, so a prior run's rows can linger. Each repair
    // test clears its OWN session first, then asserts only the (deterministic) folder its
    // session resolves to after repair — never a global "no row exists" precondition.
    async fn clear_test_session(s: &PgStore, sid: &str) {
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE client_session_id = $1")
            .bind(sid).execute(&s.pool).await.unwrap();
        sqlx_core::query::query("DELETE FROM activity.assistant_events WHERE session_id = $1")
            .bind(sid).execute(&s.pool).await.unwrap();
    }

    #[tokio::test]
    async fn repair_orphaned_sessions_reattaches_via_alias() {
        // A session captured under a since-renamed repo: its events survived but the
        // session row was cascade-deleted. The repair recreates the row, resolving the
        // folder from the (old) cwd via the alias.
        let s = pg_store().await;
        let sess = "_test-repair-orphan-session";
        clear_test_session(&s, sess).await;
        let fid = create_test_folder(&s, "repair-new").await; // /_test/repair-new
        s.add_folder_path_alias("/_test/repair-old", &fid, "rename").await.unwrap();
        // an orphaned event under the OLD path (a subdir) — no session row.
        s.insert_hook_event(sess, "claude", "PreToolUse", None, Some("/_test/repair-old/src"), 1_700_000_000, None, &serde_json::json!({}))
            .await.unwrap();
        let repaired = s.repair_orphaned_sessions().await.unwrap();
        assert!(repaired >= 1, "at least this orphaned session is re-attached; got {repaired}");
        assert_eq!(session_row_folder(&s, sess).await, Some(fid),
            "the session row now exists and points at the current folder (resolved via the alias)");
    }

    #[tokio::test]
    async fn repair_prefers_the_renamed_subdir_over_a_live_parent() {
        // The defect this guards: a session with events under BOTH a still-live parent
        // (`/_test/shadow-parent`) AND a renamed subdir aliased to a different folder
        // (`/_test/shadow-parent/sub` → new folder). Most-specific-first must attribute
        // it to the renamed subdir's folder, not the shadowing parent.
        let s = pg_store().await;
        let sess = "_test-shadow-session";
        clear_test_session(&s, sess).await;
        let parent = create_test_folder(&s, "shadow-parent").await; // live /_test/shadow-parent
        let moved = create_test_folder(&s, "shadow-moved").await; // the renamed subdir's new home
        s.add_folder_path_alias("/_test/shadow-parent/sub", &moved, "rename").await.unwrap();
        // events under BOTH the live parent and the renamed subdir.
        s.insert_hook_event(sess, "claude", "PreToolUse", None, Some("/_test/shadow-parent"), 1_700_000_100, None, &serde_json::json!({})).await.unwrap();
        s.insert_hook_event(sess, "claude", "PreToolUse", None, Some("/_test/shadow-parent/sub/x"), 1_700_000_200, None, &serde_json::json!({})).await.unwrap();
        s.repair_orphaned_sessions().await.unwrap();
        assert_eq!(session_row_folder(&s, sess).await, Some(moved),
            "attributes to the renamed subdir (via its alias), NOT the shadowing live parent");
        assert_ne!(session_row_folder(&s, sess).await, Some(parent));
    }

    async fn set_folder_remotes(s: &PgStore, id: &uuid::Uuid, urls: &[&str]) {
        let json = serde_json::Value::Array(
            urls.iter().map(|u| serde_json::json!({"name": "origin", "url": u})).collect(),
        );
        sqlx_core::query::query("UPDATE sensei.folders SET remote_urls = $2 WHERE id = $1")
            .bind(id).bind(&json).execute(&s.pool).await.unwrap();
    }

    #[tokio::test]
    async fn find_live_root_by_remote_matches_only_a_live_path_sharing_a_url() {
        let s = pg_store().await;
        let url = "git@github.com:sensei-hq/remote-probe.git";
        let live = create_test_folder(&s, "remote-live").await; // /_test/remote-live
        set_folder_remotes(&s, &live, &[url]).await;
        let live_abs = vec!["/_test/remote-live".to_string()];

        assert_eq!(s.find_live_root_by_remote(&[url.to_string()], &live_abs).await.unwrap(), Some(live),
            "a live root sharing the git remote is the remap target");
        assert_eq!(s.find_live_root_by_remote(&["git@github.com:other/x.git".to_string()], &live_abs).await.unwrap(), None,
            "a non-matching remote finds nothing");
        assert_eq!(s.find_live_root_by_remote(&[], &live_abs).await.unwrap(), None,
            "no remote to match on → None (a remote-less folder can't be remapped)");
        assert_eq!(s.find_live_root_by_remote(&[url.to_string()], &["/_test/not-live".to_string()]).await.unwrap(), None,
            "the matching folder is not in the live set → not a remap target");
    }

    #[tokio::test]
    async fn folder_has_sessions_reflects_attached_history() {
        let s = pg_store().await;
        let sess = "_test-hasshist-session";
        clear_test_session(&s, sess).await;
        let fid = create_test_folder(&s, "hashist").await;
        assert!(!s.folder_has_sessions(&fid).await.unwrap(), "no sessions yet");
        s.record_session_event(sess, &fid, None, "claude", true).await.unwrap();
        assert!(s.folder_has_sessions(&fid).await.unwrap(), "session attached → has history");
    }

    #[tokio::test]
    async fn remap_folder_moves_sessions_aliases_old_path_and_drops_old_row() {
        let s = pg_store().await;
        let sess = "_test-remap-session";
        clear_test_session(&s, sess).await;
        let old = create_test_folder(&s, "remap-old").await; // /_test/remap-old
        let new = create_test_folder(&s, "remap-new").await; // /_test/remap-new
        s.record_session_event(sess, &old, None, "claude", true).await.unwrap();

        s.remap_folder(&old, "/_test/remap-old", &new).await.unwrap();

        assert_eq!(session_row_folder(&s, sess).await, Some(new), "history moved onto the new folder");
        assert_eq!(s.find_folder_for_path("/_test/remap-old").await.unwrap().map(|(f, _)| f), Some(new),
            "the old path now aliases forward to the new folder");
        let old_still_there: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as("SELECT id FROM sensei.folders WHERE id = $1")
            .bind(old).fetch_optional(&s.pool).await.unwrap();
        assert!(old_still_there.is_none(), "the old husk row is dropped");
    }

    #[tokio::test]
    async fn archive_folder_sets_archived_status() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "to-archive").await;
        s.archive_folder(&fid).await.unwrap();
        let (status,): (String,) = sqlx_core::query_as::query_as("SELECT status::text FROM sensei.folders WHERE id = $1")
            .bind(fid).fetch_one(&s.pool).await.unwrap();
        assert_eq!(status, "archived", "the vanished history-bearing root is retained as archived");
    }

    #[tokio::test]
    async fn update_folder_remotes_populates_and_is_matchable() {
        let s = pg_store().await;
        let url = "git@github.com:sensei-hq/populate-probe.git";
        let fid = create_test_folder(&s, "populate-remotes").await; // /_test/populate-remotes
        s.update_folder_remotes(&fid, &serde_json::json!([{"name":"origin","url":url}])).await.unwrap();
        // Round-trips into remote_urls AND is now findable as a live-root remote match.
        assert_eq!(
            s.find_live_root_by_remote(&[url.to_string()], &["/_test/populate-remotes".to_string()]).await.unwrap(),
            Some(fid),
            "the written remote is what makes auto-remap able to fire"
        );
    }

    #[tokio::test]
    async fn replace_library_capabilities_is_manifest_authoritative() {
        use crate::libraries::manifest::{ProvidedAgent, ProvidedSkill};
        let s = pg_store().await;
        let lib = format!("_testlib_{}", uuid::Uuid::new_v4());
        let lid = s.upsert_library(&lib, "npm", Some(">=1.0"), None, None, None).await.unwrap();
        let sk = |n: &str, f: &str| ProvidedSkill { name: n.into(), focus: f.into(), path: Some(format!("p/{n}.md")), body: Some(format!("# {n}")) };
        let ag = ProvidedAgent { name: "rev".into(), focus: "review".into(), path: Some("a/rev.md".into()), body: Some("# rev".into()) };

        let (ns, na) = s.replace_library_capabilities(&lid, "manifest", Some(">=1.0"), &[sk("styling", "styling"), sk("a11y", "accessibility")], &[ag]).await.unwrap();
        assert_eq!((ns, na), (2, 1));
        assert_eq!(s.list_library_skills(&lib).await.unwrap().len(), 2);
        assert_eq!(s.list_library_agents(&lib).await.unwrap().len(), 1);
        assert!(s.get_library_skill(&lib, "styling").await.unwrap().is_some(), "focus lookup finds it");
        assert!(s.get_library_skill(&lib, "nope").await.unwrap().is_none(), "genuine miss → None (not an error)");

        // Re-ingest a manifest that now declares only 1 skill → the removed one disappears.
        let (ns2, _) = s.replace_library_capabilities(&lid, "manifest", Some(">=1.0"), &[sk("styling", "styling")], &[]).await.unwrap();
        assert_eq!(ns2, 1);
        assert_eq!(s.list_library_skills(&lib).await.unwrap().len(), 1, "the dropped skill is gone (manifest-authoritative)");
        assert_eq!(s.list_library_agents(&lib).await.unwrap().len(), 0);

        // A path/body-less entry is not persisted (no fabricated body).
        let bodyless = ProvidedSkill { name: "x".into(), focus: "x".into(), path: None, body: None };
        let (ns3, _) = s.replace_library_capabilities(&lid, "manifest", None, &[sk("styling", "styling"), bodyless], &[]).await.unwrap();
        assert_eq!(ns3, 1, "the body-less entry is skipped");
    }

    #[tokio::test]
    async fn list_project_library_capabilities_suggests_from_a_projects_deps() {
        use crate::libraries::manifest::ProvidedSkill;
        let s = pg_store().await;
        let pid = s.create_project(&format!("_libcap_{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let lib = format!("_libcapdep_{}", uuid::Uuid::new_v4());
        let lid = s.upsert_library(&lib, "npm", Some("1"), None, None, None).await.unwrap();
        s.replace_library_capabilities(&lid, "manifest", Some("1"),
            &[ProvidedSkill { name: "semantic-styles-rokkit".into(), focus: "styling".into(), path: Some("p".into()), body: Some("b".into()) }],
            &[]).await.unwrap();
        // The project depends on the library.
        s.execute_raw(&format!(
            "INSERT INTO sensei.project_libraries(library_id, project_id, enabled) VALUES('{lid}','{pid}',true) ON CONFLICT DO NOTHING"
        )).await.unwrap();

        let caps = s.list_project_library_capabilities(&pid).await.unwrap();
        let skills = caps["suggested_skills"].as_array().unwrap();
        assert!(skills.iter().any(|x| x["name"] == "semantic-styles-rokkit" && x["library"] == lib.as_str()),
            "the project's dependency contributes its skill: {caps:?}");
    }

    #[tokio::test]
    async fn folder_id_by_abs_path_is_exact_and_never_follows_aliases() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "exact-path").await; // /_test/exact-path
        s.add_folder_path_alias("/_test/exact-old", &fid, "rename").await.unwrap();
        assert_eq!(s.folder_id_by_abs_path("/_test/exact-path").await.unwrap(), Some(fid),
            "exact abs_path resolves to the folder");
        assert_eq!(s.folder_id_by_abs_path("/_test/exact-old").await.unwrap(), None,
            "an aliased path is NOT a real folder row — exact lookup returns None");
        assert_eq!(s.folder_id_by_abs_path("/_test/never").await.unwrap(), None);
    }

    #[tokio::test]
    async fn folder_path_alias_resolves_old_paths_after_a_rename() {
        // A renamed repo: the folder now lives at the new abs_path, and its OLD
        // path is registered as an alias. Transcripts/hooks recorded under the old
        // path (and its subdirs) must still resolve to the folder + project.
        let s = pg_store().await;
        let fid = create_test_folder(&s, "alias-new").await; // abs_path /_test/alias-new
        let old = "/_test/alias-old";
        s.add_folder_path_alias(old, &fid, "rename").await.unwrap();
        // exact-match resolver (transcript synthesis) resolves the old path via alias.
        assert_eq!(s.get_folder_ids_by_path(old).await.unwrap().map(|(id, _)| id), Some(fid),
            "old exact path resolves via alias");
        // ancestor resolver (hooks / synth fallback) resolves an old SUBDIR via alias.
        assert_eq!(s.find_folder_for_path("/_test/alias-old/docs/mockups").await.unwrap()
            .map(|(id, _)| id), Some(fid), "old subdir resolves to the folder via the alias ancestor");
        // the current path still resolves (live abs_path unaffected).
        assert_eq!(s.get_folder_ids_by_path("/_test/alias-new").await.unwrap().map(|(id, _)| id),
            Some(fid), "current path still resolves");
        // idempotent re-register.
        s.add_folder_path_alias(old, &fid, "detected").await.unwrap();
        assert_eq!(s.get_folder_ids_by_path(old).await.unwrap().map(|(id, _)| id), Some(fid));
    }

    #[tokio::test]
    async fn repo_root_for_path_resolves_nearest_git_ancestor_skipping_members() {
        // The watcher resolver: a change under a repo → the repo ROOT (git/
        // standalone), skipping structural workspace_member subdirs so the
        // one-owner repo wins.
        let s = pg_store().await;
        let uniq = uuid::Uuid::new_v4();
        let root = format!("/_test/reporoot/{uniq}");
        let root_id = s.add_watch_root(&root, "rr", &serde_json::json!([])).await.unwrap();
        let repo = format!("{root}/mono");
        let repo_fid = s.upsert_repo_kind(&root_id, "git", "mono", &repo).await.unwrap();
        // A workspace member subdir (not an index owner) — must be skipped.
        let member = format!("{repo}/packages/chart");
        s.upsert_subfolder(&root_id, "chart", "mono/packages/chart", &member, Some(&repo_fid), None).await.ok();

        let got = s.repo_root_for_path(&format!("{member}/src/x.ts")).await.unwrap();
        assert_eq!(got.map(|(p, _)| p), Some(repo.clone()), "deep file resolves to the git repo root, not the member subdir");
        assert_eq!(s.repo_root_for_path(&repo).await.unwrap().map(|(p, _)| p), Some(repo), "exact repo path resolves too");
        assert_eq!(s.repo_root_for_path(&format!("/_test/nope-{uniq}/x")).await.unwrap(), None, "path under no repo → None");

        let pool = s.pool();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id=$1").bind(root_id).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id=$1").bind(root_id).execute(pool).await.ok();
    }

    #[tokio::test]
    async fn scope_repo_roots_returns_repo_roots_not_structural_subfolders() {
        // Content grep walks repo ROOTS (git/subtree/standalone), never the
        // structural `folder` subdirs the index also tracks.
        let s = pg_store().await;
        let uniq = uuid::Uuid::new_v4();
        let root = format!("/_test/scoperoots/{uniq}");
        let root_id = s.add_watch_root(&root, "sr", &serde_json::json!([])).await.unwrap();
        let repo = format!("{root}/app");
        let repo_fid = s.upsert_repo_kind(&root_id, "git", "app", &repo).await.unwrap();
        // A structural subfolder (kind='folder') under the repo — NOT a repo root.
        let comp = format!("{repo}/src/lib");
        let comp_fid = s
            .upsert_subfolder(&root_id, "lib", "app/src/lib", &comp, Some(&repo_fid), None)
            .await
            .unwrap();

        let roots = s.scope_repo_roots(&[repo_fid, comp_fid]).await.unwrap();
        assert!(roots.contains(&repo), "the git repo root is returned");
        assert!(!roots.contains(&comp), "a structural (kind='folder') subdir is not a repo root");
        assert!(s.scope_repo_roots(&[]).await.unwrap().is_empty(), "empty scope → no roots");

        let pool = s.pool();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id=$1").bind(root_id).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id=$1").bind(root_id).execute(pool).await.ok();
    }

    #[tokio::test]
    async fn record_symbol_names_is_monotonic_history() {
        // The symbol-history registry backing doc-drift: a current symbol is
        // recorded, and a REMOVED symbol stays recorded (monotonic) so a later
        // scan can still tell it was once real (→ its stale doc refs are drift).
        let s = pg_store().await;
        let fid = create_test_folder(&s, &format!("symhist_{}", uuid::Uuid::new_v4())).await;
        let uniq = format!("SymHist_{}", uuid::Uuid::new_v4().simple());

        let nid = s
            .upsert_node(&fid, "function", &uniq, "x.rs", None, None, Some(1), Some(2))
            .await
            .unwrap();
        s.record_symbol_names().await.unwrap();
        let present: Option<(String,)> =
            sqlx_core::query_as::query_as("SELECT name FROM sensei.symbol_names WHERE name = $1")
                .bind(&uniq)
                .fetch_optional(s.pool())
                .await
                .unwrap();
        assert!(present.is_some(), "a current symbol name is recorded");

        // Remove the symbol and re-record — the name must persist.
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE id = $1").bind(nid).execute(s.pool()).await.unwrap();
        s.record_symbol_names().await.unwrap();
        let still: Option<(String,)> =
            sqlx_core::query_as::query_as("SELECT name FROM sensei.symbol_names WHERE name = $1")
                .bind(&uniq)
                .fetch_optional(s.pool())
                .await
                .unwrap();
        assert!(still.is_some(), "a removed symbol stays in the registry (monotonic history)");

        sqlx_core::query::query("DELETE FROM sensei.symbol_names WHERE name = $1").bind(&uniq).execute(s.pool()).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1").bind(fid).execute(s.pool()).await.ok();
    }

    #[tokio::test]
    async fn get_project_commands_marks_and_ranks_the_preferred_tool() {
        // G10: when several commands share a category, the user's dojo_preference
        // marks one preferred and ranks it first.
        let s = pg_store().await;
        let pid = s.create_project(&format!("_test:g10-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let fid = create_test_folder(&s, &format!("g10_{}", uuid::Uuid::new_v4())).await;
        s.set_folder_project(&fid, &pid, "primary", None).await.unwrap();
        // Two `test` commands; alphabetical order is jest, then vitest.
        sqlx_core::query::query(
            "INSERT INTO sensei.project_commands (folder_id, raw_name, command_line, category, ecosystem)
             VALUES ($1, 'jest', 'jest', 'test', 'npm'), ($1, 'vitest', 'vitest run', 'test', 'npm')",
        ).bind(fid).execute(s.pool()).await.unwrap();
        // Clean slate for the shared preference row.
        sqlx_core::query::query("DELETE FROM sensei.dojo_preferences WHERE scope='user' AND capability='test'")
            .execute(s.pool()).await.ok();

        // No preference → alphabetical, nothing marked preferred.
        let before = s.get_project_commands(&pid, Some("test")).await.unwrap();
        assert_eq!(before.len(), 2);
        assert_eq!(before[0]["raw_name"], "jest");
        assert!(before.iter().all(|c| c["preferred"] == serde_json::json!(false)), "no preference → none preferred");

        // Prefer vitest → it is marked and ranked first (ahead of the alphabetically-first jest).
        s.upsert_command_preference("user", "test", "vitest", None).await.unwrap();
        let after = s.get_project_commands(&pid, Some("test")).await.unwrap();
        assert_eq!(after[0]["raw_name"], "vitest", "preferred ranked first");
        assert_eq!(after[0]["preferred"], serde_json::json!(true));
        assert_eq!(after[1]["raw_name"], "jest");
        assert_eq!(after[1]["preferred"], serde_json::json!(false), "the non-preferred sibling isn't marked");

        let pool = s.pool();
        sqlx_core::query::query("DELETE FROM sensei.project_commands WHERE folder_id=$1").bind(fid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.dojo_preferences WHERE scope='user' AND capability='test'").execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id=$1").bind(fid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id=$1").bind(pid).execute(pool).await.ok();
    }

    #[tokio::test]
    async fn record_session_event_folds_into_one_row_and_completes() {
        // #31: every hook event of a session folds into one row keyed by the
        // assistant session id; Stop/SessionEnd marks it completed.
        let s = pg_store().await;
        let fid = create_test_folder(&s, "sess-record").await;
        let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
        let id1 = s.record_session_event(&sid, &fid, None, "claude", false).await.unwrap();
        let id2 = s.record_session_event(&sid, &fid, None, "claude", false).await.unwrap();
        assert_eq!(id1, id2, "same client_session_id must fold into one session row");
        assert!(s.get_session(&id1).await.unwrap().unwrap()["completed_at"].is_null(),
            "not completed before an end event");
        let id3 = s.record_session_event(&sid, &fid, None, "claude", true).await.unwrap();
        assert_eq!(id3, id1, "end event updates the same row");
        assert!(!s.get_session(&id1).await.unwrap().unwrap()["completed_at"].is_null(),
            "Stop/SessionEnd sets completed_at");
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
            .bind(id1).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn list_all_sessions_joins_project_and_uses_camelcase_times() {
        // #61: the observatory reads project name + startedAt/completedAt. The
        // returned row must carry the joined project NAME (not a bare folder
        // uuid) under camelCase timestamp keys, with completedAt set once the
        // session ends — otherwise every displayed column renders blank.
        let s = pg_store().await;
        let proj_name = format!("_test:obs-{}", uuid::Uuid::new_v4());
        let pid = s.create_project(&proj_name, None, None).await.unwrap();
        let fid = create_test_folder(&s, "obs-sess").await;
        let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
        let session_id = s.record_session_event(&sid, &fid, Some(&pid), "claude", false).await.unwrap();
        s.record_session_event(&sid, &fid, Some(&pid), "claude", true).await.unwrap();

        let all = s.list_all_sessions(500, None, None).await.unwrap();
        let row = all.iter()
            .find(|r| r["id"].as_str() == Some(session_id.to_string().as_str()))
            .expect("our session is listed");

        assert_eq!(row["project"], serde_json::json!(proj_name), "project name is joined, not a folder uuid");
        assert!(row["startedAt"].as_str().is_some(), "startedAt present (camelCase)");
        assert!(row.get("started_at").is_none(), "no stale snake_case started_at key");
        assert!(row["completedAt"].as_str().is_some(), "completedAt set after the end event");
        assert!(row.get("folder_id").is_none(), "folder_id no longer leaks in place of the project");

        sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
            .bind(session_id).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(pid).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn set_session_summary_if_empty_writes_then_preserves() {
        // The retrospective producer fills an empty summary, but must never
        // clobber one that already exists (assistant checkpoint summaries).
        let s = pg_store().await;
        let pid = s.create_project(&format!("_test:sum-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let fid = create_test_folder(&s, &format!("sum-{}", uuid::Uuid::new_v4())).await;
        let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
        let session_id = s.record_session_event(&sid, &fid, Some(&pid), "claude", true).await.unwrap();

        // Fresh session → summary NULL → the guarded write persists.
        s.set_session_summary_if_empty(&session_id, "touched 2 files; outcome completed").await.unwrap();
        let first: (Option<String>,) = sqlx_core::query_as::query_as("SELECT summary FROM activity.sessions WHERE id = $1")
            .bind(session_id).fetch_one(s.pool()).await.unwrap();
        assert_eq!(first.0.as_deref(), Some("touched 2 files; outcome completed"));

        // Second write with a populated summary must be a no-op (not clobbered).
        s.set_session_summary_if_empty(&session_id, "a different summary").await.unwrap();
        let second: (Option<String>,) = sqlx_core::query_as::query_as("SELECT summary FROM activity.sessions WHERE id = $1")
            .bind(session_id).fetch_one(s.pool()).await.unwrap();
        assert_eq!(second.0.as_deref(), Some("touched 2 files; outcome completed"), "populated summary preserved");

        sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1").bind(session_id).execute(s.pool()).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(s.pool()).await.ok();
    }

    #[tokio::test]
    async fn get_project_repos_excludes_subfolder_tree() {
        // #62: a single-repo project with subfolders must list only its repo
        // root(s), never the kind='folder' subfolder tree — else the UI shows it
        // as a multi-repo project with every folder as a repo.
        let s = pg_store().await;
        let pid = s.create_project(&format!("_test:repos-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        s.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) ON CONFLICT DO NOTHING"
        ).await.unwrap();
        let git_abs = format!("/_test/repos-git-{}", uuid::Uuid::new_v4());
        let sub_abs = format!("/_test/repos-sub-{}", uuid::Uuid::new_v4());
        let mem_abs = format!("/_test/repos-mem-{}", uuid::Uuid::new_v4());
        sqlx_core::query::query(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path, project_id) VALUES
               ('00000000-0000-0000-0000-000000000001','git'::sensei.folder_kind,'the-repo','the-repo',$1,$3),
               ('00000000-0000-0000-0000-000000000001','folder'::sensei.folder_kind,'subdir','subdir',$2,$3),
               ('00000000-0000-0000-0000-000000000001','workspace_member'::sensei.folder_kind,'member','member',$4,$3)"
        ).bind(&git_abs).bind(&sub_abs).bind(pid).bind(&mem_abs).execute(s.pool()).await.unwrap();

        let repos = s.get_project_repos(&pid).await.unwrap();
        let kinds: Vec<String> = repos.iter().filter_map(|r| r["kind"].as_str().map(str::to_string)).collect();
        assert!(kinds.iter().any(|k| k == "git"), "the repo root is listed: {kinds:?}");
        assert!(!kinds.iter().any(|k| k == "folder"), "kind=folder subfolders excluded: {kinds:?}");
        // D5a: monorepo members are the structural tree, NOT separate repos — else
        // a monorepo with N members regresses to an N+1-repo project (#62).
        assert!(!kinds.iter().any(|k| k == "workspace_member"), "kind=workspace_member excluded from repos: {kinds:?}");

        sqlx_core::query::query("DELETE FROM sensei.folders WHERE project_id = $1").bind(pid).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn upsert_subfolder_kind_relabels_structural_but_preserves_root() {
        // D5a: upsert_subfolder_kind relabels between the two STRUCTURAL kinds
        // (folder ↔ workspace_member) on conflict, but NEVER reclassifies a path
        // that is actually a nested project ROOT (git/standalone/subtree).
        let s = pg_store().await;
        s.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) ON CONFLICT DO NOTHING"
        ).await.unwrap();
        let rid = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let kind_at = |s: &PgStore, abs: String| {
            let pool = s.pool().clone();
            async move {
                let (k,): (String,) = query_as("SELECT kind::text FROM sensei.folders WHERE abs_path=$1")
                    .bind(&abs).fetch_one(&pool).await.unwrap();
                k
            }
        };

        // A plain structural folder → relabel to workspace_member on re-upsert.
        let a = format!("/_test/sfk-a-{}", uuid::Uuid::new_v4());
        s.upsert_subfolder(&rid, "a", "a", &a, None, None).await.unwrap();
        assert_eq!(kind_at(&s, a.clone()).await, "folder", "first upsert is a plain folder");
        s.upsert_subfolder_kind(&rid, "workspace_member", "a", "a", &a, None, None).await.unwrap();
        assert_eq!(kind_at(&s, a.clone()).await, "workspace_member", "relabelled folder → workspace_member");

        // A nested project root (subtree) must NOT be reclassified by a member upsert.
        let b = format!("/_test/sfk-b-{}", uuid::Uuid::new_v4());
        s.upsert_repo_kind(&rid, "subtree", "b", &b).await.unwrap();
        s.upsert_subfolder_kind(&rid, "workspace_member", "b", "b", &b, None, None).await.unwrap();
        assert_eq!(kind_at(&s, b.clone()).await, "subtree", "a nested root is preserved, never reclassified");

        sqlx_core::query::query("DELETE FROM sensei.folders WHERE abs_path IN ($1,$2)")
            .bind(&a).bind(&b).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn projects_with_session_activity_reports_the_project() {
        // #67: the scheduler reads (project_id, latest activity) to decide what
        // to re-analyze. A project with attributed sessions must appear.
        let s = pg_store().await;
        let pid = s.create_project(&format!("_test:act-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let fid = create_test_folder(&s, &format!("act-{}", uuid::Uuid::new_v4())).await;
        let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
        s.record_session_event(&sid, &fid, Some(&pid), "claude", true).await.unwrap();

        let activity = s.get_projects_with_session_activity().await.unwrap();
        let row = activity.iter().find(|(p, _)| *p == pid).expect("project appears in session-activity");
        assert!(row.1.timestamp() > 0, "carries a real latest-activity timestamp");

        sqlx_core::query::query("DELETE FROM activity.sessions WHERE project_id = $1").bind(pid).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn project_ftr_and_quality_decode_numeric_metrics() {
        // Regression: the headline (Σ props / value) and the daily AVG(...) trend /
        // ftr_7d / avg_duration_ms are all NUMERIC; without ::float8 casts sqlx
        // fails to decode into f64 and the endpoint 500s (masked by the client's
        // default-on-error). The project must have BOTH a stored `ftr` row (so the
        // headline decodes a real number, not a short-circuiting NULL) AND an
        // analyzed session in the window (so the inline trend decodes a numeric row).
        let s = pg_store().await;
        let pid = s.create_project(&format!("_test:ftr-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let fid = create_test_folder(&s, &format!("ftr-{}", uuid::Uuid::new_v4())).await;
        let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
        let session_id = s.record_session_event(&sid, &fid, Some(&pid), "claude", true).await.unwrap();
        s.update_session_metrics(&session_id, 3, 0, "completed", true, 1000, None, &serde_json::json!({})).await.unwrap();
        // Stored daily ftr row in the 14d window → the headline decodes a real value.
        let (ftr_mid,): (uuid::Uuid,) =
            query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'").fetch_one(s.pool()).await.unwrap();
        s.upsert_project_metric(&ftr_mid, &pid, None, None, chrono::Utc::now().date_naive(), "daily", 1.0,
            &serde_json::json!({"numerator": 1, "denominator": 1}), "measured").await.unwrap();

        let ftr = s.get_project_ftr(&pid).await.expect("get_project_ftr decodes numeric metrics");
        assert!(ftr["ftr14d"].as_f64().is_some(), "ftr14d decodes a real number from the stored row");
        assert!(ftr["ftrTrend"].as_array().is_some_and(|a| !a.is_empty()), "daily trend decodes a numeric row");
        s.get_quality_signals(&pid).await.expect("get_quality_signals decodes numeric metrics");
        s.get_tool_usage_stats().await.expect("get_tool_usage_stats decodes numeric avg_duration_ms");

        sqlx_core::query::query("DELETE FROM activity.sessions WHERE project_id = $1").bind(pid).execute(s.pool()).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(s.pool()).await.ok();
    }

    #[tokio::test]
    async fn library_upsert_updates() {
        let s = pg_store().await;
        let id1 = s.upsert_library("_test:react", "npm", Some("18"), None, None, None).await.unwrap();
        let id2 = s.upsert_library("_test:react", "npm", Some("19"), Some("UI library"), None, None).await.unwrap();
        assert_eq!(id1, id2);
        let lib = s.get_library(&id1).await.unwrap().unwrap();
        assert_eq!(lib["version"], "19");
        assert_eq!(lib["description"], "UI library");
        s.delete_library(&id1).await.unwrap();
    }

    #[tokio::test]
    async fn library_list() {
        let s = pg_store().await;
        let id1 = s.upsert_library("_test:lib_a", "npm", None, None, None, None).await.unwrap();
        let id2 = s.upsert_library("_test:lib_b", "cargo", None, None, None, None).await.unwrap();
        let all = s.list_libraries().await.unwrap();
        assert!(all.iter().any(|l| l["name"] == "_test:lib_a"));
        assert!(all.iter().any(|l| l["name"] == "_test:lib_b"));
        s.delete_library(&id1).await.unwrap();
        s.delete_library(&id2).await.unwrap();
    }

    #[tokio::test]
    async fn library_delete() {
        let s = pg_store().await;
        let id = s.upsert_library("_test:deleteme", "npm", None, None, None, None).await.unwrap();
        s.delete_library(&id).await.unwrap();
        assert!(s.get_library(&id).await.unwrap().is_none());
    }

    // ── Sessions + Events tests ────────────────────────────────────────

    #[tokio::test]
    async fn session_create_and_get() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "sess_create").await;
        let sid = s.create_session(&fid, "fix bug #42", Some("claude-code")).await.unwrap();
        let sess = s.get_session(&sid).await.unwrap().unwrap();
        assert_eq!(sess["task"], "fix bug #42");
        assert_eq!(sess["acp_id"], "claude-code");
        assert!(sess["outcome"].is_null());
        assert_eq!(sess["turns"], 0);
    }

    #[tokio::test]
    async fn session_complete() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "sess_complete").await;
        let sid = s.create_session(&fid, "add feature", None).await.unwrap();
        s.complete_session(&sid, "completed", true, 5, 0, Some("shipped it"), Some(1200), Some(3400)).await.unwrap();
        let sess = s.get_session(&sid).await.unwrap().unwrap();
        assert_eq!(sess["outcome"], "completed");
        assert_eq!(sess["ftr"], true);
        assert_eq!(sess["turns"], 5);
        assert!(sess["completed_at"].as_str().is_some());
        // summary + tokens actually PERSIST (were previously advertised-but-dropped).
        let meta: (Option<String>, Option<i32>, Option<i32>) = sqlx_core::query_as::query_as(
            "SELECT summary, tokens_in, tokens_out FROM activity.sessions WHERE id=$1")
            .bind(sid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(meta.0.as_deref(), Some("shipped it"), "summary persists");
        assert_eq!(meta.1, Some(1200), "tokens_in persists");
        assert_eq!(meta.2, Some(3400), "tokens_out persists");
    }

    #[tokio::test]
    async fn session_list_by_folder() {
        let s = pg_store().await;
        let suffix = format!("sess_list_{}", uuid::Uuid::new_v4());
        let fid = create_test_folder(&s, &suffix).await;
        s.create_session(&fid, "task 1", None).await.unwrap();
        s.create_session(&fid, "task 2", None).await.unwrap();
        let sessions = s.list_sessions_by_folder(&fid, 10).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn session_get_nonexistent() {
        let s = pg_store().await;
        assert!(s.get_session(&uuid::Uuid::new_v4()).await.unwrap().is_none());
    }

    // ── Hook events tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn hook_event_insert_and_query() {
        let s = pg_store().await;
        let session_id = format!("test-session-{}", uuid::Uuid::new_v4());
        let payload = serde_json::json!({
            "session_id": session_id,
            "hook_event_name": "PreToolUse",
            "assistant_family": "claude",
            "tool_name": "Read",
            "cwd": "/tmp/test",
        });
        let id = s.insert_hook_event(
            &session_id, "claude", "PreToolUse", Some("Read"), Some("/tmp/test"),
            chrono::Utc::now().timestamp_millis(), None, &payload,
        ).await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn hook_event_post_tool_use_success() {
        let s = pg_store().await;
        let session_id = format!("test-session-{}", uuid::Uuid::new_v4());
        let payload = serde_json::json!({"hook_event_name": "PostToolUse", "assistant_family": "claude", "tool_name": "Bash"});
        let id = s.insert_hook_event(
            &session_id, "claude", "PostToolUse", Some("Bash"), None,
            chrono::Utc::now().timestamp_millis(), Some(true), &payload,
        ).await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn hook_event_no_tool_name() {
        let s = pg_store().await;
        let session_id = format!("test-session-{}", uuid::Uuid::new_v4());
        let payload = serde_json::json!({"hook_event_name": "SessionStart", "assistant_family": "claude", "model": "claude-sonnet-4"});
        let id = s.insert_hook_event(
            &session_id, "claude", "SessionStart", None, Some("/home/user/project"),
            chrono::Utc::now().timestamp_millis(), None, &payload,
        ).await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn hook_event_cursor_family() {
        let s = pg_store().await;
        let session_id = format!("cursor-session-{}", uuid::Uuid::new_v4());
        let payload = serde_json::json!({"hook_event_name": "SessionStart", "assistant_family": "cursor"});
        let id = s.insert_hook_event(
            &session_id, "cursor", "SessionStart", None, Some("/home/user/project"),
            chrono::Utc::now().timestamp_millis(), None, &payload,
        ).await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn unclassified_verdict_sessions_returns_only_in_window_unclassified() {
        use crate::tasks::handlers::tool_insights::HEALTH_VERDICT_WINDOW_DAYS;
        let s = pg_store().await;
        let now = chrono::Utc::now().timestamp_millis();
        let day_ms = 86_400_000i64;

        // (a) in-window PostToolUse, never classified → should appear.
        let pending_sid = format!("_test-unclassified-pending-{}", uuid::Uuid::new_v4());
        s.insert_hook_event(&pending_sid, "claude", "PostToolUse", Some("Read"), None,
            now, Some(true), &serde_json::json!({"tool_response": "x"})).await.unwrap();

        // (b) in-window PostToolUse that already carries a verdict row → excluded.
        let classified_sid = format!("_test-unclassified-classified-{}", uuid::Uuid::new_v4());
        let ev_id = s.insert_hook_event(&classified_sid, "claude", "PostToolUse", Some("Read"), None,
            now, Some(true), &serde_json::json!({"tool_response": "y"})).await.unwrap();
        s.upsert_verdicts_batch(&[(
            classified_sid.clone(), ev_id, Some("Read".to_string()), "used", 0.9f32, "seed".to_string(),
        )]).await.unwrap();

        // (c) out-of-window PostToolUse (30 days old), unclassified → excluded.
        let old_sid = format!("_test-unclassified-old-{}", uuid::Uuid::new_v4());
        s.insert_hook_event(&old_sid, "claude", "PostToolUse", Some("Read"), None,
            now - 30 * day_ms, Some(true), &serde_json::json!({"tool_response": "z"})).await.unwrap();

        let pending = s.unclassified_verdict_sessions(HEALTH_VERDICT_WINDOW_DAYS).await.unwrap();
        assert!(pending.contains(&pending_sid), "in-window unclassified session is pending");
        assert!(!pending.contains(&classified_sid), "already-classified session excluded");
        assert!(!pending.contains(&old_sid), "out-of-window session excluded");
    }

    // ── Projects tests ────────────────────────────────────────────────

    #[tokio::test]
    async fn project_create_and_get() {
        let s = pg_store().await;
        let id = s.create_project("_test:proj:create", Some("desc"), Some("client")).await.unwrap();
        let p = s.get_project(&id).await.unwrap().unwrap();
        assert_eq!(p["name"], "_test:proj:create");
        assert_eq!(p["description"], "desc");
        assert_eq!(p["client"], "client");
        assert_eq!(p["maturity"], "discovery"); // default
        s.delete_project(&id).await.unwrap();
    }

    #[tokio::test]
    async fn project_list() {
        let s = pg_store().await;
        let id1 = s.create_project("_test:proj:list_a", None, None).await.unwrap();
        let id2 = s.create_project("_test:proj:list_b", None, None).await.unwrap();
        let all = s.list_projects().await.unwrap();
        let names: Vec<&str> = all.iter().filter_map(|p| p["name"].as_str()).collect();
        assert!(names.contains(&"_test:proj:list_a"));
        assert!(names.contains(&"_test:proj:list_b"));
        s.delete_project(&id1).await.unwrap();
        s.delete_project(&id2).await.unwrap();
    }

    #[tokio::test]
    async fn list_projects_under_filters_by_folder_path_boundary() {
        // find_projects (MCP) needs a folder-scoped view: only projects whose
        // folders live under a given path. This pins the SQL boundary rule —
        // exact match + child match, but never a sibling that merely shares the
        // textual prefix (`/x` must not catch `/x-other`).
        let s = pg_store().await;
        let short = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
        let base = format!("/tmp/_test-fpu-{short}");
        let under = format!("{base}/x");
        let root = s.add_watch_root(&base, &format!("fpu-{short}"), &serde_json::json!([]))
            .await.unwrap();

        // A: folder strictly beneath `under`.
        let a = s.ensure_test_project(&format!("fpu-a-{short}")).await.unwrap();
        s.upsert_folder(&root, "git", "a", "x/a", &format!("{under}/a"), None, Some(&a)).await.unwrap();
        // B: folder exactly equal to `under` (boundary: abs_path == under).
        let b = s.ensure_test_project(&format!("fpu-b-{short}")).await.unwrap();
        s.upsert_folder(&root, "git", "b", "x", &under, None, Some(&b)).await.unwrap();
        // C: folder elsewhere under base but outside `under`.
        let c = s.ensure_test_project(&format!("fpu-c-{short}")).await.unwrap();
        s.upsert_folder(&root, "git", "c", "elsewhere", &format!("{base}/elsewhere"), None, Some(&c)).await.unwrap();
        // D: sibling sharing the `under` prefix textually but across a path boundary.
        let d = s.ensure_test_project(&format!("fpu-d-{short}")).await.unwrap();
        s.upsert_folder(&root, "git", "d", "x-other", &format!("{under}-other/z"), None, Some(&d)).await.unwrap();

        let scoped: Vec<String> = s.list_projects_under(Some(&under)).await.unwrap()
            .iter().filter_map(|p| p["id"].as_str().map(str::to_string)).collect();
        let has = |id: &uuid::Uuid| scoped.contains(&id.to_string());
        assert!(has(&a), "folder strictly under `under` must match");
        assert!(has(&b), "folder equal to `under` must match (boundary)");
        assert!(!has(&c), "folder outside `under` must NOT match");
        assert!(!has(&d), "sibling `{under}-other` must NOT match (path boundary, not raw prefix)");

        // No-filter returns everything — all four present.
        let all: Vec<String> = s.list_projects_under(None).await.unwrap()
            .iter().filter_map(|p| p["id"].as_str().map(str::to_string)).collect();
        for id in [&a, &b, &c, &d] {
            assert!(all.contains(&id.to_string()), "no-filter list must include every project");
        }
        // list_projects() (public no-arg) is equivalent to the None filter.
        let plain: Vec<String> = s.list_projects().await.unwrap()
            .iter().filter_map(|p| p["id"].as_str().map(str::to_string)).collect();
        assert!(plain.contains(&a.to_string()) && plain.contains(&c.to_string()),
            "list_projects() must stay unfiltered");

        for id in [a, b, c, d] { s.delete_project(&id).await.unwrap(); }
    }

    #[tokio::test]
    async fn list_root_folders_excludes_nested_folder_descendants() {
        // find_projects (`?under=`) must return the COMPACT folder set — repo
        // roots only. The hundreds of nested `kind:'folder'` descendants are the
        // MCP token-cap bloat; list_root_folders_by_project drops them while
        // list_folders_by_project (app path) keeps the whole tree.
        let s = pg_store().await;
        let short = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
        let base = format!("/tmp/_test-rootf-{short}");
        let root = s.add_watch_root(&base, &format!("rootf-{short}"), &serde_json::json!([]))
            .await.unwrap();
        let p = s.ensure_test_project(&format!("rootf-{short}")).await.unwrap();

        // One git repo root …
        s.upsert_folder(&root, "git", "repo", "repo", &format!("{base}/repo"), None, Some(&p))
            .await.unwrap();
        // … plus one standalone root …
        s.upsert_folder(&root, "standalone", "lib", "lib", &format!("{base}/lib"), None, Some(&p))
            .await.unwrap();
        // … plus many nested `kind:'folder'` descendants (the bloat).
        for i in 0..30 {
            s.upsert_folder(
                &root, "folder", &format!("d{i}"), &format!("repo/src/d{i}"),
                &format!("{base}/repo/src/d{i}"), None, Some(&p),
            ).await.unwrap();
        }

        let all = s.list_folders_by_project(&p).await.unwrap();
        assert_eq!(all.len(), 32, "full list keeps roots + all descendants");

        let roots = s.list_root_folders_by_project(&p).await.unwrap();
        assert_eq!(roots.len(), 2, "root list is repo roots only");
        assert!(
            roots.iter().all(|f| matches!(f["kind"].as_str(), Some("git") | Some("standalone"))),
            "root list must contain no `kind:'folder'` descendants",
        );
        // The repo root's abs_path (what cwd→project resolution needs) survives.
        assert!(roots.iter().any(|f| f["abs_path"] == format!("{base}/repo")));

        s.delete_project(&p).await.unwrap();
    }

    #[tokio::test]
    async fn project_update() {
        let s = pg_store().await;
        let id = s.create_project("_test:proj:update", None, None).await.unwrap();
        s.update_project(&id, &ProjectPatch {
            name: Some("renamed"),
            maturity: Some("active"),
            ..Default::default()
        }).await.unwrap();
        let p = s.get_project(&id).await.unwrap().unwrap();
        assert_eq!(p["name"], "renamed");
        assert_eq!(p["maturity"], "active");
        s.delete_project(&id).await.unwrap();
    }

    #[tokio::test]
    async fn project_update_persists_widened_identity_fields() {
        // The About-edit form PUTs goal/icon/stack/links/client/preferred_acp
        // alongside name/maturity; all must round-trip through update_project.
        let s = pg_store().await;
        let id = s.create_project("_test:proj:widen", None, None).await.unwrap();
        let icon = serde_json::json!({"kind":"kanji","value":"識","bg":"var(--shu-soft)","fg":"var(--shu)"});
        let stack = serde_json::json!({"languages":["rust"],"frameworks":["axum"]});
        let links = serde_json::json!([{"id":"1","kind":"docs","label":"Docs","url":"https://example.com"}]);
        s.update_project(&id, &ProjectPatch {
            goal: Some("teach sensei"),
            client: Some("acme"),
            preferred_acp: Some("zed"),
            maturity: Some("active"),
            icon: Some(&icon),
            stack: Some(&stack),
            links: Some(&links),
            ..Default::default()
        }).await.unwrap();

        let p = s.get_project(&id).await.unwrap().unwrap();
        assert_eq!(p["goal"], "teach sensei");
        assert_eq!(p["client"], "acme");
        assert_eq!(p["maturity"], "active");
        assert_eq!(p["icon"], icon, "icon jsonb must persist verbatim");
        assert_eq!(p["stack"], stack, "stack jsonb must persist verbatim");
        assert_eq!(p["links"], links, "links jsonb must persist verbatim");

        // preferred_acp isn't in get_project's projection — read it directly.
        let (acp,): (Option<String>,) = query_as(
            "SELECT preferred_acp FROM sensei.projects WHERE id = $1"
        ).bind(id).fetch_one(s.pool()).await.unwrap();
        assert_eq!(acp.as_deref(), Some("zed"), "preferred_acp text must persist");

        s.delete_project(&id).await.unwrap();
    }

    #[tokio::test]
    async fn set_project_icon_round_trips() {
        // The inferred-icon write path persists the jsonb verbatim, overwriting
        // the '{}' default the row was created with.
        let s = pg_store().await;
        let id = s.create_project("_test:proj:icon", None, None).await.unwrap();
        let icon = serde_json::json!({"kind":"kanji","value":"鉄","source":"kanji_map"});
        s.set_project_icon(&id, &icon).await.unwrap();
        let p = s.get_project(&id).await.unwrap().unwrap();
        assert_eq!(p["icon"], icon, "inferred icon jsonb must persist verbatim");
        s.delete_project(&id).await.unwrap();
    }

    #[tokio::test]
    async fn project_update_rejects_unknown_maturity() {
        // maturity is the sensei.project_maturity enum — an unknown value must
        // be rejected (Err → 400 at the HTTP layer), never a raw cast 500, and
        // the row must be left untouched.
        let s = pg_store().await;
        let id = s.create_project("_test:proj:badmaturity", None, None).await.unwrap();
        let res = s.update_project(&id, &ProjectPatch {
            maturity: Some("spike"),
            ..Default::default()
        }).await;
        assert!(res.is_err(), "unknown maturity must be rejected");
        let p = s.get_project(&id).await.unwrap().unwrap();
        assert_eq!(p["maturity"], "discovery", "rejected update must not mutate the row");
        s.delete_project(&id).await.unwrap();
    }

    #[tokio::test]
    async fn project_update_omitted_fields_unchanged() {
        // Partial-update (COALESCE) semantics: a patch that only sets `name`
        // must leave goal/client/description/maturity exactly as they were.
        let s = pg_store().await;
        let id = s.create_project("_test:proj:partial", Some("orig desc"), Some("orig client")).await.unwrap();
        s.update_project(&id, &ProjectPatch {
            goal: Some("g1"),
            maturity: Some("active"),
            ..Default::default()
        }).await.unwrap();
        s.update_project(&id, &ProjectPatch {
            name: Some("renamed2"),
            ..Default::default()
        }).await.unwrap();

        let p = s.get_project(&id).await.unwrap().unwrap();
        assert_eq!(p["name"], "renamed2");
        assert_eq!(p["goal"], "g1", "omitted goal must be unchanged");
        assert_eq!(p["client"], "orig client", "omitted client must be unchanged");
        assert_eq!(p["description"], "orig desc", "omitted description must be unchanged");
        assert_eq!(p["maturity"], "active", "omitted maturity must be unchanged");
        s.delete_project(&id).await.unwrap();
    }

    #[tokio::test]
    async fn project_delete() {
        let s = pg_store().await;
        let id = s.create_project("_test:proj:delete", None, None).await.unwrap();
        s.delete_project(&id).await.unwrap();
        assert!(s.get_project(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn project_get_nonexistent() {
        let s = pg_store().await;
        let fake = uuid::Uuid::new_v4();
        assert!(s.get_project(&fake).await.unwrap().is_none());
    }

    // ── Name-duplicate phantom-project guard (creation + heal) ─────────

    /// Guard A: the scan-time creation path is get-or-adopt, not
    /// select-then-insert — a repeat call for the same name ADOPTS the existing
    /// row instead of minting a second (the mechanism that produced the 0-folder
    /// phantom).
    #[tokio::test]
    async fn get_or_create_project_by_name_is_idempotent() {
        let Ok(s) = PgStore::connect_test().await else { return; };
        let name = format!("_test:dupname:{}", uuid::Uuid::new_v4());

        let (id1, created1) = s.get_or_create_project_by_name(&name).await.unwrap();
        assert!(created1, "first call should mint the project");
        let (id2, created2) = s.get_or_create_project_by_name(&name).await.unwrap();
        assert!(!created2, "second call should adopt, not create");
        assert_eq!(id1, id2, "same name must resolve to the same project id");

        let (count,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.projects WHERE name = $1"
        ).bind(&name).fetch_one(s.pool()).await.unwrap();
        assert_eq!(count, 1, "exactly one project row for the name");

        s.delete_project(&id1).await.ok();
    }

    /// Guard A: when a folder-bearing project of the name already exists, the
    /// creation path adopts THAT one (not a fresh row) — no second "sensei".
    #[tokio::test]
    async fn get_or_create_adopts_folder_bearing_project_no_duplicate() {
        let Ok(s) = PgStore::connect_test().await else { return; };
        let name = format!("_test:dupname:{}", uuid::Uuid::new_v4());

        let keep = s.create_project(&name, None, None).await.unwrap();
        let fid = create_test_folder(&s, &format!("dupname-{}", uuid::Uuid::new_v4())).await;
        sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
            .bind(keep).bind(fid).execute(s.pool()).await.unwrap();

        let (resolved, created) = s.get_or_create_project_by_name(&name).await.unwrap();
        assert!(!created, "should adopt the existing folder-bearing project");
        assert_eq!(resolved, keep, "must resolve to the folder-bearing project");

        let (count,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.projects WHERE name = $1"
        ).bind(&name).fetch_one(s.pool()).await.unwrap();
        assert_eq!(count, 1, "exactly one project row for the name");

        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1").bind(fid).execute(s.pool()).await.ok();
        s.delete_project(&keep).await.ok();
    }

    /// Guard B: a 0-folder discovery phantom sharing its name with a
    /// folder-bearing project is pruned (merged into the survivor); its FK rows
    /// (here a session) are reassigned, never orphaned; a re-run is a no-op.
    #[tokio::test]
    async fn heal_duplicate_name_projects_prunes_empty_dupe_idempotently() {
        let Ok(s) = PgStore::connect_test().await else { return; };
        let name = format!("_test:dupheal:{}", uuid::Uuid::new_v4());

        // Survivor: folder-bearing project.
        let keep = s.create_project(&name, None, None).await.unwrap();
        let fid = create_test_folder(&s, &format!("dupheal-{}", uuid::Uuid::new_v4())).await;
        sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
            .bind(keep).bind(fid).execute(s.pool()).await.unwrap();

        // Phantom: second same-name project, 0 folders, maturity=discovery (default),
        // carrying a session so we can prove the heal reassigns FK rows.
        let phantom = s.create_project(&name, None, None).await.unwrap();
        let (sess,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.sessions(folder_id, project_id, task) VALUES($1, $2, $3) RETURNING id"
        ).bind(fid).bind(phantom).bind("_test:dupheal-session").fetch_one(s.pool()).await.unwrap();

        s.heal_duplicate_name_projects().await.unwrap();

        // Phantom gone, survivor stays.
        let (phantom_exists,): (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM sensei.projects WHERE id = $1)"
        ).bind(phantom).fetch_one(s.pool()).await.unwrap();
        assert!(!phantom_exists, "empty phantom should be pruned");
        assert!(s.get_project(&keep).await.unwrap().is_some(), "folder-bearing survivor must remain");

        // Survivor still owns its folder; the phantom's session followed it.
        let (folder_project,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT project_id FROM sensei.folders WHERE id = $1"
        ).bind(fid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(folder_project, Some(keep), "survivor keeps its folder");
        let (sess_project,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT project_id FROM activity.sessions WHERE id = $1"
        ).bind(sess).fetch_one(s.pool()).await.unwrap();
        assert_eq!(sess_project, Some(keep), "phantom's session must be reassigned to the survivor, not orphaned");

        // Idempotent: after the heal exactly one project remains for the name and
        // a re-run leaves it untouched.
        s.heal_duplicate_name_projects().await.unwrap();
        let (count,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.projects WHERE name = $1"
        ).bind(&name).fetch_one(s.pool()).await.unwrap();
        assert_eq!(count, 1, "exactly one project remains for the name; re-run is a no-op");

        sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1").bind(sess).execute(s.pool()).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1").bind(fid).execute(s.pool()).await.ok();
        s.delete_project(&keep).await.ok();
    }

    /// Guard B negative: two DIFFERENT repos (different paths) that share a name
    /// are BOTH folder-bearing — legitimately distinct projects (identity =
    /// path, not name) — and must NOT be merged.
    #[tokio::test]
    async fn heal_leaves_two_folder_bearing_same_name_projects() {
        let Ok(s) = PgStore::connect_test().await else { return; };
        let name = format!("_test:dupneg:{}", uuid::Uuid::new_v4());

        let a = s.create_project(&name, None, None).await.unwrap();
        let fa = create_test_folder(&s, &format!("dupneg-a-{}", uuid::Uuid::new_v4())).await;
        sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
            .bind(a).bind(fa).execute(s.pool()).await.unwrap();

        let b = s.create_project(&name, None, None).await.unwrap();
        let fb = create_test_folder(&s, &format!("dupneg-b-{}", uuid::Uuid::new_v4())).await;
        sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
            .bind(b).bind(fb).execute(s.pool()).await.unwrap();

        s.heal_duplicate_name_projects().await.unwrap();

        assert!(s.get_project(&a).await.unwrap().is_some(), "first folder-bearing project must survive");
        assert!(s.get_project(&b).await.unwrap().is_some(), "second folder-bearing project must survive");
        let (count,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.projects WHERE name = $1"
        ).bind(&name).fetch_one(s.pool()).await.unwrap();
        assert_eq!(count, 2, "two folder-bearing same-name projects must both survive");

        for (p, f) in [(a, fa), (b, fb)] {
            sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1").bind(f).execute(s.pool()).await.ok();
            s.delete_project(&p).await.ok();
        }
    }

    // ── Index Errors tests ───────────────────────────────────────────

    #[tokio::test]
    async fn idx_err_log_and_get() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "idx_err_log").await;
        s.clear_index_errors(&fid).await.unwrap(); // ensure clean
        s.log_index_error(&fid, "src/bad.ts", "SyntaxError", Some("typescript"), None).await.unwrap();
        s.log_index_error(&fid, "src/x.py", "IndentError", Some("python"), Some("parse")).await.unwrap();
        let errors = s.get_index_errors(Some(&fid)).await.unwrap();
        assert_eq!(errors.len(), 2);
        s.clear_index_errors(&fid).await.unwrap();
    }

    #[tokio::test]
    async fn idx_err_clear() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "idx_err_clear").await;
        s.clear_index_errors(&fid).await.unwrap();
        s.log_index_error(&fid, "a.rs", "err", Some("rust"), None).await.unwrap();
        s.clear_index_errors(&fid).await.unwrap();
        assert_eq!(s.get_index_errors(Some(&fid)).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn idx_err_empty() {
        let s = pg_store().await;
        let fid = create_test_folder(&s, "idx_err_empty").await;
        s.clear_index_errors(&fid).await.unwrap();
        assert_eq!(s.get_index_errors(Some(&fid)).await.unwrap().len(), 0);
    }

    // ── Workflow State tests ────────────────────────────────────────────

    #[tokio::test]
    async fn wf_upsert_and_get() {
        let s = pg_store().await;
        let p = "_test:wf:upsert";
        s.delete_workflow_state(p).await.unwrap();
        assert!(s.get_workflow_state(p).await.unwrap().is_none());
        s.upsert_workflow_state(p, Some("ideate"), None, None, None, None, None).await.unwrap();
        let state = s.get_workflow_state(p).await.unwrap().unwrap();
        assert_eq!(state["active_phase"], "ideate");
        assert!(state["active_task"].is_null());
        s.delete_workflow_state(p).await.unwrap();
    }

    #[tokio::test]
    async fn wf_partial_update_preserves() {
        let s = pg_store().await;
        let p = "_test:wf:partial";
        s.delete_workflow_state(p).await.unwrap();
        s.upsert_workflow_state(p, Some("build"), Some("plan.md"), Some("task 1"), Some(42), None, Some("hash123")).await.unwrap();
        s.upsert_workflow_state(p, Some("validate"), None, None, None, None, None).await.unwrap();
        let state = s.get_workflow_state(p).await.unwrap().unwrap();
        assert_eq!(state["active_phase"], "validate");
        assert_eq!(state["active_plan"], "plan.md");
        assert_eq!(state["active_task"], "task 1");
        assert_eq!(state["active_issue"], 42);
        s.delete_workflow_state(p).await.unwrap();
    }

    #[tokio::test]
    async fn wf_nonexistent_returns_none() {
        let s = pg_store().await;
        assert!(s.get_workflow_state("_test:wf:none").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn wf_delete() {
        let s = pg_store().await;
        let p = "_test:wf:delete";
        s.upsert_workflow_state(p, Some("ideate"), None, None, None, None, None).await.unwrap();
        s.delete_workflow_state(p).await.unwrap();
        assert!(s.get_workflow_state(p).await.unwrap().is_none());
    }

    // ── Tags tests ────────────────────────────────────────────────────

    #[tokio::test]
    async fn tag_add_and_list() {
        let s = pg_store().await;
        let tag = "_test:tag_add:rust";
        s.add_tag(tag, Some("stack")).await.unwrap();
        let tags = s.list_tags().await.unwrap();
        assert!(tags.iter().any(|(t, c)| t == tag && c.as_deref() == Some("stack")));
        s.remove_tag(tag).await.unwrap();
    }

    #[tokio::test]
    async fn tag_add_without_category() {
        let s = pg_store().await;
        let tag = "_test:tag_nocat:misc";
        s.add_tag(tag, None).await.unwrap();
        let tags = s.list_tags().await.unwrap();
        assert!(tags.iter().any(|(t, c)| t == tag && c.is_none()));
        s.remove_tag(tag).await.unwrap();
    }

    #[tokio::test]
    async fn tag_add_duplicate_is_upsert() {
        let s = pg_store().await;
        let tag = "_test:tag_dup:ts";
        s.add_tag(tag, Some("stack")).await.unwrap();
        s.add_tag(tag, Some("language")).await.unwrap(); // update category
        let tags = s.list_tags().await.unwrap();
        let found: Vec<_> = tags.iter().filter(|(t, _)| t == tag).collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1.as_deref(), Some("language"));
        s.remove_tag(tag).await.unwrap();
    }

    #[tokio::test]
    async fn tag_remove() {
        let s = pg_store().await;
        let tag = "_test:tag_rm:go";
        s.add_tag(tag, Some("stack")).await.unwrap();
        s.remove_tag(tag).await.unwrap();
        let tags = s.list_tags().await.unwrap();
        assert!(!tags.iter().any(|(t, _)| t == tag));
    }

    #[tokio::test]
    async fn tag_remove_nonexistent_is_noop() {
        let s = pg_store().await;
        s.remove_tag("_test:tag_rm_noop:xyz").await.unwrap();
    }

    #[tokio::test]
    async fn tag_list_by_category() {
        let s = pg_store().await;
        let t1 = "_test:tag_cat:rust";
        let t2 = "_test:tag_cat:ts";
        let t3 = "_test:tag_cat:active";
        s.add_tag(t1, Some("stack")).await.unwrap();
        s.add_tag(t2, Some("stack")).await.unwrap();
        s.add_tag(t3, Some("status")).await.unwrap();
        let stack_tags = s.list_tags_by_category("stack").await.unwrap();
        assert!(stack_tags.contains(&t1.to_string()));
        assert!(stack_tags.contains(&t2.to_string()));
        assert!(!stack_tags.contains(&t3.to_string()));
        s.remove_tag(t1).await.unwrap();
        s.remove_tag(t2).await.unwrap();
        s.remove_tag(t3).await.unwrap();
    }

    // ── Schema tests ─────────────────────────────────────────────────

    #[tokio::test]
    async fn memories_table_exists() {
        let store = PgStore::connect(&test_db_url()).await.unwrap();
        let row: (bool,) = query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = 'sensei' AND table_name = 'memories')"
        )
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert!(row.0, "sensei.memories table must exist — run `dbd apply` first");
    }

    // ── Knowledge Sources tests ───────────────────────────────────────

    #[tokio::test]
    async fn knowledge_source_crud_roundtrip() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let id = pg.create_knowledge_source(&NewKnowledgeSource {
            kind: "hive_mind".into(), name: "Org Dōjō".into(), url: "https://dojo.example".into(),
            namespace_id: None, credential_ref: "dojo-test".into(), direction: "both".into(),
        }).await.unwrap();

        let all = pg.list_knowledge_sources().await.unwrap();
        assert!(all.iter().any(|s| s.id == id && s.last_seq == 0 && s.enabled));

        pg.set_source_cursor(&id, 42).await.unwrap();
        let one = pg.get_knowledge_source(&id).await.unwrap().unwrap();
        assert_eq!(one.last_seq, 42);
        assert_eq!(one.direction, "both");

        assert!(pg.delete_knowledge_source(&id).await.unwrap());
        assert!(pg.get_knowledge_source(&id).await.unwrap().is_none());
    }

    // ── Dōjō connections tests ────────────────────────────────────────

    #[tokio::test]
    async fn dojo_membership_crud_and_project_binding_roundtrip() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        // Service-assigned membership id (the local PK; projects.dojo_id → this).
        let mid = uuid::Uuid::new_v4();
        pg.create_dojo_membership(&NewDojoMembership {
            id: mid,
            registry_url: "http://localhost:7755".into(),
            tenant_key: "github/acme".into(),
            dojo_url: "http://localhost:7755/github/acme".into(),
            kind: "client".into(),
            org_slugs: vec!["acme".into(), "acme-labs".into()],
            role: "contributor".into(),
            authenticated_via: "device_code".into(),
            attribution_default: "anonymous".into(),
            credential_ref: format!("dojo-{}", uuid::Uuid::new_v4()),
            sync_status: "authenticating".into(),
        }).await.unwrap();

        // Present in the list with sane defaults + the org_slugs roundtrip.
        let all = pg.list_dojo_memberships().await.unwrap();
        let row = all.iter().find(|m| m.id == mid).expect("membership listed");
        assert_eq!(row.kind, "client");
        assert_eq!(row.org_slugs, vec!["acme".to_string(), "acme-labs".to_string()]);
        assert_eq!(row.last_seq, 0);
        assert!(row.enabled);
        assert!(row.last_heartbeat_at.is_none());

        // org-tagging edit: replace the covered org slugs.
        assert!(pg.set_dojo_membership_orgs(&mid, &["acme".into(), "acme-corp".into()]).await.unwrap());
        assert_eq!(
            pg.get_dojo_membership(&mid).await.unwrap().unwrap().org_slugs,
            vec!["acme".to_string(), "acme-corp".to_string()]
        );
        assert!(!pg.set_dojo_membership_orgs(&uuid::Uuid::new_v4(), &[]).await.unwrap(), "unknown id → false");

        // sync-status update.
        assert!(pg.set_dojo_sync_status(&mid, "healthy").await.unwrap());
        assert_eq!(pg.get_dojo_membership(&mid).await.unwrap().unwrap().sync_status, "healthy");

        // Bind a project → projects.dojo_id → appears in the bound-projects strip.
        let proj = pg.create_project("_test:dojo:bind", None, None).await.unwrap();
        assert!(pg.bind_project_to_dojo(&proj, Some(&mid)).await.unwrap());
        let bound = pg.projects_bound_to_dojo(&mid).await.unwrap();
        assert!(bound.iter().any(|(id, _)| *id == proj), "bound project surfaces");

        // Unbind + cleanup.
        assert!(pg.bind_project_to_dojo(&proj, None).await.unwrap());
        assert!(pg.projects_bound_to_dojo(&mid).await.unwrap().is_empty());
        pg.delete_project(&proj).await.unwrap();

        assert!(pg.delete_dojo_membership(&mid).await.unwrap());
        assert!(pg.get_dojo_membership(&mid).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn collective_preferences_defaults_and_upsert_roundtrip() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        use crate::collective::preferences::{self, CollectivePreferences};
        let _guard = preferences::test_lock().lock().await;

        // Clean slate — the singleton row may linger from a prior run.
        sqlx_core::query::query("DELETE FROM sensei.collective_preferences")
            .execute(pg.pool()).await.unwrap();

        // Defaults-when-empty: no row → conservative defaults, updated_at None.
        assert!(pg.get_collective_preferences().await.unwrap().is_none());
        let defaults = preferences::get(&pg).await.unwrap();
        assert_eq!(defaults.destination, "none");
        assert_eq!(defaults.cadence, "manual");
        assert_eq!(defaults.attribution_default, "anonymous");
        assert_eq!(defaults.updated_at, None);

        // Upsert a validated body, then read it back.
        let body = serde_json::json!({
            "destination": "both", "cadence": "daily", "attribution_default": "named",
            "categories": { "memory": false, "guard": false }
        });
        let saved = preferences::set(&pg, CollectivePreferences::from_request(&body).unwrap())
            .await.unwrap();
        assert!(saved.updated_at.is_some(), "upsert assigns updated_at");

        let got = preferences::get(&pg).await.unwrap();
        assert_eq!(got.destination, "both");
        assert_eq!(got.cadence, "daily");
        assert_eq!(got.attribution_default, "named");
        assert_eq!(got.categories.get("memory"), Some(&false));
        assert_eq!(got.categories.get("guard"), Some(&false));
        assert_eq!(got.categories.get("pattern"), Some(&true));
        assert!(got.updated_at.is_some());

        // Re-upsert → still exactly one row (singleton), values fully replaced.
        let body2 = serde_json::json!({ "destination": "global", "cadence": "weekly" });
        preferences::set(&pg, CollectivePreferences::from_request(&body2).unwrap())
            .await.unwrap();
        let (n,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.collective_preferences")
            .fetch_one(pg.pool()).await.unwrap();
        assert_eq!(n, 1, "singleton table holds exactly one row after re-upsert");
        let got2 = preferences::get(&pg).await.unwrap();
        assert_eq!(got2.destination, "global");
        assert_eq!(got2.categories.get("memory"), Some(&true), "full replace resets toggles to default");

        // Cleanup.
        sqlx_core::query::query("DELETE FROM sensei.collective_preferences")
            .execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn upsert_stance_default_and_scoped_roundtrip() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        // Unique key so parallel tests / lingering rows never collide.
        let user = format!("stance-upsert-{}@test.local", uuid::Uuid::new_v4());

        // 1. Default row (namespace_id NULL): insert, then re-resolve returns it
        //    as the "default" source.
        let ua = pg.upsert_stance(&user, None, "run_freely", "private", "quorum").await.unwrap();
        assert!(!ua.is_empty(), "upsert returns an updated_at");
        let r = pg.resolve_stance(&user, None).await.unwrap();
        assert_eq!((r.autonomy.as_str(), r.sharing.as_str(), r.review.as_str(), r.source.as_str()),
                   ("run_freely", "private", "quorum", "default"));

        // 2. Re-upsert the default (same partial-index conflict target): updates in
        //    place, no duplicate default row.
        pg.upsert_stance(&user, None, "ask_always", "patterns", "me_alone").await.unwrap();
        let r = pg.resolve_stance(&user, None).await.unwrap();
        assert_eq!((r.autonomy.as_str(), r.review.as_str()), ("ask_always", "me_alone"), "default row updated");
        let (n,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.stances WHERE user_key = $1 AND namespace_id IS NULL")
            .bind(&user).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(n, 1, "exactly one default row after re-upsert");

        // 3. Scoped row: seed a throwaway namespace, upsert against it (composite
        //    conflict target), read it back, then re-upsert to prove update. The
        //    `project` scope may be absent in an unseeded test DB — seed it idempotently.
        sqlx_core::query::query(
            "INSERT INTO sensei.scopes (key, name, level) VALUES ('project', 'Project', 60)
             ON CONFLICT (key) DO NOTHING")
            .execute(pg.pool()).await.unwrap();
        let ns = uuid::Uuid::new_v4();
        sqlx_core::query::query(
            "INSERT INTO sensei.namespaces (id, scope_key, name, slug, level)
             VALUES ($1, 'project', 'stance-test', $2, 60)")
            .bind(ns).bind(format!("stance-test-{ns}"))
            .execute(pg.pool()).await.unwrap();

        pg.upsert_stance(&user, Some(&ns), "ask_on_risky", "derived", "two_maintainers").await.unwrap();
        let (au, sh, rv): (String, String, String) = sqlx_core::query_as::query_as(
            "SELECT autonomy::text, sharing::text, review::text FROM sensei.stances
             WHERE user_key = $1 AND namespace_id = $2")
            .bind(&user).bind(ns).fetch_one(pg.pool()).await.unwrap();
        assert_eq!((au.as_str(), sh.as_str(), rv.as_str()), ("ask_on_risky", "derived", "two_maintainers"));

        pg.upsert_stance(&user, Some(&ns), "run_freely", "patterns", "quorum").await.unwrap();
        let (au2,): (String,) = sqlx_core::query_as::query_as(
            "SELECT autonomy::text FROM sensei.stances WHERE user_key = $1 AND namespace_id = $2")
            .bind(&user).bind(ns).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(au2, "run_freely", "scoped row updated via composite conflict target");
        let (n,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.stances WHERE user_key = $1")
            .bind(&user).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(n, 2, "one default + one scoped row for the user");

        // Cleanup (stances cascade off the namespace delete).
        sqlx_core::query::query("DELETE FROM sensei.stances WHERE user_key = $1")
            .bind(&user).execute(pg.pool()).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE id = $1")
            .bind(ns).execute(pg.pool()).await.ok();
    }

    #[tokio::test]
    async fn dojo_outbox_and_batch_items_roundtrip() {
        let Ok(pg) = PgStore::connect_test().await else { return; };

        // A project + a memory to share + an APPROVED batch containing it.
        let proj = pg.create_project("_test:dojo:outbox", None, None).await.unwrap();
        let mem = pg.insert_memory(&InsertMemory {
            project_id: Some(proj), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "prefer migration tools".into(),
            content: "Use a dedicated migration tool over hand-rolled SQL.".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: Some("learned".into()), source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();
        let batch = pg.create_memory_share_batch(&proj, &[mem], None).await.unwrap();
        pg.set_memory_share_batch_status(&batch, "approved", None).await.unwrap();

        // batch_share_items: approved batch, one member, body = content.
        let (bp, status, items) = pg.batch_share_items(&batch).await.unwrap().expect("batch loads");
        assert_eq!(bp, proj);
        assert_eq!(status, "approved");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].memory_id, mem);
        assert!(items[0].body.contains("migration tool"));
        assert_eq!(items[0].memory_type, "convention");

        // An unbound project → no routing anchor.
        assert!(pg.project_bound_membership(&proj).await.unwrap().is_none());

        // While the member has no `sent` outbox row, an unsent approved batch exists.
        assert!(pg.next_unsent_approved_batch().await.unwrap().is_some());

        // A destination membership + the outbox dedup ledger.
        let mid = uuid::Uuid::new_v4();
        pg.create_dojo_membership(&NewDojoMembership {
            id: mid, registry_url: "http://localhost:7755".into(), tenant_key: "github/acme".into(),
            dojo_url: "http://localhost:7755/github/acme".into(), kind: "client".into(),
            org_slugs: vec![],
            role: "contributor".into(), authenticated_via: "device_code".into(),
            attribution_default: "anonymous".into(),
            credential_ref: format!("dojo-{}", uuid::Uuid::new_v4()), sync_status: "healthy".into(),
        }).await.unwrap();

        assert!(!pg.outbox_already_sent(&mid, "sig-1").await.unwrap());
        pg.outbox_mark_sent(&mid, Some(&batch), Some(&mem), "sig-1", 5, "remote-1").await.unwrap();
        assert!(pg.outbox_already_sent(&mid, "sig-1").await.unwrap());
        // A different signature is independent.
        assert!(!pg.outbox_already_sent(&mid, "sig-2").await.unwrap());
        // A late held/queued signal must NOT downgrade an already-sent row.
        pg.outbox_mark_state(&mid, Some(&batch), Some(&mem), "sig-1", "queued").await.unwrap();
        assert!(pg.outbox_already_sent(&mid, "sig-1").await.unwrap(), "sent row must survive a late queued mark");
        // A held record for a fresh signature.
        pg.outbox_mark_state(&mid, Some(&batch), Some(&mem), "sig-3", "held").await.unwrap();
        assert!(!pg.outbox_already_sent(&mid, "sig-3").await.unwrap());

        // Cleanup: membership delete cascades its outbox rows; then batch/memory/project.
        assert!(pg.delete_dojo_membership(&mid).await.unwrap());
        pg.delete_project(&proj).await.unwrap();
    }

    #[tokio::test]
    async fn dojo_inbox_upsert_apply_and_state_roundtrip() {
        let Ok(pg) = PgStore::connect_test().await else { return; };

        // A source membership (inbox rows cascade with it on delete).
        let mid = uuid::Uuid::new_v4();
        pg.create_dojo_membership(&NewDojoMembership {
            id: mid, registry_url: "http://localhost:7755".into(), tenant_key: "github/acme".into(),
            dojo_url: "http://localhost:7755/github/acme".into(), kind: "community".into(),
            org_slugs: vec![],
            role: "contributor".into(), authenticated_via: "device_code".into(),
            attribution_default: "anonymous".into(),
            credential_ref: format!("dojo-{}", uuid::Uuid::new_v4()), sync_status: "healthy".into(),
        }).await.unwrap();

        let attribution = dojo_protocol::Attribution {
            mode: dojo_protocol::AttributionMode::Anonymous,
            author: None, org: None, anonymous_id: Some("anon-1".into()),
        };
        let row = |sig: &str, title: &str| crate::collective::inbox::InboxRow {
            membership_id: mid, artifact_seq: 3, signature: sig.into(), remote_id: "art-x".into(),
            kind: "principle".into(), title: title.into(), body: "keep units testable".into(),
            scope: dojo_protocol::ArtifactScope::default(), attribution: attribution.clone(),
        };

        // Upsert is idempotent by (membership, signature): first inserts, re-pull skips.
        assert!(pg.upsert_dojo_inbox(&row("inbox-sig-1", "prefer small fns")).await.unwrap());
        assert!(!pg.upsert_dojo_inbox(&row("inbox-sig-1", "prefer small fns")).await.unwrap(), "re-pull dedups");
        pg.upsert_dojo_inbox(&row("inbox-sig-2", "write tests first")).await.unwrap();

        let items = pg.list_dojo_inbox(true).await.unwrap();
        let item1 = items.iter().find(|i| i.artifact_signature == "inbox-sig-1").expect("row 1 present").clone();
        let id2 = items.iter().find(|i| i.artifact_signature == "inbox-sig-2").expect("row 2 present").id;
        assert_eq!(item1.kind, "principle");
        assert_eq!(item1.state, "pending");
        assert_eq!(item1.attribution.anonymous_id.as_deref(), Some("anon-1"));

        // resolve_project_by_name — the scope-match lookup.
        let proj = pg.create_project("_test:dojo:inbox", None, None).await.unwrap();
        assert_eq!(pg.resolve_project_by_name("_test:dojo:inbox".into()).await.unwrap(), Some(proj));
        assert!(pg.resolve_project_by_name(format!("nope-{}", uuid::Uuid::new_v4())).await.unwrap().is_none());

        // Land item1 as a global origin='dojo' memory; the row flips to applied.
        let mem_input = InsertMemory {
            project_id: None, scope: "global".into(), scope_filter: None, mtype: "convention".into(),
            title: item1.title.clone(), content: item1.body.clone(), impact: None,
            tags: vec!["dojo".into()], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: Some("recommended".into()), origin: Some("dojo".into()), source_id: None,
            spine_slot: None, feature: None,
        };
        let memory_id = pg.land_dojo_inbox_memory(item1.id, &mem_input).await.unwrap();
        let (origin,): (String,) = sqlx_core::query_as::query_as(
            "SELECT origin FROM sensei.memories WHERE id = $1").bind(memory_id).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(origin, "dojo");
        let applied = pg.get_dojo_inbox(item1.id).await.unwrap().unwrap();
        assert_eq!(applied.state, "applied");
        assert_eq!(applied.applied_memory_id, Some(memory_id));

        // mute hides from the default list; pin floats to the top; unknown → false.
        assert!(pg.set_dojo_inbox_state(id2, "muted").await.unwrap());
        assert!(pg.list_dojo_inbox(false).await.unwrap().iter().all(|i| i.id != id2), "muted hidden by default");
        assert!(pg.list_dojo_inbox(true).await.unwrap().iter().any(|i| i.id == id2), "include_muted surfaces it");
        assert!(pg.set_dojo_inbox_state(id2, "pinned").await.unwrap());
        assert_eq!(pg.list_dojo_inbox(false).await.unwrap()[0].id, id2, "pinned floats to the top");
        assert!(!pg.set_dojo_inbox_state(uuid::Uuid::new_v4(), "muted").await.unwrap(), "unknown id → false");

        // Cursor advance lives on the membership's last_seq.
        pg.set_dojo_pull_cursor(mid, 42).await.unwrap();
        assert_eq!(pg.get_dojo_membership(&mid).await.unwrap().unwrap().last_seq, 42);

        // Cleanup: membership delete cascades inbox rows; then memory + project.
        assert!(pg.delete_dojo_membership(&mid).await.unwrap());
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(memory_id).execute(pg.pool()).await.unwrap();
        pg.delete_project(&proj).await.unwrap();
    }

    // ── scope_folder_ids tests (#60) ─────────────────────────────────

    /// Build an isolated project + root folder + child subfolder for scope tests.
    async fn setup_scope_test(s: &PgStore, suffix: &str) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid) {
        let proj_name = format!("_test:scope:{}", suffix);
        let proj_id = s.create_project(&proj_name, None, None).await.unwrap();

        // Root folder: upsert into folders_to_watch first (foreign-key for root_id).
        let watch_path = format!("/_test/scope_{}", suffix);
        let watch_id = s.add_watch_root(&watch_path, &format!("scope_root_{}", suffix), &serde_json::json!([])).await.unwrap();

        // Root repo folder (kind='git', owns root_id = watch_id).
        let root_abs = format!("/_test/scope_{}/root", suffix);
        let root_name = format!("scope_root_{}", suffix);
        let root_id = s.upsert_repo(&watch_id, &root_name, &root_abs).await.unwrap();
        s.set_folder_project(&root_id, &proj_id, "main", None).await.unwrap();

        // Child subfolder (kind='folder', parent = root, project = proj_id).
        let child_abs = format!("/_test/scope_{}/root/child", suffix);
        let child_name = format!("scope_child_{}", suffix);
        let child_id = s.upsert_subfolder(&watch_id, &child_name, &child_name, &child_abs, Some(&root_id), Some(&proj_id)).await.unwrap();

        (proj_id, root_id, child_id)
    }

    #[tokio::test]
    async fn scope_folder_ids_by_project_name_returns_all_folders() {
        let s = pg_store().await;
        let uid = uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
        let (proj_id, root_id, child_id) = setup_scope_test(&s, &uid).await;
        let proj_name = format!("_test:scope:{}", uid);

        let ids = s.scope_folder_ids(&proj_name).await.unwrap();
        assert!(ids.contains(&root_id), "root folder must be in scope ids; got {:?}", ids);
        assert!(ids.contains(&child_id), "child folder must be in scope ids; got {:?}", ids);

        // Also test by UUID string.
        let by_uuid = s.scope_folder_ids(&proj_id.to_string()).await.unwrap();
        assert!(by_uuid.contains(&child_id), "UUID lookup must find child; got {:?}", by_uuid);

        // Nonexistent ident returns empty.
        let empty = s.scope_folder_ids("nonexistent-xyz-scope-test-noop").await.unwrap();
        assert!(empty.is_empty(), "nonexistent must be empty; got {:?}", empty);

        // Cleanup.
        s.delete_nodes_by_folder(&root_id).await.unwrap();
        s.delete_nodes_by_folder(&child_id).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = ANY($1::uuid[])")
            .bind(vec![child_id, root_id]).execute(s.pool()).await.unwrap();
        s.delete_project(&proj_id).await.unwrap();
    }

    // ── project-scoped query variants tests (#60) ─────────────────────

    #[tokio::test]
    async fn scoped_search_and_count_across_child_folder() {
        let s = pg_store().await;
        let uid = uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
        let (proj_id, root_id, child_id) = setup_scope_test(&s, &uid).await;
        let proj_name = format!("_test:scope:{}", uid);

        // Insert a function node in the CHILD folder.
        let fn_id = s.upsert_node(&child_id, "function", "widget_builder", "src/widget.rs", None, Some("fn widget_builder()"), Some(1), Some(10)).await.unwrap();
        // Insert a callee node (target) in child folder.
        let tgt_id = s.upsert_node(&child_id, "function", "render_widget", "src/widget.rs", None, Some("fn render_widget()"), Some(12), Some(20)).await.unwrap();
        // Insert resolved edge: widget_builder calls render_widget.
        s.insert_edge(&child_id, &fn_id, Some(&tgt_id), Some("render_widget"), None, "calls").await.unwrap();

        // Resolve scope.
        let ids = s.scope_folder_ids(&proj_name).await.unwrap();
        assert!(!ids.is_empty());

        // search_functions_scoped must find widget_builder.
        let fns = s.search_functions_scoped(&ids, "widget_builder").await.unwrap();
        assert!(
            fns.iter().any(|f| f["name"] == "widget_builder"),
            "expected widget_builder in {:?}", fns
        );

        // count_nodes_by_kind_scoped must report at least 2 functions.
        let counts = s.count_nodes_by_kind_scoped(&ids).await.unwrap();
        let fn_count = counts.get("function").copied().unwrap_or(0);
        assert!(fn_count >= 2, "expected >=2 function nodes, got {:?}", counts);

        // get_nodes_scoped must include child nodes.
        let nodes = s.get_nodes_scoped(&ids).await.unwrap();
        assert!(nodes.iter().any(|n| n["name"] == "widget_builder"), "nodes_scoped missing widget_builder");

        // get_edges_scoped must return the calls edge.
        let edges = s.get_edges_scoped(&ids, "calls").await.unwrap();
        assert!(!edges.is_empty(), "expected >=1 calls edge in scoped result");

        // get_callers_by_name with project name: render_widget is called by widget_builder.
        let callers = s.get_callers_by_name(&proj_name, "render_widget").await.unwrap();
        assert!(
            callers.iter().any(|c| c["name"] == "widget_builder"),
            "expected widget_builder as caller of render_widget; got {:?}", callers
        );

        // get_callees_by_name with project name: widget_builder calls render_widget.
        let callees = s.get_callees_by_name(&proj_name, "widget_builder").await.unwrap();
        assert!(
            callees.iter().any(|c| c["name"] == "render_widget"),
            "expected render_widget as callee of widget_builder; got {:?}", callees
        );

        // Cleanup.
        s.delete_nodes_by_folder(&child_id).await.unwrap(); // cascades edges
        s.delete_nodes_by_folder(&root_id).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = ANY($1::uuid[])")
            .bind(vec![child_id, root_id]).execute(s.pool()).await.unwrap();
        s.delete_project(&proj_id).await.unwrap();
        let _ = (fn_id, tgt_id);
    }

    // ── public.logs read path (Observatory · Logs) ───────────────────

    /// Seed three log rows spanning levels / sources / timestamps and return
    /// a marker (via `running_on`) so the assertions can isolate this run's
    /// rows from anything already in the shared test DB.
    async fn seed_logs(pg: &PgStore, marker: &str) {
        // Oldest → newest so `logged_at DESC` ordering is observable.
        let base = chrono::Utc::now() - chrono::Duration::hours(2);
        let rows = [
            ("info",  format!("{marker}-a"), "scanner",   base),
            ("warn",  format!("{marker}-a"), "watcher",   base + chrono::Duration::minutes(30)),
            ("error", format!("{marker}-b"), "analyzer",  base + chrono::Duration::minutes(90)),
        ];
        for (level, running_on, module, ts) in rows {
            pg.insert_log(
                level,
                &running_on,
                &ts.to_rfc3339(),
                &format!("{marker} {level} message"),
                &serde_json::json!({ "module": module }),
                &None,
                &None,
            ).await.unwrap();
        }
    }

    async fn cleanup_logs(pg: &PgStore, marker: &str) {
        sqlx_core::query::query("DELETE FROM public.logs WHERE running_on LIKE $1")
            .bind(format!("{marker}-%"))
            .execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn query_logs_no_filter_newest_first_and_capped() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let marker = format!("_test:logs:{}", uuid::Uuid::new_v4());
        seed_logs(&pg, &marker).await;

        // Scope to this run via the `source` (running_on) filter is not enough
        // (two distinct sources), so fetch broadly and filter in-memory.
        let all = pg.query_logs(None, None, None, None, 1000).await.unwrap();
        let mine: Vec<_> = all.iter()
            .filter(|r| r["source"].as_str().is_some_and(|s| s.starts_with(&marker)))
            .collect();
        assert_eq!(mine.len(), 3, "all three seeded rows returned");

        // Newest-first: the analyzer/error row (base+90m) precedes the others.
        assert_eq!(mine[0]["level"], "error");
        assert_eq!(mine[2]["level"], "info");

        // Stable wire shape: source mirrors running_on, module lives in context.
        assert_eq!(mine[0]["source"], format!("{marker}-b"));
        assert_eq!(mine[0]["context"]["module"], "analyzer");
        assert!(mine[0]["logged_at"].as_str().unwrap().contains('T'));

        cleanup_logs(&pg, &marker).await;
    }

    #[tokio::test]
    async fn query_logs_level_and_source_and_module_filters() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let marker = format!("_test:logs:{}", uuid::Uuid::new_v4());
        seed_logs(&pg, &marker).await;

        // level filter → only the warn row.
        let warns = pg.query_logs(Some("warn"), Some(&format!("{marker}-a")), None, None, 1000).await.unwrap();
        assert_eq!(warns.len(), 1);
        assert_eq!(warns[0]["level"], "warn");

        // source (running_on) filter → the two `-a` rows only.
        let a_rows = pg.query_logs(None, Some(&format!("{marker}-a")), None, None, 1000).await.unwrap();
        assert_eq!(a_rows.len(), 2);
        assert!(a_rows.iter().all(|r| r["source"] == format!("{marker}-a")));

        // module (context->>'module') filter → only the analyzer row.
        let ana = pg.query_logs(None, None, Some("analyzer"), None, 1000).await.unwrap();
        let mine: Vec<_> = ana.iter()
            .filter(|r| r["source"].as_str().is_some_and(|s| s.starts_with(&marker)))
            .collect();
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0]["context"]["module"], "analyzer");

        cleanup_logs(&pg, &marker).await;
    }

    #[tokio::test]
    async fn query_logs_since_excludes_older_rows() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let marker = format!("_test:logs:{}", uuid::Uuid::new_v4());
        seed_logs(&pg, &marker).await;

        // Cutoff at 1h ago excludes the two rows at base(-2h) and base+30m(-90m),
        // keeping only the base+90m(-30m) analyzer/error row.
        let since = chrono::Utc::now() - chrono::Duration::hours(1);
        let recent = pg.query_logs(None, None, None, Some(since), 1000).await.unwrap();
        let mine: Vec<_> = recent.iter()
            .filter(|r| r["source"].as_str().is_some_and(|s| s.starts_with(&marker)))
            .collect();
        assert_eq!(mine.len(), 1, "since cutoff drops the two older rows");
        assert_eq!(mine[0]["level"], "error");

        cleanup_logs(&pg, &marker).await;
    }

    #[tokio::test]
    async fn query_logs_limit_is_honored() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let marker = format!("_test:logs:{}", uuid::Uuid::new_v4());
        seed_logs(&pg, &marker).await;

        // Scope to this run's sources so the global limit is deterministic.
        let a = pg.query_logs(None, Some(&format!("{marker}-a")), None, None, 1).await.unwrap();
        assert_eq!(a.len(), 1, "limit=1 returns exactly one of the two -a rows");
        // Newest -a row is the warn/watcher one.
        assert_eq!(a[0]["level"], "warn");

        cleanup_logs(&pg, &marker).await;
    }

    #[tokio::test]
    async fn query_logs_empty_result_is_empty_array() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        // A source that never exists → empty Vec, not an error.
        let none = pg.query_logs(None, Some("_test:logs:does-not-exist"), None, None, 200).await.unwrap();
        assert!(none.is_empty());
    }

    // ── Metrics: value store + active registry (Phase 3) ──────────────────

    /// Seed a `sensei.metrics` registry row for a test. Dates are relative to the
    /// DB's `current_date` (via `current_date + <offset> days`) so the active-window
    /// tests don't flake at a local midnight boundary. `until_offset = None` leaves
    /// `effective_until` NULL (never retired). `name` is set to `key` so facet
    /// assertions have a known value.
    async fn seed_metric(
        s: &PgStore, key: &str, task_name: &str, from_offset: i32, until_offset: Option<i32>,
    ) -> uuid::Uuid {
        let row: (uuid::Uuid,) = query_as(
            "INSERT INTO sensei.metrics
                (key, name, description, family, type, direction, purpose, how_to_read, formula,
                 task_name, effective_from, effective_until)
             VALUES ($1, $1, 'test metric', 'quality'::sensei.metric_family, 'ratio'::sensei.metric_type,
                     'higher_better'::sensei.metric_direction, 'test purpose', 'test how', 'test formula',
                     $2, current_date + $3::int, current_date + $4::int)
             RETURNING id",
        )
        .bind(key).bind(task_name).bind(from_offset).bind(until_offset)
        .fetch_one(s.pool()).await.unwrap();
        row.0
    }

    #[tokio::test]
    async fn upsert_project_metric_is_idempotent() {
        // Two upserts of the same identity (metric x project x null folder x null
        // session x date x daily) collapse to ONE row: the second updates value,
        // props, source and bumps modified_at rather than duplicating.
        let s = pg_store().await;
        let uniq = uuid::Uuid::new_v4();
        let pid = s.create_project(&format!("_test:pm-idem:{uniq}"), None, None).await.unwrap();
        let mid = seed_metric(&s, &format!("_test:pm-idem:{uniq}:ftr"), "ComputeFtr", 0, None).await;
        let day = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();

        let id1 = s.upsert_project_metric(
            &mid, &pid, None, None, day, "daily", 0.5,
            &serde_json::json!({"numerator": 1, "denominator": 2}), "measured",
        ).await.unwrap();

        // Backdate modified_at so the second upsert's bump is observable.
        sqlx_core::query::query(
            "UPDATE sensei.project_metrics SET modified_at = now() - interval '1 hour' WHERE id = $1")
            .bind(id1).execute(s.pool()).await.unwrap();
        let (before,): (chrono::DateTime<chrono::Utc>,) =
            query_as("SELECT modified_at FROM sensei.project_metrics WHERE id = $1")
                .bind(id1).fetch_one(s.pool()).await.unwrap();

        let id2 = s.upsert_project_metric(
            &mid, &pid, None, None, day, "daily", 0.75,
            &serde_json::json!({"numerator": 3, "denominator": 4}), "estimated",
        ).await.unwrap();
        assert_eq!(id1, id2, "same identity upserts the same row (no duplicate)");

        let (n,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics
              WHERE metric_id = $1 AND project_id = $2 AND folder_id IS NULL
                AND session_id IS NULL AND computed_on = $3 AND grain = 'daily'")
            .bind(mid).bind(pid).bind(day).fetch_one(s.pool()).await.unwrap();
        assert_eq!(n, 1, "one row per identity — the second upsert updated in place");

        let (value, props, source, after): (f64, serde_json::Value, String, chrono::DateTime<chrono::Utc>) =
            query_as("SELECT value::float8, props, source::text, modified_at FROM sensei.project_metrics WHERE id = $1")
                .bind(id1).fetch_one(s.pool()).await.unwrap();
        assert_eq!(value, 0.75, "value updated to the second upsert's");
        assert_eq!(props, serde_json::json!({"numerator": 3, "denominator": 4}), "props updated");
        assert_eq!(source, "estimated", "source updated");
        assert!(after > before, "modified_at bumped past the backdated value");

        // cleanup — project_metrics rows cascade from the metric + project.
        sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = $1").bind(mid).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn active_metrics_excludes_retired_and_future() {
        // active_metrics() returns only rows live on current_date: the retired
        // (past effective_until) and not-yet-effective (future effective_from) rows
        // are excluded. Assertions are key-specific so the pre-seeded registry and
        // concurrent tests don't interfere.
        let s = pg_store().await;
        let uniq = uuid::Uuid::new_v4();
        let active_key       = format!("_test:active:{uniq}");
        let retired_key      = format!("_test:retired:{uniq}");
        let future_key       = format!("_test:future:{uniq}");
        let today_retire_key = format!("_test:today-retire:{uniq}");
        let active_task        = format!("ComputeActive_{uniq}");
        let retired_task       = format!("ComputeRetired_{uniq}");
        let future_task        = format!("ComputeFuture_{uniq}");
        let today_retire_task  = format!("ComputeTodayRetire_{uniq}");
        seed_metric(&s, &active_key,       &active_task,       0,   None).await;      // from today, no end
        seed_metric(&s, &retired_key,      &retired_task,      -10, Some(-1)).await;  // ended yesterday
        seed_metric(&s, &future_key,       &future_task,       1,   None).await;      // effective tomorrow
        // Retired EFFECTIVE TODAY: effective_until = current_date. The window is
        // half-open [from, until), so `until > current_date` is false today — this
        // row must already be inactive (locks the exclusive-upper-bound boundary).
        seed_metric(&s, &today_retire_key, &today_retire_task, -10, Some(0)).await;

        let metrics = s.active_metrics().await.unwrap();
        let keys: Vec<&str> = metrics.iter().map(|m| m.key.as_str()).collect();
        assert!(keys.contains(&active_key.as_str()), "active metric is returned");
        assert!(!keys.contains(&retired_key.as_str()), "retired metric is excluded");
        assert!(!keys.contains(&future_key.as_str()), "not-yet-effective metric is excluded");
        assert!(!keys.contains(&today_retire_key.as_str()),
            "a metric retired effective today is excluded (effective_until is exclusive)");

        let tasks = s.active_task_names().await.unwrap();
        assert!(tasks.contains(&active_task), "active metric's task_name is present");
        assert!(!tasks.contains(&retired_task), "retired metric's task_name is absent");
        assert!(!tasks.contains(&future_task), "future metric's task_name is absent");
        assert!(!tasks.contains(&today_retire_task),
            "task_name of a metric retired effective today is not scheduled");

        // The mapped Metric carries the facets/knobs.
        let active = metrics.iter().find(|m| m.key == active_key).unwrap();
        assert_eq!(active.family, "quality");
        assert_eq!(active.metric_type, "ratio");
        assert_eq!(active.direction, "higher_better");
        assert_eq!(active.weight, 1.0, "numeric weight defaults to 1");
        assert!(active.effective_until.is_none(), "active metric has no end date");

        sqlx_core::query::query("DELETE FROM sensei.metrics WHERE key IN ($1, $2, $3, $4)")
            .bind(&active_key).bind(&retired_key).bind(&future_key).bind(&today_retire_key)
            .execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn active_metric_ids_maps_active_keys_for_task_name_only() {
        // active_metric_ids(task) returns key→id for ONLY the active metrics whose
        // task_name matches: a same-task retired metric and a different-task metric
        // are both absent, so a compute handler's `ids.get(key)` skips them.
        let s = pg_store().await;
        let uniq = uuid::Uuid::new_v4();
        let task       = format!("ComputeSO_{uniq}");     // unique → no pre-seeded rows share it
        let other_task = format!("ComputeOther_{uniq}");
        let k_a       = format!("_test:ami:{uniq}:a");
        let k_b       = format!("_test:ami:{uniq}:b");
        let k_other   = format!("_test:ami:{uniq}:other");
        let k_retired = format!("_test:ami:{uniq}:retired");
        let id_a = seed_metric(&s, &k_a, &task, 0, None).await;             // active, our task
        let id_b = seed_metric(&s, &k_b, &task, 0, None).await;             // active, our task
        seed_metric(&s, &k_other, &other_task, 0, None).await;             // active, DIFFERENT task
        seed_metric(&s, &k_retired, &task, -10, Some(-1)).await;           // our task but RETIRED

        let ids = s.active_metric_ids(&task).await.unwrap();
        assert_eq!(ids.len(), 2, "only this task's two ACTIVE keys (task_name is unique to this test)");
        assert_eq!(ids.get(&k_a).copied(), Some(id_a), "active key → its metric_id");
        assert_eq!(ids.get(&k_b).copied(), Some(id_b));
        assert!(!ids.contains_key(&k_other), "a key with a different task_name is excluded");
        assert!(!ids.contains_key(&k_retired), "a retired (inactive) key is excluded (never computed)");

        sqlx_core::query::query("DELETE FROM sensei.metrics WHERE key IN ($1, $2, $3, $4)")
            .bind(&k_a).bind(&k_b).bind(&k_other).bind(&k_retired)
            .execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn resolve_folder_from_path_uses_aliases() {
        // A live folders.abs_path resolves; a folder_path_aliases OLD path resolves
        // to the CURRENT folder + project; an unknown path is an honest None.
        let s = pg_store().await;
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = create_test_project_and_folder(&s, &format!("resolve-{uniq}")).await;
        let abs_path = format!("/_test/resolve-{uniq}"); // create_test_folder sets abs_path = /_test/{suffix}
        let alias = format!("/_test/old-resolve-{uniq}");
        s.add_folder_path_alias(&alias, &fid, "rename").await.unwrap();

        assert_eq!(
            s.resolve_folder_by_path(&abs_path).await.unwrap(), Some((fid, pid)),
            "a live folders.abs_path resolves to (folder_id, project_id)");
        assert_eq!(
            s.resolve_folder_by_path(&alias).await.unwrap(), Some((fid, pid)),
            "a folder_path_aliases old path resolves to the current folder + project");
        assert_eq!(
            s.resolve_folder_by_path(&format!("/_test/unknown-{uniq}")).await.unwrap(), None,
            "an unknown path resolves to None (never fabricated)");

        // A folder with NO project attached (folders.project_id null — a real,
        // reachable state: create_test_folder does not wire a project) resolves to
        // None. This pins the never-fabricate contract: the impl must NOT invent a
        // project id (e.g. from the folder id) when the folder has no project.
        let noproj_fid = create_test_folder(&s, &format!("noproj-{uniq}")).await;
        let noproj_path = format!("/_test/noproj-{uniq}");
        assert_eq!(
            s.resolve_folder_by_path(&noproj_path).await.unwrap(), None,
            "a folder without a project resolves to None (never a fabricated project id)");

        // cleanup — the alias cascades on folder delete.
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1").bind(noproj_fid).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1").bind(fid).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(s.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn get_project_metrics_reads_views() {
        // After upserting two daily rows on different dates, get_project_metrics
        // returns the LATEST-per-metric value + props with the catalog facets
        // (name/type/unit/direction/purpose/how_to_read) joined from sensei.metrics.
        let s = pg_store().await;
        let uniq = uuid::Uuid::new_v4();
        let pid = s.create_project(&format!("_test:gpm:{uniq}"), None, None).await.unwrap();
        let key = format!("_test:gpm:{uniq}:cov");
        let mid = seed_metric(&s, &key, "ComputeCoverage", 0, None).await;
        let d1 = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let d2 = chrono::NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(); // later => latest

        s.upsert_project_metric(&mid, &pid, None, None, d1, "daily", 0.5,
            &serde_json::json!({"numerator": 1, "denominator": 2}), "measured").await.unwrap();
        s.upsert_project_metric(&mid, &pid, None, None, d2, "daily", 0.75,
            &serde_json::json!({"numerator": 3, "denominator": 4}), "measured").await.unwrap();

        let rows = s.get_project_metrics(&pid).await.unwrap();
        let row = rows.iter().find(|r| r.metric == key).expect("our metric is present");
        assert_eq!(row.date, d2, "latest date per metric wins");
        assert_eq!(row.value, 0.75, "latest value");
        assert_eq!(row.props, serde_json::json!({"numerator": 3, "denominator": 4}), "props from the latest row");
        assert_eq!(row.name, key, "name facet joined from sensei.metrics (seed sets name = key)");
        assert_eq!(row.metric_type, "ratio", "type facet");
        assert_eq!(row.direction, "higher_better", "direction facet");
        assert_eq!(row.purpose, "test purpose", "purpose facet");
        assert_eq!(row.how_to_read, "test how", "how_to_read facet");
        assert!(row.unit.is_none(), "seed leaves unit null");

        // cleanup — project_metrics rows cascade from the metric + project.
        sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = $1").bind(mid).execute(s.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(s.pool()).await.unwrap();
    }

    /// Phase 8.1: both FTR getters read `project_metrics` (via
    /// `project_metric_daily`), NOT the retired `sensei.ftr_daily` /
    /// `sensei.project_ftr_metrics` views. Seeds daily `ftr` rows across the 14d,
    /// 7d, and prior-14d windows and asserts the re-sourced values + unchanged
    /// response shape.
    #[tokio::test]
    async fn ftr_getters_read_project_metrics() {
        let s = pg_store().await;
        let uniq = uuid::Uuid::new_v4();
        let pid = s.create_project(&format!("_test:ftrget:{uniq}"), None, None).await.unwrap();
        // The REAL registry ftr metric — the getters filter `metric = 'ftr'`.
        let (ftr_mid,): (uuid::Uuid,) =
            sqlx_core::query_as::query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'")
                .fetch_one(s.pool()).await.expect("ftr metric seeded in registry");

        let today = chrono::Utc::now().date_naive();
        let d_recent = today - chrono::Duration::days(3);   // 7d + 14d window
        let d_prev = today - chrono::Duration::days(20);    // prior-14d window only
        // day A (today):     3/4 = 0.75
        s.upsert_project_metric(&ftr_mid, &pid, None, None, today, "daily", 0.75,
            &serde_json::json!({"numerator": 3, "denominator": 4, "correction_count": 1}), "measured").await.unwrap();
        // day B (today-3):   1/2 = 0.50
        s.upsert_project_metric(&ftr_mid, &pid, None, None, d_recent, "daily", 0.5,
            &serde_json::json!({"numerator": 1, "denominator": 2, "correction_count": 2}), "measured").await.unwrap();
        // day C (today-20):  1/2 = 0.50 — prior-14d window, excluded from 14d/7d
        s.upsert_project_metric(&ftr_mid, &pid, None, None, d_prev, "daily", 0.5,
            &serde_json::json!({"numerator": 1, "denominator": 2, "correction_count": 3}), "measured").await.unwrap();

        // ── get_ftr_daily (per-project): value → ftr_rate, props.denominator →
        //    session_count; day C (older than 14d) excluded. Shape unchanged.
        let daily = s.get_ftr_daily(Some(&pid), 14).await.unwrap();
        assert_eq!(daily.len(), 2, "only the two rows inside the 14d window (day C excluded)");
        let a = daily.iter().find(|r| r["day"].as_str() == Some(today.to_string().as_str())).expect("today row");
        assert_eq!(a.as_object().unwrap().keys().collect::<Vec<_>>(),
            vec!["day", "ftr_rate", "session_count"], "exact response shape preserved");
        assert!((a["ftr_rate"].as_f64().unwrap() - 0.75).abs() < 1e-9, "ftr_rate = stored value");
        assert_eq!(a["session_count"].as_i64(), Some(4), "session_count = props.denominator");
        let b = daily.iter().find(|r| r["day"].as_str() == Some(d_recent.to_string().as_str())).expect("today-3 row");
        assert!((b["ftr_rate"].as_f64().unwrap() - 0.5).abs() < 1e-9);
        assert_eq!(b["session_count"].as_i64(), Some(2));

        // ── get_ftr_daily (holistic): sums denominators across projects for the
        //    day; our contribution is a safe lower bound (other test data may add).
        let holistic = s.get_ftr_daily(None, 14).await.unwrap();
        let ht = holistic.iter().find(|r| r["day"].as_str() == Some(today.to_string().as_str())).expect("today holistic row");
        assert!(ht["ftr_rate"].as_f64().is_some(), "holistic ftr_rate present");
        assert!(ht["session_count"].as_i64().unwrap() >= 4, "holistic session_count sums denominators (>= our 4)");

        // ── get_project_ftr headline: Σnum/Σden per window; shape unchanged.
        let ftr = s.get_project_ftr(&pid).await.unwrap();
        assert_eq!(ftr.as_object().unwrap().keys().collect::<Vec<_>>(),
            vec!["ftr14d", "ftr14dPrev", "ftrTrend", "sessions7d"], "exact response shape preserved");
        assert!((ftr["ftr14d"].as_f64().unwrap() - (4.0 / 6.0)).abs() < 1e-9,
            "ftr14d = Σnum/Σden over 14d = (3+1)/(4+2)");
        assert!((ftr["ftr14dPrev"].as_f64().unwrap() - 0.5).abs() < 1e-9,
            "ftr14dPrev = Σnum/Σden over prior-14d window (day C only) = 1/2");
        assert_eq!(ftr["sessions7d"].as_i64(), Some(6), "sessions7d = Σdenominator over 7d = 4+2");
        assert!(ftr["ftrTrend"].as_array().is_some(), "ftrTrend is an array (trend reads sessions, not the store)");

        // ── shared rate helper agrees with the headline.
        assert!((s.get_project_ftr_rate(&pid).await.unwrap().unwrap() - (4.0 / 6.0)).abs() < 1e-9,
            "get_project_ftr_rate == ftr14d");

        // cleanup — project_metrics rows cascade from the project (ftr metric kept).
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(s.pool()).await.unwrap();
    }

    /// FIX 1 (DB-free): the shared headline builder serializes an absent 14d /
    /// prior-14d FTR as JSON `null` — NEVER a fabricated `0.0`. Covers BOTH
    /// `get_project_ftr` and `get_holistic_ftr`, which share this builder.
    /// Mutation guard: reverting the builder to `.unwrap_or(0.0)` fails this.
    #[test]
    fn ftr_headline_json_absent_serializes_null_not_zero() {
        let absent = PgStore::ftr_headline_json(None, None, vec![], 0);
        assert!(absent["ftr14d"].is_null(), "absent ftr14d → JSON null, not 0.0");
        assert!(absent["ftr14dPrev"].is_null(), "absent ftr14dPrev → JSON null, not 0.0");
        assert_eq!(absent["sessions7d"].as_i64(), Some(0), "sessions7d is an honest count");
        let present = PgStore::ftr_headline_json(Some(0.5), Some(0.25), vec![0.5], 3);
        assert_eq!(present["ftr14d"].as_f64(), Some(0.5), "a present value still serializes as a number");
        assert_eq!(present["ftr14dPrev"].as_f64(), Some(0.25));
    }

    /// FIX 1 (end-to-end): a project with zero stored `ftr` rows reports honest
    /// `null` for the headline through `get_project_ftr` — never a fabricated 0%.
    #[tokio::test]
    async fn get_project_ftr_absent_is_null_not_zero() {
        let s = pg_store().await;
        let pid = s.create_project(&format!("_test:ftrnull:{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let ftr = s.get_project_ftr(&pid).await.unwrap();
        assert!(ftr["ftr14d"].is_null(), "no ftr rows → ftr14d is null, NOT 0.0");
        assert!(ftr["ftr14dPrev"].is_null(), "no ftr rows → ftr14dPrev is null, NOT 0.0");
        assert_eq!(ftr["sessions7d"].as_i64(), Some(0), "sessions7d is an honest 0 (a count)");
        assert!(ftr["ftrTrend"].as_array().is_some_and(|a| a.is_empty()), "no sessions → empty trend");
        assert_eq!(s.get_project_ftr_rate(&pid).await.unwrap(), None, "rate helper is None on no data");
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(s.pool()).await.unwrap();
    }

    /// FIX 5 (window mutation guard): `ftr14d` must reach back the full 14 days.
    /// The only stored row is 10 days old — inside 14d, outside 7d — so `ftr14d`
    /// is 1.0 while `sessions7d` (7d) excludes it. Narrowing the 14d window to 7d
    /// would make `ftr14d` null, failing the `.expect` below.
    #[tokio::test]
    async fn ftr14d_window_reaches_the_8_to_13_day_band() {
        let s = pg_store().await;
        let pid = s.create_project(&format!("_test:ftrwin:{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let (ftr_mid,): (uuid::Uuid,) =
            query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'").fetch_one(s.pool()).await.unwrap();
        let d10 = chrono::Utc::now().date_naive() - chrono::Duration::days(10); // 8–13d band
        s.upsert_project_metric(&ftr_mid, &pid, None, None, d10, "daily", 1.0,
            &serde_json::json!({"numerator": 2, "denominator": 2}), "measured").await.unwrap();

        let ftr = s.get_project_ftr(&pid).await.unwrap();
        assert!((ftr["ftr14d"].as_f64().expect("ftr14d includes the 10-day-old row (14d window)") - 1.0).abs() < 1e-9,
            "only row is 10d old → ftr14d = 1.0; a 7d-narrowed window would make this null");
        assert_eq!(ftr["sessions7d"].as_i64(), Some(0),
            "sessions7d (7d window) excludes the 10-day-old row — proves 14d ≠ 7d");
        assert_eq!(s.get_project_ftr_rate(&pid).await.unwrap(), Some(1.0), "rate helper (14d) includes it too");

        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(s.pool()).await.unwrap();
    }

    /// FIX 3: the holistic (no-project) `get_ftr_daily` branch POOLS Σnum/Σden per
    /// day — it must NOT average per-project daily rates (the `project_metrics`
    /// ratio invariant). Two projects on one day with unequal denominators make
    /// pooled ≠ avg-of-rates; the getter must match the pooled value.
    #[tokio::test]
    async fn holistic_ftr_daily_pools_not_average_of_rates() {
        let s = pg_store().await;
        let (ftr_mid,): (uuid::Uuid,) =
            query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'").fetch_one(s.pool()).await.unwrap();
        // A day off the busy 'today' (compute-writing tests seed today) but inside 14d.
        let day = chrono::Utc::now().date_naive() - chrono::Duration::days(6);
        let p1 = s.create_project(&format!("_test:ftrpool1:{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let p2 = s.create_project(&format!("_test:ftrpool2:{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        // P1: 1/1 = 1.0 ; P2: 0/3 = 0.0 → avg-of-rates 0.5, pooled 1/4 = 0.25.
        s.upsert_project_metric(&ftr_mid, &p1, None, None, day, "daily", 1.0,
            &serde_json::json!({"numerator": 1, "denominator": 1}), "measured").await.unwrap();
        s.upsert_project_metric(&ftr_mid, &p2, None, None, day, "daily", 0.0,
            &serde_json::json!({"numerator": 0, "denominator": 3}), "measured").await.unwrap();

        let holistic = s.get_ftr_daily(None, 14).await.unwrap();
        let row = holistic.iter().find(|r| r["day"].as_str() == Some(day.to_string().as_str()))
            .expect("holistic row for the seeded day");

        // Compare to the DIRECT pooled + avg over whatever exists globally for that
        // day (robust to other rows), and assert the getter matches POOLED, not avg.
        let (sum_num, sum_den, avg_rate): (f64, i64, f64) = query_as(
            "SELECT SUM((props->>'numerator')::float8), SUM((props->>'denominator')::int8)::int8, AVG(value)::float8 \
               FROM sensei.project_metric_daily WHERE metric = 'ftr' AND date = $1",
        ).bind(day).fetch_one(s.pool()).await.unwrap();
        let pooled = sum_num / sum_den as f64;
        assert!((row["ftr_rate"].as_f64().unwrap() - pooled).abs() < 1e-9,
            "holistic ftr_rate is pooled Σnum/Σden, not an average of per-project rates");
        assert_eq!(row["session_count"].as_i64(), Some(sum_den), "holistic session_count is Σdenominator");
        assert!((pooled - avg_rate).abs() > 1e-9,
            "seed makes pooled ({pooled}) differ from avg-of-rates ({avg_rate}) — so the check above is a real discriminator");

        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = ANY($1)")
            .bind(vec![p1, p2]).execute(s.pool()).await.unwrap();
    }
}

#[cfg(test)]
mod knowledge_tests {
    use super::*;

    fn ddl_test_skip() -> bool {
        // Tests require a running sensei_dev DB. Skip if env var not set.
        std::env::var("SENSEI_TEST_DB_URL").is_err()
    }

    #[tokio::test]
    async fn list_memories_filters_by_status() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let project_id = pg.ensure_test_project("list-status").await.unwrap();
        let m1 = pg.insert_memory(&InsertMemory {
            project_id: Some(project_id), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "t1".into(), content: "c1".into(),
            impact: None, tags: vec![], triage_signal: None, status: "proposed".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();
        let _m2 = pg.insert_memory(&InsertMemory {
            project_id: Some(project_id), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "t2".into(), content: "c2".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();

        let proposed = pg.list_memories(Some(project_id), Some("proposed"), None, 50).await.unwrap();
        assert_eq!(proposed.len(), 1);
        assert_eq!(proposed[0]["id"].as_str().unwrap(), m1.to_string());

        // `list-status` is a reused fixture project (ensure_test_project, #34) —
        // clean up so repeated runs don't accrete proposed rows into the count.
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
            .bind(&[m1, _m2][..]).execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn set_memory_status_accept_proposal() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let pid = pg.ensure_test_project("accept-prop").await.unwrap();
        let mid = pg.insert_memory(&InsertMemory {
            project_id: Some(pid), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "t".into(), content: "c".into(),
            impact: None, tags: vec![], triage_signal: Some("revert".into()),
            status: "proposed".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();

        let new_status = pg.set_memory_status(mid, "active", &["proposed"]).await.unwrap();
        assert_eq!(new_status.as_deref(), Some("active"));

        // Trying to accept a now-active memory fails.
        let err = pg.set_memory_status(mid, "active", &["proposed"]).await;
        assert!(err.is_err() || err.unwrap().is_none(), "second accept should not match WHERE clause");
    }

    #[tokio::test]
    async fn get_memory_detail_includes_outcomes() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let pid = pg.ensure_test_project("detail").await.unwrap();
        let mid = pg.insert_memory(&InsertMemory {
            project_id: Some(pid), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "t".into(), content: "c".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();
        let skipped = pg.record_outcomes_batch(&[
            OutcomeRow { memory_id: mid, session_id: None, outcome: "applied".into(), context: None }
        ]).await.unwrap();
        assert_eq!(skipped.len(), 0);

        let detail = pg.get_memory_detail(mid).await.unwrap();
        assert!(detail["memory"]["id"].as_str().unwrap() == mid.to_string());
        assert_eq!(detail["outcomes"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn assemble_context_blends_three_scopes() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let pid = pg.ensure_test_project("blend").await.unwrap();

        let m_p = pg.insert_memory(&InsertMemory {
            project_id: Some(pid), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "P".into(), content: "p".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();
        let m_s = pg.insert_memory(&InsertMemory {
            project_id: None, scope: "stack".into(), scope_filter: Some("rust".into()),
            mtype: "convention".into(), title: "S".into(), content: "s".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();
        let m_g = pg.insert_memory(&InsertMemory {
            project_id: None, scope: "global".into(), scope_filter: None,
            mtype: "convention".into(), title: "G".into(), content: "g".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();

        let blob = pg.assemble_context(pid, &["rust".into()], None, 50, None).await.unwrap();
        let titles: Vec<String> = blob["memories"].as_array().unwrap().iter()
            .map(|m| m["title"].as_str().unwrap().to_string()).collect();
        assert!(titles.contains(&"P".to_string()));
        assert!(titles.contains(&"S".to_string()));
        assert!(titles.contains(&"G".to_string()));

        // Proposed memories must not appear.
        let m_prop = pg.insert_memory(&InsertMemory {
            project_id: Some(pid), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "PROP".into(), content: "x".into(),
            impact: None, tags: vec![], triage_signal: Some("revert".into()),
            status: "proposed".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();
        let blob2 = pg.assemble_context(pid, &["rust".into()], None, 50, None).await.unwrap();
        let titles2: Vec<String> = blob2["memories"].as_array().unwrap().iter()
            .map(|m| m["title"].as_str().unwrap().to_string()).collect();
        assert!(!titles2.contains(&"PROP".to_string()));

        // `blend` is a reused fixture project (#34) and "S"/"G" are global/stack
        // scoped — visible to every project. Clean up so repeated runs don't
        // accrete rows that eventually push this test's own memories out of
        // assemble_context's top-N window.
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
            .bind(&[m_p, m_s, m_g, m_prop][..]).execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn list_memories_for_slot_matches_slot_and_feature() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let pid = pg.ensure_test_project("slot-retrieval").await.unwrap();

        let m_design = pg.create_memory(Some(&pid), "project", None, "decision",
            "design-project-scope", "c", None, None, Some("design"), None).await.unwrap();
        let m_design_auth = pg.create_memory(Some(&pid), "project", None, "decision",
            "design-auth-feature", "c", None, None, Some("design"), Some("auth")).await.unwrap();
        let m_decisions = pg.create_memory(Some(&pid), "project", None, "decision",
            "decisions-project-scope", "c", None, None, Some("decisions"), None).await.unwrap();

        let design_project = pg.list_memories_for_slot(&pid, "design", None, 50).await.unwrap();
        assert_eq!(design_project.len(), 1);
        assert_eq!(design_project[0]["id"].as_str().unwrap(), m_design.to_string());

        let design_auth = pg.list_memories_for_slot(&pid, "design", Some("auth"), 50).await.unwrap();
        assert_eq!(design_auth.len(), 1);
        assert_eq!(design_auth[0]["id"].as_str().unwrap(), m_design_auth.to_string());

        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
            .bind(&[m_design, m_design_auth, m_decisions][..]).execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn assemble_context_leads_with_slot_anchored_memory() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let pid = pg.ensure_test_project("slot-leads").await.unwrap();

        // Unanchored memory created first so a strength/recency-only ordering
        // would put it ahead of the slot-anchored one below.
        let m_unanchored = pg.create_memory(Some(&pid), "project", None, "decision",
            "unanchored", "c", None, None, None, None).await.unwrap();
        let m_design = pg.create_memory(Some(&pid), "project", None, "decision",
            "design-anchored", "c", None, None, Some("design"), None).await.unwrap();

        let blob = pg.assemble_context(pid, &[], None, 50, Some(("design", None))).await.unwrap();
        let ids: Vec<String> = blob["memories"].as_array().unwrap().iter()
            .map(|m| m["id"].as_str().unwrap().to_string()).collect();
        assert_eq!(ids.first().map(String::as_str), Some(m_design.to_string().as_str()),
            "slot-anchored memory must lead the assembled bundle");
        assert!(ids.contains(&m_unanchored.to_string()), "general blend still present");

        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
            .bind(&[m_unanchored, m_design][..]).execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn assemble_context_logs_one_load_per_memory() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let pid = pg.ensure_test_project("loads-writer").await.unwrap();
        // Project-scoped active memory → loaded exactly once per assemble_context
        // call on this (test-unique) project.
        let mid = pg.insert_memory(&InsertMemory {
            project_id: Some(pid), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "L".into(), content: "l".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();

        let blob = pg.assemble_context(pid, &[], None, 50, None).await.unwrap();
        // Context is still delivered (writer is additive, non-fatal).
        assert!(blob["memories"].as_array().unwrap().iter()
            .any(|m| m["id"].as_str() == Some(&mid.to_string())));

        let (loaded, followed, skipped) = pg.memory_telemetry_7d(mid).await.unwrap();
        assert_eq!(loaded, 1, "one load row per delivered memory");
        assert_eq!(followed, 0);
        assert_eq!(skipped, 0);

        // A second delivery logs a second load row.
        pg.assemble_context(pid, &[], None, 50, None).await.unwrap();
        let (loaded2, _, _) = pg.memory_telemetry_7d(mid).await.unwrap();
        assert_eq!(loaded2, 2);

        // Source + a non-null loaded_at are recorded; session_id NULL is tolerated.
        let (source, sess_null): (String, bool) = sqlx_core::query_as::query_as(
            "SELECT source, session_id IS NULL FROM activity.memory_loads
              WHERE memory_id = $1 ORDER BY id DESC LIMIT 1"
        ).bind(mid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(source, "get_layered_context");
        assert!(sess_null, "v1 logs loads with session_id NULL");
    }

    #[tokio::test]
    async fn memory_loaded_last_7d_respects_window() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let pid = pg.ensure_test_project("loads-window").await.unwrap();
        let mid = pg.insert_memory(&InsertMemory {
            project_id: Some(pid), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "W".into(), content: "w".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();

        // One load in-window, one back-dated outside the 7d window.
        sqlx_core::query::query("INSERT INTO activity.memory_loads (memory_id) VALUES ($1)")
            .bind(mid).execute(pg.pool()).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO activity.memory_loads (memory_id, loaded_at) VALUES ($1, now() - interval '10 days')"
        ).bind(mid).execute(pg.pool()).await.unwrap();

        let (loaded, _, _) = pg.memory_telemetry_7d(mid).await.unwrap();
        assert_eq!(loaded, 1, "only the in-window load is counted");
    }

    #[tokio::test]
    async fn memory_followed_skipped_last_7d_over_outcomes() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let pid = pg.ensure_test_project("followed-skipped").await.unwrap();
        let mid = pg.insert_memory(&InsertMemory {
            project_id: Some(pid), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "F".into(), content: "f".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();

        // In-window outcomes: applied + ignored count; consulted + violated do not.
        for oc in ["applied", "ignored", "consulted", "violated"] {
            sqlx_core::query::query(
                "INSERT INTO sensei.memory_outcomes (memory_id, outcome) VALUES ($1, $2::sensei.memory_outcome)"
            ).bind(mid).bind(oc).execute(pg.pool()).await.unwrap();
        }
        // Back-dated applied must NOT count toward followed.
        sqlx_core::query::query(
            "INSERT INTO sensei.memory_outcomes (memory_id, outcome, recorded_at)
             VALUES ($1, 'applied'::sensei.memory_outcome, now() - interval '10 days')"
        ).bind(mid).execute(pg.pool()).await.unwrap();

        let (loaded, followed, skipped) = pg.memory_telemetry_7d(mid).await.unwrap();
        assert_eq!(loaded, 0, "no loads logged in this test");
        assert_eq!(followed, 1, "only the in-window applied outcome");
        assert_eq!(skipped, 1, "only the in-window ignored outcome");
    }

    #[tokio::test]
    async fn get_memory_detail_includes_7d_telemetry() {
        if ddl_test_skip() { return; }
        let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
        let pid = pg.ensure_test_project("detail-telemetry").await.unwrap();
        let mid = pg.insert_memory(&InsertMemory {
            project_id: Some(pid), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "D".into(), content: "d".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: None, origin: None, source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();

        sqlx_core::query::query("INSERT INTO activity.memory_loads (memory_id) VALUES ($1)")
            .bind(mid).execute(pg.pool()).await.unwrap();
        for oc in ["applied", "ignored"] {
            sqlx_core::query::query(
                "INSERT INTO sensei.memory_outcomes (memory_id, outcome) VALUES ($1, $2::sensei.memory_outcome)"
            ).bind(mid).bind(oc).execute(pg.pool()).await.unwrap();
        }

        let detail = pg.get_memory_detail(mid).await.unwrap();
        assert_eq!(detail["loaded_last_7d"].as_i64().unwrap(), 1);
        assert_eq!(detail["followed_last_7d"].as_i64().unwrap(), 1);
        assert_eq!(detail["skipped_last_7d"].as_i64().unwrap(), 1);
    }

    #[tokio::test]
    async fn insert_memory_persists_source_id() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let src = uuid::Uuid::new_v4();
        let id = pg.insert_memory(&InsertMemory {
            project_id: None, scope: "global".into(), scope_filter: None,
            mtype: "convention".into(), title: "fed".into(), content: "federated content".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: Some("recommended".into()),
            origin: Some("federated".into()), source_id: Some(src),
            spine_slot: None, feature: None,
        }).await.unwrap();
        let got: (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT source_id FROM sensei.memories WHERE id = $1")
            .bind(id).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(got.0, Some(src));
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(id).execute(pg.pool()).await.unwrap();
    }

    /// Governance scope hygiene: a project's learned convention (an unscoped
    /// memory carrying a `project_id`, exactly what the L2 generator writes in
    /// `tasks::handlers::generate::generate_for_project`) must resolve ONLY for
    /// its own project's repo — labeled `project`, not the always-on `general`
    /// set — and must never bleed into another project's ruleset or the global
    /// `~/.sensei/rules.md`. Regression for the cross-project general-rule bleed
    /// found by dogfooding `get_rules`.
    #[tokio::test]
    async fn project_learned_convention_scopes_to_its_own_project_not_general() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let pool = pg.pool();

        // Two projects, each with its own repo folder attributed to it.
        let proj_a = pg.create_project(&format!("_test:rules-A-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let proj_b = pg.create_project(&format!("_test:rules-B-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let root = pg.add_watch_root(&format!("/_test/rules-root-{}", uuid::Uuid::new_v4()), "t", &serde_json::json!([])).await.unwrap();
        let folder_a = pg.upsert_repo(&root, "rules-repo-a", &format!("/_test/rules-a-{}", uuid::Uuid::new_v4())).await.unwrap();
        let folder_b = pg.upsert_repo(&root, "rules-repo-b", &format!("/_test/rules-b-{}", uuid::Uuid::new_v4())).await.unwrap();
        pg.set_folder_project(&folder_a, &proj_a, "root", None).await.unwrap();
        pg.set_folder_project(&folder_b, &proj_b, "root", None).await.unwrap();

        // A learned convention captured for project A: namespace_id NULL,
        // project-tied — the shape the L2 generator emits.
        let conv_content = format!("project A convention {}", uuid::Uuid::new_v4());
        let conv = pg.insert_memory(&InsertMemory {
            project_id: Some(proj_a), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "conv A".into(), content: conv_content.clone(),
            impact: None, tags: vec![], triage_signal: Some("repeat_pattern".into()), status: "active".into(),
            namespace_id: None, enforcement: None, origin: Some("learned".into()), source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();

        // A genuinely-global rule: unscoped AND not tied to a project — the real
        // always-on set that must keep working.
        let global_content = format!("genuinely global rule {}", uuid::Uuid::new_v4());
        let global = pg.insert_memory(&InsertMemory {
            project_id: None, scope: "global".into(), scope_filter: None,
            mtype: "convention".into(), title: "global rule".into(), content: global_content.clone(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
            namespace_id: None, enforcement: Some("recommended".into()), origin: Some("authored".into()), source_id: None,
            spine_slot: None, feature: None,
        }).await.unwrap();

        // Project A's ruleset: A's convention (labeled `project`) + the global rule.
        let a_rules = pg.resolve_rules_raw(&folder_a).await.unwrap();
        let a_conv = a_rules.iter().find(|r| r.content == conv_content).expect("A's convention resolves for A");
        assert_eq!(a_conv.scope, "project", "a project-tied unscoped convention is labeled project, not general");
        assert!(a_rules.iter().any(|r| r.content == global_content), "the genuinely-global rule applies to A");

        // Project B's ruleset: MUST NOT contain A's convention; the global rule still applies.
        let b_rules = pg.resolve_rules_raw(&folder_b).await.unwrap();
        assert!(!b_rules.iter().any(|r| r.content == conv_content), "A's learned convention must NOT bleed into project B");
        assert!(b_rules.iter().any(|r| r.content == global_content), "the genuinely-global rule still applies to B");

        // Global always-on set: the genuinely-global rule, NOT any project convention.
        let global_set = pg.resolve_global_rules().await.unwrap();
        assert!(global_set.iter().any(|r| r.content == global_content), "genuinely-global rule is in the always-on set");
        assert!(!global_set.iter().any(|r| r.content == conv_content), "a project convention must NOT be in the always-on global set");

        // cleanup (best-effort)
        for id in [conv, global] {
            sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(id).execute(pool).await.ok();
        }
        for f in [folder_a, folder_b] {
            sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1").bind(f).execute(pool).await.ok();
        }
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1").bind(root).execute(pool).await.ok();
        for p in [proj_a, proj_b] {
            sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(p).execute(pool).await.ok();
        }
    }

    #[tokio::test]
    async fn federated_ledger_and_shareability() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        // Seed the scopes used by the test (sensei_test is empty; production data
        // is seeded via staging.import_scopes — we replicate the two rows we need).
        sqlx_core::query::query(
            "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('organization', 'Organization', 20, true),
                    ('technology',   'Technology',   40, false)
             ON CONFLICT (key) DO UPDATE SET shareable = EXCLUDED.shareable")
            .execute(pg.pool()).await.unwrap();

        // organization is shareable; technology is not (seeded scopes ladder).
        let org_ns = pg.upsert_namespace("organization", "Test Org", "test-org-fed").await.unwrap();
        let tech_ns = pg.upsert_namespace("technology", "Rust", "rust-fed").await.unwrap();
        assert!(pg.namespace_is_shareable(&org_ns).await.unwrap());
        assert!(!pg.namespace_is_shareable(&tech_ns).await.unwrap());

        let src = pg.create_knowledge_source(&NewKnowledgeSource {
            kind: "hive_mind".into(), name: "H".into(), url: "u".into(), namespace_id: None,
            credential_ref: "c".into(), direction: "both".into() }).await.unwrap();
        let remote = uuid::Uuid::new_v4();
        let mem = pg.insert_memory(&InsertMemory {
            project_id: None, scope: "global".into(), scope_filter: None, mtype: "convention".into(),
            title: "t".into(), content: "c".into(), impact: None, tags: vec![], triage_signal: None,
            status: "active".into(), namespace_id: Some(org_ns), enforcement: Some("recommended".into()),
            origin: Some("federated".into()), source_id: Some(src),
            spine_slot: None, feature: None }).await.unwrap();
        pg.upsert_federated_memory(&src, &remote, "hash1", Some(&mem), 5).await.unwrap();
        pg.upsert_federated_memory(&src, &remote, "hash1", Some(&mem), 9).await.unwrap(); // idempotent
        let link = pg.find_federated_memory(&src, &remote).await.unwrap().unwrap();
        assert_eq!(link.memory_id, Some(mem));
        assert_eq!(link.remote_seq, 9);

        // push payload: returns snapshot + namespace identity (incl. name) + origin/scope_key
        let payload = pg.memory_push_payload(&mem).await.unwrap().unwrap();
        assert_eq!(payload.scope_key, "organization");
        assert_eq!(payload.slug, "test-org-fed");
        assert_eq!(payload.name, "Test Org");
        assert_eq!(payload.origin, "federated");

        // archive retires a federated memory (drops out of resolution)
        assert!(pg.archive_federated_memory(&mem).await.unwrap());
        let (status,): (String,) = sqlx_core::query_as::query_as("SELECT status::text FROM sensei.memories WHERE id=$1")
            .bind(mem).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(status, "archived");

        pg.delete_knowledge_source(&src).await.unwrap(); // cascades the ledger row
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id=$1").bind(mem).execute(pg.pool()).await.unwrap();
        // clean up namespaces and seeded scopes
        sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE id = ANY($1::uuid[])")
            .bind(vec![org_ns, tech_ns]).execute(pg.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.scopes WHERE key IN ('organization','technology')")
            .execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn latest_hook_event_ts_returns_max_for_family() {
        let pg = PgStore::connect_test().await.unwrap();
        let base = 1_900_000_000_000_i64; // far-future, won't collide with seeded data
        for (i, off) in [0_i64, 5000, 2000].iter().enumerate() {
            pg.insert_hook_event(
                &format!("sess-test-{i}"), "claude", "PreToolUse", Some("Bash"),
                Some("/tmp"), base + off, Some(true), &serde_json::json!({"t": i}),
            ).await.unwrap();
        }
        let max = pg.latest_hook_event_ts("claude").await.unwrap().unwrap();
        assert!(max >= base + 5000, "expected >= {} got {max}", base + 5000);
    }
}

#[cfg(test)]
mod run_tests {
    //! DB-touching CRUD tests for the relay run-state model. Each test is
    //! self-contained: `project_id` is `None` (nullable FK), and every created
    //! run is cascade-deleted at the end (`run_events` cascade with the run).
    //! Guarded like the neighbouring pg_store tests — a missing test DB means
    //! the test no-ops rather than fails.
    use super::*;

    async fn delete_run(pg: &PgStore, id: &uuid::Uuid) {
        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(id).execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn create_get_and_defaults() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        // Minimal create — plan_ref/max_concurrency fall back to DDL defaults.
        let id = pg.create_run(&NewRun::default()).await.unwrap();
        let run = pg.get_run(&id).await.unwrap().expect("run exists");
        assert_eq!(run.id, id);
        assert_eq!(run.project_id, None);
        assert_eq!(run.plan_ref, "", "plan_ref defaults to ''");
        assert_eq!(run.status, RelayRunStatus::Running, "status defaults to running");
        assert_eq!(run.max_concurrency, 1, "max_concurrency defaults to 1");
        assert!(run.paused_until.is_none());
        assert!(run.completed_at.is_none());
        assert!(run.started_at.contains('T'), "started_at is RFC-3339 text");
        assert!(run.created_at.contains('T'));

        // Unknown id → None, not an error.
        assert!(pg.get_run(&uuid::Uuid::new_v4()).await.unwrap().is_none());

        delete_run(&pg, &id).await;
    }

    #[tokio::test]
    async fn create_with_fields() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let session = uuid::Uuid::new_v4();
        let id = pg.create_run(&NewRun {
            project_id: None,
            plan_ref: Some("docs/plan/P3.md".into()),
            goal: Some("ship relay".into()),
            dojo_session_id: Some(session),
            max_concurrency: Some(3),
            author_name: Some("Sensei HQ".into()),
            author_email: Some("dev@sensei-hq.com".into()),
            plan_graph: Some(serde_json::json!({
                "phases": [{ "title": "P", "tasks": [{ "id": "t1", "title": "x" }] }]
            })),
        }).await.unwrap();
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(run.plan_ref, "docs/plan/P3.md");
        assert_eq!(run.goal.as_deref(), Some("ship relay"));
        assert_eq!(run.dojo_session_id, Some(session));
        assert_eq!(pg.run_author(&id).await.unwrap(),
            (Some("Sensei HQ".into()), Some("dev@sensei-hq.com".into())),
            "create_run stamps + run_author reads the git author back");
        assert_eq!(run.max_concurrency, 3);
        // plan_graph stored + read back on demand (off the 16-col RUN_SELECT).
        let g = pg.run_plan_graph(&id).await.unwrap().expect("plan_graph stored");
        assert_eq!(g["phases"][0]["tasks"][0]["id"], serde_json::json!("t1"));
        // set_run_plan_graph overwrites it (the update_task_status write-back path).
        pg.set_run_plan_graph(&id, &serde_json::json!({ "phases": [] })).await.unwrap();
        assert_eq!(pg.run_plan_graph(&id).await.unwrap().unwrap(), serde_json::json!({ "phases": [] }));
        delete_run(&pg, &id).await;
    }

    #[tokio::test]
    async fn set_run_dojo_session_id_persists_the_cloud_join() {
        // The P1 run→relay bridge persists the cloud session id after the first
        // successful publish, so the local run joins to its relay session.
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let id = pg.create_run(&NewRun::default()).await.unwrap();
        // Fresh run has no cloud session yet.
        assert!(pg.get_run(&id).await.unwrap().unwrap().dojo_session_id.is_none());

        let cloud = uuid::Uuid::new_v4();
        pg.set_run_dojo_session_id(&id, &cloud).await.unwrap();
        assert_eq!(
            pg.get_run(&id).await.unwrap().unwrap().dojo_session_id,
            Some(cloud),
            "the cloud session id is persisted onto the run"
        );

        delete_run(&pg, &id).await;
    }

    #[tokio::test]
    async fn status_pause_progress_heartbeat_complete() {
        // Holds the shared resume lock: this test asserts a run STAYS paused, but
        // the global resume_due_runs (scheduler tests) would resume any paused run
        // whose paused_until has elapsed — so serialize against those callers.
        let _guard = crate::runs::resume_test_lock().lock().await;
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let id = pg.create_run(&NewRun::default()).await.unwrap();

        // Pause with a FAR-FUTURE resume time + reason (never "due", so the
        // global resume sweep can't flip it mid-assertion).
        pg.update_run_status(
            &id, RelayRunStatus::Paused,
            Some("2999-07-17T11:29:00Z"), Some("weekly cap"),
        ).await.unwrap();
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(run.status, RelayRunStatus::Paused);
        assert!(run.paused_until.as_deref().unwrap().contains("2999-07-17"));
        assert_eq!(run.pause_reason.as_deref(), Some("weekly cap"));

        // Resume clears the pause fields.
        pg.update_run_status(&id, RelayRunStatus::Running, None, None).await.unwrap();
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(run.status, RelayRunStatus::Running);
        assert!(run.paused_until.is_none());
        assert!(run.pause_reason.is_none());

        // Progress markers.
        pg.set_run_progress(&id, Some("P3"), Some("run-state model")).await.unwrap();
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(run.current_phase.as_deref(), Some("P3"));
        assert_eq!(run.current_feature.as_deref(), Some("run-state model"));

        // Heartbeat sets heartbeat_at.
        assert!(run.heartbeat_at.is_none());
        pg.touch_run_heartbeat(&id).await.unwrap();
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert!(run.heartbeat_at.as_deref().unwrap().contains('T'));

        // Terminal completion stamps completed_at.
        pg.complete_run(&id, RelayRunStatus::Done).await.unwrap();
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(run.status, RelayRunStatus::Done);
        assert!(run.completed_at.as_deref().unwrap().contains('T'));

        delete_run(&pg, &id).await;
    }

    #[tokio::test]
    async fn list_active_runs_filters_by_status() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let active = pg.create_run(&NewRun::default()).await.unwrap(); // running
        let paused = pg.create_run(&NewRun::default()).await.unwrap();
        pg.update_run_status(&paused, RelayRunStatus::Paused, None, None).await.unwrap();
        // A blocked run (waiting on a gate) must stay in the active set so the
        // scheduler keeps heartbeating it and GET /api/runs keeps showing it —
        // otherwise once P3.3 sets status='blocked' the run drops out and looks
        // crashed. (The advance_run handler has a Blocked-heartbeat branch.)
        let blocked = pg.create_run(&NewRun::default()).await.unwrap();
        pg.update_run_status(&blocked, RelayRunStatus::Blocked, None, None).await.unwrap();
        let terminal = pg.create_run(&NewRun::default()).await.unwrap();
        pg.complete_run(&terminal, RelayRunStatus::Done).await.unwrap();

        let ids: std::collections::HashSet<uuid::Uuid> =
            pg.list_active_runs().await.unwrap().into_iter().map(|r| r.id).collect();
        assert!(ids.contains(&active), "running run is active");
        assert!(ids.contains(&paused), "paused run is active");
        assert!(ids.contains(&blocked), "blocked run is active");
        assert!(!ids.contains(&terminal), "done run is excluded");

        for id in [active, paused, blocked, terminal] { delete_run(&pg, &id).await; }
    }

    #[tokio::test]
    async fn append_and_list_events_newest_first() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let id = pg.create_run(&NewRun::default()).await.unwrap();

        let e1 = pg.append_run_event(
            &id, RunEventKind::PhaseStarted, Some("P3"), None, &serde_json::json!({}),
        ).await.unwrap();
        let e2 = pg.append_run_event(
            &id, RunEventKind::PausedOnLimit, Some("P3"), Some("run-state"),
            &serde_json::json!({ "reset_at": "2026-07-17T11:29:00Z" }),
        ).await.unwrap();
        assert!(e2 > e1, "bigserial is monotonic");

        let events = pg.list_run_events(&id, 10).await.unwrap();
        assert_eq!(events.len(), 2);
        // Newest first — the paused_on_limit event leads.
        assert_eq!(events[0].id, e2);
        assert_eq!(events[0].kind, RunEventKind::PausedOnLimit);
        assert_eq!(events[0].feature.as_deref(), Some("run-state"));
        assert_eq!(events[0].detail["reset_at"], serde_json::json!("2026-07-17T11:29:00Z"));
        assert_eq!(events[1].kind, RunEventKind::PhaseStarted);
        assert_eq!(events[1].detail, serde_json::json!({}), "detail defaults to {{}}");
        assert!(events[0].created_at.contains('T'));

        // limit caps the result.
        assert_eq!(pg.list_run_events(&id, 1).await.unwrap().len(), 1);

        delete_run(&pg, &id).await; // cascades run_events
        assert!(pg.list_run_events(&id, 10).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn resume_due_runs_flips_only_elapsed_pauses() {
        // resume_due_runs is a global UPDATE; serialize with the scheduler test
        // that also creates due-paused runs (see runs::resume_test_lock).
        let _guard = crate::runs::resume_test_lock().lock().await;
        let Ok(pg) = PgStore::connect_test().await else { return; };
        // Clear any stray due pauses so our set assertions are exact.
        pg.resume_due_runs().await.unwrap();

        // Due: paused with paused_until in the past → should resume.
        let due = pg.create_run(&NewRun::default()).await.unwrap();
        pg.update_run_status(
            &due, RelayRunStatus::Paused,
            Some("2000-01-01T00:00:00Z"), Some("elapsed cap"),
        ).await.unwrap();

        // Not-yet-due: paused with paused_until far in the future.
        let future = pg.create_run(&NewRun::default()).await.unwrap();
        pg.update_run_status(
            &future, RelayRunStatus::Paused,
            Some("2999-01-01T00:00:00Z"), Some("weekly cap"),
        ).await.unwrap();

        // Indefinite: paused with NULL paused_until (manual pause) → never auto-resumes.
        let indefinite = pg.create_run(&NewRun::default()).await.unwrap();
        pg.update_run_status(&indefinite, RelayRunStatus::Paused, None, None).await.unwrap();

        let resumed: std::collections::HashSet<uuid::Uuid> =
            pg.resume_due_runs().await.unwrap().into_iter().collect();
        assert!(resumed.contains(&due), "elapsed pause resumes");
        assert!(!resumed.contains(&future), "future pause stays paused");
        assert!(!resumed.contains(&indefinite), "indefinite pause stays paused");

        // The due run is now running with its pause fields cleared.
        let run = pg.get_run(&due).await.unwrap().unwrap();
        assert_eq!(run.status, RelayRunStatus::Running);
        assert!(run.paused_until.is_none(), "paused_until cleared on resume");
        assert!(run.pause_reason.is_none(), "pause_reason cleared on resume");
        // The future run is untouched.
        assert_eq!(pg.get_run(&future).await.unwrap().unwrap().status, RelayRunStatus::Paused);

        // Idempotent: a second call resumes nothing (nothing left due).
        assert!(pg.resume_due_runs().await.unwrap().into_iter().all(|id| id != due));

        for id in [due, future, indefinite] { delete_run(&pg, &id).await; }
    }
}

#[cfg(test)]
mod playbook_tests {
    use super::*;

    #[tokio::test]
    async fn playbook_rules_load_and_run_roundtrip() {
        let Ok(pg) = PgStore::connect_test().await else { return; };

        let rules = pg.list_playbook_rules().await.unwrap();
        assert!(rules.iter().any(|r| r.playbook == "spec_driven"));

        let playbooks = pg.list_playbooks().await.unwrap();
        assert!(playbooks.iter().any(|p| p["name"] == "spec_driven"));

        let guide = pg.list_intake_guide().await.unwrap();
        assert!(guide.iter().any(|g| g["kind"] == "frame"));

        let (proj, _) = pg.get_or_create_project_by_name("_test:playbook_roundtrip").await.unwrap();
        let run_id = pg.insert_playbook_run(
            None, None, "greenfield", "feature", "high",
            None, "spec_driven", "hi", true,
            Some("manual"), false, proj,
        ).await.unwrap();

        let row: (String, String, String, String, bool) = sqlx_core::query_as::query_as(
            "SELECT lifecycle::text, intent::text, risk::text, playbook, confirmed
               FROM sensei.playbook_run WHERE id = $1"
        ).bind(run_id).fetch_one(&pg.pool).await.unwrap();
        assert_eq!(row, ("greenfield".into(), "feature".into(), "high".into(), "spec_driven".into(), true));

        sqlx_core::query::query("DELETE FROM sensei.playbook_run WHERE id = $1")
            .bind(run_id).execute(&pg.pool).await.unwrap();
    }

    #[tokio::test]
    async fn get_playbook_tone() {
        let Ok(pg) = PgStore::connect_test().await else { return; };

        let pb = pg.get_playbook("debug_flow").await.unwrap().unwrap();
        assert_eq!(pb["name"], "debug_flow");
        assert!(!pb["opening_tone"].as_str().unwrap().is_empty());

        assert!(pg.get_playbook("_test:no_such_playbook").await.unwrap().is_none());
    }

    /// A session's nudge gate flips false → true once a *confirmed*
    /// playbook_run is recorded against it — this is the query `hook_nudge`
    /// (api/handlers/sessions.rs) uses to decide whether to suggest
    /// `/sensei:intake`. Mirrors `create_test_folder` from the sibling
    /// `tests` module inline since that helper isn't visible here.
    #[tokio::test]
    async fn session_confirmed_run_gate() {
        let Ok(pg) = PgStore::connect_test().await else { return; };

        pg.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) \
             VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) \
             ON CONFLICT DO NOTHING"
        ).await.unwrap();
        let suffix = format!("nudge_{}", uuid::Uuid::new_v4());
        let abs_path = format!("/_test/{}", suffix);
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path) \
             VALUES('00000000-0000-0000-0000-000000000001', 'git'::sensei.folder_kind, $1, $1, $2) \
             ON CONFLICT(abs_path) DO UPDATE SET name = EXCLUDED.name RETURNING id"
        ).bind(&suffix).bind(&abs_path).fetch_one(&pg.pool).await.unwrap();
        let fid = row.0;

        let sid = pg.create_session(&fid, "intake test", None).await.unwrap();
        assert!(!pg.session_has_confirmed_run(&sid).await.unwrap());

        let (proj, _) = pg.get_or_create_project_by_name("_test:nudge_gate").await.unwrap();
        pg.insert_playbook_run(
            Some(sid), None, "stable", "bug", "low",
            None, "debug_flow", "r", true,
            None, false, proj,
        ).await.unwrap();
        assert!(pg.session_has_confirmed_run(&sid).await.unwrap());
        // clean slate — shared test DB; this combo is also asserted exactly by
        // playbook_combo_trust_counts_ftr
        pg.execute_raw(&format!("delete from sensei.playbook_run where session_id = '{sid}'")).await.ok();
    }

    /// The §9 attribution join: a confirmed playbook_run picks up its session's
    /// outcome/ftr, feeds the per-combo FTR aggregate, and is idempotent (a
    /// second attribution pass touches nothing new). Mirrors the
    /// `session_confirmed_run_gate` folder/session setup above.
    #[tokio::test]
    async fn attribution_and_stats_roundtrip() {
        let Ok(pg) = PgStore::connect_test().await else { return; };

        pg.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) \
             VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) \
             ON CONFLICT DO NOTHING"
        ).await.unwrap();
        let suffix = format!("attrib_{}", uuid::Uuid::new_v4());
        let abs_path = format!("/_test/{}", suffix);
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path) \
             VALUES('00000000-0000-0000-0000-000000000001', 'git'::sensei.folder_kind, $1, $1, $2) \
             ON CONFLICT(abs_path) DO UPDATE SET name = EXCLUDED.name RETURNING id"
        ).bind(&suffix).bind(&abs_path).fetch_one(&pg.pool).await.unwrap();
        let fid = row.0;

        // a confirmed run linked to a session with a known ftr
        let sid = pg.create_session(&fid, "§9 test", None).await.unwrap();
        pg.execute_raw(&format!(
            "update activity.sessions set outcome='completed', ftr=true where id='{sid}'"
        )).await.unwrap();
        let (proj, _) = pg.get_or_create_project_by_name("_test:attrib").await.unwrap();
        pg.insert_playbook_run(
            Some(sid), None, "stable", "bug", "low",
            None, "debug_flow", "r", true, Some("manual"), false, proj,
        ).await.unwrap();

        let n = pg.attribute_playbook_outcomes().await.unwrap();
        assert!(n >= 1);

        let stats = pg.playbook_combo_stats().await.unwrap();
        assert!(stats.iter().any(|s| s.playbook == "debug_flow" && s.n >= 1));

        // idempotent: second attribution touches 0 new
        assert_eq!(pg.attribute_playbook_outcomes().await.unwrap(), 0);
        // clean slate — shared test DB; this combo is also asserted exactly by
        // playbook_combo_trust_counts_ftr
        pg.execute_raw(&format!("delete from sensei.playbook_run where session_id = '{sid}'")).await.ok();
    }

    #[tokio::test]
    async fn apply_learn_plan_reweights_and_upserts() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        pg.execute_raw("delete from sensei.playbook_rules where source='learned'").await.ok(); // clean slate — shared test DB
        let rules = pg.list_playbook_rules().await.unwrap();
        let debug = rules.iter().find(|r| r.playbook == "debug_flow").unwrap();
        use crate::playbook::{LearnPlan, LearnedRule, Lifecycle, Intent, Risk};
        let plan = LearnPlan {
            reweights: vec![(debug.id.unwrap(), debug.base_priority + 5)],
            proposals: vec![LearnedRule { lifecycle: Lifecycle::Stable, intent: Intent::Feature,
                risk: Risk::Low, playbook: "mockup_first".into(), priority: 200, rationale: "t".into() }],
        };
        pg.apply_learn_plan(&plan).await.unwrap();
        let after = pg.list_playbook_rules().await.unwrap();
        assert_eq!(after.iter().find(|r| r.id == debug.id).unwrap().priority, debug.base_priority + 5);
        // proposal is enabled=false → NOT in the resolver-visible list_playbook_rules (which filters WHERE enabled)
        let proposals = pg.list_playbook_rule_proposals().await.unwrap();
        assert!(proposals.iter().any(|p| p["playbook"] == "mockup_first"));
        pg.apply_learn_plan(&plan).await.unwrap();   // idempotent upsert
        assert_eq!(pg.list_playbook_rule_proposals().await.unwrap().iter().filter(|p| p["playbook"]=="mockup_first").count(), 1);
    }

    #[tokio::test]
    async fn accept_flips_proposal_enabled() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        pg.execute_raw("delete from sensei.playbook_rules where source='learned'").await.ok(); // clean slate — shared test DB
        use crate::playbook::{LearnPlan, LearnedRule, Lifecycle, Intent, Risk};
        pg.apply_learn_plan(&LearnPlan { reweights: vec![], proposals: vec![LearnedRule {
            lifecycle: Lifecycle::Greenfield, intent: Intent::Ux, risk: Risk::High,
            playbook: "spec_driven".into(), priority: 205, rationale: "t".into() }] }).await.unwrap();
        let props = pg.list_playbook_rule_proposals().await.unwrap();
        let id = props.iter().find(|p| p["playbook"]=="spec_driven").unwrap()["id"].as_str().unwrap().to_string();
        // A real learned proposal flips → returns true AND persists (visible to the resolver list).
        assert!(pg.accept_playbook_rule(&id.parse().unwrap()).await.unwrap(), "accepting a real learned proposal returns true");
        assert!(pg.list_playbook_rules().await.unwrap().iter().any(|r| r.id == Some(id.parse().unwrap())));
        // A nonexistent id flips NOTHING → returns false (never a fabricated success).
        assert!(!pg.accept_playbook_rule(&uuid::Uuid::new_v4()).await.unwrap(),
            "accepting an unknown id returns false, not a fabricated accept");
    }

    #[tokio::test]
    async fn find_duplicates_scoped_surfaces_same_folder_pairs() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let u = uuid::Uuid::new_v4();
        let pid = pg.create_project(&format!("_dupproj_{u}"), None, None).await.unwrap();
        pg.execute_raw("INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000002','/_dup','_dup','watching'::sensei.watch_status) ON CONFLICT DO NOTHING").await.unwrap();
        let fid = uuid::Uuid::new_v4();
        pg.execute_raw(&format!(
            "INSERT INTO sensei.folders(id, root_id, kind, name, path, abs_path, project_id) VALUES('{fid}','00000000-0000-0000-0000-000000000002','git'::sensei.folder_kind,'_dup_{u}','_dup','/_dup/{u}','{pid}')"
        )).await.unwrap();
        // Two near-identical function nodes in the SAME folder (identical 384-dim
        // embedding → similarity 1.0). The old cross-folder-only predicate hid these.
        let emb = "(select '['||string_agg('0.1',',')||']' from generate_series(1,384))::vector";
        for n in ["_dupfn_a", "_dupfn_b"] {
            pg.execute_raw(&format!(
                "INSERT INTO sensei.nodes(folder_id, kind, name, file_path, line_start, line_end, embedding) \
                 VALUES('{fid}','function'::sensei.node_kind,'{n}','/_dup/{u}/x.rs',1,10,{emb})"
            )).await.unwrap();
        }
        let dups = pg.find_duplicates_scoped(&[fid], 0.9, 50).await.unwrap();
        assert!(dups.iter().any(|d| {
            let names = (d["a"]["name"].as_str(), d["b"]["name"].as_str());
            names == (Some("_dupfn_a"), Some("_dupfn_b")) || names == (Some("_dupfn_b"), Some("_dupfn_a"))
        }), "same-folder near-duplicate functions must surface (regression: the cross-folder-only predicate masked all monorepo dupes)");
    }

    #[tokio::test]
    async fn patterns_for_symbol_matches_by_file_and_is_honest_empty() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let u = uuid::Uuid::new_v4();
        let pid = pg.create_project(&format!("_pfsproj_{u}"), None, None).await.unwrap();
        pg.execute_raw("INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000003','/_pfs','_pfs','watching'::sensei.watch_status) ON CONFLICT DO NOTHING").await.unwrap();
        let fid = uuid::Uuid::new_v4();
        pg.execute_raw(&format!("INSERT INTO sensei.folders(id, root_id, kind, name, path, abs_path, project_id) VALUES('{fid}','00000000-0000-0000-0000-000000000003','git'::sensei.folder_kind,'_pfs_{u}','_pfs','/_pfs/{u}','{pid}')")).await.unwrap();
        // A node 'my_handler' at a repo-RELATIVE path; a project pattern whose instance is its ABSOLUTE form.
        pg.execute_raw(&format!("INSERT INTO sensei.nodes(folder_id, kind, name, file_path, line_start, line_end) VALUES('{fid}','function'::sensei.node_kind,'my_handler','src/routes/x.rs',1,10)")).await.unwrap();
        pg.execute_raw(&format!("INSERT INTO inference.detected_patterns(project_id, name, family, instance_count, instances) VALUES('{pid}','route-handler','route',1,'[{{\"file\":\"/_pfs/{u}/src/routes/x.rs\",\"line\":1}}]'::jsonb)")).await.unwrap();
        // The symbol's file IS in the pattern's instances (abs↔rel reconciled) → match.
        let hit = pg.patterns_for_symbol(&pid, &[fid], "my_handler").await.unwrap();
        assert!(hit.iter().any(|p| p["name"] == "route-handler"),
            "symbol's file matches the pattern instance (was always-null against a nonexistent members field)");
        // A symbol in no pattern → honest empty, never a fabricated null.
        let miss = pg.patterns_for_symbol(&pid, &[fid], "not_a_symbol").await.unwrap();
        assert!(miss.is_empty(), "no file membership → honest empty");
    }

    #[tokio::test]
    async fn playbook_combo_trust_is_project_scoped() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let (proj_a, _) = pg.get_or_create_project_by_name("_test:trust_a").await.unwrap();
        let (proj_b, _) = pg.get_or_create_project_by_name("_test:trust_b").await.unwrap();
        pg.execute_raw("delete from sensei.playbook_run where feature = 'trust-test'").await.ok();
        // project A: two confirmed+attributed runs for (stable,bug,low, debug_flow): one ftr, one not → n=2, ftr=0.5
        for ftr in ["true", "false"] {
            pg.execute_raw(&format!(
                "insert into sensei.playbook_run (feature, lifecycle, intent, risk, playbook, rationale, confirmed, outcome_ftr, project_id) \
                 values ('trust-test','stable','bug','low','debug_flow','t', true, {ftr}, '{proj_a}')")).await.unwrap();
        }
        // project B: one confirmed run, ftr true — must NOT bleed into A's trust
        pg.execute_raw(&format!(
            "insert into sensei.playbook_run (feature, lifecycle, intent, risk, playbook, rationale, confirmed, outcome_ftr, project_id) \
             values ('trust-test','stable','bug','low','debug_flow','t', true, true, '{proj_b}')")).await.unwrap();

        // scoped to A: only A's 2 runs → n=2, ftr=0.5 (B's run excluded — trust is per-project, never global)
        let (na, fa) = pg.playbook_combo_trust("stable","bug","low","debug_flow", &proj_a).await.unwrap();
        assert_eq!(na, 2, "trust must count only the in-scope project's runs");
        assert!((fa - 0.5).abs() < 1e-9);
        // scoped to B: only B's run → n=1, ftr=1.0
        let (nb, fb) = pg.playbook_combo_trust("stable","bug","low","debug_flow", &proj_b).await.unwrap();
        assert_eq!(nb, 1);
        assert!((fb - 1.0).abs() < 1e-9);
        // empty combo in A → (0, 0.0)
        let (n0, f0) = pg.playbook_combo_trust("greenfield","ux","high","vibe", &proj_a).await.unwrap();
        assert_eq!(n0, 0); assert_eq!(f0, 0.0);
        pg.execute_raw("delete from sensei.playbook_run where feature = 'trust-test'").await.ok();
    }

    #[tokio::test]
    async fn model_stats_groups_by_classified_by() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let rows = pg.playbook_model_stats().await.unwrap();
        // shape check: each row has classified_by + n + ftr_rate keys (may be empty on a fresh DB)
        if let Some(r) = rows.first() { assert!(r.get("classified_by").is_some() && r.get("ftr_rate").is_some()); }
    }
}

#[cfg(test)]
mod pack_resolution_tests {
    //! DB-backed: `resolve_local_pack_raws` folds ADOPTED rule-pack rules into the
    //! local governance ladder (D-LOCAL-PACKS) — the offline half of the two-plane
    //! resolution. Proves the field mapping (statement→title, body→content,
    //! rationale→impact, adoption-namespace scope_key→scope, source→namespace),
    //! never-weaken effective
    //! enforcement (an adoption tier LIFTS a weaker rule but never LOWERS a stronger
    //! one), and that an UN-adopted pack governs nothing. Self-skips when the test DB
    //! is absent, like the neighbouring pg_store tests.
    use super::*;

    #[tokio::test]
    async fn adopted_pack_rules_resolve_with_never_weaken() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let pool = pg.pool();

        // Clean any leftovers from a prior aborted run (slug is globally unique;
        // delete cascades the pack's rules + adoptions).
        for slug in ["pack-resolution-test", "pack-unadopted-test"] {
            sqlx_core::query::query("DELETE FROM sensei.rule_packs WHERE slug = $1")
                .bind(slug).execute(pool).await.unwrap();
        }

        // A 'general' scope + namespace: a general/user adoption resolves for ANY folder.
        sqlx_core::query::query(
            "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('general', 'General', 5, false)
             ON CONFLICT (key) DO NOTHING")
            .execute(pool).await.unwrap();
        let ns = pg.upsert_namespace("general", "Bundled", "bundled-test").await.unwrap();

        // Adopted pack: two rules with different default tiers (advisory < required).
        let (pack,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.rule_packs
                (slug, name, area, source, summary, enforcement, owner_namespace_id, status, published_by)
             VALUES ('pack-resolution-test', 'T', 'principles', 'TestSource', 's',
                     'recommended', NULL, 'active', 'test')
             RETURNING id")
            .fetch_one(pool).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_pack_rules(pack_id, ordinal, statement, body, rationale, enforcement)
             VALUES ($1, 1, 'S1', 'B1', 'R1', 'advisory'),
                    ($1, 2, 'S2', 'B2', NULL, 'required')")
            .bind(pack).execute(pool).await.unwrap();

        // Un-adopted pack: its rule must never resolve (a pack governs nothing until adopted).
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_packs
                (slug, name, area, source, summary, enforcement, owner_namespace_id, status, published_by)
             VALUES ('pack-unadopted-test', 'U', 'security', '', 's', 'mandatory', NULL, 'active', 'test')")
            .execute(pool).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_pack_rules(pack_id, ordinal, statement, body, enforcement)
             SELECT id, 1, 'NOPE', 'B', 'mandatory' FROM sensei.rule_packs WHERE slug='pack-unadopted-test'")
            .execute(pool).await.unwrap();

        // Adopt the first pack at the general namespace with a 'recommended' override.
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_pack_adoptions(pack_id, namespace_id, pinned_version, enforcement, adopted_by)
             VALUES ($1, $2, 1, 'recommended', 'test')")
            .bind(pack).bind(ns).execute(pool).await.unwrap();

        // Resolve for a folder with NO folder_namespaces — only the general clause matches.
        let raws = pg.resolve_local_pack_raws(Some(&uuid::Uuid::new_v4())).await.unwrap();

        let mine: Vec<_> = raws.iter().filter(|r| r.title == "S1" || r.title == "S2").collect();
        assert_eq!(mine.len(), 2, "both adopted-pack rules resolve");
        assert!(!raws.iter().any(|r| r.title == "NOPE"), "an un-adopted pack governs nothing");

        let r1 = raws.iter().find(|r| r.title == "S1").unwrap();
        assert_eq!(r1.content, "B1", "body → content");
        assert_eq!(r1.impact.as_deref(), Some("R1"), "rationale → impact");
        // scope = the GOVERNANCE scope the pack was ADOPTED at (this pack is adopted
        // at the 'general' namespace), NOT the pack's own area/category ('principles').
        // The constitution ladder groups by governance scope, so a rule must carry the
        // scope it entered at — mirrors `resolve_rules_raw` (memories use n.scope_key).
        assert_eq!(r1.scope, "general", "adoption namespace scope_key → scope (not pack area)");
        assert_eq!(r1.namespace.as_deref(), Some("TestSource"), "source → namespace");
        assert_eq!(r1.enforcement, "recommended",
            "an advisory rule is LIFTED to the stronger 'recommended' adoption tier");

        let r2 = raws.iter().find(|r| r.title == "S2").unwrap();
        assert_eq!(r2.enforcement, "required",
            "a 'required' rule is NOT weakened by the lower 'recommended' adoption tier");
        assert_eq!(r2.impact, None, "NULL rationale → None impact");

        // Cleanup (pack delete cascades rules + adoption; then this test's
        // namespace). The shared 'general' scope is left in place — other bundled
        // packs (e.g. the constitution seed) may adopt at it concurrently, so
        // deleting it would FK-fail; in the throwaway test DB a stray scope row
        // is harmless.
        for slug in ["pack-resolution-test", "pack-unadopted-test"] {
            sqlx_core::query::query("DELETE FROM sensei.rule_packs WHERE slug = $1")
                .bind(slug).execute(pool).await.unwrap();
        }
        sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE id = $1").bind(ns).execute(pool).await.unwrap();
    }

    #[tokio::test]
    async fn default_constitution_seed_adopts_offline_but_not_stack_templates() {
        // D-SEED: seed_default_constitution() bundles the constitution as packs
        // and AUTO-ADOPTS the three constitution packs at the general namespace,
        // so a fresh install resolves them offline. The stack-templates pack is
        // seeded but NOT adopted (opt-in per stack).
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let pool = pg.pool();

        // The proc guards on the always-on 'general' scope (seeded by import_scopes
        // in prod); provide it here. Left in place on cleanup (shared).
        sqlx_core::query::query(
            "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('general', 'General', 0, false) ON CONFLICT (key) DO NOTHING")
            .execute(pool).await.unwrap();

        // Fresh from this procedure's definition (idempotent — run twice).
        sqlx_core::query::query("CALL sensei.seed_default_constitution()").execute(pool).await.unwrap();
        sqlx_core::query::query("CALL sensei.seed_default_constitution()").execute(pool).await.unwrap();

        // Four packs; the three constitution packs adopted, stack-templates not.
        let (adopted,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.rule_pack_adoptions a
               JOIN sensei.rule_packs p ON p.id = a.pack_id
               JOIN sensei.namespaces n ON n.id = a.namespace_id
              WHERE n.scope_key='general' AND n.slug='global-dojo'
                AND p.slug IN ('default-principles','default-architecture','default-process')")
            .fetch_one(pool).await.unwrap();
        assert_eq!(adopted, 3, "three constitution packs adopted at general (idempotent — no dup)");

        let raws = pg.resolve_local_pack_raws(Some(&uuid::Uuid::new_v4())).await.unwrap();

        // A mandatory principle resolves, scoped by the ADOPTION (the three
        // constitution packs auto-adopt at the 'general' namespace), NOT the pack area.
        let measure = raws.iter().find(|r| r.title == "Measure, then keep what helps")
            .expect("constitution principle resolves offline");
        assert_eq!(measure.enforcement, "mandatory");
        assert_eq!(measure.scope, "general", "adopted at general → scope 'general' (not pack area)");

        // The 21 adopted constitution rules resolve (4 + 5 + 12); stack templates do not.
        // All three packs are adopted at the SAME 'general' namespace, so every rule
        // now carries scope 'general' (the adoption scope) regardless of its pack area.
        let constitution = raws.iter()
            .filter(|r| r.scope == "general")
            .filter(|r| r.namespace.as_deref() == Some("sensei default constitution (DORA · XP/CD · Core Protocols)"))
            .count();
        assert_eq!(constitution, 21, "all constitution rules resolve at the adoption scope (idempotent re-seed did not duplicate)");
        assert!(!raws.iter().any(|r| r.title.contains("[stack:")),
            "stack-templates is seeded but NOT adopted — its rules must not resolve");

        // Cleanup: delete the four packs (cascade rules + adoptions) + the seeded
        // namespace. Leave the shared 'general' scope.
        sqlx_core::query::query(
            "DELETE FROM sensei.rule_packs
              WHERE owner_namespace_id IS NULL
                AND slug IN ('default-principles','default-architecture','default-process','stack-templates')")
            .execute(pool).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE scope_key='general' AND slug='global-dojo'")
            .execute(pool).await.unwrap();
    }

    #[tokio::test]
    async fn resolve_local_checker_rules_returns_only_checker_backed_rules() {
        // D-CHECKER: resolve_local_checker_rules surfaces ONLY adopted rules with
        // verification='checker' + a checker_ref — a 'review' rule in the same pack
        // must not appear. Uses a general adoption so a random folder resolves it.
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM sensei.rule_packs WHERE slug = 'checker-resolve-test'")
            .execute(pool).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('general', 'General', 0, false) ON CONFLICT (key) DO NOTHING")
            .execute(pool).await.unwrap();
        let ns = pg.upsert_namespace("general", "Bundled", "checker-ns-test").await.unwrap();
        let (pack,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.rule_packs
                (slug, name, area, source, summary, enforcement, owner_namespace_id, status, published_by)
             VALUES ('checker-resolve-test', 'C', 'tech_stack', 's', 's', 'advisory', NULL, 'active', 'test')
             RETURNING id")
            .fetch_one(pool).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_pack_rules(pack_id, ordinal, statement, body, enforcement, verification, checker_ref)
             VALUES ($1, 1, 'run the linter', 'B', 'advisory', 'checker', 'lint'),
                    ($1, 2, 'a manual rule',  'B', 'advisory', 'review',  NULL)")
            .bind(pack).execute(pool).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_pack_adoptions(pack_id, namespace_id, pinned_version, adopted_by)
             VALUES ($1, $2, 1, 'test')")
            .bind(pack).bind(ns).execute(pool).await.unwrap();

        let rules = pg.resolve_local_checker_rules(&uuid::Uuid::new_v4()).await.unwrap();
        let mine: Vec<_> = rules.iter().filter(|(s, _)| s == "run the linter" || s == "a manual rule").collect();
        assert_eq!(mine.len(), 1, "only the checker-backed rule resolves, not the review rule");
        assert_eq!(mine[0], &("run the linter".to_string(), "lint".to_string()));

        sqlx_core::query::query("DELETE FROM sensei.rule_packs WHERE slug = 'checker-resolve-test'")
            .execute(pool).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE id = $1").bind(ns).execute(pool).await.unwrap();
    }
}
