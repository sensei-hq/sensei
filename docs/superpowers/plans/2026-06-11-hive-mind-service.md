# Hive-mind Service (`sensei-hive`) Implementation Plan — Governance P4 / #25

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `sensei-hive` — a slim, self-hosted org "shared brain": an Axum REST service over an embedded Postgres whose schema is the `hive`-scoped subset of the one DDL tree, with API-key auth + RBAC and an audit log. This is issue **#25**, the standalone (no-daemon-dependency) first slice of federation.

**Architecture:** A new monorepo binary crate `crates/hive-mind/` (binary `sensei-hive`) + a shared wire-types crate `crates/hive-protocol/`. The service owns an embedded Postgres (`postgresql_embedded`) bootstrapped via `dbd-core` with a new `hive` scope, exposes the `/v1` REST protocol from the design spec, authenticates bearer API keys (hash-at-rest, constant-time), and enforces three roles (`member`/`publisher`/`admin`). No code-graph / scanner / session machinery.

**Tech Stack:** Rust 2024, Axum 0.8, sqlx 0.8 (`sqlx-core` + `sqlx-postgres`), `dbd-core` v0.4.4, `postgresql_embedded`, `tower-http` (CORS), `subtle` (constant-time), `rand`, `sha2`, `clap`, `serde`, `tracing`.

**Spec:** [`docs/superpowers/specs/2026-06-11-hive-mind-federation-design.md`](../specs/2026-06-11-hive-mind-federation-design.md) (§4, §6, §7, §8, §12, §13).

---

## File Structure

**New crates:**
- `crates/hive-protocol/` — wire types + content-hash. Depended on by `hive-mind` now and `senseid` in #26. No heavy deps (serde, sha2). One responsibility: *the federation wire contract*.
  - `Cargo.toml`, `src/lib.rs`
- `crates/hive-mind/` — the service binary. One module per responsibility:
  - `src/main.rs` — config, boot embedded PG + deploy, build router, serve.
  - `src/config.rs` — `HiveConfig` (data dir, port, bind, optional TLS).
  - `src/db.rs` — `hive_db`: embedded-PG lifecycle + `dbd-core` deploy + scopes seed → `sqlx` pool.
  - `src/store.rs` — `HiveStore`: all SQL (rules publish/pull/retract, namespaces, members, keys, audit).
  - `src/auth.rs` — bearer extraction, key hashing, role-floor middleware.
  - `src/api.rs` — Axum router + handlers for `/v1/*`.
  - `src/keygen.rs` — `keygen` CLI subcommand (bootstrap admin key).

**New DDL (in the single tree under `database/`):**
- `database/ddl/table/hive/shared_rules.ddl`
- `database/ddl/table/hive/members.ddl`
- `database/ddl/table/hive/api_keys.ddl`
- `database/ddl/table/hive/audit_log.ddl`
- `database/design.hive.yaml` — hive manifest (no-`vector` target, `hive` scope), reusing `database/ddl/`.

**Modified:**
- `Cargo.toml` (root) — add the two crates to `members`.
- `crates/bootstrap/Cargo.toml` — bump `dbd-core` tag `v0.4.1` → `v0.4.4`.
- `crates/bootstrap/src/database.rs:137,150` — add the new `scope` arg (`None`) to `apply`/`import_data`.
- `database/design.yaml` — add `skip_schemas: [hive]` to the target so the daemon never materializes hive tables.

---

## Task 1: Bump `dbd-core` to v0.4.4 and fix the daemon's call sites

`dbd-core` v0.4.4 adds group-scopes (`scopes:` with include/exclude) — required by the hive — but inserts a new `scope: Option<&ResolvedScope>` 4th argument into `Design::apply` and `Design::import_data`. Bumping the dep breaks the daemon's two existing calls; fix them first so `develop` stays green.

**Files:**
- Modify: `crates/bootstrap/Cargo.toml`
- Modify: `crates/bootstrap/src/database.rs:137-147` and `:150-160`

- [ ] **Step 1: Bump the dependency**

In `crates/bootstrap/Cargo.toml`, change the `dbd-core` line:

```toml
dbd-core = { git = "https://github.com/sensei-hq/dbd", tag = "v0.4.4" }
```

- [ ] **Step 2: Run the build to see the breakage (the "failing test")**

Run: `cargo build -p sensei-bootstrap`
Expected: FAIL — `this method takes 7 arguments but 6 arguments were supplied` (or similar) at the `design.apply(...)` and `design.import_data(...)` calls.

- [ ] **Step 3: Fix the `apply` call**

In `crates/bootstrap/src/database.rs`, the `design.apply(...)` call (~line 137). Insert `None` as the new 4th argument (after `false`):

```rust
        design.apply(
            &adapter,
            None,           // entity-name filter
            false,          // dry_run
            None,           // scope: Option<&ResolvedScope> — daemon applies the full schema
            |desc: &str| tracing::debug!(dbd_step = "apply", desc, "starting"),
            |desc: &str, err: Option<&str>| match err {
                Some(e) => tracing::warn!(dbd_step = "apply", desc, error = e, "failed"),
                None    => tracing::debug!(dbd_step = "apply", desc, "done"),
            },
            |_summary| tracing::info!("dbd apply complete"),
        ).await.map_err(|e| format!("dbd apply failed: {e}"))?;
```

- [ ] **Step 4: Fix the `import_data` call**

The `design.import_data(...)` call (~line 150). Insert `None` as the new 4th argument the same way:

```rust
        design.import_data(
            &adapter,
            None,
            false,
            None,           // scope
            |desc: &str| tracing::debug!(dbd_step = "import", desc, "starting"),
            |desc: &str, err: Option<&str>| match err {
                Some(e) => tracing::warn!(dbd_step = "import", desc, error = e, "failed"),
                None    => tracing::debug!(dbd_step = "import", desc, "done"),
            },
            |_summary| tracing::info!("dbd import complete"),
        ).await.map_err(|e| format!("dbd import failed: {e}"))?;
```

- [ ] **Step 5: Verify build + existing tests pass**

Run: `cargo build -p sensei-bootstrap && cargo test -p sensei-bootstrap`
Expected: PASS — 152 tests pass (the existing bootstrap suite), no compile errors.

- [ ] **Step 6: Commit**

```bash
git add crates/bootstrap/Cargo.toml crates/bootstrap/src/database.rs
git commit -m "chore(dbd): bump dbd-core v0.4.1→v0.4.4 (group-scopes); add scope arg to apply/import_data"
```

---

## Task 2: `hive-protocol` crate — content hashing + wire types

The dedup key `content_hash` must be computed identically on the daemon and the hive, so it lives in the shared crate. The hash matches the daemon's existing dedup normalization (`content.trim().to_lowercase()`, `crates/senseid/src/governance.rs:59`).

**Files:**
- Create: `crates/hive-protocol/Cargo.toml`
- Create: `crates/hive-protocol/src/lib.rs`
- Modify: `Cargo.toml` (root) — add to `members`

- [ ] **Step 1: Scaffold the crate**

Create `crates/hive-protocol/Cargo.toml`:

```toml
[package]
name = "hive-protocol"
version = "0.2.17"
edition = "2024"
description = "Shared wire types + content hashing for sensei hive-mind federation"
license = "MIT"

[dependencies]
serde = { version = "1", features = ["derive"] }
sha2 = "0.10"
```

Add `"crates/hive-protocol",` to the `members` array in the root `Cargo.toml`.

- [ ] **Step 2: Write the failing test**

Create `crates/hive-protocol/src/lib.rs` with the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_matches_governance_normalization() {
        // governance.rs dedups on `content.trim().to_lowercase()`.
        // Same logical content (differing only by surrounding ws / case) → same hash.
        let a = content_hash("  Use TDD always.  ");
        let b = content_hash("use tdd always.");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64); // sha256 hex
    }

    #[test]
    fn content_hash_differs_for_different_content() {
        assert_ne!(content_hash("rule one"), content_hash("rule two"));
    }

    #[test]
    fn published_rule_round_trips() {
        let r = PublishedRule {
            content_hash: content_hash("x"),
            scope_key: "organization".into(),
            namespace_slug: "sensei-hq".into(),
            namespace_name: "Sensei HQ".into(),
            rule_type: "convention".into(),
            title: "t".into(),
            content: "x".into(),
            impact: None,
            enforcement: "mandatory".into(),
            origin_repo: Some("sensei/daemon".into()),
            published_by: "jerry".into(),
            published_at: "2026-06-11T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: PublishedRule = serde_json::from_str(&json).unwrap();
        assert_eq!(back.namespace_slug, "sensei-hq");
        assert_eq!(back.enforcement, "mandatory");
    }
}
```

Add `serde_json = "1"` under `[dev-dependencies]` in `crates/hive-protocol/Cargo.toml`.

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p hive-protocol`
Expected: FAIL — `cannot find function content_hash` / `cannot find type PublishedRule`.

