# Frontend (SvelteKit + Tauri) — guidelines

Authoritative rules for the desktop app under `sensei/app/`. Every screen, every
refactor, every new feature follows this. When in doubt, this document overrides
inherited habits from the existing code.

Companion docs:
- Visual system reference — `sensei/docs/mockups/Zen-Sumi Design System/colors_and_type.css`
- Mockup library — `sensei/docs/mockups/Sensei/lib/*.jsx`
- Mockup conventions — `sensei/docs/mockups/Sensei/CLAUDE.md`
- Rokkit preset README — `node_modules/@rokkit/unocss/README.md`

---

## 1. Visual system

### 1.1 Tokens — canonical 24, named only

Rokkit emits a fixed 24-token semantic vocabulary. Prefer those names and extend as needed:

| Group   | Tokens                                                       |
| ------- | ------------------------------------------------------------ |
| Surface | `paper`, `paper-soft`, `paper-mute`, `paper-edge`            |
| Ink     | `ink`, `ink-mute`, `ink-soft`, `ink-faint`                   |
| Primary | `primary`, `on-primary`                                      |
| Accent  | `accent`, `accent-soft`                                      |
| Status  | `success` / `-soft`, `warning` / `-soft`, `danger` / `-soft`, `error` / `-soft`, `info` / `-soft` |
| Misc    | `focus-ring`, `shadow-tint`                                  |

`buildNamedShortcuts()` in `@rokkit/unocss` auto-emits utility shortcuts for every
name: `bg-{name}`, `text-{name}`, `border-{name}`, `border-t/b/l/r-{name}`,
`ring-{name}`, `fill-{name}`, `stroke-{name}`. So `bg-paper-mute`, `text-ink-faint`,
`border-paper-edge`, `bg-success-soft` are all live with **zero** config beyond the
preset.

**Banned:**
- `bg-surface-z*` / `text-surface-z*` / `border-surface-z*` and other z-scale
  utilities — these are back-compat aliases scheduled for removal.
- `text-primary-z5` / `bg-primary-z1` / `text-success-z5` — likewise. Use the
  named tokens (`text-accent`, `bg-accent-soft`, `text-success`).
- Avoid Hex / OKLCH / RGB color literals in `.svelte` files. Ever.
- Avoid `<style>` blocks that set color. Animations and one-off geometry only.
- Avoid `style="color: oklch(var(--color-*) / N)"` inline strings.
- Avoid `oklch(var(--color-*))` inside `<style>` blocks.

### 1.2 Tuning tokens — `overrides:` only

When the canonical default shade doesn't match the design intent (e.g., `paper`
defaults to shade 50, but the design system places it at shade 100), tune the
value in `rokkit.config.js` via `overrides:`. Two forms:

```js
// rokkit.config.js
overrides: {
  // Reserved name — tunes the value of a built-in named token.
  // The auto-shortcut (bg-paper, text-paper, …) is unchanged.
  paper:        { light: 'kami.100', dark: 'sumi.50'  },
  'paper-soft': { light: 'kami.200', dark: 'sumi.100' },

  // Non-reserved name — emits a brand-new CSS var AND auto-shortcuts
  // (bg-script-bg, text-script-bg, border-script-bg, …) when the
  // resolved value looks like a color.
  'script-bg':  { light: 'kami.50', dark: 'sumi.50' },
},
```

`{ light, dark }` produces the right value per mode automatically — no `dark:`
prefixes in component code.

**Never** use `shortcuts:` or `rules:` in `uno.config.js` for color/spacing
tokens. Those bypass the token system and break the guarantee that one config
change propagates everywhere.

### 1.3 Typography — fixed scale, named families

| Family    | Class         | Use                                |
| --------- | ------------- | ---------------------------------- |
| Display   | `font-display`| Headlines, hero text (Fraunces)    |
| Body / UI | `font-body`   | Default. Implicit on `<body>`.     |
| Mono      | `font-mono`   | Code, paths, version numbers       |
| Kanji     | `font-kanji`  | Standalone CJK glyphs (Yu Mincho)  |

Sizes — eight stops. Never literal `px` / `rem`:

| Class       | Size  | Use                              |
| ----------- | ----- | -------------------------------- |
| `text-xs`   | 11px  | Labels, micro-meta               |
| `text-sm`   | 13px  | Secondary UI                     |
| `text-base` | 15px  | Default body                     |
| `text-lg`   | 17px  | Lead, h3                         |
| `text-xl`   | 22px  | h2, section titles               |
| `text-2xl`  | 28px  | h1, page titles                  |
| `text-3xl`  | 40px  | Display body                     |
| `text-4xl`  | 56px  | Hero                             |

Weights: `font-light` / `font-normal` / `font-medium` / `font-semibold` only.

### 1.4 Spacing — 4px grid via Tailwind names

Use `gap-N`, `p[xy]-N`, `m[xy]-N` with `N ∈ {0,1,2,3,4,5,6,8,10,12,16}`. Never
literal `px`. If a value isn't on the scale, the design is wrong, not the system.

Density:
- Compact (data tables, sidebars): base `gap-1`/`gap-2`, padding `p-2`/`p-3`
- Comfortable (cards, forms): base `gap-3`/`gap-4`, padding `p-4`/`p-6`
- Relaxed (marketing, landing): `gap-6`/`gap-8`, padding `p-8`/`p-12`

### 1.5 Radius / shape

`rounded-sm` (4px) · `rounded` (6px) · `rounded-lg` (10px) · `rounded-full`.
Driven by `rokkit.shape` config. Never `rounded-[6px]` — bypasses the system.

### 1.6 Borders and dividers

Hairlines via `border border-paper-edge` (or `border-t-paper-edge`,
`border-l-paper-edge`, etc.). Dashed sub-checks etc. via
`border-dashed border-paper-edge`. Status-tinted edges via the auto-emitted
`border-success-soft`, `border-accent-soft`, etc.

---

## 2. State and data flow

### 2.1 State lives in `*.svelte.ts`

Every screen with non-trivial state has a `*-state.svelte.ts` module. The state
class owns:

- All derivations from collections — `filter`, `find`, `count`, `reduce`.
- All status-driven copy and content — `display.eyebrow`, `display.headline`,
  per-status labels.
- All actions — `init()`, `verify()`, `retry()`, mutations.
- Wire integration — connecting transport events, mapping wire shapes to
  view-friendly shapes.

```ts
// example-state.svelte.ts
class ExampleState {
  status      = $state<HealthStatus>('checking');
  components  = $state<Component[]>([]);

  get readyCount(): number  { return this.components.filter(c => c.status === 'ready').length; }
  get display(): Display    { /* status → copy mapping */ }
  retry(id: ComponentId): void { /* … */ }
}
export const exampleState = new ExampleState();
```

State has its own `.spec.svelte.ts` covering every derivation and action.

### 2.2 Components — presentational only

Components take props, render markup, apply utility classes. **No collection
derivations. No data fetching. No API calls.**

Acceptable inside a component:
- Status → utility-class lookup map (the lookup *is* the rendering rule).
- Status → kanji glyph map.
- Conditional rendering on a discriminator prop.

Not acceptable:
- `$derived.by(() => props.items.filter(...))` — that's state work.
- `onMount(() => fetch(...))` — that's transport work.
- Reading `state` directly in a leaf component — pass via props.

### 2.3 Server-side data via SvelteKit load functions

When a screen needs data that exists before user interaction:
- `+page.ts` `load()` for client-resolvable data.
- `+page.server.ts` `load()` for server-only data (auth, secrets, DB-backed).
- Components receive `let { data }: Props = $props()`.

