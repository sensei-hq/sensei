# Front-door Intake — Plan 1: Data + Recommender Core

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the data model + the pure recommender + the classify/tool surface so `axes → playbook + rationale` works end-to-end via an MCP tool and every intake decision is persisted for §9.

**Architecture:** New `sensei`-schema DDL (3 axis enums + `playbooks`/`playbook_rules`/`intake_guide`/`playbook_run` tables, seeded via staging+import). A **pure** Rust resolver (`crates/senseid/src/playbook.rs`) maps classified axes over the rule set → recommendation. `pg_store` gains CRUD. A daemon endpoint + MCP tool `recommend_playbook` exposes it. `classify_chunk` (gateway, local-first, heuristic fallback) turns a chunk into axes for non-agent callers.

**Tech Stack:** PostgreSQL + `dbd` (DDL/seed), Rust (`crates/senseid` bin, `crates/mcp`), `sqlx_core::query_as`, embedded `gateway`. Tests: `cargo test -p senseid`. DB tests use `sensei_test` (`TEST_DATABASE_URL` default `postgresql://localhost:5432/sensei_test`). Commit per task to `develop` (approach A).

**Design:** `docs/plan/2026-07-19-frontdoor-intake-design.md`.

---

## File Structure

**Create (DDL):**
- `database/ddl/enum/sensei/chunk_lifecycle.ddl` · `chunk_intent.ddl` · `chunk_risk.ddl` — axis enums.
- `database/ddl/table/sensei/playbooks.ddl` · `playbook_rules.ddl` · `intake_guide.ddl` · `playbook_run.ddl` — core tables.
- `database/ddl/table/staging/playbooks.ddl` · `playbook_rules.ddl` · `intake_guide.ddl` — seed staging.
- `database/ddl/procedure/staging/import_playbooks.ddl` · `import_playbook_rules.ddl` · `import_intake_guide.ddl` — idempotent seed importers.

**Create (Rust):**
- `crates/senseid/src/playbook.rs` — PURE: `Axis*` enums, `Rule`, `Recommendation`, `recommend(axes, &[Rule])`. Declared `pub mod playbook;` in `crates/senseid/src/main.rs`.
- `crates/senseid/src/api/handlers/playbook.rs` — `classify_chunk` (gateway + fallback) + `recommend_playbook` HTTP handler.

**Modify:**
- `crates/senseid/src/main.rs` — add `pub mod playbook;`.
- `crates/senseid/src/db/pg_store.rs` — add `list_playbooks`, `list_playbook_rules`, `list_intake_guide`, `insert_playbook_run`.
- `crates/senseid/src/api/routes.rs` — add `POST /api/playbook/recommend`.
- `crates/senseid/src/api/handlers/mod.rs` — `pub mod playbook;`.
- `crates/mcp/src/lib.rs` — `recommend_playbook` tool schema + dispatcher branch.
- `crates/senseid/src/api/handlers/mcp_manifests.rs` — manifest entry for `recommend_playbook`.

---

### Task 1: DDL — axis enums

**Files:**
- Create: `database/ddl/enum/sensei/chunk_lifecycle.ddl`, `chunk_intent.ddl`, `chunk_risk.ddl`

- [ ] **Step 1: Write the three enum files** (mirror `enum/sensei/spine_slot.ddl`).

`chunk_lifecycle.ddl`:
```sql
set search_path to sensei, extensions;
create type chunk_lifecycle as enum ('greenfield', 'stable');
```
`chunk_intent.ddl`:
```sql
set search_path to sensei, extensions;
create type chunk_intent as enum ('explore', 'ux', 'feature', 'enhancement', 'bug');
```
`chunk_risk.ddl`:
```sql
set search_path to sensei, extensions;
create type chunk_risk as enum ('low', 'high');
```

- [ ] **Step 2: Apply to the test DB and verify** (mirrors how spine_slot was applied)

Run:
```bash
psql "$TEST_DATABASE_URL" -c "create type sensei.chunk_lifecycle as enum ('greenfield','stable');" \
  -c "create type sensei.chunk_intent as enum ('explore','ux','feature','enhancement','bug');" \
  -c "create type sensei.chunk_risk as enum ('low','high');"
psql "$TEST_DATABASE_URL" -tAc "select enum_range(null::sensei.chunk_intent);"
```
Expected: `{explore,ux,feature,enhancement,bug}` (NB: dbd deploys variants alphabetically on real deploy — never rely on ordinal; resolver keys on the label).

- [ ] **Step 3: Commit**
```bash
git add database/ddl/enum/sensei/chunk_lifecycle.ddl database/ddl/enum/sensei/chunk_intent.ddl database/ddl/enum/sensei/chunk_risk.ddl
git commit -m "feat(db): sensei chunk axis enums (lifecycle/intent/risk) for the intake recommender"
```

---

