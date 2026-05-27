# Knowledge Plane (Phase 0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the AI-facing + human-facing interaction surface on top of sensei's existing multi-scope memory schema — write APIs, context-assembly read path, outcome feedback loop, and a three-tab Learnings UI.

**Architecture:** All new code lives inside the existing daemon (`crates/senseid/src/knowledge/` + handler in `api/handlers/knowledge.rs`) and the existing MCP server (`crates/mcp/src/main.rs`). Schema delta is additive: two enum values, two new columns, one new enum, one new table, one new trigger function. App Learnings route (`app/src/routes/(observatory)/learnings/`) goes from stub to a real three-tab page driven by a singleton `memoryState.svelte.ts`.

**Tech Stack:** Rust (axum, sqlx, tokio), TypeScript + SvelteKit 5 ($state/$derived), Tauri 2, vitest, Playwright, PostgreSQL (existing `sensei` / `sensei_dev` DB), `dbd` for DDL apply.

**Reference spec:** `docs/superpowers/specs/2026-05-27-knowledge-plane-design.md`

---

## File map

### DDL (edited / created via `dbd`)

| Path | Action | Owns |
|---|---|---|
| `database/ddl/enum/sensei/memory_status.ddl` | Modify | Add `proposed`, `rejected` enum values |
| `database/ddl/enum/sensei/memory_outcome.ddl` | Create | New enum: applied/consulted/violated/ignored |
| `database/ddl/table/sensei/memories.ddl` | Modify | Add `tags text[]`, `triage_signal text` columns + GIN index |
| `database/ddl/table/sensei/memory_outcomes.ddl` | Create | New table |
| `database/ddl/function/sensei/memory_outcome_apply.ddl` | Create | AFTER INSERT trigger function |

### Daemon (Rust)

| Path | Action | Owns |
|---|---|---|
| `crates/senseid/src/knowledge/mod.rs` | Create | Module root, types |
| `crates/senseid/src/db/pg_store.rs` | Modify | Add 7 new methods: `list_memories`, `get_memory_detail`, `assemble_context`, `insert_memory`, `set_memory_status`, `record_outcomes_batch`, `count_memories_by_status` |
| `crates/senseid/src/api/handlers/knowledge.rs` | Create | All 8 HTTP handlers |
| `crates/senseid/src/api/handlers/mod.rs` | Modify | Re-export `knowledge` |
| `crates/senseid/src/api/routes.rs` | Modify | Register 8 new routes |
| `crates/senseid/src/main.rs` | Modify | Add `pub mod knowledge` |
| `crates/senseid/tests/knowledge_api.rs` | Create | Integration tests |

### MCP (Rust)

| Path | Action | Owns |
|---|---|---|
| `crates/mcp/src/main.rs` | Modify | Six new tools: `propose_memory`, `save_memory`, `accept_proposal`, `reject_proposal`, `record_outcome`, `get_layered_context` |

### App (TypeScript / Svelte 5)

| Path | Action | Owns |
|---|---|---|
| `app/src/lib/setup/contracts.ts` | Modify | `Memory`, `MemoryDetail`, `Proposal`, `Outcome` types |
| `app/src/lib/setup/mock-contracts.ts` | Modify | Factory functions for tests |
| `app/src/lib/api.ts` | Modify | 8 new client methods |
| `app/src/lib/memoryState.svelte.ts` | Create | Singleton state — slices for triage/active/archive, mutations |
| `app/src/routes/(observatory)/learnings/+page.svelte` | Modify | Three-tab shell + detail pane |
| `app/src/routes/(observatory)/learnings/TriageList.svelte` | Create | Triage row + Accept/Reject/Edit actions |
| `app/src/routes/(observatory)/learnings/ActiveList.svelte` | Create | Active row + strength meter + counts |
| `app/src/routes/(observatory)/learnings/ArchiveList.svelte` | Create | Archive row (read-only) |
| `app/src/routes/(observatory)/learnings/MemoryDetail.svelte` | Create | Detail pane with outcomes + evidence + examples |
| `app/src/routes/(observatory)/learnings/MemoryEditForm.svelte` | Create | Edit form for accept-with-edits |
| `app/src/lib/memoryState.spec.svelte.ts` | Create | Unit tests for state slice + mutations |
| `app/e2e/tests/learnings-triage.spec.ts` | Create | E2E |
| `app/e2e/tests/learnings-edit.spec.ts` | Create | E2E |
| `app/e2e/tests/learnings-detail.spec.ts` | Create | E2E |
| `app/e2e/tests/learnings-stack-filter.spec.ts` | Create | E2E |

---

## Tasks

### Task 1: Schema — extend `memory_status`, add `memory_outcome` enum

**Files:**
- Modify: `database/ddl/enum/sensei/memory_status.ddl`
- Create: `database/ddl/enum/sensei/memory_outcome.ddl`

- [ ] **Step 1: Edit `memory_status.ddl` to include the two new values**

Replace the file contents with:

```sql
set search_path to sensei, extensions;

create type memory_status
    as enum ('proposed', 'active', 'reinforced', 'challenged',
             'battle_tested', 'archived', 'rejected');
```

- [ ] **Step 2: Create `memory_outcome.ddl`**

```sql
set search_path to sensei, extensions;

create type memory_outcome
    as enum ('applied', 'consulted', 'violated', 'ignored');
```

- [ ] **Step 3: Apply via dbd**

Run: `dbd apply` (from repo root, dev DB)

Expected output: includes `CREATE TYPE memory_outcome` and an `ALTER TYPE memory_status ADD VALUE` line for each new value. Exit 0.

- [ ] **Step 4: Verify with psql**

Run: `psql sensei_dev -c "SELECT unnest(enum_range(NULL::sensei.memory_status))"`
Expected: 7 rows, including `proposed` and `rejected`.

Run: `psql sensei_dev -c "SELECT unnest(enum_range(NULL::sensei.memory_outcome))"`
Expected: 4 rows: `applied`, `consulted`, `violated`, `ignored`.

- [ ] **Step 5: Commit**

```bash
git add database/ddl/enum/sensei/memory_status.ddl \
        database/ddl/enum/sensei/memory_outcome.ddl
git commit -m "feat(db): extend memory_status, add memory_outcome enum"
```

---

### Task 2: Schema — add `tags` and `triage_signal` columns to `memories` + GIN index

**Files:**
- Modify: `database/ddl/table/sensei/memories.ddl`

- [ ] **Step 1: Edit `memories.ddl` — add two columns and the GIN index**

Replace the table definition with:

```sql
set search_path to sensei, extensions;

create table if not exists memories (
  id                       uuid          primary key default gen_random_uuid()
, project_id               uuid          references sensei.projects(id) on delete cascade
, scope                    memory_scope  not null default 'project'
, scope_filter             text
, type                     memory_type   not null
, title                    text          not null
, content                  text          not null
, impact                   text
, strength                 real          not null default 1.0
, status                   memory_status not null default 'active'
, reinforced_count         integer       not null default 0
, violated_count           integer       not null default 0
, last_relevant_at         timestamptz
, session_id               uuid
, tags                     text[]        not null default '{}'
, triage_signal            text
, modified_at              timestamptz   not null default now()
);

create index if not exists memories_project_id_idx
    on memories(project_id, scope, status);

create index if not exists memories_scope_idx
    on memories(scope, scope_filter)
 where status = 'active';

create index if not exists memories_strength_idx
    on memories(strength desc)
 where status = 'active';

create index if not exists memories_tags_idx
    on memories using gin (tags);
```

Keep all existing `comment on column` statements; add two new ones at the bottom:

```sql
comment on column memories.tags
     is 'Free-form tags (e.g. security, performance, compliance). GIN-indexed for &&/@> filters.';
comment on column memories.triage_signal
     is 'Which capture heuristic surfaced this memory (revert/correction/actually/repeat_pattern/override/test_failure). Null for explicit /save.';
```

- [ ] **Step 2: Apply and verify**

Run: `dbd apply`
Expected: includes `ALTER TABLE sensei.memories ADD COLUMN tags` and `... ADD COLUMN triage_signal`, plus `CREATE INDEX memories_tags_idx`.

Run: `psql sensei_dev -c "\d sensei.memories" | grep -E 'tags|triage_signal'`
Expected: two matching column lines.

- [ ] **Step 3: Commit**

```bash
git add database/ddl/table/sensei/memories.ddl
git commit -m "feat(db): add tags + triage_signal to memories"
```

---

### Task 3: Schema — create `memory_outcomes` table

**Files:**
- Create: `database/ddl/table/sensei/memory_outcomes.ddl`

- [ ] **Step 1: Write the table DDL**

```sql
set search_path to sensei, extensions;

create table if not exists memory_outcomes (
  id            uuid              primary key default gen_random_uuid()
, memory_id     uuid              not null references sensei.memories(id) on delete cascade
, session_id    uuid              references activity.sessions(id) on delete set null
, outcome       memory_outcome    not null
, context       text
, recorded_at   timestamptz       not null default now()
);

create index if not exists memory_outcomes_memory_id_idx
    on memory_outcomes(memory_id, recorded_at desc);

comment on table memory_outcomes is
'Per-memory event log: applied/consulted/violated/ignored.
Insert triggers update memories.reinforced_count / violated_count / strength / status.';

comment on column memory_outcomes.memory_id
     is 'Foreign key to memories — which memory this event is about.';
comment on column memory_outcomes.session_id
     is 'Foreign key to activity.sessions. Null when unknown or after the session is deleted.';
comment on column memory_outcomes.outcome
     is 'What happened: applied (used in output), consulted (loaded but not used), violated (user overruled), ignored (loaded but discarded).';
comment on column memory_outcomes.context
     is 'Optional free-form note (e.g. file path, brief reason).';
comment on column memory_outcomes.recorded_at
     is 'When the event was recorded (server clock).';
```

- [ ] **Step 2: Apply and verify**

Run: `dbd apply`
Expected: `CREATE TABLE sensei.memory_outcomes` and `CREATE INDEX memory_outcomes_memory_id_idx`.

