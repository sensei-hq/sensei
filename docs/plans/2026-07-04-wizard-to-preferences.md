---
name: 2026-07-04 — Wizard → Preferences arch change
issue: TBD
epic: —
mockups:
  - docs/mockups/Sensei/lib/observatory.jsx
  - docs/mockups/Sensei/lib/settings.jsx
---

# Wizard → Preferences arch change

Split the 11-stage setup wizard into two surfaces:

- **First-run setup** (thin) — 5 stages a fresh install must walk before the
  observatory has any data.
- **Preferences** (persistent) — an editable Settings surface for everything
  that is not a one-time bootstrap step. Reachable any time from the
  observatory rail.

## Why

The wizard has drifted into "settings that also happen to run once". Users
who want to change model assignments, add a library, or install a new
instrument are pushed through a 11-step gauntlet. The first-run cost is
also too high: 11 steps to see the observatory. This lands the split we
have been asking users to route around.

## Decisions (recorded at plan time)

1. **Location**: extend `(observatory)/settings/` with new sub-routes.
   No new route group. Reuses ObservatorySidebar for the outer chrome.
2. **First-run keeps**: Welcome, Assistants, Roots, Scan (+ Done ceremony).
   Preferences (name/tone), Libraries, Instruments, Inference, Assignments,
   Projects **move to Settings** — they were setup-time only by
   convention, not by necessity.
3. **Shape inside Settings**: rokkit grouped list rail on the left, content
   pane on the right. Matches the observatory pattern and the rokkit
   1.3.1 sidebar rehab already shipped.

## Surface diff

```
Before (wizard, 11 stages)               After
─────────────────────────────            ─────────────────────────────
welcome                                  First-run wizard (thinner):
preferences   ← moves to Settings          welcome
assistants                                 assistants
roots                                      roots
scan                                       scan
projects      ← moves to Settings          done
libraries     ← moves to Settings
instruments   ← moves to Settings        Settings (rail):
inference     ← moves to Settings          General          (name/tone/prefs)
assignments   ← moves to Settings          Assistants       (re-detect + config)
done                                       Roots            (add/remove)
                                           Projects
                                           Libraries
                                           Instruments
                                           Inference        (chains)
                                           Assignments      (role → chain)
                                           Extensions       (already there)
```

Assistants + Roots appear in BOTH first-run AND Settings — same page, two
entry points. First-run walks them once; Settings lets them be re-visited.

## Slices

### Slice A — Settings rail + shell (day 1)

**Deliverables**
- `app/src/routes/(observatory)/settings/settings-nav.ts` — rail data (pure,
  unit-testable). Groups: "You" (General, Assistants), "Sources" (Roots,
  Projects, Libraries), "Reasoning" (Instruments, Inference, Assignments),
  "Extensions". No badges initially — leave hooks in for later.
- `app/src/routes/(observatory)/settings/+layout.svelte` — 220px rokkit
  grouped List rail + content slot. Reuses the same `List` config as
  `ObservatorySidebar` (see `_using rokkit style tokens_` memory rule).
- `app/src/routes/(observatory)/settings/+page.ts` — redirect `/settings` →
  `/settings/general` so the rail always has an active entry.

**Tests**
- Vitest: `settings-nav.spec.ts` — rail shape, `resolveActiveHref` behaviour.
- Vitest: `SettingsSidebar.spec.svelte.ts` — renders groups, active class.

### Slice B — Sub-route pages (day 2–3)

Extract each moved-stage page into a route-hosted component that both the
wizard and Settings can render.

**Deliverables**
- `app/src/lib/components/settings/` — new folder. One `.svelte` component
  per moved stage (`PreferencesForm.svelte`, `AssistantsSection.svelte`,
  `RootsSection.svelte`, `ProjectsSection.svelte`, `LibrariesSection.svelte`,
  `InstrumentsSection.svelte`, `InferenceSection.svelte`,
  `AssignmentsSection.svelte`). These wrap the current wizard-stage bodies
  minus the WizardRail/Next-button chrome.
- `app/src/routes/(observatory)/settings/general/+page.svelte`,
  `.../assistants/`, `.../roots/`, `.../projects/`, `.../libraries/`,
  `.../instruments/`, `.../inference/`, `.../assignments/` — each hosts the
  matching section component.
- `app/src/routes/(observatory)/settings/extensions/+page.svelte` — extract
  the existing `extensions` tab from `settings/+page.svelte`.
- Keep the existing `+page.svelte` as a thin redirect (Slice A already
  redirects at the `.ts` level; this removes the file).

**Tests**
- Reuse existing per-section specs. Add a route smoke test that mounts each
  sub-page and asserts a headline landmark.

### Slice C — Wizard shrink (day 4)

**Deliverables**
- `app/src/routes/(config)/stages.ts` — `STAGES` shrinks to
  `[welcome, assistants, roots, scan, done]`. Icons/copy stay; ordering
  matches the first-run script.
- Delete moved-stage folders under `app/src/routes/(config)/setup/`
  (preferences, projects, libraries, instruments, inference, assignments).
- `app/src/routes/(config)/setup/assistants/+page.svelte`,
  `.../roots/+page.svelte` — render the shared `AssistantsSection` /
  `RootsSection` components from Slice B (source of truth is one file, not
  two).
- `WizardRail.svelte` — no code change; the shorter STAGES array cascades.

**Tests**
- Existing `stages.spec.ts` (if present) — update expectations to the new
  count.
- Playwright: existing wizard e2e — trim to 5 stages, verify navigation.

### Slice D — Observatory rail entry (day 4, small)

**Deliverables**
- `app/src/routes/(observatory)/observatory-nav.ts` — rename the existing
  "Preferences" leaf link (currently pointing at `/settings`) to "Settings".
  Same route, better label. Kanji stays 調.

**Tests**
- `observatory-nav.spec.ts` — update the copy assertion.

### Slice E — E2E + spec updates (day 5)

**Deliverables**
- New Playwright spec `settings-rail.spec.ts`: rail lists 8 groups; each
  sub-route renders non-empty content; Assignments + Inference show live
  role/chain data (the flows that landed in `d67eb93e`).
- Update the observatory rail spec's active-highlighting assertions for the
  Settings leaf.
- Update wizard e2e to the 5-stage shape.

## Non-goals for this plan

- No new backend endpoints. Everything moved is already wired.
- No change to `wizard_done` config flag semantics — the daemon still
  gates first-run on it.
- No mockup redesign — the moved sections keep their current visuals; only
  the container changes.
- No Dōjō / governance surfaces. That is its own epic.
- No migration for users mid-setup. Users with `wizard_done=true` see the
  full Settings; users mid-way finish the (now shorter) wizard.

## Sequencing

A → B → C → D → E. Each slice compiles + passes checks on its own.
Slice A can merge before B/C — the rail renders even if the sub-routes are
placeholder pages. C waits for B so the wizard stages can reference the
extracted components.

## Success gate

- `/settings` in the observatory opens a rail with 8 entries in 4 groups.
- Every entry has a live sub-route with real data or an editable form.
- Fresh install walks Welcome → Assistants → Roots → Scan → Done in 5
  steps.
- Playwright: rail + wizard e2e both green.
- svelte-check: 0 errors.
- No regression in wizard-completed users (existing Settings behaviour
  preserved for General + Assistants + Inference + Extensions).
