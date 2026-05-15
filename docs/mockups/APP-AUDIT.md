# Sensei App — Component & Styling Audit

> Goal: identify which canonical mockup components (PageHeader, ListRow, Card, …) already exist in the SvelteKit app, which are duplicated inline across routes, and which still need to be built — preferring Rokkit primitives when one is available.
>
> Scope: `/Users/Jerry/Developer/sensei-hq/sensei/app/src/**` and surrounding config (`rokkit.config.js`, `uno.config.js`, `svelte.config.js`, `app.css`, `app.html`, `tokens.css`). 34 +page.svelte files, ~3,500 lines of route markup.
>
> Snapshot date: 2026-05-15. Svelte 5 (runes), UnoCSS + `@rokkit/unocss` preset, `@rokkit/themes/zen-sumi.css`, Tauri 2, SPA mode (`adapter-static` + `index.html` fallback).

---

## 1. Inventory of existing shared components

All under `src/lib/components/` (re-exported via `src/lib/components/index.ts`).

| File | LoC | What it does | Used by (routes) | Notes |
|---|---:|---|---|---|
| `EmptyState.svelte` | 19 | Vertical centered placeholder — large kanji glyph at 30% opacity, title (display 17/normal), descriptive paragraph (text-ui surface-z6). Three required props: `kanji`, `title`, `description`. | `(observatory)/sessions/+page.svelte` (×1), `(observatory)/libraries/+page.svelte` (×1), `(observatory)/instruments/+page.svelte` (×4 — inside `{#snippet ToolInsights}`), used internally by `MemoryList` | Already maps cleanly to the canonical **EmptyState** in mockup audit. Only consumed in observatory routes — project subroutes re-implement the pattern inline (see §2). |
| `FolderInput.svelte` | 40 | Path input + `Add` button + folder-picker icon-button that calls Tauri `pick_folder`. Emits `onadd(path)`. | **NOT USED ANYWHERE in routes** (verified with grep). | Dead code? `setup/roots/+page.svelte` re-implements this inline (lines 50–62, no Tauri picker). Either adopt it or delete it. |
| `MemoryList.svelte` | 161 | Page-level component: eyebrow + title (`pageLabel`/`pageTitle` props) → TabBar (4 tabs: all/memories/patterns/corrections) → split list+detail (`grid-cols-[1fr_340px]`). Cards show 5-dot strength + scope. **Calls no API — `memories` is empty state only.** | `(observatory)/insights/+page.svelte`, `(observatory)/learnings/+page.svelte` (both 5-line files that just render `<MemoryList ... />`). | Over-coupled — bundles PageHeader, TabBar, ListRow, Card, EmptyState. Needs to be split into those parts and replaced by composition. |
| `Switch.svelte` | 50 | Toggle switch (36×20px pill, animated thumb). `bind:value` + `label`. | `(config)/setup/preferences/+page.svelte` (×4). | Could be replaced by Rokkit's `@rokkit/ui` `Switch.svelte` or `Toggle.svelte`. |
| `TabBar.svelte` | 31 | Tab strip with `tabs: [key, label][]` + `bind:active`. Active tab gets vermillion underline. | `(observatory)/instruments/+page.svelte`, `(observatory)/settings/+page.svelte`, `(observatory)/projects/[id]/+page.svelte`, used internally by `MemoryList`. | Could be replaced by Rokkit's `@rokkit/ui` `Tabs.svelte`. |
| `WizardShell.svelte` | 230 | Full setup shell: 220px rail + content + bottom nav. Takes `stages: Stage[]` and `current`. Has CSS for rail items, watermark glyph, bottom nav. | **NOT USED** — the actual setup wizard lives in `(config)/+layout.svelte` (255 lines, same idea but driven by `STAGES` from `(config)/stages.ts`). | Looks like a previous wizard implementation that was abandoned in favor of file-routing. Either retire or repurpose for non-routing use cases. |

### Health/route-local components

Treated as page parts, not shared:

- `routes/(health)/health/Header.svelte` — bootstrap eyebrow + huge headline; effectively a **PageHeader variant** with platform context.
- `routes/(health)/health/Hero.svelte` — large status card with circular indicator + headline + button. Component-of-Card.
- `routes/(health)/health/Ledger.svelte` — list of components with status dots + badges. Effectively a **ListRow** sequence.
- `routes/(health)/health/Remedy.svelte` — bordered card with header / `<pre>` / footer with two buttons.
- `routes/(health)/health/HealthView.svelte` — page-shell composer.

These are good examples of the granularity the rest of the app is missing. They have spec.svelte.ts test files alongside (see §5) and use `interface Props` for props typing.

### Other UI logic in `lib/` (state, not components)

`appstate.svelte.ts`, `wizard-state.svelte.ts`, `health-state.svelte.ts`, `scan-state.svelte.ts`, `repos.svelte.ts`, `api.ts`, `events.ts`, `logger.ts`, `health-cache.ts`, `health-transport.ts`, `bootstrap.ts`, `setup/*` (loaders + contracts + mock data), `stores/windows.svelte.ts`. Out of scope for this audit.

### Near-duplicates already on disk