Run: `psql sensei_dev -c "\d sensei.memory_outcomes"`
Expected: table listing with all six columns, FK to `sensei.memories(id)` and `activity.sessions(id)`.

- [ ] **Step 3: Commit**

```bash
git add database/ddl/table/sensei/memory_outcomes.ddl
git commit -m "feat(db): add memory_outcomes table"
```

---

### Task 4: Schema — trigger function that updates memory state on outcome insert

**Files:**
- Create: `database/ddl/function/sensei/memory_outcome_apply.ddl`

- [ ] **Step 1: Write the trigger function + trigger binding**

```sql
set search_path to sensei, extensions;

create or replace function sensei.memory_outcome_apply()
    returns trigger
    language plpgsql as
$$
declare
    last_violated_at  timestamptz;
    consec_applied    integer;
    cur_status        memory_status;
begin
    if NEW.outcome = 'applied' then
        select status into cur_status from sensei.memories where id = NEW.memory_id;

        update sensei.memories
           set reinforced_count = reinforced_count + 1
             , strength         = least(strength + 0.5, 5.0)
             , last_relevant_at = now()
             , modified_at      = now()
         where id = NEW.memory_id;

        if cur_status = 'challenged' then
            -- Recover to 'reinforced' only after 3 consecutive applied since last violation.
            select recorded_at
              into last_violated_at
              from sensei.memory_outcomes
             where memory_id = NEW.memory_id
               and outcome = 'violated'
             order by recorded_at desc
             limit 1;

            select count(*) into consec_applied
              from sensei.memory_outcomes
             where memory_id = NEW.memory_id
               and outcome = 'applied'
               and (last_violated_at is null or recorded_at > last_violated_at);

            if consec_applied >= 3 then
                update sensei.memories set status = 'reinforced' where id = NEW.memory_id;
            end if;
        else
            -- Promote to battle_tested when strength >= 4.0 and never violated.
            update sensei.memories
               set status = 'battle_tested'
             where id = NEW.memory_id
               and strength >= 4.0
               and violated_count = 0
               and status in ('active', 'reinforced');
        end if;

    elsif NEW.outcome = 'violated' then
        update sensei.memories
           set violated_count = violated_count + 1
             , strength       = greatest(strength - 0.7, 0.0)
             , status         = case when greatest(strength - 0.7, 0.0) < 1.0
                                     then 'archived'::memory_status
                                     else 'challenged'::memory_status end
             , last_relevant_at = now()
             , modified_at      = now()
         where id = NEW.memory_id;

    elsif NEW.outcome = 'consulted' then
        update sensei.memories
           set last_relevant_at = now()
             , modified_at      = now()
         where id = NEW.memory_id;

    -- ignored: no-op
    end if;

    return NEW;
end;
$$;

drop trigger if exists memory_outcome_apply_trg on sensei.memory_outcomes;
create trigger memory_outcome_apply_trg
    after insert on sensei.memory_outcomes
    for each row
    execute function sensei.memory_outcome_apply();
```

- [ ] **Step 2: Apply via dbd**

Run: `dbd apply`
Expected: `CREATE FUNCTION sensei.memory_outcome_apply` and `CREATE TRIGGER memory_outcome_apply_trg`.

- [ ] **Step 3: Smoke-test the trigger with psql**

Run:

```bash
psql sensei_dev <<'SQL'
-- Create a throwaway project + memory, insert outcomes, observe state.
INSERT INTO sensei.projects (id, name, abs_path) VALUES
  ('11111111-1111-1111-1111-111111111111', 'trig-test', '/tmp/trig-test')
  ON CONFLICT (id) DO NOTHING;

INSERT INTO sensei.memories (id, project_id, type, title, content, status, strength)
VALUES ('22222222-2222-2222-2222-222222222222',
        '11111111-1111-1111-1111-111111111111',
        'convention', 't', 'c', 'active', 1.0);

INSERT INTO sensei.memory_outcomes (memory_id, outcome) VALUES
  ('22222222-2222-2222-2222-222222222222', 'applied');

SELECT strength, status, reinforced_count
  FROM sensei.memories WHERE id = '22222222-2222-2222-2222-222222222222';

-- Cleanup
DELETE FROM sensei.memories WHERE id = '22222222-2222-2222-2222-222222222222';
DELETE FROM sensei.projects WHERE id = '11111111-1111-1111-1111-111111111111';
SQL
```

Expected: row shows `strength=1.5`, `status=active`, `reinforced_count=1`.

- [ ] **Step 4: Commit**

```bash
git add database/ddl/function/sensei/memory_outcome_apply.ddl
git commit -m "feat(db): memory_outcomes trigger updates strength + status"
```

---

### Task 5: DB layer — list, detail, context-assembly queries

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs`

- [ ] **Step 1: Write tests in `crates/senseid/src/db/pg_store.rs` (bottom of file in `#[cfg(test)] mod tests`)**

Add:

```rust
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
        }).await.unwrap();
        let _m2 = pg.insert_memory(&InsertMemory {
            project_id: Some(project_id), scope: "project".into(), scope_filter: None,
            mtype: "convention".into(), title: "t2".into(), content: "c2".into(),
            impact: None, tags: vec![], triage_signal: None, status: "active".into(),
        }).await.unwrap();

        let proposed = pg.list_memories(Some(project_id), Some("proposed"), None, 50).await.unwrap();
        assert_eq!(proposed.len(), 1);
        assert_eq!(proposed[0]["id"].as_str().unwrap(), m1.to_string());
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

Run: `SENSEI_TEST_DB_URL=postgres://localhost/sensei_dev cargo test -p senseid --features dev knowledge_tests -- --nocapture`
Expected: compilation error (`insert_memory`, `list_memories`, `InsertMemory`, `ensure_test_project` not found).

- [ ] **Step 3: Add `InsertMemory` struct and methods to `pg_store.rs`**

Above the `impl PgStore` block, add the struct:

```rust
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
}
```

Inside `impl PgStore`, add (placed just below `get_project_memories`):

```rust
pub async fn ensure_test_project(&self, name: &str) -> Result<uuid::Uuid, String> {
    let id = uuid::Uuid::new_v4();
    sqlx_core::query::query(
        "INSERT INTO sensei.projects (id, name, abs_path)
         VALUES ($1, $2, $3)
         ON CONFLICT (id) DO NOTHING"
    ).bind(id).bind(name).bind(format!("/tmp/{name}-{id}"))
     .execute(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(id)
}

pub async fn insert_memory(&self, m: &InsertMemory) -> Result<uuid::Uuid, String> {
    let id: (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.memories
            (project_id, scope, scope_filter, type, title, content, impact,
             tags, triage_signal, status)
         VALUES ($1, $2::memory_scope, $3, $4::memory_type, $5, $6, $7,
                 $8, $9, $10::memory_status)
         RETURNING id"
    )
        .bind(m.project_id)
        .bind(&m.scope).bind(&m.scope_filter)
        .bind(&m.mtype).bind(&m.title).bind(&m.content).bind(&m.impact)
        .bind(&m.tags).bind(&m.triage_signal).bind(&m.status)
        .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(id.0)
}

pub async fn list_memories(
    &self,
    project_id: Option<uuid::Uuid>,
    status:     Option<&str>,
    scope:      Option<&str>,
    limit:      i64,
) -> Result<Vec<serde_json::Value>, String> {
    // Build a single query with optional filters. Always order by strength desc, last_relevant_at desc.
    let rows: Vec<(uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, String, String,
                   Option<String>, f64, String, i32, i32,
                   Option<chrono::DateTime<chrono::Utc>>, Vec<String>, Option<String>,
                   chrono::DateTime<chrono::Utc>)> =
        sqlx_core::query_as::query_as(
            "SELECT id, project_id, scope::text, scope_filter, type::text, title, content,
                    impact, strength, status::text, reinforced_count, violated_count,
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
```

- [ ] **Step 4: Run tests — verify they pass**

Run: `SENSEI_TEST_DB_URL=postgres://localhost/sensei_dev cargo test -p senseid --features dev knowledge_tests -- --nocapture`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/senseid/src/db/pg_store.rs
git commit -m "feat(db): insert_memory + list_memories + ensure_test_project helpers"
```

---

### Task 6: DB layer — memory detail (with evidence/examples/outcomes) and status mutations

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs`

- [ ] **Step 1: Add failing test**

Inside the existing `knowledge_tests` module:

```rust
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
    }).await.unwrap();
    let skipped = pg.record_outcomes_batch(&[
        OutcomeRow { memory_id: mid, session_id: None, outcome: "applied".into(), context: None }
    ]).await.unwrap();
    assert_eq!(skipped.len(), 0);

    let detail = pg.get_memory_detail(mid).await.unwrap();
    assert!(detail["memory"]["id"].as_str().unwrap() == mid.to_string());
    assert_eq!(detail["outcomes"].as_array().unwrap().len(), 1);
}
```

- [ ] **Step 2: Run — expect failure**

Run: `SENSEI_TEST_DB_URL=postgres://localhost/sensei_dev cargo test -p senseid --features dev knowledge_tests`
Expected: compile failure on `set_memory_status`, `get_memory_detail`, `record_outcomes_batch`, `OutcomeRow`.

- [ ] **Step 3: Add `OutcomeRow` and three new methods**

