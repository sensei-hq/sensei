# Handoff — what the app should take from the mock

## Read in this order

1. **`DESIGN_PRINCIPLES.md`** — decide-these-first rules. Read before writing a screen.
2. **`MIGRATION.md`** — converting hand-rolled CSS to utilities, and the traps that cost the most time.
3. **`STYLING.md`** — the app's own house convention (UnoCSS + rokkit, breakpoints, the conflicting-utility gotcha).
4. This file — what's in the package and how the mock complies.

## The package

| File | What it is |
|---|---|
| `uno.config.ts` | **Reference, not a merge.** The literals the app's own config already ships, so mock ↔ app can be diffed value-for-value. Merging it verbatim would break the app — see below. |
| `uno.runtime.js` | The mock's runtime equivalent. Same theme — the parity reference, not for the app. |
| `tokens.css` | Tokens, reset, type roles, `zs-*` components. Copy as-is **including the `@layer` wrappers**. |
| `fonts.css` | Fontsource `@font-face` blocks, one per shipped weight. |
| `fonts.manifest.js` | Package / family / token / weights, machine-readable. A manifest, not a runtime dependency. |
| `dojo2-api.js` | The data seam. Every accessor names the endpoint it maps to. |
| `dojo2-nav.ts` | The IA: one Inbox, three org zones, redirect maps for retired routes. |

## Fonts — install these exactly

```bash
npm i @fontsource/inter @fontsource-variable/fraunces \
      @fontsource/jetbrains-mono @fontsource/shippori-mincho
```

```ts
import '@fontsource/inter/latin-300.css';
import '@fontsource/inter/latin-400.css';
import '@fontsource/inter/latin-400-italic.css';
import '@fontsource/inter/latin-500.css';
import '@fontsource/inter/latin-600.css';
import '@fontsource-variable/fraunces';               // covers 300–600
import '@fontsource/jetbrains-mono/latin-400.css';
import '@fontsource/jetbrains-mono/latin-500.css';
import '@fontsource/shippori-mincho/japanese-400.css'; // the kanji marks
```

**Weights 300 / 400 / 500 / 600. Nothing else.** `theme.fontWeight` is
constrained to exactly these, so an unshipped weight is unwritable rather than
merely discouraged. Inter 700 is deliberately not loaded.

Two notes. The mock **self-hosts** these same Fontsource files rather than
`@import`-ing the CDN, because html-to-image can't inline a cross-origin
stylesheet and every screenshot silently degrades to a fallback face — that bug
was live here (Fraunces' `src` pointed at a path that didn't exist). And
Shippori Mincho is **not** vendored in the mock: the kanji marks fall back to the
OS Mincho stack, so mock and app differ slightly on Windows. That's the one known
typography gap.

## Why utilities kept getting mixed up

Not an agent-discipline problem — a **two-vocabularies** problem.

The design system used to ship its own hand-rolled utility layer inside
`tokens.css`: `.zs .gap-6 { gap: var(--space-6) }` and ~1,300 lines like it. The
app generates utilities with Uno + `presetRokkit`. Same class *names*, different
*meanings*: the mock's `gap-6` was 32px on a bespoke 9-stop scale, the app's is
24px on the 4px grid. Both engines fail *silently* on an unknown class, so
nothing errored — it just drifted.

That layer is deleted. The mock now runs real UnoCSS; every spacing usage was
renumbered to the standard scale (5→6, 6→8, 7→12, 8→16, 9→24) and 16,543 inline
style properties became utility classes, verified pixel-identical across 801
elements. `var(--space-6)` is `p-6` is 24px, so token and class can't disagree.

**Spacing and breakpoints are deliberately not in `uno.config.ts`.** Uno's
defaults already are this system's scale; re-declaring them would be a second
copy to keep in sync, and a partial spacing table drops the fractional stops
(`p-0.5` = 2px) that hairline-gap controls need.

## Load order — not optional

`tokens.css` keeps its `@layer zs-base` / `@layer zs-components` wrappers, and
Uno emits to `@layer uno-utilities`, declared after them.

**Unlayered CSS outranks every layer**, and specificity doesn't save you:
`.zs-card` (0,1,0) already ties with `.p-4` (0,1,0). Before this layering,
`zs-card p-4` rendered 24px — the utility lost. Strip the layers and "reach for a
utility first" becomes untrue in exactly the cases that matter.

