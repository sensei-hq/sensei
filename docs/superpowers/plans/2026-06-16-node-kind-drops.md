# node_kind Enum-Drop Fix — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop silently dropping `doc`/`struct`/`component`/`hook`/`extension` nodes by making the `node_kind` enum cover every kind the indexer produces, log any future cast failure, embed the new kinds, and backfill production.

**Architecture:** All DB node-kind strings flow from `NodeKind::as_str()` (symbols via `code.rs:52`, docs via `doc.rs:22`, files via the literal `"file"`). `upsert_node` casts the string to `sensei.node_kind`; kinds absent from the enum raise an error that `.ok()` swallows. Fix = add the 5 missing values to the enum DDL (append, so `dbd` emits `ALTER TYPE … ADD VALUE`), remove 3 dead `NodeKind` variants, log the swallow, widen the embedding allowlist, then `dbd deploy` + rescan.

**Tech Stack:** Rust (senseid, BIN-ONLY tests via `cargo test -p senseid --bin senseid`), PostgreSQL (`sensei` prod / `sensei_test` @ localhost:5432), `dbd` schema tool, tracing.

**Branch:** Work on `develop` (project convention). Live-DB steps (Task 7) require explicit in-chat authorization.

---

### Task 1: Guard test — every `NodeKind::as_str()` is a valid enum value

**Files:**
- Modify: `crates/senseid/src/types.rs` (add `NodeKind::all()` + a `#[cfg(test)] mod` guard)

- [ ] **Step 1: Add `NodeKind::all()`** — insert immediately after `from_symbol_kind` (ends `types.rs:144`), inside `impl NodeKind`:

```rust
    /// Every node kind, in declaration order. Backs the schema-consistency
    /// guard test and any exhaustive enumeration of kinds.
    pub fn all() -> &'static [NodeKind] {
        use NodeKind::*;
        &[
            Repo, CodeGroup, DocGroup, Package, Module, Function, Method,
            Class, Struct, Interface, Enum, Const, Type, Component, Hook,
            File, Doc, Extension,
        ]
    }
```

- [ ] **Step 2: Add the guard test** — append at the end of `crates/senseid/src/types.rs`:

```rust
#[cfg(test)]
mod node_kind_schema_tests {
    use super::NodeKind;
    use std::collections::HashSet;

    /// Every NodeKind::as_str() must be a value in the node_kind DDL enum.
    /// Otherwise upsert_node's `$2::sensei.node_kind` cast fails and the node
    /// is dropped. Reading the DDL keeps code and schema from drifting.
    #[test]
    fn every_node_kind_is_a_valid_enum_value() {
        let ddl = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../database/ddl/enum/sensei/node_kind.ddl"
        ));
        // Enum labels are the only single-quoted tokens in this file.
        let enum_values: HashSet<&str> = ddl.split('\'').skip(1).step_by(2).collect();
        for k in NodeKind::all() {
            assert!(
                enum_values.contains(k.as_str()),
                "NodeKind::{:?} emits {:?}, absent from node_kind.ddl: {:?}",
                k, k.as_str(), enum_values
            );
        }
    }
}
```

- [ ] **Step 3: Run the test — expect FAIL**

Run: `cargo test -p senseid --bin senseid every_node_kind_is_a_valid_enum_value`
Expected: FAIL — `NodeKind::Repo emits "repo", absent from node_kind.ddl` (or one of `code-group`/`doc-group`/`struct`/`component`/`hook`/`doc`/`extension`).

- [ ] **Step 4: Commit (red test)**

```bash
git add crates/senseid/src/types.rs
git commit -m "test(senseid): guard that NodeKind::as_str matches the node_kind enum (red)"
```

---

### Task 2: Make the enum and `NodeKind` consistent

**Files:**
- Modify: `database/ddl/enum/sensei/node_kind.ddl`
- Modify: `crates/senseid/src/types.rs` (remove 3 dead variants from the enum, `as_str`, `from_str`, `all()`)

- [ ] **Step 1: Confirm the 3 variants are dead** (no producers/consumers outside `types.rs`)

Run: `grep -rn "NodeKind::Repo\b\|NodeKind::CodeGroup\|NodeKind::DocGroup\|\"repo\"\|\"code-group\"\|\"doc-group\"" crates --include="*.rs" | grep -v "src/types.rs"`
Expected: no output. (If any consumer appears, stop and map it instead of removing.)

- [ ] **Step 2: Append the 5 new values to the enum DDL** — replace the body of `database/ddl/enum/sensei/node_kind.ddl`:

```sql
set search_path to sensei, extensions;

create type node_kind
    as enum (
        'file'
      , 'module', 'package'
      , 'class', 'interface', 'function', 'method'
      , 'property', 'field', 'parameter'
      , 'type', 'const', 'enum', 'enum_variant'
      , 'section'
      , 'rationale'
      , 'struct', 'component', 'hook', 'doc', 'extension'
    );
```

