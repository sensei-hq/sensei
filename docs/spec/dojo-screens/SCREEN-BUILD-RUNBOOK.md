# Dōjō screen-build runbook

> How to build a dōjō screen against its mockup WITHOUT the cycles we hit on the inbox.
> Two parts: **(A)** the design-system baseline is configured **once** — you inherit it, you
> don't re-tune it per screen; **(B)** what to actually do for each screen. Canonical styling
> rules live in [`../../architecture/frontend-svelte-guidelines.md`](../../architecture/frontend-svelte-guidelines.md)
> and [`../../mockups/Sensei/CLAUDE.md`](../../mockups/Sensei/CLAUDE.md); this is the
> operational "how not to repeat the inbox cycles" layer.

## The #1 discipline: verify by MEASUREMENT, not by eye

The lesson that cost the most cycles on the inbox: **you cannot tell these differences by
looking** — 0-spacing vs proper padding, an 8px vs a 12px gap, Inter vs system-ui at the same
weight, a solid vs a 12%-alpha `accent-soft`, `rounded`(6) vs `rounded-lg`(10). Nearly every
fix guessed from a screenshot this cycle was **wrong**; the only reliable check was reading the
numbers. So the loop for every screen is:

1. Render the mockup **and** the app **in parallel** — the mockup in a harness that mounts the
   real component (e.g. `docs/mockups/Sensei/_inbox-harness.html` served over http) in one
   browser tab, the app in another. Eyeballing the `.jsx` is not comparing.
2. Drive both with **Playwright** and read the **computed** values via `browser_evaluate` +
   `getComputedStyle` — `font-size`/`weight`/`family`, `color`, `border-radius`, `padding`/`gap`,
   `background`, and the `--token` vars. **Diff the numbers**, don't compare screenshots
   (screenshots hide sub-pixel + token drift, and the spacing/weight diffs are invisible by eye).
3. Fix at the source, re-measure, repeat until the numbers match.

Gotcha: the mockup harness must actually **load the fonts** (Inter), or it silently falls back
to system-ui and reads thinner than the app — an invalid comparison (the harness force-loads
Inter via the FontFace API).

## Part A — Design-system baseline (configured ONCE — inherit it, don't re-tune per screen)

These are set for the whole dōjō by the inbox build. A new screen **inherits** them by using the
named tokens, the `text-*`/`p-*`/`gap-*` utilities, and rokkit components. **Do not re-pick
colors, fonts, spacing, or radii per screen** — if a value looks off it's a *usage* bug (wrong
utility/token), not a config gap. Touch these files only if the **design system itself** changes
— and then restart vite + re-verify **once, globally**, never per screen.

| Concern | Configured in | Settled value |
|---|---|---|
| Colors / tokens | `rokkit.config.js` `overrides:` + `dojo/src/lib/tokens.css` `:root` | 24 named tokens (paper/ink/accent/…), dual-palette kami/sumi → dark-mode; `--accent-soft` overridden to the mockup's **alpha** accent (light **and** `[data-mode='dark']`) |
| Fonts | `tokens.css` `@import` + `app.css` | Fraunces (display) · Inter (ui) · JetBrains (mono); weight scale shifted **one step lighter** (`font-normal`=300, `medium`=400, `semibold`=500) |
| Type scale | `uno.config.js` `theme.fontSize` | 8 stops `text-xs` 11 … `text-4xl` |
| Spacing | `uno.config.js` (+ a `--space-*` shim in `tokens.css` for the ported kit) | `p-*`/`gap-*` = N×4px; the `--space-*` var scale exists **only** so already-ported kit doesn't render 0 |
| Radii | `uno.config.js` `theme.borderRadius` | `rounded-sm`=4 · `rounded`=6 · `rounded-lg`=10 · `rounded-full` |
| Icons | `rokkit.config.js` `icons.overrides` | Solar **bold-duotone** |
| Rokkit component look | `app.css` `[data-*]` overrides | e.g. `Toggle` restyled to the mockup pill strip |

**`rokkit.config.js` and `uno.config.js` are read at dev-server startup — NOT hot-reloaded. If
you ever change them, RESTART `vite dev`** (CSS files + `.svelte` DO hot-reload — no restart).