- [ ] **Step 4: Implement the types + hashing**

Prepend to `crates/hive-protocol/src/lib.rs` (above the test module):

```rust
//! Shared wire contract for sensei hive-mind federation.
//! `content_hash` MUST stay in lockstep with the daemon's dedup
//! normalization in `senseid/src/governance.rs` (trim + lowercase).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Normalize rule content for dedup/identity: trim + lowercase.
/// Mirrors `governance::structure_ruleset`'s dedup key.
pub fn normalize_content(content: &str) -> String {
    content.trim().to_lowercase()
}

/// Stable dedup key for a rule's content (sha256 hex of the normalized form).
pub fn content_hash(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(normalize_content(content).as_bytes());
    format!("{:x}", h.finalize())
}

/// A rule published to the hive — a flattened snapshot (no memory graph).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedRule {
    pub content_hash: String,
    pub scope_key: String,
    pub namespace_slug: String,
    pub namespace_name: String,
    pub rule_type: String,
    pub title: String,
    pub content: String,
    pub impact: Option<String>,
    pub enforcement: String,
    pub origin_repo: Option<String>,
    pub published_by: String,
    pub published_at: String,
}

/// Response to a publish: the canonical identity assigned by the hive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResponse {
    pub id: String,
    pub version: i32,
    pub seq: i64,
}

/// A rule as returned by a pull (snapshot + hive identity + lifecycle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulledRule {
    pub id: String,
    pub seq: i64,
    pub status: String, // "active" | "tombstoned"
    pub version: i32,
    #[serde(flatten)]
    pub rule: PublishedRule,
}

/// Response to a pull: deltas + the new cursor to persist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullResponse {
    pub rules: Vec<PulledRule>,
    pub cursor: i64,
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p hive-protocol`
Expected: PASS — 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/hive-protocol Cargo.toml
git commit -m "feat(hive-protocol): wire types + content_hash (governance-parity normalization)"
```

---

## Task 3: Hive DDL, manifest, and the `hive` scope

Author the four hive-only tables in the shared DDL tree, a hive manifest (`design.hive.yaml`) that selects them via a `hive` scope and a `vector`-free target, and keep the daemon from materializing them by adding `skip_schemas: [hive]` to the main manifest.

**Files:**
- Create: `database/ddl/table/hive/shared_rules.ddl`, `members.ddl`, `api_keys.ddl`, `audit_log.ddl`
- Create: `database/design.hive.yaml`
- Modify: `database/design.yaml`
- Create: `crates/hive-mind/Cargo.toml`, `crates/hive-mind/src/main.rs` (stub) + `crates/hive-mind/tests/scope_test.rs`
- Modify: `Cargo.toml` (root) — add `crates/hive-mind` to `members`

- [ ] **Step 1: Write the four DDL files**

`database/ddl/table/hive/shared_rules.ddl`:

```sql
set search_path to hive, sensei, extensions;

create sequence if not exists hive.shared_rules_seq;

create table if not exists hive.shared_rules (
  id            uuid        primary key default gen_random_uuid()
, seq           bigint      not null default nextval('hive.shared_rules_seq')
, namespace_id  uuid        not null references sensei.namespaces(id)
, content_hash  text        not null
, rule_type     text        not null
, title         text        not null
, content       text        not null
, impact        text
, enforcement   enforcement not null
, status        text        not null default 'active'    -- active | tombstoned
, version       integer     not null default 1
, origin_repo   text
, published_by  text        not null
, published_at  timestamptz not null
, updated_at    timestamptz not null default now()
, constraint shared_rules_ns_content unique (namespace_id, content_hash)
);

create index if not exists shared_rules_seq_idx on hive.shared_rules(seq);

comment on table hive.shared_rules is
'Published-rule registry for the hive-mind. A flattened snapshot of a promoted
rule (no memory graph). seq is a monotonic cursor advanced on every insert,
republish, and tombstone (the store sets seq = nextval on every write — bigserial
alone would only fire on insert). Self-contained: no FK to projects/folders/sessions.';
```

`database/ddl/table/hive/members.ddl`:

```sql
set search_path to hive, sensei, extensions;

create table if not exists hive.members (
  id           uuid        primary key default gen_random_uuid()
, name         text        not null
, email        text
, role         text        not null default 'member'    -- member | publisher | admin
, disabled_at  timestamptz
, created_at   timestamptz not null default now()
);

comment on table hive.members is
'A federation participant. role gates the REST API: member=pull, publisher=pull+publish,
admin=publisher+manage members/keys/namespaces+audit. Instance-global roles (one hive
instance == one org); per-namespace ACLs are a deferred extension.';
```

`database/ddl/table/hive/api_keys.ddl`:

```sql
set search_path to hive, sensei, extensions;

create table if not exists hive.api_keys (
  id           uuid        primary key default gen_random_uuid()
, member_id    uuid        not null references hive.members(id)
, key_hash     text        not null
, label        text
, last_used_at timestamptz
, revoked_at   timestamptz
, created_at   timestamptz not null default now()
);

create index if not exists api_keys_key_hash_idx on hive.api_keys(key_hash);

comment on table hive.api_keys is
'Bearer API keys. Only the sha256 hash is stored; the plaintext is shown once at
issue. Lookups compare hashes (the compared value is itself a hash, so timing
leaks nothing about the key). revoked_at/disabled_at gate validity.';
```

`database/ddl/table/hive/audit_log.ddl`:

```sql
set search_path to hive, sensei, extensions;

create table if not exists hive.audit_log (
  id         bigserial    primary key
, ts         timestamptz  not null default now()
, member_id  uuid         references hive.members(id)
, action     text         not null   -- publish | retract | key.issue | key.revoke | member.add
, target     text
, detail     jsonb        not null default '{}'
);

comment on table hive.audit_log is
'Append-only audit of mutating API actions, stamped by the auth middleware.';
```

- [ ] **Step 2: Write the hive manifest**

`database/design.hive.yaml` (reuses `database/ddl/`; no `vector` extension since no hive table uses it and `gen_random_uuid()` is core Postgres):

```yaml
project:
  name: sensei-hive
  note: Hive-mind shared-brain schema — the hive-scoped subset of the sensei DDL tree.
source:
  dialect: postgresql
target:
  postgres:
    url: $DATABASE_URL
    extensions: []          # no pgvector; gen_random_uuid() is core PG
schemas:
  - extensions
  - sensei
  - hive
scopes:
  hive:
    includes:
      - hive.shared_rules
      - hive.members
      - hive.api_keys
      - hive.audit_log
    deps: include           # auto-expand closure → sensei.namespaces, sensei.scopes, sensei.enforcement
ignore: []
```

- [ ] **Step 3: Keep the daemon from materializing hive tables**

In `database/design.yaml`, add `skip_schemas` to the existing target so the daemon's full deploy never loads `hive.*` entities. Change the `target:` block to:

```yaml
target:
  postgres:
    url: $DATABASE_URL
    skip_schemas:
      - hive
    extensions:
      - name: uuid-ossp
        schema: extensions
      - name: vector
        schema: extensions
```

- [ ] **Step 4: Scaffold the hive-mind crate + write the failing scope test**

Create `crates/hive-mind/Cargo.toml`:

```toml
[package]
name = "hive-mind"
version = "0.2.17"
edition = "2024"
description = "sensei-hive — the org shared-brain federation service"
license = "MIT"

[[bin]]
name = "sensei-hive"
path = "src/main.rs"

[dependencies]
hive-protocol = { path = "../hive-protocol" }
dbd-core = { git = "https://github.com/sensei-hq/dbd", tag = "v0.4.4" }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "fs", "sync"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"