### Task 2: DDL — `playbooks` table + seed

**Files:**
- Create: `database/ddl/table/sensei/playbooks.ddl`, `database/ddl/table/staging/playbooks.ddl`, `database/ddl/procedure/staging/import_playbooks.ddl`

- [ ] **Step 1: Write `table/sensei/playbooks.ddl`** (mirror `table/sensei/memories.ddl` style)
```sql
set search_path to sensei, extensions;
create table if not exists playbooks (
  name          text        primary key
, title         text        not null
, when_to_use   text        not null
, opening_tone  text        not null
, method_ref    text
, enabled       boolean     not null default true
, source        text        not null default 'builtin'
, created_at    timestamptz not null default now()
, constraint playbooks_source_chk check (source in ('builtin','org','learned'))
);
```

- [ ] **Step 2: Write `table/staging/playbooks.ddl`** (mirror `table/staging/fallback_chain_models.ddl`)
```sql
set search_path to staging, extensions;
drop table if exists playbooks cascade;
create table playbooks (
  name text, title text, when_to_use text, opening_tone text,
  method_ref text, enabled boolean default true, source text default 'builtin',
  modified_at timestamptz not null default now()
);
```

- [ ] **Step 3: Write `procedure/staging/import_playbooks.ddl`** (mirror `import_fallback_chain_models.ddl`: idempotent, timestamp-guarded)
```sql
set search_path to staging, extensions;
create or replace procedure import_playbooks()
language plpgsql as $$
begin
  insert into sensei.playbooks (name, title, when_to_use, opening_tone, method_ref, enabled, source, created_at)
  select name, title, when_to_use, opening_tone, method_ref,
         coalesce(enabled, true), coalesce(source,'builtin'), coalesce(modified_at, now())
    from staging.playbooks
  on conflict (name) do update
     set title=excluded.title, when_to_use=excluded.when_to_use,
         opening_tone=excluded.opening_tone, method_ref=excluded.method_ref
   where sensei.playbooks.source = 'builtin';  -- never clobber org/learned edits
end;
$$;
```

- [ ] **Step 4: Author the seed rows** as a repeatable SQL insert into staging (the 6 named routes). Add to a seed script the plan's executor runs (or inline in the import test). Seed content:
```sql
insert into staging.playbooks (name, title, when_to_use, opening_tone) values
 ('vibe','Vibe / spike','Greenfield, objective fuzzy — explore then extract learnings (discardable).','Explore fast and loose; capture what you learn, keep nothing you cannot justify.'),
 ('mockup_first','Mockup-first','Greenfield, UX-heavy — design the surface before the spec.','Start from the mockup; let the UI shape the spec.'),
 ('spec_driven','Spec-driven','Clear objective + high blast-radius — force a deep design first.','Slow down: write the design, enumerate edge cases, before any code.'),
 ('gsd','Get stuff done','Known feature, low risk — lean plan then build.','Lean plan, then build; keep it tight.'),
 ('change_flow','Change-flow','Stable product enhancement — impact analysis then targeted design.','Map impact first; design the smallest change that lands the value.'),
 ('debug_flow','Debug-flow','Stable product bug — reproduce, fix, add a regression test.','Reproduce first; fix; lock it with a regression test.');
```

- [ ] **Step 5: Apply + seed to the test DB and verify**
```bash
psql "$TEST_DATABASE_URL" -f database/ddl/table/sensei/playbooks.ddl
psql "$TEST_DATABASE_URL" -f database/ddl/table/staging/playbooks.ddl
psql "$TEST_DATABASE_URL" -f database/ddl/procedure/staging/import_playbooks.ddl
psql "$TEST_DATABASE_URL" < <seed-inserts-from-step-4>
psql "$TEST_DATABASE_URL" -c "call staging.import_playbooks();"
psql "$TEST_DATABASE_URL" -tAc "select count(*) from sensei.playbooks;"   # Expected: 6
```

- [ ] **Step 6: Commit**
```bash
git add database/ddl/table/sensei/playbooks.ddl database/ddl/table/staging/playbooks.ddl database/ddl/procedure/staging/import_playbooks.ddl
git commit -m "feat(db): sensei.playbooks registry + staging import (6 named-route seeds)"
```

---

### Task 3: DDL — `playbook_rules` table + seed

**Files:**
- Create: `database/ddl/table/sensei/playbook_rules.ddl`, `database/ddl/table/staging/playbook_rules.ddl`, `database/ddl/procedure/staging/import_playbook_rules.ddl`

