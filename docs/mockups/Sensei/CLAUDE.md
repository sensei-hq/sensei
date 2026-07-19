# Sensei — design-system conventions

The visual system lives in **`lib/tokens.css`** (the published copy is `site/tokens.css` — keep them in sync). Every screen is wrapped in a `.sensei` (or `.zs` / `.artboard-shell`) scope, which is what activates the utility classes below.

The rule for all UI code: **reach for a semantic token or utility class first.** Inline `style={{…}}` is only for geometry the system doesn't model (fixed control dimensions, asymmetric padding, absolute offsets, transitions, opacity). Never hand-write a hex/oklch color or a raw `font-size` in component code.

## Color — semantic ramp (use these names)

Backgrounds and text each have a 4-step ramp. Prefer the **semantic name**; the numbered tokens (`--paper-2`, `--ink-3`, …) are deprecated aliases kept only so older JSX keeps working.

| Surface | Token | Utility | Use for |
|---|---|---|---|
| paper | `--paper` | `bg-paper` | base surface |
| paper-soft | `--paper-soft` | `bg-paper-soft` | raised cards / panels |
| paper-mute | `--paper-mute` | `bg-paper-mute` | sunken wells, toggle tracks |
| paper-edge | `--paper-edge` | `border-paper-edge` | hairline borders |

| Ink | Token | Utility | Use for |
|---|---|---|---|
| ink | `--ink` | `text-ink` | primary text |
| ink-soft | `--ink-soft` | `text-ink-soft` | secondary text, input placeholders |
| ink-mute | `--ink-mute` | `text-ink-mute` | captions, meta, status labels |
| ink-faint | `--ink-faint` | `text-ink-faint` / `border-ink-faint` | disabled, empty/placeholder rings |

Status families (each has base / `-soft` fill / `-edge` border where relevant):
`accent` · `success` · `warning` · `danger`. Utilities: `text-accent`, `text-success`, `text-danger`, `bg-accent-soft`, `bg-success-soft`, `bg-warning-soft`, `bg-danger-soft`, `border-success-edge`, `border-danger-edge`. `bg-ink` / `bg-accent` set their own readable foreground.

## Borders

- `border-1px` → 1px solid, colored `paper-edge` by default. Add `border-dashed` for dashed, or a `border-*-edge` / `border-ink-faint` class to recolor.
- Don't write `border: var(--hairline)` inline — use `border-1px border-paper-edge` (or the directional `border-t/-b/-l/-r`).

## Type — fixed scale, no stray sizes

Sizes: `text-xs` 11 · `text-sm` 13 · `text-base` 15 · `text-lg` 17 · `text-xl` 22 · `text-2xl` 28 · `text-3xl` 40 · `text-4xl`. Weights: `font-light/normal/medium/semibold`.

For headings and stock text roles, prefer the **named type classes** over picking a size:
`zs-h1` `zs-h2` `zs-h3` · `zs-hero` `zs-display-lg` · `zs-body` `zs-body-sm` · `zs-meta` `zs-eyebrow` · `mono` `display` `kanji`.

Never put a literal `fontSize` in JSX — snap to a scale stop or a type class.

## Spacing

4px base scale via `gap-N` / `p*-N` / `m*-N`, N=0–9 (`1`=4 `2`=8 `3`=12 `4`=16 `5`=24 `6`=32 `7`=48 `8`=64 `9`=96). Lay rows/groups out with `flex`/`grid` + `gap-*`, not per-child margins.

## Components

`zs-btn` (+ `zs-btn-primary/secondary/ghost/sm/lg`), `zs-badge` (+ `-success/-warning/-accent`), `zs-card`, `zs-input`, `zs-dot`. Reuse these before building a new control.

## Reference implementation

`lib/assistant-card.jsx` is the canonical example of these conventions: semantic color classes, scale type, geometry-only inline styles. Match its style when adding or editing UI.

## Dark mode

`[data-theme="dark"]` re-defines the tokens; because everything resolves through the semantic vars, themed code needs no per-component dark handling. Don't bypass tokens or dark mode will break.

---

# Mockup / claude.ai design styling — the Zen-Sumi system without rokkit

Guide for generating **mockups and design artifacts on claude.ai** (the Artifact tool)
that match the sensei product's look. Artifacts are self-contained (strict CSP: no
external CSS/JS/fonts/images, no CDN), so they **cannot use rokkit** — but they can and
must use the **same design system**: the same tokens, the same 8-stop type scale, the
same component shapes.

This is the claude.ai/no-rokkit counterpart of the in-app convention
[`../architecture/frontend-svelte-guidelines.md`](../architecture/frontend-svelte-guidelines.md).
Same rules, different delivery. The core discipline is identical: **use the tokens and
the scale — never hand-code colors, font sizes, spacing, or media queries.**

