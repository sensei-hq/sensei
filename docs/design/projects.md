---
type: design
---

# Projects — module

Behind-the-scenes design for the [Project](../features/04-project.md) window and
the project entity it renders. The feature doc says what the user sees and does
in a project; this says how the model, the graph, and the window mechanics work.

## The folder → project model

- DDL: `database/ddl/table/sensei/folders_to_watch.ddl` (config: user-chosen
  watch roots, survives a data wipe), `folders.ddl` (content: discovered
  filesystem tree, fully re-derivable by rescan), `projects.ddl` (independent
  grouping entity, one project per git/subtree folder by default).
- `folders.kind` (`folder_kind` enum): `git` / `workspace_member` / `subtree` /
  `sibling` / `standalone` / `folder`. Only `git`/`subtree`/`workspace_member`
  roots are repos and get a 1:1 `projects` row; `kind='folder'` rows are
  structural subfolders — members with a `folder_role` (`backend`/`frontend`/
  `library`/`tool`/`docs`/`infra`/…), owning **no** code nodes.
- **One-owner invariant:** every file belongs to exactly one folder (the
  repo/git-root). Enforced at scan-classification and by a self-healing
  reconcile — `PgStore::dedup_structural_folder_nodes`
  (`crates/senseid/src/db/pg_store.rs:5937`); re-parenting logic at
  `pg_store.rs:5864` ("folder row re-classified `kind='folder'`, re-parented
  under…"). Full rationale: [data layer](../architecture/data.md#one-owner-invariant).
- Root resolution: `PgStore::repo_root_for_path`
  (`crates/senseid/src/db/pg_store.rs:4008`) walks up from a changed file to
  its owning repo root — the join point the watcher uses per file.
- `projects` fields the window renders directly: `maturity` (`project_maturity`
  enum: discovery → active → maintenance → archived), `stack` (derived from
  member folders), `icon`, `links`, `guidelines`, `backlog`, `preferred_acp`,
  `privacy`, `excluded_globs`, `dojo_id` (bind target, nullable — opt-in).

## The code + activity graph

- DDL: `database/ddl/table/sensei/nodes.ddl` + `edges.ddl`; kind enums
  `enum/sensei/node_kind.ddl`, `enum/sensei/edge_kind.ddl`,
  `enum/sensei/edge_confidence.ddl`.
- Rust mirror: `crates/senseid/src/types.rs` — `NodeKind` (`Package`/`Module`/
  `Function`/`Method`/`Class`/`Struct`/`Interface`/`Enum`/`Const`/`Type`/
  `Component`/`Hook`/`File`/`Doc`/`Extension` — 16 kinds spanning code +
  documentation + marketplace-extension hierarchies) and `ParsedEdge`
  (`types.rs:258`).
- `nodes.embedding vector(384)` (HNSW index) gives semantic search as one SQL
  query — no separate vector store (see [data layer](../architecture/data.md)).
  `edges.edge_confidence` is extracted/inferred/ambiguous.
- Extraction: `crates/senseid/src/tasks/handlers/scan.rs` (the scan task) →
  `crates/senseid/src/tasks/processors/code.rs` walks a repo and builds the
  hierarchy; `crates/senseid/src/indexer/cross_repo.rs` links symbols across
  repo boundaries (imports/monorepo members).
- Language-specific parsing: `crates/senseid/src/languages/*` (tree-sitter
  adapters); manifest/command discovery per-folder:
  `crates/senseid/src/adapters/manifest/*` (e.g. `pyproject.rs`).
- Incremental keep-current: `crates/senseid/src/watcher/root_watcher.rs` +
  `scan_state` content-hash/mtime fingerprints (see
  [data layer §metadata model](../architecture/data.md)).
- Atlas (project's code + architecture graph UI) reads this via
  `crates/senseid/src/api/handlers/project_detail.rs` and renders at
  `app/src/routes/(project)/project/[id]/*` (structure · calls · communities —
  see `get_communities`/`get_callers`/`get_callees` MCP tools backing the same
  data).

## Project window as a separate Tauri window

- Opened from the frontend via `openProjectWindow(projectId, projectName)`
  (`app/src/lib/stores/windows.svelte.ts`), which uses the Tauri
  `WebviewWindow` JS API directly (`@tauri-apps/api/webviewWindow`). Idempotent
  — windows are labelled `project-{id}`; a repeat call for the same id focuses
  the existing window (`setFocus`) instead of stacking a duplicate.
- The window loads route `/project/{id}` — the frontend reads `project_id`
  from the URL route param, and `hooks::reroute` resolves it to the project
  overview. The window chrome (overlay titlebar, hidden title, transparent)
  matches the main window.
- Callers: `(observatory)/projects/+page.svelte`,
  `(observatory)/insights/+page.svelte`, and the ⌘K `project.open` commands in
  `(observatory)/+layout.svelte`.
- Frontend routing: `(project)/project/[id]/{overview,sessions,memories,
  patterns,libraries,impact,traceability,about,instruments}` — one route
  group, name-or-UUID resolves everywhere. Shared layout
  `app/src/routes/(project)/+layout.svelte` + `+layout.ts` just calls
  `appState.load()` — routing/redirect is `hooks::reroute`'s job, not the
  layout's.

## Project detail API surface

- `crates/senseid/src/api/handlers/project_detail.rs` — per-project HTTP
  handlers under `/api/projects/{id}/*`. Accepts either a project **name** or
  **UUID** in the path (resolved via `get_project_by_name` then UUID parse
  fallback) — the same dual-resolution pattern the MCP `get_commands` tool
  uses server-side.
- `get_project_commands` returns discoverable commands (populated by
  `ManifestAdapter::parse_commands` during `extract_deps`, refreshed
  atomically per folder on rescan); `set_command_preference` writes a
  user-scope capability → preferred-tool bias (`PUT /api/preferences/commands`,
  G10) that ranks the preferred command first on read.
- `crates/senseid/src/project_overview.rs` computes the Overview "one thing to
  act on now" surface; per-section reads (sessions/memories/patterns/impact)
  fan out to the matching `activity`/`inference` schema tables scoped by
  `project_id` (see [data layer](../architecture/data.md)).

## Bind to a dōjō (opt-in, future-facing on the write side)

- `projects.dojo_id` is the bind pointer; `crates/senseid/src/dojo/
  attribution.rs` + `relay_project.rs` carry the org-attribution and
  relay-scoping logic once bound.
- Shipped: local read/UI wiring for identity/stack/repos/links/guidelines.
  Auto-discover (classify personal vs org-owned remote → suggest a dōjō) is
  **not built** — see [setup-and-config.md](setup-and-config.md#dōjō-auto-discover-not-built).

## Where the gaps are

- Tasks/plan (backlog → phases) has no planner yet — `projects.backlog` is a
  flat jsonb list today, not a phased plan.
- Atlas and Traceability are partial: the graph read-paths exist
  (`project_detail.rs`, `nodes`/`edges`), but some UI panes are scaffolded
  ahead of full data wiring (see [04-project.md status table](../features/04-project.md#status)).
</content>