- [ ] **Step 1: Write `table/sensei/playbook_rules.ddl`**
```sql
set search_path to sensei, extensions;
create table if not exists playbook_rules (
  id               uuid            primary key default gen_random_uuid()
, name             text            not null
, match_lifecycle  chunk_lifecycle
, match_intent     chunk_intent
, match_risk       chunk_risk
, playbook         text            not null references sensei.playbooks(name)
, rationale        text            not null
, priority         integer         not null
, enabled          boolean         not null default true
, source           text            not null default 'builtin'
, created_at       timestamptz     not null default now()
, constraint playbook_rules_source_chk check (source in ('builtin','org','learned'))
);
create index if not exists playbook_rules_match_idx on playbook_rules(enabled, priority desc);
```

- [ ] **Step 2: Write `table/staging/playbook_rules.ddl`** (text columns for enums; import casts them)
```sql
set search_path to staging, extensions;
drop table if exists playbook_rules cascade;
create table playbook_rules (
  name text, match_lifecycle text, match_intent text, match_risk text,
  playbook text, rationale text, priority integer, enabled boolean default true,
  source text default 'builtin', modified_at timestamptz not null default now()
);
```

- [ ] **Step 3: Write `procedure/staging/import_playbook_rules.ddl`** (cast text→enum; idempotent on `name`)
```sql
set search_path to staging, extensions;
create or replace procedure import_playbook_rules()
language plpgsql as $$
begin
  insert into sensei.playbook_rules
     (name, match_lifecycle, match_intent, match_risk, playbook, rationale, priority, enabled, source)
  select name,
         nullif(match_lifecycle,'')::sensei.chunk_lifecycle,
         nullif(match_intent,'')::sensei.chunk_intent,
         nullif(match_risk,'')::sensei.chunk_risk,
         playbook, rationale, priority, coalesce(enabled,true), coalesce(source,'builtin')
    from staging.playbook_rules stg
  where not exists (select 1 from sensei.playbook_rules r where r.name = stg.name and r.source='builtin');
end;
$$;
```

- [ ] **Step 4: Seed the 6 §3.3 rules** (into staging, then import):
```sql
insert into staging.playbook_rules (name, match_lifecycle, match_intent, match_risk, playbook, rationale, priority) values
 ('clear + high blast-radius','', '', 'high','spec_driven','High blast-radius — force a deep design before code.',100),
 ('greenfield, objective fuzzy','greenfield','explore','','vibe','Greenfield with a fuzzy objective — spike to learn.',60),
 ('greenfield, UX-heavy','greenfield','ux','','mockup_first','Greenfield and UX-heavy — design the surface first.',60),
 ('stable product, bug','stable','bug','','debug_flow','A bug in a stable product — reproduce, fix, regress.',60),
 ('stable product, enhancement','stable','enhancement','','change_flow','Enhancing a stable product — impact-analyse first.',50),
 ('known feature, low risk','', 'feature','low','gsd','Known low-risk feature — lean plan then build.',40);
```

- [ ] **Step 5: Apply + seed + verify**
```bash
psql "$TEST_DATABASE_URL" -f database/ddl/table/sensei/playbook_rules.ddl
psql "$TEST_DATABASE_URL" -f database/ddl/table/staging/playbook_rules.ddl
psql "$TEST_DATABASE_URL" -f database/ddl/procedure/staging/import_playbook_rules.ddl
psql "$TEST_DATABASE_URL" < <seed-inserts-from-step-4>
psql "$TEST_DATABASE_URL" -c "call staging.import_playbook_rules();"
psql "$TEST_DATABASE_URL" -tAc "select count(*) from sensei.playbook_rules;"   # Expected: 6
```

- [ ] **Step 6: Commit**
```bash
git add database/ddl/table/sensei/playbook_rules.ddl database/ddl/table/staging/playbook_rules.ddl database/ddl/procedure/staging/import_playbook_rules.ddl
git commit -m "feat(db): sensei.playbook_rules (nullable-match rule set) + 6 seed rules"
```

---

### Task 4: DDL — `intake_guide` table + seed

**Files:**
- Create: `database/ddl/table/sensei/intake_guide.ddl`, `database/ddl/table/staging/intake_guide.ddl`, `database/ddl/procedure/staging/import_intake_guide.ddl`

- [ ] **Step 1: Write `table/sensei/intake_guide.ddl`**
```sql
set search_path to sensei, extensions;
create table if not exists intake_guide (
  id          uuid        primary key default gen_random_uuid()
, kind        text        not null
, axis        text
, prompt      text        not null
, help        text
, enabled     boolean     not null default true
, source      text        not null default 'builtin'
, created_at  timestamptz not null default now()
, constraint intake_guide_kind_chk check (kind in ('frame','axis'))
, constraint intake_guide_axis_chk check ((kind='axis') = (axis is not null))
, constraint intake_guide_source_chk check (source in ('builtin','org','learned'))
);
```

- [ ] **Step 2: Write `table/staging/intake_guide.ddl`**
```sql
set search_path to staging, extensions;
drop table if exists intake_guide cascade;
create table intake_guide (
  kind text, axis text, prompt text, help text,
  enabled boolean default true, source text default 'builtin',
  modified_at timestamptz not null default now()
);
```