Use client-side `*.svelte.ts` state only when data must come from runtime —
e.g., the bootstrap health page (the daemon may not be running yet, which is
the screen's whole purpose), or live SSE streams.

### 2.4 API contracts win over mockup prototypes

The wire types (`health-types.ts`, future `wizard-types.ts`, etc.) are the
source of truth for enums. When the mockup invents enum values that don't exist
on the wire — `'starting'`, `'missing'`, `'error'` in mockup bootstrap-splash.jsx that
don't appear in `ComponentStatus = 'pending' | 'checking' | 'installing' | 'ready' | 'failed'`
— use the wire enum. Match the mockup's *visual* treatment using the wire's
discriminators.

---

## 3. Components

### 3.1 DRY — 2+ uses becomes a component or snippet

If you write the same kanji-eyebrow-title pattern twice, extract `KanjiHeader`.
If you write the same status disc twice, extract `StatusDisc` (or a
status-label-plus-disc combination like `StatusIndicator`).
Three near-identical lines is a smell; four is a refactor.

### 3.2 Two-tier extraction

- **Shared primitives** → `src/lib/components/` once they have a second consumer
  (or are likely to within the same phase).
- **Screen-local primitives** → co-located in `src/routes/.../` until promotion.

Don't pre-promote — let the second consumer pull a primitive up.

### 3.3 Naming

Match the mockup vocabulary: `KanjiHeader`, `StatusDisc`, `StatusIndicator`,
`Wordmark`, `GateRow`. Don't invent parallel names. When a screen genuinely
needs a primitive the mockup doesn't name (e.g., a `ProgressRail` for the
older bootstrap layout), borrow the mockup's variable/function name rather
than coining new terminology.

### 3.4 No `<style>` blocks for color or spacing

Allowed in `<style>`: keyframe animations, asymmetric geometry Tailwind can't
model, absolute offsets for unusual layouts, transitions and durations beyond
the token presets. Never color, never spacing, never typography.

### 3.5 Props

- Type every prop interface explicitly. No `any`.
- Discriminator props type against the wire enum from `*-types.ts`, not a
  component-local string union.
- Snippets for content slots (`title: Snippet`, `right?: Snippet`) — not props
  containing JSX-like trees.

---

## 4. Voice and copy

From `docs/mockups/Zen-Sumi Design System/SKILL.md` — non-negotiables for any
user-facing text:

- No exclamation marks. Ever.
- No emoji. Use a kanji or a sentence.
- Sentence case. Never title case.
- Lowercase "sensei" when sensei speaks in third person.
- No marketing speak: no "AI-powered", "supercharge", "unlock", "let's get started".
- Specific numbers over inflated ones — "3rd time" beats "1,247 patterns recognized".
- Loading copy is content, not a spinner caption — "Still listening." beats "Loading…".

Copy lives in the state file's `display` getter, not in the component template.
That way the same status produces the same copy across every screen.

---

## 5. Mockup fidelity

### 5.1 Mockups are the visual source of truth

`docs/mockups/Sensei/lib/*.jsx` defines what every screen should look like. The
current implementation is reference, not target. When the mockup and the current
screen disagree, rebuild against the mockup.

### 5.2 But the wire API overrides the mockup

Where the mockup uses enums/data shapes that don't exist on the wire, defer to
the wire. Mockup says `'starting'` for a daemon gate? Use `'installing'` from
`ComponentStatus`. Mockup styles a "blocked" pill in vermillion? Style the
`failed` status pill in vermillion.

### 5.3 Mockup discipline reading order

1. `docs/mockups/Sensei/CLAUDE.md` — conventions.
2. `docs/mockups/Sensei/lib/{feature}.jsx` — layout, components, copy.
3. `docs/mockups/Sensei/lib/tokens.css` — values to compare against
   `rokkit.config.js`.

---

## 6. Testing

### 6.1 Vitest component snapshots — checked in

Per-primitive `.spec.ts` for plain ts or `.spec.svelte.ts` for .svelte, .svelte.ts alongside the component. Mount every discriminator
variant; snapshot rendered HTML; assert semantic-class presence per variant.

```ts
// StatusDisc.spec.svelte.ts
import { render } from 'vitest-browser-svelte';   // or @testing-library/svelte
import StatusDiscHarness from './StatusDisc.harness.svelte';

describe('StatusDisc', () => {
  for (const status of ['pending','checking','installing','ready','failed'] as const) {
    it(`renders ${status}`, () => {
      const { container } = render(StatusDiscHarness, { props: { status } });
      expect(container.innerHTML).toMatchSnapshot();
    });
  }

  it('uses accent border for failed', () => {
    const { container } = render(StatusDiscHarness, { props: { status: 'failed' } });
    expect(container.firstElementChild!.className).toMatch(/border-accent/);
  });
});
```

Layout:
```
src/lib/components/StatusDisc.svelte
src/lib/components/StatusDisc.harness.svelte
src/lib/components/StatusDisc.spec.svelte.ts
src/lib/components/__snapshots__/StatusDisc.spec.svelte.ts.snap     ← checked in
```

