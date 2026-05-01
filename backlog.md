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
| F2 | Contracts + mock data — `contracts.ts`, `mock-contracts.ts`, shape tests | Not started |
| F3 | WizardState singleton — hydrate, commitStage, canAdvance, firstPending | Not started |
| F4 | Loaders + layout integration — `loadWizardData()`, `+layout.ts`, rename folders→roots | Not started |

**Plan:** [superpowers/plans/2026-04-30-wizard-state-implementation.md](./superpowers/plans/2026-04-30-wizard-state-implementation.md)

### Per-stage

Each stage: read mockup → build component from `wizardState` → unit tests → integration test → verify in browser → commit.

| # | Stage | Route | Status | Notes |
|---|-------|-------|--------|-------|
| 礼 | Welcome | `/setup/welcome` | UI done, needs commit wiring | Verify end-to-end commit flow |
| 名 | Preferences | `/setup/preferences` | Placeholder | Display name, sharing, correction, telemetry |
| 連 | Assistants | `/setup/assistants` | UI done, needs refactor | Read from `wizardState`, remove old `+page.ts` load |
| 庵 | Roots | `/setup/roots` | UI done (was /folders), needs refactor | Exclusion editing, rename, read from `wizardState` |
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

## 3. Daemon Scan Pipeline

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
| Spike: install plugin, write one test for health→setup flow | Not started |
| Configure browser mode (headless, mocked IPC) for CI | Not started |
| Configure Tauri mode for local E2E | Not started |
| Add wizard stage test fixtures | Not started |

**Setup:**
- Rust: feature flag `e2e-testing = ["tauri-plugin-playwright"]` in src-tauri/Cargo.toml
- JS: `@srsholmes/tauri-playwright` + `@playwright/test`
- Config: `"withGlobalTauri": true` in tauri.conf.json

---

## Order of work

```
1. Bootstrap           → Done (2026-04-29)
2. Layout chrome       → Done (2026-04-30)
3. Foundation (F2-F4)  → Done (2026-05-01) — contracts, wizard state, loaders, layout wiring
3a. E2E spike          → tauri-plugin-playwright setup + first test
4. Welcome             → verify commit flow end-to-end
5. Preferences         → match mockup → wire config API
6. Assistants          → refactor to wizardState → wire configure API
7. Roots               → refactor + exclusions → wire scan roots API
8. Scan                → baseline + incremental → wire SSE
9. Projects            → match mockup → wire project CRUD
10. Libraries          → match mockup → wire libs API
11. Instruments        → match mockup → wire MCP registry API
12. Done               → wire commitAll
13. Daemon pipeline    → complete scan events (parallel with UI)
```

Each step: mockup → state → component → API → test. No skipping.