- [ ] **Step 3: Write `procedure/staging/import_intake_guide.ddl`** (idempotent on (kind, coalesce(axis,'')))
```sql
set search_path to staging, extensions;
create or replace procedure import_intake_guide()
language plpgsql as $$
begin
  insert into sensei.intake_guide (kind, axis, prompt, help, enabled, source)
  select kind, nullif(axis,''), prompt, help, coalesce(enabled,true), coalesce(source,'builtin')
    from staging.intake_guide stg
  where not exists (
     select 1 from sensei.intake_guide g
      where g.kind = stg.kind and coalesce(g.axis,'') = coalesce(nullif(stg.axis,''),'')
        and g.source = 'builtin');
end;
$$;
```

- [ ] **Step 4: Seed the frame + 3 axis rows**
```sql
insert into staging.intake_guide (kind, axis, prompt, help) values
 ('frame', null, 'You are running the sensei intake. Identify the chunk''s lifecycle, intent, and risk, then recommend a playbook with a one-line reason. Ask only what you cannot infer from the project.', 'Grounds the whole intake conversation.'),
 ('axis','lifecycle','Is this a greenfield effort or a change to a stable product?','Infer from the spine/drift: existing code + docs → stable; empty/new → greenfield.'),
 ('axis','intent','What is the goal — explore, a UX-heavy surface, a new feature, an enhancement, or a bug fix?','Maps to chunk_intent: explore/ux/feature/enhancement/bug.'),
 ('axis','risk','How much does this touch — is the blast-radius high?','Use the code graph (callers/community reach) — many dependents → high.');
```

- [ ] **Step 5: Apply + seed + verify**
```bash
psql "$TEST_DATABASE_URL" -f database/ddl/table/sensei/intake_guide.ddl
psql "$TEST_DATABASE_URL" -f database/ddl/table/staging/intake_guide.ddl
psql "$TEST_DATABASE_URL" -f database/ddl/procedure/staging/import_intake_guide.ddl
psql "$TEST_DATABASE_URL" < <seed-inserts-from-step-4>
psql "$TEST_DATABASE_URL" -c "call staging.import_intake_guide();"
psql "$TEST_DATABASE_URL" -tAc "select count(*) from sensei.intake_guide;"   # Expected: 4
```

- [ ] **Step 6: Commit**
```bash
git add database/ddl/table/sensei/intake_guide.ddl database/ddl/table/staging/intake_guide.ddl database/ddl/procedure/staging/import_intake_guide.ddl
git commit -m "feat(db): sensei.intake_guide (frame + per-axis elicitation) + seeds"
```

---

### Task 5: DDL — `playbook_run` table

**Files:**
- Create: `database/ddl/table/sensei/playbook_run.ddl`

- [ ] **Step 1: Write `table/sensei/playbook_run.ddl`** (FK to `activity.sessions` — the real session table — and `sensei.playbooks`)
```sql
set search_path to sensei, extensions;
create table if not exists playbook_run (
  id          uuid            primary key default gen_random_uuid()
, session_id  uuid            references activity.sessions(id) on delete set null
, feature     text
, lifecycle   chunk_lifecycle not null
, intent      chunk_intent    not null
, risk        chunk_risk      not null
, rule_id     uuid            references sensei.playbook_rules(id) on delete set null
, playbook    text            not null references sensei.playbooks(name)
, rationale   text            not null
, confirmed   boolean         not null default false
, outcome     text
, created_at  timestamptz     not null default now()
);
create index if not exists playbook_run_session_idx on playbook_run(session_id);
```

- [ ] **Step 2: Apply + verify**
```bash
psql "$TEST_DATABASE_URL" -f database/ddl/table/sensei/playbook_run.ddl
psql "$TEST_DATABASE_URL" -tAc "\d sensei.playbook_run" | grep -E "session_id|playbook|rule_id"
```
Expected: columns present with the FK references.

- [ ] **Step 3: Commit**
```bash
git add database/ddl/table/sensei/playbook_run.ddl
git commit -m "feat(db): sensei.playbook_run decision record (FK activity.sessions; §9 outcome seam)"
```

---

### Task 6: Pure resolver — `recommend(axes, &[Rule])`

**Files:**
- Create: `crates/senseid/src/playbook.rs`
- Modify: `crates/senseid/src/main.rs` (add `pub mod playbook;`)

