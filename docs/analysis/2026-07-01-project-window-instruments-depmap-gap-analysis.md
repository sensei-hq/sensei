---
name: 2026-07-01 — Project Window · Instruments · Dep-Map Gap Analysis
date: 2026-07-01
status: analysis (no code)
tracks: [dep-map + adapter cleanup, instruments UI, project window]
---

# Project Window · Instruments · Dependency Map — Gap Analysis

Analysis-only. No implementation. Each track that survives becomes its own brainstorm → spec → plan → build cycle.

## Purpose

Three coupled asks:

1. **Dependency map** between projects and libraries — starting from folder-level correctness (rokkit, dbd, and any project's real manifest deps must resolve), rolling up to project-level, and only reaching for project→project edges when a `link:` / `workspace:` / `path=` protocol names a sibling. **In scope now: extend this same effort to a general command surface (npm scripts, Makefile targets, Cargo aliases, tox.ini, pyproject scripts, uv scripts, go run targets) so assistants can call `get_commands(project|folder, action?)` instead of grep/sed guessing.**
2. **Instruments** — the observatory MCP inspector rebuilt against the mockups: modular components, separate store for MCP connection, daemon-fed, rokkit primitives where they fit, variant-driven cards otherwise.
3. **Project window** — a per-project inspection window (separate Tauri window) built page-by-page against the existing mockups, starting with high-signal projects (sensei, rokkit, dbd).

Two cross-cutting themes surfaced during analysis and are treated as first-class here:

- **Adapter cleanup.** Language parsing already routes through `LanguageAdapter` (`crates/senseid/src/languages/mod.rs:17-22`). Manifest parsing, command surface, config/metadata extraction, and file classification are **not** abstracted — they are inline `match ext` / hardcoded-filename spaghetti scattered across ten call sites. Introducing sibling adapter traits removes the class of bug that produced #30, #62, #63.
- **Governance overlay (Dōjō).** Preferences (dbd vs sqlalchemy; playwright vs cypress) and policies (security/compliance violations) attach to capabilities, not to specific tools. Adapters need to be capability-aware so the same request (`db-schema:apply`) resolves to the user's or org's preferred tool. Skills / agents run policy checks in the development cycle.

## Deliverable of this document

A gap map. No design commitment. Sequencing recommendation at the end. User picks which track to brainstorm first.

---

## Track 1 — Dependency map, command surface, adapter cleanup

### 1.1 What exists today

**Schema (working).**

- `sensei.libraries` — the library catalog (`libraries.ddl`).
- `sensei.referenced_libraries(folder_id, library_id, version_used, props)` — folder-grained dep edge (`referenced_libraries.ddl`).
- `sensei.project_libraries(library_id, project_id, enabled, props)` — project-level rollup with a global (`project_id NULL`) variant (`project_libraries.ddl`).

**Extractor (partly working, structurally inline).**

- `extract_dep_versions()` at `crates/senseid/src/indexer/lib_indexer.rs:36-107` — a single function with cascading `if let Ok(...)` blocks for `package.json`, `Cargo.toml`, `pyproject.toml`. Go's `go.mod` is not parsed at all.
- `ExtractDeps` task at `crates/senseid/src/tasks/handlers/libraries.rs:100+` — orchestrates the extraction, writes `libraries` + `referenced_libraries`, and (since #30 fix) rolls up to `project_libraries`.
- Storage helpers: `pg_store.rs:1847` (aggregate), `:1918` (upsert), `:4562` (rollup).

**Adapter surface (asymmetric).**

- `LanguageAdapter` trait at `crates/senseid/src/languages/mod.rs:17-22` — dispatch via `adapter_for_ext(ext)` at `:25-40`.
- **No sibling** trait for manifests, commands, or config. Every manifest concern is inline conditionals.

### 1.2 Folder→library coverage matrix

| Ecosystem | External | `@scoped/*` | Workspace-internal | `link:` / `path=` / `workspace:` |
|---|:-:|:-:|:-:|:-:|
| **npm** | ✅ string versions | ⚠️ scope dropped — stored as `rokkit`, not `@rokkit/core` (`libraries.rs:68-74` name extraction) | ⚠️ workspace members detected (#63 fix, `libraries.rs:309-327`); their deps aren't linked back | ❌ `link:` unparsed |
| **cargo** | ✅ string + table syntax | n/a | ⚠️ workspace members detected via `[workspace] members` glob | ❌ `path = "../sibling"` treated as external if a version is present |
| **pypi** | ✅ `[project]` PEP 508 only | n/a | ❌ no workspace detection (poetry / uv untouched) | ❌ |
| **go** | ❌ go.mod not parsed at all | n/a | detect-only via `go.work` (`detector.rs:141-165`); deps not extracted | ❌ |

Version-inconsistency detection: **absent**. `version_used` is stored per (folder, library) but never compared across folders of the same project. `list_libraries_with_usage()` at `pg_store.rs:1841-1878` counts refs and lists folder names — no conflict computation, no view, no task.

### 1.3 Known-bug traces (verify + close residual gap)

- **#30 rokkit scope drop** — rollup fix shipped (`51851217`). **Residual:** `libraries.rs:68-74` name extraction still drops the `@rokkit/` prefix. Test `pg_store.rs:4561-4575` passes because the fixture stores the full scoped name; production extraction truncates. **Fix:** preserve the full `@scope/name` in `resolve_libs`.
- **#63 partial detection** — shipped (`33e52402`): removed `min_repos >= 2` filter; workspace members now registered as first-party libs when `public`; cargo ecosystem enum corrected. Residual same as #30.
- **#62 multi-repo misclass** — shipped (`a69ff642`): `classify_folders()` at `scan_logic.rs:116-162` guards against promoting subdirectories of an already-discovered project root.

### 1.4 Command surface (new scope, no mockup yet)

**Motivation.** Assistants guess build/test/e2e/lint/release invocations from convention, or chain `sed`/`grep` through Makefiles. Grounding this in the manifest is cheaper and correct. See `feedback_no_command_guessing` memory. The user has explicitly seen wrong scripts run because the manifest wasn't read.

**Surface shape.** Add two MCP tools once the daemon can answer them:

- `list_actions(project|folder)` → `[{ verb: "test:e2e", invocation: "make test-e2e", source: "Makefile", confidence, capability: "e2e-runner", tool: "playwright" }, ...]`
- `get_commands(project|folder, action?)` → resolved command(s) for a canonical verb, honoring the Dōjō preference overlay.

**Canonical verbs (proposed).** `build`, `dev`, `test:unit`, `test:e2e`, `test:ci`, `lint`, `format`, `release`, `patch`, `deploy`, `db:apply`, `db:reset`. Extendable via the Dōjō vocabulary.

**Sources.**

- npm `scripts` (root + each workspace child)
- `Makefile` targets (no code opens Makefiles today — **greenfield**)
- Cargo `[[bin]]`, aliases, `[workspace.metadata.…]`
- `pyproject.toml` `[project.scripts]` + poetry scripts + uv scripts
- `tox.ini`, `noxfile.py` (detection only, not execution)
- `deno.jsonc` tasks, `justfile`, `mise` tasks, `Taskfile.yml`

**Classification.** Name-heuristic first (regex over the script name), then adapter defaults, then Dōjō override. Confidence is exposed so the UI can badge low-confidence guesses.

### 1.5 Spaghetti audit — other inline dispatch that must move to adapters

The manifest handling is the biggest offender but not the only one. Audit surfaced ten sites; grouped into four proposed adapter surfaces:

| # | Site | Filenames / extensions hardcoded | Belongs on |
|---|---|---|---|
| 1 | `tasks/processors/router.rs:40-60` | md, mdx, txt, json, toml, yaml, yml, jsonl | **ConfigAdapter** |
| 2 | `api/handlers/codebase.rs:336-360` | rs, ts/mts/cts, tsx, js/mjs/cjs, jsx, py, go, rb, java, kt/kts, svelte, vue, sql, sh, md, toml, yaml, json, css, html | **LanguageAdapter.display_name()** extension |
| 3 | `tasks/handlers/libraries.rs:271-276, 345-349` | package.json→npm, Cargo.toml→cargo, pyproject.toml→pypi | **ManifestAdapter.ecosystem()** |
| 4 | `tasks/handlers/scan_logic.rs:270-337` | Cargo.toml, package.json, pnpm-workspace.yaml, go.work, go.mod, pyproject.toml, requirements.txt, Package.swift, Gemfile, global.json, `*.sln`, `*.csproj` | **ManifestAdapter** (workspace root + stack detection) |
| 5 | `config/detector.rs:5-150+` | package.json, pnpm-workspace.yaml, Cargo.toml (three sibling `if` blocks each doing its own parse) | **ManifestAdapter.detect_workspace_members()** |
| 6 | `tasks/processors/metadata/external_links.rs:28-148` | package.json, Cargo.toml (metadata link extraction) | **ManifestAdapter.extract_metadata_links()** or **ConfigAdapter** |
| 7 | `tasks/processors/metadata/summary.rs:16-58` | package.json, Cargo.toml (description) | **ManifestAdapter.extract_description()** |
| 8 | `tasks/processors/metadata/icons.rs:174-195` | svg, png, ico | Small isolated priority list; leave OR move to **AssetAdapter** |
| 9 | `tasks/handlers/helpers.rs:6-25` | 50+ binary extensions | **FileClassifier.is_binary()** |
| 10 | `tasks/handlers/scan_logic.rs:216-234` | 30+ source extensions (mixed with an `adapter_for_ext` call + fallback) | **FileClassifier.is_source_file()** unified with LanguageAdapter |

**Makefile handling today:** none. Nothing opens a Makefile anywhere. Command-surface adapters own this from day one.

### 1.6 Proposed adapter architecture

Four sibling traits. Each ecosystem/config family gets one implementation set.

```
LanguageAdapter   (exists — extend)
  fn language()            -> &str
  fn display_name()        -> &str          // NEW: "Rust" for label surfaces
  fn parse(...)            -> ParsedFile
  fn parse_to_ir(...)      -> IRParsedFile

ManifestAdapter   (NEW — largest gain)
  fn manifest_filenames()  -> &[&str]       // e.g. ["Cargo.toml"]
  fn ecosystem()           -> &str          // "cargo" | "npm" | "pypi" | "go" | ...
  fn is_workspace_root(&self, content) -> bool
  fn parse_manifest(&self, content) -> ParsedManifest   // deps, workspace members, name, version, publish, description, links
  fn detect_workspace_members(&self, repo_root: &Path) -> Vec<PackageInfo>
  fn parse_commands(&self, content, repo_root: &Path) -> Vec<Command>
  fn detect_toolchain(&self, content) -> Vec<CapabilityTool>  // (capability="db-schema", tool="dbd")
  fn infer_role(&self, manifest: &ParsedManifest, fs: FsSignals) -> Option<&'static str>

ConfigAdapter     (NEW — router + metadata cleanup)
  fn config_extensions()   -> &[&str]
  fn extract_metadata_links(&self, content) -> Vec<ExternalLink>
  fn extract_description(&self, content) -> Option<String>
  fn extract_version(&self, content) -> Option<String>

FileClassifier    (NEW — small, orthogonal)
  fn is_binary(&self, ext: &str) -> bool
  fn is_source_file(&self, ext: &str) -> bool
  fn is_parseable(&self, ext: &str) -> bool
```

Command surface can live on `ManifestAdapter` (`parse_commands` above) or split into a fifth `CommandAdapter` if we want Makefile / justfile / Taskfile to have their own adapters independent of an ecosystem manifest. Recommend: keep it on `ManifestAdapter` for manifest-owned scripts (npm scripts, Cargo aliases, pyproject scripts) and introduce a lightweight `BuildAdapter` for standalone command files (`Makefile`, `justfile`, `Taskfile.yml`, `mise.toml`) that don't map to a specific ecosystem.

### 1.7 DB implications

New tables (schema not yet designed):

- **`sensei.folder_commands`** — `(folder_id, name, invocation, source_manifest, capability?, tool?, canonical_actions text[], confidence)`. Populated by the manifest/build adapters.
- **`sensei.project_commands`** — view or materialized rollup over `folder_commands`, with a distinct-per-verb constraint feeding a **command-conflict signal** (same project, same canonical verb, incompatible invocations).
- **`sensei.dojo_preferences`** — `(scope: 'user'|'org'|'project', scope_id, capability, preferred_tool, rationale?)`. Read at command-resolution time to bias `get_commands` output. Standalone-first: scope='user' rows only until Dōjō ships (see [[project_standalone_completion_plan]]).
- **`sensei.dojo_policies`** — `(scope, scope_id, policy_kind, definition_ref, severity, enforcement: advisory|blocking)`. Referenced by skills/agents that run in the dev cycle (§1.8).

Existing tables that need touch-ups: `referenced_libraries.props` should preserve `link:` / `workspace:` / `path=` intent so the read-path can surface it as "local dep" rather than external; a **version-inconsistency view** over `referenced_libraries` grouped by `(project_id, library_id)` with distinct `version_used` count > 1 gives us the signal for free.

### 1.8 Governance overlay & policy/security enforcement

**Preferences (routine).** Same-shape overlay every adapter consults. Example: `get_commands(project, action="db:apply")` returns `dbd deploy` when the active preference is `capability=db-schema, tool=dbd`; returns `alembic upgrade head` when `tool=sqlalchemy`. If nothing preferred, adapters return the detected candidate list ordered by confidence and expose the ambiguity — never guess silently.

**Policy / security (dev-cycle enforcement).** This is the newer scope. Two layers:

- **Skills** — Dōjō-authored, invokable during coding. A `dojo/db-schema-migration-review` skill runs when the assistant is about to write a migration; it reads the active `dojo_policies` for that capability, applies them (e.g., "no destructive DDL in a single deploy", "column renames must be two-phase") and reports blockers before the change lands. Skills are already a first-class surface in the marketplace/plugin architecture ([[project_sensei_plugin_architecture]]).
- **Agents** — long-running or on-demand reviewers. A `dojo-security-reviewer` agent walks recent changes for OWASP-family concerns using the Dōjō policy definitions (semgrep-style rules can live in `definition_ref`), scoped to the project's toolchain. Ties into the existing sensei-security-reviewer agent, but Dōjō-configurable instead of hard-coded.

The `dojo_policies` table is intentionally thin — a pointer, not a rules engine. Enforcement lives in the skills/agents that consume it. That keeps the daemon agnostic; the policy vocabulary belongs to Dōjō.

### 1.9 Gap list for Track 1

**Correctness first.**
1. Preserve full `@scope/name` in npm dep extraction — `libraries.rs:68-74`, `lib_indexer.rs:47-60`.
2. Recognize `link:` / `path=` / `workspace:` protocols — mark as local, do not promote to external, feed the project→project edge (rare-but-real).
3. Version-inconsistency view + signal — `(project_id, library_id)` with multiple `version_used`.

**Adapter refactor (unblocks everything else).**
4. Introduce `ManifestAdapter` and migrate `lib_indexer.rs:36-107`, `libraries.rs:271-276 / 345-349`, `scan_logic.rs:270-337`, `detector.rs:5-150+`, `external_links.rs`, `summary.rs` to route through it.
5. Extend `LanguageAdapter` with `display_name()`; migrate `codebase.rs:337-360` and `scan_logic.rs:216-234` to it. Fill missing language adapters (shell, ruby, php, etc.) that today only exist as hardcoded lists.
6. Introduce `FileClassifier`; migrate `helpers.rs:6-25` and the source-list half of `scan_logic.rs:216-234`.
7. Introduce `ConfigAdapter`; migrate `router.rs:40-60` and metadata extractors.

**Command surface.**
8. `parse_commands` on `ManifestAdapter` for npm / cargo / pyproject.
9. New lightweight `BuildAdapter` for Makefile / justfile / Taskfile / mise.
10. `folder_commands` + `project_commands` tables; MCP tools `list_actions` / `get_commands`.

**Governance overlay.**
11. `dojo_preferences` table + read-path bias in `get_commands`.
12. `dojo_policies` table + skill/agent hooks that consume it. Deferrable behind Dōjō main effort.

---

## Track 2 — Instruments

### 2.1 Mockup screens

Three tabs, from `docs/mockups/Sensei/lib/instruments.jsx` + siblings:

- **Playground** — MCP chooser (top row), per-MCP tool list, kind chips (all / action / query), search, two-pane list + detail. Tool detail carries `summary`, `kind`, structured `inputs` (label / type / required / default / help), example response.
- **Replay** — per-session timeline. Left: session picker with FTR / correction badges. Right: call list with request/args, response snippet, duration, and a **usage classification** (used / partial / ignored).
- **Health** — KPI strip (sessions analyzed, total calls, FTR, warnings, dormant tools), signal cards (`warn` / `opportunity` / `unused` / `win`), per-tool usage table (calls, 14d trend, usage split %, FTR delta), by-project adoption table.

The project-scoped instruments page (`(project)/project/[id]/instruments/+page.svelte`) is currently a flat list with `[global]` vs `[project]` scope badges — mockup extends this to per-tool call/FTR stats scoped to the project.

### 2.2 Data-contract vs endpoint matrix

| Screen | Data prop | Endpoint | Table | Status |
|---|---|---|---|---|
| Playground | `mcps[]`, `tools[]` w/ kind/summary/inputs/example | `GET /api/mcp/tools` | REGISTRY const (13 hard-coded sensei tools) | Placeholder — no kind/summary/inputs/example |
| Playground | Third-party MCPs, per-server tools | none | none | Missing |
| Replay | `sessions[]` metadata | `GET /api/sessions` | `activity.sessions` | Exists |
| Replay | per-session call timeline | none | `activity.assistant_events` (rows exist) | No aggregation endpoint |
| Replay | usage classification | none | none | Verdict inference not built |
| Health | usage-split %, trend, FTR delta, signals | `GET /api/observatory/tool-usage` | view over `assistant_events` | Shallow — no split, no delta, no signals |
| Project-scoped | MCP tools with scope + call/FTR | `GET /api/projects/{id}/instruments` | returns **extensions** (skills/commands), not MCP tools | Wrong table — needs MCP-tool aggregation |

### 2.3 Gaps

- Tool manifest richness — the daemon has to expose `{ kind, mcp, summary, inputs[], example_response }`. Either derive from JSON-schema of each MCP or store curated metadata alongside the registered tool.
- `mcp_servers` table — no persistent store today. Installed state comes from scanning `.acp` files. Persisting the registry is the entry point for per-project enable/disable and connection state.
- Replay aggregation endpoint — pair PreToolUse ↔ PostToolUse events on `activity.assistant_events`, join with the next assistant turn to derive the used/partial/ignored verdict, serialize the timeline. New table (`tool_calls`) OR a heavy view over `assistant_events`.
- Insights task — periodic aggregation that computes per-tool usage split, 14d trend, FTR delta (sessions calling the tool vs not), signal recommendations.
- Project-scoped instruments endpoint — return MCP tools joined with a per-project call/FTR aggregation, not extensions.

### 2.4 Frontend architecture (mockup fidelity + reuse)

- **MCP-connection store.** A single `mcp.svelte.ts` state slice owning connection status, server list, tool manifests, and cached recent responses. Fed exclusively by `load()` + a targeted subscription (SSE) once the daemon can push. Every screen reads from this store; no component fetches MCP state independently.
- **Rokkit primitives.** Use `List` (grouped) for the MCP + tool sidebar, `Tabs` for the three sections, `Table` for the health screen. Fall back to bespoke Svelte components only where rokkit's data model doesn't fit (specifically: the two-pane playground and the timeline).
- **Variant-driven cards.** One `SignalCard.svelte` with `variant: 'warn' | 'opportunity' | 'unused' | 'win' | 'neutral'` for the Health screen; one `StatBlock.svelte` with `tone: 'positive' | 'negative' | 'neutral'` for KPIs; one `ToolRow.svelte` for both playground and project-scoped lists (kind/scope/badge props). No duplication.
- **Presentation vs state separation.** `*.svelte.ts` files own derivations (usage-split %, FTR delta labels, verdict copy). `.svelte` files are pure templates. Per `sensei/app/CLAUDE.md`.

---

## Track 3 — Project window

### 3.1 Overall shell

- Route scaffolding exists: `(project)/project/[id]/{overview, sessions, patterns, libraries, traceability, memories, impact, instruments, about}` + `ProjectSidebar.svelte` + `+layout.svelte`. Redirect from `[id]` → `[id]/overview` in place.
- Mockup shell: `project-pages.jsx:257-305` (`ProjectPageSidebar`) — 220px sticky sidebar, 9 section buttons with badges, right pane flex.
- Multi-window: **not wired.** `tauri.conf.json` defines only `"main"`. No `WebviewWindow` creation in Rust today. To open as a separate Tauri window: add a `label:"project"` window config, add a `#[tauri::command] open_project_window(project_id)`, wire it into `invoke_handler`. This is a one-time infra addition, not per-screen.

### 3.2 Per-screen status

| Screen | Frontend | Endpoint | Data source | Overall |
|---|---|---|---|---|
| **Overview** | partial | `projects/{id}/ftr` ✓; sessions/hotspots/recs missing | `activity.sessions`, `inference.recommendations`, hotspots need derivation | **50%** |
| **Sessions** | partial | falls back to `/api/sessions?project=` | `activity.sessions` (has project_id, ftr, corrections) | **60%** |
| **Memories** | stub | `projects/{id}/memories` ✓ | `sensei.memories` (has project_id, kind, status) | **30%** — no "ready-to-share" batch surface |
| **Traceability** | stub | `projects/{id}/drift` returns raw | no drift storage; detection job absent | **10%** |
| **Libraries** | stub | `projects/{id}/libraries` ✓ | `project_libraries` + `libraries` | **20%** — no wrap/instrument-attached badges; needs Track-1 version-inconsistency signal |
| **Instruments** | stub | `projects/{id}/instruments` returns extensions | Track 2 territory | **5%** |
| **Patterns** | stub | `projects/{id}/patterns` ✓ but thin | reasoning traces + candidate patterns; mockup expects confidence/enforcement/example | **30%** |
| **Impact** | stub | **none** | greenfield — no outcomes/verdict logging | **0%** |
| **About** | stub | `projects/{id}` ✓ | `sensei.projects` (JSONB settings) | **40%** — no edit-mode form |

### 3.3 Cross-cutting metrics

| Metric | Screens | Producer | Reader | Status |
|---|---|---|---|---|
| FTR 14d + sparkline | Overview, Header | `get_project_ftr` | endpoint | ✓ |
| Sessions 7d count | Overview, Header | filter over `activity.sessions` | ⚠️ project-scoped endpoint missing | partial |
| Memories count + status | Overview, Memories | `sensei.memories` | ⚠️ status-grouping endpoint missing | partial |
| Doc drift | Overview, Traceability | ❌ background scan not built | none | missing |
| Hotspots (rework count) | Overview | needs `inference.reasoning_traces` + corrections aggregation | ⚠️ endpoint missing | partial |
| Pattern compliance % | Overview signals | ❌ not computed | none | missing |
| Tool effectiveness FTR-per-tool | Instruments | ❌ aggregation not built | none | missing (Track 2) |
| Impact verdict | Impact | ❌ no outcome logging | none | greenfield |
| Commands / actions | About (planned) | ← Track 1 command surface | new MCP tools | ← Track 1 |

### 3.4 Missing daemon endpoints

- `GET /api/projects/{id}/sessions?limit&since`
- `GET /api/projects/{id}/hotspots?since&limit`
- `GET /api/projects/{id}/recommendations?status`
- `GET /api/projects/{id}/drift/summary` + `.../drift`
- `GET /api/projects/{id}/impact/verdicts`
- `GET /api/projects/{id}/tools/stats?since=7d` (Track 2)
- `GET /api/projects/{id}/memories?status=sharing` batch surface
- `GET /api/projects/{id}/commands` (Track 1)

### 3.5 Missing DB structures

- Doc-drift tracking (target: `sensei.doc_drift`) — none today.
- Tool-call aggregation per-project (target: view or `tool_call_stats` table) — none today.
- Impact verdicts (target: `sensei.impact_verdicts` or extend `memory_outcomes`) — none today.
- Memory-sharing schedule (target: `sensei.memory_share_batches`) — none today.

---

## Sequencing recommendation

Priority ordering per the ask, with an internal ordering that maximises early value:

**A. Dep-map correctness sprint (Priority 1a — 3-5 days).** Ship the residual scoped-name fix (#30 residual), `link:` / `path=` / `workspace:` protocol handling, version-inconsistency view + signal. Also lands: mark `link:`/`workspace:` deps so the future project→project edge is one small step. No new adapters yet — surgical.

**B. Adapter refactor + command surface (Priority 1b — 1-2 weeks).** Introduce `ManifestAdapter`, `FileClassifier`, `ConfigAdapter`; migrate the ten spaghetti sites; add `LanguageAdapter.display_name()`. Then land `parse_commands` on `ManifestAdapter` + lightweight `BuildAdapter` for Makefile / justfile / Taskfile. New `folder_commands` + `project_commands` + MCP `list_actions` / `get_commands`. This is the largest wedge — it removes the class of bug that produced #30/#62/#63 and unlocks the assistants-stop-guessing-commands work.

**C. Instruments rebuild (Priority 2 — 1 week).** New `mcp.svelte.ts` store, three tabs, variant-driven cards. Needs three daemon additions in parallel: rich tool manifest, replay aggregation endpoint, insights aggregation task. Rokkit primitives where they fit, bespoke components (playground, timeline) where they don't.

**D. Project window shell + easy screens (Priority 3a — 3-4 days).** Multi-window Tauri wiring; Overview + Sessions + Libraries + About wired to existing endpoints. Real content on four of the nine screens.

**E. Project window analytics screens (Priority 3b — 1-2 weeks).** Hotspots + recommendations + patterns rich shape; Memories with sharing batch; Traceability with drift detection. These need new daemon endpoints and, for drift, a new detection job.

**F. Greenfield screens (Priority 3c — deferrable).** Impact verdicts — this is not just a UI; it's an outcome-logging effort that ties into the existing `memory_outcomes` and the analyzer. Deferrable unless a specific use case forces the timing.

**G. Governance overlay (crosses tracks — Dōjō-timing dependent).** `dojo_preferences` biasing `get_commands` output. `dojo_policies` + skill/agent hooks for policy/security enforcement. Standalone-first: user-scope preferences only until Dōjō ships. Aligns with the standalone completion plan (see [[project_standalone_completion_plan]]).

Each of A/B/C/D/E/F/G becomes its own brainstorm → spec → plan → build cycle. Pick which to open first.

---

## References

**Mockups**
- `docs/mockups/Sensei/lib/instruments.jsx`, `instruments-simple.jsx`, `mcp-replay-insights.jsx`, `mcp-signals-data.js`, `instruments-data.js`
- `docs/mockups/Sensei/lib/project-atlas.jsx`, `project-pages.jsx`, `project-lite-panes.jsx`, `project-logs.jsx`, `project-shared.jsx`, `project-filter.jsx`, `project-data.js`

**Adjacent analysis / plans**
- `docs/analysis/2026-06-24-mockup-vs-daemon-data-gap.md` — standalone completion plan reference
- `docs/analysis/2026-06-25-mockup-vs-app-implementation-gap.md` — UI rebuild spec

**Key files cited**
- Adapter: `crates/senseid/src/languages/mod.rs:17-22`
- Dep extraction: `crates/senseid/src/indexer/lib_indexer.rs:36-107`, `crates/senseid/src/tasks/handlers/libraries.rs:100+`
- Storage: `crates/senseid/src/db/pg_store.rs:1841-1878`, `:1918`, `:4562`
- Spaghetti sites: `router.rs:40-60`, `codebase.rs:336-360`, `libraries.rs:271-276`, `scan_logic.rs:270-337`, `detector.rs:5-150+`, `external_links.rs:28-148`, `summary.rs:16-58`, `icons.rs:174-195`, `helpers.rs:6-25`, `scan_logic.rs:216-234`
- Schema: `database/ddl/table/sensei/{libraries,referenced_libraries,project_libraries,projects,memories}.ddl`; `database/ddl/table/activity/{sessions,assistant_events}.ddl`
- Tauri window: `sensei/app/tauri.conf.json` (single window, no multi-window wiring)
- Project endpoints: `crates/senseid/src/api/handlers/project_detail.rs`

**Memory pointers**
- [[project_manifest_adapter_direction]] — the adapter direction distilled for future sessions
- [[feedback_no_command_guessing]] — feedback rule this analysis operationalises
- [[project_standalone_completion_plan]] — where Dōjō governance is sequenced
- [[project_ui_rebuild_2026_06_25]] — the current app UI rebuild phase this fits into