- **Two switch implementations**: local `Switch.svelte` (50 LoC) AND `@rokkit/ui/Switch.svelte` + `Toggle.svelte` (installed but unused).
- **Two tab implementations**: local `TabBar.svelte` AND `@rokkit/ui/Tabs.svelte`.
- **Two wizards**: `lib/components/WizardShell.svelte` (unused) AND `routes/(config)/+layout.svelte` (the live wizard).
- **Two folder inputs**: `lib/components/FolderInput.svelte` (unused) AND inline implementation in `setup/roots/+page.svelte` (lines 50–62) — neither is reused elsewhere, both differ.
- **Two sparkline functions**: identical `sparklinePath(points, w, h)` definition copy-pasted in `(observatory)/+page.svelte:28-40` and `(project)/project/[id]/overview/+page.svelte:9-21`.

---

## 2. Repeating patterns in route files — extraction candidates

Quantified by grep + manual inspection. "Inline" means the pattern is written out in route markup, not consumed via a component import.

### PageHeader (eyebrow + display title)

The pattern is consistent across observatory pages — a small uppercase tracking-loose eyebrow above a `display text-2xl font-normal` title, often with a kanji prefix in the title.

Inline occurrences:

- `(observatory)/+page.svelte:46-53` — date eyebrow + greeting headline (display-3xl variant).
- `(observatory)/sessions/+page.svelte:62-67` — "Sessions" / "刻 Sessions".
- `(observatory)/libraries/+page.svelte:46-51` — "Libraries" / "書 Libraries".
- `(observatory)/instruments/+page.svelte:49-54` — "Instruments" / "具 Instruments".
- `(observatory)/settings/+page.svelte:53-59` — "Settings" / "設 Settings".
- `(observatory)/projects/[id]/+page.svelte:58-93` — projectheader variant with kanji, name, maturity pill, goal + stack chips.
- `(observatory)/projects/+page.svelte:7` — bare `<h2>` only (drift from convention).
- `(observatory)/help/+page.svelte:2` — `text-2xl font-normal` without the eyebrow (drift).
- `MemoryList.svelte:43-48` — same pattern, accepts pageLabel + pageTitle props (the only "componentized" PageHeader).
- `(config)/+layout.svelte:140-153` — eyebrow + `display text-4xl font-light` title (wizard step variant).
- `(health)/health/Header.svelte` — eyebrow + 4xl display title (componentized but Health-specific).
- `(health)/upgrade/+page.svelte:124-139` — same pattern again, inline.
- `(project)/project/[id]/+layout.svelte:80-84` — titlebar variant with kanji + name in drag-region.
- Every `(project)/project/[id]/*/+page.svelte` (8 files) starts with `<h2 class="text-xl font-normal m-0 mb-X">` — a smaller PageHeader variant without eyebrow.

**Verdict:** ≥14 distinct inline implementations. A `PageHeader` component with slots/props for eyebrow / kanji / title / sizeVariant (`xl|2xl|3xl|4xl`) / right-slot would replace all of them.

### SectionHeader (smaller block heading within a page)

Pattern: `text-sm font-medium m-0 mb-3.5 text-surface-z9` (project subpages) or `display text-base font-normal m-0` (observatory hero columns) or `text-xs font-semibold opacity-60 m-0 mb-2 uppercase tracking-wider`.

- `(observatory)/+page.svelte:131-132, 154-155` — "Also worth noticing" / "System has learned".
- `(observatory)/projects/[id]/+page.svelte:101, 134-136, 173` — Repositories / Recommendations / Anti-patterns / Followed patterns.
- `(project)/project/[id]/about/+page.svelte:14-18, 36-40` — Repos / Stack (uppercase variant).
- `(project)/project/[id]/patterns/+page.svelte:9-13, 30-33` — Followed / Anti-patterns.
- `(project)/project/[id]/overview/+page.svelte:125, 152, 169` — Recent sessions / Hotspots / Adopted teachings.
- `(health)/health/Ledger.svelte:38` — "what this resolves" (tracking-tag uppercase).
- `(health)/upgrade/+page.svelte:211` — "upgrade steps" (tracking-tag uppercase).
- `(config)/+layout.svelte:73` — "Setup" (tracking-tag uppercase).
- `(config)/setup/done/+page.svelte`, `welcome/+page.svelte` — pillar cards each have a smaller section heading.

**Verdict:** ≥3 distinct heading styles (display-base, sm-medium, xs-semibold-uppercase). A `SectionHeader` with `tone="display"|"label"|"caps"` would consolidate.

### Eyebrow (small uppercase label)

`text-2xs tracking-loose uppercase text-surface-z6` / `text-micro tracking-label uppercase text-surface-z6` / `text-3xs tracking-cap uppercase`.

Counted **20+ inline instances** across `(observatory)/+page.svelte`, `sessions/+page.svelte`, `libraries/+page.svelte`, `instruments/+page.svelte`, `settings/+page.svelte`, `(observatory)/projects/[id]/+page.svelte`, `(health)/upgrade/+page.svelte`, `(health)/health/*`, `(health)/logs/+page.svelte`, `(config)/+layout.svelte`, `setup/welcome`, `setup/done`. Tracking value drifts (`tracking-loose`, `tracking-tag`, `tracking-cap`, `tracking-label`, `tracking-widest`) — see §3 token drift.

