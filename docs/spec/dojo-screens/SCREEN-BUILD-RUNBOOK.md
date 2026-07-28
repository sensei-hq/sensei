# Dōjō screen-build runbook

> What to check when building or reviewing a dōjō screen against its mockup. Distilled
> from the inbox rebuild — every item here is a mistake we actually made and fixed. Read
> it before starting a screen and use it as the pre-commit checklist. Canonical styling
> rules live in [`../../architecture/frontend-svelte-guidelines.md`](../../architecture/frontend-svelte-guidelines.md)
> and [`../../mockups/Sensei/CLAUDE.md`](../../mockups/Sensei/CLAUDE.md); this is the
> operational "how not to get it wrong" layer.

## 0. Render the mockup and compare in the browser — don't infer from JSX

- Bring the mockup up **rendered** (a harness that mounts the real component, e.g.
  `docs/mockups/Sensei/_inbox-harness.html` served over http) and the app side-by-side in
  two tabs. Eyeballing the `.jsx` is not comparing.
- For every disagreement, read the **computed** value with DevTools / `browser_evaluate`
  (`getComputedStyle`) — `font-size`, `font-weight`, `font-family`, `color`,
  `border-radius`, `padding`/`gap`, `background`. Screenshots alone hide sub-pixel and
  token drift. Several "fixes" this cycle were wrong because they were guessed from a
  screenshot, not measured.
- The mockup preview must load the **real fonts** or it silently falls back to system-ui
  and reads thinner than the app — an invalid comparison. Force-load if the CSS
  `@font-face` doesn't register (the harness uses a FontFace-API injection).

## 1. Restart the dev server after config edits

`rokkit.config.js` and `uno.config.js` are read **once at dev-server startup** — Vite does
**not** hot-reload them. After editing icons/skin/theme/spacing there, **restart** `vite dev`
or the change won't show (this is why the icons stayed linear for a whole pass). CSS files
(`app.css`, `tokens.css`) and `.svelte` files DO hot-reload — no restart.

## 2. Spacing — uno utilities, never `var(--space-N)`

- Use `p-*` / `px-*` / `py-*` / `gap-*` / `m-*`. **Do not** write `style="padding: var(--space-3)"`
  in a component: the dōjō never defined the `--space-*` scale, so it collapses to `0`
  (a `--space-*` shim exists in `tokens.css` only to un-break already-ported kit).
- uno's scale is `N × 4px`; the mockup's `--space` **jumps** at 5+ (`5`=24, `6`=32, `7`=48).
  Map by **value ÷ 4**: mockup `space-3`(12)→`p-3`, `space-5`(24)→`p-6`, `space-6`(32)→`p-8`.
- 4px grid only. "If you need 18px, use 16 or 20."

## 3. Radii — pick the right stop, it's usage not override

`rounded-sm`=4 · `rounded`=6 · `rounded-lg`=10 · `rounded-full`. These are correctly defined
in `uno.config.js`; nothing is overridden. The mockup uses **6px** for pills/switcher/nav
item/search and **10px** for cards. Measure the element and use the matching stop — don't
default everything to `rounded-lg`.

## 4. Color — tokens only, tuned in `rokkit.config.js`

- Never a hex / `oklch()` / `rgba()` in a component. Reach for a named token
  (`bg-paper`, `text-ink`, `bg-accent-soft`, `border-paper-edge`, …). rokkit owns the
  palette via `rokkit.config.js` `overrides:` (dual-palette kami/sumi → dark-mode blocks).
- Verify the **computed** `--token` value equals the mockup's `tokens.css` value (they must
  match). Where rokkit's derived token diverges from the mockup — e.g. `--accent-soft` is a
  **translucent alpha accent** in the mockup (`oklch(0.58 0.15 35 / 0.12)`), not rokkit's
  solid pale derivative — override the token in `tokens.css` `:root` **and** a
  `[data-mode='dark']` block, copying the mockup's light + dark values so it flips correctly.

## 5. Components — use rokkit, don't hand-roll

`@rokkit/ui` ships `Toggle` (tabs / segmented mode switch), `Button`, `List`, `Tree`,
`Select`, `MultiSelect`, `Menu`, `Table`, `SearchFilter`, `ChatComposer`, … Reach for these
before building a bespoke control (there is **no** `Input`/`TextField` — a themed native
`<input>` is fine, mirroring the mockup's `zs-input`). Customize a rokkit component's look
with a global CSS override on its **data-attributes** (e.g.
`[data-toggle][data-toggle-variant='group'] [data-toggle-option][data-selected='true']`) or
`rokkit.config.js` — not by rebuilding it.

## 6. Type — 8-stop scale + named roles, never a literal `font-size`

- `text-xs` 11 · `sm` 13 · `base` 15 · `lg` 17 · `xl` 22 · `2xl` 28 · `3xl` 40 · `4xl`.
- Match the mockup's **named role**: `zs-h1/h2/h3`, `zs-eyebrow`, `zs-body`, `zs-meta`.
  E.g. a detail title is `zs-h3` = `text-lg` (17px), a section title is `text-xl` (22px), an
  eyebrow is `text-xs` (11px). Measure per element — a 22 vs 17 title looks wrong instantly.
- Fonts: Fraunces (display) · Inter (ui) · JetBrains Mono (mono), from the `tokens.css`
  `@import`. Weight scale is shifted **one step lighter** app-wide (`font-normal`=300,
  `font-medium`=400, `font-semibold`=500) via `app.css` — keep new code on the `font-*`
  utilities so it inherits that.

## 7. State — the three-layer pattern (`sensei:ui-state-pattern`)

Component (pure, reads state, routes intent back) ← State (`<screen>-state.svelte.ts`
singleton: data + getters + named methods; realtime lives here) ← Load
(`<screen>.ts`: **mock-first**, hand-crafted data exercising empty/edge/error, body-swapped
to a real fetch later without touching component or state). Copy is a 4th layer —
paraglide `m.*()` from `$lib/paraglide/messages`, **no inline string literals**.

## 8. Layout & scroll — panels scroll, headers stick

The shell content is a single scroll area, so a full-height screen scrolls as one page.
For a master-detail (or any header+body) panel: make it `h-full min-h-0 flex flex-col`, with
the header `shrink-0` (never scrolls) and the body `flex-1 overflow-y-auto`. Independent
panes each get their own such column inside a `h-full overflow-hidden` grid.

## 9. Access axis — user/membership-primary

Personal surfaces (`/you/*`) are **user-wide across all memberships**, not single-tenant —
see [`../../architecture/entity-access-model.md`](../../architecture/entity-access-model.md).
The Load layer's real read must honor that (and universal source-dereference on anything
that crosses the boundary). Don't bake a single `tenantKey` filter into a personal screen.

## 10. Pre-commit gate (zero-errors)

- `bun run check` → **0 errors / 0 warnings**.
- `bun run test` → green (unit-test the `-view`/`-state` modules; render-spec each component
  with a mock prop; Load mock → shape).
- Svelte MCP autofixer on **every** edited `.svelte` file.
- Dev-env notes: `vite dev` needs `.env.local` (it doesn't read `.dev.vars`); browse via
  `http://localhost:…`, not `127.0.0.1` (IPv6 binding refuses the latter).