(New values are **appended last** so `dbd` emits `ALTER TYPE … ADD VALUE` rather than recreating the type.)

- [ ] **Step 3: Remove the 3 dead variants from `NodeKind`** in `crates/senseid/src/types.rs`:
  - In the `enum NodeKind` block (`types.rs:44-67`), delete the `Repo,`, `CodeGroup,`, `DocGroup,` lines and the `// Structural grouping` comment line.
  - In `as_str` (`types.rs:71-90`), delete the `Self::Repo => "repo",`, `Self::CodeGroup => "code-group",`, `Self::DocGroup => "doc-group",` arms.
  - In `from_str` (`types.rs:95-115`), delete the `"repo" => Self::Repo,`, `"code-group" => Self::CodeGroup,`, `"doc-group" => Self::DocGroup,` arms.
  - In `all()` (added in Task 1), delete `Repo, CodeGroup, DocGroup, ` from the list.

- [ ] **Step 4: Run the guard test + full bin suite — expect PASS**

Run: `cargo test -p senseid --bin senseid every_node_kind_is_a_valid_enum_value`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add database/ddl/enum/sensei/node_kind.ddl crates/senseid/src/types.rs
git commit -m "fix(schema): add struct/component/hook/doc/extension to node_kind; drop dead variants"
```

---

### Task 3: Integration test — new kinds persist (`sensei_test`)

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs` (add a test beside `node_upsert_and_query`, ~`:3241`)

- [ ] **Step 1: Write the integration test** — insert after `node_upsert_and_query` (`pg_store.rs:3241`), inside the same `mod tests`:

```rust
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
```

- [ ] **Step 2: Run it — expect FAIL** (sensei_test still has the old enum)

Run: `cargo test -p senseid --bin senseid upsert_persists_doc_and_symbol_kinds`
Expected: FAIL — `upsert doc failed: … invalid input value for enum sensei.node_kind: "doc"`.

