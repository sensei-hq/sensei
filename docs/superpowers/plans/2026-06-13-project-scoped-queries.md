# Project-Scoped Code-Graph Queries (#60 Part A) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]`.

**Goal:** Make the code-graph query tools (`search`, `get_project_summary`, `get_callers`, `get_callees`, code-graph nodes/edges) return results scoped to **all folders in a project**, and fix the MCP layer so it passes a working project identifier. Implements the user's directive for issue #60: *individual folders are children of the parent project, and a search scoped to a project includes its child folders.*

**Architecture:** Two layers are broken. (A2, daemon) Query handlers resolve a project/folder name to **one** folder (`get_repo_by_name`) and filter `WHERE folder_id = <that folder>` — they never include child folders, so a project whose code lives in sub-folders returns ~nothing. (A1, MCP) `mcp/main.rs::resolve_project` reads `p["repo_id"]`/`p["path"]`/`p["libs"]` from `/api/projects`, but that endpoint (`observatory::list_solutions` → `pg.list_projects()` enriched with a `folders[]` array) returns projects-table rows `{id, name, description, …, folders:[…]}` with **no** `repo_id`/`path`/`libs`. So `resolve_project` emits an empty string → daemon gets `repoId=""` → empty results for every project.

**Tech Stack:** Rust (`senseid` bin crate + `sensei-mcp` bin crate), sqlx/Postgres (`sensei_test` for DB tests).

**Scope:** Read side only (Part A). Write-side rollup (B) and data cleanup (C) are separate tasks. No DDL changes — `folders` already has `project_id` and `parent_id`; `call_graph` view already exposes `folder_id`, `project`, `project_id`; `list_folders_by_project(project_id)` already exists (pg_store:465).

---

## Design: one resolver, folder-set scoping

Add a single resolver in `pg_store` that turns the identifier the tools already pass into the **set of folder ids** to query:

```rust
/// Resolve a scope identifier (project name, project UUID, or folder name) to the
/// set of folder ids to query. A project expands to ALL its folders (children
/// included); a bare folder name with a project expands to that project's folders;
/// a folder with no project falls back to just itself. Empty Vec if nothing matches.
pub async fn scope_folder_ids(&self, ident: &str) -> Result<Vec<uuid::Uuid>, String>
```

Resolution order: `get_project_by_name(ident)` → `list_folders_by_project(pid)`; else `Uuid::parse_str(ident)` + `get_project(id)` → its folders; else `get_repo_by_name(ident)` → if it has `project_id`, that project's folders, else `[folder.id]`.

All read queries scope to that set via `folder_id = ANY($1)`. Single-folder fns delegate to the scoped fns (DRY): `search_functions(fid) = search_functions_scoped(&[*fid], q)`. The `call_graph` queries switch from `WHERE folder = $name` to `WHERE folder_id = ANY($1)` (the view exposes `folder_id`).

Handlers resolve `scope_folder_ids(repo_id)` once, then call the `_scoped` fns. Internal callers in `process.rs`/`resolve.rs`/`indexer` keep using the single-folder fns unchanged (they are legitimately folder-scoped).

---

## Task A2-1: `scope_folder_ids` resolver (pg_store)

**Files:** `crates/senseid/src/db/pg_store.rs` (new fn + test in the `#[cfg(test)] mod tests`)

- [ ] **Step 1 — failing DB test.** Add to the pg_store tests module (uses `connect_test`, `add_watch_root`, `create_project`, `upsert_repo`/`upsert_node`, `set_folder_project`):