**Verdict:** one `Eyebrow` (or `Caption`) component with a fixed tracking token would erase the drift and replace 20+ inline spans.

### Kanji glyph

Pattern `<span class="kanji text-{size} text-primary-z5 [opacity-…]">{char}</span>`. Used in nav items, page headers, watermarks, empty-state hero, recommendation cards, status indicators. Count of `class="kanji ` strings:

- `(observatory)/+layout.svelte` — 5 (sidebar, project list, watermark variants)
- `(observatory)/+page.svelte` — 4
- `(observatory)/projects/[id]/+page.svelte` — 5
- `(config)/setup/preferences/+page.svelte` — 4 section headers
- 24 other files with 1–3 each.

**Verdict:** a `Kanji` component (`char`, `size`, `tone` = `accent|muted|watermark`) would centralize the opacity/color rules currently sprinkled inline.

### Card (bg-surface-z2 + border + rounded-lg)

Inline (`bg-surface-z2 rounded-lg` or `bg-surface-z2 border border-surface-z3 rounded-lg`):

- `(observatory)/+page.svelte:92, 110, 147, 160` — hero cards, recommendation rows.
- `(observatory)/sessions/+page.svelte:71` — stats strip.
- `(observatory)/libraries/+page.svelte:105` — detail panel.
- `(observatory)/instruments/+page.svelte:93` — tool detail panel.
- `(observatory)/settings/+page.svelte:69, 91, 138, 154` — section cards (×4).
- `(observatory)/projects/[id]/+page.svelte:139, 155, 184` — recommendations / graph / sessions placeholder cards.
- `(project)/project/[id]/overview/+page.svelte:34, 46, 53, 67, 74, 80, 84, 171` — stat blocks + teaching cards (×8).
- `(project)/project/[id]/impact/+page.svelte:62, 77, 83, 89, 100` — verdict detail + 4 micro-stat cells.
- `(config)/setup/roots/+page.svelte:69` — root entries.
- `(config)/setup/scan/+page.svelte:113, 162, 232` — empty card + project cards + activity feed.
- `(config)/setup/assistants/+page.svelte:35-39` — selectable cards (variant).
- `(config)/setup/projects/+page.svelte`, `setup/libraries/+page.svelte`, `setup/instruments/+page.svelte` — empty-card stubs (3 near-identical files).
- `(health)/health/Hero.svelte:26` — hero card.
- `(health)/health/Remedy.svelte:12` — remedy card.
- `(health)/upgrade/+page.svelte:159` — hero card.
- `(health)/logs/+page.svelte` — multiple small cards, modal panel.

**Verdict:** ≥30 inline cards. The canonical pattern is `bg-surface-z2 border border-surface-z3 rounded-lg p-{4|5|6}` with optional `border-l-2 border-l-primary-z5` ("hero" accent). Worth a `Card` with `tone="muted"|"hero"|"selectable"`, `padding="sm"|"md"|"lg"`, `border={"hairline"|"strong"|"none"}`. Rokkit has `@rokkit/ui/Card.svelte` — should be evaluated as the base.

### ListRow (hairline-separated row inside a list)

Pattern `border-b border-surface-z2` or `border-b border-surface-z3` on a flex/grid row, often with `:last-child { border-bottom: none }` rule in component CSS. Counted **22 occurrences** across:

- `(observatory)/+page.svelte:136` — "Also worth noticing" rows.
- `(observatory)/sessions/+page.svelte:109-133` — session-row.
- `(observatory)/libraries/+page.svelte:130-139` — usage rows.
- `(observatory)/instruments/+page.svelte:182` — tool stats row.
- `(observatory)/settings/+page.svelte:74, 106, 168` — setting/assistant/extension rows (×3).
- `(observatory)/projects/[id]/+page.svelte:111-128` — repo-row.
- `(project)/project/[id]/about/+page.svelte:21` — repo-row.
- `(project)/project/[id]/sessions/+page.svelte:10` — session-row.
- `(project)/project/[id]/memories/+page.svelte:17` — memory-row.
- `(project)/project/[id]/libraries/+page.svelte:22` — lib-row.
- `(project)/project/[id]/instruments/+page.svelte:19` — tool-row.
- `(project)/project/[id]/patterns/+page.svelte:16, 38` — pattern-row (×2).
- `(project)/project/[id]/traceability/+page.svelte:20` — drift-row.
- `(project)/project/[id]/overview/+page.svelte:130, 154` — session/hotspot rows.
- `(project)/project/[id]/impact/+page.svelte:130` — session row inside.
- `(health)/health/Ledger.svelte:42` — component row.
- `(health)/upgrade/+page.svelte:218` — step row.
- `(health)/logs/+page.svelte:200` — trace row.

Each file defines its own `.X-row:last-child { border-bottom: none }` rule (10+ duplicated CSS blocks).

**Worst offenders for inline ListRow:** all 8 `(project)/project/[id]/*/+page.svelte` files (excluding overview) each have a near-identical 3-column flex row with hairline. They are the most direct candidates for `ListRow` + `slot:left|center|right` extraction.

