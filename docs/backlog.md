---
name: Implementation Backlog
description: Screen-by-screen task list — what needs to be built, in order
date: 2026-04-28
---

# Implementation Backlog

One screen at a time. Each screen: read mockup → state class → component → wire API → test.

## 1. Bootstrap (6 gates)

**Mockup:** [mockups/lib/bootstrap.jsx](./mockups/lib/bootstrap.jsx)
**Journey:** [journeys/01-install-bootstrap.md](./journeys/01-install-bootstrap.md)
**Route:** `/health`

| # | Gate | Status | What remains |
|---|------|--------|-------------|
| 一 | Homebrew | Done | Platform provider (macOS/Windows stub) |
| 二 | PostgreSQL | Done | `check_binary_and_service` — binary + port probe |
| 三 | Ollama | Done | `check_binary_and_service` — binary + port probe |
| 四 | Sensei components | Done | `check_binary` — sensei CLI |
| 五 | Database | Done | `database::check` — pg_isready + DB exists + pgvector + schema |
| 六 | Daemon | Done | `check_service` — port 7744 + /health |

**Completed (2026-04-29):**
- [x] Platform provider trait (macOS Homebrew + Windows winget stub)
- [x] Tauri sidecar: `run_bootstrap`, `install_prerequisites`, `start_services`, `setup_database`
- [x] Event contract: `{ action, entity, id, data }` via Tauri events
- [x] Health page: platform-aware remedies, collapsed prereq view, auto-phase progression
- [x] Auto-advance when all gates ready (health → setup wizard)
- [x] Auto-start daemon when down (senseid start --port 7744 fallback)
- [x] PATH enrichment for macOS .app bundle (Homebrew binaries)
- [x] Verified in compiled Tauri app — zero console errors, daemon stop → restart → auto-advance
- [x] 82 tests (41 Rust crate + 18 Tauri integration + 23 frontend unit)

---

## 2. Setup Wizard (11 stages)

**Mockup:** [mockups/lib/setup-wizard.jsx](./mockups/lib/setup-wizard.jsx)
**Journey:** [journeys/02-setup-discovery.md](./journeys/02-setup-discovery.md)
**Spec:** [superpowers/specs/2026-04-30-wizard-state-architecture-design.md](./superpowers/specs/2026-04-30-wizard-state-architecture-design.md)
**Route:** `/setup/*`

Each stage is a separate SvelteKit route under `(config)/setup/`.

### Foundation

| # | Task | Status |
|---|------|--------|
| F1 | Layout chrome — meaning kanji, 11 stages, rail+bottom bar mockup match | Done (2026-04-30) |
| F2 | Contracts + mock data — `contracts.ts`, `mock-contracts.ts`, shape tests | Done (2026-05-01) |
| F3 | WizardState singleton — hydrate, commitStage, canAdvance, firstPending | Done (2026-05-01) |
| F4 | Loaders + layout integration — `loadWizardData()`, `+layout.ts`, rename folders→roots | Done (2026-05-01) |

**Plan:** [superpowers/plans/2026-04-30-wizard-state-implementation.md](./superpowers/plans/2026-04-30-wizard-state-implementation.md)

### Per-stage

Each stage: read mockup → build component from `wizardState` → unit tests → integration test → verify in browser → commit.

| # | Stage | Route | Status | Notes |
|---|-------|-------|--------|-------|
| 礼 | Welcome | `/setup/welcome` | Done (2026-05-01) | |
| 名 | Preferences | `/setup/preferences` | Done (2026-05-06) | Switch component, bind:value, OKLCH tokens |
| 連 | Assistants | `/setup/assistants` | UI done, needs refactor | Read from `wizardState`, remove old `+page.ts` load |
| 庵 | Roots | `/setup/roots` | Done (2026-05-06) | CRUD wired — POST /api/scan/roots, DELETE /api/scan/roots/{id} |
| 観 | Scan | `/setup/scan` | UI done, needs baseline | Incremental counts (+N), keep SSE |
| 組 | Projects | `/setup/projects` | Placeholder | Project cards, role dropdowns, merge/split |
| 書 | Libraries | `/setup/libraries` | Placeholder | Library list, enabled toggle, add by URL |
| 器 | Instruments | `/setup/instruments` | Placeholder | MCP cards, project_count, toggle |
| 想 | Inference | `/setup/inference` | DEFERRED | Needs gateway integration design |
| 任 | Assignments | `/setup/assignments` | DEFERRED | Needs gateway integration design |
| 入 | Done | `/setup/done` | UI done, needs commitAll | Verify end-to-end |

