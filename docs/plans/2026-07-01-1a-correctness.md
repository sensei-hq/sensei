---
name: 2026-07-01 — 1a Correctness sprint
issue: https://github.com/sensei-hq/sensei/issues/92
epic: https://github.com/sensei-hq/sensei/issues/83
analysis: docs/analysis/2026-07-01-project-window-instruments-depmap-gap-analysis.md
---

# 1a — Correctness sprint

Fixes folder→library dep extraction so `sensei`, `rokkit`, and `dbd` produce accurate `referenced_libraries` + `project_libraries` rows, and surfaces the two new signals — project→project local-dep edges and per-library version conflicts — needed by Track 3 Libraries screen.

**No new adapter traits** (1b owns that). **No command surface** (1c owns that). **No Dōjō preferences** (deferred entirely). This chunk is surgical.

## Deliverables

| # | Deliverable | Files |
|---|---|---|
| 1 | Preserve full `@scope/name` for scoped imports | `crates/senseid/src/tasks/handlers/libraries.rs:68-74` |
| 2 | Parse `link:` / `path=` / `workspace:` as local (do NOT count as external) + tag in props | `crates/senseid/src/indexer/lib_indexer.rs:36-107` |
| 3 | New `sensei.project_dependencies` DDL table | `database/ddl/table/sensei/project_dependencies.ddl` (new) |
| 4 | `upsert_project_dependency` writer + call from `extract_deps` when a local-dep resolves to a sibling project | `crates/senseid/src/db/pg_store.rs`, `crates/senseid/src/tasks/handlers/libraries.rs:249-334` |
| 5 | `GET /api/projects/{id}/project-deps` endpoint | `crates/senseid/src/api/handlers/project_detail.rs`, `.../routes.rs` |
| 6 | Version-inconsistency **view** | `database/ddl/view/sensei/project_library_version_conflicts.ddl` (new) |
| 7 | `GET /api/projects/{id}/library-version-conflicts` endpoint | `crates/senseid/src/api/handlers/project_detail.rs`, `.../routes.rs` |
| 8 | Extend `upsert_referenced_library` to accept optional `props` (JSON) | `crates/senseid/src/db/pg_store.rs:1914` |

## Order of work (smallest reviewable diff first)

Each step is committed separately on the `1a-correctness` branch.

### Step 1 — scoped-name preservation
- Change `libraries.rs:68-74`:
  ```rust
  // BEFORE
  let lib_name = if path.starts_with('@') {
      path.split('/').next().unwrap_or("").trim_start_matches('@').to_string()
  } ...
  // AFTER — keep the "@scope/name" pair (two segments), drop deeper paths
  let lib_name = if path.starts_with('@') {
      let mut parts = path.splitn(3, '/');
      match (parts.next(), parts.next()) {
          (Some(scope), Some(name)) => format!("{scope}/{name}"),
          (Some(only), None) => only.to_string(),
          _ => String::new(),
      }
  } ...
  ```
- **Unit tests (added to `libraries.rs` mod tests):**
  - `@rokkit/core` → `@rokkit/core`
  - `@rokkit/actions/foo/bar` → `@rokkit/actions`
  - `@scope` (no `/`) → `@scope`
  - `svelte/store` → `svelte`
  - `crate::foo` skipped (untouched)
- Zero-errors gate.

### Step 2 — local-protocol detection in manifest parser
- In `lib_indexer.rs:36-107`, add three helpers:
  - `is_local_npm_ver(s: &str) -> bool` → `s.starts_with("link:") || s.starts_with("workspace:") || s.starts_with("file:")`
  - `is_local_cargo_dep(table: &toml::Value) -> Option<&str>` → if the table has `path` key, return the path string; otherwise `None`.
- Extend `DepVersion` with `local_source: Option<String>` (path/link target string, `None` for external). Serde-serialize.
- In each ecosystem branch:
  - `package.json`: if `is_local_npm_ver(&version)`, still push a `DepVersion` but set `local_source = Some(target_after_prefix)`.
  - `Cargo.toml`: for a table value, check `is_local_cargo_dep`; if a path is present, set `local_source`.
  - `pyproject.toml`: `path` deps in `[tool.uv.sources]` / `[tool.poetry.dependencies]` variants — handle later if trivial, else defer.
- **Unit tests:** fixtures under `crates/senseid/src/indexer/tests/fixtures/1a-correctness/` with representative manifests. Assert `DepVersion.local_source` is `Some(...)` for `link:../foo` / `path = "../sibling"` and `None` for `"^1.2.3"`.
- Zero-errors gate.