```rust
pub struct OutcomeRow {
    pub memory_id:  uuid::Uuid,
    pub session_id: Option<uuid::Uuid>,
    pub outcome:    String,
    pub context:    Option<String>,
}

impl PgStore {
    // ... existing methods ...

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
                SET status      = $1::memory_status
                  , modified_at = now()
              WHERE id = $2
                AND status::text = ANY($3)
              RETURNING status::text"
        )
            .bind(to_status).bind(memory_id).bind(&from_owned)
            .fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.map(|r| r.0))
    }

    /// Full memory detail bundle: row + evidence + examples + recent outcomes.
    pub async fn get_memory_detail(&self, id: uuid::Uuid) -> Result<serde_json::Value, String> {
        // Dedicated by-id query (LIMIT 1 on list_memories cannot find a specific id).
        let row: Option<(uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, String, String,
                         Option<String>, f64, String, i32, i32,
                         Option<chrono::DateTime<chrono::Utc>>, Vec<String>, Option<String>,
                         chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, project_id, scope::text, scope_filter, type::text, title, content,
                        impact, strength, status::text, reinforced_count, violated_count,
                        last_relevant_at, tags, triage_signal, modified_at
                   FROM sensei.memories WHERE id = $1"
            ).bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        let r = row.ok_or_else(|| format!("memory {id} not found"))?;
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
        });

        // Evidence
        let evidence: Vec<(Option<String>, Option<String>, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT url, note, recorded_at
                   FROM sensei.memory_evidence
                  WHERE memory_id = $1
                  ORDER BY recorded_at DESC"
            ).bind(id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        // Examples
        let examples: Vec<(Option<String>, Option<bool>, Option<bool>, Option<String>)> =
            sqlx_core::query_as::query_as(
                "SELECT node_id, is_good, is_bad, note
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

        Ok(serde_json::json!({
            "memory":   memory,
            "evidence": evidence.into_iter().map(|(url, note, ts)|
                serde_json::json!({ "url": url, "note": note, "recorded_at": ts.to_rfc3339() })
            ).collect::<Vec<_>>(),
            "examples": examples.into_iter().map(|(node, good, bad, note)|
                serde_json::json!({ "node_id": node, "is_good": good, "is_bad": bad, "note": note })
            ).collect::<Vec<_>>(),
            "outcomes": outcomes.into_iter().map(|(outcome, sess, ctx, ts)|
                serde_json::json!({ "outcome": outcome, "session_id": sess, "context": ctx, "recorded_at": ts.to_rfc3339() })
            ).collect::<Vec<_>>(),
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
                 VALUES ($1, $2, $3::memory_outcome, $4)"
            )
                .bind(r.memory_id).bind(r.session_id).bind(&r.outcome).bind(&r.context)
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(skipped)
    }
}
```

- [ ] **Step 4: Run tests**

Run: `SENSEI_TEST_DB_URL=postgres://localhost/sensei_dev cargo test -p senseid --features dev knowledge_tests`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/senseid/src/db/pg_store.rs
git commit -m "feat(db): set_memory_status + get_memory_detail + record_outcomes_batch"
```

---

### Task 7: DB layer — `assemble_context` (three-scope blend)

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs`

- [ ] **Step 1: Add failing test**

```rust
#[tokio::test]
async fn assemble_context_blends_three_scopes() {
    if ddl_test_skip() { return; }
    let pg = PgStore::connect(&std::env::var("SENSEI_TEST_DB_URL").unwrap()).await.unwrap();
    let pid = pg.ensure_test_project("blend").await.unwrap();

    pg.insert_memory(&InsertMemory {
        project_id: Some(pid), scope: "project".into(), scope_filter: None,
        mtype: "convention".into(), title: "P".into(), content: "p".into(),
        impact: None, tags: vec![], triage_signal: None, status: "active".into(),
    }).await.unwrap();
    pg.insert_memory(&InsertMemory {
        project_id: None, scope: "stack".into(), scope_filter: Some("rust".into()),
        mtype: "convention".into(), title: "S".into(), content: "s".into(),
        impact: None, tags: vec![], triage_signal: None, status: "active".into(),
    }).await.unwrap();
    pg.insert_memory(&InsertMemory {
        project_id: None, scope: "global".into(), scope_filter: None,
        mtype: "convention".into(), title: "G".into(), content: "g".into(),
        impact: None, tags: vec![], triage_signal: None, status: "active".into(),
    }).await.unwrap();

    let blob = pg.assemble_context(pid, &["rust".into()], None, 50).await.unwrap();
    let titles: Vec<String> = blob["memories"].as_array().unwrap().iter()
        .map(|m| m["title"].as_str().unwrap().to_string()).collect();
    assert!(titles.contains(&"P".to_string()));
    assert!(titles.contains(&"S".to_string()));
    assert!(titles.contains(&"G".to_string()));
    // Pin a memory with status='proposed'; it must not appear.
    let m_prop = pg.insert_memory(&InsertMemory {
        project_id: Some(pid), scope: "project".into(), scope_filter: None,
        mtype: "convention".into(), title: "PROP".into(), content: "x".into(),
        impact: None, tags: vec![], triage_signal: Some("revert".into()),
        status: "proposed".into(),
    }).await.unwrap();
    let blob2 = pg.assemble_context(pid, &["rust".into()], None, 50).await.unwrap();
    let titles2: Vec<String> = blob2["memories"].as_array().unwrap().iter()
        .map(|m| m["title"].as_str().unwrap().to_string()).collect();
    assert!(!titles2.contains(&"PROP".to_string()));
    let _ = m_prop;
}
```

- [ ] **Step 2: Run — verify failure**

Run: `SENSEI_TEST_DB_URL=postgres://localhost/sensei_dev cargo test -p senseid --features dev knowledge_tests::assemble_context`
Expected: compile error — `assemble_context` not defined.

- [ ] **Step 3: Implement**

Inside `impl PgStore`:

```rust
pub async fn assemble_context(
    &self,
    project_id: uuid::Uuid,
    stack_ids:  &[String],
    tags:       Option<&[String]>,
    limit:      i64,
) -> Result<serde_json::Value, String> {
    let allowed = ["active", "reinforced", "battle_tested", "challenged"];
    let allowed_owned: Vec<String> = allowed.iter().map(|s| s.to_string()).collect();
    let stack_owned: Vec<String> = stack_ids.to_vec();
    let tags_owned: Option<Vec<String>> = tags.map(|t| t.to_vec());

    // Single query — three scope branches OR'd together, deduplicated by id.
    let rows: Vec<(uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, String, String,
                   Option<String>, f64, String, i32, i32,
                   Option<chrono::DateTime<chrono::Utc>>, Vec<String>, Option<String>,
                   chrono::DateTime<chrono::Utc>)> =
        sqlx_core::query_as::query_as(
            "SELECT id, project_id, scope::text, scope_filter, type::text, title, content,
                    impact, strength, status::text, reinforced_count, violated_count,
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

    let memories: Vec<serde_json::Value> = rows.into_iter().map(|r| serde_json::json!({
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
```

- [ ] **Step 4: Run tests**

Run: `SENSEI_TEST_DB_URL=postgres://localhost/sensei_dev cargo test -p senseid --features dev knowledge_tests`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/senseid/src/db/pg_store.rs
git commit -m "feat(db): assemble_context blends project + stack + global"
```

---

### Task 8: Daemon — knowledge HTTP handlers (all 8 endpoints)

**Files:**
- Create: `crates/senseid/src/api/handlers/knowledge.rs`
- Modify: `crates/senseid/src/api/handlers/mod.rs`
- Modify: `crates/senseid/src/api/routes.rs`

- [ ] **Step 1: Create the handlers file**

```rust
//! `/api/knowledge/*` — memory CRUD, proposals, outcomes, context assembly.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;
use crate::db::pg_store::{InsertMemory, OutcomeRow};

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": msg })))
}

// ============================================================================
// GET /api/knowledge/memories?status=&scope=&project_id=&limit=
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct ListQuery {
    pub status:     Option<String>,
    pub scope:      Option<String>,
    pub project_id: Option<String>,
    pub limit:      Option<i64>,
}

pub(crate) async fn list_memories(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pid = match q.project_id {
        Some(s) => Some(uuid::Uuid::parse_str(&s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?),
        None => None,
    };
    let rows = state.pg.list_memories(pid, q.status.as_deref(), q.scope.as_deref(), q.limit.unwrap_or(200))
        .await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "memories": rows })))
}

// ============================================================================
// GET /api/knowledge/memories/:id
// ============================================================================

pub(crate) async fn get_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let detail = state.pg.get_memory_detail(mid).await
        .map_err(|e| {
            if e.contains("not found") { err(StatusCode::NOT_FOUND, "memory not found") }
            else { err(StatusCode::INTERNAL_SERVER_ERROR, &e) }
        })?;
    Ok(Json(detail))
}

// ============================================================================
// GET /api/knowledge/context?project_id=&limit=&tags=csv
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct ContextQuery {
    pub project_id: String,
    pub limit:      Option<i64>,
    pub tags:       Option<String>,   // comma-separated
}

pub(crate) async fn get_context(
    State(state): State<AppState>,
    Query(q): Query<ContextQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pid = uuid::Uuid::parse_str(&q.project_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?;
    let tags: Option<Vec<String>> = q.tags.map(|s|
        s.split(',').filter(|t| !t.trim().is_empty()).map(|t| t.trim().to_string()).collect()
    );
    let stack_ids = state.pg.get_project_stack_ids(&pid).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let blob = state.pg.assemble_context(pid, &stack_ids, tags.as_deref(), q.limit.unwrap_or(200))
        .await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(blob))
}

// ============================================================================
// POST /api/knowledge/proposals  — propose_memory
// POST /api/knowledge/memories   — save_memory (explicit)
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct MemoryBody {
    pub project_id:    Option<String>,
    pub scope:         String,
    pub scope_filter:  Option<String>,
    #[serde(rename = "type")]
    pub mtype:         String,
    pub title:         String,
    pub content:       String,
    pub impact:        Option<String>,
    #[serde(default)]
    pub tags:          Vec<String>,
    pub triage_signal: Option<String>,
}

async fn insert_with_status(
    state: AppState,
    body:  MemoryBody,
    status: &str,
    require_triage_signal: bool,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if body.title.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "title must not be empty"));
    }
    if body.content.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "content must not be empty"));
    }
    if body.scope == "stack" && body.scope_filter.as_deref().unwrap_or("").is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "scope_filter required for scope='stack'"));
    }
    if require_triage_signal && body.triage_signal.as_deref().unwrap_or("").is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "triage_signal required for proposals"));
    }
    let pid = match body.project_id {
        Some(s) => Some(uuid::Uuid::parse_str(&s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?),
        None => None,
    };
    let id = state.pg.insert_memory(&InsertMemory {
        project_id:    pid,
        scope:         body.scope,
        scope_filter:  body.scope_filter,
        mtype:         body.mtype,
        title:         body.title,
        content:       body.content,
        impact:        body.impact,
        tags:          body.tags,
        triage_signal: body.triage_signal,
        status:        status.into(),
    }).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "id": id, "status": status })))
}