- [ ] **Step 1: Write the failing test** (append to `crates/senseid/src/playbook.rs`)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> Vec<Rule> {
        vec![
            Rule { id: None, name: "high-blast".into(), match_lifecycle: None, match_intent: None, match_risk: Some(Risk::High), playbook: "spec_driven".into(), rationale: "hi".into(), priority: 100 },
            Rule { id: None, name: "gf-fuzzy".into(), match_lifecycle: Some(Lifecycle::Greenfield), match_intent: Some(Intent::Explore), match_risk: None, playbook: "vibe".into(), rationale: "gf".into(), priority: 60 },
            Rule { id: None, name: "known-low".into(), match_lifecycle: None, match_intent: Some(Intent::Feature), match_risk: Some(Risk::Low), playbook: "gsd".into(), rationale: "gsd".into(), priority: 40 },
        ]
    }

    #[test]
    fn high_risk_wins_by_priority() {
        let axes = Axes { lifecycle: Lifecycle::Greenfield, intent: Intent::Feature, risk: Risk::High };
        let r = recommend(&axes, &seed());
        assert_eq!(r.playbook, "spec_driven");
        assert_eq!(r.rule_name.as_deref(), Some("high-blast"));
    }

    #[test]
    fn wildcard_and_specific_match() {
        let axes = Axes { lifecycle: Lifecycle::Stable, intent: Intent::Feature, risk: Risk::Low };
        assert_eq!(recommend(&axes, &seed()).playbook, "gsd");
    }

    #[test]
    fn no_match_defaults_to_gsd_flagged() {
        let axes = Axes { lifecycle: Lifecycle::Stable, intent: Intent::Ux, risk: Risk::Low };
        let r = recommend(&axes, &seed());
        assert_eq!(r.playbook, "gsd");
        assert!(r.rule_name.is_none());
        assert!(r.defaulted);
    }
}
```

- [ ] **Step 2: Run it and confirm it fails**

Run: `cargo test -p senseid playbook:: 2>&1 | tail -5`
Expected: FAIL — `Axes`, `Rule`, `recommend` not found.

- [ ] **Step 3: Write the implementation** (top of `crates/senseid/src/playbook.rs`)
```rust
//! Pure playbook recommender: classified axes + a rule set -> a recommendation.
//! No IO — the rule set is passed in (DB-source-agnostic).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifecycle { Greenfield, Stable }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent { Explore, Ux, Feature, Enhancement, Bug }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk { Low, High }

impl Lifecycle {
    pub fn as_str(self) -> &'static str { match self { Self::Greenfield => "greenfield", Self::Stable => "stable" } }
    pub fn parse(s: &str) -> Option<Self> { match s { "greenfield" => Some(Self::Greenfield), "stable" => Some(Self::Stable), _ => None } }
}
impl Intent {
    pub fn as_str(self) -> &'static str { match self { Self::Explore=>"explore", Self::Ux=>"ux", Self::Feature=>"feature", Self::Enhancement=>"enhancement", Self::Bug=>"bug" } }
    pub fn parse(s: &str) -> Option<Self> { match s { "explore"=>Some(Self::Explore),"ux"=>Some(Self::Ux),"feature"=>Some(Self::Feature),"enhancement"=>Some(Self::Enhancement),"bug"=>Some(Self::Bug),_=>None } }
}
impl Risk {
    pub fn as_str(self) -> &'static str { match self { Self::Low=>"low", Self::High=>"high" } }
    pub fn parse(s: &str) -> Option<Self> { match s { "low"=>Some(Self::Low),"high"=>Some(Self::High),_=>None } }
}

#[derive(Clone, Copy, Debug)]
pub struct Axes { pub lifecycle: Lifecycle, pub intent: Intent, pub risk: Risk }

#[derive(Clone, Debug)]
pub struct Rule {
    pub id: Option<uuid::Uuid>,
    pub name: String,
    pub match_lifecycle: Option<Lifecycle>,
    pub match_intent: Option<Intent>,
    pub match_risk: Option<Risk>,
    pub playbook: String,
    pub rationale: String,
    pub priority: i32,
}

#[derive(Clone, Debug)]
pub struct Recommendation {
    pub playbook: String,
    pub rationale: String,
    pub rule_id: Option<uuid::Uuid>,
    pub rule_name: Option<String>,
    pub defaulted: bool,
}

fn matches(rule: &Rule, a: &Axes) -> bool {
    rule.match_lifecycle.map_or(true, |m| m == a.lifecycle)
        && rule.match_intent.map_or(true, |m| m == a.intent)
        && rule.match_risk.map_or(true, |m| m == a.risk)
}