## Part B — Per screen

### B1. Use the tokens + utilities the baseline gives you (don't re-tune)
- **Color** → named tokens only (`bg-paper`, `text-ink`, `bg-accent-soft`, `border-paper-edge`);
  never a hex/`oklch()`/`rgba()` in a component.
- **Spacing** → `p-*`/`gap-*` utilities; **never** `style="padding: var(--space-3)"` in new code
  (that scale is a compat shim, not for authoring).
- **Radii** → measure the mock element + pick the stop (`rounded`=6 for pills/nav/search,
  `rounded-lg`=10 for cards) — don't default everything to `rounded-lg`.
- **Type** → `text-*` + the named roles (`zs-h1/h2/h3`, `zs-eyebrow`, `zs-body`, `zs-meta`): a
  detail title is `zs-h3`(17), a section title `text-xl`(22), an eyebrow `text-xs`(11).
- The point is *using* the baseline correctly, not re-configuring it.

### B2. Reach for rokkit components — don't hand-roll (our biggest cycle)
`@rokkit/ui`: `Toggle` (tabs / segmented switch), `Button`, `List`, `Tree`, `Select`,
`MultiSelect`, `Menu`, `Table`, `Range`, `SearchFilter`, `ChatComposer`. (No `Input`/`TextField`
— a themed native `<input>` is fine.) Rich cards (dot · pips · why-line) stay **custom**.
**Each spec's Build-notes maps this screen's controls to the specific component** so no one
hand-rolls a tab strip / dropdown / list and then re-does it.

### B3. Layout & scroll — panels scroll, headers stick
The shell content is one scroll area, so a full-height screen scrolls as one page. For a
header+body (or master-detail) panel: `h-full min-h-0 flex flex-col`, header `shrink-0` (never
scrolls), body `flex-1 overflow-y-auto`; independent panes each get such a column inside a
`h-full overflow-hidden` grid.

### B4. State — the three-layer pattern (`sensei:ui-state-pattern`)
Component (pure, reads state, routes intent back) ← State (`<screen>-state.svelte.ts` singleton:
data + getters + named methods; realtime lives here) ← Load (`<screen>.ts` **mock-first**,
body-swapped to a real fetch later without touching component or state). Copy is a 4th layer —
paraglide `m.*()`, **no inline string literals**.

### B5. Access axis — user/membership-primary
Personal `/you/*` surfaces are **user-wide across all memberships** (fan-out / `owns_membership`),
not single-tenant — see [`../../architecture/entity-access-model.md`](../../architecture/entity-access-model.md).
Honor universal source-dereference on anything crossing the boundary.

### B6. Tests — assert the CONTRACT the next layer needs, not the current output
Every layer gets a test, but the test must assert what the **consumer** requires, not what the
code happens to emit today:
- **State/view** (`-state`/`-view` modules) — call methods, assert state values (no DOM).
- **Components** — render-spec with a mock prop (asserts the mockup wiring: labels, pips, why-line).
- **Load** — mock → assert the shape.
- **Mappers / cross-boundary seams** — assert the shape the DOWNSTREAM layer consumes.

> **The false-green trap (learned this cycle).** The daemon's plan→segment projection emitted
> `parent_id = null` (flat), and its unit test *asserted* `all(parent_id.is_none())` — it codified
> the bug as intended, so it stayed green while the UI outline was broken, and only a **visual**
> check caught it. A test that enshrines the current output is worse than none: it blocks nothing.
> Write the test against the requirement ("tasks **nest** under their phase") — it fails on the
> flat impl and forces the fix *before* any visual verification. And when data crosses a boundary
> (daemon → federation → UI), test the SEAM: the producer's output must carry what the consumer
> needs (here: the parent linkage the dōjō mapper nests on) — two isolated green tests with
> mismatched contracts is exactly how this slipped through.

### B7. Pre-commit gate (zero-errors)
`bun run check` → **0/0** · `bun run test` → green · Svelte MCP autofixer on **every** edited
`.svelte`. Dev-env: `vite dev` needs `.env.local` (not `.dev.vars`); browse `http://localhost:…`,
not `127.0.0.1` (IPv6).