[dev-dependencies]
tempfile = "3"
```

Create `crates/hive-mind/src/main.rs` as a stub:

```rust
fn main() {
    println!("sensei-hive");
}
```

Add `"crates/hive-mind",` to the root `Cargo.toml` `members`.

Create `crates/hive-mind/tests/scope_test.rs`:

```rust
//! Verifies the `hive` scope in design.hive.yaml resolves to exactly the
//! intended entity set — the 4 hive tables + the closure of shared governance
//! tables (namespaces, scopes, enforcement) — and NOTHING daemon-only.

use std::path::PathBuf;

fn database_dir() -> PathBuf {
    // crates/hive-mind/tests/ -> ../../../database
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database")
}

#[test]
fn hive_scope_resolves_to_expected_entities() {
    let dir = database_dir();
    let cfg = dir.join("design.hive.yaml");
    let design = dbd_core::Design::from_config_with_dir(&cfg, "prod", Some(&dir))
        .expect("load design.hive.yaml");
    let scope = design.resolve_scope(Some("hive"), None).expect("resolve hive scope");
    let entities = design.scoped_entities(&scope).expect("scoped entities");

    let names: std::collections::HashSet<String> =
        entities.iter().map(|e| e.name.clone()).collect();

    // Hive-only tables present:
    for n in ["hive.shared_rules", "hive.members", "hive.api_keys", "hive.audit_log"] {
        assert!(names.contains(n), "expected {n} in hive scope; got {names:?}");
    }
    // Shared governance closure present:
    for n in ["sensei.namespaces", "sensei.scopes", "sensei.enforcement"] {
        assert!(names.contains(n), "expected {n} (closure) in hive scope; got {names:?}");
    }
    // Daemon-only tables absent:
    for n in ["sensei.memories", "sensei.nodes", "sensei.folder_namespaces"] {
        assert!(!names.contains(n), "{n} must NOT be in the hive scope");
    }
}
```

- [ ] **Step 5: Run the test to verify it fails, then passes**

Run: `cargo test -p hive-mind --test scope_test`
Expected first run: FAIL if entity names are schema-qualified differently than assumed. If it fails on the name format (e.g. names are bare `shared_rules` not `hive.shared_rules`), inspect one entity: temporarily add `eprintln!("{:?}", names);` and adjust the assertions to the actual `Entity::name` format dbd produces for `ddl/table/<schema>/<name>.ddl`. Then re-run.
Expected after alignment: PASS.

> Note for the implementer: dbd derives entity names from the path `ddl/<type>/<schema>/<name>.ddl`. Confirm whether `Entity::name` is `"<schema>.<name>"` or bare `"<name>"` by reading `/tmp/dbd-044/crates/dbd-core/src/parser.rs` (or printing once), and make the test assertions match reality. This is the only name-format unknown in the plan.

- [ ] **Step 6: Commit**

```bash
git add database/ddl/table/hive database/design.hive.yaml database/design.yaml crates/hive-mind Cargo.toml
git commit -m "feat(hive): hive DDL (shared_rules/members/api_keys/audit_log) + design.hive.yaml scope + daemon skip_schemas"
```

---

## Task 4: Embedded Postgres bootstrap (`hive_db`)

The hive owns its database. `db.rs` starts an embedded Postgres, deploys the `hive` scope via `dbd-core`, seeds the `scopes` ladder from the canonical `scopes.jsonl`, and returns a `sqlx` pool.

**Files:**
- Modify: `crates/hive-mind/Cargo.toml` (add deps)
- Create: `crates/hive-mind/src/db.rs`
- Create: `crates/hive-mind/tests/db_test.rs`

- [ ] **Step 1: Add dependencies**

In `crates/hive-mind/Cargo.toml` `[dependencies]` add:

```toml
sqlx-core = { version = "0.8", features = ["_rt-tokio", "chrono", "uuid", "json"] }
sqlx-postgres = { version = "0.8", features = ["chrono", "uuid", "json"] }
postgresql_embedded = "0.18"
thiserror = "2"
```

> Implementer note: confirm the exact `postgresql_embedded` 0.18 API on docs.rs before coding Step 3 — specifically `Settings` field names and whether `setup()`/`start()`/`create_database()`/`database_exists()` are the current method names. The shape below matches 0.17–0.18; adjust field/method names if the pinned minor differs.

- [ ] **Step 2: Write the failing test**

Create `crates/hive-mind/tests/db_test.rs`:

```rust
use hive_mind::db::HiveDb;

#[tokio::test]
async fn bootstrap_creates_hive_schema_seeds_scopes_and_excludes_daemon_tables() {
    let db = HiveDb::bootstrap_temp().await.expect("bootstrap embedded hive");
    let pool = db.pool();

    // hive tables exist
    let (rules_exists,): (bool,) = sqlx_core::query_as::query_as(
        "SELECT to_regclass('hive.shared_rules') IS NOT NULL")
        .fetch_one(pool).await.unwrap();
    assert!(rules_exists, "hive.shared_rules should exist");

    // scopes ladder seeded (8 rows from scopes.jsonl)
    let (n,): (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.scopes")
        .fetch_one(pool).await.unwrap();
    assert_eq!(n, 8, "scopes ladder should be seeded");

    // daemon-only table absent
    let (memories_exists,): (bool,) = sqlx_core::query_as::query_as(
        "SELECT to_regclass('sensei.memories') IS NOT NULL")
        .fetch_one(pool).await.unwrap();
    assert!(!memories_exists, "daemon-only sensei.memories must NOT exist in the hive DB");
}
```

- [ ] **Step 3: Implement `db.rs`**

Create `crates/hive-mind/src/db.rs`:

```rust
//! Embedded-Postgres lifecycle + schema deploy for the hive.

use std::path::{Path, PathBuf};
use postgresql_embedded::{PostgreSQL, Settings};
use sqlx_postgres::{PgPool, PgPoolOptions};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("embedded postgres: {0}")]
    Embedded(String),
    #[error("dbd deploy: {0}")]
    Deploy(String),
    #[error("seed: {0}")]
    Seed(String),
    #[error("pool: {0}")]
    Pool(String),
}

/// A running embedded Postgres with the hive schema applied + scopes seeded.
pub struct HiveDb {
    _pg: PostgreSQL,   // owns the process; dropped on shutdown
    pool: PgPool,
}

impl HiveDb {
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Production bootstrap: embedded PG rooted at `data_dir`, schema `database_dir`.
    pub async fn bootstrap(data_dir: PathBuf, database_dir: PathBuf) -> Result<Self, DbError> {
        let settings = Settings {
            data_dir,
            temporary: false,
            ..Default::default()
        };
        let mut pg = PostgreSQL::new(settings);
        pg.setup().await.map_err(|e| DbError::Embedded(e.to_string()))?;
        pg.start().await.map_err(|e| DbError::Embedded(e.to_string()))?;
        if !pg.database_exists("hive").await.map_err(|e| DbError::Embedded(e.to_string()))? {
            pg.create_database("hive").await.map_err(|e| DbError::Embedded(e.to_string()))?;
        }
        let url = pg.settings().url("hive");

        deploy_hive_schema(&url, &database_dir).await?;

        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&url)
            .await
            .map_err(|e| DbError::Pool(e.to_string()))?;

        seed_scopes(&pool, &database_dir).await?;

        Ok(Self { _pg: pg, pool })
    }

    /// Test bootstrap: a throwaway embedded PG under a temp dir.
    pub async fn bootstrap_temp() -> Result<Self, DbError> {
        let tmp = std::env::temp_dir().join(format!("sensei-hive-test-{}", std::process::id()));
        let database_dir = workspace_database_dir();
        Self::bootstrap(tmp, database_dir).await
    }
}

/// `crates/hive-mind/ -> ../../database`
fn workspace_database_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database")
}

async fn deploy_hive_schema(db_url: &str, database_dir: &Path) -> Result<(), DbError> {
    use dbd_core::adapter::postgres::PostgresAdapter;
    use dbd_core::Design;

    let cfg = database_dir.join("design.hive.yaml");
    let design = Design::from_config_with_dir(&cfg, "prod", Some(database_dir))
        .map_err(|e| DbError::Deploy(format!("config load: {e}")))?;
    let scope = design
        .resolve_scope(Some("hive"), None)
        .map_err(|e| DbError::Deploy(format!("resolve scope: {e}")))?;
    let adapter = PostgresAdapter::new(db_url, "hive")
        .await
        .map_err(|e| DbError::Deploy(format!("connect: {e}")))?;

    design
        .apply(
            &adapter,
            None,
            false,
            Some(&scope),
            |_| {},
            |desc: &str, err: Option<&str>| {
                if let Some(e) = err {
                    tracing::warn!(dbd_step = "apply", desc, error = e, "failed");
                }
            },
            |_| tracing::info!("hive schema applied"),
        )
        .await
        .map_err(|e| DbError::Deploy(format!("apply: {e}")))?;
    Ok(())
}

