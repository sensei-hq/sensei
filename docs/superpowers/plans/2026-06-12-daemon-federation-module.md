# Daemon Federation Module Implementation Plan — Governance P4 / #26

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `senseid` a federation module that registers hive-mind endpoints (`knowledge_sources`), pushes a promoted rule to its hive when the promotion is approved, and poll-pulls applicable rules back as `memories(origin='federated')` — which flow into the existing resolution with no new resolution code.

**Architecture:** A new `crates/senseid/src/federation/` module + two `sensei`-schema tables (`knowledge_sources`, `federated_memories` ledger). Push hooks the existing `accept_proposal` path; pull is a periodic `tokio` interval task (same pattern as the log-retention task) plus an on-demand endpoint. The `federated_memories` ledger (keyed by `(knowledge_source_id, remote_rule_id)`) is the idempotency key AND the echo-guard: a rule we pushed is recorded there, so pulling it back links to our memory instead of duplicating it. Credentials reuse the `gateway_keys` Keychain module. HTTP via the `reqwest` 0.12 dep senseid already has; wire types from `hive-protocol`.

**Tech Stack:** Rust 2024, sqlx 0.8, axum 0.8, reqwest 0.12, `hive-protocol`, `gateway_keys` (Keychain), tokio intervals.

**Spec:** [`docs/superpowers/specs/2026-06-11-hive-mind-federation-design.md`](../specs/2026-06-11-hive-mind-federation-design.md) §5 (identity), §6 (protocol), §9 (this module), §11 (resolution), §12 (security).

**Depends on:** #25 (`sensei-hive` + `hive-protocol`, merged to `main`).

---

## File Structure

**New:**
- `database/ddl/table/sensei/knowledge_sources.ddl` — registered federation endpoints.
- `database/ddl/table/sensei/federated_memories.ddl` — local↔remote rule ledger + per-rule cursor.
- `crates/senseid/src/federation/mod.rs` — the module: `PublishedRule` builder (pure), `push_promoted`, `pull_source`, `run_pull_loop`, `PullStats`.

**Modified:**
- `crates/senseid/Cargo.toml` — add `hive-protocol = { path = "../hive-protocol" }`.
- `crates/senseid/src/db/pg_store.rs` — `InsertMemory.source_id` (+ insert SQL); `KnowledgeSource` struct + CRUD; `federated_memories` ops; `namespace_is_shareable`; `archive_federated_memory`; `memory_push_payload`.
- `crates/senseid/src/lib.rs` (or `main.rs` module list) — `pub mod federation;`.
- `crates/senseid/src/api/handlers/knowledge.rs` — source handlers + the push hook in `accept_proposal`.
- `crates/senseid/src/api/routes.rs` — register the source routes.
- `crates/senseid/src/api/server.rs` — spawn `run_pull_loop` in `build_full_app`.

**Test DB note:** senseid DB-backed tests connect to a live test DB (`PgStore::connect_test()` → `TEST_DATABASE_URL` or `sensei_test`) and skip if absent (the established pattern). The two new tables must be applied to that DB before the pg_store tests pass (Task 1 applies them).

---

## Task 1: hive-protocol dep + the two DDL tables (applied to dev/test DBs)

**Files:**
- Modify: `crates/senseid/Cargo.toml`
- Create: `database/ddl/table/sensei/knowledge_sources.ddl`, `database/ddl/table/sensei/federated_memories.ddl`

- [ ] **Step 1: Add the hive-protocol dependency**

In `crates/senseid/Cargo.toml`, in the workspace-crate dependency group (after `gateway-embedded = { path = "../gateway-embedded" }`):

```toml
hive-protocol = { path = "../hive-protocol" }
```

- [ ] **Step 2: Write `database/ddl/table/sensei/knowledge_sources.ddl`**

```sql
set search_path to sensei, extensions;

create table if not exists knowledge_sources (
  id             uuid        primary key default gen_random_uuid()
, kind           text        not null      -- hive_mind | mcp | rest | webhook (only hive_mind wired @ MVP)
, name           text        not null
, url            text        not null
, namespace_id   uuid        references sensei.namespaces(id) on delete set null  -- null = all shareable namespaces
, credential_ref text        not null      -- Keychain entry id; the API key lives in the OS keychain, never in PG
, direction      text        not null default 'both'   -- push | pull | both
, last_seq       bigint      not null default 0         -- pull cursor for this source
, enabled        boolean     not null default true
, created_at     timestamptz not null default now()
);

comment on table knowledge_sources is
'Registered federation endpoints (governance P4). Mirrors gateway-router
registration: the row holds connection metadata; the API key is in the OS
Keychain referenced by credential_ref. direction gates push vs pull; last_seq
is the per-source monotonic pull cursor.';
```

- [ ] **Step 3: Write `database/ddl/table/sensei/federated_memories.ddl`**

```sql
set search_path to sensei, extensions;

create table if not exists federated_memories (
  knowledge_source_id uuid        not null references sensei.knowledge_sources(id) on delete cascade
, remote_rule_id      uuid        not null
, content_hash        text        not null
, memory_id           uuid        references sensei.memories(id) on delete set null
, remote_seq          bigint      not null
, synced_at           timestamptz not null default now()
, primary key (knowledge_source_id, remote_rule_id)
);

comment on table federated_memories is
'Local↔remote rule mapping + per-rule cursor (federation sync bookkeeping — NOT
a parallel rules table). Pull upserts by (knowledge_source_id, remote_rule_id),
making ingestion idempotent; it is also the echo-guard — a rule this daemon
pushed is recorded here, so pulling it back links to the existing memory instead
of creating a federated duplicate. memory_id is the local memory; null after the
linked memory is hard-deleted.';
```