```rust
    #[tokio::test]
    async fn scope_folder_ids_expands_project_to_all_child_folders() {
        let s = PgStore::connect_test().await.unwrap();
        let root = s.add_watch_root("/tmp/scope-test", "scope", &serde_json::json!([])).await.unwrap();
        let pid = s.create_project("ScopeProj", None, None).await.unwrap();
        // root folder + child folder, both tagged with the project
        let rootf = s.upsert_repo(&root, "ScopeProj", "/tmp/scope-test").await.unwrap();
        s.set_folder_project(&rootf, &pid, "primary", None).await.unwrap();
        let childf = s.upsert_subfolder(&root, "folder", "child", "child", "/tmp/scope-test/child", Some(&rootf), Some(&pid)).await.unwrap();

        let ids = s.scope_folder_ids("ScopeProj").await.unwrap();
        assert!(ids.contains(&rootf) && ids.contains(&childf), "project expands to root + child, got {:?}", ids);
        // by uuid string
        let by_id = s.scope_folder_ids(&pid.to_string()).await.unwrap();
        assert!(by_id.contains(&childf), "uuid identifier resolves to project folders");
        // unknown → empty
        assert!(s.scope_folder_ids("nope-xyz").await.unwrap().is_empty());
    }
```
(Confirm the exact signature of `upsert_subfolder` at pg_store.rs:925 and `create_project` before writing; adjust arg lists to match.)

- [ ] **Step 2 — run, expect fail** (`scope_folder_ids` undefined): `cargo test -p senseid --bin senseid scope_folder_ids_expands -- --nocapture 2>&1 | tail -20`

- [ ] **Step 3 — implement `scope_folder_ids`** per the design above, reusing `get_project_by_name`, `get_project`, `list_folders_by_project`, `get_repo_by_name`. Return `Vec<uuid::Uuid>`.

- [ ] **Step 4 — run, expect pass.**

- [ ] **Step 5 — commit** (`crates/senseid/src/db/pg_store.rs`): `feat(senseid): scope_folder_ids resolver — project → all child folders (#60)`

## Task A2-2: project-scoped query variants (pg_store)

**Files:** `crates/senseid/src/db/pg_store.rs`

- [ ] **Step 1 — failing DB test** asserting a function node in a CHILD folder is found when scoping by the project, and `get_callers`/`get_callees` include child-folder edges. Build: project P, root folder + child folder (both project_id=P), a function node `widget_fn` + a `calls` edge in the CHILD folder; assert:
  - `search_functions_scoped(&scope_folder_ids("P"), "widget")` finds it,
  - `count_nodes_by_kind_scoped(&ids)` counts the child's function,
  - `get_callers_by_name("P", <target>)` / `get_callees_by_name("P", <caller>)` return the child-folder edge.

- [ ] **Step 2 — run, expect fail.**

- [ ] **Step 3 — implement.** Add `*_scoped(folder_ids: &[uuid::Uuid], …)` variants for `search_functions`, `search_types`, `count_nodes_by_kind`, `get_nodes_by_folder` (→ `get_nodes_scoped`), `get_edges_by_kind` (→ `get_edges_scoped`) using `WHERE folder_id = ANY($1)`. Make the existing single-folder fns delegate: `self.x_scoped(std::slice::from_ref(folder_id), …)`. Change `get_callers_by_name`/`get_callees_by_name` to accept the scope identifier, internally `scope_folder_ids(ident)` → filter `call_graph WHERE folder_id = ANY($1) AND … edge_kind='calls'` (keep `get_callees` `target_name.or(unresolved_target)` display logic).

- [ ] **Step 4 — run, expect pass; run full pg_store suite for no regressions.**

- [ ] **Step 5 — commit.**

## Task A2-3: handlers use project scope

**Files:** `crates/senseid/src/api/handlers/codebase.rs`, `observatory.rs`, `query.rs`, `mcp.rs`

- [ ] **Step 1 — failing integration test** (in `codebase.rs` or `observatory.rs` tests): process a 2-folder project (root + child with a function/edge), hit `search_functions`/`project_summary`/`fn_callers` with the project name, assert child-folder results are included. (Mirror the make_ctx harness used in process.rs/resolve.rs tests.)