/// Seed the scope ladder directly from the canonical scopes.jsonl (DRY — same
/// data file the daemon imports, without dragging in the staging machinery).
async fn seed_scopes(pool: &PgPool, database_dir: &Path) -> Result<(), DbError> {
    #[derive(serde::Deserialize)]
    struct ScopeRow {
        key: String,
        name: String,
        level: i32,
        shareable: bool,
        description: Option<String>,
    }
    let path = database_dir.join("import/staging/scopes.jsonl");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| DbError::Seed(format!("read {}: {e}", path.display())))?;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let r: ScopeRow = serde_json::from_str(line)
            .map_err(|e| DbError::Seed(format!("parse: {e}")))?;
        sqlx_core::query::query(
            "INSERT INTO sensei.scopes(key, name, level, shareable, description)
             VALUES($1,$2,$3,$4,$5)
             ON CONFLICT(key) DO UPDATE SET
               name=EXCLUDED.name, level=EXCLUDED.level,
               shareable=EXCLUDED.shareable, description=EXCLUDED.description",
        )
        .bind(&r.key).bind(&r.name).bind(r.level).bind(r.shareable).bind(&r.description)
        .execute(pool).await
        .map_err(|e| DbError::Seed(e.to_string()))?;
    }
    Ok(())
}
```

Add to `crates/hive-mind/src/main.rs` so the test crate can see the module:

```rust
pub mod db;

fn main() {
    println!("sensei-hive");
}
```

> Implementer note: a `[[bin]]`-only crate isn't importable as `hive_mind::db` from an integration test. Add a `src/lib.rs` exposing the modules (`pub mod db; pub mod store; ...`) and have `main.rs` use the lib crate. Create `crates/hive-mind/src/lib.rs` with `pub mod db;` and change `main.rs` to `use hive_mind::...`. Add `[lib]` name `hive_mind` implicitly (default). Do this now so later tasks' modules are testable.

- [ ] **Step 4: Run the test**

Run: `cargo test -p hive-mind --test db_test`
Expected: PASS. First run downloads the embedded PG binary (cached afterward) — allow extra time.

- [ ] **Step 5: Commit**

```bash
git add crates/hive-mind
git commit -m "feat(hive): embedded Postgres bootstrap — dbd hive-scope deploy + scopes seed"
```

---

## Task 5: `HiveStore` — namespace upsert + publish

**Files:**
- Create: `crates/hive-mind/src/store.rs`
- Modify: `crates/hive-mind/src/lib.rs` (add `pub mod store;`)
- Create: `crates/hive-mind/tests/store_publish_test.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/hive-mind/tests/store_publish_test.rs`:

```rust
use hive_mind::db::HiveDb;
use hive_mind::store::HiveStore;
use hive_protocol::{content_hash, PublishedRule};

fn rule(content: &str, title: &str) -> PublishedRule {
    PublishedRule {
        content_hash: content_hash(content),
        scope_key: "organization".into(),
        namespace_slug: "sensei-hq".into(),
        namespace_name: "Sensei HQ".into(),
        rule_type: "convention".into(),
        title: title.into(),
        content: content.into(),
        impact: None,
        enforcement: "mandatory".into(),
        origin_repo: Some("sensei/daemon".into()),
        published_by: "jerry".into(),
        published_at: "2026-06-11T00:00:00Z".into(),
    }
}

#[tokio::test]
async fn publish_creates_then_republish_bumps_version_and_seq() {
    let db = HiveDb::bootstrap_temp().await.unwrap();
    let store = HiveStore::new(db.pool().clone());

    let r1 = store.publish(&rule("always use tdd", "TDD")).await.unwrap();
    assert_eq!(r1.version, 1);

    // Same content_hash, changed title → upsert bumps version + advances seq.
    let mut again = rule("always use tdd", "TDD (revised)");
    again.content_hash = content_hash("always use tdd");
    let r2 = store.publish(&again).await.unwrap();
    assert_eq!(r2.id, r1.id, "same (namespace, content_hash) → same row");
    assert_eq!(r2.version, 2);
    assert!(r2.seq > r1.seq, "seq must advance on republish");

    // A different rule → a different row, higher seq.
    let r3 = store.publish(&rule("prefer pure functions", "Purity")).await.unwrap();
    assert_ne!(r3.id, r1.id);
    assert!(r3.seq > r2.seq);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p hive-mind --test store_publish_test`
Expected: FAIL — `cannot find type HiveStore`.

- [ ] **Step 3: Implement `store.rs` (namespace upsert + publish)**

Create `crates/hive-mind/src/store.rs`:

```rust
//! All hive SQL. One type, focused methods, sqlx 0.8.

use hive_protocol::{PublishResponse, PublishedRule};
use sqlx_postgres::PgPool;

#[derive(Clone)]
pub struct HiveStore {
    pool: PgPool,
}

impl HiveStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert a namespace by (scope_key, slug); return its id.
    async fn upsert_namespace(
        &self,
        scope_key: &str,
        slug: &str,
        name: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.namespaces(scope_key, slug, name)
             VALUES($1,$2,$3)
             ON CONFLICT(scope_key, slug) DO UPDATE SET name = EXCLUDED.name
             RETURNING id",
        )
        .bind(scope_key).bind(slug).bind(name)
        .fetch_one(&self.pool).await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// Publish (upsert) a rule. Identity = (namespace, content_hash).
    /// seq is advanced from the sequence on EVERY write (insert or republish).
    pub async fn publish(&self, r: &PublishedRule) -> Result<PublishResponse, String> {
        let ns = self.upsert_namespace(&r.scope_key, &r.namespace_slug, &r.namespace_name).await?;

        let row: (uuid::Uuid, i32, i64) = sqlx_core::query_as::query_as(
            "INSERT INTO hive.shared_rules
               (namespace_id, content_hash, rule_type, title, content, impact,
                enforcement, origin_repo, published_by, published_at,
                seq, status, version, updated_at)
             VALUES
               ($1,$2,$3,$4,$5,$6,$7::enforcement,$8,$9,$10::timestamptz,
                nextval('hive.shared_rules_seq'), 'active', 1, now())
             ON CONFLICT (namespace_id, content_hash) DO UPDATE SET
               rule_type    = EXCLUDED.rule_type,
               title        = EXCLUDED.title,
               content      = EXCLUDED.content,
               impact       = EXCLUDED.impact,
               enforcement  = EXCLUDED.enforcement,
               origin_repo  = EXCLUDED.origin_repo,
               published_by = EXCLUDED.published_by,
               published_at = EXCLUDED.published_at,
               status       = 'active',
               version      = hive.shared_rules.version + 1,
               seq          = nextval('hive.shared_rules_seq'),
               updated_at   = now()
             RETURNING id, version, seq",
        )
        .bind(ns)
        .bind(&r.content_hash).bind(&r.rule_type).bind(&r.title).bind(&r.content)
        .bind(&r.impact).bind(&r.enforcement).bind(&r.origin_repo)
        .bind(&r.published_by).bind(&r.published_at)
        .fetch_one(&self.pool).await
        .map_err(|e| e.to_string())?;

        Ok(PublishResponse { id: row.0.to_string(), version: row.1, seq: row.2 })
    }
}
```

Add `uuid = { version = "1", features = ["v4"] }` to `crates/hive-mind/Cargo.toml` `[dependencies]`, and `pub mod store;` to `src/lib.rs`.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p hive-mind --test store_publish_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/hive-mind
git commit -m "feat(hive): HiveStore publish — namespace upsert + content-hash upsert with seq/version bump"
```

---

## Task 6: `HiveStore` — pull deltas + retract

**Files:**
- Modify: `crates/hive-mind/src/store.rs`
- Create: `crates/hive-mind/tests/store_pull_test.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/hive-mind/tests/store_pull_test.rs`:

