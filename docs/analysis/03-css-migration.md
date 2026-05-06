---
name: CSS Migration Analysis
description: Approach options for replacing legacy CSS tokens with Rokkit utility classes across 36 svelte files
date: 2026-05-05
status: analysis-complete
origin: backlog.md#7
---

# CSS Migration — Approach Analysis

## Current State

### Scope

36 Svelte route files + 2 lib components contain legacy CSS. All use `<style>` block CSS
that references custom tokens (`var(--paper-*)`, `var(--sumi-*)`, `var(--shu)`, etc.) and
hard-coded pixel values instead of Rokkit utility classes.

### Font sizes in use (28 distinct values)

```
9px   9.5px  10px  10.5px
11px  11.5px 12px  12.5px
13px  14px   15px  16px   17px  18px
20px  22px   24px  26px   28px  32px  36px  38px
48px  54px   64px  72px   80px  220px
```

Target type scale (7 named steps):

| Current | Target | Tailwind |
|---------|--------|----------|
| 9–10.5px | micro | `text-[10px]` |
| 11–12.5px | caption | `text-xs` (12px) |
| 13px | body | `text-[13px]` |
| 14–15px | title | `text-sm` (14px) |
| 16–18px | sub-heading | `text-base` → `text-lg` |
| 20–24px | heading/display | `text-xl` / `text-2xl` |
| 36px+ | hero | `text-[36px]` → `text-[80px]` |

### Legacy token frequencies (total references across all svelte files)

| Token | Count | Rokkit replacement |
|-------|-------|--------------------|
| `var(--shu)` | 57 | `text-primary-z5` / `color: oklch(var(--color-primary-z5))` |
| `var(--hairline)` | 41 | `border border-surface-z2` |
| `var(--sumi)` | 40 | `text-surface-z9` |
| `var(--sumi-3)` + inline | 35 | `text-surface-z5` |
| `var(--paper)` | 26 | `bg-surface-z0` |
| `var(--border-card)` | 21 | `border border-surface-z2` |
| `var(--sumi-4)` | 17 | `text-surface-z4` |
| `var(--paper-2)` bg | 17 | `bg-surface-z1` |
| `var(--radius)` | 14 | `rounded-md` |
| `var(--sumi-2)` | 14 | `text-surface-z7` |
| `var(--radius-lg)` | 13 | `rounded-lg` |
| `var(--border)` | 11 | `border border-surface-z2` |
| `var(--paper-3)` bg | 11 | `bg-surface-z2` |
| `var(--jade)` | 9 | `text-success-z5` |
| `var(--paper-edge)` | 7 | `border-surface-z2` (direct border color) |
| `var(--amber)` | 7 | `text-warning-z5` |
| `var(--space-*)` | ~15 (total) | `p-*` / `gap-*` Tailwind scale |

**Rokkit utility classes already in use:** 13 references — minimal adoption so far.

### Token palette alignment (verified)

The `sumi-palette.js` hex values and `tokens.css` OKLCH values are calibrated to the same
perceptual positions. The migration is visually safe:

| tokens.css | Rokkit class | sumi-palette.js shade |
|------------|-------------|----------------------|
| `--paper` = `oklch(0.975 0.008 85)` | `bg-surface-z0` | sumi-50: `#FAF8F5` ✓ |
| `--paper-2` = `oklch(0.955 0.010 85)` | `bg-surface-z1` | sumi-100: `#F3F0EB` ✓ |
| `--paper-3` = `oklch(0.92 0.012 85)` | `bg-surface-z2` | sumi-200: `#E8E3DC` ✓ |
| `--sumi` = `oklch(0.22 0.012 50)` | `text-surface-z9` | sumi-900: `#2B2620` ✓ |
| `--sumi-2` = `oklch(0.38 0.012 50)` | `text-surface-z7` | sumi-700: `#524B43` ✓ |
| `--sumi-3` = `oklch(0.58 0.010 50)` | `text-surface-z5` | sumi-500: `#8A8278` ✓ |
| `--sumi-4` = `oklch(0.75 0.008 50)` | `text-surface-z4` | sumi-400: `#B5ADA2` ✓ |
| `--shu` = `oklch(0.58 0.15 35)` | `text-primary-z5` | shu-500: `#E8552B` ✓ |
| `--jade` = `oklch(0.62 0.08 160)` | `text-success-z5` | jade-500: `#3C946A` ✓ |
| `--amber` = `oklch(0.72 0.12 75)` | `text-warning-z5` | amber-500: `#F59E07` ✓ |