- [ ] **Step 2 — run, expect fail** (handlers still single-folder).

- [ ] **Step 3 — implement.** In `search_functions`/`search_types`/`code_graph`(nodes/edges)/`project_summary`: replace `get_repo_by_name → folder_id` with `scope_folder_ids(repo_id) → Vec<Uuid>` then call the `_scoped` fns (sum `count_nodes_by_kind_scoped` for the summary; aggregate `count_edges` across the set or add `count_edges_scoped`). `fn_callers`/`fn_callees`/`query.rs`/`mcp.rs` already pass the identifier to `get_callers_by_name`/`get_callees_by_name`, which now scope internally — verify they compile and return project-wide results. `project_summary` should report the resolved project's name/path (look it up via `get_project_by_name`) rather than a single folder row.

- [ ] **Step 4 — run, expect pass; `cargo test -p senseid --bin senseid` full suite green; clippy clean.**

- [ ] **Step 5 — commit.**

## Task A1: MCP resolver passes a real identifier

**Files:** `crates/mcp/src/main.rs`

- [ ] **Step 1 — failing unit test** for `resolve_project` against a representative `/api/projects` payload (projects-table shape + `folders[]`), e.g.:

```rust
    #[test]
    fn resolve_project_matches_by_name_and_returns_usable_id() {
        let projects = vec![serde_json::json!({
            "id": "11111111-1111-1111-1111-111111111111",
            "name": "sensei",
            "folders": [{"abs_path": "/Users/x/dev/sensei", "name": "sensei"}]
        })];
        // exact + case-insensitive name match returns the project name (or id), NOT ""
        assert_eq!(resolve_project_in(&projects, "sensei"), Some("sensei".to_string()));
        assert_eq!(resolve_project_in(&projects, "SENSEI"), Some("sensei".to_string()));
        assert_eq!(resolve_project_in(&projects, "nope"), None);
    }

    #[test]
    fn resolve_from_cwd_matches_folder_abs_path_prefix() {
        let projects = vec![serde_json::json!({
            "id": "1", "name": "sensei",
            "folders": [{"abs_path": "/Users/x/dev/sensei", "name": "sensei"}]
        })];
        assert_eq!(resolve_from_cwd_in(&projects, "/Users/x/dev/sensei/crates/senseid"), "sensei".to_string());
    }
```
(Refactor the pure matching logic into `resolve_project_in(projects: &[Value], hint)` / `resolve_from_cwd_in(projects, cwd)` so they're unit-testable without HTTP; the existing `resolve_project`/`resolve_project_from_cwd` call them with a live `get_projects(client)`.)

- [ ] **Step 2 — run, expect fail.**

- [ ] **Step 3 — implement.** Match by `p["name"]` (exact, case-insensitive, then partial) and `p["id"]`; for cwd, match against each project's `folders[].abs_path` (longest prefix of cwd wins). Return the project **name** (the daemon's `scope_folder_ids` resolves a name). Drop the dead `repo_id`/`path`/`libs` field reads. Keep returning `None` on no match so the tool emits a clear "project not found" error.

- [ ] **Step 4 — run, expect pass; clippy clean.**

- [ ] **Step 5 — commit.**

## Verification (live, after build+install+restart — gated on user per daemon-restart policy)

- `get_project_summary(project="src")` and `get_callers(insert_memory, project="src")` return data (project `src` already has child folders db/federation/src sharing one project_id — proves traversal on current data).
- After Part C (sensei project owns the code): the same with `project="sensei"`, run from inside `…/sensei-hq/sensei`.

## Self-review
- Spec coverage: directive part 1 (children) → write-side Part B (separate); directive part 2 (search includes children) → A2 traversal ✓. MCP empty-repoId root cause → A1 ✓.
- DRY: single-folder fns delegate to `_scoped`; one resolver reused by all handlers.
- No DDL; reuses `list_folders_by_project`, `call_graph.folder_id`.