```rust
use hive_mind::db::HiveDb;
use hive_mind::store::HiveStore;
use hive_protocol::{content_hash, PublishedRule};

fn rule(content: &str) -> PublishedRule {
    PublishedRule {
        content_hash: content_hash(content),
        scope_key: "organization".into(),
        namespace_slug: "sensei-hq".into(),
        namespace_name: "Sensei HQ".into(),
        rule_type: "convention".into(),
        title: "t".into(),
        content: content.into(),
        impact: None,
        enforcement: "recommended".into(),
        origin_repo: None,
        published_by: "jerry".into(),
        published_at: "2026-06-11T00:00:00Z".into(),
    }
}

#[tokio::test]
async fn pull_since_returns_deltas_and_tombstones() {
    let db = HiveDb::bootstrap_temp().await.unwrap();
    let store = HiveStore::new(db.pool().clone());

    let a = store.publish(&rule("rule a")).await.unwrap();
    let _b = store.publish(&rule("rule b")).await.unwrap();

    let page = store.pull_since(0).await.unwrap();
    assert_eq!(page.rules.len(), 2);
    assert!(page.cursor >= 2);
    // round-tripped fields survive
    assert_eq!(page.rules[0].rule.namespace_slug, "sensei-hq");
    assert_eq!(page.rules[0].rule.scope_key, "organization");

    // Nothing new since the cursor.
    let empty = store.pull_since(page.cursor).await.unwrap();
    assert_eq!(empty.rules.len(), 0);

    // Retract A → it reappears past the cursor as a tombstone.
    store.retract(&a.id).await.unwrap();
    let after = store.pull_since(page.cursor).await.unwrap();
    assert_eq!(after.rules.len(), 1);
    assert_eq!(after.rules[0].status, "tombstoned");
    assert_eq!(after.rules[0].id, a.id);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p hive-mind --test store_pull_test`
Expected: FAIL — `no method named pull_since`.

- [ ] **Step 3: Implement `pull_since` + `retract`**

Append to `impl HiveStore` in `crates/hive-mind/src/store.rs`:

```rust
    /// Pull all rules changed after `since` (inclusive of tombstones), ordered by seq.
    /// Returns the rows + the new cursor (max seq observed, or `since` if none).
    pub async fn pull_since(&self, since: i64) -> Result<hive_protocol::PullResponse, String> {
        use hive_protocol::{PublishedRule, PulledRule, PullResponse};

        // (id, seq, status, version, ns fields, snapshot fields)
        let rows: Vec<(
            uuid::Uuid, i64, String, i32,
            String, String, String,            // scope_key, slug, name
            String, String, String, Option<String>, String, Option<String>, String, String,
        )> = sqlx_core::query_as::query_as(
            "SELECT r.id, r.seq, r.status, r.version,
                    n.scope_key, n.slug, n.name,
                    r.rule_type, r.title, r.content, r.impact,
                    r.enforcement::text, r.origin_repo, r.published_by,
                    to_char(r.published_at, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
             FROM hive.shared_rules r
             JOIN sensei.namespaces n ON n.id = r.namespace_id
             WHERE r.seq > $1
             ORDER BY r.seq",
        )
        .bind(since)
        .fetch_all(&self.pool).await
        .map_err(|e| e.to_string())?;

        let mut cursor = since;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            cursor = cursor.max(r.1);
            out.push(PulledRule {
                id: r.0.to_string(),
                seq: r.1,
                status: r.2,
                version: r.3,
                rule: PublishedRule {
                    scope_key: r.4,
                    namespace_slug: r.5,
                    namespace_name: r.6,
                    rule_type: r.7,
                    title: r.8,
                    content: r.9.clone(),
                    impact: r.10,
                    enforcement: r.11,
                    origin_repo: r.12,
                    published_by: r.13,
                    published_at: r.14,
                    content_hash: hive_protocol::content_hash(&r.9),
                },
            });
        }
        Ok(PullResponse { rules: out, cursor })
    }

    /// Retract a rule: mark it tombstoned and advance its seq so pullers observe it.
    pub async fn retract(&self, id: &str) -> Result<bool, String> {
        let uuid = uuid::Uuid::parse_str(id).map_err(|e| e.to_string())?;
        let res = sqlx_core::query::query(
            "UPDATE hive.shared_rules
             SET status='tombstoned', seq=nextval('hive.shared_rules_seq'), updated_at=now()
             WHERE id=$1 AND status <> 'tombstoned'",
        )
        .bind(uuid)
        .execute(&self.pool).await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p hive-mind --test store_pull_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/hive-mind/src/store.rs crates/hive-mind/tests/store_pull_test.rs
git commit -m "feat(hive): HiveStore pull_since (incl tombstones, cursor) + retract"
```

---

## Task 7: `HiveStore` — members, API keys, audit

**Files:**
- Modify: `crates/hive-mind/src/store.rs`, `crates/hive-mind/Cargo.toml`
- Create: `crates/hive-mind/tests/store_auth_test.rs`

- [ ] **Step 1: Add deps**

In `crates/hive-mind/Cargo.toml` `[dependencies]` add:

```toml
sha2 = "0.10"
rand = "0.8"
subtle = "2"
```

- [ ] **Step 2: Write the failing test**

Create `crates/hive-mind/tests/store_auth_test.rs`:

```rust
use hive_mind::db::HiveDb;
use hive_mind::store::HiveStore;

#[tokio::test]
async fn member_key_issue_lookup_and_revoke() {
    let db = HiveDb::bootstrap_temp().await.unwrap();
    let store = HiveStore::new(db.pool().clone());

    let member = store.create_member("Jerry", Some("jerry@x.io"), "publisher").await.unwrap();
    let issued = store.issue_key(&member, Some("laptop")).await.unwrap();
    assert!(issued.plaintext.len() >= 40, "high-entropy key");

    // Valid key resolves to the member's role.
    let who = store.find_member_by_key(&issued.plaintext).await.unwrap();
    assert!(who.is_some());
    let who = who.unwrap();
    assert_eq!(who.role, "publisher");
    assert_eq!(who.member_id, member);

    // Garbage key → None.
    assert!(store.find_member_by_key("not-a-real-key").await.unwrap().is_none());

    // Revoked key → None.
    store.revoke_key(&issued.key_id).await.unwrap();
    assert!(store.find_member_by_key(&issued.plaintext).await.unwrap().is_none());
}
```

- [ ] **Step 3: Implement members/keys/audit**

Append to `crates/hive-mind/src/store.rs` (and add the structs above the `impl`):

```rust
/// A resolved caller identity (from a valid API key).
#[derive(Debug, Clone)]
pub struct Caller {
    pub member_id: uuid::Uuid,
    pub role: String,
}

/// A freshly issued key — `plaintext` is shown to the operator exactly once.
pub struct IssuedKey {
    pub key_id: String,
    pub plaintext: String,
}

fn hash_key(plaintext: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(plaintext.as_bytes());
    format!("{:x}", h.finalize())
}

fn random_key() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    // url-safe, no padding — ~43 chars of high entropy
    use std::fmt::Write;
    let mut s = String::with_capacity(64);
    for b in bytes { let _ = write!(s, "{b:02x}"); }
    s
}
```

Then, inside `impl HiveStore`:

```rust
    pub async fn create_member(
        &self, name: &str, email: Option<&str>, role: &str,
    ) -> Result<uuid::Uuid, String> {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO hive.members(name, email, role) VALUES($1,$2,$3) RETURNING id",
        )
        .bind(name).bind(email).bind(role)
        .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    pub async fn issue_key(
        &self, member_id: &uuid::Uuid, label: Option<&str>,
    ) -> Result<IssuedKey, String> {
        let plaintext = random_key();
        let key_hash = hash_key(&plaintext);
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO hive.api_keys(member_id, key_hash, label) VALUES($1,$2,$3) RETURNING id",
        )
        .bind(member_id).bind(&key_hash).bind(label)
        .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(IssuedKey { key_id: row.0.to_string(), plaintext })
    }

    /// Resolve a presented bearer key to a caller. Compares hashes (the compared
    /// value is itself a sha256, so timing reveals nothing about the key); the
    /// `subtle` check guards against any residual byte-wise short-circuit.
    pub async fn find_member_by_key(&self, presented: &str) -> Result<Option<Caller>, String> {
        use subtle::ConstantTimeEq;
        let presented_hash = hash_key(presented);
        let rows: Vec<(uuid::Uuid, uuid::Uuid, String, String)> = sqlx_core::query_as::query_as(
            "SELECT k.id, m.id, m.role, k.key_hash
             FROM hive.api_keys k JOIN hive.members m ON m.id = k.member_id
             WHERE k.revoked_at IS NULL AND m.disabled_at IS NULL",
        )
        .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        for (key_id, member_id, role, key_hash) in rows {
            if key_hash.as_bytes().ct_eq(presented_hash.as_bytes()).into() {
                let _ = sqlx_core::query::query(
                    "UPDATE hive.api_keys SET last_used_at = now() WHERE id = $1")
                    .bind(key_id).execute(&self.pool).await;
                return Ok(Some(Caller { member_id, role }));
            }
        }
        Ok(None)
    }

    pub async fn revoke_key(&self, key_id: &str) -> Result<(), String> {
        let uuid = uuid::Uuid::parse_str(key_id).map_err(|e| e.to_string())?;
        sqlx_core::query::query("UPDATE hive.api_keys SET revoked_at = now() WHERE id = $1")
            .bind(uuid).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn record_audit(
        &self, member_id: Option<&uuid::Uuid>, action: &str, target: Option<&str>,
        detail: serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO hive.audit_log(member_id, action, target, detail) VALUES($1,$2,$3,$4)")
            .bind(member_id).bind(action).bind(target).bind(detail)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p hive-mind --test store_auth_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/hive-mind/src/store.rs crates/hive-mind/tests/store_auth_test.rs crates/hive-mind/Cargo.toml
git commit -m "feat(hive): HiveStore members/api_keys/audit — issue (hash-at-rest), constant-time lookup, revoke"
```

---

## Task 8: Auth middleware + role floor

**Files:**
- Modify: `crates/hive-mind/Cargo.toml` (axum + tower deps), `crates/hive-mind/src/lib.rs`
- Create: `crates/hive-mind/src/auth.rs`
- Create: `crates/hive-mind/tests/auth_test.rs`

- [ ] **Step 1: Add deps**

In `crates/hive-mind/Cargo.toml` `[dependencies]`:

```toml
axum = "0.8"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors"] }
```

And `[dev-dependencies]`:

```toml
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"
```

Add `pub mod auth;` and `pub mod api;` to `crates/hive-mind/src/lib.rs` (create `api.rs` in Task 9; declare it now or add in Task 9 — declare `auth` now).

- [ ] **Step 2: Write the failing test**

Create `crates/hive-mind/tests/auth_test.rs`:

```rust
use hive_mind::auth::{role_satisfies, Role};

#[test]
fn role_floor_ordering() {
    assert!(role_satisfies(Role::Admin, Role::Publisher));
    assert!(role_satisfies(Role::Publisher, Role::Member));
    assert!(role_satisfies(Role::Member, Role::Member));
    assert!(!role_satisfies(Role::Member, Role::Publisher));
    assert!(!role_satisfies(Role::Publisher, Role::Admin));
}

#[test]
fn role_parses_from_db_text() {
    assert_eq!(Role::parse("admin"), Some(Role::Admin));
    assert_eq!(Role::parse("publisher"), Some(Role::Publisher));
    assert_eq!(Role::parse("member"), Some(Role::Member));
    assert_eq!(Role::parse("nonsense"), None);
}
```

- [ ] **Step 3: Run it to verify it fails**

Run: `cargo test -p hive-mind --test auth_test`
Expected: FAIL — `cannot find ... Role`.

- [ ] **Step 4: Implement `auth.rs`**

Create `crates/hive-mind/src/auth.rs`:

```rust
//! Bearer-token auth + role-floor enforcement.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use crate::api::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Member = 0,
    Publisher = 1,
    Admin = 2,
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "member" => Some(Role::Member),
            "publisher" => Some(Role::Publisher),
            "admin" => Some(Role::Admin),
            _ => None,
        }
    }
}

/// True when `have` meets-or-exceeds the required `floor`.
pub fn role_satisfies(have: Role, floor: Role) -> bool {
    have >= floor
}

/// The resolved caller, attached to the request by `require`.
#[derive(Clone)]
pub struct AuthCaller {
    pub member_id: uuid::Uuid,
    pub role: Role,
}

fn bearer(req: &Request) -> Option<String> {
    req.headers()
        .get(axum::http::header::AUTHORIZATION)?
        .to_str().ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

/// Middleware: resolve the bearer key → 401 if invalid; attach `AuthCaller`.
/// Per-route role floors are checked in handlers via `AuthCaller`.
pub async fn require(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let key = bearer(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    let caller = state.store.find_member_by_key(&key).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let role = Role::parse(&caller.role).ok_or(StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(AuthCaller { member_id: caller.member_id, role });
    Ok(next.run(req).await)
}
```

> Note: `auth.rs` references `crate::api::AppState`. Declare `pub mod api;` in `lib.rs` now and create a minimal `api.rs` with just `AppState` (filled out in Task 9):
> ```rust
> use std::sync::Arc;
> use crate::store::HiveStore;
> pub struct SharedState { pub store: HiveStore }
> pub type AppState = Arc<SharedState>;
> ```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p hive-mind --test auth_test`
Expected: PASS (the unit tests don't need the DB).

- [ ] **Step 6: Commit**

```bash
git add crates/hive-mind/src/auth.rs crates/hive-mind/src/api.rs crates/hive-mind/src/lib.rs crates/hive-mind/Cargo.toml crates/hive-mind/tests/auth_test.rs
git commit -m "feat(hive): bearer auth middleware + Role floor ordering"
```

---

## Task 9: Router + handlers (the `/v1` API)

**Files:**
- Modify: `crates/hive-mind/src/api.rs`
- Create: `crates/hive-mind/tests/api_test.rs`

- [ ] **Step 1: Write the failing integration test**

Create `crates/hive-mind/tests/api_test.rs`:

```rust
use hive_mind::api::{build_router, SharedState};
use hive_mind::db::HiveDb;
use hive_mind::store::HiveStore;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt; // oneshot

async fn app_with_keys() -> (axum::Router, String, String) {
    let db = HiveDb::bootstrap_temp().await.unwrap();
    let store = HiveStore::new(db.pool().clone());
    let pub_member = store.create_member("Pub", None, "publisher").await.unwrap();
    let mem_member = store.create_member("Mem", None, "member").await.unwrap();
    let pub_key = store.issue_key(&pub_member, None).await.unwrap().plaintext;
    let mem_key = store.issue_key(&mem_member, None).await.unwrap().plaintext;
    // Leak the db so the embedded PG outlives the test app.
    Box::leak(Box::new(db));
    let router = build_router(Arc::new(SharedState { store }));
    (router, pub_key, mem_key)
}

fn publish_body() -> String {
    serde_json::json!({
        "content_hash": hive_protocol::content_hash("always tdd"),
        "scope_key": "organization", "namespace_slug": "sensei-hq", "namespace_name": "Sensei HQ",
        "rule_type": "convention", "title": "TDD", "content": "always tdd",
        "impact": null, "enforcement": "mandatory",
        "origin_repo": "sensei/daemon", "published_by": "jerry",
        "published_at": "2026-06-11T00:00:00Z"
    }).to_string()
}

