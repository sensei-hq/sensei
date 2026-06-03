# Phase 2 — Setup wizard migration (lighter process)

> **For executors:** Lightweight plan, not a full spec. Each task is a focused commit; verification = check + tests + visual eyeball via `make app-dev`. Defers TDD-by-default to per-task judgment since most work is structural decomposition, not new logic.

**Goal:** Bring the setup wizard up to the same standard as `/health` did Phase 1 — pure-template stages, decomposed shared chrome, state-owned derivations, mockup-faithful layout — without rebuilding what already works.

**Tech stack:** SvelteKit + Tauri, Svelte 5 runes, Rokkit named tokens (already migrated app-wide in `ee06cc59`), `wizard-state.svelte.ts` for state.

**Companion files:**
- Spec / guideline (read these): `docs/design/frontend-svelte-guidelines.md`
- Mockup (visual target): `docs/mockups/Sensei/lib/setup-wizard.jsx` — `SetupWizard` (line 64), `WizRail` (line 193)
- Current layout: `app/src/routes/(config)/+layout.svelte` — 250+ lines, all chrome inline

## State assessment

The post-Phase-1 token sweep (`ee06cc59`) already migrated every z-scale class in the wizard. What's left:

- The `(config)/+layout.svelte` file inlines three reusable chunks: **the rail sidebar** (lines 93–156), **the stage header** (lines 161–179), **the bottom nav with progress bar + back/next** (lines 204–252). Each should be its own component.
- The wizard kanji wordmark in the rail (`先生 Sensei`, lines 96–99) is a copy of the existing `Wordmark` primitive that ships in `$lib/components/`. Reuse it.
- The kanji-eyebrow-title-description block in the stage header (lines 164–178) is exactly the `KanjiHeader` pattern from Phase 1 — also reusable.
- Per-stage pages mostly look fine after the token sweep but may have stale custom classes (e.g., `btn-primary` instead of `btn-solid`) — quick audit per stage.

## Out of scope

- New stages or removed stages — keep the existing 10.
- Wizard navigation logic, `wizardState`'s API, or daemon-side wiring.
- Phase 3 (Observatory).

---

## Task 1 — Promote Phase 1 screen-local primitives to `$lib/components/`

Per user decision (promote-now): move `StatusIndicator` and `GateRow` from `(health)/health/` to `$lib/components/`. They're not used by Phase 2 directly but should be promoted so the barrel + Phase 3's gate-like lists can adopt them without an extra move later.

**Files:**
- Move: `app/src/routes/(health)/health/StatusIndicator.svelte` → `app/src/lib/components/StatusIndicator.svelte`
- Move: harness + spec accordingly
- Move: `app/src/routes/(health)/health/GateRow.svelte` → `app/src/lib/components/GateRow.svelte` + harness + spec
- Update: `app/src/lib/components/index.ts` (add the two exports)
- Update: imports in `HealthView.svelte` and `(health)/health/GateRow.svelte` (which currently does `import StatusIndicator from './StatusIndicator.svelte'` → `from '$lib/components'`)

Verify: `bun run check && bun run test:unit` green.

Commit: `refactor(app): promote StatusIndicator + GateRow to $lib/components/`

---

## Task 2 — Extract `WizardRail` to `app/src/routes/(config)/`

Pull the rail sidebar (current layout lines 93–156) into `WizardRail.svelte`. Internally uses the `Wordmark` primitive for the kanji 先生 mark and `StatusDot` for the services bottom indicator. Props: `stages: WizardStage[]`, `currentIdx: number`.

**Files:**
- Create: `app/src/routes/(config)/WizardRail.svelte`
- Modify: `app/src/routes/(config)/+layout.svelte` — replace the `<aside>...</aside>` block with `<WizardRail {stages} {currentIdx} />`

The rail's interactive bits (click → `goto(s.path)`) can stay inside the rail since they're presentational hooks; the layout just passes data.

Per-stage active styling stays via class:active / class:done — those bindings move with the rail.

Commit: `refactor(app): extract WizardRail from (config) layout`

---

## Task 3 — Extract `StageHeader` to `app/src/routes/(config)/`

Pull the kanji+eyebrow+title+description block (current layout lines 161–179). The pattern is **almost** the existing `KanjiHeader` primitive but the wizard's header has a different style (bigger display headline, accent-tinted-faint kanji, "Step" eyebrow). Two options:

- **A** (recommended): create a screen-local `StageHeader.svelte` that adapts the `KanjiHeader` primitive — composes `KanjiHeader` with the wizard-specific title slot + a `data-stage-header` attribute for e2e.
- **B**: render the markup inline as a screen-local `StageHeader.svelte` if `KanjiHeader` doesn't fit cleanly. Keep the file small.

The header is hidden on the `welcome` stage (`stage?.id !== "welcome"`), so the parent controls visibility.

Props: `stage: WizardStage`.

**Files:**
- Create: `app/src/routes/(config)/StageHeader.svelte`
- Modify: `+layout.svelte` — replace inline block with `{#if stage?.id !== 'welcome'}<StageHeader {stage} />{/if}`

Commit: `refactor(app): extract StageHeader from (config) layout`

---

## Task 4 — Extract `StageNav` (bottom progress + back/next) to `app/src/routes/(config)/`

Pull the bottom nav (current layout lines 204–252). Props: `currentIdx: number`, `total: number`, `stage: WizardStage`, `canAdvance: boolean`, `committing: boolean`, `isFirst: boolean`, `isLast: boolean`, `onBack: () => void`, `onNext: () => void`.

The label-on-next-button special cases (`assistants` → "Configure & Continue →", last → "Enter observatory →", etc.) stay inside the component as a status-to-label lookup — that's the "rendering rule" allowed pattern.

Replace the inline `btn-primary` class with the canonical `btn-solid` utility (already in tokens.css). Verify visually that the Continue button looks the same.

**Files:**
- Create: `app/src/routes/(config)/StageNav.svelte`
- Modify: `+layout.svelte` — replace inline block with `<StageNav ... />`

Commit: `refactor(app): extract StageNav from (config) layout`

---

## Task 5 — Audit + fix per-stage pages

For each setup stage page, do a quick read and clean up obvious deviations from the mockup or the guideline. Examples expected (will confirm during the read):

- Stale `mono` class → `font-mono`
- Inline `style="..."` attributes that should be utility classes
- `text-` colors that snuck in (literals) → named tokens
- Custom button classes that should be `btn-solid` / `btn-outline`
- Pages where the header is `<header>` but should match `StageHeader` pattern (probably none — welcome is the only exception and the layout suppresses the header for it)

**Files to audit (each gets a small commit if changes are needed; skip if clean):**

| Stage | Path |
|---|---|
| welcome | `(config)/setup/welcome/+page.svelte` |
| roots | `(config)/setup/roots/+page.svelte` |
| preferences | `(config)/setup/preferences/+page.svelte` |
| assistants | `(config)/setup/assistants/+page.svelte` |
| projects | `(config)/setup/projects/+page.svelte` |
| libraries | `(config)/setup/libraries/+page.svelte` |
| instruments | `(config)/setup/instruments/+page.svelte` |
| inference | `(config)/setup/inference/+page.svelte` |
| scan | `(config)/setup/scan/+page.svelte` |
| done | `(config)/setup/done/+page.svelte` |

Single sweep commit if changes are minor; per-page commits if substantive.

Commit (bundle): `refactor(app): per-stage wizard page cleanup`

---

## Task 6 — Visual smoke + e2e + push

1. `make app-dev` (or `make install`) — walk through every stage manually, comparing against `setup-wizard.jsx`.
2. `bun run test:e2e -- --grep 'setup-wizard|wizard-done'` — existing wizard e2e tests should still pass.
3. `bun run check && bun run test:unit` — green.
4. Push to `develop`.

Commit: any visual-smoke fixes inline.

---

## Verification gates (each task)

- `bun run check && bun run test:unit` — zero errors, 545+ tests green
- The `(config)/+layout.svelte` line count drops noticeably across tasks 2–4 (target: from 250+ → ~80 lines as just composition)
- No new `oklch(var(--color-*-z*))` patterns introduced
- No new z-scale utility classes introduced

## Open decisions deferred to execution

- `StageHeader` — reuse `KanjiHeader` primitive (option A) vs inline (option B). Decide during Task 3 after seeing how cleanly `KanjiHeader`'s title-as-snippet maps to the wizard's bigger display headline.
- Per-stage commits vs single bundled refactor commit for Task 5 — depends on what the audit surfaces.
