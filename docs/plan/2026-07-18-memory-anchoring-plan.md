# Memory-Anchoring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Anchor memories to spine slots (`spine_slot` + `feature` on `sensei.memories`) with heuristic auto-anchoring for analyzer memories and slot-scoped retrieval, so memories can be surfaced by the slot being worked on (push-not-pull foundation).

**Architecture:** Additive DDL (a `spine_slot` enum + two nullable columns + an index). A pure `default_slot(category, mtype)` heuristic feeds the analyzer's write path so new memories self-anchor with no LLM. The manual write paths (`save_memory`/`propose_memory`) accept an optional slot+feature (scope-validated). Retrieval gains `list_memories_for_slot` and a slot-aware `assemble_context`; the MCP `context_pack`/`get_layered_context` tools accept an optional `slot`/`feature`. No hook/agent-loop change (auto-inject-by-cwd is a deferred follow-up).

**Tech Stack:** Rust (edition 2024), `sqlx`, PostgreSQL (`dbd` for DDL), `clap`-free (daemon + MCP crates). Reuses `sensei.memories`, `assemble_context`, `create_memory`, `MemoryBody`/`insert_with_status`, `build_memory_body`.

**Spec:** `docs/plan/2026-07-18-memory-anchoring-design.md`.

---

## File Structure