### Per-stage work pattern

1. Read the mockup JSX section for that stage
2. Write failing tests (`.spec.svelte.ts`)
3. Build Svelte component reading from `wizardState.{slice}`
4. Commit logic in `wizardState.commitStage()`
5. Verify in browser with daemon running
6. Full test suite + type check — zero errors
7. Commit

---

## 3. Bootstrap Diagnostic Logging + Debug Mode

**Priority:** High — needed for debugging DMG/installed app issues

### Structured Trace Logging (bootstrap crate)

Every bootstrap check returns a `BootstrapTrace`:
```rust
struct BootstrapTrace {
    step: String,           // "pg_is_ready", "database_exists", "create_database"
    command: String,         // "pg_isready --quiet"
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    duration_ms: u64,
    success: bool,
}
```

- `check()` and `setup()` accumulate traces alongside results
- Every `Command::new()` call wrapped in a trace-capturing helper
- Traces returned to Tauri sidecar and emitted as events

### Log Viewer Screen (app)

- New route: `(app)/log` or `(health)/log`
- Shows bootstrap steps as a timeline: step name, command, result, duration
- Color-coded: green (success), red (failure), yellow (warning)
- Expandable: click a step to see full stdout/stderr
- **Submit button**: creates a GitHub issue in `sensei-hq/sensei` with:
  - System info (macOS version, chip, RAM)
  - Bootstrap trace log
  - App version
  - Uses `gh` CLI or GitHub API via Tauri command

### App Menu

- Add `View > Log` menu item (Tauri menu API)
- Opens the log viewer screen

### Debug Mode

- CLI flag: `--debug` enables verbose logging
- Or: Settings toggle in the app
- When active: toast notifications on gate failures with error details
- Trace log always captured, debug mode just surfaces it in the UI

### Scope

| Component | Work |
|-----------|------|
| `daemon/crates/bootstrap` | `BootstrapTrace` struct, trace-capturing helper, update all checks |
| `app/src-tauri/src/commands/bootstrap.rs` | Expose traces via Tauri command |
| `app/src/routes/` | Log viewer page |
| `app/src-tauri/` | Menu item, `--debug` flag |
| Tests | Trace struct tests, log viewer E2E |

---

## 4. Daemon Scan Pipeline

**Spec:** [design/api/03-scan-event-flow.md](./design/api/03-scan-event-flow.md)

| Task | Status |
|------|--------|
| ScanRoot: discover + classify + enqueue | Done (scan_logic.rs, 14 tests) |
| ProcessGitFolder: stack + files + project + events | Partially done |
| Executor progress tracking (FolderProgress) | Not started |
| ResolveEdges: folder → indexed event | Not started |
| BuildConnections: project → active event | Not started |
| RegisterWatchRoots: add to watcher after scan | Existing, needs cleanup |

---

## 4. Daemon API alignment

Each screen needs daemon endpoints that return data in the shape the UI expects.

| Screen | Endpoint | Status |
|--------|----------|--------|
| Bootstrap | `GET /api/health/components` | Exists |
| Scan | `GET /api/scan/events` (SSE) | Exists, needs incremental mode for re-scan |
| Scan | `POST /api/scan` | Exists |
| Roots | `POST /api/scan/roots` (add with exclusions) | Missing |
| Roots | `PUT /api/scan/roots/:id` (update exclusions) | Missing |
| Roots | `DELETE /api/scan/roots/:id` | DB exists, no HTTP route |
| Projects | CRUD + merge | CRUD exists, merge not yet |
| Libraries | `GET /api/libs` | Exists |
| Libraries | `PUT /api/libs/configure` (toggle indexing) | Missing |
| Assistants | `GET /api/assistants/families` | Exists |
| Assistants | `POST /api/assistants/configure` | Exists |
| Instruments | `GET /api/mcp/registry` | Missing |
| Instruments | `POST /api/mcp/configure` | Missing |

---

## 5. E2E Testing Infrastructure