pub(crate) async fn propose_memory(
    State(state): State<AppState>,
    Json(body):   Json<MemoryBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    insert_with_status(state, body, "proposed", true).await
}

pub(crate) async fn save_memory(
    State(state): State<AppState>,
    Json(body):   Json<MemoryBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    insert_with_status(state, body, "active", false).await
}

// ============================================================================
// POST /api/knowledge/proposals/:id/accept
// POST /api/knowledge/proposals/:id/reject
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct AcceptBody {
    pub edits: Option<MemoryBody>,    // optional override of fields
}

pub(crate) async fn accept_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(_body): Json<serde_json::Value>,    // edits not yet applied in Phase 0; accept transitions only
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let new_status = state.pg.set_memory_status(mid, "active", &["proposed"]).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    match new_status {
        Some(s) => Ok(Json(serde_json::json!({ "id": mid, "status": s }))),
        None => Err(err(StatusCode::CONFLICT, "proposal not in 'proposed' state")),
    }
}

#[derive(Deserialize)]
pub(crate) struct RejectBody {
    pub reason: Option<String>,
}

pub(crate) async fn reject_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(_body): Json<RejectBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let new_status = state.pg.set_memory_status(mid, "rejected", &["proposed"]).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    match new_status {
        Some(s) => Ok(Json(serde_json::json!({ "id": mid, "status": s }))),
        None => Err(err(StatusCode::CONFLICT, "proposal not in 'proposed' state")),
    }
}

// ============================================================================
// POST /api/knowledge/outcomes
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct OutcomeBody {
    pub memory_id:  String,
    pub outcome:    String,    // applied | consulted | violated | ignored
    pub session_id: Option<String>,
    pub context:    Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct OutcomesBatch {
    pub outcomes: Vec<OutcomeBody>,
}

pub(crate) async fn record_outcomes(
    State(state): State<AppState>,
    Json(body):   Json<OutcomesBatch>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let valid_outcomes = ["applied", "consulted", "violated", "ignored"];
    let mut rows: Vec<OutcomeRow> = Vec::with_capacity(body.outcomes.len());
    for o in body.outcomes {
        if !valid_outcomes.contains(&o.outcome.as_str()) {
            return Err(err(StatusCode::BAD_REQUEST, &format!("invalid outcome: {}", o.outcome)));
        }
        let mid = uuid::Uuid::parse_str(&o.memory_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "bad memory_id"))?;
        let sess = match o.session_id {
            Some(s) => Some(uuid::Uuid::parse_str(&s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad session_id"))?),
            None => None,
        };
        rows.push(OutcomeRow {
            memory_id: mid, session_id: sess, outcome: o.outcome, context: o.context,
        });
    }
    let total = rows.len();
    let skipped = state.pg.record_outcomes_batch(&rows).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({
        "recorded": total - skipped.len(),
        "skipped":  skipped,
    })))
}
```

- [ ] **Step 2: Re-export in `handlers/mod.rs`**

Add a line:

```rust
pub(crate) mod knowledge;
```

- [ ] **Step 3: Add `get_project_stack_ids` to `pg_store.rs`**

```rust
pub async fn get_project_stack_ids(&self, project_id: &uuid::Uuid) -> Result<Vec<String>, String> {
    let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
        "SELECT stack_id FROM sensei.project_stacks WHERE project_id = $1"
    ).bind(project_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}
```

If the actual table name differs (`project_stacks` vs. column on `projects`), substitute the equivalent — confirm with `psql sensei_dev -c "\dt sensei.*"`. The integration test in Task 9 will surface a mismatch.

- [ ] **Step 4: Register routes in `routes.rs`**

Insert in the router builder, alongside the existing routes:

```rust
        .route("/api/knowledge/memories",                  get(knowledge::list_memories).post(knowledge::save_memory))
        .route("/api/knowledge/memories/{id}",             get(knowledge::get_memory))
        .route("/api/knowledge/proposals",                 post(knowledge::propose_memory))
        .route("/api/knowledge/proposals/{id}/accept",     post(knowledge::accept_proposal))
        .route("/api/knowledge/proposals/{id}/reject",     post(knowledge::reject_proposal))
        .route("/api/knowledge/outcomes",                  post(knowledge::record_outcomes))
        .route("/api/knowledge/context",                   get(knowledge::get_context))
```

And add at the top of `routes.rs`:

```rust
use crate::api::handlers::knowledge;
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build --features dev -p senseid`
Expected: clean build.

- [ ] **Step 6: Commit**

```bash
git add crates/senseid/src/api/handlers/knowledge.rs \
        crates/senseid/src/api/handlers/mod.rs \
        crates/senseid/src/api/routes.rs \
        crates/senseid/src/db/pg_store.rs
git commit -m "feat(daemon): /api/knowledge/* endpoints"
```

---

### Task 9: Integration test — full HTTP lifecycle

**Files:**
- Create: `crates/senseid/tests/knowledge_api.rs`

- [ ] **Step 1: Write the integration test**

```rust
//! End-to-end: HTTP request → daemon → DB → response.
//! Requires SENSEI_TEST_DB_URL pointing at a running sensei_dev.

use reqwest::Client;
use serde_json::json;

fn base_url() -> Option<String> { std::env::var("SENSEI_API_URL").ok() }

#[tokio::test]
async fn full_lifecycle_proposal_accept_outcome() {
    let Some(url) = base_url() else { return; };
    let c = Client::new();

    // Use the existing test-project bootstrap by hitting /api/projects (placeholder —
    // adjust to whatever endpoint exists in the suite). For simplicity, target a known
    // project_id from local dev or skip when env var SENSEI_TEST_PROJECT_ID is unset.
    let Some(pid) = std::env::var("SENSEI_TEST_PROJECT_ID").ok() else { return; };

    // 1. Propose
    let propose = c.post(format!("{url}/api/knowledge/proposals"))
        .json(&json!({
            "project_id":    pid,
            "scope":         "project",
            "type":          "convention",
            "title":         "lifecycle-test",
            "content":       "do the thing",
            "tags":          ["test"],
            "triage_signal": "revert",
        }))
        .send().await.unwrap();
    assert!(propose.status().is_success(), "propose status: {}", propose.status());
    let mid = propose.json::<serde_json::Value>().await.unwrap()["id"].as_str().unwrap().to_string();

    // 2. It appears in status=proposed
    let listing = c.get(format!("{url}/api/knowledge/memories?status=proposed&project_id={pid}"))
        .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    let titles: Vec<&str> = listing["memories"].as_array().unwrap().iter()
        .map(|m| m["title"].as_str().unwrap()).collect();
    assert!(titles.contains(&"lifecycle-test"));

    // 3. Accept
    let accept = c.post(format!("{url}/api/knowledge/proposals/{mid}/accept"))
        .json(&json!({})).send().await.unwrap();
    assert!(accept.status().is_success(), "accept: {}", accept.status());

    // 4. Context now contains it
    let ctx = c.get(format!("{url}/api/knowledge/context?project_id={pid}"))
        .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    let ctx_titles: Vec<&str> = ctx["memories"].as_array().unwrap().iter()
        .map(|m| m["title"].as_str().unwrap()).collect();
    assert!(ctx_titles.contains(&"lifecycle-test"));

    // 5. Record an outcome
    let outcome = c.post(format!("{url}/api/knowledge/outcomes"))
        .json(&json!({"outcomes":[{"memory_id":mid, "outcome":"applied"}]}))
        .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    assert_eq!(outcome["recorded"].as_i64().unwrap(), 1);
    assert_eq!(outcome["skipped"].as_array().unwrap().len(), 0);

    // 6. Detail shows the outcome
    let detail = c.get(format!("{url}/api/knowledge/memories/{mid}"))
        .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    assert_eq!(detail["outcomes"].as_array().unwrap().len(), 1);
}
```

- [ ] **Step 2: Boot the dev daemon and run the test**

```bash
make install-dev
senseid-dev start
SENSEI_API_URL=http://127.0.0.1:7745 \
  SENSEI_TEST_PROJECT_ID=<any-real-project-id-from-your-dev-db> \
  cargo test --features dev -p senseid --test knowledge_api -- --nocapture
```

Expected: 1 passed (or skipped when env vars absent).

- [ ] **Step 3: Commit**

```bash
git add crates/senseid/tests/knowledge_api.rs
git commit -m "test(daemon): knowledge HTTP lifecycle integration test"
```

---

### Task 10: MCP — six new tools

**Files:**
- Modify: `crates/mcp/src/main.rs`

- [ ] **Step 1: Locate the tool-registration block**

The file has tool definitions in a single registry. Search for an existing tool like `log_event` to see the pattern. Each tool has: name, description, input schema, handler closure that calls the daemon via the shared `client`.

- [ ] **Step 2: Add the six tools — paste after the last existing tool**

```rust
// ─── Knowledge plane tools ────────────────────────────────────────────────

tool("propose_memory",
    "Capture an AI-detected learning into the triage queue. Used when a heuristic \
     (revert, correction, 'actually...', repeat pattern, override, test failure) fires. \
     User reviews these in the Learnings UI before they enter active memory.",
    serde_json::json!({
        "type": "object",
        "properties": {
            "project_id":    {"type": "string"},
            "scope":         {"type": "string", "enum": ["global", "project", "stack"]},
            "scope_filter":  {"type": "string", "description": "stack id when scope='stack'"},
            "type":          {"type": "string"},
            "title":         {"type": "string"},
            "content":       {"type": "string"},
            "impact":        {"type": "string"},
            "tags":          {"type": "array", "items": {"type": "string"}},
            "triage_signal": {"type": "string", "description": "which heuristic fired"},
        },
        "required": ["scope", "type", "title", "content", "triage_signal"]
    }),
    |args, client| async move {
        let body = filter_empty_string_fields(args);
        client.post("/api/knowledge/proposals", &body).await
    });