- [ ] **Step 3: Preview the migration with dbd** (verify it's `ADD VALUE`, not a type recreate)

Run: `cd database && dbd graph`
Expected: the planned change for `node_kind` is `ALTER TYPE … ADD VALUE` for the 5 new labels. If it proposes dropping/recreating the type, STOP — re-check the append ordering before proceeding.

- [ ] **Step 4: Deploy the schema to `sensei_test`**

Run: confirm `dbd`'s target is `sensei_test` (`dbd deploy --help` for the target flag/env), then `cd database && dbd deploy`.
Expected: success; `psql -d sensei_test -tAc "SELECT 1 FROM pg_enum e JOIN pg_type t ON e.enumtypid=t.oid WHERE t.typname='node_kind' AND e.enumlabel='doc'"` returns `1`.

- [ ] **Step 5: Run the test — expect PASS**

Run: `cargo test -p senseid --bin senseid upsert_persists_doc_and_symbol_kinds`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/senseid/src/db/pg_store.rs
git commit -m "test(senseid): doc/struct/component/hook/extension nodes persist"
```

---

### Task 4: Widen the embedding allowlist

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs:556-557` and `:647-648`

- [ ] **Step 1: Write the test** — append inside `mod tests`:

```rust
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
```

- [ ] **Step 2: Run it — expect FAIL**

Run: `cargo test -p senseid --bin senseid doc_nodes_are_embeddable`
Expected: FAIL — assertion (doc not in allowlist).

- [ ] **Step 3: Add the new kinds to both allowlist queries.** In `nodes_without_embeddings` (`pg_store.rs:556-557`) replace the `kind IN (...)` clause with:

```rust
                    AND kind IN ('file','function','method','class','interface',
                                 'type','const','enum','enum_variant','section',
                                 'struct','component','hook','doc','extension')
```

Apply the identical replacement to the second occurrence (`pg_store.rs:647-648`).

- [ ] **Step 4: Run it — expect PASS**

Run: `cargo test -p senseid --bin senseid doc_nodes_are_embeddable`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/senseid/src/db/pg_store.rs
git commit -m "feat(embed): embed doc/struct/component/hook/extension nodes"
```

---

### Task 5: Log the swallowed `upsert_node` failures

**Files:**
- Modify: `crates/senseid/src/tasks/handlers/process.rs:614-616` and `:627-634`

No new unit test: post-fix every produced kind is valid, so the error branch is unreachable in normal operation — it exists purely so a *future* mismatch is visible, not silent. Verified by clippy + the existing suite compiling/passing.

- [ ] **Step 1: Replace the file-node `.ok()`** (`process.rs:614-616`):

```rust
            // Write file node
            let file_node_id = match ctx.pg().upsert_node(
                &folder_id, &result.kind, &result.rel_path, &result.rel_path, None, None, None, None
            ).await {
                Ok(id) => Some(id),
                Err(e) => {
                    tracing::warn!(kind = %result.kind, file = %result.rel_path, error = %e, "upsert file node failed; skipping file");
                    None
                }
            };
```

- [ ] **Step 2: Replace the symbol-node `if let Ok`** (`process.rs:627-633`):

```rust
                match ctx.pg().upsert_node(
                    &folder_id, &sym.kind, &sym.name, &result.rel_path,
                    parent_uuid.as_ref(), sym.signature.as_deref(),
                    Some(sym.line as i32), Some(sym.line_end as i32),
                ).await {
                    Ok(id) => { sym_ids.insert((sym.name.clone(), sym.line as i32), id); }
                    Err(e) => tracing::warn!(kind = %sym.kind, name = %sym.name, file = %result.rel_path, error = %e, "upsert symbol node failed; skipping symbol"),
                }
```

- [ ] **Step 3: Verify it compiles + suite passes**

Run: `cargo test -p senseid --bin senseid`
Expected: PASS (no behavior change in the happy path).

- [ ] **Step 4: Commit**

```bash
git add crates/senseid/src/tasks/handlers/process.rs
git commit -m "fix(scan): log upsert_node failures instead of swallowing with .ok()"
```

---

### Task 6: Zero-errors gate + push

- [ ] **Step 1: Clippy clean**

Run: `cargo clippy -p senseid --bin senseid --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 2: Full bin suite (incl. `sensei_test` integration)**

Run: `cargo test -p senseid --bin senseid`
Expected: all pass (incl. the 3 new tests).

- [ ] **Step 3: Push `develop`**

```bash
git push origin develop
```

---

### Task 7: Production deploy + backfill (option B — needs in-chat authorization)

This rewrites the live `sensei` DB. Get explicit authorization before each live step.

- [ ] **Step 1: Deploy the enum to prod**

Run: confirm `dbd`'s target is `sensei` (prod), then `cd database && dbd deploy`.
Verify: `psql -d sensei -tAc "SELECT enumlabel FROM pg_enum e JOIN pg_type t ON e.enumtypid=t.oid WHERE t.typname='node_kind' AND e.enumlabel IN ('doc','struct','component','hook','extension')"` returns 5 rows.

- [ ] **Step 2: Install the new daemon + restart**

Run: `make install-debug && brew services restart sensei`

- [ ] **Step 3: Capture the before-counts**

Run: `psql -d sensei -tAc "SELECT kind, count(*) FROM sensei.nodes GROUP BY kind ORDER BY 2 DESC"`

- [ ] **Step 4: Clear scan_state + rescan the watched roots**

Run: `psql -d sensei -c "DELETE FROM sensei.scan_state"` then, for each registered root, `sensei scan <abs_root_path>` (roots: `psql -d sensei -tAc "SELECT path FROM sensei.folders_to_watch WHERE status='watching'"`).

- [ ] **Step 5: Verify the backfill**

Run:
```bash
psql -d sensei -tAc "SELECT kind, count(*) FROM sensei.nodes WHERE kind IN ('doc','struct','component','hook','extension') GROUP BY kind ORDER BY 1"
psql -d sensei -tAc "SELECT kind, count(*) FROM sensei.edges WHERE kind IN ('covers','references') GROUP BY kind ORDER BY 1"
```
Expected: `doc`/`struct`/`component`/`hook` > 0 and `covers`/`references` > 0.

- [ ] **Step 6: Mark the backlog item done**

Update `docs/backlog.md §2` (node_kind row) to ✅ and remove from the open list per the backlog convention; commit.

---

## Self-Review

**Spec coverage:** enum additions (T2) ✓; first-class kinds not remapped (T2) ✓; append-for-ADD-VALUE + dbd verify (T2/T3) ✓; silent-swallow logging (T5) ✓; embedding allowlist (T4) ✓; remove dead variants (T2) ✓; guard test (T1) ✓; integration test (T3) ✓; production backfill option B (T7) ✓. Out-of-scope items (enum_variant, IR, calls adapters, library identity, #31, the broader silent-error audit) correctly excluded.

**Placeholder scan:** the only non-literal commands are `dbd` target selection (T3.4, T7.1) — intentional, since `dbd`'s target flag must be confirmed via `--help` at execution (guarded by an explicit verify query after each deploy). No TBDs.

**Type consistency:** `NodeKind::all()` defined T1, edited T2; `upsert_node(folder_id, kind: &str, name, file_path, parent, signature, line_start, line_end)` signature used consistently; `count_nodes_by_kind` → `HashMap<String,i64>` (`.get(kind) == Some(&1)`); `nodes_without_embeddings` → `Vec<(Uuid,String,String,Option<String>,String)>` (tuple destructure matches). Consistent.