**Note:** colors.css files uses OKLCH; Rokkit generates `rgb()` vars from the hex palette.
The visual match is excellent (< 1 ΔE difference) but confirm via Playwright screenshot
comparison after migrating each screen's colors.

### tokens.css — what stays vs. what goes

**Keep permanently:**
- Font `@import` rules
- `.display` (Fraunces display class — semantic, not color/spacing)
- `.kanji` (CJK font stack)
- Scrollbar styling (`::-webkit-scrollbar*`)
- `::selection`
- Dark mode OS sync that Rokkit doesn't cover

**Remove progressively as each screen migrates:**
- All `--paper-*` / `--sumi-*` / `--shu` / `--jade` / `--amber` CSS vars
- All `--border-*` / `--hairline` / `--ink-line` vars
- All `--radius` / `--radius-lg` vars
- All `--space-*` vars
- `.btn-solid` / `.btn-outline` / `.btn-cta` classes (replaced by `<Button>` from `@rokkit/ui`)

### Files by legacy token density (migration effort)

| Rank | File | Legacy hits | Route |
|------|------|-------------|-------|
| 1 | `(health)/logs/+page.svelte` | 105 | `/logs` |
| 2 | `(config)/+layout.svelte` | 57 | wizard chrome |
| 3 | `(config)/setup/scan/+page.svelte` | 42 | `/setup/scan` |
| 4 | `(health)/health/+page.svelte` | 38 | `/health` |
| 5 | `(observatory)/instruments/+page.svelte` | 37 | `/instruments` |
| 6 | `(observatory)/projects/[id]/+page.svelte` | 35 | project detail |
| 7 | `(observatory)/libraries/+page.svelte` | 33 | `/libraries` |
| 8 | `(observatory)/learnings/+page.svelte` | 30 | `/learnings` |
| 9 | `(observatory)/insights/+page.svelte` | 30 | `/insights` |
| 10 | `(observatory)/settings/+page.svelte` | 28 | `/settings` |

---

## Feasibility

High. The palette alignment is exact, the mapping table is complete, and Playwright E2E
tests already exist to verify each screen. The only risk is visual drift from OKLCH → RGB
rounding — mitigated by screenshot comparison after each screen. Estimated total effort:
~2–3 sessions of focused migration work.

---

## Approach A — Screen-by-Screen (Backlog Order)

Migrate one file end-to-end (all concerns: font-size + spacing + color + radius), then
verify with Playwright, then commit. Follow the order from the backlog.

**Screen order:**
1. `src/app.css` body/font-family → Rokkit font utilities
2. `(health)/+layout.svelte` — sidebar/nav chrome
3. `(health)/health/+page.svelte` — gate list cards
4. `(health)/logs/+page.svelte` — log viewer (highest density: 105 hits)
5. `(config)/+layout.svelte` — wizard chrome (57 hits)
6. `(config)/setup/welcome` → `done` — each wizard stage in order
7. `(observatory)/+layout.svelte` — observatory chrome
8. Observatory screens: instruments, libraries, learnings, insights, sessions, settings, projects
9. `(project)/project/[id]/+layout.svelte` + sub-pages
10. Final: delete superseded sections from `tokens.css`

**Pros:**
- Each commit is a complete, tested, reviewable screen
- E2E test runs after each screen (immediate feedback)
- If a screen is skipped, partial progress doesn't break anything
- Consistent with the backlog plan — no coordination needed