tool("save_memory",
    "Explicit memory save — used when the user runs /save or otherwise asks to record \
     a memory directly (no triage). Goes straight into active state.",
    serde_json::json!({
        "type": "object",
        "properties": {
            "project_id":    {"type": "string"},
            "scope":         {"type": "string", "enum": ["global", "project", "stack"]},
            "scope_filter":  {"type": "string"},
            "type":          {"type": "string"},
            "title":         {"type": "string"},
            "content":       {"type": "string"},
            "impact":        {"type": "string"},
            "tags":          {"type": "array", "items": {"type": "string"}}
        },
        "required": ["scope", "type", "title", "content"]
    }),
    |args, client| async move {
        let body = filter_empty_string_fields(args);
        client.post("/api/knowledge/memories", &body).await
    });

tool("accept_proposal",
    "Accept a proposed memory — moves it from triage to active.",
    serde_json::json!({
        "type": "object",
        "properties": { "id": {"type": "string"} },
        "required": ["id"]
    }),
    |args, client| async move {
        let id = args["id"].as_str().ok_or("id required")?.to_string();
        client.post(&format!("/api/knowledge/proposals/{id}/accept"), &serde_json::json!({})).await
    });

tool("reject_proposal",
    "Reject a proposed memory — moves it to rejected state.",
    serde_json::json!({
        "type": "object",
        "properties": {
            "id":     {"type": "string"},
            "reason": {"type": "string"}
        },
        "required": ["id"]
    }),
    |args, client| async move {
        let id = args["id"].as_str().ok_or("id required")?.to_string();
        let body = serde_json::json!({ "reason": args.get("reason") });
        client.post(&format!("/api/knowledge/proposals/{id}/reject"), &body).await
    });

tool("record_outcome",
    "Record one or more memory outcomes (applied/consulted/violated/ignored). \
     Batched — call once per turn with all outcomes the session generated.",
    serde_json::json!({
        "type": "object",
        "properties": {
            "outcomes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "memory_id":  {"type": "string"},
                        "outcome":    {"type": "string", "enum": ["applied","consulted","violated","ignored"]},
                        "session_id": {"type": "string"},
                        "context":    {"type": "string"}
                    },
                    "required": ["memory_id", "outcome"]
                }
            }
        },
        "required": ["outcomes"]
    }),
    |args, client| async move {
        client.post("/api/knowledge/outcomes", &args).await
    });

tool("get_layered_context",
    "Fetch the blended memory context for a project — global + project + stack-matched \
     memories, ordered by strength. Call at session start and on /recall.",
    serde_json::json!({
        "type": "object",
        "properties": {
            "project_id": {"type": "string"},
            "limit":      {"type": "integer", "minimum": 1, "maximum": 500},
            "tags":       {"type": "string", "description": "comma-separated filter"}
        },
        "required": ["project_id"]
    }),
    |args, client| async move {
        let pid = args["project_id"].as_str().ok_or("project_id required")?.to_string();
        let mut path = format!("/api/knowledge/context?project_id={pid}");
        if let Some(l) = args.get("limit").and_then(|v| v.as_i64()) {
            path.push_str(&format!("&limit={l}"));
        }
        if let Some(t) = args.get("tags").and_then(|v| v.as_str()) {
            if !t.trim().is_empty() {
                path.push_str(&format!("&tags={}", urlencoding::encode(t)));
            }
        }
        client.get(&path).await
    });
```

> Note: the helper `filter_empty_string_fields` and `tool(...)` factory are existing in `crates/mcp/src/main.rs`. If their names differ, match the prevailing pattern used by tools like `log_event` and `infer`.

- [ ] **Step 3: Build the MCP binary**

Run: `cargo build --features dev -p sensei-mcp`
Expected: clean build.

- [ ] **Step 4: Smoke-test against the running daemon**

Run:
```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | sensei-mcp-dev | jq '.result.tools[].name' | grep -E 'propose_memory|save_memory|accept_proposal|reject_proposal|record_outcome|get_layered_context'
```
Expected: 6 lines.

- [ ] **Step 5: Commit**

```bash
git add crates/mcp/src/main.rs
git commit -m "feat(mcp): six knowledge tools wired to /api/knowledge/*"
```

---

### Task 11: App — contracts + api client + state singleton

**Files:**
- Modify: `app/src/lib/setup/contracts.ts`
- Modify: `app/src/lib/setup/mock-contracts.ts`
- Modify: `app/src/lib/api.ts`
- Create: `app/src/lib/memoryState.svelte.ts`

- [ ] **Step 1: Add types to `contracts.ts`**

Append:

```typescript
export type MemoryStatus =
    | 'proposed' | 'active' | 'reinforced' | 'challenged'
    | 'battle_tested' | 'archived' | 'rejected';

export type MemoryScope = 'global' | 'project' | 'stack' | 'task_type' | 'module';

export type OutcomeKind = 'applied' | 'consulted' | 'violated' | 'ignored';

export interface Memory {
    id:               string;
    project_id:       string | null;
    scope:            MemoryScope;
    scope_filter:     string | null;
    type:             string;
    title:            string;
    content:          string;
    impact:           string | null;
    strength:         number;
    status:           MemoryStatus;
    applied_count:    number;
    violated_count:   number;
    last_relevant_at: string | null;
    tags:             string[];
    triage_signal:    string | null;
    modified_at:      string;
}

export interface MemoryDetail {
    memory:    Memory;
    evidence:  { url: string | null; note: string | null; recorded_at: string }[];
    examples:  { node_id: string | null; is_good: boolean | null; is_bad: boolean | null; note: string | null }[];
    outcomes:  { outcome: OutcomeKind; session_id: string | null; context: string | null; recorded_at: string }[];
}

export interface MemoryListResponse { memories: Memory[]; }

export interface ContextResponse {
    version:     string;
    memories:    Memory[];
    cache_until: string;
}

export interface ProposalCreateBody {
    project_id?:   string;
    scope:         MemoryScope;
    scope_filter?: string;
    type:          string;
    title:         string;
    content:       string;
    impact?:       string;
    tags?:         string[];
    triage_signal: string;
}

export interface MemoryCreateBody {
    project_id?:   string;
    scope:         MemoryScope;
    scope_filter?: string;
    type:          string;
    title:         string;
    content:       string;
    impact?:       string;
    tags?:         string[];
}

export interface OutcomeBody {
    memory_id:   string;
    outcome:     OutcomeKind;
    session_id?: string;
    context?:    string;
}

export interface OutcomesBatchResponse {
    recorded: number;
    skipped:  { memory_id: string; reason: string }[];
}
```

- [ ] **Step 2: Mock factories in `mock-contracts.ts`**

Append:

```typescript
import type { Memory, MemoryDetail } from './contracts.js';

export function mockMemory(overrides: Partial<Memory> = {}): Memory {
    return {
        id: 'mem-1', project_id: 'proj-1', scope: 'project', scope_filter: null,
        type: 'convention', title: 'Test memory', content: 'body',
        impact: null, strength: 1.0, status: 'proposed',
        applied_count: 0, violated_count: 0, last_relevant_at: null,
        tags: [], triage_signal: 'revert',
        modified_at: '2026-05-27T00:00:00Z',
        ...overrides,
    };
}

export function mockMemoryDetail(overrides: Partial<MemoryDetail> = {}): MemoryDetail {
    return {
        memory: mockMemory(),
        evidence: [],
        examples: [],
        outcomes: [],
        ...overrides,
    };
}
```

- [ ] **Step 3: Client methods in `api.ts`**

Inside the `senseiApi` factory, add:

```typescript
        listMemories: async (q: { status?: string; scope?: string; project_id?: string; limit?: number } = {}) => {
            const p = new URLSearchParams();
            for (const [k, v] of Object.entries(q)) if (v !== undefined) p.set(k, String(v));
            return tryGet<MemoryListResponse>(`/api/knowledge/memories?${p.toString()}`);
        },

        getMemoryDetail: async (id: string) =>
            tryGet<MemoryDetail>(`/api/knowledge/memories/${encodeURIComponent(id)}`),

        getLayeredContext: async (project_id: string, opts: { limit?: number; tags?: string[] } = {}) => {
            const p = new URLSearchParams({ project_id });
            if (opts.limit !== undefined) p.set('limit', String(opts.limit));
            if (opts.tags?.length) p.set('tags', opts.tags.join(','));
            return tryGet<ContextResponse>(`/api/knowledge/context?${p.toString()}`);
        },

        proposeMemory: async (body: ProposalCreateBody) =>
            tryPost<{ id: string; status: 'proposed' }>('/api/knowledge/proposals', body),

        saveMemory: async (body: MemoryCreateBody) =>
            tryPost<{ id: string; status: 'active' }>('/api/knowledge/memories', body),

        acceptProposal: async (id: string) =>
            tryPost<{ id: string; status: string }>(`/api/knowledge/proposals/${encodeURIComponent(id)}/accept`, {}),

        rejectProposal: async (id: string, reason?: string) =>
            tryPost<{ id: string; status: string }>(`/api/knowledge/proposals/${encodeURIComponent(id)}/reject`, { reason }),

        recordOutcomes: async (outcomes: OutcomeBody[]) =>
            tryPost<OutcomesBatchResponse>('/api/knowledge/outcomes', { outcomes }),
```

And add the matching imports at the top of `api.ts`:

```typescript
import type {
    MemoryListResponse, MemoryDetail, ContextResponse,
    ProposalCreateBody, MemoryCreateBody, OutcomeBody, OutcomesBatchResponse,
} from './setup/contracts.js';
```

- [ ] **Step 4: Create `memoryState.svelte.ts`**

```typescript
import { senseiApi } from './api.js';
import type { Memory, MemoryDetail, MemoryStatus } from './setup/contracts.js';

type Tab = 'triage' | 'active' | 'archive';