### Sidebar / NavItem

The observatory sidebar (`(observatory)/+layout.svelte:48-193`) defines TWO Svelte `{#snippet}` macros — `navItem(item, collapsed)` and `projectItem(proj, collapsed)`. The project window sidebar (`(project)/project/[id]/+layout.svelte:73-115`) reimplements the same shape inline with a different class name (`proj-nav-item`). Logs sidebar (`(health)/logs/+page.svelte:48-118`) does it again with a custom date-grouped tree.

**Verdict:** three sidebar implementations sharing nothing. A `NavItem` (with `kanji`, `label`, `href`, `active`, `collapsed`) + a `Sidebar` shell would consolidate.

### TabBar / Filter chips

- `TabBar.svelte` (component) — used 3 times.
- Inline pill-style filter chips (`.filter-chip` class) duplicated in `(observatory)/sessions/+page.svelte:88-95 + style 140-147` and `(observatory)/libraries/+page.svelte:62-69 + style 153-160` — same exact pattern, ~15 lines each.
- `(config)/setup/preferences/+page.svelte` — segmented control (`.segment-btn`) used twice for "Correction tone" and "Morning digest". Effectively a TabBar variant.

**Verdict:** `ChipRow` + `Chip` (or `SegmentedControl`) component.

### Badge / Pill / Chip (inline rounded-full)

Inline `rounded-full` pills with `px-2|2.5|3` + `py-0.5|0.75|1` + `text-2xs|3xs` + `bg-surface-z3 text-surface-z6|7`.

- `(observatory)/projects/[id]/+page.svelte:74, 87, 124` — maturity, stack tag, repo role.
- `(observatory)/libraries/+page.svelte:119` — repo chip.
- `(project)/project/[id]/instruments/+page.svelte:24-28`, `libraries/+page.svelte:27-31` — `.scope-badge` with two variants.
- `(project)/project/[id]/about/+page.svelte:43` — stack tag.
- `(config)/setup/preferences/+page.svelte` — many.
- `(config)/setup/assistants/+page.svelte:60` — capability `.chip`.

**Verdict:** `Badge` (or `Pill`/`Chip`) with `tone="muted"|"accent"|"success"|"warning"`. Rokkit has `Badge.svelte` and `Pill.svelte`.

### StatusDot

Pattern `w-{1.5|1.75|2} h-{1.5|1.75|2} rounded-full` with bg derived from status.

- `(observatory)/sessions/+page.svelte:113` — `.ftr-dot` green/amber.
- `(observatory)/settings/+page.svelte:122-125` — `.status-dot` configured/unconfigured.
- `(project)/project/[id]/traceability/+page.svelte:25-38` — drift-row dot.
- `(health)/health/Ledger.svelte:44, 64-67` — `.dot-ready|running|blocked|pending`.
- `(health)/upgrade/+page.svelte:222-227` — same 4-state dot.
- `(health)/logs/+page.svelte:96` — session outcome dot.
- `(config)/+layout.svelte:120` — services dot.
- `(observatory)/+page.svelte:119` — bullet dot.

**Verdict:** `StatusDot` with `status="ok|busy|warn|fail|idle"` will eliminate 8+ duplications and the inline `oklch(var(--color-…)/1)` CSS rules each file currently carries.

### Button

`tokens.css` defines `.btn-solid`, `.btn-outline`, `.btn-cta` and `(config)/+layout.svelte` adds `.btn-primary`/`.btn-back`. `(observatory)/+layout.svelte` introduces yet another `.collapse-btn`. `(health)/logs/+page.svelte` adds `.report-btn` and `.outline-btn`. Each file restyles the same idea.

- Rokkit has `Button.svelte` and `ButtonGroup.svelte`.

**Verdict:** a `Button` component with `variant="solid|outline|ghost|cta"` + `size="sm|md|lg"` would replace the 5 different CSS classes used today.

### TextField / SearchField

- `(observatory)/libraries/+page.svelte:55-60` — `.lib-search` with focus styles.
- `(config)/setup/roots/+page.svelte:50-59` — `.folder-input`.
- `(config)/setup/preferences/+page.svelte:24-32` — `.name-input`.
- `FolderInput.svelte` — its own implementation (unused).
- `(observatory)/instruments/+page.svelte:114-120` — `.param-input` (×N in a loop).

All have near-identical CSS (`border border-surface-z3`, `bg-surface-z1|2`, focus → `border-surface-z6|7`).

**Verdict:** `TextField` component.

### EmptyState / "kanji + headline + sub" placeholder

`EmptyState.svelte` exists but only used 6 times. Inline reimplementations:

- `(observatory)/+page.svelte:91-101` — "Still listening" (uses larger 7xl kanji + card wrap).
- `(observatory)/projects/+page.svelte:9` — single `<p>`, no kanji.
- `(observatory)/projects/[id]/+page.svelte:51-55` — "Project not found."
- `(observatory)/projects/[id]/+page.svelte:138-150, 154-167, 184-195` — three "data not yet" placeholders (with kanji 薦/紋/刻).
- `(config)/setup/projects/+page.svelte:9-13`, `setup/libraries/+page.svelte:7-12`, `setup/instruments/+page.svelte:7-12` — three near-identical "data will appear" cards.
- `(config)/setup/scan/+page.svelte:112-125` — start-state with 探 kanji.

