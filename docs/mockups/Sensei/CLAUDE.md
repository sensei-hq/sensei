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