const STATUSES_BY_TAB: Record<Tab, MemoryStatus[]> = {
    triage:  ['proposed'],
    active:  ['active', 'reinforced', 'challenged', 'battle_tested'],
    archive: ['archived', 'rejected'],
};

class MemoryState {
    triage   = $state<Memory[]>([]);
    active   = $state<Memory[]>([]);
    archive  = $state<Memory[]>([]);
    detail   = $state<MemoryDetail | null>(null);
    selected = $state<string | null>(null);
    loading  = $state(false);

    triageCount  = $derived(this.triage.length);
    activeCount  = $derived(this.active.length);
    archiveCount = $derived(this.archive.length);

    async load(projectId?: string) {
        this.loading = true;
        try {
            const [t, a, x] = await Promise.all([
                this.fetchTab('triage',  projectId),
                this.fetchTab('active',  projectId),
                this.fetchTab('archive', projectId),
            ]);
            this.triage  = t;
            this.active  = a;
            this.archive = x;
        } finally {
            this.loading = false;
        }
    }

    private async fetchTab(tab: Tab, projectId?: string): Promise<Memory[]> {
        const statuses = STATUSES_BY_TAB[tab];
        const buckets = await Promise.all(statuses.map(status =>
            senseiApi().listMemories({ status, project_id: projectId, limit: 500 })
        ));
        return buckets.flatMap(b => b?.memories ?? []);
    }

    async select(id: string) {
        this.selected = id;
        this.detail = await senseiApi().getMemoryDetail(id);
    }

    async accept(id: string) {
        await senseiApi().acceptProposal(id);
        this.triage = this.triage.filter(m => m.id !== id);
        // Refresh active so the row appears with correct status.
        this.active = await this.fetchTab('active');
    }

    async reject(id: string) {
        await senseiApi().rejectProposal(id);
        this.triage = this.triage.filter(m => m.id !== id);
        this.archive = await this.fetchTab('archive');
    }
}

export const memoryState = new MemoryState();
```

- [ ] **Step 5: Run vitest — verify it builds**

Run: `cd app && bunx vitest run --reporter=verbose memoryState 2>&1 | tail -5`
Expected: no test file yet — output says "no test files found". That's fine; we add tests in the next step.

- [ ] **Step 6: Commit**

```bash
git add app/src/lib/setup/contracts.ts \
        app/src/lib/setup/mock-contracts.ts \
        app/src/lib/api.ts \
        app/src/lib/memoryState.svelte.ts
git commit -m "feat(app): knowledge contracts + api client + memoryState singleton"
```

---

### Task 12: App — unit tests for `memoryState`

**Files:**
- Create: `app/src/lib/memoryState.spec.svelte.ts`

- [ ] **Step 1: Write the test file**

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { memoryState } from './memoryState.svelte.js';
import { mockMemory, mockMemoryDetail } from './setup/mock-contracts.js';

vi.mock('./api.js', () => {
    const listMemoriesMock = vi.fn();
    const acceptMock = vi.fn().mockResolvedValue({ id: 'mem-1', status: 'active' });
    const rejectMock = vi.fn().mockResolvedValue({ id: 'mem-1', status: 'rejected' });
    const detailMock = vi.fn().mockResolvedValue(mockMemoryDetail());
    return {
        senseiApi: () => ({
            listMemories:   listMemoriesMock,
            acceptProposal: acceptMock,
            rejectProposal: rejectMock,
            getMemoryDetail: detailMock,
        }),
    };
});

import { senseiApi } from './api.js';

describe('memoryState', () => {
    beforeEach(() => {
        memoryState.triage = [];
        memoryState.active = [];
        memoryState.archive = [];
        memoryState.detail = null;
        memoryState.selected = null;
        const api = senseiApi() as any;
        api.listMemories.mockReset();
    });

    it('partitions memories by tab on load', async () => {
        const api = senseiApi() as any;
        api.listMemories.mockImplementation(({ status }: { status: string }) => Promise.resolve({
            memories: status === 'proposed' ? [mockMemory({ id: 'p1', status: 'proposed' })]
                    : status === 'active'   ? [mockMemory({ id: 'a1', status: 'active' })]
                    : status === 'archived' ? [mockMemory({ id: 'x1', status: 'archived' })]
                    : []
        }));
        await memoryState.load('proj-1');
        expect(memoryState.triage).toHaveLength(1);
        expect(memoryState.active).toHaveLength(1);
        expect(memoryState.archive).toHaveLength(1);
    });

    it('accept removes from triage', async () => {
        const api = senseiApi() as any;
        memoryState.triage = [mockMemory({ id: 'm-acc', status: 'proposed' })];
        api.listMemories.mockResolvedValue({ memories: [] });
        await memoryState.accept('m-acc');
        expect(memoryState.triage).toHaveLength(0);
        expect(api.acceptProposal).toHaveBeenCalledWith('m-acc');
    });

    it('reject removes from triage and refreshes archive', async () => {
        const api = senseiApi() as any;
        memoryState.triage = [mockMemory({ id: 'm-rej', status: 'proposed' })];
        api.listMemories.mockResolvedValue({ memories: [mockMemory({ id: 'm-rej', status: 'rejected' })] });
        await memoryState.reject('m-rej');
        expect(memoryState.triage).toHaveLength(0);
        expect(api.rejectProposal).toHaveBeenCalledWith('m-rej');
        expect(memoryState.archive).toHaveLength(1);
    });
});
```

- [ ] **Step 2: Run**

Run: `cd app && bunx vitest run memoryState`
Expected: 3 passed.

- [ ] **Step 3: Commit**

```bash
git add app/src/lib/memoryState.spec.svelte.ts
git commit -m "test(app): memoryState load/accept/reject"
```

---

### Task 13: App — Learnings page shell + three tabs

**Files:**
- Modify: `app/src/routes/(observatory)/learnings/+page.svelte`
- Create: `app/src/routes/(observatory)/learnings/TriageList.svelte`
- Create: `app/src/routes/(observatory)/learnings/ActiveList.svelte`
- Create: `app/src/routes/(observatory)/learnings/ArchiveList.svelte`

- [ ] **Step 1: Replace the existing `+page.svelte`**

```svelte
<script lang="ts">
    import { onMount } from 'svelte';
    import { memoryState } from '$lib/memoryState.svelte.js';
    import { appState } from '$lib/appstate.svelte.js';
    import TriageList  from './TriageList.svelte';
    import ActiveList  from './ActiveList.svelte';
    import ArchiveList from './ArchiveList.svelte';
    import MemoryDetail from './MemoryDetail.svelte';

    type Tab = 'triage' | 'active' | 'archive';
    let tab = $state<Tab>('triage');

    onMount(async () => {
        await memoryState.load(appState.activeProjectId ?? undefined);
        // Default to the active tab when there are no proposals.
        if (memoryState.triageCount === 0) tab = 'active';
    });
</script>

<div class="learnings-page" data-testid="learnings-page">
    <header class="tabs">
        <button
            type="button"
            class="tab"
            class:active={tab === 'triage'}
            data-testid="tab-triage"
            onclick={() => (tab = 'triage')}
        >
            Triage <span class="count">{memoryState.triageCount}</span>
        </button>
        <button
            type="button"
            class="tab"
            class:active={tab === 'active'}
            data-testid="tab-active"
            onclick={() => (tab = 'active')}
        >
            Active <span class="count">{memoryState.activeCount}</span>
        </button>
        <button
            type="button"
            class="tab"
            class:active={tab === 'archive'}
            data-testid="tab-archive"
            onclick={() => (tab = 'archive')}
        >
            Archive <span class="count">{memoryState.archiveCount}</span>
        </button>
    </header>

    <div class="layout">
        <section class="list" data-testid={`list-${tab}`}>
            {#if tab === 'triage'}<TriageList />
            {:else if tab === 'active'}<ActiveList />
            {:else}<ArchiveList />{/if}
        </section>

        <aside class="detail" data-testid="detail-pane">
            <MemoryDetail />
        </aside>
    </div>
</div>

<style>
    .learnings-page { display: flex; flex-direction: column; height: 100%; }
    .tabs { display: flex; gap: 0.5rem; border-bottom: 1px solid var(--surface-z3); padding: 0.5rem 1rem; }
    .tab { padding: 0.5rem 1rem; border: none; background: transparent; cursor: pointer; }
    .tab.active { border-bottom: 2px solid var(--accent); font-weight: 600; }
    .count { margin-left: 0.5rem; opacity: 0.6; }
    .layout { display: grid; grid-template-columns: 1fr 1fr; flex: 1; min-height: 0; }
    .list { overflow-y: auto; padding: 1rem; border-right: 1px solid var(--surface-z3); }
    .detail { overflow-y: auto; padding: 1rem; }
</style>
```

- [ ] **Step 2: Create `TriageList.svelte`**

```svelte
<script lang="ts">
    import { memoryState } from '$lib/memoryState.svelte.js';
</script>

{#if memoryState.triage.length === 0}
    <p class="empty">No proposals yet. AI memories appear here for your review.</p>
{:else}
    {#each memoryState.triage as m (m.id)}
        <article
            class="memory-row"
            class:selected={memoryState.selected === m.id}
            data-testid="triage-row"
            data-id={m.id}
        >
            <div class="meta">
                <span class="chip scope">{m.scope}{m.scope_filter ? ':' + m.scope_filter : ''}</span>
                {#if m.triage_signal}<span class="chip signal">{m.triage_signal}</span>{/if}
                {#each m.tags as tag}<span class="chip tag">{tag}</span>{/each}
            </div>
            <h3 onclick={() => memoryState.select(m.id)}>{m.title}</h3>
            <div class="actions">
                <button type="button" data-testid="accept-btn" onclick={() => memoryState.accept(m.id)}>Accept</button>
                <button type="button" data-testid="reject-btn" onclick={() => memoryState.reject(m.id)}>Reject</button>
            </div>
        </article>
    {/each}
{/if}

<style>
    .memory-row { padding: 0.75rem; border: 1px solid var(--surface-z3); border-radius: 4px; margin-bottom: 0.5rem; }
    .memory-row.selected { border-color: var(--accent); }
    .memory-row h3 { margin: 0.25rem 0; cursor: pointer; }
    .meta { display: flex; gap: 0.25rem; flex-wrap: wrap; }
    .chip { padding: 0.1rem 0.5rem; border-radius: 99px; font-size: 0.75rem; background: var(--surface-z3); }
    .chip.signal { background: var(--warning-bg, #553); color: var(--warning-fg, #fff); }
    .actions { display: flex; gap: 0.5rem; margin-top: 0.5rem; }
    .empty { color: var(--text-muted); padding: 1rem; }
</style>
```