**Verdict:** extend `EmptyState` with optional `variant="inline"|"card"` + optional `cta` slot, and route everything through it.

### Drawer / SidePanel / Detail panel

Pattern `sticky top-6` 340-wide right column with details:

- `MemoryList.svelte:96-146` — memory detail.
- `(observatory)/libraries/+page.svelte:103-143` — library detail.
- `(observatory)/instruments/+page.svelte:92-151` — tool detail + form.
- `(project)/project/[id]/impact/+page.svelte:60-118` — verdict detail.
- `(health)/logs/+page.svelte` modal (different but related — a true Dialog).

**Verdict:** `SidePanel` component (sticky right-column variant) + a separate `Dialog`/`Modal` for logs.

### SplitPane / ListDetail layout

`grid-cols-[1fr_340px] gap-6` or `grid-cols-[260px_1fr]`:

- `MemoryList.svelte:61` and 4 observatory pages mentioned above + `(project)/project/[id]/impact/+page.svelte:24`.
- Inverted variant (`grid-cols-[260px_1fr]`) in `(observatory)/instruments/+page.svelte:68` and `(config)/+layout.svelte:62`.

**Verdict:** `SplitPane` or `ListDetail` layout component.

### Stat tile / MiniStat (display number + caption)

- `(observatory)/sessions/+page.svelte:73-84` — 4-up stats grid.
- `(observatory)/+page.svelte:55-87` — FTR stat with delta + sparkline.
- `(project)/project/[id]/overview/+page.svelte:52-87` — 4 stat tiles in a row.
- `(project)/project/[id]/impact/+page.svelte:75-110` — 4 small stat cells inside a card.
- `(config)/setup/scan/+page.svelte:128-141` — 4-up scan stats.
- `(project)/project/[id]/+layout.svelte:90-93` — FTR sidebar stat.

**Verdict:** `MiniStat` with `value`, `label`, optional `delta`, optional `sparkline`.

### Sparkline + ftr-up/down arrow

- `sparklinePath(points, w, h)` defined **identically** in `(observatory)/+page.svelte:28-40` and `(project)/project/[id]/overview/+page.svelte:9-21`. Same SVG plumbing both times.

**Verdict:** export `sparklinePath` from a util module + provide a `Sparkline` component wrapper.

### ActivityTimeline / Event row

- `(config)/setup/scan/+page.svelte:243-260` — SSE activity feed (timestamp + level + message).
- `(health)/logs/+page.svelte:194-313` — trace row + expandable detail.
- `(health)/logs/+page.svelte:61-117` — date-grouped session sidebar (collapsible).

**Verdict:** `ActivityRow` + `Timeline` shell.

### EnsoRing / BarRow / HairlineGrid

None present in the app yet. Mockup audit listed these — they're greenfield.

### TauriChrome (drag-region + title bar)

Inline in **every layout**:

- `(observatory)/+layout.svelte:51`, `(config)/+layout.svelte:60`, `(health)/+layout.svelte:6`, `(project)/project/[id]/+layout.svelte:80`.

Project window adds a 0.5px primary stripe + kanji + name + drag region. The other three just have `<div class="drag-region h-8 shrink-0"></div>`.

**Verdict:** `TauriChrome` component with `accent={true|false}`, `title?`, `kanji?`.

### Worst offenders — pages with the most copy-paste markup

Ranked by inline duplication of patterns above (not just LoC):

1. **`(health)/logs/+page.svelte` (511 lines)** — sidebar nav, ListRows, Cards, Eyebrows, status dots, modal, expandable rows — almost every canonical component, all inline.
2. **`(observatory)/projects/[id]/+page.svelte` (216 lines)** — PageHeader (kanji + maturity + stack chips), TabBar, 3 EmptyState variants, ListRow, SectionHeader.
3. **`(observatory)/settings/+page.svelte` (212 lines)** — PageHeader, TabBar (4 sections), 4 Cards each with rows of settings.
4. **`(project)/project/[id]/overview/+page.svelte` (212 lines)** — Hero card, 4 MiniStat tiles, Sparkline, signal pills, 3 SectionHeaders, ListRows, teaching cards.
5. **`(observatory)/instruments/+page.svelte` (217 lines)** — PageHeader, TabBar, EmptyState (×4), SplitPane, ListRow, form fields.
6. **`(observatory)/libraries/+page.svelte` (169 lines)** — PageHeader, search field, ChipRow filter, SplitPane, ListRow, detail panel, chips.
7. **`(observatory)/sessions/+page.svelte` (159 lines)** — PageHeader, stats strip, ChipRow filter, ListRow with status dot.
8. **`(observatory)/+page.svelte` (185 lines)** — PageHeader-3xl, MiniStat + Sparkline, hero card, two-column SectionHeader + ListRow, EmptyState variants.
9. **`(config)/setup/scan/+page.svelte` (264 lines)** — full activity feed, project progress cards, 4-up MiniStat row.
10. **`(config)/setup/preferences/+page.svelte` (240 lines)** — 4 sections × 3–4 rows each, segmented control, Switch + select fields.