/// Highest-priority matching rule wins. No match -> `gsd`, flagged (never silent).
pub fn recommend(axes: &Axes, rules: &[Rule]) -> Recommendation {
    let best = rules.iter().filter(|r| matches(r, axes)).max_by_key(|r| r.priority);
    match best {
        Some(r) => Recommendation {
            playbook: r.playbook.clone(), rationale: r.rationale.clone(),
            rule_id: r.id, rule_name: Some(r.name.clone()), defaulted: false,
        },
        None => Recommendation {
            playbook: "gsd".into(),
            rationale: "no rule matched — defaulted to gsd".into(),
            rule_id: None, rule_name: None, defaulted: true,
        },
    }
}
```
Then add `pub mod playbook;` to `crates/senseid/src/main.rs` (next to `pub mod memory_slot;`).

- [ ] **Step 4: Run tests and confirm they pass**

Run: `cargo test -p senseid playbook:: 2>&1 | tail -5`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: Commit**
```bash
git add crates/senseid/src/playbook.rs crates/senseid/src/main.rs
git commit -m "feat(senseid): pure playbook recommender (axes + rules -> recommendation)"
```

---

### Task 7: pg_store CRUD

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs`

- [ ] **Step 1: Write the failing test** (add to the pg_store DB test module; gated like existing DB tests on `SENSEI_TEST_DB_URL`/`TEST_DATABASE_URL`)
```rust
#[tokio::test]
async fn playbook_rules_load_and_run_roundtrip() {
    let Some(store) = test_store().await else { return; }; // skip if no test DB (match existing helper)
    let rules = store.list_playbook_rules().await.unwrap();
    assert!(rules.iter().any(|r| r.playbook == "spec_driven"));
    let id = store.insert_playbook_run(None, None, "greenfield", "feature", "high",
        None, "spec_driven", "hi", true).await.unwrap();
    let rows = store.list_playbook_runs_for_session_dbg(id).await.unwrap(); // helper reading by run id
    assert_eq!(rows.len(), 1);
}
```
(Use whatever test-store helper the surrounding module already uses; mirror an existing `#[tokio::test]` in `pg_store.rs`.)

- [ ] **Step 2: Run it and confirm it fails**

Run: `cargo test -p senseid playbook_rules_load_and_run_roundtrip 2>&1 | tail -5`
Expected: FAIL — methods not found.

- [ ] **Step 3: Implement the methods** (mirror `list_memories_for_slot` / `insert_memory` at `pg_store.rs`)
```rust
pub async fn list_playbooks(&self) -> Result<Vec<serde_json::Value>, String> {
    let rows: Vec<(String, String, String, String, Option<String>)> = sqlx_core::query_as::query_as(
        "SELECT name, title, when_to_use, opening_tone, method_ref
           FROM sensei.playbooks WHERE enabled ORDER BY name"
    ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(name,title,wtu,tone,mref)| serde_json::json!({
        "name":name,"title":title,"when_to_use":wtu,"opening_tone":tone,"method_ref":mref
    })).collect())
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
    let rows: Vec<(uuid::Uuid, String, Option<String>, Option<String>, Option<String>, String, String, i32)> =
        sqlx_core::query_as::query_as(
            "SELECT id, name, match_lifecycle::text, match_intent::text, match_risk::text,
                    playbook, rationale, priority
               FROM sensei.playbook_rules WHERE enabled ORDER BY priority DESC"
        ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(id,name,lf,it,rk,pb,rat,pri)| Rule {
        id: Some(id), name,
        match_lifecycle: lf.as_deref().and_then(Lifecycle::parse),
        match_intent:    it.as_deref().and_then(Intent::parse),
        match_risk:      rk.as_deref().and_then(Risk::parse),
        playbook: pb, rationale: rat, priority: pri,
    }).collect())
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_playbook_run(
    &self, session_id: Option<uuid::Uuid>, feature: Option<&str>,
    lifecycle: &str, intent: &str, risk: &str,
    rule_id: Option<uuid::Uuid>, playbook: &str, rationale: &str, confirmed: bool,
) -> Result<uuid::Uuid, String> {
    let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.playbook_run
           (session_id, feature, lifecycle, intent, risk, rule_id, playbook, rationale, confirmed)
         VALUES ($1,$2,$3::sensei.chunk_lifecycle,$4::sensei.chunk_intent,$5::sensei.chunk_risk,$6,$7,$8,$9)
         RETURNING id"
    ).bind(session_id).bind(feature).bind(lifecycle).bind(intent).bind(risk)
     .bind(rule_id).bind(playbook).bind(rationale).bind(confirmed)
     .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(row.0)
}
```
(Add a small `list_playbook_runs_for_session_dbg`/count helper if the test needs a reader, or assert via a direct count query in the test.)

- [ ] **Step 4: Run tests and confirm pass** (requires the test DB seeded from Tasks 2–5)