- [ ] **Step 3: Create `ActiveList.svelte`**

```svelte
<script lang="ts">
    import { memoryState } from '$lib/memoryState.svelte.js';
</script>

{#if memoryState.active.length === 0}
    <p class="empty">No memories yet. Use /save or wait for AI proposals.</p>
{:else}
    {#each memoryState.active as m (m.id)}
        <article
            class="memory-row"
            class:selected={memoryState.selected === m.id}
            data-testid="active-row"
            data-id={m.id}
        >
            <div class="meta">
                <span class="chip scope">{m.scope}{m.scope_filter ? ':' + m.scope_filter : ''}</span>
                <span class="chip status status-{m.status}">{m.status}</span>
                {#each m.tags as tag}<span class="chip tag">{tag}</span>{/each}
            </div>
            <h3 onclick={() => memoryState.select(m.id)}>{m.title}</h3>
            <div class="metrics">
                <span title="strength">★ {m.strength.toFixed(1)} / 5</span>
                <span title="applied">✓ {m.applied_count}</span>
                <span title="violated">✗ {m.violated_count}</span>
            </div>
        </article>
    {/each}
{/if}

<style>
    .memory-row { padding: 0.75rem; border: 1px solid var(--surface-z3); border-radius: 4px; margin-bottom: 0.5rem; }
    .memory-row.selected { border-color: var(--accent); }
    .memory-row h3 { margin: 0.25rem 0; cursor: pointer; }
    .meta { display: flex; gap: 0.25rem; flex-wrap: wrap; }
    .chip { padding: 0.1rem 0.5rem; border-radius: 99px; font-size: 0.75rem; background: var(--surface-z3); }
    .metrics { display: flex; gap: 1rem; margin-top: 0.5rem; font-size: 0.85rem; opacity: 0.8; }
    .empty { color: var(--text-muted); padding: 1rem; }
</style>
```

- [ ] **Step 4: Create `ArchiveList.svelte`**

```svelte
<script lang="ts">
    import { memoryState } from '$lib/memoryState.svelte.js';
</script>

{#if memoryState.archive.length === 0}
    <p class="empty">Nothing archived.</p>
{:else}
    {#each memoryState.archive as m (m.id)}
        <article class="memory-row" data-testid="archive-row" data-id={m.id}>
            <div class="meta">
                <span class="chip scope">{m.scope}</span>
                <span class="chip status status-{m.status}">{m.status}</span>
            </div>
            <h3 onclick={() => memoryState.select(m.id)}>{m.title}</h3>
        </article>
    {/each}
{/if}

<style>
    .memory-row { padding: 0.75rem; border: 1px solid var(--surface-z3); border-radius: 4px; margin-bottom: 0.5rem; opacity: 0.7; }
    .memory-row h3 { margin: 0.25rem 0; cursor: pointer; }
    .meta { display: flex; gap: 0.25rem; flex-wrap: wrap; }
    .chip { padding: 0.1rem 0.5rem; border-radius: 99px; font-size: 0.75rem; background: var(--surface-z3); }
    .empty { color: var(--text-muted); padding: 1rem; }
</style>
```

- [ ] **Step 5: Run svelte-check + unit tests**

Run: `cd app && bunx svelte-check && bunx vitest run`
Expected: 0 errors, all tests pass.

- [ ] **Step 6: Commit**

```bash
git add app/src/routes/(observatory)/learnings/+page.svelte \
        app/src/routes/(observatory)/learnings/TriageList.svelte \
        app/src/routes/(observatory)/learnings/ActiveList.svelte \
        app/src/routes/(observatory)/learnings/ArchiveList.svelte
git commit -m "feat(app): Learnings page three-tab shell"
```

---

### Task 14: App — `MemoryDetail.svelte` (detail pane)

**Files:**
- Create: `app/src/routes/(observatory)/learnings/MemoryDetail.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
    import { memoryState } from '$lib/memoryState.svelte.js';
    const d = $derived(memoryState.detail);
</script>

{#if !d}
    <p class="empty">Select a memory to view details.</p>
{:else}
    <header class="head">
        <span class="chip scope">{d.memory.scope}{d.memory.scope_filter ? ':' + d.memory.scope_filter : ''}</span>
        <span class="chip status status-{d.memory.status}">{d.memory.status}</span>
        {#each d.memory.tags as tag}<span class="chip tag">{tag}</span>{/each}
    </header>

    <h2>{d.memory.title}</h2>
    <p class="content" data-testid="detail-content">{d.memory.content}</p>

    {#if d.memory.impact}
        <section><h4>Impact</h4><p>{d.memory.impact}</p></section>
    {/if}

    <section><h4>Metrics</h4>
        <p>Strength: <strong>{d.memory.strength.toFixed(1)} / 5</strong>
           — applied {d.memory.applied_count}× · violated {d.memory.violated_count}×</p>
    </section>

    {#if d.evidence.length}
        <section><h4>Evidence</h4>
            <ul>{#each d.evidence as e}
                <li>{#if e.url}<a href={e.url} target="_blank" rel="noreferrer">{e.url}</a>{/if} {e.note ?? ''}</li>
            {/each}</ul>
        </section>
    {/if}

    {#if d.outcomes.length}
        <section data-testid="detail-outcomes"><h4>Recent outcomes</h4>
            <ul>{#each d.outcomes as o}
                <li>
                    <span class="chip outcome-{o.outcome}">{o.outcome}</span>
                    <time>{new Date(o.recorded_at).toLocaleString()}</time>
                    {#if o.context}— {o.context}{/if}
                </li>
            {/each}</ul>
        </section>
    {/if}
{/if}

<style>
    .empty { color: var(--text-muted); }
    .head { display: flex; gap: 0.25rem; flex-wrap: wrap; margin-bottom: 0.5rem; }
    .chip { padding: 0.1rem 0.5rem; border-radius: 99px; font-size: 0.75rem; background: var(--surface-z3); }
    h2 { margin: 0.5rem 0; }
    .content { white-space: pre-wrap; }
    section { margin-top: 1rem; }
    section h4 { margin: 0 0 0.25rem 0; font-size: 0.85rem; opacity: 0.7; text-transform: uppercase; letter-spacing: 0.05em; }
    .chip.outcome-applied { background: var(--success-bg, #353); }
    .chip.outcome-violated { background: var(--danger-bg, #533); }
</style>
```

- [ ] **Step 2: Verify the route renders**

Run: `cd app && make app-dev` (in another terminal) — navigate to `/learnings` in the running app. Click a Triage row. Expected: detail pane fills with memory body + Recent outcomes section.

(If the dev DB has no memories, insert one with psql for the visual check.)

- [ ] **Step 3: Run tests + svelte-check**

Run: `cd app && bunx svelte-check && bunx vitest run`
Expected: 0 errors, all tests pass.

- [ ] **Step 4: Commit**

```bash
git add app/src/routes/(observatory)/learnings/MemoryDetail.svelte
git commit -m "feat(app): MemoryDetail pane with outcomes timeline"
```

---

### Task 15: App — E2E tests (4 specs from design)

**Files:**
- Create: `app/e2e/tests/learnings-triage.spec.ts`
- Create: `app/e2e/tests/learnings-edit.spec.ts`
- Create: `app/e2e/tests/learnings-detail.spec.ts`
- Create: `app/e2e/tests/learnings-stack-filter.spec.ts`

- [ ] **Step 1: Create `learnings-triage.spec.ts`**

```typescript
import { test, expect } from '../fixtures';
import { mockMemory } from '../../src/lib/setup/mock-contracts';

test('Triage tab renders proposals, Accept moves row out', async ({ tauriPage, mockApi }) => {
    const proposal = mockMemory({ id: 'p-1', status: 'proposed', title: 'Add idempotency to webhooks' });
    mockApi.listMemories.mockImplementation((q: { status?: string } = {}) => Promise.resolve({
        memories: q.status === 'proposed' ? [proposal] : []
    }));
    mockApi.acceptProposal.mockResolvedValue({ id: 'p-1', status: 'active' });

    await tauriPage.goto('/learnings');
    await expect(tauriPage.locator('[data-testid="tab-triage"]')).toContainText('Triage');
    await expect(tauriPage.locator('[data-testid="triage-row"]')).toHaveCount(1);

    await tauriPage.locator('[data-testid="accept-btn"]').click();
    await expect(tauriPage.locator('[data-testid="triage-row"]')).toHaveCount(0);
});
```

- [ ] **Step 2: Create `learnings-edit.spec.ts`**

```typescript
import { test, expect } from '../fixtures';
import { mockMemory } from '../../src/lib/setup/mock-contracts';

test('Reject moves row to Archive', async ({ tauriPage, mockApi }) => {
    const proposal = mockMemory({ id: 'p-rej', status: 'proposed', title: 'reject me' });
    const archived = { ...proposal, status: 'rejected' as const };

    let archived_returned = false;
    mockApi.listMemories.mockImplementation((q: { status?: string } = {}) => {
        if (q.status === 'proposed') {
            return Promise.resolve({ memories: archived_returned ? [] : [proposal] });
        }
        if (q.status === 'archived' || q.status === 'rejected') {
            return Promise.resolve({ memories: archived_returned ? [archived] : [] });
        }
        return Promise.resolve({ memories: [] });
    });
    mockApi.rejectProposal.mockImplementation(async () => {
        archived_returned = true;
        return { id: 'p-rej', status: 'rejected' };
    });

    await tauriPage.goto('/learnings');
    await tauriPage.locator('[data-testid="reject-btn"]').click();
    await expect(tauriPage.locator('[data-testid="triage-row"]')).toHaveCount(0);

    await tauriPage.locator('[data-testid="tab-archive"]').click();
    await expect(tauriPage.locator('[data-testid="archive-row"]')).toHaveCount(1);
});
```