The `(project)/project/[id]/*/+page.svelte` family is **8 stub pages** that all reimplement: title, optional eyebrow line, hairline list — they would each shrink to 10–20 lines once `PageHeader` + `ListRow` exist.

---

## 3. Tooling & styling system

### Toolchain

- **UnoCSS** via `@unocss/vite`, configured in `uno.config.js` with `presetRokkit(rokkitConfig)` only. No Tailwind, no preflight beyond `@unocss/reset/tailwind.css`. `transformerDirectives()` enabled (so `@apply`-style is allowed).
- **Rokkit** via `@rokkit/themes/zen-sumi.css` (provides `--color-surface-z0…z9`, `--color-primary-z0…z9`, `--color-success/warning/danger-z*`) and `@rokkit/unocss` (generates the `bg-surface-z*` / `text-primary-z*` / `border-*-z*` utilities + the `data-mode="dark"` swap).
- `app.css` imports the reset + Rokkit base + Rokkit zen-sumi + `$lib/tokens.css`. Total: ~50 lines including the `.drag-region`, `.sidebar-vibrancy`, `.ink-dot` helpers.
- `app.html` boots `data-mode` from system preference + localStorage; this is the dark-mode trigger.

### Tokens used in routes (UnoCSS classes seen)

Custom UnoCSS extensions (uno.config.js):

- Font sizes: `text-nano|micro|3xs|2xs|ui|body|prose|wm`. **`text-ui` (13px) and `text-2xs` (11px) carry the majority of UI copy** — confirmed by the mockup audit.
- Letter-spacing: `tracking-cap|tag|label|loose` — **but routes mix these with Tailwind's `tracking-wider`/`tracking-widest`/`tracking-tight`** — drift.
- Line-height: `leading-reading` — used 14× in routes.
- Spacing: half-step values like `py-1.25`, `py-3.5`, `gap-4.5`, `mt-0.75` — used heavily. Indicates strict 4 px-grid discipline.
- Transition: `duration-120|140`.

### Tokens vs mockups' `tokens.css`

The mockup `docs/mockups/lib/tokens.css` defines a parallel set of CSS variables in raw `oklch()` form: `--paper`, `--paper-2`, `--ink`, `--accent`, `--success`, etc. The app's `lib/tokens.css` **does not redefine these** — color is owned entirely by Rokkit's `--color-surface-z*` / `--color-primary-z*` scale. Only typography, borders, radii, spacing, and three button classes live in the app's tokens.css.

Net result: the mockup palette name → app palette name mapping is documented in `rokkit.config.js`:

| Mockup token | Rokkit palette | Class shorthand |
|---|---|---|
| `--paper` / `--ink` | `kami` / `sumi` (z-flipped in dark) | `bg-surface-z1`, `text-surface-z9` |
| `--accent` (vermillion) | `shu` → `primary` | `text-primary-z5`, `bg-primary-z5` |
| `--success` (jade) | `hisui` → `success` | `text-success-z5` |
| `--warning` (amber) | `kohaku` → `warning` | `text-warning-z5` |
| `--danger` (crimson) | `beni` → `danger` | `text-danger-z5` |

Both systems use OKLCH end-to-end. Cleanly aligned.

### Theme / dark-mode plumbing

- `app.html` script (inline, before first paint) reads `prefers-color-scheme` + localStorage and sets `document.body.dataset.mode`.
- `+layout.svelte` re-applies on `matchMedia('change')` events and additionally reads the OS AccentColor (macOS, via a hidden `<div>` probe) to generate a Rokkit-compatible `--color-primary-z1…z9` scale matching the user's system accent — this overrides the static vermillion `shu` palette. So in practice, primary is the OS accent, not vermillion. (This may be a problem for the mockups' insistence on `shu` vermillion as the brand colour — flag for product decision.)
- `rokkit.config.js` sets `switcher: 'manual'` and `storageKey: 'sensei-desktop-theme'`. The theme file is `@rokkit/themes/zen-sumi.css`.

---

## 4. Routes inventory ↔ Mockup mapping