Run via `bun run test`. CI gate.

### 6.2 State spec — checked in

`*-state.spec.svelte.ts` covers every derivation and every action. The wire is
mocked at the transport boundary, not inside the state.

### 6.3 Harness pattern — `*.harness.svelte` + `*.spec.svelte.ts`

Existing convention (see `Eyebrow.harness.svelte` + `Eyebrow.spec.svelte.ts`).
Harnesses are mountable wrappers imported by spec files — they are **not**
exposed as routes.

```
src/lib/components/
  Wordmark.svelte
  Wordmark.harness.svelte              ← test-friendly wrapper, takes simple props
  Wordmark.spec.svelte.ts              ← imports harness, mounts in Vitest browser
  __snapshots__/Wordmark.spec.svelte.ts.snap
```

The harness takes the same shape as the component but exposes the entire prop
surface as plain props (no snippets, no internal state). The spec mounts the
harness, drives every discriminator, and snapshots.

Manual visual scrubbing in dev: open the app at the actual screen
(`make app-dev` → navigate to `/health`, etc.) and drive the screen through
its states. Eyeball against `docs/mockups/Sensei/lib/*.jsx` rendered in the
design canvas. No dedicated harness route is needed.

### 6.4 Behaviour tests — Playwright e2e

Existing `e2e/tests/*.spec.ts` is the contract layer — clicks, navigation, real
daemon interactions. CI gate.

### 6.5 No checked-in PNGs

Visual baselines (Playwright `toHaveScreenshot`, Vitest browser screenshots,
etc.) are not the regression strategy. Machine-specific font rendering and
antialiasing make pixel diffs brittle. If a future use case genuinely needs
visual baselines, route the output under `test-results/visual/` (gitignored)
and gate with `test.skip(!!process.env.CI, 'local visual only')`. Never commit
PNGs.

---

## 7. Reference

### 7.1 Substitution table — z-scale → named tokens

Use this when migrating existing screens off the deprecated z-scale. Derived
from `Z_COLLAPSE_MAP_SURFACE` in `@rokkit/core`.

| Old (z-scale, deprecated)         | New (canonical named)          |
| --------------------------------- | ------------------------------ |
| `bg-surface-z0`                   | `bg-paper`                     |
| `bg-surface-z1`                   | `bg-paper-soft`                |
| `bg-surface-z2`, `bg-surface-z3`  | `bg-paper-mute`                |
| `border-surface-z2/z3/z4`         | `border-paper-edge`            |
| `text-surface-z5/z6`              | `text-ink-soft`                |
| `text-surface-z7/z8`              | `text-ink-mute`                |
| `text-surface-z9`                 | `text-ink`                     |
| `text-primary-z5` / `-z6`         | `text-accent`                  |
| `bg-primary-z5`                   | `bg-accent`                    |
| `bg-primary-z1`                   | `bg-accent-soft`               |
| `border-primary-z5`               | `border-accent`                |
| `text-success-z5`                 | `text-success`                 |
| `bg-success-z1`                   | `bg-success-soft`              |
| `text-warning-z6` / `bg-warning-z1` | `text-warning` / `bg-warning-soft` |
| `text-danger-z6` / `bg-danger-z1`   | `text-danger` / `bg-danger-soft`   |

Plus: every inline `oklch(var(--color-*) / N)` string in a `<style>` block or
`style="…"` attribute moves into the component via the equivalent named
utility, or — when the alpha matters — into a small utility class:
`bg-accent/30` for 30% opacity tints over an accent token.

### 7.2 Where to look for what

| Question                                         | Source                                                       |
| ------------------------------------------------ | ------------------------------------------------------------ |
| What color does `text-ink-soft` resolve to?      | `rokkit.config.js` overrides → `sumi-palette.js` shade       |
| What does my screen need to look like?           | `docs/mockups/Sensei/lib/<feature>.jsx`                      |
| What enum values does the wire use?              | `src/lib/<feature>-types.ts`                                 |
| Which voice rule applies to this copy?           | `docs/mockups/Zen-Sumi Design System/SKILL.md` §Voice        |
| Is there already a primitive for this?           | `src/lib/components/` — read before extracting               |
| How do I add a new semantic token?               | `rokkit.config.js` overrides — non-reserved name with `{light,dark}` |