- [ ] **Step 3: Create `learnings-detail.spec.ts`**

```typescript
import { test, expect } from '../fixtures';
import { mockMemory, mockMemoryDetail } from '../../src/lib/setup/mock-contracts';

test('Detail pane shows evidence + outcomes after clicking a row', async ({ tauriPage, mockApi }) => {
    const m = mockMemory({ id: 'm-1', status: 'active', title: 'pick me' });
    mockApi.listMemories.mockImplementation((q: { status?: string } = {}) => Promise.resolve({
        memories: q.status === 'active' ? [m] : []
    }));
    mockApi.getMemoryDetail.mockResolvedValue(mockMemoryDetail({
        memory: m,
        outcomes: [{ outcome: 'applied', session_id: null, context: 'test', recorded_at: '2026-05-27T10:00:00Z' }]
    }));

    await tauriPage.goto('/learnings');
    await tauriPage.locator('[data-testid="tab-active"]').click();
    await tauriPage.locator('[data-testid="active-row"] h3').click();

    await expect(tauriPage.locator('[data-testid="detail-content"]')).toBeVisible();
    await expect(tauriPage.locator('[data-testid="detail-outcomes"]')).toContainText('applied');
});
```

- [ ] **Step 4: Create `learnings-stack-filter.spec.ts`**

```typescript
import { test, expect } from '../fixtures';
import { mockMemory } from '../../src/lib/setup/mock-contracts';

test('Active tab shows stack-matched memories when current project stack matches', async ({ tauriPage, mockApi }) => {
    // Two memories: one stack=rust matching a Rust project, one stack=python that should not appear.
    const rust_mem   = mockMemory({ id: 'r1', scope: 'stack', scope_filter: 'rust',   status: 'active', title: 'rust thing' });
    const python_mem = mockMemory({ id: 'p1', scope: 'stack', scope_filter: 'python', status: 'active', title: 'python thing' });

    // The active tab uses listMemories with status; the daemon's context endpoint does the
    // stack matching. For this test we exercise the list endpoint and confirm only the
    // active memories the daemon returned appear. (Daemon-side stack matching is covered
    // by integration test in Task 9.)
    mockApi.listMemories.mockImplementation((q: { status?: string } = {}) => Promise.resolve({
        memories: q.status === 'active' ? [rust_mem] : (q.status === 'archived' ? [python_mem] : [])
    }));

    await tauriPage.goto('/learnings');
    await tauriPage.locator('[data-testid="tab-active"]').click();
    await expect(tauriPage.locator('[data-testid="active-row"]')).toHaveCount(1);
    await expect(tauriPage.locator('[data-testid="active-row"]')).toContainText('rust thing');
});
```

- [ ] **Step 5: Update `e2e/fixtures.ts` to mock the knowledge endpoints**

Find the `ipcMocks` (or `apiMocks`) block in `app/e2e/fixtures.ts`. Add a `mockApi` fixture that exposes vitest-style `vi.fn()` mocks for each of the eight new client methods, similar to the existing pattern. The exact mock shape must match the project's existing test fixtures — search the file for the `senseiApi` mock to follow the convention.

If the convention is to mock at the HTTP level (intercepted `fetch`), add intercepts for these paths instead:

| Method | Path |
|---|---|
| GET    | `/api/knowledge/memories` |
| GET    | `/api/knowledge/memories/:id` |
| GET    | `/api/knowledge/context` |
| POST   | `/api/knowledge/proposals` |
| POST   | `/api/knowledge/proposals/:id/accept` |
| POST   | `/api/knowledge/proposals/:id/reject` |
| POST   | `/api/knowledge/memories` |
| POST   | `/api/knowledge/outcomes` |

- [ ] **Step 6: Run E2E**

Run: `cd app && bunx playwright test --config e2e/playwright.config.ts --grep learnings`
Expected: 4 passed.

- [ ] **Step 7: Commit**

```bash
git add app/e2e/tests/learnings-triage.spec.ts \
        app/e2e/tests/learnings-edit.spec.ts \
        app/e2e/tests/learnings-detail.spec.ts \
        app/e2e/tests/learnings-stack-filter.spec.ts \
        app/e2e/fixtures.ts
git commit -m "test(e2e): Learnings page — triage / edit / detail / stack-filter"
```

---

### Task 16: Final zero-errors sweep + push

**Files:** (none new — verification)

- [ ] **Step 1: Full Rust test sweep**

Run: `cargo test --features dev`
Expected: all suites pass.

- [ ] **Step 2: Clippy with `-D warnings`**

Run: `cargo clippy --features dev --all-targets -- -D warnings`
Expected: 0 errors.

- [ ] **Step 3: Full app test sweep**

Run: `cd app && bunx svelte-check && bunx vitest run && bunx playwright test --config e2e/playwright.config.ts`
Expected: 0 type errors, all unit + E2E tests pass.

- [ ] **Step 4: Manual smoke — start daemon, open app, click through Learnings**

Run: `make install-dev && senseid-dev start && make app-dev`
In app: navigate to Learnings. With no memories, see empty-state copy. Insert one via psql or via curl `POST /api/knowledge/proposals`. Refresh — see it in Triage. Click Accept — moves to Active.

- [ ] **Step 5: Push develop, then merge to main**

```bash
git push origin develop
git checkout main
git pull --ff-only origin main
git merge --no-ff develop -m "Merge branch 'develop' — knowledge plane Phase 0"
git push origin main
git checkout develop
git merge --no-ff main -m "Merge branch 'main' back into develop"
git push origin develop
```

---

## Self-review

**Spec coverage** — every section of the spec has a task:

| Spec section | Task(s) |
|---|---|
| Schema delta § 1 (status enum) | Task 1 |
| Schema delta § 2 (tags + triage_signal + GIN) | Task 2 |
| Schema delta § 3 (memory_outcomes + enum + trigger) | Tasks 1, 3, 4 |
| API surface — writes (proposals, memories) | Task 8 |
| API surface — accept/reject | Task 8 |
| API surface — outcomes batch | Task 8 |
| API surface — context | Task 8 |
| API surface — list + detail | Task 8 |
| MCP six tools | Task 10 |
| UI three-tab layout | Task 13 |
| Detail pane | Task 14 |
| Edit form for accept-with-edits | **gap — see below** |
| Outcome wiring (trigger logic) | Task 4 |
| Capture-trigger guidance | **gap — see below** |
| Testing (unit, integration, E2E) | Tasks 5, 6, 7, 9, 12, 15 |

**Gaps found in self-review:**

1. **Edit-and-accept form.** Spec § Triage UI lists `Edit & Accept` as a row action. Plan stops at `accept_proposal` (no edits applied). Phase 0 ships accept as transition-only; the spec's optional `edits` field on `POST /proposals/:id/accept` is acknowledged but deferred. **Resolution:** I am leaving this deferred to Phase 0.1 and adding an explicit note (below).
2. **Capture-trigger guidance in plugin's CLAUDE.md.** Spec § Capture-trigger guidance puts the heuristic list in the sensei marketplace plugin. Plan doesn't include a task to write that document. **Resolution:** add Task 17.

### Task 17: Marketplace plugin — capture-trigger guidance

**Files:**
- Create: `marketplace/plugins/sensei-knowledge/CLAUDE.md`

- [ ] **Step 1: Write the plugin's CLAUDE.md**

```markdown
# Sensei knowledge plane — assistant guidance

You have access to sensei's knowledge plane via MCP. Use it as follows:

## At session start (and on user-issued /recall)

1. Determine the current project_id (from the IDE / cwd context).
2. Call `get_layered_context(project_id=...)`. Read every returned memory
   and consider it authoritative.
3. After 60 minutes or on /recall, re-fetch.

## When to call `propose_memory`

Sensei expects you to capture learnings as proposals (triage queue),
not directly as active memories. Call `propose_memory` when any of
these triggers fires — and only then:

| Trigger value      | Fires when |
|--------------------|-----------|
| `revert`           | User reverted code you just suggested. |
| `correction`       | User edited your output non-trivially (>3 lines diff). |
| `actually`         | User said "actually...", "no, we always...", "remember that...", "we never...". |
| `repeat_pattern`   | The same kind of fix or edit has happened in 2+ sessions in this repo. |
| `override`         | You cited a memory and the user overruled. Also call `record_outcome(violated)` for that memory. |
| `test_failure`     | A test failed on first run of your generated code, and the user's fix is non-trivial. |

Don't propose memories outside these triggers. Ask the user explicitly
via `/save` for anything else.

## When to call `record_outcome`

End every turn that involved memories with one batched `record_outcome`
call covering each memory you loaded:

- `applied` — the memory shaped output the user accepted.
- `consulted` — you loaded it and considered it, but didn't apply.
- `violated` — you applied it and the user reversed the change.
- `ignored` — loaded but irrelevant; record so we can decay it.

## When to call `save_memory`

Only on explicit user instruction (`/save`, "save this as a project
memory", etc.). Never on heuristic detection.
```

- [ ] **Step 2: Commit**

```bash
git add marketplace/plugins/sensei-knowledge/CLAUDE.md
git commit -m "feat(marketplace): sensei-knowledge plugin guidance for assistants"
```

---

## Deferred to a future Phase 0.1

- Inline **Edit & Accept** form on triage rows (use `MemoryEditForm.svelte` placeholder, wire it up via a new endpoint that combines accept + update).
- `MemoryEditForm.svelte` itself — referenced in the spec UI mockup but not built in Phase 0.
- Strength-over-time sparkline on the detail pane.
- Pagination / virtualization on long memory lists (current limit cap is 500).

---

*End of plan.*
