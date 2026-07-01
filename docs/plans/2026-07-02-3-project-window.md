---
name: 2026-07-02 — T3 Project window
issue: TBD
epic: https://github.com/sensei-hq/sensei/issues/83
analysis: docs/analysis/2026-07-01-project-window-instruments-depmap-gap-analysis.md
mockups:
  - docs/mockups/Sensei/lib/project-atlas.jsx
  - docs/mockups/Sensei/lib/project-pages.jsx
  - docs/mockups/Sensei/lib/project-lite-panes.jsx
  - docs/mockups/Sensei/lib/project-logs.jsx
  - docs/mockups/Sensei/lib/project-shared.jsx
  - docs/mockups/Sensei/lib/project-filter.jsx
  - docs/mockups/Sensei/lib/project-data.js
---

# T3 — Project window

Nine-screen project-scoped window (Overview, Sessions, Patterns, Libraries,
Traceability, Memories, Impact, Instruments, About) with a per-project
sidebar shell and multi-window Tauri opening. Splits into a fast **shell +
easy screens** slice and a slower **analytics screens** slice.

**Assumes T2 landed for the Instruments tab.** Sessions / Libraries / About
already have working endpoints; the analytics screens (Hotspots,
Recommendations, Patterns, Memories, Traceability, Impact) each need new
daemon capabilities.

## Success gate

- `#[tauri::command] open_project_window(project_id)` opens the project in a
  separate Tauri window from the app shell.
- Nine sidebar entries render with per-project badges (counts).
- Overview + Sessions + Libraries + About show real data.
- Analytics screens (Hotspots / Recommendations / Patterns / Memories /
  Traceability) show real data driven by new endpoints.
- Impact screen renders even if empty (greenfield).
- Playwright e2e opens the project window, clicks each sidebar entry,
  verifies non-empty content on the wired screens.

## Slice 1 — Shell + easy screens (3–4 days)

The four screens whose endpoints already exist plus the multi-window
plumbing.

### 1.1 Multi-window Tauri (1 day)

**Deliverables**
- `app/src-tauri/tauri.conf.json` — add a `"project"` `WebviewWindow` config
  (label pattern, default size, min size).
- `app/src-tauri/src/commands.rs` — new `#[tauri::command] open_project_window(project_id: String)`
  that creates the labeled window and passes `project_id` as an init arg.
- Register in `invoke_handler!`.
- `app/src/routes/(project)/+layout.svelte` — read the init arg on window
  load, hydrate the ProjectSidebar's active project.

**Tests**
- Playwright: from the main window, click a project row → new window opens
  showing the Overview screen for that project.

### 1.2 Sidebar shell (0.5 day)

**Deliverables**
- `app/src/lib/components/ProjectSidebar.svelte` matched to
  `project-pages.jsx:257-305` — 220px sticky, 9 buttons with badges,
  active-route highlight.
- `app/src/routes/(project)/project/[id]/+layout.svelte` — 3-col layout
  (sidebar 220 / content flex / right pane 320 optional).

**Tests**
- Vitest: sidebar renders 9 entries; active class flips on route change.

### 1.3 Overview (1 day)

**Deliverables**
- `+page.svelte` reads `sessions_7d`, `ftr_14d`, `hotspots`, `recommendations`.
- Endpoint work:
  - `GET /api/projects/{id}/sessions?limit&since` (new — currently the app
    falls back to `/api/sessions?project=`).
  - `GET /api/projects/{id}/hotspots?since&limit` (new).
  - `GET /api/projects/{id}/recommendations?status` (new).

**Tests**
- Endpoint round-trips return the expected shape.
- Playwright: overview renders KPIs + hotspots list + recent sessions.

### 1.4 Sessions screen (0.5 day)

**Deliverables**
- `+page.svelte` — table filtered to project, using the same
  `GET /api/projects/{id}/sessions` endpoint from 1.3.
- Columns per mockup: date / model / turns / corrections / FTR / outcome.

**Tests**
- Playwright: sessions page shows real rows and each row is clickable to
  the session detail.

