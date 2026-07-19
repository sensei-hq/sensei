# Spacing cleanup — designer instructions

Same discipline as the type/token cleanup you just did, now for **spacing** (padding,
margin, gap, inset). Audit the mockups and snap every off-grid spacing value to the
canonical 4px scale, so the mockups become the **grid-aligned source of truth** the app
/ dōjō / website implementations then port to. Companion to
[`STYLING.md`](./STYLING.md) and this folder's `CLAUDE.md`.

## The canonical scale — use these stops only

The Zen-Sumi 4px scale (matches `Zen-Sumi Design System/colors_and_type.css` `--space-*`
and the mockup `CLAUDE.md`):

| step | px | | step | px |
|---|---|---|---|---|
| `space-0` | 0 | | `space-5` | **24** |
| `space-1` | 4 | | `space-6` | 32 |
| `space-2` | 8 | | `space-7` | 48 |
| `space-3` | 12 | | `space-8` | 64 |
| `space-4` | 16 | | `space-9` | 96 |

**The scale skips 20.** `5 = 24`, not 20. If you're reaching for 18–20px, use **16 or
24** — decide which by eye, don't split the difference. The jump from `space-4` (16) to
`space-5` (24) is intentional (density vs. air).

## What to change

Every `padding` / `margin` / `gap` / inset with a literal px **not on the scale** → snap
to the nearest stop (`p-N` `px-N` `py-N` `gap-N` `m*-N`, or `var(--space-N)` in CSS).
Nearest-step, ties → the **smaller** stop (keep density):

| current px | → stop |
|---|---|
| 2, 3 | `space-1` (4) |
| 5, 6, 7 | `space-2` (8) |
| 9, 10, 11 | `space-3` (12) |
| 13, 14, 15, 16, 17, 18 | `space-4` (16) |
| 19, 20, 21, 22, 23, 24, 25, 26, 27, 28 | `space-5` (24) |
| 29 – 40 | `space-6` (32) |
| 41 – 56 | `space-7` (48) |
| 57 – 80 | `space-8` (64) |
| > 80 | `space-9` (96) |

**Asymmetric padding snaps per axis:** `padding: 7px 9px` → `8px 12px` (`py-2 px-3`);
`padding: 15px 18px` → `16px 16px` (`p-4`); `padding: 5px 10px` → `4px 12px`
(`py-1 px-3`).

## Interactive elements (buttons, inputs, chips, nav rows)

Snap these too — they're the biggest source of off-grid values (`7px 9px`, `8px 13px`).
If a control looks wrong at grid padding, fix the **box model** (line-height, explicit
height, icon gap) rather than reintroducing an off-grid pad. A control at `py-2 px-3`
(8/12) with the right line-height reads the same as a hand-tuned `7px 11px`.

## Deliverable

1. Mockups with **100% on-scale spacing**.
2. An **exceptions list** — any spacing value where no stop works and why. Don't
   hand-code around it; flag it. In particular: **if 20px turns out to be genuinely
   needed across many components, say so** — that's the signal to either add a stop or
   standardize on 16/24. (This resolves an open question: UnoCSS's default scale has
   `p-5 = 20`, but our design scale has `5 = 24`. Your call in the mockups decides which
   the implementation configs adopt.)

## Then — implementation alignment (my side)

Once the mockups are grid-clean, I align the code: set `theme.spacing` in each
`uno.config.js` to the finalized scale (so `p-5 = 24`, etc. — §1.8 config parity), then
port the ~179 dōjō + website inline/`<style>` paddings to the utilities/vars, matching
the mockups value-for-value. The mockups are the source of truth for the targets — which
is why resolving spacing there first makes the code pass mechanical instead of
guesswork.