| Route | LoC | Mockup file (`/sensei/docs/mockups/lib/`) | Mapping |
|---|---:|---|---|
| `routes/+layout.svelte` | 91 | — (boot/theme plumbing) | root layout |
| `(health)/health/+page.svelte` + `HealthView.svelte`/`Hero`/`Ledger`/`Remedy`/`Header` | 16 + 4 parts | `bootstrap.jsx`, `bootstrap-simple.jsx` | matches |
| `(health)/upgrade/+page.svelte` | 276 | (variant of bootstrap) | matches |
| `(health)/logs/+page.svelte` | 511 | `project-logs.jsx` (partial) | partial — logs view is app-specific |
| `(config)/setup/welcome/+page.svelte` | 44 | `setup-wizard.jsx` (welcome stage) | matches |
| `(config)/setup/preferences/+page.svelte` | 240 | `setup-wizard.jsx`, `collective-settings.jsx` | matches |
| `(config)/setup/assistants/+page.svelte` | 110 | `agent-persona-editors.jsx` (partial) | partial |
| `(config)/setup/roots/+page.svelte` | 115 | `setup-wizard.jsx` | matches |
| `(config)/setup/scan/+page.svelte` | 264 | `setup-wizard.jsx` (scan stage) | matches |
| `(config)/setup/projects/+page.svelte` | 13 | `setup-wizard.jsx` | placeholder only |
| `(config)/setup/libraries/+page.svelte` | 12 | `libraries.jsx` (lite) | placeholder only |
| `(config)/setup/instruments/+page.svelte` | 12 | `instruments-simple.jsx` | placeholder only |
| `(config)/setup/inference/+page.svelte` | 8 | `inference-settings.jsx`, `wiz-inference.jsx` | placeholder only |
| `(config)/setup/assignments/+page.svelte` | 8 | `wiz-assignments.jsx` | placeholder only |
| `(config)/setup/done/+page.svelte` | 43 | `setup-wizard.jsx` (done) | matches |
| `(config)/config/+page.svelte` | 10 | — | redirect stub |
| `(observatory)/+page.svelte` (Today) | 185 | `observatory.jsx` | matches; mockup is much richer |
| `(observatory)/projects/+page.svelte` | 31 | `observatory.jsx` (project grid) | partial — drift from mockup style |
| `(observatory)/projects/[id]/+page.svelte` | 216 | `project-pages.jsx`, `project-shared.jsx`, `project-lite-panes.jsx` | matches (single-window variant) |
| `(observatory)/sessions/+page.svelte` | 159 | `sessions.jsx`, `sessions-zen.jsx` | matches |
| `(observatory)/insights/+page.svelte` | 5 | `mcp-replay-insights.jsx`, `learnings.jsx` | wraps `MemoryList` only |
| `(observatory)/learnings/+page.svelte` | 5 | `learnings.jsx`, `learnings-v2.jsx`, `learnings-anatomy-v2.jsx` | wraps `MemoryList` only |
| `(observatory)/libraries/+page.svelte` | 169 | `libraries.jsx` | matches |
| `(observatory)/instruments/+page.svelte` | 217 | `instruments.jsx`, `instruments-simple.jsx` | matches |
| `(observatory)/settings/+page.svelte` | 212 | `collective-settings.jsx`, `inference-settings.jsx` | matches |
| `(observatory)/help/+page.svelte` | 82 | — | no mockup |
| `(project)/project/[id]/+layout.svelte` | 127 | `project-pages.jsx` (window chrome) | matches |
| `(project)/project/[id]/+page.svelte` | 2 | — | redirect stub |
| `(project)/project/[id]/overview/+page.svelte` | 212 | `project-pages.jsx`, `impact.jsx` | matches |
| `(project)/project/[id]/about/+page.svelte` | 56 | `project-pages.jsx` | matches |
| `(project)/project/[id]/sessions/+page.svelte` | 23 | `sessions.jsx` (scoped) | stub |
| `(project)/project/[id]/memories/+page.svelte` | 34 | `consolidation.jsx`, `sharing-review.jsx` | stub |
| `(project)/project/[id]/libraries/+page.svelte` | 52 | `libraries.jsx` (scoped) | stub |
| `(project)/project/[id]/instruments/+page.svelte` | 48 | `instruments.jsx` (scoped) | stub |
| `(project)/project/[id]/patterns/+page.svelte` | 56 | (no direct mockup; under `project-pages.jsx`) | stub |
| `(project)/project/[id]/traceability/+page.svelte` | 46 | `traceability.jsx` | stub |
| `(project)/project/[id]/impact/+page.svelte` | 129 | `impact.jsx` | matches |

### Routes with **no mockup**

- `(observatory)/help/+page.svelte` — keyboard cheatsheet + FAQ. App-specific.
- `(health)/logs/+page.svelte` — diagnostic logs viewer. Partially overlaps `project-logs.jsx`.
- `(health)/upgrade/+page.svelte` — post-restart upgrade flow. Bootstrap-adjacent.
- `(config)/config/+page.svelte` — redirect-only.
- `(project)/project/[id]/patterns/+page.svelte` — patterns subview (only referenced inline, no dedicated mockup).

### Mockups with **no route yet**

Listing every `.jsx` in `docs/mockups/lib/` that doesn't already have a corresponding route:

| Mockup file | Likely route | Status |
|---|---|---|
| `agent-persona-editors.jsx` | (settings sub-tab?) | partially covered by setup/assistants |
| `benchmark.jsx` | new — `(observatory)/benchmark`? | not built |
| `consolidation.jsx` | could be project sub-view | covered as a stub in `(project)/.../memories` |
| `design-canvas.jsx`, `direction-*.jsx`, `primitives.jsx` | — | these are design exploration files, not screens |
| `extensions-browser.jsx` | `(observatory)/extensions`? | not built |
| `learnings-anatomy-v2.jsx` | learnings detail | not built (`learnings/+page.svelte` is just MemoryList) |
| `mcp-replay-insights.jsx` | instruments/replay tab | partially covered (instruments has a Replay tab stub) |
| `navigation.jsx` | sidebar reference | mocked but not productionized |
| `perspective-split.jsx` | — | design exploration |
| `project-filter.jsx`, `project-lite-panes.jsx` | project list filter / lite panes | not yet wired |
| `project-logs.jsx` | logs (already exists but in (health)) | overlap with `(health)/logs` |
| `sharing-review.jsx` | (settings → collective → review) | partial coverage |
| `skill-editor.jsx` | skill editor screen | not built |
| `upgrades.jsx` | upgrade history view | not built (`(health)/upgrade` is the flow, not history) |