### Step 3 — extend `upsert_referenced_library` with `props`
- Change signature to accept `props: Option<serde_json::Value>`. All existing callers pass `None`. New call from step 4 passes `Some(json!({ "local_source": "..." }))`.
- SQL: `INSERT INTO sensei.referenced_libraries(folder_id, library_id, version_used, props) VALUES ($1,$2,$3, COALESCE($4, '{}'::jsonb)) ON CONFLICT ... DO UPDATE SET props = referenced_libraries.props || EXCLUDED.props, ...`.
- **Integration test:** `test_upsert_referenced_library_merges_props` — insert with `{"local_source":"…"}`, reinsert with `{"pinned":true}`, expect merged `{"local_source":"…","pinned":true}`.
- Zero-errors gate.

### Step 4 — new DDL: `project_dependencies` table
- New file `database/ddl/table/sensei/project_dependencies.ddl`:
  ```sql
  set search_path to sensei, extensions;

  create table if not exists sensei.project_dependencies (
    from_project_id  uuid          not null references sensei.projects(id) on delete cascade
  , to_project_id    uuid          not null references sensei.projects(id) on delete cascade
  , from_folder_id   uuid          not null references sensei.folders(id)  on delete cascade
  , source_protocol  text          not null   -- 'link' | 'workspace' | 'path'
  , source_manifest  text          not null   -- 'package.json' | 'Cargo.toml' | 'pyproject.toml'
  , resolved_target  text                     -- the raw local-source string
  , modified_at      timestamptz   not null default now()
  , primary key (from_project_id, to_project_id, from_folder_id, source_manifest)
  , check (from_project_id <> to_project_id)
  );

  create index if not exists project_dependencies_from_idx
    on sensei.project_dependencies(from_project_id);
  create index if not exists project_dependencies_to_idx
    on sensei.project_dependencies(to_project_id);

  comment on table sensei.project_dependencies is
  'First-class project→project edges detected via local-path/link:/workspace:/path= protocols.
  from_project_id references to_project_id via a local sibling folder (from_folder_id).
  source_protocol distinguishes link/workspace/path so the UI can badge the origin.';
  ```
- Apply via `dbd deploy` after `make bump` (memory rule: dbd cache reversion — `make bump` runs `make dbd-cache-clear`).
- **Integration test:** manual `dbd deploy` then a smoke test that inserts + selects.
- Zero-errors gate.

### Step 5 — `upsert_project_dependency` writer + integration into `extract_deps`
- New `PgStore::upsert_project_dependency(from_project_id, to_project_id, from_folder_id, source_protocol, source_manifest, resolved_target)`.
- In `extract_deps`:
  - When a `DepVersion` has `local_source: Some(target)`, resolve `target` (relative to `folder.abs_path`) to a filesystem path.
  - Look up `sensei.folders WHERE abs_path = <resolved>` — if it exists AND belongs to a different `project_id`, upsert `project_dependencies`.
  - When `local_source` is `Some` but the target doesn't resolve to a known folder, log at INFO and skip (not an error — could point outside indexed roots).
  - **CRITICAL:** deps with `local_source = Some(...)` are NOT written to `referenced_libraries` as external libraries. They only feed `project_dependencies`.
