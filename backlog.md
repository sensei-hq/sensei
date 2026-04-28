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
| 一 | Homebrew | UI done | Wire sidecar `check_homebrew` → gate status |
| 二 | PostgreSQL | UI done | Wire sidecar `check_formula("postgresql@17")` + `check_service` |
| 三 | Ollama | UI done | Wire sidecar `check_formula("ollama")` + `check_service` |
| 四 | Sensei components | UI done | Wire sidecar checks for CLI, MCP, daemon binaries |
| 五 | Database | UI done | Wire sidecar `check_database` |
| 六 | Daemon | UI done | Wire sidecar `check_service("daemon", 7744)` |

**Remaining work:**
- [ ] Verify browser rendering matches mockup exactly
- [ ] Wire Tauri sidecar commands to update gate statuses
- [ ] Retry button triggers actual sidecar install/start
- [ ] Auto-advance when all gates ready
- [ ] Test with actual Tauri app (not just browser)

---

## 2. Setup Wizard (8 stages)

**Mockup:** [mockups/lib/setup-wizard.jsx](./mockups/lib/setup-wizard.jsx)
**Journey:** [journeys/02-setup-discovery.md](./journeys/02-setup-discovery.md)
**Route:** `/setup/*`

Each stage is a separate SvelteKit route under `(config)/setup/`.

| # | Stage | Route | Mockup section | Status |
|---|-------|-------|---------------|--------|
| 一 | Welcome | `/setup/welcome` | Landing + "Begin setup" | UI skeleton, needs mockup match |
| 二 | Assistants | `/setup/assistants` | ACP detection + toggle cards | UI skeleton, needs mockup match |
| 三 | Folders | `/setup/folders` | Add watch roots | UI skeleton, needs mockup match |
| 四 | Scan | `/setup/scan` | SSE progress + project cards + activity feed | State classes done, UI needs mockup match |
| 五 | Projects | `/setup/projects` | Confirm/edit project groupings | Placeholder |
| 六 | Libraries | `/setup/libraries` | Detected libraries toggle | Placeholder |
| 七 | Instruments | `/setup/instruments` | MCP registry recommendations | Placeholder |
| 八 | Done | `/setup/done` | "Observatory is ready" | UI skeleton |

Additional setup screens from mockups:
- [ ] Inference providers + models ([mockups/lib/wiz-inference.jsx](./mockups/lib/wiz-inference.jsx))
- [ ] Model role assignments ([mockups/lib/wiz-assignments.jsx](./mockups/lib/wiz-assignments.jsx))

**Per-stage work pattern:**
1. Read the mockup JSX section for that stage
2. Define state class (or use existing ReactiveStageContext)
3. Build Svelte component matching mockup exactly
4. Wire load function / SSE events
5. Verify in browser with mock data
6. Test with Tauri + daemon

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
| Bootstrap | `GET /api/health/components` | Exists, response needs gate alignment |
| Scan | `GET /api/scan/events` (SSE) | Endpoint exists, needs 3-entity format |
| Scan | `POST /api/scan` | Exists |
| Projects | CRUD + merge | CRUD exists, merge not yet |
| Libraries | `GET /api/libs` | Exists |
| Assistants | `GET /api/assistants/detect` | Exists |

---

## Order of work

```
1. Bootstrap UI       → verify mockup match → wire sidecar
2. Welcome            → match mockup
3. Assistants         → match mockup → wire detect API
4. Folders            → match mockup → wire scan roots API
5. Scan               → match mockup → wire SSE events
6. Projects           → match mockup → wire project CRUD
7. Libraries          → match mockup → wire libs API
8. Instruments        → match mockup → wire services API
9. Done               → match mockup
10. Daemon pipeline   → complete scan events (parallel with UI)
```

Each step: mockup → state → component → API → test. No skipping.
