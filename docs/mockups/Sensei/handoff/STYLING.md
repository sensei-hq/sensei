# Dōjō styling guideline — utility-first with UnoCSS + rokkit

The dōjō app styles with **UnoCSS** (via `@rokkit/unocss`'s `presetRokkit`). UnoCSS is
Tailwind-compatible in spirit: same utility names, same `sm:`/`md:`/`lg:` responsive
prefixes. This doc is the house convention so screens are built from consistent utility
classes instead of per-screen inline `style="…"` or hand-rolled `@media` blocks.

## The 6 rules

1. **Utility classes, not inline `style`.** Inline `style` is only for *runtime-dynamic*
   values (a bound width %, a color computed from data). Everything static → utilities.
2. **Rokkit named tokens for color/surface** — `bg-paper text-ink border-paper-edge`.
   Never raw hex, never `style="color: var(--ink)"`.
3. **Responsive via prefixes, never `@media` in `<style>`.** Mobile-first base, layer
   desktop with `md:`/`lg:`.
4. **Mobile-first.** Unprefixed = phone. `md:` = desktop console. Write the phone layout
   first, then add `md:` overrides.
5. **Prefer the scale; brackets only when needed.** `p-4 gap-2 w-full` over
   `style="padding:16px"`. Arbitrary values (`w-[218px]`) are fine but sparing.
6. **Swap conflicting utilities, don't stack them** (see the gotcha below).

## Breakpoints (UnoCSS defaults — rokkit inherits them)

| prefix | min-width | dōjō meaning |
|---|---|---|
| *(none)* | 0 | **phone** — the base layout |
| `sm:` | 640px | large phone / small tablet |
| `md:` | **768px** | **desktop console** — the canonical split: `< md` phone, `md:+` console |
| `lg:` | 1024px | wide desktop (finer two-pane / grid tuning) |
| `xl:` | 1280px | extra-wide |

**Canonical split:** below `md` = phone (single column, drawer nav); `md:`+ = the
existing desktop console (persistent sidebar, two-pane).

## Color & surface — use the rokkit tokens

`presetRokkit` generates `bg-{token}` / `text-{token}` / `border-{token}` (and
`border-t/-b/-l/-r`, `ring-`, `outline-`, `fill-`, `stroke-`) → each resolves to
`var(--{token})`. Use these, never raw values.

| role | tokens |
|---|---|
| surfaces | `paper`, `paper-soft`, `paper-mute`, `paper-edge` (hairline) |
| text | `ink`, `ink-soft`, `ink-mute`, `ink-faint` |
| accent | `accent`, `accent-soft`, `on-primary` (contrast text only), `primary` |
| status | `success`, and the semantic status tokens |

```svelte
<!-- good -->
<div class="bg-paper-soft border-paper-edge text-ink border rounded-lg p-4">…</div>
<!-- avoid -->
<div style="background: var(--paper-soft); border: 1px solid var(--paper-edge); padding: 16px">…</div>
```

## Responsive recipes (intent → classes)

| intent | classes |
|---|---|
| desktop-only element | `hidden md:block` (or `hidden md:flex`) |
| mobile-only element (e.g. hamburger) | `md:hidden` |
| stack on phone, row on desktop | `flex flex-col md:flex-row` |
| 1-col → multi-col grid | `grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3` |
| full-width phone, fixed desktop | `w-full md:w-[218px]` |
| off-canvas drawer → static sidebar | `fixed md:static -translate-x-full md:translate-x-0` + swap-on-open |
| wide content (tables) w/o page h-scroll | wrap in a `overflow-x-auto` container |
| drop a long label on phone | `hidden sm:inline` on the label span |

## The conflicting-utility gotcha

Two utilities that set the **same** CSS property (e.g. `translate-x-0` and
`-translate-x-full`, both drive `transform`) resolve by **stylesheet source order**, not
class-attribute order — so conditionally *adding* the second one is unreliable.
**Swap the whole class instead:**

```svelte
<!-- good: exactly one translate applies below md; md: always wins at ≥768px -->
<aside class="-translate-x-full transition-transform md:static md:translate-x-0
              {open ? 'translate-x-0' : '-translate-x-full'}">
```

## When inline `style` IS acceptable

Only for values computed at runtime that no utility can express statically:

```svelte
<div style="width: {pct}%"></div>            <!-- bound progress -->
<span style="background: {row.tone}"></span> <!-- data-driven color -->
```

## DRY: repeated class strings

If the same multi-utility combo repeats across screens (a card, a pill), add a
**shortcut** in `uno.config.js` rather than copy-pasting the class list, then use the
one shortcut name. Keeps the vocabulary small and consistent.

## Migrating existing screens

Current screens carry a lot of inline `style="…"` and mirror the mockups' `wide` prop.
Convert **opportunistically** when you touch a screen: static inline style → utilities;
`wide` prop → responsive prefixes (the mockup stays the *visual spec* for what each
breakpoint should look like — see `docs/mockups/Sensei/lib/dojo/dojo-relay.jsx`).
Do **not** introduce `@media` blocks in `<style>` for layout. The one legitimate
`@media` is `prefers-reduced-motion` (a motion concern, not layout).