---

## 5. Test pattern

### Convention

`X.spec.svelte.ts` or `X.spec.ts` lives **alongside** the component or state module it tests. There is no `__tests__/` folder. Naming:

- `Foo.svelte` → `Foo.spec.svelte.ts` (DOM mount tests using `@testing-library/svelte` + `vitest`).
- `foo.svelte.ts` (rune state module) → `foo.spec.svelte.ts`.
- Pure TS → `foo.spec.ts` or `foo.test.ts`.

**Co-located component tests exist only for the health route** (`Header`, `Hero`, `Ledger`, `Remedy`, `HealthView`, `page`). Every other route has zero component-level tests. Shared `lib/components/*` — **no tests** for `EmptyState`, `FolderInput`, `MemoryList`, `Switch`, `TabBar`, `WizardShell`.

State modules have heavy unit coverage: `appstate`, `wizard-state`, `health-state`, `health-transport`, `scan-state`, `repos`, `stage`, `setup/loaders`, `setup/contracts`, `events`, `logger`, `stores/windows`.

### Mount helper

`src/lib/test-mount.ts` (29 lines) provides `mountComponent(Component, props)` returning `{ container, destroy }`. Used by every `*.spec.svelte.ts`. Uses `// @vitest-environment jsdom` directive at the top of each spec.

### E2E

`e2e/tests/*.spec.ts` (6 files) run via `@playwright/test` + `@srsholmes/tauri-playwright`. Covers boot-flow, setup-wizard, configure-assistants, db-setup, daemon-verification, multi-window. Run with `bun test:e2e` (script `playwright test --config e2e/playwright.config.ts --project=tauri`).

### Vitest config

`vitest.config.ts` exists (622 bytes) — not inspected here but referenced from `package.json`. Coverage via `@vitest/coverage-v8`.

---

## 6. Recommendations — extraction priority

Prioritized by reach × duplication count.

**Tier 1 — biggest reach, ≥10 inline copies each:**

1. `PageHeader` (eyebrow + kanji + title; variants for `2xl`/`3xl`/`4xl`).
2. `ListRow` (3-slot flex row + hairline + `:last-child` reset baked in).
3. `Card` (variants: `padding`, `tone`, accent left-border).
4. `Eyebrow` (single source of letter-spacing token; replaces 4 drifting variants).
5. `Kanji` (size + tone props; replaces all inline `text-primary-z5 opacity-…` declarations).
6. `StatusDot` (one component, four states; eliminates 8 sets of CSS rules).

**Tier 2 — moderate reach, 4–8 copies:**

7. `Button` (wrap or replace `.btn-solid`/`.btn-outline`/`.btn-cta`/`.btn-primary`/`.btn-back` with `variant` prop). Evaluate Rokkit `Button`.
8. `Badge` / `Pill` / `Chip` (tone). Evaluate Rokkit `Badge` and `Pill`.
9. `TabBar` (rebuild on top of Rokkit `Tabs`).
10. `ChipRow` / `SegmentedControl`.
11. `TextField` (single source for input border/focus styling).
12. `MiniStat` (display value + label + delta + optional sparkline slot).
13. `EmptyState` (extend existing with `variant="inline|card"`, `cta` slot, replace inline copies).
14. `Sidebar` + `NavItem` (one component used in three layouts).

**Tier 3 — narrower, but eliminates copy-pasted state machines:**

15. `Sparkline` (with shared `sparklinePath` util).
16. `SplitPane` / `ListDetail` layout.
17. `SidePanel` (sticky right-column detail).
18. `Dialog` / `Modal` (just the logs page — but mockups will need more).
19. `ActivityRow` + `Timeline` (scan SSE feed + log traces).
20. `TauriChrome` (drag-region + optional accent stripe + optional title).

**Tier 4 — greenfield (mockups call them out, no app code yet):**

21. `EnsoRing`, `BarRow`, `HairlineGrid`, `EventGlyph`, `Toast`, `Avatar` (Rokkit `Avatar` already installed), `Drawer`.

**Cleanup before any of the above:**

- Delete or productionize unused `FolderInput.svelte` and `WizardShell.svelte`.
- Extract `sparklinePath` into a util (`$lib/sparkline.ts`) and import it in both observatory and overview pages.
- Decide whether OS-accent override in `+layout.svelte:19-53` is intended; if not, remove it so the vermillion `shu` brand colour wins consistently.
- Split `MemoryList.svelte` into `PageHeader` + `TabBar` + `ListDetail(ListRow + SidePanel)` once those exist, so `insights/+page.svelte` and `learnings/+page.svelte` can compose distinct content.
- Resolve drift between Tailwind `tracking-wider`/`tracking-widest`/`tracking-tight` and Uno's custom `tracking-cap|tag|label|loose` — settle on one set.

---
