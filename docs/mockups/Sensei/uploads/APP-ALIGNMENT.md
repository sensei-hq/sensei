# Handoff corrections — align the package with the app's real UnoCSS config

> From the app team, after diffing the new UnoCSS handoff against the shipped
> dōjō app (`sensei/dojo`). **The value-level alignment is correct and a real win**
> — type scale (`11/13/15/17/22/28/40/56`), radii (`4/6/10/full`), weights
> (`300/400/500/600`), and spacing (Uno's default 4px scale) all now agree, so a
> mock class and the app class finally mean the same thing.
>
> The fixes below are all in the **handoff package**, not the app. They exist
> because `uno.config.ts` describes itself as a drop-in merge, but it references
> CSS variables the app doesn't define — so merging it verbatim would break the
> app. The app already dodged this by **hardcoding** the same literals. So the
> direction is: **make the handoff config match what the app actually ships
> (hardcoded, minimal), and stop telling the app to merge it.** No new app config
> is needed for the scale — over-config isn't wanted.

## The one root cause

`handoff/uno.config.ts` writes the theme as `var(--…)` references:

```ts
fontSize:   { sm: 'var(--text-sm)', … }   // and var(--shadow), var(--leading-*),
lineHeight: { tight: 'var(--leading-tight)', … }  // var(--tracking-*), var(--dur)
```

Verified against the app: **`--text-*`, `--shadow`, `--leading-*`, `--tracking-*`,
`--dur*` are NOT defined anywhere in the app** (`dojo/src/lib/tokens.css`,
`dojo/rokkit.config.js`, `app/rokkit.config.js`). That's exactly why the app's own
`dojo/uno.config.js` **hardcodes** `sm: ['13px', '1.5']` instead of a var. If a dev
follows this package's "Merge into the app's config" instruction, every `text-*`
becomes `font-size: var(--text-sm)` → **empty** → the type scale, shadows, and
line-heights silently collapse.

`--radius-*`, `--font-*`, `--space-*` **do** exist app-side, so those var-refs are
safe. Only the missing ones must be hardcoded.

---

## Corrections (in priority order)

### 1 — [High] Hardcode the theme to the app's literals; relabel as reference, not "merge"
`handoff/uno.config.ts`. Replace the missing-var references with the literal values
the app ships (below). Change the file header from "Merge the theme and shortcuts
blocks into the app's existing uno.config.ts" to "**Reference — these are the
literals the app's `uno.config.js` mirrors. Do not merge; the app hardcodes to
avoid an undefined-var trap.**"

### 2 — [High] Bundle line-height into each `fontSize` tuple (match the app)
The app uses UnoCSS's `[size, lineHeight]` tuple form, so `text-sm` alone sets
**both** size and line-height `1.5`. The handoff uses a bare size + a separate
`leading-*`, so a mock `text-sm` alone renders at the inherited line-height →
vertical-rhythm drift when comparing to the app. Use the tuples below. (Bonus: with
the utility layer outranking `zs-components`, the bundled line-height also wins over
the `.zs-*` role's `line-height`, so the two auto-reconcile — but see follow-up A.)

### 3 — [High] Delete the `card` / `card-pad` shortcuts — they contradict this package's own README
`handoff/uno.config.ts:107-108` defines `card` + `card-pad`, but
`handoff/README.md` (§Components) says *"There are almost no shortcuts, on purpose…
a `card` shortcut next to `<Card>` would be a second way to say the same thing and
the two would drift. The only one that survives is `border-1px` (plus `surface-ink`
/ `surface-accent`)."* There is a `<Card>` component — the README is right. Remove
`card`/`card-pad` from the config so the package is self-consistent.

### 4 — [Med] Confirm the shortcut set = exactly the 3 the README blesses
Keep only `border-1px`, `surface-ink`, `surface-accent`. The app currently defines
**none** of these and 0 shipped screens use them (screens use raw
`border border-paper-edge` / `border-b border-paper-edge`), so mock JSX using
`border-1px` (README cites 474 call sites) renders nothing in-app today. The app
team will add these 3 to `dojo/uno.config.js` + `app/uno.config.js` so the
vocabulary matches — **as long as the handoff ships exactly these 3 and no more.**
(If you'd rather the mock emit raw `border border-paper-edge`, say so and we won't
add the shortcut — but pick one side.)

---

## The corrected `handoff/uno.config.ts` (drop-in)

```ts
// ═══════════════════════════════════════════════════════════════════
// Zen-Sumi · UnoCSS theme — REFERENCE for the app's config
// ═══════════════════════════════════════════════════════════════════
// These are the literals the app's dojo/uno.config.js + app/uno.config.js
// already ship. This file exists so mock ↔ app can be diffed value-for-value.
// DO NOT "merge" it into the app: the app hardcodes these on purpose because
// --text-*, --shadow, --leading-*, --tracking-*, --dur* are NOT defined in the
// app's token layer — a var() reference there resolves to nothing.
//
// presetRokkit already gives bg/text/border-{token}, Uno's 4px spacing, and the
// sm/md/lg/xl breakpoints — all of which the mock matches. Only the scale below
// (type, radii, shadow, families, tracking/leading, motion) is declared.

import { defineConfig } from 'unocss';
import { presetRokkit } from '@rokkit/unocss';

export default defineConfig({
  presets: [presetRokkit()],

  theme: {
    // Type · 8 stops, floor xs (11). Tuple form [size, lineHeight] — line-height
    // is bundled so `text-sm` alone matches the app exactly (no stray leading-*).
    fontSize: {
      xs:    ['11px', '1.4'],
      sm:    ['13px', '1.5'],
      base:  ['15px', '1.6'],
      lg:    ['17px', '1.5'],
      xl:    ['22px', '1.2'],
      '2xl': ['28px', '1.2'],
      '3xl': ['40px', '1.2'],
      '4xl': ['56px', '1.05'],
    },

    // Families resolve to --font-* tokens, which DO exist app-side (safe var-ref).
    fontFamily: {
      display: 'var(--font-display)', // Fraunces — headings
      ui:      'var(--font-ui)',       // Inter — body + chrome
      mono:    'var(--font-mono)',     // JetBrains — numbers, ids, paths
      kanji:   'var(--font-kanji)',    // Mincho — functional marks
    },

    // Hardcoded — app has no --leading-*/--tracking-* tokens.
    lineHeight:    { tight: '1.2', snug: '1.4', normal: '1.6', loose: '1.75' },
    letterSpacing: { tight: '-0.02em', normal: '0', wide: '0.18em' },

    // Radii — hardcoded to match the app (which also hardcodes, not var()).
    // rounded-full is pills/dots/avatars/toggle tracks ONLY — never a wide card.
    borderRadius: { sm: '4px', DEFAULT: '6px', lg: '10px', full: '9999px' },

    // Motion — one curve, three durations. Hardcoded (no --dur* app-side).
    transitionDuration:       { fast: '120ms', DEFAULT: '180ms', slow: '280ms' },
    transitionTimingFunction: { DEFAULT: 'cubic-bezier(0.2, 0.6, 0.2, 1)' },

    // Elevation — modals/popovers/palettes ONLY; cards + buttons never shadow.
    // Keep your --shadow* tokens; only requirement is visual parity with the
    // app's shadow (soft, ink-tinted). Exact expression need not match — this is
    // the one place the app itself keeps a var (its internal ink token).
    boxShadow: {
      sm:      'var(--shadow-sm)',
      DEFAULT: 'var(--shadow)',
      lg:      'var(--shadow-lg)',
    },

    // Weights — see follow-up C: the app doesn't restrict these yet, so font-bold
    // (700) is writable in-app but not here. Align on the app side, or drop this
    // block. Values match Uno defaults regardless.
    fontWeight: { light: '300', normal: '400', medium: '500', semibold: '600' },

    // Spacing + breakpoints — deliberately NOT declared. Uno's defaults already
    // ARE this system's scale (4px × n; sm 640 / md 768 / lg 1024 / xl 1280) and
    // presetRokkit inherits them. Re-declaring drops fractional stops (p-0.5=2px).
  },

  shortcuts: {
    // The three the README blesses — no `card`/`card-pad` (the <Card> component
    // owns that; a shortcut would be a second, drifting way to say it).
    'border-1px':     'border border-solid border-paper-edge', // hairline: width+style+edge
    'surface-ink':    'bg-ink text-on-primary',
    'surface-accent': 'bg-accent text-on-primary',
  },

  // Required ONLY if the app adopts tokens.css's @layer zs-components (it doesn't
  // today — it has its own tokens.css with no component layer). The mock keeps it.
  outputToCssLayers: { cssLayerName: () => 'uno-utilities' },
});
```

Also fix the README ↔ config: the README's "almost no shortcuts" paragraph is now
accurate — nothing to change there once `card`/`card-pad` are gone from the config.

---

## App-side follow-ups (the app team owns these — listed for context, not for you)

- **A. `.zs-*` role line-heights vs bundled `text-*`.** `.zs-body-sm` uses
  `leading-normal` (1.6) but `text-sm` bundles 1.5. With the utility layer
  outranking components, `text-sm` (1.5) wins where both apply — matching the app.
  Worth reconciling the role token to 1.5 so a bare `.zs-body-sm` (no utility)
  agrees too.
- **B. Add the 3 shortcuts** (`border-1px`, `surface-ink`, `surface-accent`) to
  `dojo/uno.config.js` + `app/uno.config.js` so mock JSX using them renders. (Gated
  on correction #4 landing exactly these 3.)
- **C. `fontFamily` + restricted `fontWeight`** aren't in the app's uno.config yet,
  so `font-display` and the "only 4 weights" guarantee aren't enforced app-side.
  Add both to the app config if we want them enforced.
- **D. Token-name lint.** `presetRokkit` resolves `bg-anything → var(--anything)`
  silently; the mock's allow-list fails visibly. Add a lint over token names so a
  typo'd token is caught in the app too.
- **E. Shadow z-scale.** The app's `uno.config.js` boxShadow uses
  `oklch(var(--color-ink-z9) / …)`, but the design system says "no z-scale" and
  `app/CLAUDE.md` lists `shadow-tint` as the token. App-internal cleanup, unrelated
  to the mock.

---

## Acceptance — the package is aligned when

- [ ] `handoff/uno.config.ts` contains **no `var(--text-*)`, `var(--leading-*)`,
      `var(--tracking-*)`, `var(--dur*)`** (only `--font-*` / `--shadow*` var-refs,
      which are fine).
- [ ] Every `fontSize` entry is a `[size, lineHeight]` tuple.
- [ ] No `card` / `card-pad` shortcut; exactly `border-1px`, `surface-ink`,
      `surface-accent` remain.
- [ ] The header says reference, not "merge into the app."
- [ ] `README.md` and `uno.config.ts` agree on the shortcut set.