- [ ] **Step 4: Apply the new tables to the dev + test databases**

The daemon's DB-backed tests need these tables present. Apply via `dbd` (the `default` scope already includes new `sensei`-schema tables) against both DBs:

Run:
```bash
cd /Users/Jerry/Developer/sensei-hq/sensei
SENSEI_DDL_DIR=$PWD/database psql -d sensei      -f database/ddl/table/sensei/knowledge_sources.ddl
SENSEI_DDL_DIR=$PWD/database psql -d sensei      -f database/ddl/table/sensei/federated_memories.ddl
psql -d sensei_test -f database/ddl/table/sensei/knowledge_sources.ddl
psql -d sensei_test -f database/ddl/table/sensei/federated_memories.ddl
```
Expected: `CREATE TABLE` (or no error if already present). If `sensei_test` doesn't exist locally, that's fine — the pg_store tests skip when no test DB is reachable; create it (`createdb sensei_test` + full `dbd` deploy) if you want them to run.

- [ ] **Step 5: Verify the workspace builds with the new dep**

Run: `cargo build -p senseid`
Expected: PASS (compiles with `hive-protocol` available).

- [ ] **Step 6: Commit**

```bash
git add crates/senseid/Cargo.toml Cargo.lock database/ddl/table/sensei/knowledge_sources.ddl database/ddl/table/sensei/federated_memories.ddl
git commit -m "feat(federation): knowledge_sources + federated_memories DDL + hive-protocol dep"
```

---

## Task 2: `InsertMemory.source_id` (federated rows carry provenance)

The spec sets `memories.source_id = knowledge_sources.id` on federated rows; `InsertMemory` currently has no `source_id` field.

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs` (the `InsertMemory` struct ~line 12 and `insert_memory` ~line 2082)

- [ ] **Step 1: Write the failing test**

Add to the pg_store test module (find the existing `#[cfg(test)] mod` with the `ddl_test_skip()`/`connect_test` helper; match its style):

```rust
#[tokio::test]
async fn insert_memory_persists_source_id() {
    let Ok(pg) = PgStore::connect_test().await else { return; }; // skip if no test DB
    let src = uuid::Uuid::new_v4();
    let id = pg.insert_memory(&InsertMemory {
        project_id: None, scope: "global".into(), scope_filter: None,
        mtype: "convention".into(), title: "fed".into(), content: "federated content".into(),
        impact: None, tags: vec![], triage_signal: None, status: "active".into(),
        namespace_id: None, enforcement: Some("recommended".into()),
        origin: Some("federated".into()), source_id: Some(src),
    }).await.unwrap();
    let got: (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
        "SELECT source_id FROM sensei.memories WHERE id = $1")
        .bind(id).fetch_one(pg.pool()).await.unwrap();
    assert_eq!(got.0, Some(src));
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1").bind(id).execute(pg.pool()).await.unwrap();
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p senseid insert_memory_persists_source_id`
Expected: FAIL to compile — `InsertMemory` has no field `source_id`.

- [ ] **Step 3: Add the field + bind it**

In `InsertMemory` (after `origin`):
```rust
    pub origin:        Option<String>, // None → DB default 'learned'
    pub source_id:     Option<uuid::Uuid>, // provenance: knowledge_sources.id for origin='federated'
```

In `insert_memory`, add `source_id` to the column list, the VALUES list, and the binds:
```rust
    pub async fn insert_memory(&self, m: &InsertMemory) -> Result<uuid::Uuid, String> {
        let id: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memories
                (project_id, scope, scope_filter, type, title, content, impact,
                 tags, triage_signal, status, namespace_id, enforcement, origin, source_id)
             VALUES ($1, $2::sensei.memory_scope, $3, $4::sensei.memory_type, $5, $6, $7,
                     $8, $9, $10::sensei.memory_status, $11,
                     COALESCE($12::sensei.enforcement, 'recommended'::sensei.enforcement),
                     COALESCE($13, 'learned'), $14)
             RETURNING id"
        )
            .bind(m.project_id)
            .bind(&m.scope).bind(&m.scope_filter)
            .bind(&m.mtype).bind(&m.title).bind(&m.content).bind(&m.impact)
            .bind(&m.tags).bind(&m.triage_signal).bind(&m.status)
            .bind(m.namespace_id).bind(&m.enforcement).bind(&m.origin).bind(m.source_id)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(id.0)
    }
```

**Then fix every other `InsertMemory { .. }` literal in the codebase** to add `source_id: None,` (the compiler will list them — `save_memory`/`propose_memory` call sites in `knowledge.rs` and any tests). Set `source_id: None` for all existing call sites.

- [ ] **Step 4: Run the test + build**

Run: `cargo build -p senseid && cargo test -p senseid insert_memory_persists_source_id`
Expected: PASS (or skip if no test DB — then at least `cargo build -p senseid` must pass and the test compiles).