## Drop-in base: `colors_and_type.css`

[`Zen-Sumi Design System/colors_and_type.css`](./Zen-Sumi%20Design%20System/colors_and_type.css)
**is** the design system as one self-contained stylesheet — CSS-variable tokens + a
`.zs`-scoped utility layer + component classes. To build a mockup:

1. Inline the whole file into the artifact's `<style>`.
2. Wrap the mockup in a container with `class="zs"` (utilities + components are scoped
   to `.zs`).
3. Build with the classes below — never raw values.

### claude.ai CSP caveats
- The file's Google-Fonts `@import` is **blocked** by the artifact CSP → the font-stack
  fallbacks render (Fraunces→Georgia, Inter→system-ui, JetBrains→Menlo, Shippori→serif).
  That's acceptable; do **not** rely on the web fonts loading. Delete the `@import` line
  to avoid a console error.
- No external images — use inline SVG or an emoji. No external scripts.
- Everything inline; nothing fetched.

## Tokens (CSS vars — use via the classes, don't inline the vars)

| Group | Tokens |
| --- | --- |
| Surface | `paper` `paper-soft` `paper-mute` `paper-edge` |
| Ink (text) | `ink` `ink-soft` `ink-mute` `ink-faint` |
| Accent / primary | `accent` `accent-soft` · `primary`=ink · `on-primary`=paper |
| Status | `success`/`-soft` `warning`/`-soft` `danger` |
| Aesthetic | washi paper · sumi ink · 朱 vermillion accent (rationed). Hairlines over shadows. Air over density. |

Dark mode: add `data-theme="dark"` on a wrapper — the tokens flip automatically. Never
write per-mode colors in the markup.

## Type scale — 8 stops, never a literal px

`text-xs` 11 · `text-sm` 13 · `text-base` 15 · `text-lg` 17 · `text-xl` 22 ·
`text-2xl` 28 · `text-3xl` 40. Headings use `.zs-h1/-h2/-h3` (Fraunces display) or
`.zs-display`/`.zs-hero`. Body `.zs-body` / `.zs-body-sm`; metadata `.zs-meta` (mono);
labels `.zs-eyebrow` (uppercase, tracked); `.zs-kanji` for CJK glyphs (accent-colored).
Weights: `font-light/normal/medium/semibold` only.

## Spacing — 4px grid, never a literal px

`p-N` `px-N` `py-N` `gap-N` `mt-N` `mb-N`, `N ∈ 1..8` (`1`=4 `2`=8 `3`=12 `4`=16 `5`=24
`6`=32 `7`=48 `8`=64px). Radii: `rounded-sm` 4 · `rounded` 6 · `rounded-lg` 10 ·
`rounded-full`. "If you need 18px, use 16 or 20." Don't invent a stop.

## Component shapes (mirror rokkit's)

| Component | Class | Notes |
| --- | --- | --- |
| Button | `.zs-btn` + `.zs-btn-primary` / `-secondary` / `-ghost`, sizes `.zs-btn-sm` / `-lg` | ink-on-paper primary; vermillion is rationed |
| Card | `.zs-card` (or `.zs-card-flush`) | `paper-soft` + `paper-edge` hairline + `rounded-lg` |
| Badge / pill | `.zs-badge` + `.zs-badge-success` / `-warning` / `-accent` | mono, `text-xs` |
| Input | `.zs-input` (wraps a bare `<input>`) | hairline border, focuses to ink |
| Status dot | `.zs-dot` + `.zs-dot-accent` / `-success` / `-warning` | 7px |
| List | a column of rows (`flex items-center justify-between px-4 py-3`) separated by `border-b border-paper-edge`; last row no border | hairlines, not boxes |
| Rule | `.zs-rule` | 1px hairline divider |
| Eyebrow+title header | `.zs-eyebrow` over `.zs-h1`/`.zs-h2`, optional `.zs-kanji` to the left | the standard section header |

Layout uses the `.zs .flex/.grid/.items-*/.justify-*/.gap-*/.grid-cols-2|3|4` utilities.

## Responsive (artifacts CAN use media queries — keep breakpoints consistent)

`.zs`'s utility layer has no `md:` prefixes, so for a responsive mockup add media
queries **at the app's breakpoints** (`sm` 640 · `md` 768 · `lg` 1024), **mobile-first**
(base = phone, `min-width` queries widen). Match the app's split: `<768px` phone,
`≥768px` desktop. Keep every size/color inside those queries on the tokens + scale too.

## The one rule