- **Modify** `database/ddl/enum/sensei/spine_slot.ddl` (**create**) — the `spine_slot` enum.
- **Modify** `database/ddl/table/sensei/memories.ddl` — add `spine_slot`/`feature` columns + index.
- **Create** `crates/senseid/src/memory_slot.rs` — the `SpineSlot` domain type, `default_slot()` heuristic, and `validate_scope()` (pure, unit-tested).
- **Modify** `crates/senseid/src/lib.rs` (or the daemon's module root) — `mod memory_slot;`.
- **Modify** `crates/senseid/src/db/pg_store.rs` — `create_memory` gains slot params; new `list_memories_for_slot`; `assemble_context` gains an optional slot hint; the analyzer generate INSERT sets `default_slot`.
- **Modify** `crates/senseid/src/api/handlers/knowledge.rs` — `MemoryBody` + `insert_with_status` carry `spine_slot`/`feature` (scope-validated).
- **Modify** `crates/senseid/src/api/handlers/query.rs` — `context_pack` accepts + forwards the slot hint.
- **Modify** `crates/mcp/src/lib.rs` — `context_pack`/`get_layered_context` tool schemas gain optional `slot`/`feature`; `build_memory_body` carries them for `save_memory`/`propose_memory`.

---

## Task 1: DDL — `spine_slot` enum + columns + index

**Files:**
- Create: `database/ddl/enum/sensei/spine_slot.ddl`
- Modify: `database/ddl/table/sensei/memories.ddl`

- [ ] **Step 1: Create the enum DDL**

`database/ddl/enum/sensei/spine_slot.ddl`:

```sql
set search_path to sensei, extensions;
-- The doc-slot names the scaffolder produces (project spine §3.2 + feature dossier).
-- `feature` (on memories) disambiguates scope: design/decisions exist at both scopes.
create type spine_slot as enum (
  'vision', 'personas', 'journeys', 'roadmap', 'design', 'mockups',
  'decisions', 'brief', 'plan', 'tests'
);
```

- [ ] **Step 2: Add the columns + index to `memories.ddl`**

In `database/ddl/table/sensei/memories.ddl`, add two columns after `tags` (before `category`), keeping the leading-comma style:

```sql
, spine_slot               spine_slot
, feature                  text
```

And after the existing `memories_scope_idx` index, add:

```sql
create index if not exists memories_spine_slot_idx
    on memories(project_id, spine_slot)
 where status = 'active';
```

- [ ] **Step 3: Apply + verify against the dev DB**

Run (dev supabase / the local sensei DB — this is additive, `dbd reconcile` won't drop):
```bash
cd database
dbd reconcile --scope sensei -e dev --dry-run    # confirm: + create spine_slot, ~ alter memories (add cols/index)
dbd reconcile --scope sensei -e dev
```
Then verify:
```bash
psql "$SENSEI_DEV_DB_URL" -tAc "select column_name from information_schema.columns where table_schema='sensei' and table_name='memories' and column_name in ('spine_slot','feature') order by 1;"
```
Expected: `feature` and `spine_slot` printed. (If the daemon reads a released DDL bundle, set `SENSEI_DDL_DIR` to this repo's `database/` for the following Rust tests — see CLAUDE.md.)

- [ ] **Step 4: Commit**

```bash
git add database/ddl/enum/sensei/spine_slot.ddl database/ddl/table/sensei/memories.ddl
git commit -m "feat(db): sensei.spine_slot enum + memories.spine_slot/feature + index

Spec 2026-07-18-memory-anchoring-design.md §1. Additive: the spine_slot enum
(doc-slot names), nullable spine_slot/feature columns on sensei.memories, and a
(project_id, spine_slot) partial index for slot-scoped retrieval.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Pure `SpineSlot` + `default_slot` + `validate_scope`

**Files:**
- Create: `crates/senseid/src/memory_slot.rs`
- Modify: `crates/senseid/src/lib.rs` (add `mod memory_slot;` next to the other `mod`s)

- [ ] **Step 1: Create `memory_slot.rs` with types + stubs + failing tests**

```rust
//! Spine-slot anchoring for memories (design 2026-07-18-memory-anchoring).
//! Pure: the slot vocabulary, the analyzer's default-slot heuristic, and the
//! project-vs-feature scope rule. No IO.

/// The doc-slot a memory anchors to. `as_str()` matches the `sensei.spine_slot`
/// enum labels exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpineSlot {
    Vision, Personas, Journeys, Roadmap, Design, Mockups,
    Decisions, Brief, Plan, Tests,
}

impl SpineSlot {
    pub fn as_str(self) -> &'static str {
        match self {
            SpineSlot::Vision => "vision",
            SpineSlot::Personas => "personas",
            SpineSlot::Journeys => "journeys",
            SpineSlot::Roadmap => "roadmap",
            SpineSlot::Design => "design",
            SpineSlot::Mockups => "mockups",
            SpineSlot::Decisions => "decisions",
            SpineSlot::Brief => "brief",
            SpineSlot::Plan => "plan",
            SpineSlot::Tests => "tests",
        }
    }
    pub fn parse(s: &str) -> Option<SpineSlot> {
        Some(match s {
            "vision" => SpineSlot::Vision,
            "personas" => SpineSlot::Personas,
            "journeys" => SpineSlot::Journeys,
            "roadmap" => SpineSlot::Roadmap,
            "design" => SpineSlot::Design,
            "mockups" => SpineSlot::Mockups,
            "decisions" => SpineSlot::Decisions,
            "brief" => SpineSlot::Brief,
            "plan" => SpineSlot::Plan,
            "tests" => SpineSlot::Tests,
            _ => return None,
        })
    }
    /// Project-only slots never carry a feature.
    fn is_project_only(self) -> bool {
        matches!(self, SpineSlot::Vision | SpineSlot::Personas | SpineSlot::Journeys
            | SpineSlot::Roadmap | SpineSlot::Mockups)
    }
    /// Feature-only slots require a feature.
    fn is_feature_only(self) -> bool {
        matches!(self, SpineSlot::Brief | SpineSlot::Plan | SpineSlot::Tests)
    }
}

/// The analyzer's default slot for a generated memory, from its category/type.
/// Structural knowledge → design; settled learnings/decisions → decisions.
/// `category` is `sensei.memory_category`, `mtype` is `sensei.memory_type`.
pub fn default_slot(category: Option<&str>, mtype: &str) -> SpineSlot {
    if category == Some("pattern") || category == Some("convention") {
        return SpineSlot::Design;
    }
    match mtype {
        "pattern" | "convention" => SpineSlot::Design,
        _ => SpineSlot::Decisions, // decision, correctness, preference, continuity, question
    }
}

/// Validate a (slot, feature) pair against the scope rule. Ok(()) or an error msg.
pub fn validate_scope(slot: SpineSlot, feature: Option<&str>) -> Result<(), String> {
    let has_feature = feature.map(|f| !f.is_empty()).unwrap_or(false);
    if slot.is_project_only() && has_feature {
        return Err(format!("slot {:?} is project-scope — drop the feature", slot.as_str()));
    }
    if slot.is_feature_only() && !has_feature {
        return Err(format!("slot {:?} is feature-scope — a feature is required", slot.as_str()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_roundtrips_through_parse() {
        for s in [SpineSlot::Vision, SpineSlot::Design, SpineSlot::Decisions,
                  SpineSlot::Brief, SpineSlot::Plan, SpineSlot::Tests, SpineSlot::Mockups] {
            assert_eq!(SpineSlot::parse(s.as_str()), Some(s));
        }
        assert_eq!(SpineSlot::parse("nope"), None);
    }

    #[test]
    fn default_slot_maps_structural_to_design_and_rest_to_decisions() {
        assert_eq!(default_slot(Some("pattern"), "pattern"), SpineSlot::Design);
        assert_eq!(default_slot(Some("convention"), "convention"), SpineSlot::Design);
        assert_eq!(default_slot(Some("correctness"), "decision"), SpineSlot::Decisions);
        assert_eq!(default_slot(None, "preference"), SpineSlot::Decisions);
        assert_eq!(default_slot(None, "continuity"), SpineSlot::Decisions);
        assert_eq!(default_slot(None, "question"), SpineSlot::Decisions);
    }

    #[test]
    fn validate_scope_enforces_the_rule() {
        assert!(validate_scope(SpineSlot::Vision, None).is_ok());
        assert!(validate_scope(SpineSlot::Vision, Some("auth")).is_err());
        assert!(validate_scope(SpineSlot::Brief, Some("auth")).is_ok());
        assert!(validate_scope(SpineSlot::Brief, None).is_err());
        assert!(validate_scope(SpineSlot::Design, None).is_ok());
        assert!(validate_scope(SpineSlot::Design, Some("auth")).is_ok());
        assert!(validate_scope(SpineSlot::Decisions, Some("auth")).is_ok());
    }
}
```

Add `mod memory_slot;` to `crates/senseid/src/lib.rs` alongside the existing module declarations.

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test -p senseid memory_slot:: -- --nocapture`
Expected: PASS — all three tests. (These are pure; they define the contract Tasks 3–5 consume. Written green because the impl is trivial and fully shown; the red-first discipline lives in the IO tasks that follow.)

- [ ] **Step 3: Commit**

```bash
git add crates/senseid/src/memory_slot.rs crates/senseid/src/lib.rs
git commit -m "feat(senseid): SpineSlot + default_slot heuristic + validate_scope (pure)

The slot vocabulary (matches sensei.spine_slot), the analyzer's default-slot map
(pattern/convention→design, else→decisions), and the project-vs-feature scope rule.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Write-path anchoring

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs` (`create_memory` + the analyzer generate INSERT)
- Modify: `crates/senseid/src/api/handlers/knowledge.rs` (`MemoryBody`, `insert_with_status`)

- [ ] **Step 1: Extend `create_memory` to persist slot/feature (failing test)**

In `crates/senseid/src/db/pg_store.rs`, add a tempdb test near the other memory tests:

```rust
    #[tokio::test]
    async fn create_memory_persists_spine_slot_and_feature() {
        let pg = pg_store().await;
        let pid = create_test_project(&pg, "slot_write").await;
        let id = pg.create_memory(
            Some(&pid), "project", None, "decision", "t", "c", None, None,
            Some("decisions"), Some("auth"),
        ).await.unwrap();
        let row: (Option<String>, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT spine_slot::text, feature FROM sensei.memories WHERE id = $1"
        ).bind(id).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(row, (Some("decisions".into()), Some("auth".into())));
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p senseid create_memory_persists_spine_slot -- --nocapture`
Expected: FAIL to compile — `create_memory` takes 8 args, not 10.

- [ ] **Step 3: Add slot/feature params to `create_memory`**

In `crates/senseid/src/db/pg_store.rs`, change `create_memory`'s signature + INSERT:

```rust
    pub async fn create_memory(
        &self, project_id: Option<&uuid::Uuid>, scope: &str, scope_filter: Option<&str>,
        mem_type: &str, title: &str, content: &str, impact: Option<&str>,
        session_id: Option<&uuid::Uuid>,
        spine_slot: Option<&str>, feature: Option<&str>,
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
```

Update every existing `create_memory(...)` caller to pass `None, None` for the two new args (search: `grep -rn "create_memory(" crates/senseid/src`), EXCEPT the analyzer generate caller handled in Step 5.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p senseid create_memory_persists_spine_slot -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Anchor analyzer-generated memories via `default_slot`**

Find the analyzer's memory creation in the generate path (`grep -rn "create_memory(" crates/senseid/src/tasks/handlers/generate.rs` and `crates/senseid/src/db/pg_store.rs` for the generate INSERT). At that call site, compute the slot and pass it:

```rust
    let slot = crate::memory_slot::default_slot(category.as_deref(), &mem_type);
    // …create_memory(…, Some(slot.as_str()), None)  // analyzer memories are project-scope
```

Add a test asserting a generated memory lands anchored (mirror the existing `generate_writes_memory_and_recommendation_idempotently` test, adding a `spine_slot is not null` assertion on the written row). Run: `cargo test -p senseid generate_writes_memory -- --nocapture` → PASS.

- [ ] **Step 6: Thread slot/feature through `MemoryBody` + `insert_with_status`**

In `crates/senseid/src/api/handlers/knowledge.rs`, add to `MemoryBody`:

```rust
    pub spine_slot: Option<String>,
    pub feature:    Option<String>,
```

In `insert_with_status`, before creating the memory, validate + pass them:

```rust
    if let Some(slot_s) = body.spine_slot.as_deref() {
        let slot = crate::memory_slot::SpineSlot::parse(slot_s)
            .ok_or_else(|| err(StatusCode::BAD_REQUEST, &format!("unknown spine_slot {slot_s:?}")))?;
        crate::memory_slot::validate_scope(slot, body.feature.as_deref())
            .map_err(|e| err(StatusCode::BAD_REQUEST, &e))?;
    }
    // …pass body.spine_slot.as_deref(), body.feature.as_deref() into the create call…
```

(If `insert_with_status` calls a store helper other than `create_memory`, extend that helper the same way; the two new bind params are additive.)

- [ ] **Step 7: Test the handler validates + persists (failing → pass)**

Add a handler-level test (mirror an existing knowledge.rs handler test) posting a `MemoryBody` with `spine_slot="brief"` and no feature → expect 400; with `spine_slot="brief", feature="auth"` → 200 and the row has the slot. Run the knowledge tests → PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/senseid/src/db/pg_store.rs crates/senseid/src/api/handlers/knowledge.rs
git commit -m "feat(senseid): anchor memories on write (analyzer heuristic + manual slot)

create_memory persists spine_slot/feature; analyzer generate sets default_slot
(project scope); save_memory/propose_memory accept + scope-validate spine_slot/feature.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Slot-scoped retrieval

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs` (`list_memories_for_slot`, slot-aware `assemble_context`)

- [ ] **Step 1: Add `list_memories_for_slot` (failing test)**

Add a tempdb test: create three memories (one `design`/no-feature, one `design`/`feature=auth`, one `decisions`), then assert `list_memories_for_slot(pid, "design", None)` returns only the first, and `(pid, "design", Some("auth"))` returns only the second.

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p senseid list_memories_for_slot -- --nocapture`
Expected: FAIL — method does not exist.

- [ ] **Step 3: Implement `list_memories_for_slot`**

```rust
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
```

Run: `cargo test -p senseid list_memories_for_slot -- --nocapture` → PASS.

- [ ] **Step 4: Make `assemble_context` slot-aware (failing test)**

Add a test: with two memories (one anchored to `design`, one unanchored), `assemble_context(pid, &[], None, 50, Some(("design", None)))` puts the design-anchored memory first in the returned bundle.

- [ ] **Step 5: Add the optional slot hint to `assemble_context`**

Change the signature to `assemble_context(&self, project_id, stack_ids, tags, limit, slot: Option<(&str, Option<&str>)>)`. When `slot` is `Some`, prepend `list_memories_for_slot(project_id, slot, feature, limit)` results to the assembled bundle (deduped by id) so slot-anchored memories lead. When `None`, behavior is unchanged. Update all existing callers to pass `None` (`grep -rn "assemble_context(" crates/senseid/src`).

Run: `cargo test -p senseid assemble_context -- --nocapture` → PASS (incl. the pre-existing `assemble_context_*` tests, now with the extra `None` arg).

- [ ] **Step 6: Commit**

```bash
git add crates/senseid/src/db/pg_store.rs
git commit -m "feat(senseid): slot-scoped retrieval — list_memories_for_slot + slot-aware assemble_context

list_memories_for_slot(project, slot, feature) behind the partial index; assemble_context
gains an optional (slot,feature) hint that leads the bundle with slot-anchored memories,
backward-compatible when absent.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: MCP + HTTP plumbing

**Files:**
- Modify: `crates/senseid/src/api/handlers/query.rs` (`context_pack` forwards the hint)
- Modify: `crates/mcp/src/lib.rs` (tool schemas + `build_memory_body`)

- [ ] **Step 1: `build_memory_body` carries slot/feature (failing test)**

In `crates/mcp/src/lib.rs`, `build_memory_body` (the pure JSON builder for save/propose) — add a test asserting that when the tool args include `spine_slot`/`feature`, the built body JSON contains them. Run → FAIL.

- [ ] **Step 2: Implement — pass slot/feature into the body**

Extend `build_memory_body` to copy `spine_slot` + `feature` from the tool args into the JSON body (they map onto `MemoryBody`). Add `spine_slot`/`feature` to the `save_memory` + `propose_memory` tool schemas (optional string properties, with a one-line description: "spine slot to anchor to (vision|design|decisions|brief|plan|tests|…); feature name for feature-scope slots"). Run the test → PASS.

- [ ] **Step 3: `context_pack`/`get_layered_context` accept a slot hint**

Add optional `slot`/`feature` properties to the `context_pack` + `get_layered_context` tool schemas. In `crates/senseid/src/api/handlers/query.rs`, thread an optional `slot`/`feature` query param into `context_pack` → `assemble_context(..., Some((slot, feature)))`. When absent, pass `None`. Add a test that `context_pack` with a `slot` returns the slot's memories in the bundle (or an MCP-arg mapping test if a full daemon isn't available in the mcp crate's tests).

- [ ] **Step 4: Full build + test + clippy**

Run:
```bash
cargo test -p senseid -p sensei-mcp
cargo clippy -p senseid -p sensei-mcp -- -D warnings
```
Expected: all green, no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/senseid/src/api/handlers/query.rs crates/mcp/src/lib.rs
git commit -m "feat(mcp): slot-aware context + slot-anchored save/propose

save_memory/propose_memory tools accept spine_slot/feature; context_pack &
get_layered_context accept an optional slot/feature that flows into the slot-aware
assemble_context. Completes the memory-anchoring foundation.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review

- **Spec coverage:** §1 data model → Task 1 (enum+cols+index); §2 acquisition → Task 2 (`default_slot`, `validate_scope`) + Task 3 (analyzer heuristic + manual save/propose with validation); §3 retrieval → Task 4 (`list_memories_for_slot` + slot-aware `assemble_context`) + Task 5 (MCP `context_pack`/`get_layered_context` + save/propose tools). Deferred items (LLM classification, auto-inject-by-cwd, backfill, federation) are explicitly out and untouched.
- **Type consistency:** `SpineSlot` + `.as_str()`/`.parse()` (Task 2) used by `create_memory(..., Some(&str), Option<&str>)` (Task 3), `list_memories_for_slot(&Uuid,&str,Option<&str>,i64)` + `assemble_context(..., Option<(&str,Option<&str>)>)` (Task 4), and the MCP body/schemas (Task 5). Enum labels match the `sensei.spine_slot` DDL (Task 1) exactly.
- **Placeholders:** none — pure logic + DDL shown in full; wiring steps name exact functions/files + the `grep` to find every caller to update (create_memory, assemble_context).
- **Reuse (CLAUDE.md DRY):** extends `create_memory`/`assemble_context`/`MemoryBody`/`build_memory_body` rather than adding parallel paths; the one `default_slot`/`validate_scope`/`SpineSlot` lives in `memory_slot.rs` and is the single source used by every layer.
- **Migration note (verify at execution):** confirm the daemon reads this repo's `database/` (via a release bump or `SENSEI_DDL_DIR`) so the new columns are present when the Rust tempdb tests run (per [[feedback_ddl_source_first]]).