- **Unit tests:** fake folders + projects; assert both paths (resolves → edge written; doesn't resolve → INFO log, no edge, no external lib row).
- **Integration test:** end-to-end with a rokkit-shaped fixture (packages/ui with `"@rokkit/actions": "link:../actions"`) — assert `project_dependencies` gets a row and `referenced_libraries` does NOT.
- Zero-errors gate.

### Step 6 — API endpoint: `GET /api/projects/{id}/project-deps`
- Handler in `project_detail.rs`: returns `[{ to_project_id, to_project_name, source_protocol, source_manifest, resolved_target, from_folder }]`.
- Route in `api/routes.rs`.
- Contract test: hits real Postgres via `PgStore`, seeds two projects + a `project_dependencies` row, calls the handler, asserts payload shape.
- Zero-errors gate.

### Step 7 — new DDL view: `project_library_version_conflicts`
- New file `database/ddl/view/sensei/project_library_version_conflicts.ddl`:
  ```sql
  set search_path to sensei, extensions;

  create or replace view sensei.project_library_version_conflicts as
  with per_folder as (
    select f.project_id
         , rl.library_id
         , rl.version_used
         , f.id   as folder_id
         , f.name as folder_name
      from sensei.referenced_libraries rl
      join sensei.folders f on f.id = rl.folder_id
     where f.project_id is not null
       and rl.version_used is not null
       and rl.version_used <> ''
       and (rl.props ? 'local_source') = false
  ),
  conflicts as (
    select project_id, library_id
      from per_folder
     group by project_id, library_id
    having count(distinct version_used) > 1
  )
  select c.project_id
       , c.library_id
       , l.name           as library_name
       , array_agg(distinct pf.version_used order by pf.version_used) as versions
       , array_agg(distinct pf.folder_name  order by pf.folder_name)  as folders
    from conflicts c
    join per_folder pf on pf.project_id = c.project_id and pf.library_id = c.library_id
    join sensei.libraries l on l.id = c.library_id
   group by c.project_id, c.library_id, l.name;

  comment on view sensei.project_library_version_conflicts is
  'Per-project libraries pinned to different versions across folders.
  Excludes local-protocol deps (link:/workspace:/path=) so only real registry-version conflicts surface.
  Powers a "version drift" signal on the Libraries screen (Track 3).';
  ```
- Apply via `dbd deploy`.
- **Integration test:** insert two `referenced_libraries` rows for the same `(project_id, library_id)` with different `version_used`; assert the view emits a row with both versions.
- Zero-errors gate.

### Step 8 — API endpoint: `GET /api/projects/{id}/library-version-conflicts`
- Handler + route + contract test as in Step 6.
- Zero-errors gate.

## DDL rules (project memory)

- Edit `.ddl` FIRST, then `dbd deploy` (`feedback_ddl_source_first`).
- `dbd combine` is NOT the source of truth (`feedback_dbd_deploy_not_combine`).
- `make bump` clears the dbd cache before deploy (`project_dbd_cache_reversion`).

Since 1a introduces two new objects (one table + one view) without altering existing ones, we do NOT bump the released version — the daemon reads local DDL when `SENSEI_DDL_DIR` is set. For install-debug verification below we set `SENSEI_DDL_DIR=$(pwd)/database` (documented in `sensei/CLAUDE.md`).

## Verification against sensei / rokkit / dbd

After all steps land on `1a-correctness` branch and `develop` is merged:

1. `SENSEI_DDL_DIR=/Users/Jerry/Developer/sensei-hq/sensei/database make install-debug`
2. Trigger a rescan of `~/Developer` via the daemon API.
3. Wait for the queue to drain.
4. Run these SQL assertions (in Postgres, `sensei` DB):

```sql
-- (a) Scoped names preserved
SELECT name FROM sensei.libraries
 WHERE name LIKE '@rokkit/%';
--    Expect: at least @rokkit/actions, @rokkit/core, @rokkit/states, @rokkit/ui, @rokkit/utils

-- (b) No scope-truncated 'rokkit' library (residual bug)
SELECT count(*) FROM sensei.libraries WHERE name = 'rokkit';
--    Expect: 0

-- (c) rokkit workspace-internal link edge captured
SELECT from_p.name AS from_project
     , to_p.name   AS to_project
     , pd.source_protocol
     , pd.source_manifest
  FROM sensei.project_dependencies pd
  JOIN sensei.projects from_p ON from_p.id = pd.from_project_id
  JOIN sensei.projects to_p   ON to_p.id   = pd.to_project_id
 ORDER BY from_project, to_project;
--    Expect: at least one edge where rokkit's packages/ui → packages/actions
--    (via "link:../actions" in packages/ui/package.json)

-- (d) sensei cargo path= workspace deps NOT counted as external
SELECT count(*)
  FROM sensei.libraries l
  JOIN sensei.referenced_libraries rl ON rl.library_id = l.id
  JOIN sensei.folders f ON f.id = rl.folder_id
 WHERE l.ecosystem = 'cargo'
   AND l.name IN ('senseid','sensei-cli','sensei-mcp','bootstrap','gateway','gateway-embedded');
--    Expect: 0 (these are internal workspace crates referenced via path=)

-- (e) Version-inconsistency view emits at least one real conflict
SELECT project_id, library_name, versions, folders
  FROM sensei.project_library_version_conflicts
 LIMIT 10;
--    Inspect manually; a rokkit or sensei monorepo is likely to have at least one
--    library pinned differently across its folders.
```

5. Hit the two new HTTP endpoints against the running daemon:
   ```bash
   curl -s http://127.0.0.1:7744/api/projects/<rokkit-project-id>/project-deps | jq .
   curl -s http://127.0.0.1:7744/api/projects/<sensei-project-id>/library-version-conflicts | jq .
   ```

## Test coverage summary

- **Unit:** name preservation (5 cases), local-protocol detection (npm + cargo, positive + negative), `resolve_local_target` path resolution.
- **Integration (`test_db_url`):** props merge on upsert, `upsert_project_dependency` idempotency, both new views/tables selectable, both new endpoints return expected shape.
- **End-to-end fixture:** synthetic rokkit-shaped folder tree drives `extract_deps` and asserts `project_dependencies` gets a row while `referenced_libraries` skips it.

## Sequencing / exit criteria

Merge sub-chunk to `develop` when:
- All unit + integration tests green (`make test`).
- `svelte-check` unaffected (backend-only chunk).
- `SENSEI_DDL_DIR` install-debug + reindex passes the five SQL assertions above.
- Zero-errors-policy checklist clean.

Then `develop → main` per user's rule "when a logical feature is complete, merge into main and push".

## Filed follow-ups (from this chunk's work)

Any surprises during 1a get their own GH issue rather than expanding scope.