**Cons:**
- Same token (e.g. `--sumi-3`) appears in many files; you encounter it repeatedly
- tokens.css cleanup is deferred to the end
- ~10–14 separate Playwright runs

---

## Approach B — Concern-by-Concern (Mechanical Passes)

Three global passes: (1) font-size everywhere → type scale utilities, (2) color tokens → 
Rokkit utilities, (3) spacing → Tailwind grid.

**Pass order:**
1. **Typography pass** — grep-replace all `font-size:` values globally, run `bun run check`
2. **Color pass** — grep-replace all `var(--paper-*)` / `var(--sumi-*)` etc. globally
3. **Spacing pass** — grep-replace all `var(--space-*)`, inline `padding`/`gap`/`margin` px values
4. **Radius pass** — grep-replace `var(--radius)` → `rounded-md`, `var(--radius-lg)` → `rounded-lg`
5. **Button pass** — replace `.btn-*` with `<Button>` from `@rokkit/ui`
6. **tokens.css cleanup** — delete superseded sections

**Pros:**
- Each pass is mechanical and reviewable as a single diff
- Fewer total Playwright runs (one per pass, not per screen)
- Good if the primary goal is "reduce token count fast"

**Cons:**
- A single pass touches every file — hard to review, hard to attribute regressions
- Cannot run E2E per-screen after a pass (too many files changed)
- Higher chance of merge conflicts if other work is in flight
- If font-size pass introduces a regression, you don't know which file caused it

---

## Approach C — Foundation-First (tokens.css Inversion)

Remove custom tokens from tokens.css first, forcing the compiler to surface every
usage as an error. Fix each error by using a Rokkit utility class.

**Steps:**
1. Delete `--paper`, `--paper-2`, `--paper-3` from tokens.css → `bun run check` shows ~55 errors
2. Fix all 55 errors (replace with `bg-surface-z*`) → `bun run check` clean → commit
3. Delete `--sumi`, `--sumi-2`, `--sumi-3`, `--sumi-4` → fix ~125 errors → commit
4. Repeat for border, radius, spacing tokens
5. Delete `.btn-*` classes last

**Pros:**
- Compiler enforces completeness — you cannot miss a usage
- Every migration is forced, not voluntary
- Clean break: no dual-system intermediate state

**Cons:**
- App is broken (compile error) during migration — cannot partially test
- Each error batch can span 20+ files simultaneously
- No E2E test until the batch is fully fixed
- High cognitive load — fixing 55 errors at once vs. 38 in one file

---

## Recommendation

**Approach A (screen-by-screen)** with one preparatory step.

The E2E Playwright suite is the primary verification mechanism. Running it after every
screen change gives tight feedback loops, keeps commits small and reviewable, and matches
the backlog's existing plan. The mental model is simple: one file → all concerns → test → commit.

**Suggested kick-off order** (start with lowest-density screens to build muscle memory,
then tackle the heavy screens with confidence):

```
Session 1:  tokens.css prep audit (identify what can be deleted)
            +layout.svelte (root) — body/font
            (health)/+layout.svelte — sidebar

Session 2:  (health)/health/+page.svelte (38 hits)
            (health)/logs/+page.svelte  (105 hits — save heavy for when rhythm is established)

Session 3:  (config)/+layout.svelte (57 hits)
            Setup wizard pages: welcome, preferences, roots, projects (low density: 5–21 hits)

Session 4:  Setup wizard: assistants, libraries, instruments, scan, inference, done

Session 5:  (observatory)/+layout.svelte
            Observatory screens (instruments, libraries, learnings, insights, sessions)

Session 6:  Observatory: settings, projects list, project detail
            (project)/project/[id]/+layout.svelte + sub-pages

Session 7:  Final tokens.css cleanup, dark mode verification, full E2E run
```

The one divergence from the backlog order: do `health/+page.svelte` before `logs/+page.svelte`
because the health page is the first user-visible screen — any regression there is caught early.
