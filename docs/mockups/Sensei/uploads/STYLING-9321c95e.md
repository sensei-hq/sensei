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

## Non-negotiables — generate it right the first time

These rules exist so no audit/correct cycle is needed. Follow them **while authoring**.
Every file must satisfy all of them before it's done.

1. **Link the tokens; never redefine them.** Link the canonical stylesheet
   (`site/tokens.css` / `lib/tokens.css`, or inline `colors_and_type.css` in a strict-CSP
   artifact). Never open a local `:root{}` that re-declares `--paper`/`--ink`/`--accent` as
   hex — that drift is the #1 bug. A file that wants its own short var names (a deck's
   `--disp`, `--kanji`) must **alias them onto the tokens** (`--disp: var(--font-display)`),
   never onto raw hex.

2. **Color = tokens only.** No `#hex`, `rgba()`, or ad-hoc `oklch()` in design code.
   - page bg → `--paper`; canvas/desk behind artboards → `--paper-2`
   - hairline / border → `--paper-edge` (or `border-1px border-paper-edge`) — never `rgba(40,30,20,.x)`
   - raised card → `--paper-2` (soft) · sunken well / track → `--paper-3` (mute)
   - text → `--ink / --ink-2 / --ink-3 / --ink-4`; light text on an ink surface → `--on-primary-soft` / `--on-primary-mute`
   - status → `--success/--warning/--danger/--accent` (+ `-soft` fill, `-edge` border)
   - shadow (only things that lift) → `--shadow-sm/--shadow/--shadow-lg`; cards & buttons never shadow

3. **Type = the 8-stop scale only** (`text-xs…3xl` / `--text-*` / `zs-h1/2/3`, `zs-hero`,
   `zs-body`, `zs-meta`, `zs-eyebrow`). Floor is `xs` (11px); no smaller, no literal `font-size`.

4. **Spacing = the 4px grid only** (`p-*/px-*/py-*/gap-*/m-*` / `--space-*`). No `18px`, `5px`.
   Groups use flex/grid + `gap-*`, not per-child margins.

5. **Utility classes over custom-class + `var()`.** Wrap the root in `class="zs"` and build
   with utilities — it translates cleanly to implementation. Keep a custom `<style>` rule
   only for geometry the system can't model (`grid-template-columns`, `aspect-ratio`,
   `::before` accent bars, absolute diagram nodes); those still use `var(--*)` for every value.

6. **Reuse shared components; never hand-roll a duplicate.** Button = `zs-btn zs-btn-primary|
   secondary|ghost` (not a bare styled `<button>`); likewise `zs-badge` (+`-success/-warning/
   -accent`), `zs-card`, `zs-input`, `zs-dot`. Missing variant → add it once as a `zs-*`
   modifier from tokens, don't scatter one-offs. Card radius is `rounded-lg`; `rounded-full`
   is for pills/dots/avatars/toggle tracks only — never a wide card.

7. **Only allowed literals:** full-screen decks may use display type > the top stop and gutters
   > the top space stop; `@media print{body{background:#fff}}`; a `deck-stage` letterbox chrome
   color; a low-opacity watermark kanji (`oklch(1 0 0/.05)`) on ink. Nothing else.

**Pre-delivery self-check (grep the file):** no `#hex` outside rule 7 · no `rgba(` · no
`oklch(` in design code · no literal `font-size:` · no `px` in `padding|margin|gap`.