#[tokio::test]
async fn health_is_unauthenticated() {
    let (app, _, _) = app_with_keys().await;
    let res = app.oneshot(Request::get("/v1/health").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn publish_requires_publisher_then_pull_returns_it() {
    let (app, pub_key, mem_key) = app_with_keys().await;

    // member key cannot publish
    let res = app.clone().oneshot(
        Request::post("/v1/rules")
            .header("authorization", format!("Bearer {mem_key}"))
            .header("content-type", "application/json")
            .body(Body::from(publish_body())).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // publisher key can
    let res = app.clone().oneshot(
        Request::post("/v1/rules")
            .header("authorization", format!("Bearer {pub_key}"))
            .header("content-type", "application/json")
            .body(Body::from(publish_body())).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // member can pull and see it
    let res = app.oneshot(
        Request::get("/v1/rules?since=0")
            .header("authorization", format!("Bearer {mem_key}"))
            .body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["rules"].as_array().unwrap().len(), 1);
    assert_eq!(v["rules"][0]["enforcement"], "mandatory");
}

#[tokio::test]
async fn no_key_is_unauthorized() {
    let (app, _, _) = app_with_keys().await;
    let res = app.oneshot(Request::get("/v1/rules?since=0").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p hive-mind --test api_test`
Expected: FAIL — `cannot find function build_router`.

- [ ] **Step 3: Implement `api.rs` (state, handlers, router)**

Replace `crates/hive-mind/src/api.rs` with:

```rust
//! Axum router + handlers for the `/v1` federation API.

use std::sync::Arc;

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;

use crate::auth::{require, role_satisfies, AuthCaller, Role};
use crate::store::HiveStore;
use hive_protocol::PublishedRule;

pub struct SharedState {
    pub store: HiveStore,
}
pub type AppState = Arc<SharedState>;

fn err(code: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (code, Json(serde_json::json!({ "error": msg })))
}

fn require_role(caller: &AuthCaller, floor: Role) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if role_satisfies(caller.role, floor) {
        Ok(())
    } else {
        Err(err(StatusCode::FORBIDDEN, "insufficient role"))
    }
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "service": "sensei-hive", "scope": "hive" }))
}

async fn publish_rule(
    State(state): State<AppState>,
    Extension(caller): Extension<AuthCaller>,
    Json(rule): Json<PublishedRule>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_role(&caller, Role::Publisher)?;
    let resp = state.store.publish(&rule).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let _ = state.store.record_audit(
        Some(&caller.member_id), "publish", Some(&resp.id),
        serde_json::json!({ "version": resp.version, "seq": resp.seq })).await;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

#[derive(Deserialize)]
struct PullQuery { since: Option<i64> }

async fn pull_rules(
    State(state): State<AppState>,
    Extension(_caller): Extension<AuthCaller>,
    Query(q): Query<PullQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let page = state.store.pull_since(q.since.unwrap_or(0)).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::to_value(page).unwrap()))
}

async fn retract_rule(
    State(state): State<AppState>,
    Extension(caller): Extension<AuthCaller>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    require_role(&caller, Role::Publisher)?;
    let ok = state.store.retract(&id).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let _ = state.store.record_audit(Some(&caller.member_id), "retract", Some(&id),
        serde_json::json!({})).await;
    if ok { Ok(StatusCode::NO_CONTENT) } else { Err(err(StatusCode::NOT_FOUND, "no such rule")) }
}

#[derive(Deserialize)]
struct NewMember { name: String, email: Option<String>, role: String }

async fn add_member(
    State(state): State<AppState>,
    Extension(caller): Extension<AuthCaller>,
    Json(m): Json<NewMember>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_role(&caller, Role::Admin)?;
    if Role::parse(&m.role).is_none() {
        return Err(err(StatusCode::BAD_REQUEST, "role must be member|publisher|admin"));
    }
    let id = state.store.create_member(&m.name, m.email.as_deref(), &m.role).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let _ = state.store.record_audit(Some(&caller.member_id), "member.add", Some(&id.to_string()),
        serde_json::json!({ "role": m.role })).await;
    Ok(Json(serde_json::json!({ "id": id.to_string() })))
}

#[derive(Deserialize)]
struct NewKey { label: Option<String> }

async fn issue_key(
    State(state): State<AppState>,
    Extension(caller): Extension<AuthCaller>,
    axum::extract::Path(member_id): axum::extract::Path<String>,
    Json(k): Json<NewKey>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_role(&caller, Role::Admin)?;
    let mid = uuid::Uuid::parse_str(&member_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "bad member id"))?;
    let issued = state.store.issue_key(&mid, k.label.as_deref()).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let _ = state.store.record_audit(Some(&caller.member_id), "key.issue", Some(&issued.key_id),
        serde_json::json!({ "member_id": member_id })).await;
    // plaintext shown exactly once
    Ok(Json(serde_json::json!({ "key_id": issued.key_id, "api_key": issued.plaintext })))
}

async fn subscriptions_stub(
    Extension(_caller): Extension<AuthCaller>,
) -> (StatusCode, Json<serde_json::Value>) {
    err(StatusCode::NOT_IMPLEMENTED, "webhook subscriptions not yet implemented")
}

/// Build the full router: public `/v1/health`, everything else behind `require`.
pub fn build_router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/v1/rules", post(publish_rule).get(pull_rules))
        .route("/v1/rules/:id", delete(retract_rule))
        .route("/v1/members", post(add_member))
        .route("/v1/members/:id/keys", post(issue_key))
        .route("/v1/subscriptions", post(subscriptions_stub))
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), require));

    Router::new()
        .route("/v1/health", get(health))
        .merge(protected)
        .with_state(state)
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p hive-mind --test api_test`
Expected: PASS — health 200, member publish 403, publisher publish 200, pull returns 1 mandatory rule, no-key 401.

- [ ] **Step 5: Commit**

```bash
git add crates/hive-mind/src/api.rs crates/hive-mind/tests/api_test.rs
git commit -m "feat(hive): /v1 router + handlers — publish/pull/retract/members/keys, role floors, audit, subscriptions stub"
```

---

## Task 10: `keygen` CLI (bootstrap the first admin key)

Before any REST key-management exists, an operator needs a first `admin` key. `keygen` operates directly on the embedded DB.

**Files:**
- Modify: `crates/hive-mind/Cargo.toml` (clap), `crates/hive-mind/src/lib.rs`
- Create: `crates/hive-mind/src/keygen.rs`
- Create: `crates/hive-mind/tests/keygen_test.rs`

- [ ] **Step 1: Add clap**

In `crates/hive-mind/Cargo.toml` `[dependencies]`:

```toml
clap = { version = "4", features = ["derive"] }
```

Add `pub mod keygen;` to `crates/hive-mind/src/lib.rs`.

- [ ] **Step 2: Write the failing test**

Create `crates/hive-mind/tests/keygen_test.rs`:

```rust
use hive_mind::db::HiveDb;
use hive_mind::keygen::generate_key;
use hive_mind::store::HiveStore;

#[tokio::test]
async fn keygen_creates_resolvable_admin_key() {
    let db = HiveDb::bootstrap_temp().await.unwrap();
    let store = HiveStore::new(db.pool().clone());

    let key = generate_key(&store, "bootstrap admin", "admin", Some("initial")).await.unwrap();
    let caller = store.find_member_by_key(&key).await.unwrap().unwrap();
    assert_eq!(caller.role, "admin");
}
```

- [ ] **Step 3: Implement `keygen.rs`**

Create `crates/hive-mind/src/keygen.rs`:

```rust
//! `sensei-hive keygen` — mint a member + API key directly against the DB.

use crate::store::HiveStore;

/// Create a member with `role` and issue one key; returns the plaintext key.
pub async fn generate_key(
    store: &HiveStore, name: &str, role: &str, label: Option<&str>,
) -> Result<String, String> {
    if crate::auth::Role::parse(role).is_none() {
        return Err(format!("invalid role '{role}' (member|publisher|admin)"));
    }
    let member = store.create_member(name, None, role).await?;
    let issued = store.issue_key(&member, label).await?;
    store.record_audit(Some(&member), "key.issue", Some(&issued.key_id),
        serde_json::json!({ "via": "keygen", "role": role })).await?;
    Ok(issued.plaintext)
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p hive-mind --test keygen_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/hive-mind/src/keygen.rs crates/hive-mind/tests/keygen_test.rs crates/hive-mind/src/lib.rs crates/hive-mind/Cargo.toml
git commit -m "feat(hive): keygen — bootstrap a member + API key directly on the DB"
```

---

## Task 11: `main.rs` — config, boot, serve

**Files:**
- Create: `crates/hive-mind/src/config.rs`
- Modify: `crates/hive-mind/src/main.rs`, `crates/hive-mind/src/lib.rs`
- Create: `crates/hive-mind/tests/serve_test.rs`

- [ ] **Step 1: Implement `config.rs`**

Create `crates/hive-mind/src/config.rs`:

```rust
//! Runtime config for sensei-hive (env-driven, with sane defaults).

use std::path::PathBuf;

pub struct HiveConfig {
    pub data_dir: PathBuf,     // embedded PG data dir
    pub database_dir: PathBuf, // the sensei `database/` DDL tree
    pub bind: String,          // e.g. "127.0.0.1:7755"
}

impl HiveConfig {
    pub fn from_env() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let data_dir = std::env::var("SENSEI_HIVE_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(&home).join(".sensei-hive/pg"));
        let database_dir = std::env::var("SENSEI_HIVE_DDL_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database"));
        let bind = std::env::var("SENSEI_HIVE_BIND").unwrap_or_else(|_| "127.0.0.1:7755".into());
        Self { data_dir, database_dir, bind }
    }
}
```

Add `pub mod config;` to `lib.rs`.

- [ ] **Step 2: Write the failing smoke test**

Create `crates/hive-mind/tests/serve_test.rs`:

```rust
use hive_mind::api::{build_router, SharedState};
use hive_mind::db::HiveDb;
use hive_mind::store::HiveStore;
use std::sync::Arc;

