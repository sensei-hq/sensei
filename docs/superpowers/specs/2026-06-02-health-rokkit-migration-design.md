# Phase 1 — `/health` migration to Rokkit named tokens

Date: 2026-06-02
Scope: `sensei/app/src/routes/(health)/health/**`
Companion: `sensei/docs/design/frontend-svelte-guidelines.md` (read first)

## Problem

The `/health` screen renders correctly but in a way that blocks the rest of the
UI migration:

1. Tokens are deprecated z-scale (`bg-surface-z0`, `text-surface-z9`,
   `text-primary-z5`) plus inline `oklch(var(--color-*) / 1)` strings inside
   `<style>` blocks and `style="…"` attributes. Token drift is invisible to
   review.
2. Components carry data derivations (`components.filter(c => c.status === 'ready').length`,
   `components.find(...)`) that belong in the state file. Tests at the
   component layer end up requiring fixture data and mocking.
3. Layout drifts from the canonical mockup
   (`docs/mockups/Sensei/lib/bootstrap-splash.jsx`). The mockup defines a
   two-column layout with kanji-numeral gate rows, per-gate zen prose, a
   mono-uppercase StatusIndicator (label + 20px disc), and an all-green state
   that hides the right column and reveals a `先生` logo watermark — most of
   which the live screen lacks.
4. Repeated inline patterns (status disc + spinner appears in `Hero.svelte` and
   `Ledger.svelte` with two separate `<style>` blocks; kanji+eyebrow+title
   pattern is open-coded in Header and Hero) — no shared primitives.

Migrating now sets the pattern for every subsequent phase (setup wizard,
observatory, project pages) and validates the [frontend guidelines](../../design/frontend-svelte-guidelines.md).

## Goals

- Zero z-scale, zero inline OKLCH, zero `<style>` color blocks in `/health`.
- `HealthView` matches `bootstrap-splash.jsx` layout in both light and dark mode.
- 8 presentational primitives extracted; component code is purely template.
- `health-state.svelte.ts` owns every derivation and every display string.
- Vitest snapshots per primitive + state spec — CI gate.
- Playwright visual baselines per primitive + composed screen — local dev aid,
  gitignored.

## Non-goals (this phase)

- `/health/upgrade` and `/health/logs` screens (Phase 2).
- Setup wizard, observatory, project pages (later phases).
- Changes to `health-transport.ts` or the Rust wire shape, except possibly
  adding `Remedy.kind` if it's not already there (see §4).
- Copy rewrites unrelated to token migration (existing user-facing strings
  stay unless the mockup specifies different content).

## Design

### 1. Config change — `rokkit.config.js`

Two changes: **skin re-alignment** + **overrides for shade tuning**.

#### 1a. Skin re-alignment

The design system says `--primary: var(--ink)` (CTA buttons are ink-on-paper)
and `--accent: var(--shu-500)` (vermillion, rationed). The current config has
this inverted — `primary: 'shu'` (vermillion) and `accent: 'fuji'` (wisteria,
unused). Re-align:

```js
skin: {
  surface:   { light: 'kami', dark: 'sumi' },
  ink:       { light: 'kami', dark: 'sumi' },
  primary:   { light: 'kami', dark: 'sumi' },  // ← was 'shu'. Primary = ink-colored (see overrides)
  accent:    'shu',                             // ← was 'fuji'. Vermillion accent — design system
  secondary: 'murasaki',
  success:   'hisui',
  warning:   'kohaku',
  danger:    'beni',
  error:     'beni',
  info:      'ai',
},
```

(`fuji` palette stays defined in `sumi-palette.js` for future use but is no
longer skin-assigned.)

#### 1b. Overrides

