# Phase 3 — Observatory migration (lighter process)

> Mirrors the Phase 2 setup wizard plan: focused chunks, plan-only rigor, per-task commits, verify via `bun run check && bun run test:unit` + visual smoke via `make install`.

**Goal:** Bring `(observatory)/+layout.svelte` and the 11 child routes up to the same standard as `(config)/`. Tokens already migrated app-wide in `ee06cc59`; remaining work is component decomposition + per-page audit.

**Companion files:**
- Guideline: `docs/design/frontend-svelte-guidelines.md`
- Mockup: `docs/mockups/Sensei/lib/observatory.jsx`
- Current layout: `app/src/routes/(observatory)/+layout.svelte` (211 lines, sidebar inline)

## State assessment

The observatory layout has one large chunk of inline chrome that should be its own component:

- The **sidebar** (lines 58–180-ish) — collapsed/expanded states, NAV_ITEMS, BOTTOM_ITEMS, dynamic projects list, `先` wordmark, daemon port footer. ~120 lines.

The rest of the layout is composition + `appState.load()` lifecycle. Each child page is its own concern; some already use `PageHeader` from `$lib/components/` and may not need much.

## Tasks

### Task 1 — Extract `ObservatorySidebar`

Pull the entire `<aside>...</aside>` into `app/src/routes/(observatory)/ObservatorySidebar.svelte`. Props:
- `projects: SidebarProject[]`
- `port: number`
- `currentPath: string` (for `isActive` computation, OR keep that logic in the sidebar with `page` from `$app/state`)

Internally:
- Owns the `sidebarCollapsed` state (presentational toggle, not app-wide)
- Reuses `Wordmark` from `$lib/components/` for the kanji `先` mark (currently inline `<span class="kanji text-xl text-accent">先</span>` — but the mockup uses the full `先生 Sensei`; check which is intended)
- Reuses `Eyebrow` for "Observatory" / "Projects" section labels

Layout reduces from 211 → ~70 lines (composition + lifecycle).

Commit: `refactor(app): extract ObservatorySidebar from (observatory) layout`

### Task 2 — Per-page audit

Quick sweep across 11 routes for:
- Stale custom classes (e.g., `divide-paper-edge` now works via tokens.css shim — verify nothing's broken)
- `text-` colors that should be named tokens (already done in `ee06cc59`)
- Page header pattern — use the existing `PageHeader` primitive consistently
- Inline `<style>` blocks that should be replaced by utility classes

Files to audit:
| Path | Notes |
|---|---|
| `(observatory)/+page.svelte` | 239 lines — home / Today |
| `(observatory)/RecentSessions.svelte` | 74 lines — used by home |
| `(observatory)/insights/+page.svelte` | |
| `(observatory)/learnings/+page.svelte` + ArchiveList / ActiveList / TriageList / MemoryDetail | |
| `(observatory)/projects/+page.svelte` + `[id]/+page.svelte` | |
| `(observatory)/libraries/+page.svelte` | |
| `(observatory)/instruments/+page.svelte` | |
| `(observatory)/sessions/+page.svelte` | |
| `(observatory)/settings/+page.svelte` | |
| `(observatory)/help/+page.svelte` | |

Single bundled commit if changes are minor; per-page if substantive.

Commit: `refactor(app): per-page observatory cleanup`

### Task 3 — Visual smoke + push

`make install`, walk through each page in light + dark, compare against the mockup. Any deviations get one-line fixes.

## Verification gates

- `bun run check && bun run test:unit` — zero errors, 545+ tests green
- `(observatory)/+layout.svelte` line count drops to ~70 lines
- No new z-scale, no new oklch in `(observatory)/`
- Existing observatory e2e tests (if any) still pass

## Out of scope

- Project window (`(project)/`) — separate sub-app, separate phase
- Daemon-side issues (project classification #8, sessions #5, libraries #4, etc.) — out of UI scope