#[tokio::test]
async fn server_serves_health_on_ephemeral_port() {
    let db = HiveDb::bootstrap_temp().await.unwrap();
    let store = HiveStore::new(db.pool().clone());
    Box::leak(Box::new(db));
    let app = build_router(Arc::new(SharedState { store }));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });

    // give the server a tick
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let body = reqwest::get(format!("http://{addr}/v1/health")).await.unwrap()
        .text().await.unwrap();
    assert!(body.contains("\"status\":\"ok\""), "got: {body}");
}
```

Add `reqwest = { version = "0.12", features = ["json"] }` to `[dev-dependencies]`.

- [ ] **Step 3: Run it to verify it fails, then implement `main.rs`**

Run: `cargo test -p hive-mind --test serve_test`
Expected: FAIL until `main.rs`/lib expose `build_router` (already do from Task 9) — this test mainly guards the serve wiring; it should pass once deps compile. If it fails on missing `reqwest`, add the dev-dep.

Replace `crates/hive-mind/src/main.rs` with the real entrypoint:

```rust
use std::sync::Arc;

use clap::{Parser, Subcommand};
use hive_mind::api::{build_router, SharedState};
use hive_mind::config::HiveConfig;
use hive_mind::db::HiveDb;
use hive_mind::keygen::generate_key;
use hive_mind::store::HiveStore;

#[derive(Parser)]
#[command(name = "sensei-hive", about = "sensei hive-mind shared-brain service")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the federation service (default).
    Serve,
    /// Mint a member + API key (bootstrap the first admin).
    Keygen {
        #[arg(long)] name: String,
        #[arg(long, default_value = "member")] role: String,
        #[arg(long)] label: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info".into())).init();

    let cli = Cli::parse();
    let cfg = HiveConfig::from_env();
    let db = HiveDb::bootstrap(cfg.data_dir.clone(), cfg.database_dir.clone()).await?;
    let store = HiveStore::new(db.pool().clone());

    match cli.cmd.unwrap_or(Cmd::Serve) {
        Cmd::Keygen { name, role, label } => {
            let key = generate_key(&store, &name, &role, label.as_deref()).await?;
            println!("API key for {name} ({role}) — store it now, shown once:\n{key}");
        }
        Cmd::Serve => {
            let app = build_router(Arc::new(SharedState { store }));
            let listener = tokio::net::TcpListener::bind(&cfg.bind).await?;
            tracing::info!(bind = %cfg.bind, "sensei-hive listening");
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await?;
        }
    }
    Ok(())
}
```

Add `tracing-subscriber = { version = "0.3", features = ["env-filter"] }` to `[dependencies]`.

- [ ] **Step 4: Run the test + full crate suite**

Run: `cargo test -p hive-mind`
Expected: PASS — all hive-mind tests (scope, db, publish, pull, auth, api, keygen, serve).

- [ ] **Step 5: Commit**

```bash
git add crates/hive-mind
git commit -m "feat(hive): main entrypoint — serve + keygen subcommands, env config, graceful shutdown"
```

---

## Task 12: README + build target

**Files:**
- Create: `crates/hive-mind/README.md`
- Modify: `Makefile` (add a `hive` build target)

- [ ] **Step 1: Write the README**

Create `crates/hive-mind/README.md`:

```markdown
# sensei-hive

The org "shared brain" for sensei governance federation — a slim Axum service
over an embedded Postgres holding promoted, shareable rules.

## Run

```bash
# First, mint an admin key (creates the embedded DB on first run):
sensei-hive keygen --name "admin" --role admin --label initial
# Then serve:
SENSEI_HIVE_BIND=0.0.0.0:7755 sensei-hive serve
```

Config (env): `SENSEI_HIVE_DATA_DIR` (embedded PG data dir, default `~/.sensei-hive/pg`),
`SENSEI_HIVE_DDL_DIR` (the `database/` DDL tree), `SENSEI_HIVE_BIND` (default `127.0.0.1:7755`).

Schema = the `hive` dbd scope (`database/design.hive.yaml`): `shared_rules`, `members`,
`api_keys`, `audit_log` + the shared `scopes`/`namespaces`/`enforcement`. Roles:
`member` (pull), `publisher` (+publish), `admin` (+manage). See the
[federation design spec](../../docs/superpowers/specs/2026-06-11-hive-mind-federation-design.md).
```

- [ ] **Step 2: Add a Makefile build target**

Add to `Makefile`:

```makefile
.PHONY: hive
hive:  ## Build the sensei-hive service binary
	cargo build --release -p hive-mind
```

- [ ] **Step 3: Verify the workspace builds clean + run zero-errors gate**

Run: `cargo build -p hive-mind -p hive-protocol && cargo clippy -p hive-mind -p hive-protocol -- -D warnings && cargo test -p hive-mind -p hive-protocol`
Expected: PASS — clean build, no clippy warnings, all tests green. (Use the `zero-errors-policy` skill here.)

- [ ] **Step 4: Commit**

```bash
git add crates/hive-mind/README.md Makefile
git commit -m "docs(hive): README + make hive build target"
```

---

## Self-Review

**Spec coverage (against `2026-06-11-hive-mind-federation-design.md`):**
- §4 boundary / new crates `hive-mind` + `hive-protocol` → Tasks 2, 3–12. ✅
- §6 protocol endpoints (health, publish, pull `since`, retract, members, keys, subscriptions stub) → Task 9. ✅ (audit GET endpoint deferred — `audit_log` is written Task 7/9; a read endpoint is admin-only sugar, noted below.)
- §6 monotonic `seq` cursor advancing on every mutation → Tasks 5 (publish), 6 (retract). ✅
- §7 `PublishedRule` flattened snapshot + `content_hash` parity → Task 2. ✅
- §8 hive schema (scope-tagged), embedded PG bootstrap, keygen, TLS → Tasks 3, 4, 10, 11. ⚠️ TLS: `main.rs` binds plain TCP; rustls is **deferred** (see gap below) — acceptable for the loopback/MVP default, but §12 expects https for non-loopback.
- §12 security (hash-at-rest, constant-time, key shown once) → Tasks 7, 9. ✅
- §13 dbd `hive` scope + daemon `skip_schemas` → Tasks 1, 3. ✅

**Gaps found + resolution:**
1. **`GET /v1/audit` (admin)** is in the spec table but no task implements the read side. *Resolution:* add a one-handler step to Task 9 if desired; low-risk. Flag as a small follow-up rather than blocking — the audit data is captured; only the read endpoint is missing.
2. **TLS (rustls)** is specified in §8/§12 but Task 11 binds plain TCP. *Resolution:* MVP default is loopback (`127.0.0.1`), where plain http is allowed by the daemon's own rule (§12). Non-loopback TLS termination = a documented follow-up (reverse proxy or an `axum-server` rustls task). Called out explicitly so it isn't a silent omission.
3. **`namespaces.slug_aliases`** (rename handling, §5) is not added here — it's only needed when the daemon pushes/pulls (#26) and for renames. Deferred to #26; noted.

**Placeholder scan:** No TBD/TODO. The two "implementer notes" (dbd entity-name format in Task 3; `postgresql_embedded` 0.18 API in Task 4) are explicit verification steps against named on-disk sources, not hand-waving — each says exactly where to confirm and what to adjust.

**Type consistency:** `HiveStore` methods (`publish`/`pull_since`/`retract`/`create_member`/`issue_key`/`find_member_by_key`/`revoke_key`/`record_audit`), `Caller`/`AuthCaller`, `Role`/`role_satisfies`, `SharedState`/`AppState`/`build_router`, and the `hive-protocol` types (`PublishedRule`/`PublishResponse`/`PulledRule`/`PullResponse`, `content_hash`) are used with identical names/signatures across Tasks 2–11. ✅