Replace the existing one-entry `overrides:` block with the full lock-down.
Rationale: the canonical defaults pick wrong shades for the kami (text starts
at shade 600, not 500) and sumi (two-pole) palettes, and `primary` needs to
resolve to ink color per the design system — see
[§1.2 of the guidelines](../../design/frontend-svelte-guidelines.md#12-tuning-tokens--overrides-only)
and inline comments in `sumi-palette.js`.

```js
overrides: {
  // Surface — kami (warm paper) light, sumi (ink) dark
  paper:        { light: 'kami.100', dark: 'sumi.50'  },
  'paper-soft': { light: 'kami.200', dark: 'sumi.100' },
  'paper-mute': { light: 'kami.300', dark: 'sumi.200' },
  'paper-edge': { light: 'kami.400', dark: 'sumi.100' },  // etched hairline in dark

  // Ink — text-zone shades only (sumi.600-900 are the text half of the two-pole palette)
  ink:          { light: 'kami.900', dark: 'sumi.900' },
  'ink-soft':   { light: 'kami.700', dark: 'sumi.800' },
  'ink-mute':   { light: 'kami.500', dark: 'sumi.700' },
  'ink-faint':  { light: 'kami.300', dark: 'sumi.600' },

  // Primary — ink-colored CTA fill (design system: --primary: var(--ink))
  primary:      { light: 'kami.900', dark: 'sumi.900' },
  'on-primary': { light: 'kami.100', dark: 'sumi.50'  },  // paper text on ink button

  // Accent + status — lighten for legibility in dark mode (shade 400 vs 500)
  accent:       { light: 'shu.500',    dark: 'shu.400'    },
  success:      { light: 'hisui.500',  dark: 'hisui.400'  },
  warning:      { light: 'kohaku.500', dark: 'kohaku.400' },
  danger:       { light: 'beni.500',   dark: 'beni.400'   },
  info:         { light: 'ai.500',     dark: 'ai.400'     },
},
```

`*-soft` companions (`accent-soft`, `success-soft`, etc.) automatically resolve
to shade 100 of the relevant palette per the canonical `NAMED_TOKEN_SHADE_MAP`
in `@rokkit/core` — since the skin now maps `accent: 'shu'`, `bg-accent-soft`
resolves to `shu.100` automatically (vermillion-tinted background). No override
needed for the soft variants.

**Nothing else changes** in `rokkit.config.js`, `uno.config.js`, or `app.css`.
No `shortcuts:`, no `rules:`. Rokkit's preset auto-emits the utility shortcuts
for every named token.

### 2. State extensions — `health-state.svelte.ts`

Add the derivations and display content currently scattered across `Hero`,
`Ledger`, `Header`, and `HealthView`:

```ts
class HealthState {
  // … existing: status, packageManager, components, remedy, version, platform, isOk, needsAction

  // Collection derivations (move FROM HealthView, Hero, Ledger)
  get gates(): Component[]      { return [this.packageManager, ...this.components]; }
  get total(): number           { return this.gates.length; }
  get readyCount(): number      { return this.gates.filter(g => g.status === 'ready').length; }
  get activeLabel(): string     {
    return this.gates.find(g => g.status === 'installing' || g.status === 'checking')?.label ?? '';
  }
  get firstBlockedIdx(): number { return this.gates.findIndex(g => g.status === 'failed'); }

  // Display content per HealthStatus (move FROM Header, mockup-faithful)
  get display(): {
    eyebrow:      string;
    headlinePre:  string;            // "The foundation"
    headlineKey:  string;            // "holds." | "missing." | "foundation…"
    headlineTone: 'success' | 'accent' | 'ink-mute';   // utility-tone for the key word
    subCopy:      string;
  } { /* status switch — see §3 for copy strings */ }

  // Action — retry a single gate (new, used by Remedy "retry" button per mockup)
  retry(id: ComponentId | PackageManagerId): void { /* re-fires check via transport */ }
}
```

Tests in `health-state.spec.svelte.ts` extend to cover every new getter and
the `retry()` action against a transport mock.

### 3. Screen rebuild — `HealthView.svelte`

Match `docs/mockups/Sensei/lib/bootstrap-splash.jsx` (the current/canonical
mockup; `bootstrap.jsx` is the older variant and **not** the target). The
current screen is already structurally close — the migration is finer-grained
than a full rebuild.

#### Layout

Two-column when checks are showing (`HealthStatus !== 'ok'`), single column
when all-green (right column hidden, watermark revealed):

```
HealthStatus !== 'ok'  (probing / resolving / needs-action)

┌─────────────────────────────────────┬───────────────────────────────────┐
│  先生 Sensei                        │  支  FOUNDATION                   │
│                                     │      Checking components       ◯ │
│  STARTING                           │  ─────────────────────────────── │
│  Checking the foundation.           │  一 Homebrew · package manager   │
│  A quick health check before…       │     The gardener who tends…  ✓ READY  ◯ │
│                                     │  二 PostgreSQL · storage @16     │
│  [Remedy panel — needs-action only] │     A still pond…           ⟳ CHECKING │
│  pre script                         │  三 Ollama · local models        │
│  [Copy] [I've run it · re-check]    │     A mind that thinks…    · WAITING   │
│                                     │  四 Sensei components            │
│                                     │  五 Database                     │
│                                     │  六 Daemon                       │
│  sensei 0.1.0 · macOS 14.4 · arm64  │                          [Continue →] │
└─────────────────────────────────────┴───────────────────────────────────┘

HealthStatus === 'ok'  (right column hidden, watermark revealed)

┌─────────────────────────────────────────────────────────────────────────┐
│  先生 Sensei              (large)                  ╱─╲                  │
│                                                   │   │  ← logo         │
│  READY                                            │ S │   watermark     │
│  The foundation holds.                            │   │   (text-ink/10%)│
│  Opening the observatory.                          ╲─╱                  │
│                                                                         │
│  ─── opening…                                                           │
│                                                                         │
│  sensei 0.1.0 · macOS 14.4 · arm64                                      │
└─────────────────────────────────────────────────────────────────────────┘
```

#### Header copy per `HealthStatus` (per `splashCopyFor()` in mockup)

| `HealthStatus`  | Eyebrow             | Headline                                  | Sub copy                                                                                                                            |
| --------------- | ------------------- | ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `checking`      | "starting"          | "Checking the `foundation.`" (accent)     | "A quick health check before opening the observatory."                                                                              |
| `resolving`     | "setting up"        | "Putting the room `in order.`" (accent)   | "Running `brew bundle` with the manifest from `sensei-hq/homebrew-tap`. No input needed." (mono spans on `brew bundle` and repo)    |
| `needs-action`  | "needs your hand"   | "One last `step.`" (accent)               | "Homebrew isn't here yet. Run the script — it installs Homebrew, then everything else."                                              |
| `ok`            | "ready"             | "The foundation `holds.`" (success)       | "Homebrew, Postgres, Ollama, sensei components, database, and the daemon are all present. Opening the observatory."                  |

The mockup's `auto-fixing` headline (currently the live app says "Setting up
your foundation.") becomes "Putting the room in order." — that's the only
copy change from current. All other text matches.

All copy lives in `healthState.display`; `Header.svelte` reads strings only.

#### Right column — KanjiHeader title per overall state

The KanjiHeader title (right-column top, next to the 32px StatusDisc) is
composed in `healthState.display.heroTitle`. It uses the active gate's
`installingVerb` (already on the wire — `health-types.ts:25–28`) so the verb
matches the actual operation: "Installing · 3/6", "Starting · 3/6",
"Configuring · 4/6", "Creating · 5/6" — whichever verb the Rust
`DependencySpec` ships.

| Condition                                    | `display.heroTitle`                                       |
| -------------------------------------------- | --------------------------------------------------------- |
| `status === 'ok'`                            | "The foundation holds"                                    |
| `status === 'needs-action'`                  | "Needs your hand"                                         |
| `status === 'resolving'` (active gate G)     | `${capitalize(G.installingVerb)} · ${readyCount}/${total}` |
| `status === 'checking'` (active gate G)      | `${capitalize(G.installingVerb)} · ${readyCount}/${total}` (or "Checking components" when no active gate) |
| otherwise                                    | "Checking components"                                     |

This is purely presentation — the wire shape and the
`installingVerb` field are unchanged. The component reads
`state.display.heroTitle` as a string; the verb composition lives in state.

#### All-green watermark + transition

The mockup shows a `先生` logo watermark (10% opacity, mid-right) and a small
"opening…" indicator with a tickling underline. Currently the screen
`goto('/')` immediately on `isOk`. Two options:

- **A** (recommended) — add a brief 600ms hold before navigation so the
  watermark + "opening…" treatment is visible. Closer to mockup, friendlier
  hand-off.
- **B** — keep instant navigation; the watermark only flashes for a render.

Implementation defers this to step 6; either is acceptable.

### 4. Primitives — 6 components

| Component          | Location                         | Props                                                                  | Replaces                                                                  |
| ------------------ | -------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `Wordmark`         | `$lib/components/`               | `size?: 'sm' \| 'md' \| 'lg'`                                          | Inline `先生 Sensei` markup in `Header.svelte`                              |
| `KanjiHeader`      | `$lib/components/`               | `kanji: string`, `eyebrow: string`, `title: Snippet`, `right?: Snippet` | Hero's kanji+eyebrow+text+disc pattern (right slot holds `StatusDisc`)    |
| `StatusDisc`       | `$lib/components/`               | `status: ComponentStatus \| HealthStatus`, `size?: number`, `tone?: 'accent' \| 'success' \| 'ink'` | Hero's `.hero-disc` (32px) + Ledger row's trailing disc (20px); spinner / check / `?` glyph cycling |
| `StatusIndicator`  | `(health)/health/`               | `status: ComponentStatus`, `label?: string`                            | Per-row mono-label + 20px `StatusDisc` (Ledger badge + disc combined)    |
| `GateRow`          | `(health)/health/`               | `gate: Component`, `numeral: string` (`一` `二` `三` `四` `五` `六`)                                | Ledger `<li>` row markup; adds kanji numeral + zen-italic `description` line per splash mockup |
| `Spinner`          | `$lib/components/`               | `size?: number`, `tone?: 'accent' \| 'success' \| 'ink'`               | Two duplicated `<style>` blocks (Hero's `.hero-spinner`, Ledger's `.ledger-spinner`) |

Existing primitives (`Kanji`, `Eyebrow`, `StatusDot`, `PageHeader`) remain.
`Hero.svelte` and `Ledger.svelte` are **deleted** — composition lives in
`HealthView` (right-column wires `KanjiHeader` + `<GateRow>` per `state.gates` +
Continue button). `Remedy.svelte` keeps its current panel framing (kanji 手
header + intro + script + buttons); only tokens and the verify-button label
change (see §6).

### 5. Status → tone mapping (inside `StatusIndicator`)

Wire-typed against `ComponentStatus`. The splash mockup renders this as a
mono-uppercase label inline with the 20px `StatusDisc` — **not** a pill with
tinted background. The `bg-*-soft` chip treatment from bootstrap.jsx is dropped.

| `ComponentStatus` | Label                                | Disc / glyph                  | Label color       |
| ----------------- | ------------------------------------ | ----------------------------- | ----------------- |
| `pending`         | (no label)                           | 20px disc, `border-ink-faint` | n/a               |
| `checking`        | "checking"                           | 20px disc, accent border + `Spinner` | `text-accent`     |
| `installing`      | `component.installingVerb` from wire | 20px disc, accent border + `Spinner` | `text-accent`     |
| `ready`           | (no label)                           | 20px disc, success border + check | `text-success`    |
| `failed`          | "blocked"                            | 20px disc, accent border + `?` glyph | `text-accent`     |

`installingVerb` is already on `Component` (`health-types.ts:25–28`). When
caller omits `label`, `StatusIndicator` derives from status (`'checking'`,
`'blocked'`, `gate.installingVerb`, etc.) — keeps callers thinner.

Label typography: `font-mono text-xs uppercase tracking-wide`.

### 6. Remedy — keep panel shell, migrate tokens

`Remedy.svelte` keeps its current shape — kanji 手 header, "Run this in your
terminal" lead-in, `remedy.message`, optional "Learn more" link, `<pre>` for
the script, footer with Copy + verify buttons. The framing reads cleaner than
the splash mockup's bare-inline treatment and gives the message room to
breathe.

Migration is class-only:
- `bg-surface-z1` → `bg-paper-soft` (panel bg)
- `border-primary-z5/30` → `border-accent/30` (panel border tint)
- `border-surface-z2` → `border-paper-edge` (header / footer dividers)
- `text-surface-z9` → `text-ink`
- `text-surface-z7` → `text-ink-mute` (message, "Learn more" link)
- `bg-surface-z3` → `bg-paper-mute` (script `<pre>` bg)
- Inline `style="color: oklch(var(--color-primary-z5) / 1); border-color: oklch(var(--color-primary-z5) / 0.4);"`
  on the verify button → utility classes `text-accent border-accent/40`.

Copy change (matches splash mockup):
- Verify button label: **"I've run it · verify"** → **"I've run it · re-check"**.

The wire `Remedy` interface stays unchanged — no `kind` discriminator. One
remedy per overall `needs-action` state, surfaced from `state.remedy`.

### 7. Test layout

Follow the existing convention from `Eyebrow.svelte` / `Eyebrow.harness.svelte` /
`Eyebrow.spec.svelte.ts` — harnesses are mountable wrappers imported by spec
files, not exposed as routes.

```
src/lib/components/
  Wordmark.svelte
  Wordmark.harness.svelte
  Wordmark.spec.svelte.ts              ← imports harness, Vitest browser mount + snapshot
  __snapshots__/Wordmark.spec.svelte.ts.snap

  KanjiHeader.svelte
  KanjiHeader.harness.svelte
  KanjiHeader.spec.svelte.ts
  __snapshots__/

  StatusDisc.svelte
  StatusDisc.harness.svelte
  StatusDisc.spec.svelte.ts
  __snapshots__/

  Spinner.svelte
  Spinner.harness.svelte
  Spinner.spec.svelte.ts
  __snapshots__/

src/routes/(health)/health/
  +page.svelte
  HealthView.svelte
  Header.svelte
  Remedy.svelte                        ← slimmed per §6
  StatusIndicator.svelte
  StatusIndicator.harness.svelte
  StatusIndicator.spec.svelte.ts
  GateRow.svelte
  GateRow.harness.svelte
  GateRow.spec.svelte.ts
  __snapshots__/                       ← all snapshots checked in

src/lib/
  health-state.spec.svelte.ts          ← extended for new derivations + retry()

e2e/tests/
  boot-flow.spec.ts                    ← existing, unchanged
```

No new e2e files. No `(dev)/harness/*` route group. No Playwright visual layer.

### 8. Verification

Per checkpoint:
1. `bun run test` — Vitest snapshots + state spec green.
2. `bun run lint && bun run check` — zero errors.
3. `bun run test:e2e -- --grep "boot-flow"` — behaviour intact.
4. Local dev (`make app-dev`) — drive each `HealthStatus` through the wire
   (or via the test seam on `healthState`), light + dark, eyeball against
   `docs/mockups/Sensei/lib/bootstrap-splash.jsx` rendered in the design
   canvas. Confirm visual parity, especially:
   - Kanji numerals 一-六 in Ledger rows
   - Zen-italic `description` lines under each gate name
   - StatusIndicator mono-label + 20px disc treatment
   - Two-column collapse to single-column with logo watermark on `ok`

### 9. Commit cadence (Phase 1)

| # | Branch checkpoint                                                                                          | Vitest gate | e2e gate |
| - | ---------------------------------------------------------------------------------------------------------- | ----------- | -------- |
| 1 | `rokkit.config.js` — skin re-alignment + overrides block                                                   | n/a         | yes      |
| 2 | `Spinner` + `StatusDisc` (+ harnesses + spec)                                                              | yes         | yes      |
| 3 | `Wordmark` + `KanjiHeader` (+ harnesses + spec)                                                            | yes         | yes      |
| 4 | `StatusIndicator` + `GateRow` (+ harnesses + spec)                                                         | yes         | yes      |
| 5 | `health-state.svelte.ts` extensions (`gates`, `total`, `readyCount`, `activeLabel`, `display`, `retry()`) + spec | yes         | yes      |
| 6 | `Header.svelte` rewrite + `Remedy.svelte` token swap + button label + `HealthView.svelte` rebuild; delete `Hero`, `Ledger`; `(health)/+layout.svelte` token swap | yes | yes |

Each commit independently revertible. Lands on `develop`; merges to `main` when
Phase 1 acceptance criteria are met.

## Acceptance criteria

A reviewer can confirm Phase 1 is complete by:

1. Running `rg "z[0-9]+|oklch\(var" sensei/app/src/routes/\(health\)/health/` and getting zero results.
2. Running `rg "<style>" sensei/app/src/routes/\(health\)/health/` and finding only animations/geometry, no color.
3. Running `bun run test` — snapshots for all 6 primitives green; extended `health-state.spec.svelte.ts` green.
4. Running `bun run test:e2e -- --grep boot-flow` — unchanged behaviour.
5. Opening `/health` in dev (`make app-dev`), driving each `HealthStatus`,
   light and dark, and confirming visual parity with
   `docs/mockups/Sensei/lib/bootstrap-splash.jsx`.

## Out-of-scope items flagged for follow-up

- **Phase 2**: `/health/upgrade`, `/health/logs` migration. Same pattern.
- **Phase 3+**: setup wizard, observatory, project pages.
- **Cross-cutting**: `Wordmark`, `KanjiHeader`, `Spinner`, `StatusDisc` are
  likely first-reuse candidates in Phase 2/3. `StatusIndicator` and `GateRow`
  may stay screen-local — promote at first cross-screen reuse.

## Open questions

- Existing `paper-edge` override sets dark to `sumi.100` (etched). The design
  system CSS specifies `oklch(0.300 0.010 50)` (~ `sumi.250`-ish). Visual
  review during step 1 will confirm whether to keep `sumi.100` or shift to
  `sumi.300` for a softer hairline in dark mode.
- All-green watermark + 600 ms hold (§3): worth adopting, or stay with instant
  `goto('/')`. Decision deferred to step 6 with visual eyeball.