### 1.5 Libraries screen (0.5 day)

**Deliverables**
- Use existing `GET /api/projects/{id}/libraries` + the version-inconsistency
  endpoint T1a shipped (`.../library-version-conflicts`).
- Add wrap/instrument-attached badges from the T1a props JSONB
  (`local_source`).

**Tests**
- Playwright: libraries page shows conflicts row highlighted; local-source
  libs get the workspace badge.

### 1.6 About screen (0.5 day)

**Deliverables**
- Read-mode: existing `GET /api/projects/{id}`.
- Edit-mode form: `PUT /api/projects/{id}` accepting the JSONB settings
  patch (name, role, notes, custom tags). Backend already stores it; the
  form is new.

**Tests**
- Playwright: edit-mode toggle, save round-trip, revert flow.

## Slice 2 — Analytics screens (1–2 weeks)

Each screen needs a new endpoint plus (in some cases) new derivation logic.

### 2.1 Patterns screen (1 day)

**Deliverables**
- Extend `GET /api/projects/{id}/patterns` to include confidence,
  enforcement, example fields (mockup expects these).
- Frontend: `SignalCard.svelte` reuse from T2 Health tab.

**Tests**
- Endpoint returns confidence field.
- Playwright: renders confidence rings.

### 2.2 Memories screen (2 days)

**Deliverables**
- New endpoint `GET /api/projects/{id}/memories?status=sharing` — a batch
  surface returning ready-to-share memories grouped by kind.
- New DDL: `sensei.memory_share_batches` (batch_id, project_id, memories
  uuid[], created_at, status).
- Frontend: "share batch" list with checkbox multi-select and a
  bulk-approve button.

**Tests**
- Batch creation writes the row and marks memories `status=sharing`.
- Playwright: create batch, approve → memories move to `status=shared`.

### 2.3 Traceability screen — doc drift (3 days)

**Deliverables**
- New DDL: `sensei.doc_drift` (id, project_id, doc_path, drift_score,
  reason, detected_at, resolved_at).
- New scheduled task `DetectDocDrift` — nightly. Cross-refs `README.md` and
  `.sensei/*.md` against the last N days of code churn; flags docs whose
  claims contradict the code.
- Endpoints: `GET /api/projects/{id}/drift/summary`,
  `GET /api/projects/{id}/drift` (rows).
- Frontend: sorted list with severity chips + "mark resolved" action.

**Tests**
- Drift detector unit tests: a doc mentioning a deleted function scores > 0.
- Playwright: drift table renders, mark-resolved round-trips.

### 2.4 Hotspots — reuse (already in 1.3)

Nothing extra — 1.3 already ships the endpoint. This slice only surfaces
it in the Hotspots panel on the Overview screen (row already covered).

### 2.5 Recommendations — reuse (already in 1.3)

Same — 1.3 covers the endpoint; this slice wires the detail-pane action
(accept / reject) via the existing `POST /api/mcp/call
accept_proposal|reject_proposal` MCP tools.

**Tests**
- Accept and reject flows update the row status.

## Slice 3 — Impact (deferrable)

Impact is greenfield — no outcome logging exists today.

**Deliverables**
- New DDL: `sensei.impact_verdicts` (id, project_id, session_id, kind
  enum, verdict enum, note text, created_at).
- New endpoint `POST /api/projects/{id}/impact/verdicts` +
  `GET /api/projects/{id}/impact/verdicts`.
- Frontend: verdict cards + timeline.

**Tests**
- Verdict round-trip.

**Deferred unless a specific use case forces the timing.**

## Sequencing

Slice 1 first, all six sub-slices merge to `develop` in order (1.1 → 1.6);
merge `develop → main` after 1.6 clears E2E.

Slice 2: 2.1 → 2.2 → 2.3, each merged to `develop` and then to `main`
after E2E.

Slice 3 deferred to its own session/window when the impact concept matures.

## Non-goals

- Real-time collaboration on the About form.
- Attempt-at-impact-verdict auto-detection (would need cross-session
  attribution — not scoped here).
- Multi-project comparison views (roll-up dashboards).