Because `tokens.css` owns the reset, it also carries what Uno's suppressed
preflight would normally set: `box-sizing`, `border-width: 0` + `border-style:
solid` (without it every `border-*` utility silently collapses — `MIGRATION.md`
§5), and the `--un-*` shadow fallbacks.

## Components — the inventory

The kit is `lib/dojo2/dojo2-kit.jsx`. Names carry over verbatim; variants are
enumerated props, never class strings passed in.

| Component | Props | Notes |
|---|---|---|
| `Button` | `variant` primary·ghost·danger·link, `size` sm·md·lg, `full`, `icon`, `kanji`, `disabled` | **The only button.** `link` is the bare-text affordance. |
| `Card` | `tone` paper·accent·ink·success·warning, `pad`, `selected`, `onClick` | The one card family. Never shadows — not offered. |
| `Row` | `cols`, `gap`, `pad`, `onClick`, `selected`, `align`, `last` | Hairline-separated row inside a card. |
| `SectionHead` | `kanji`, `eyebrow`, `title`, `count`, `right` | The one section header. |
| `ListSection` | `icon`, `title`, `count`, `right` | Header + flush card of rows. |
| `SegmentedControl` | `options[{id,label,icon}]`, `value`, `onPick` | Two/three-way view switch. |
| `PillChoice` | `options[{id,label,kanji}]`, `value`, `onPick` | Wrapping pick-one chips. |
| `NumberedChoice` | `n`, `label`, `selected`, `onPick` | Numbered ask option. |
| `Toggle` | `on`, `onToggle`, `label` | `role="switch"`. |
| `Metric` / `MetricRow` | `label`, `value`, `unit`, `delta`, `deltaGood`, `sub` | `deltaGood` decides which direction reads green. |
| `Chip` · `Banner` · `EmptyState` · `InboxRow` · `PlanOutline` · `AppShell` | see source | |

Names carry over verbatim — no prefix, no translation layer. They're exported
on `window` by the kit, so a screen reads `<Button variant="ghost">` exactly as
the React app will.

**There are no shortcuts at all, on purpose.** A component already owns its
utility run, so a `card` shortcut next to `<Card>` would be a second way to say
the same thing and the two would drift. The hairline used to be a `border-1px`
shortcut; on the app team's "pick one side" question it's now expanded to raw
`border border-paper-edge` at all 576 call sites — exactly what the app's shipped
screens already write. Nothing to add app-side, no alias to keep in sync.

What must exist somewhere is the reset both engines assume:
`*, ::before, ::after { box-sizing: border-box; border-width: 0; border-style:
solid }`. Without `border-style` every `border` utility silently collapses to
nothing — it lives in `tokens.css` `@layer zs-base`.

Two mechanics worth knowing: a shortcut may compose **only utilities** (pointing
one at a component class like `zs-card-flush` makes Uno warn and emit nothing),
and a dynamic class must appear literally in source — `"gap-" + n` generates
nothing under a build-time scan, which is why `Row` maps its `gap` prop through
a lookup table.

## Data — the seam

`dojo2-api.js` is what screens call; they never read the fixture global. Every
accessor is already `async` and already returns the shape a row needs, so making
it real can't change a call site. Set `transport` to a fetch wrapper and each body
becomes one line. Server contracts worth honouring are called out in comments —
inbox ordering and progress rollups belong to the server, and roles are derived
from git server-side, never client-computed.

**Every screen now reads through it** — inbox, session detail, asks, teams, the
drill-downs, and all the org consoles. There is deliberately no `const D2 =
window.DOJO2` alias left in the app: that back door is what would make the seam
a fiction.

Two consequences of async data worth carrying into the app. A screen must not
dereference its slice on the first render, so each guards with the system's own
empty state ("Still listening.") rather than a skeleton. And the guard has to sit
**after every hook** — an early return placed among them changes the hook count
between renders and React throws.

## Open items

- The mock loads Uno from CDN at runtime, so generation is async and there's a
  brief unstyled moment. The app uses the build plugin; nothing else differs.
- `presetRokkit` resolves `bg-{anything}` dynamically, so `bg-inkk` yields
  `var(--inkk)` — no CSS, no error. The mock registers an explicit allow-list
  instead. Neither errors; a lint rule over token names is the only real fix.
- ~3,100 raw numeric `fontSize` and ~800 off-scale `borderRadius` values remain in
  the mock's **older** screens (not the Dōjō app). Snapping them to the scale is a
  visible change, so it's pending a decision.
- A handful of hand-rolled `<button>`s remain in the org consoles; they should
  become `Button variant="link"` / `SegmentedControl` / `PillChoice`.