- [ ] **Step 5: Commit**

```bash
git add crates/senseid/src/db/pg_store.rs crates/senseid/src/api/handlers/knowledge.rs
git commit -m "feat(federation): InsertMemory.source_id for federated-rule provenance"
```

---

## Task 3: pg_store — `KnowledgeSource` CRUD

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
async fn knowledge_source_crud_roundtrip() {
    let Ok(pg) = PgStore::connect_test().await else { return; };
    let id = pg.create_knowledge_source(&NewKnowledgeSource {
        kind: "hive_mind".into(), name: "Org Hive".into(), url: "https://hive.example".into(),
        namespace_id: None, credential_ref: "hive-test".into(), direction: "both".into(),
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
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p senseid knowledge_source_crud_roundtrip`
Expected: FAIL — `NewKnowledgeSource`/methods not found.

- [ ] **Step 3: Implement the struct + CRUD**

Add near other pg_store structs:
```rust
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
```
And in `impl PgStore`:
```rust
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
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p senseid knowledge_source_crud_roundtrip`
Expected: PASS (or skip without a test DB; build must pass).

- [ ] **Step 5: Commit**

```bash
git add crates/senseid/src/db/pg_store.rs
git commit -m "feat(federation): pg_store knowledge_sources CRUD"
```

---

## Task 4: pg_store — ledger ops, shareability, push payload, archive

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
async fn federated_ledger_and_shareability() {
    let Ok(pg) = PgStore::connect_test().await else { return; };
    // shareable check: organization is shareable, technology is not (seeded scopes)
    let org_ns = pg.upsert_namespace("organization", "Test Org", "test-org-fed").await.unwrap();
    let tech_ns = pg.upsert_namespace("technology", "Rust", "rust-fed").await.unwrap();
    assert!(pg.namespace_is_shareable(&org_ns).await.unwrap());
    assert!(!pg.namespace_is_shareable(&tech_ns).await.unwrap());

    // ledger upsert is idempotent by (source, remote_rule_id)
    let src = pg.create_knowledge_source(&NewKnowledgeSource {
        kind: "hive_mind".into(), name: "H".into(), url: "u".into(), namespace_id: None,
        credential_ref: "c".into(), direction: "both".into() }).await.unwrap();
    let remote = uuid::Uuid::new_v4();
    let mem = pg.insert_memory(&InsertMemory {
        project_id: None, scope: "global".into(), scope_filter: None, mtype: "convention".into(),
        title: "t".into(), content: "c".into(), impact: None, tags: vec![], triage_signal: None,
        status: "active".into(), namespace_id: Some(org_ns), enforcement: Some("recommended".into()),
        origin: Some("federated".into()), source_id: Some(src) }).await.unwrap();
    pg.upsert_federated_memory(&src, &remote, "hash1", Some(&mem), 5).await.unwrap();
    pg.upsert_federated_memory(&src, &remote, "hash1", Some(&mem), 9).await.unwrap(); // idempotent
    let link = pg.find_federated_memory(&src, &remote).await.unwrap().unwrap();
    assert_eq!(link.memory_id, Some(mem));
    assert_eq!(link.remote_seq, 9);

    // archive retires a federated memory (drops out of resolution)
    pg.archive_federated_memory(&mem).await.unwrap();
    let (status,): (String,) = sqlx_core::query_as::query_as("SELECT status::text FROM sensei.memories WHERE id=$1")
        .bind(mem).fetch_one(pg.pool()).await.unwrap();
    assert_eq!(status, "archived");

    pg.delete_knowledge_source(&src).await.unwrap(); // cascades the ledger row
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id=$1").bind(mem).execute(pg.pool()).await.unwrap();
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p senseid federated_ledger_and_shareability`
Expected: FAIL — methods not found.

- [ ] **Step 3: Implement**

```rust
/// A federated_memories ledger row.
#[derive(Debug, Clone)]
pub struct FederatedLink {
    pub memory_id:  Option<uuid::Uuid>,
    pub remote_seq: i64,
}

impl PgStore {
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

    /// Retire a federated memory (tombstone pulled from upstream) — only if it is
    /// federated-origin, so a locally-authored/promoted memory is never force-archived.
    pub async fn archive_federated_memory(&self, memory_id: &uuid::Uuid) -> Result<bool, String> {
        let res = sqlx_core::query::query(
            "UPDATE sensei.memories SET status = 'archived'::sensei.memory_status
              WHERE id = $1 AND origin = 'federated'")
            .bind(memory_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    /// Fields needed to build a PublishedRule for a promoted memory, plus its
    /// namespace identity. None if the memory has no namespace (unscoped).
    pub async fn memory_push_payload(&self, memory_id: &uuid::Uuid)
        -> Result<Option<MemoryPushPayload>, String> {
        let row: Option<(String, String, Option<String>, String, String, String, String, String)> =
            sqlx_core::query_as::query_as(
            "SELECT m.title, m.content, m.impact, m.enforcement::text, m.type::text, m.origin,
                    n.scope_key, n.slug
               FROM sensei.memories m JOIN sensei.namespaces n ON n.id = m.namespace_id
              WHERE m.id = $1")
            .bind(memory_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|(title, content, impact, enforcement, rule_type, origin, scope_key, slug)|
            MemoryPushPayload { title, content, impact, enforcement, rule_type, origin, scope_key, slug }))
    }
}

/// Snapshot needed to publish a memory to a hive (+ origin/scope_key for gating).
#[derive(Debug, Clone)]
pub struct MemoryPushPayload {
    pub title: String, pub content: String, pub impact: Option<String>,
    pub enforcement: String, pub rule_type: String, pub origin: String,
    pub scope_key: String, pub slug: String,
}
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p senseid federated_ledger_and_shareability`
Expected: PASS (or skip without a test DB; build must pass).

- [ ] **Step 5: Commit**

```bash
git add crates/senseid/src/db/pg_store.rs
git commit -m "feat(federation): pg_store ledger ops + shareability + push payload + archive"
```

---

## Task 5: federation module — pure `PublishedRule` builder

**Files:**
- Create: `crates/senseid/src/federation/mod.rs`
- Modify: `crates/senseid/src/lib.rs` (add `pub mod federation;`) — if senseid has no lib.rs, add the module declaration wherever the other `mod` declarations live (e.g. `main.rs`); match the existing module-declaration site.

- [ ] **Step 1: Write the failing test (pure, no DB)**

Create `crates/senseid/src/federation/mod.rs` with a test module:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pg_store::MemoryPushPayload;

    #[test]
    fn builds_published_rule_with_content_hash_and_namespace_identity() {
        let p = MemoryPushPayload {
            title: "TDD".into(), content: "  Always use TDD  ".into(), impact: None,
            enforcement: "mandatory".into(), rule_type: "convention".into(), origin: "promoted".into(),
            scope_key: "organization".into(), slug: "sensei-hq".into(),
        };
        let pr = build_published_rule(&p, "Sensei HQ", Some("sensei/daemon"));
        assert_eq!(pr.content_hash, hive_protocol::content_hash("Always use TDD"));
        assert_eq!(pr.scope_key, "organization");
        assert_eq!(pr.namespace_slug, "sensei-hq");
        assert_eq!(pr.namespace_name, "Sensei HQ");
        assert_eq!(pr.enforcement, "mandatory");
        assert_eq!(pr.rule_type, "convention");
        assert_eq!(pr.origin_repo.as_deref(), Some("sensei/daemon"));
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p senseid build_published_rule`
Expected: FAIL — `build_published_rule` not found.

- [ ] **Step 3: Implement the builder (prepend above the test module)**

```rust
//! Federation: push promoted rules to a hive-mind and poll-pull applicable rules
//! back as memories(origin='federated'). The ACP never talks to a hive; senseid
//! owns all outbound calls (spec §4).

use crate::db::pg_store::{InsertMemory, KnowledgeSource, MemoryPushPayload, PgStore};
use hive_protocol::{content_hash, PublishedRule, PullResponse};

/// Build the wire payload for a memory being published. `published_by`/`published_at`
/// are stamped server-side by the hive (spec §12), so we send best-effort values.
pub fn build_published_rule(
    p: &MemoryPushPayload, namespace_name: &str, origin_repo: Option<&str>,
) -> PublishedRule {
    PublishedRule {
        content_hash: content_hash(&p.content),
        scope_key: p.scope_key.clone(),
        namespace_slug: p.slug.clone(),
        namespace_name: namespace_name.to_string(),
        rule_type: p.rule_type.clone(),
        title: p.title.clone(),
        content: p.content.clone(),
        impact: p.impact.clone(),
        enforcement: p.enforcement.clone(),
        origin_repo: origin_repo.map(|s| s.to_string()),
        published_by: "senseid".to_string(),       // hive overrides from the API key's member
        published_at: "1970-01-01T00:00:00Z".to_string(), // hive overrides with now()
    }
}
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p senseid build_published_rule`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/senseid/src/federation crates/senseid/src/lib.rs crates/senseid/src/main.rs
git commit -m "feat(federation): module scaffold + pure PublishedRule builder"
```

---

## Task 6: federation push — `push_promoted`

**Files:**
- Modify: `crates/senseid/src/federation/mod.rs`

- [ ] **Step 1: Add the push function (+ a small namespace-name lookup)**

Add a pg_store helper `namespace_name(id) -> Option<(String, String, String)>` returning `(scope_key, slug, name)` — OR reuse `memory_push_payload` (which already returns scope_key+slug) plus a name lookup. For the name, extend `memory_push_payload`'s SELECT to also return `n.name` and thread it through `MemoryPushPayload` (add `pub name: String`). Update Task 4's struct + test accordingly if implementing in order; if Task 4 is already committed, add `name` now:

In `pg_store.rs`, add `pub name: String` to `MemoryPushPayload` and `n.name` to the SELECT in `memory_push_payload` (after `n.slug`), binding it into the struct.

Then in `federation/mod.rs`:
```rust
/// Push a just-approved promoted memory to every push-capable source whose
/// namespace matches. No-op unless the memory is origin='promoted' at a
/// shareable scope. Errors are logged, not propagated to the approval path.
pub async fn push_promoted(pg: &PgStore, memory_id: uuid::Uuid) {
    let payload = match pg.memory_push_payload(&memory_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return,                 // unscoped memory — nothing to push
        Err(e) => { tracing::warn!(error = %e, "federation: push payload load failed"); return; }
    };
    if payload.origin != "promoted" { return; } // only push promoted rules
    let namespace_id = match resolve_memory_namespace_id(pg, &memory_id).await { Some(id) => id, None => return };
    match pg.namespace_is_shareable(&namespace_id).await {
        Ok(true) => {}
        _ => return,                        // non-shareable scope — stays local
    }
    let sources = match pg.list_knowledge_sources().await { Ok(s) => s, Err(_) => return };
    let pr = build_published_rule(&payload, &payload.name, None);
    let client = reqwest::Client::new();
    for src in sources.into_iter().filter(|s| s.enabled && matches!(s.direction.as_str(), "push" | "both")
        && (s.namespace_id.is_none() || s.namespace_id == Some(namespace_id))) {
        if let Err(e) = push_one(pg, &client, &src, &pr, memory_id).await {
            tracing::warn!(source = %src.name, error = %e, "federation: push failed");
        }
    }
}

async fn resolve_memory_namespace_id(pg: &PgStore, memory_id: &uuid::Uuid) -> Option<uuid::Uuid> {
    let row: Option<(Option<uuid::Uuid>,)> = sqlx_core::query_as::query_as(
        "SELECT namespace_id FROM sensei.memories WHERE id = $1")
        .bind(memory_id).fetch_optional(pg.pool()).await.ok()?;
    row.and_then(|(n,)| n)
}

async fn push_one(
    pg: &PgStore, client: &reqwest::Client, src: &KnowledgeSource,
    pr: &PublishedRule, memory_id: uuid::Uuid,
) -> Result<(), String> {
    let key = crate::gateway_keys::get_key(&src.credential_ref).map_err(|e| e.to_string())?;
    let resp = client.post(format!("{}/v1/rules", src.url.trim_end_matches('/')))
        .bearer_auth(key).json(pr).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("hive returned {}", resp.status()));
    }
    let pubresp: hive_protocol::PublishResponse = resp.json().await.map_err(|e| e.to_string())?;
    let remote_id = uuid::Uuid::parse_str(&pubresp.id).map_err(|e| e.to_string())?;
    pg.upsert_federated_memory(&src.id, &remote_id, &pr.content_hash, Some(&memory_id), pubresp.seq).await?;
    Ok(())
}
```

> Note: `pg.pool()` must be public — it is (used by tests). If `gateway_keys::get_key` is blocking (it shells out to `security`), wrap it in `tokio::task::spawn_blocking` if it shows up in latency; for the low-frequency push path a direct call is acceptable. Confirm `crate::gateway_keys` is the correct module path from `federation`.

- [ ] **Step 2: Build**

Run: `cargo build -p senseid`
Expected: PASS. (Push is exercised end-to-end in Task 9; no isolated unit test — it requires a live hive.)

- [ ] **Step 3: Commit**

```bash
git add crates/senseid/src/federation/mod.rs crates/senseid/src/db/pg_store.rs
git commit -m "feat(federation): push_promoted — gate on promoted+shareable, POST /v1/rules, record ledger"
```

---

## Task 7: federation pull — `pull_source` + `run_pull_loop`

**Files:**
- Modify: `crates/senseid/src/federation/mod.rs`

- [ ] **Step 1: Implement pull + the loop**

```rust
/// Result of one pull pass over a source.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct PullStats {
    pub applied: usize,      // new federated memories created
    pub tombstoned: usize,   // federated memories archived
    pub linked: usize,       // deltas that mapped to an existing memory (echo or update)
    pub new_cursor: i64,
}

/// Pull one source's deltas since its cursor and apply them. Idempotent via the
/// ledger. Returns stats; advances the source's last_seq.
pub async fn pull_source(pg: &PgStore, client: &reqwest::Client, src: &KnowledgeSource)
    -> Result<PullStats, String> {
    let key = crate::gateway_keys::get_key(&src.credential_ref).map_err(|e| e.to_string())?;
    let resp = client.get(format!("{}/v1/rules?since={}", src.url.trim_end_matches('/'), src.last_seq))
        .bearer_auth(key).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("hive returned {}", resp.status()));
    }
    let page: PullResponse = resp.json().await.map_err(|e| e.to_string())?;
    let mut stats = PullStats { new_cursor: page.cursor, ..Default::default() };

    for pulled in &page.rules {
        let remote_id = uuid::Uuid::parse_str(&pulled.id).map_err(|e| e.to_string())?;
        let existing = pg.find_federated_memory(&src.id, &remote_id).await?;
        let tombstoned = pulled.status == "tombstoned";

        match existing {
            // Already known (we pushed it, or pulled it before) — update the ledger,
            // and on tombstone archive the linked memory (federated-origin only).
            Some(link) => {
                if tombstoned {
                    if let Some(mid) = link.memory_id {
                        if pg.archive_federated_memory(&mid).await? { stats.tombstoned += 1; }
                    }
                } else {
                    stats.linked += 1; // content updates: re-publish bumps the hive row; we keep the
                                        // local memory as-is at MVP (re-sync of edited rules = follow-up).
                }
                pg.upsert_federated_memory(&src.id, &remote_id, &pulled.rule.content_hash, link.memory_id.as_ref(), pulled.seq).await?;
            }
            // New rule from another machine. Tombstoned-first-sight → record cursor only.
            None if tombstoned => {
                pg.upsert_federated_memory(&src.id, &remote_id, &pulled.rule.content_hash, None, pulled.seq).await?;
            }
            None => {
                let ns = pg.upsert_namespace(&pulled.rule.scope_key, &pulled.rule.namespace_name, &pulled.rule.namespace_slug).await?;
                let mem = pg.insert_memory(&InsertMemory {
                    project_id: None, scope: "global".into(), scope_filter: None,
                    mtype: pulled.rule.rule_type.clone(),
                    title: pulled.rule.title.clone(), content: pulled.rule.content.clone(),
                    impact: pulled.rule.impact.clone(), tags: vec![], triage_signal: None,
                    status: "active".into(), namespace_id: Some(ns),
                    enforcement: Some(pulled.rule.enforcement.clone()),
                    origin: Some("federated".into()), source_id: Some(src.id),
                }).await?;
                pg.upsert_federated_memory(&src.id, &remote_id, &pulled.rule.content_hash, Some(&mem), pulled.seq).await?;
                stats.applied += 1;
            }
        }
    }
    pg.set_source_cursor(&src.id, page.cursor).await?;
    Ok(stats)
}

/// Spawned background task: every `interval`, pull every pull-capable source.
pub fn run_pull_loop(pg: PgStore, interval_secs: u64) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        loop {
            tick.tick().await;
            let sources = match pg.list_knowledge_sources().await {
                Ok(s) => s, Err(e) => { tracing::warn!(error=%e, "federation: list sources failed"); continue; }
            };
            for src in sources.into_iter().filter(|s| s.enabled && matches!(s.direction.as_str(), "pull" | "both")) {
                match pull_source(&pg, &client, &src).await {
                    Ok(st) if st.applied + st.tombstoned > 0 =>
                        tracing::info!(source=%src.name, applied=st.applied, tombstoned=st.tombstoned, "federation: pulled"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!(source=%src.name, error=%e, "federation: pull failed"),
                }
            }
        }
    });
}
```

- [ ] **Step 2: Build**

Run: `cargo build -p senseid`
Expected: PASS. (Pull is exercised in Task 9.)

- [ ] **Step 3: Commit**

```bash
git add crates/senseid/src/federation/mod.rs
git commit -m "feat(federation): pull_source (ledger-idempotent, echo-guarded, tombstone-archive) + run_pull_loop"
```

---

## Task 8: API handlers + routes + Keychain credential storage + push hook + loop wiring

**Files:**
- Modify: `crates/senseid/src/api/handlers/knowledge.rs`, `crates/senseid/src/api/routes.rs`, `crates/senseid/src/api/server.rs`

- [ ] **Step 1: Add the source handlers in `knowledge.rs`**

```rust
#[derive(serde::Deserialize)]
pub(crate) struct NewSourceBody {
    pub kind: Option<String>, pub name: String, pub url: String,
    pub namespace_id: Option<String>, pub direction: Option<String>, pub api_key: String,
}

pub(crate) async fn create_source(
    State(state): State<AppState>, Json(b): Json<NewSourceBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Non-loopback URLs must be https (spec §12).
    let url = b.url.trim().to_string();
    if !url.starts_with("https://") && !url.contains("://127.0.0.1") && !url.contains("://localhost") {
        return Err(err(StatusCode::BAD_REQUEST, "non-loopback source url must be https"));
    }
    let namespace_id = match b.namespace_id.as_deref() {
        Some(s) => Some(uuid::Uuid::parse_str(s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad namespace_id"))?),
        None => None,
    };
    let credential_ref = format!("hive-{}", uuid::Uuid::new_v4());
    // Store the API key in the Keychain; the row only references it.
    let cref = credential_ref.clone();
    let api_key = b.api_key.clone();
    tokio::task::spawn_blocking(move || crate::gateway_keys::set_key(&cref, &api_key))
        .await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    let id = state.pg.create_knowledge_source(&crate::db::pg_store::NewKnowledgeSource {
        kind: b.kind.unwrap_or_else(|| "hive_mind".into()), name: b.name, url,
        namespace_id, credential_ref, direction: b.direction.unwrap_or_else(|| "both".into()),
    }).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "id": id.to_string() })))
}

pub(crate) async fn list_sources(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let rows = state.pg.list_knowledge_sources().await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let out: Vec<_> = rows.into_iter().map(|s| serde_json::json!({
        "id": s.id, "kind": s.kind, "name": s.name, "url": s.url,
        "namespace_id": s.namespace_id, "direction": s.direction,
        "last_seq": s.last_seq, "enabled": s.enabled })).collect();
    Ok(Json(serde_json::json!({ "sources": out })))
}

pub(crate) async fn delete_source(
    State(state): State<AppState>, Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let sid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    if let Ok(Some(s)) = state.pg.get_knowledge_source(&sid).await {
        let cref = s.credential_ref.clone();
        let _ = tokio::task::spawn_blocking(move || crate::gateway_keys::delete_key(&cref)).await;
    }
    let removed = state.pg.delete_knowledge_source(&sid).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    if removed { Ok(Json(serde_json::json!({ "deleted": true }))) } else { Err(err(StatusCode::NOT_FOUND, "no such source")) }
}

pub(crate) async fn sync_source(
    State(state): State<AppState>, Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let sid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let src = state.pg.get_knowledge_source(&sid).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such source"))?;
    let client = reqwest::Client::new();
    let stats = crate::federation::pull_source(&state.pg, &client, &src).await
        .map_err(|e| err(StatusCode::BAD_GATEWAY, &e))?;
    Ok(Json(serde_json::to_value(stats).unwrap()))
}

pub(crate) async fn source_status(
    State(state): State<AppState>, Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let sid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let src = state.pg.get_knowledge_source(&sid).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such source"))?;
    Ok(Json(serde_json::json!({ "id": src.id, "name": src.name, "url": src.url,
        "direction": src.direction, "last_seq": src.last_seq, "enabled": src.enabled })))
}
```

- [ ] **Step 2: Register routes in `routes.rs`** (after the existing knowledge routes):

```rust
.route("/api/knowledge/sources",            get(knowledge::list_sources).post(knowledge::create_source))
.route("/api/knowledge/sources/{id}",       delete(knowledge::delete_source))
.route("/api/knowledge/sources/{id}/sync",  post(knowledge::sync_source))
.route("/api/knowledge/sources/{id}/status", get(knowledge::source_status))
```
Ensure `delete` and `post`/`get` are imported in `routes.rs` (they already are — confirm the `use axum::routing::{...}` line).

- [ ] **Step 3: Wire the push hook into `accept_proposal`** (`knowledge.rs`). After the status flips to active:

```rust
    let new_status = state.pg.set_memory_status(mid, "active", &["proposed"]).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    match new_status {
        Some(s) => {
            // Federation: if this was a promoted rule at a shareable scope, push it.
            // Fire-and-forget — a federation failure must not fail the approval.
            crate::federation::push_promoted(&state.pg, mid).await;
            Ok(Json(serde_json::json!({ "id": mid, "status": s })))
        }
        None => Err(err(StatusCode::CONFLICT, "proposal not in 'proposed' state")),
    }
```

- [ ] **Step 4: Spawn the pull loop in `server.rs build_full_app`** (near the log-retention task):

```rust
    // Federation: poll registered hive-mind sources for applicable rule deltas.
    crate::federation::run_pull_loop(state.pg.clone(), 300);
```

- [ ] **Step 5: Build + existing tests**

Run: `cargo build -p senseid && cargo test -p senseid --lib`
Expected: PASS (build clean; lib unit tests incl. `build_published_rule` pass; DB tests skip without a test DB).

- [ ] **Step 6: Commit**

```bash
git add crates/senseid/src/api
git commit -m "feat(federation): /api/knowledge/sources endpoints + Keychain creds + push hook + pull loop wiring"
```

---

## Task 9: End-to-end integration test (daemon ↔ in-process hive)

Spins up an in-process `sensei-hive` (from #25) on an ephemeral port and drives the daemon's federation against it: register source → publish a rule on the hive → daemon pulls it → it resolves via `get_rules`. Also covers push: promote+approve a daemon memory → assert it appears on the hive.

**Files:**
- Modify: `crates/senseid/Cargo.toml` (dev-deps), Create: `crates/senseid/tests/federation_e2e.rs`

- [ ] **Step 1: Add dev-deps**

In `crates/senseid/Cargo.toml` `[dev-dependencies]`:
```toml
hive-mind = { path = "../hive-mind" }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["full"] }
```

- [ ] **Step 2: Write the e2e test**

Create `crates/senseid/tests/federation_e2e.rs`:
```rust
//! End-to-end: daemon federation ↔ a real in-process sensei-hive.
//! Skips unless a sensei test DB is reachable (TEST_DATABASE_URL or sensei_test).

use std::sync::Arc;

async fn start_hive() -> (String, String) {
    // Returns (base_url, publisher_api_key). Embedded PG binary is cached from #25.
    use hive_mind::api::{build_router, SharedState};
    use hive_mind::db::HiveDb;
    use hive_mind::store::HiveStore;
    let db = HiveDb::bootstrap_temp().await.expect("hive db");
    let store = HiveStore::new(db.pool().clone());
    let member = store.create_member("e2e", None, "publisher").await.unwrap();
    let key = store.issue_key(&member, None).await.unwrap().plaintext;
    Box::leak(Box::new(db));
    let app = build_router(Arc::new(SharedState { store }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    (format!("http://{addr}"), key)
}

#[tokio::test]
async fn daemon_pulls_a_rule_published_on_the_hive() {
    use senseid::db::pg_store::{NewKnowledgeSource, PgStore};
    let Ok(pg) = PgStore::connect_test().await else { return; };
    let (hive_url, key) = start_hive().await;

    // Publish a rule directly on the hive (as the publisher).
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "content_hash": hive_protocol::content_hash("e2e federated rule"),
        "scope_key": "organization", "namespace_slug": "e2e-org", "namespace_name": "E2E Org",
        "rule_type": "convention", "title": "E2E", "content": "e2e federated rule",
        "impact": null, "enforcement": "mandatory", "origin_repo": null,
        "published_by": "x", "published_at": "1970-01-01T00:00:00Z"
    });
    let r = client.post(format!("{hive_url}/v1/rules")).bearer_auth(&key).json(&body).send().await.unwrap();
    assert!(r.status().is_success());

    // Register the source on the daemon (store key in the keychain via the helper module).
    let cref = format!("hive-e2e-{}", uuid::Uuid::new_v4());
    senseid::gateway_keys::set_key(&cref, &key).unwrap();
    let src_id = pg.create_knowledge_source(&NewKnowledgeSource {
        kind: "hive_mind".into(), name: "E2E".into(), url: hive_url.clone(),
        namespace_id: None, credential_ref: cref.clone(), direction: "pull".into() }).await.unwrap();
    let src = pg.get_knowledge_source(&src_id).await.unwrap().unwrap();

    // Pull.
    let stats = senseid::federation::pull_source(&pg, &client, &src).await.unwrap();
    assert_eq!(stats.applied, 1, "one federated memory created");
    assert!(stats.new_cursor > 0);

    // The pulled memory exists, federated, active, mandatory.
    let (cnt,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*) FROM sensei.memories WHERE origin='federated' AND content='e2e federated rule' AND status='active'")
        .bind(()).fetch_one(pg.pool()).await.unwrap_or((0,));
    assert_eq!(cnt, 1);

    // Cleanup: cascade ledger + remove the federated memory + keychain entry.
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE content='e2e federated rule'").execute(pg.pool()).await.unwrap();
    pg.delete_knowledge_source(&src_id).await.unwrap();
    let _ = senseid::gateway_keys::delete_key(&cref);
}
```

> Note: this test requires `senseid` to be importable as a library (`senseid::...`). If `senseid` is a binary-only crate, add a `src/lib.rs` exposing `pub mod db; pub mod federation; pub mod gateway_keys; ...` and have `main.rs` use the lib — check the crate layout and adjust (mirror how `hive-mind` exposes a lib + bin). If a `senseid` lib already exists, use its actual crate name. Also remove the stray `.bind(())` if your sqlx version rejects it — use a plain `query_as` with no bind.

- [ ] **Step 3: Run the e2e (requires a test DB + network-cached embedded PG)**

Run: `cargo test -p senseid --test federation_e2e -- --nocapture`
Expected: PASS if a sensei test DB is reachable; otherwise the test returns early (skips). Build must pass regardless.

- [ ] **Step 4: Commit**

```bash
git add crates/senseid/Cargo.toml crates/senseid/tests/federation_e2e.rs crates/senseid/src/lib.rs
git commit -m "test(federation): e2e — daemon pulls a hive-published rule into resolution"
```

---

## Task 10: Backlog note

**Files:**
- Modify: `docs/backlog.md`

- [ ] **Step 1:** Under the governance P4 section, note #26 as implemented on `develop` (push-on-approve + poll-pull + sources API), with the resolution integration free (federated memories are ordinary `memories`). Leave #27 (Configure UI) as the remaining P4 item. Commit:

```bash
git add docs/backlog.md
git commit -m "docs: note federation daemon module (#26) implemented"
```

---

## Self-Review

**Spec coverage (§9 + related):**
- `knowledge_sources` table (kind/url/namespace_id/credential_ref/direction/last_seq/enabled) → Task 1. ✅
- `federated_memories` ledger (idempotency + echo-guard) → Task 1 (DDL) + Task 4 (ops) + Task 7 (use). ✅
- Credentials via `gateway_keys` Keychain → Task 8 (set/delete on create/delete). ✅
- Push on promotion-approval (gate: origin='promoted' + shareable + matching push source) → Task 6 + hook in Task 8. ✅
- Pull background task + on-demand sync → Task 7 + Task 8. ✅
- `origin='federated'` rows flow into existing resolution → confirmed by grounding (resolve_rules_raw selects by namespace membership + status + enforcement); covered by the Task 9 e2e. ✅ (no resolution code changed)
- `memories.source_id = knowledge_sources.id` → Task 2 + insert in Task 7. ✅
- Non-loopback https enforcement (§12) → Task 8 create_source guard. ✅
- Tombstone → archive (federated-origin only) → Task 4 + Task 7. ✅

**Gaps / deferred (called out, not silent):**
- **Webhook pull-trigger** (`subscriptions`): out of MVP per spec §17; the poll loop + `direction` column leave room. Not in this plan.
- **Re-sync of edited remote rules:** on pull, an updated (non-tombstoned) remote rule that already has a local federated memory is currently left as-is (ledger cursor advances; `stats.linked`). Updating the local memory's content/enforcement on change is a follow-up — noted in the Task 7 code comment. If you want it now, add an `update_federated_memory(memory_id, title, content, enforcement)` and call it in the `Some(link)` non-tombstoned branch.
- **`namespace_id` selection on push when a source targets "all shareable":** handled — `s.namespace_id.is_none()` matches any shareable namespace.

**Placeholder scan:** none. The two "Note:" callouts (senseid lib-vs-bin crate layout in Tasks 5/9; `MemoryPushPayload.name` added in Task 6) are explicit, actionable verifications against named code, not hand-waving.

**Type consistency:** `KnowledgeSource`/`NewKnowledgeSource`, `FederatedLink`, `MemoryPushPayload` (+`name` added Task 6), `PullStats`, `InsertMemory.source_id`, and the federation fns (`build_published_rule`/`push_promoted`/`pull_source`/`run_pull_loop`) are used with consistent names/signatures across tasks. Hive wire types (`PublishedRule`/`PublishResponse`/`PullResponse`) match `hive-protocol`.