Run: `cargo test -p senseid playbook_rules_load_and_run_roundtrip 2>&1 | tail -5`
Expected: PASS (or clean skip when no test DB — matches the module's convention).

- [ ] **Step 5: Commit**
```bash
git add crates/senseid/src/db/pg_store.rs
git commit -m "feat(senseid): pg_store CRUD for playbooks/rules/intake_guide + playbook_run insert"
```

---

### Task 8: `recommend_playbook` — daemon endpoint + MCP tool

**Files:**
- Create: `crates/senseid/src/api/handlers/playbook.rs`
- Modify: `crates/senseid/src/api/handlers/mod.rs`, `crates/senseid/src/api/routes.rs`, `crates/mcp/src/lib.rs`, `crates/senseid/src/api/handlers/mcp_manifests.rs`

- [ ] **Step 1: Write the handler** (`api/handlers/playbook.rs`) — pure axes-in path first (classify added in Task 9)
```rust
use axum::{extract::State, Json};
use crate::api::AppState;
use crate::playbook::{Axes, Lifecycle, Intent, Risk, recommend};

/// POST /api/playbook/recommend  { lifecycle, intent, risk, session_id?, feature?, confirm? }
pub(crate) async fn recommend_playbook(
    State(state): State<AppState>, Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let parse = |k: &str, f: fn(&str)->Option<_>| body[k].as_str().and_then(f);
    let (Some(lf), Some(it), Some(rk)) = (
        parse("lifecycle", Lifecycle::parse), parse("intent", Intent::parse), parse("risk", Risk::parse)
    ) else {
        return Json(serde_json::json!({ "error": "lifecycle/intent/risk required (valid axis values)" }));
    };
    let axes = Axes { lifecycle: lf, intent: it, risk: rk };
    let rules = match state.store.list_playbook_rules().await {
        Ok(r) => r, Err(e) => return Json(serde_json::json!({ "error": e })),
    };
    let rec = recommend(&axes, &rules);
    // Persist the run (recommend-and-confirm defaults confirmed=false until the caller confirms).
    let confirmed = body["confirm"].as_bool().unwrap_or(false);
    let session_id = body["session_id"].as_str().and_then(|s| s.parse().ok());
    let _ = state.store.insert_playbook_run(
        session_id, body["feature"].as_str(),
        lf.as_str(), it.as_str(), rk.as_str(),
        rec.rule_id, &rec.playbook, &rec.rationale, confirmed,
    ).await;
    Json(serde_json::json!({
        "playbook": rec.playbook, "rationale": rec.rationale,
        "rule": rec.rule_name, "defaulted": rec.defaulted
    }))
}
```
Add `pub mod playbook;` to `api/handlers/mod.rs`; add `.route("/api/playbook/recommend", post(handlers::playbook::recommend_playbook))` in `routes.rs`.

- [ ] **Step 2: Add the MCP tool** (`crates/mcp/src/lib.rs`) — schema (mirror `save_memory` `tool(...)`) + dispatcher branch (mirror the `save_memory` branch)
```rust
tool("recommend_playbook",
    "Recommend a playbook for the current work chunk from its lifecycle/intent/risk. \
     Call after the intake dialogue has classified the chunk. Returns playbook + rationale.",
    &[
        ("lifecycle", "string", "greenfield | stable"),
        ("intent",    "string", "explore | ux | feature | enhancement | bug"),
        ("risk",      "string", "low | high (blast-radius)"),
    ],
    &[
        ("session_id", "string", "session UUID to attribute the run to"),
        ("feature",    "string", "feature slug when the chunk maps to a dossier"),
        ("confirm",    "string", "true to record the run as confirmed"),
    ]),
```
Dispatcher branch:
```rust
"recommend_playbook" => {
    let mut body = serde_json::json!({
        "lifecycle": args["lifecycle"], "intent": args["intent"], "risk": args["risk"],
    });
    for k in ["session_id","feature","confirm"] {
        if let Some(v) = args[k].as_str().filter(|s| !s.is_empty()) { body[k] = serde_json::json!(v); }
    }
    Some(DaemonRequest::post_json("/api/playbook/recommend", body))
}
```

- [ ] **Step 3: Add the manifest entry** in `mcp_manifests.rs` (mirror an existing `McpToolManifest` with `kind: McpToolKind::Query`, summary, inputs, example).

- [ ] **Step 4: Build + test the round-trip**

Run: `cargo build -p senseid -p sensei-mcp 2>&1 | tail -5` (Expected: clean)
Add an integration assertion (or manual curl in the step):
```bash
curl -s localhost:7744/api/playbook/recommend -H 'content-type: application/json' \
  -d '{"lifecycle":"stable","intent":"bug","risk":"low"}' | python3 -m json.tool
```
Expected: `"playbook": "debug_flow"`.

- [ ] **Step 5: Commit**
```bash
git add crates/senseid/src/api/handlers/playbook.rs crates/senseid/src/api/handlers/mod.rs crates/senseid/src/api/routes.rs crates/mcp/src/lib.rs crates/senseid/src/api/handlers/mcp_manifests.rs
git commit -m "feat(senseid,mcp): recommend_playbook endpoint + MCP tool (axes -> playbook, persists run)"
```

---

### Task 9: `classify_chunk` — gateway classify with heuristic fallback

**Files:**
- Modify: `crates/senseid/src/api/handlers/playbook.rs`

- [ ] **Step 1: Write the failing test** (heuristic fallback is pure + testable without a gateway)
```rust
#[cfg(test)]
mod classify_tests {
    use super::*;
    #[test]
    fn heuristic_bug_on_stable() {
        let a = heuristic_axes("fix the crash when the token refreshes", /*has_existing_code=*/true, /*blast=*/2);
        assert_eq!(a.intent.as_str(), "bug");
        assert_eq!(a.lifecycle.as_str(), "stable");
    }
    #[test]
    fn heuristic_high_blast() {
        let a = heuristic_axes("rename the session store used everywhere", true, 40);
        assert_eq!(a.risk.as_str(), "high");
    }
}
```

- [ ] **Step 2: Run + confirm fail**

Run: `cargo test -p senseid classify_tests:: 2>&1 | tail -5`
Expected: FAIL — `heuristic_axes` not found.

- [ ] **Step 3: Implement `heuristic_axes` + `classify_chunk`** (gateway call mirrors `hook_gate` in `api/handlers/sessions.rs`: `InferenceRequest{ capability: TextChat, chain: Some("reasoning"), ... }`, `tokio::time::timeout(8s, state.gateway.execute(&req))`, FAIL to heuristic on error/timeout)
```rust
use crate::playbook::{Axes, Lifecycle, Intent, Risk};

/// Deterministic fallback — also the classifier when the gateway is unavailable.
pub(crate) fn heuristic_axes(text: &str, has_existing_code: bool, blast: i64) -> Axes {
    let t = text.to_lowercase();
    let lifecycle = if has_existing_code { Lifecycle::Stable } else { Lifecycle::Greenfield };
    let intent =
        if t.contains("bug") || t.contains("fix") || t.contains("crash") || t.contains("regression") { Intent::Bug }
        else if t.contains("ui") || t.contains("design") || t.contains("mockup") || t.contains("screen") { Intent::Ux }
        else if t.contains("improve") || t.contains("enhance") || t.contains("tweak") { Intent::Enhancement }
        else if !has_existing_code && (t.contains("explore") || t.contains("spike") || t.contains("try")) { Intent::Explore }
        else { Intent::Feature };
    let risk = if blast >= 10 { Risk::High } else { Risk::Low };
    Axes { lifecycle, intent, risk }
}
```
Then `classify_chunk(state, text, has_existing_code, blast) -> Axes`: build the `reasoning`-chain `InferenceRequest` asking for a JSON `{lifecycle,intent,risk}`, `timeout(8s)`; on `Ok(Ok(resp)) if resp.success` parse+validate against the axis `parse` fns, else `heuristic_axes(...)` (log the fail-open, per the no-silent-errors rule). Wire an optional `text` path into the `recommend_playbook` handler: if `lifecycle/intent/risk` are absent but `chunk` text is present, classify first.

- [ ] **Step 4: Run tests + build**

Run: `cargo test -p senseid classify_tests:: 2>&1 | tail -5` (Expected: 2 passed)
Run: `cargo build -p senseid 2>&1 | tail -3` (Expected: clean)

- [ ] **Step 5: Commit**
```bash
git add crates/senseid/src/api/handlers/playbook.rs
git commit -m "feat(senseid): classify_chunk (gateway reasoning + heuristic fallback) for the intake recommender"
```

---

## Final verification (whole plan)

- [ ] `cargo test -p senseid 2>&1 | tail -20` — green (incl. playbook resolver + classify + pg_store round-trip).
- [ ] `cargo clippy -p senseid -p sensei-mcp 2>&1 | tail -5` — no warnings (zero-errors-policy).
- [ ] DDL applies cleanly on a fresh `sensei_test` (enums + 4 tables + indexes + 3 imports); seeds = 6 playbooks, 6 rules, 4 guide rows.
- [ ] `POST /api/playbook/recommend` returns the §3.3 mapping for each of the 6 situations; a no-match input returns `gsd` + `defaulted:true`.
- [ ] Dispatch a final whole-plan code review (subagent) against this plan + the design.

## Self-review notes (author)

- **Spec coverage:** DDL (enums+playbooks+rules+intake_guide+run+seeds) ✓ T1–T5; resolver ✓ T6; pg_store ✓ T7; recommend_playbook tool ✓ T8; classify_chunk ✓ T9. Always-the-entry + `/sensei:intake` = **Plan 2** (out of scope here, by design).
- **Session FK:** corrected to `activity.sessions(id)` (not `sensei.sessions`).
- **Type consistency:** axis `as_str`/`parse` labels (`greenfield/stable`, `explore/ux/feature/enhancement/bug`, `low/high`) are identical across enums, seeds, pg_store casts, and the resolver.
- **Deferred (not this plan):** `outcome` population (§9), org/learned rule authoring (Dōjō), the Sensei-app form renderer.