Whether in-app (rokkit) or a claude.ai mockup (`.zs`): the design system is robust —
**use it.** Reach for a token / a `text-*` step / a `p-*` stop / a `.zs-*` component,
never a hand-picked `#hex`, `font-size: 13px`, `padding: 15px`, or a bespoke `@media`.
Consistency is the whole point.

---

# Non-negotiables — generate it right the first time

These rules exist so no audit/correct cycle is needed. Follow them **while authoring**;
they are the exact things a review keeps catching. Each design file must satisfy all of
them before it's considered done.

1. **Link the tokens; never redefine them.** Every standalone file links the canonical
   stylesheet (`site/tokens.css` or `lib/tokens.css`) in `<head>`. Never open a local
   `:root{}` that re-declares `--paper` / `--ink` / `--accent` (etc.) as hex — that drift
   is the single most common bug. If a file wants its own short var names (a deck's
   `--disp`, `--kanji`), **alias them onto the tokens** (`--disp: var(--font-display)`),
   never onto raw hex.

2. **Color = tokens only.** No `#hex`, no `rgba()`, no ad-hoc `oklch()` in design code. Map:
   - page background → `--paper`; the canvas/desk *behind* artboards → `--paper-2`
   - any hairline / border → `--paper-edge` (or `border-1px border-paper-edge`, `--hairline`) — never `rgba(40,30,20,.x)`
   - raised card → `--paper-2` (soft) · sunken well / toggle track → `--paper-3` (mute)
   - text → `--ink / --ink-2 / --ink-3 / --ink-4`; light text **on an ink surface** → `--on-primary-soft` / `--on-primary-mute`
   - status fill / edge → `--success` `-soft` `-edge`, `--warning` `-soft` `-edge`, `--danger` `-soft` `-edge`, `--accent` `-soft` `-edge`
   - shadow (only on things that physically lift — modals, popovers) → `--shadow-sm / --shadow / --shadow-lg`; **cards and buttons never shadow**

3. **Type = the 8-stop scale only.** Use `--text-xs…4xl` (or `text-*` utilities / the named
   roles `zs-h1/2/3`, `zs-hero`, `zs-body`, `zs-meta`, `zs-eyebrow`). Floor is `xs` (11px) —
   never smaller, never a literal `font-size`. Weights: `font-light/normal/medium/semibold` only.

4. **Spacing = the 4px, 9-stop grid only.** Use `--space-0…9` (or `p-*/px-*/py-*/m-*/gap-*`).
   No `18px`, `26px`, `5px`. Lay groups out with flex/grid + `gap-*`, not per-child margins.

5. **Prefer utility classes over custom-class + `var()`.** Wrap the root in `class="zs"`
   (or `.sensei`) and build with the utilities (`flex`, `grid`, `gap-4`, `p-5`, `text-sm`,
   `text-ink-2`, `bg-paper-soft`, `border-b`, `rounded-lg`) — this is what translates cleanly
   to implementation. Reserve a custom `<style>` rule **only** for geometry the system
   doesn't model: specific `grid-template-columns`, `aspect-ratio`, `::before` accent bars,
   absolutely-positioned diagram/timeline nodes. Those custom rules still use `var(--*)` for
   every color, size, and space value.

6. **Reuse the shared components; never hand-roll a duplicate.** A button is
   `zs-btn zs-btn-primary|secondary|ghost` (+ `zs-btn-sm|lg`) — not a bare styled `<button>`.
   Same for `zs-badge` (+ `-success/-warning/-accent`), `zs-card`, `zs-input`,
   `zs-dot` (+ `-accent/-success/-warning`), `zs-chrome`. If a needed variant is missing
   (e.g. a danger badge), add it **once** as `zs-badge zs-badge-…` driven by tokens — don't
   scatter one-off bare elements. Card radius is `--radius-lg`; `--radius-full` is for pills,
   dots, avatars and toggle tracks **only** — never a wide card.

7. **The only allowed literals (nothing else):**
   - Full-screen decks (1920×1080): display/hero/watermark type may exceed `--text-4xl`, and
     slide gutters may exceed `--space-9` — the scales are *UI* scales; a full-bleed slide
     needs more. Everything in-scale still uses tokens.
   - `@media print { body { background: #fff } }` may use white.
   - A `deck-stage` letterbox background may be a near-black chrome color.
   - A giant watermark kanji at low opacity may use `oklch(1 0 0 / .05)` on an ink surface.

**Pre-delivery self-check (grep your own file):** no `#hex` outside rule 7 · no `rgba(` ·
no `oklch(` in design code · no literal `font-size:` · no `px` inside `padding|margin|gap`.
If any appear, map them to tokens before delivering.