**Crate:** [tauri-plugin-playwright](https://github.com/srsholmes/tauri-playwright) v0.2.2
**Why:** Test wizard stages in the actual Tauri webview, not just a browser. Same test files run in browser mode (fast, mocked IPC) and Tauri mode (real webview + real daemon).

| Task | Status |
|------|--------|
| Spike: install plugin, write one test for health→setup flow | Done (2026-05-01) |
| Configure browser mode (headless, mocked IPC) for CI | Done — 21 tests passing |
| Configure Tauri mode for local E2E | Not started |
| Add wizard stage test fixtures | Done — fixtures.ts with IPC mocks |

**Setup:**
- Rust: feature flag `e2e-testing = ["tauri-plugin-playwright"]` in src-tauri/Cargo.toml
- JS: `@srsholmes/tauri-playwright` + `@playwright/test`
- Config: `"withGlobalTauri": true` in tauri.conf.json

---

## 6. Mockup gaps — screens needing design before build

Screens confirmed to have no mockup. Design (mockup → spec → plan) must precede implementation.

### 6a. Observatory — Configure section

**Priority:** High — "Configure" nav item in the observatory sidebar currently routes to `ObsPlaceholder`.

**What to design:**
- Global/app-level settings: display name, language, theme
- Inference section: local model management (pull/delete/update), MOE panel composition (proposer/challenger/synthesizer roles), external provider API keys, routing preference (auto/local/external)
- Assistants section: registered ACPs with registration status, re-register, transport config, test connection
- Privacy / telemetry controls

**Mockup file:** needs `configure.jsx` or extension of an existing settings mockup

---

### 6b. J7 — Extend & Customize (all screens missing)

**Journey:** [journeys/07-extend-customize.md](./journeys/07-extend-customize.md)
**Priority:** Medium — none of these screens have mockups

| Screen | What to design | Mockup file (needed) |
|--------|---------------|----------------------|
| Extensions browser | Filter by kind (skill/command/agent/hook/plugin), scope toggle, enable/disable per project, create/import actions | `extensions.jsx` |
| Skill editor | Frontmatter fields + markdown body editor, context preview panel, export/import as .md | `extensions.jsx` |
| Agent editor | Tool access checklist, autonomy level, template selector, test-against-replay panel | `extensions.jsx` |
| Persona editor | Trigger conditions (cwd glob, file types), rules list, evidence trail, context preview | `extensions.jsx` |
| Benchmark runner | Task corpus definition, A/B variant config, results table (FTR/corrections/tokens delta), conclusion | `benchmark.jsx` |

---

### 6c. J5 — Pattern knowledge catalog

**Journey:** [journeys/05-understand-codebase.md](./journeys/05-understand-codebase.md)
**Priority:** Low

**What to design:**
- Catalog browser with category filters: GoF (structural/behavioral/creational), resilience, data access
- Per pattern: name, family, description, detection status ("detected in N places" / "not present — recommended")
- Evidence: sessions that used this pattern and FTR correlation
- Import pattern as a project rule (suggested or rule)

**Mockup file:** extend `project-shared.jsx` or new `patterns.jsx`

---

### 6d. J9 — Context pack tool

**Journey:** [journeys/09-memory-and-learning.md](./journeys/09-memory-and-learning.md)
**Priority:** Low

**What to design:**
- Mid-session context reset panel triggered when session feels bloated
- Shows: current progress snapshot (what's done, pending, in-flight files)
- Memories to reload: grouped by scope (global/project/module/task-type) with counts
- Noise to clear: stale file reads and accumulated context
- Execute rotation or cancel

**Mockup file:** new `context-pack.jsx` or extend `learnings.jsx`

---

## Known Issues

| # | Area | Issue | Notes |
|---|------|-------|-------|
| K1 | Daemon — ACP config | JSONC comment loss on rewrite | `upsert_sensei_in_json` / `remove_sensei_from_json` parse with `json5` (preserves all key/value data) but serialize back with `serde_json` (strips comments). All settings values are preserved; only `// comments` and trailing commas are lost. Full fix requires a CST-preserving JSONC editor — no mature Rust crate exists. Options: (a) preserve leading comment block (prefix before `{`), (b) surgical text splice at the key range. |
| K2 | Database — DDL upgrades | No rollback for partial or failed upgrades | A partial DDL upgrade (e.g. `dbd apply` interrupted mid-run) leaves the schema in an inconsistent state. The current `dbd reset + apply` workflow is acceptable pre-release but will not be safe once stable versions ship to users. A rollback mechanism is needed — likely transactional DDL snapshots or migration checkpoints inside `dbd` itself (not in `senseid` code). Offload to `dbd` team. Consider before first stable release. |

---

## App load status review findings (2026-05-14)

Source: rigorous code review of the cold-start gate flow (`hooks.client.ts` reroute, `health-state.svelte.ts`, `+layout.ts` load order, Tauri event listeners). The original symptom — `make app-dev` loaded the observatory with the daemon down — was triggered by a `VITE_BYPASS_HEALTH` env leak (fixed in commit `5ae40b4f`: env var bound to `bun run dev` script in `app/package.json` only, no longer exported globally by the Makefile). The review found other bypass paths that were not exercised this time but ship in production and will surface eventually.

| # | Area | Severity | Issue | Fix |
|---|------|----------|-------|-----|
| L1 | `+layout.svelte:77-80` + `src-tauri/src/lib.rs:110-119` | ~~Critical~~ Done (2026-05-15) | `dev-navigate` Tauri event listener wrote `sessionStorage.setItem('sensei:health', 'ready')` unconditionally. Bypass shipped in production. | Extracted `health-cache.ts` as sole owner of the `sensei:health` key and `VITE_BYPASS_HEALTH` env read. `HealthState` and `hooks.client.ts` both route through it. `+layout.svelte` listener now only navigates. Menu items kept in production per product decision; bypass is impossible because no other code can write the cache key. Deleted dead `setup/daemon.ts`. |
| L2 | `hooks.client.ts:60` | ~~Critical~~ Done (2026-05-15) | Upgrade gate `if (pendingUpgrade && !HEALTH_EXEMPT.has(path) && path !== '/')` excluded the observatory route from the upgrade redirect. | Removed the `&& path !== '/'` clause. HEALTH_EXEMPT already covers /upgrade itself; no loop concern. |
| L3 | `+layout.ts:14-19` + `appstate.svelte.ts:106-110` | ~~Critical~~ Done (2026-05-15) | Root layout `load()` called `appState.load()` → `fetch('/api/config')` on every navigation, including `/health`. | Stripped root `+layout.ts` to just `ssr/prerender` flags. Added `+layout.ts` to `(observatory)`, `(project)`, `(config)` groups (each calls `appState.load()` when not yet loaded). `(health)` has no layout load, so the health route makes zero daemon calls before the gate is green. Removed redundant `appState.load()` from `(project)/project/[id]/+layout.ts`. |
| L4 | `hooks.client.ts:76` vs `appstate.svelte.ts.setupComplete` | ~~Critical~~ Done (2026-05-15) | Two sources of truth for setup-complete drift when daemon write silently failed. | Daemon is now canonical; localStorage is a sync cache. Added `tryPut` + `trySetConfig` + `tryGetConfig` to senseiApi. `setSetupComplete()` only writes localStorage on daemon success and throws on failure (wizard UI surfaces the error). `appState.load()` reconciles localStorage from daemon truth on every Tauri-mode load; transient outages leave the cache untouched so the user isn't bounced back through setup. |
| L5 | `api.ts:20-50` | ~~Important~~ Done (2026-05-15) — layout-level | Adopted audit option (b): `(observatory)/+layout.ts`, `(project)/+layout.ts`, `(config)/+layout.ts` now `throw error(503)` if `appState.load()` returns false. `appState.load()` returns `boolean` indicating Tauri-mode reachability. Per-loader `tryGet` migration of all 20+ existing `+page.ts` callers is deferred — guarded at the group-layout boundary, so non-health pages never mount with empty config. |
| L6 | `health-transport.ts:26-29` + `src-tauri/commands/bootstrap.rs:14-18` | ~~Important~~ Done (2026-05-15) | Added `bootstrap::health::process_util::output_with_timeout` (std-only spawn + try_wait poll + kill on deadline). Default 5s per checker. `BinaryChecker` and `PostgresDatabaseChecker` wrapped; timeouts surface as `Failed` with explicit "timed out after Ns" detail. Tested with `true` (Done), `sleep 10` clamped to 200ms (TimedOut, returns near deadline, child killed), and missing binary (Failed). |
| L7 | `appstate.svelte.ts:115-127` (`appState.reset()`) | ~~Important~~ Done (2026-05-15) | Replaced `localStorage.clear() + restore sensei:port` with an explicit PROTECTED list (`sensei:port`, `sensei:app-version`). Reset now snapshots protected values, clears, and restores them. A staged upgrade survives reset. |
| L8 | `(config)/+layout.svelte:40` | ~~Important~~ Done (2026-05-15) | `goto("/observatory")` referenced a non-existent route. | Changed to `goto('/')`. |
| L9 | `app/src/lib/bootstrap.ts` | ~~Important~~ Done (2026-05-15) | Six orphaned exports (`checkAndFixBootstrap`, `listenBootstrapEvents`, `listenBootstrapReport`, `detectHardware`, `listModels`, `missingModels`) + five interface types, none with live callers. `checkAndFixBootstrap` invoked a Tauri command that no longer existed. | Reduced `bootstrap.ts` to just `hasTauri()`. |
| L10 | `hooks.client.ts:55-58` + DB resolver | ~~Minor~~ Done (2026-05-15) — both layers | (a) reroute now compares `localStorage['sensei:app-version']` against build-time `__SENSEI_APP_VERSION__` (from package.json via vite define); stale flags matching the running binary no longer loop users through /upgrade. (b) Restored deleted `bootstrap::database` module from `7a73eb70`'s `_legacy/database.rs` — `pg_is_ready`, `database_exists`, `create`, `ensure_extensions`, `deploy` (dbd-core resolve_source + PostgresAdapter), `setup`. `DatabaseResolver` now calls `database::setup(db_name, env!("CARGO_PKG_VERSION"))` (no more phantom `sensei db:create`). New `bootstrap::upgrade` module composes `brew upgrade <formula>` + `database::deploy` (same deploy fn — DRY across initial install and post-update). `run_upgrade_steps` calls `bootstrap::upgrade::run` with a Tauri emit closure; emits the `prereqs`/`db_deploy` events the existing /upgrade UI already listens for. |
| L11 | `+layout.svelte:69` + 2 other sites | ~~Minor~~ Done (2026-05-15) | Inline `(window as any).__TAURI__` cast in three production sites. | All three now call `hasTauri()`; the only remaining cast is the canonical one inside `hasTauri()` itself, typed (not `any`). |

**Recommended order:** L1 → L2 → L3 → L4 (the critical four), then L5/L6/L7 (foundational), then minor cleanups. Several of these (L4, L5) need a brief design discussion before implementation.

---

## Order of work

```
1.  Bootstrap           → Done (2026-04-29)
2.  Layout chrome       → Done (2026-04-30)
3.  Foundation (F2-F4)  → Done (2026-05-01) — contracts, wizard state, loaders, layout wiring
3a. E2E spike           → Done (2026-05-01) — 21 browser-mode tests passing
3b. Preferences stage   → Done (2026-05-01) — UI + unit + E2E tests
4.  Bootstrap logging   → structured trace, log viewer, debug mode, GitHub submit
5.  Assistants          → refactor to wizardState → wire configure API
6.  Roots              → refactor + exclusions → wire scan roots API
7.  Scan               → baseline + incremental → wire SSE
8.  Projects           → match mockup → wire project CRUD
9.  Libraries          → match mockup → wire libs API
10. Instruments        → match mockup → wire MCP registry API
11. Done               → wire commitAll
12. Daemon pipeline    → complete scan events (parallel with UI)
```

Each step: mockup → state → component → API → test. No skipping.

---

## 7. CSS Migration — inline utility classes and semantic design tokens

**Priority:** Medium (quality / maintainability — do screen-by-screen alongside other work)
**Goal:** Replace `<style>` block CSS with Tailwind/UnoCSS utility classes, eliminate
fractional pixel values, and make all spacing / typography / color driven by the
rokkit.config.js token system so the whole app feels consistent and cohesive.

### Why this matters

The app currently has ~30 distinct font-size values (9px, 9.5px, 10px, 10.5px, 11px,
11.5px, 12px, 12.5px, 13px, 14px…) spread across per-component `<style>` blocks.
Each screen picked its own spacing independently — `padding: 48px 48px 64px`, `gap: 10px`,
`gap: 12px`, `margin: 0 0 6px` — with no shared constraint. The result: no screen looks
obviously wrong but no two screens feel like they came from the same system.

The fix is to collapse these into a small named type scale and a 4px-grid spacing system,
both adjustable through config rather than scattered across dozens of style blocks.

---

### Phase 1 — Audit (1–2 hours, do once)

| Step | What to do |
|------|-----------|
| A1 | Run: `grep -rh "font-size:" app/src --include="*.svelte" \| sort -u` — list every unique font-size value |
| A2 | Run: `grep -rh "padding:\|margin:\|gap:" app/src --include="*.svelte" \| sort -u` — list every unique spacing value |
| A3 | Run: `grep -rh "var(--paper\|var(--sumi\|var(--shu\|var(--jade\|var(--amber" app/src --include="*.svelte" \| sort -u` — list every custom token reference |
| A4 | Map each font-size to the nearest Tailwind size: 9–10px → `text-[10px]`, 11–11.5px → `text-xs` (12px), 12–12.5px → `text-xs` or `text-[13px]`, 13px → `text-sm`, 14px → `text-sm`, 20px → `text-xl`, 24px → `text-2xl` |
| A5 | Map spacing to 4px-grid: 6px → `gap-1.5`, 8px → `gap-2`/`p-2`, 10px → `gap-2.5`, 12px → `gap-3`/`p-3`, 14px → `p-3.5`, 16px → `p-4`, 18px → `p-4.5`, 20px → `p-5`, 24px → `p-6`, 32px → `p-8`, 48px → `p-12`, 64px → `p-16`, 80px → `p-20` |

---

### Phase 2 — Type scale (define once, apply everywhere)

**Target type scale** (maps to Tailwind text-* utilities, configured via `font-size` in theme if needed):

| Role | Size | Tailwind | Usage |
|------|------|----------|-------|
| display | 24px | `text-2xl` | Page titles, kanji callouts |
| heading | 20px | `text-xl` | Section headings, empty-state titles |
| title | 14px | `text-sm` | Card titles, step labels |
| body | 13px | `text-[13px]` | Primary body text, descriptions |
| caption | 12px | `text-xs` | Labels, badge text, metadata |
| micro | 10px | `text-[10px]` | Status dots, superscripts, secondary badge content |

**Steps:**

| Step | What to do |
|------|-----------|
| T1 | Add a Tailwind text-size alias to `uno.config.js` (if needed) for `text-[13px]` so it's consistent |
| T2 | Replace every `font-size: 13px` → `text-[13px]` (body class or utility) |
| T3 | Replace `font-size: 9px–10.5px` → `text-[10px]` (micro) |
| T4 | Replace `font-size: 11px–11.5px` → `text-xs` (12px, caption) |
| T5 | Replace `font-size: 12px–12.5px` → `text-xs` (caption) |
| T6 | Replace `font-size: 14px` → `text-sm` (title) |
| T7 | Replace `font-size: 20px` → `text-xl` |
| T8 | Replace `font-size: 24px` → `text-2xl` |
| T9 | Replace `font-size: 64px` (kanji) → `text-[64px]` or a named utility |

---

### Phase 3 — Spacing system (apply screen-by-screen)

Collapse all ad-hoc `padding/margin/gap` values to the 4px grid. Work one Svelte file at a time.

| Step | What to do |
|------|-----------|
| S1 | Replace `<style>` block rules that only control padding/margin/gap with inline utility classes on the element |
| S2 | Replace `padding: 24px` → `p-6`, `padding: 24px 48px` → `py-6 px-12` |
| S3 | Replace `gap: 8px` → `gap-2`, `gap: 10px` → `gap-2.5`, `gap: 12px` → `gap-3` |
| S4 | Replace `margin: 0 0 8px` → `mb-2`, `margin: 0 0 6px` → `mb-1.5` |
| S5 | Remove `--space-*` custom properties from `tokens.css` as each is superseded |
| S6 | Remove `--radius` / `--radius-lg` custom properties — use `rounded-md` / `rounded-lg` or `--radius-md` / `--radius-lg` from rokkit shape config |

---

### Phase 4 — Color tokens → Rokkit utilities (apply screen-by-screen)

Replace custom `var(--paper-*)` / `var(--sumi-*)` references with Rokkit semantic utility classes.

Palette is OKLCH (`colorSpace: 'oklch'`). Dual-surface: `kami` (light) + `sumi` (dark).
Use `oklch(var(--color-surface-z*) / alpha)` in pure-CSS contexts.

| Custom token | Rokkit utility | Notes |
|--------------|---------------|-------|
| `var(--paper)` | `bg-surface-z1` | kami-100 = 0.975 0.008 85 — exact match |
| `var(--paper-2)` | `bg-surface-z2` | kami-200 = 0.955 0.010 85 |
| `var(--paper-3)` | `bg-surface-z3` | kami-300 = 0.920 0.012 85 |
| `var(--paper-edge)` | `border border-surface-z2` | opaque approximation of alpha ink |
| `var(--sumi)` | `text-surface-z9` | kami-900 = 0.220 0.012 50 — exact |
| `var(--sumi-2)` | `text-surface-z7` | kami-700 = 0.380 0.012 50 — exact |
| `var(--sumi-3)` | `text-surface-z6` | kami-600 = 0.580 0.010 50 — exact |
| `var(--sumi-4)` | `text-surface-z5` | kami-500 = 0.750 0.008 50 — exact |
| `var(--shu)` | `text-primary-z5` | shu-500 = 0.580 0.150 35 — exact |
| `var(--shu-soft)` | `oklch(var(--color-primary-z5) / 0.12)` | alpha variants need CSS var |
| `var(--jade)` → `var(--hisui)` | `text-success-z5` | hisui-500 = 0.620 0.080 160 — exact |
| `var(--jade-soft)` | `oklch(var(--color-success-z5) / 0.14)` | |
| `var(--amber)` → `var(--kohaku)` | `text-warning-z5` | kohaku-500 = 0.720 0.120 75 — exact |
| `var(--amber-soft)` | `oklch(var(--color-warning-z5) / 0.15)` | |
| `var(--hairline)` | `border border-surface-z2` | or `oklch(var(--color-surface-z9)/0.08)` |
| `var(--border-card)` | `border border-surface-z2` | |
| `var(--border-focus)` | `border border-surface-z6` | focus ring |
| `var(--radius)` | `rounded-md` | soft preset = 6px |
| `var(--radius-lg)` | `rounded-lg` | soft preset = 10px |
| body background | `oklch(var(--color-surface-z0) / 1)` | z0 = kami-50 = 0.985, body/html only |

**Steps:**

| Step | What to do |
|------|-----------|
| C1 | Replace `color: var(--sumi)` → `text-surface-z9` |
| C2 | Replace `color: var(--sumi-3)` → `text-surface-z6` |
| C3 | Replace `background: var(--paper)` → `bg-surface-z1` (body stays as `oklch(var(--color-surface-z0)/1)`) |
| C4 | Replace `background: var(--paper-2)` → `bg-surface-z2` |
| C5 | Replace `border: var(--hairline)` or `var(--border-card)` → `border border-surface-z2` |
| C6 | Replace `color: var(--shu)` → `text-primary-z5` |
| C7 | Replace alpha tokens (`--shu-soft`, `--jade-soft`, `--amber-soft`) → `oklch(var(--color-*-z5) / alpha)` |
| C8 | After each screen is done, prune the token set from `tokens.css` |

---

### Phase 5 — Shared component CSS → Rokkit components

The `.btn-solid`, `.btn-outline`, `.btn-cta` classes in `tokens.css` are hand-rolled
versions of `@rokkit/ui` Button. Replace progressively.

| Step | What to do |
|------|-----------|
| B1 | Replace `<button class="btn-solid">` → `<Button>` from `@rokkit/ui` |
| B2 | Replace `<button class="btn-outline">` → `<Button variant="outline">` |
| B3 | Replace `<button class="btn-cta">` → `<Button variant="primary">` |
| B4 | After all references removed, delete `.btn-solid`, `.btn-outline`, `.btn-cta` from `tokens.css` |

---

### Phase 6 — Cleanup

| Step | What to do |
|------|-----------|
| X1 | Delete `tokens.css` sections that are fully superseded (spacing vars, color aliases, btn-* classes) |
| X2 | Keep `tokens.css` only for: font imports, `.display` / `.kanji` typography classes, scrollbar styling, `::selection`, dark mode OS-sync that Rokkit doesn't cover |
| X3 | Verify dark mode still works: toggle `[data-mode="dark"]` on body and check all screens |
| X4 | Run lint + full test suite — zero errors |

---

### Suggested order of screens

Work top-to-bottom through the `routes/` tree, one file per session:

```
1. +layout.svelte (root)         — body/html styles, font-family
2. (health)/+layout.svelte       — sidebar/nav chrome
3. (health)/health/+page.svelte  — gate list cards
4. (health)/logs/+page.svelte    — log viewer
5. (config)/+layout.svelte       — wizard chrome
6. setup/welcome → done          — each wizard stage in order
7. (observatory)/observatory     — main content screens
8. (observatory)/settings        — settings panels
```

After each screen: `bun run lint` (zero errors), smoke-test the screen in both light and dark mode.
