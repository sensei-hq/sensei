---
title: Mockup-Drift Audit — app vs WCAG-clean mockups
description: Per-screen drift + rokkit-migration punch-list across 28 screens; 396 findings; foundational token reconciliation + alpha-soft architecture + rokkit override snippets
type: spec
status: spec
created: 2026-08-05
references:
  - docs/spec/MOCKUP-INDEX.md
  - docs/mockups/Sensei/lib/tokens.css
  - app/rokkit.config.js
---

# Mockup-Drift Audit — Authoritative Fix Punch-List

> Synthesis across 28 screens + the token baseline. Execute **foundational tokens first**, then **rokkit override snippets**, then **one screen at a time** in the order in §5.

---

## 1. Executive Summary

Five systemic themes account for the large majority of the ~300 findings. Fix the systemic ones first — they resolve dozens of per-screen items at once.

### Theme A — Token-value drift from the recent AA darkening (VERDICT: KEEP the darkening, do NOT revert)
The app deliberately darkened `ink-mute`, `ink-faint`, and the four status **text** bases (accent/success/warning/danger) to ~0.50–0.52L to clear WCAG AA 4.5:1 on small text. Measured against the mockup's own values on paper, the mockup genuinely **fails** AA on its muted/status tiers (ink-mute 4.0:1, ink-faint 2.07:1, success 3.26:1, warning 2.35:1) — only ink (16:1), ink-soft (9:1), and danger (4.94:1) pass. **Therefore the darkening is a defensible accessibility fix and must NOT be reverted to the lighter mockup values.** Per the repo rule "when the mock is wrong, fix the mock," push the corrected AA values **back into `lib/tokens.css`** so mock and app re-converge instead of silently forking.

**The one unambiguous defect in the darkening is a REGRESSION that MUST be fixed:** in light mode `ink-mute` (0.500L) and `ink-faint` (0.500L) are now **identical**, collapsing the 4-step ink ramp to 3 steps. Every screen that places `text-ink-faint` next to `text-ink-mute` loses its hierarchy (today recent-rows, sessions captions, proj-overview meta, proj-atlas captions, insights, traceability, dojo-connections). In dark mode the two are still distinct (faint 0.42–0.68L vs mute 0.60–0.74L), so faint-tier captions render *too dim* in dark mode where mute was intended.

### Theme B — Solid-vs-alpha `-soft` architecture (adopt the mockup's alpha model)
The mockup's four `-soft` tokens are **alpha composites** — `oklch(<base L C H> / 0.10–0.20)` — that ride whatever surface they sit on and stay dark-safe by construction. The app emits **solid two-pole palette shades** (light `.100`, dark `.900`), which don't composite and forced a documented hardcode to dodge the "pale-tint-in-dark" bug (e.g. `hisui.400` on `hisui.100` = 2.03:1). The alpha model was fully available (`colorSpace:'oklch'` stores bare triplets; `overrides:` accept `oklch(...)` strings) and was simply not taken. This surfaces on nearly every screen (insight tag pills, verdict pills, callouts, chips). Adopt the alpha softs and delete the hardcoded solid-shade workarounds.

### Theme C — Recurring type-scale & spacing drift
- **Header kanji undersized:** most screens render the hero/PageHeader kanji at 28px (`variant="h2"`) where the mockup ScreenHeader/KanjiHeader uses **40px** (`3xl`). (projects, libraries, traceability, impact, share-review, dojo-connections, dojo-sharing, settings-inference, settings-projects, proj-memories, proj-impact; and **oversized** at 56px on instruments-obs / proj-atlas.)
- **Off-grid gutters:** `px-7` (28px) is used pervasively where the mockup uses `px-8`/`px-12`. `px-7` is the discouraged stop.
- **Arbitrary px literals** (`text-[40px]`, `text-[13px]`, `text-[22px]`, `font-size:9px`) bypass the 8-stop scale on several screens.
- **Display-vs-UI face confusion:** stat numbers rendered in Fraunces (`display`) where the mockup uses UI-light + mono (today, sessions, proj-sessions, consolidation, impact).
- **Over-tracking:** `tracking-wide`/`tracking-tight` applied where the mockup specifies hand-tuned 0.01–0.18em values; eyebrows especially (shared `Eyebrow.svelte` ~0.1em vs zs-eyebrow 0.18em).

### Theme D — Missing states & structural omissions
Many screens ship only ready+empty and delegate loading/error to the SvelteKit boundary, dropping the mockup's `ScreenState` loading-skeleton + error-with-Retry scaffold. Several loaders fall back to `null`/`{}` on failure, **masking a fetch error as an honest-empty state** (consolidation, libraries, dojo-sharing, share-review) — a no-fabrication violation. Larger structural gaps: whole sections/features missing (traceability is a full rebuild; consolidation implements a *different feature*; memories/`learnings` implements the *sibling* mockup; the sessions/proj-sessions Retrospective lane; the instruments docs layer / atlas docs layer).

### Theme E — Incomplete rokkit migration
The "migrate + override" mandate is partially done. Buttons are largely migrated; **Toggle/Select/List/Tabs/Table/Menu are frequently hand-rolled or migrated-but-unstyled** (default rokkit chrome instead of the mockup look). The `app.css` `[data-component="observatory-sidebar"] [data-list]` override is the proven precedent to copy. A recurring gotcha: **rokkit v1.3.1 Toggle/Select/Switch drop arbitrary `data-*`/`aria-*` props** — put test/aria ids on a wrapper `<div>`, and render composite content (kanji/badge/count) via the `itemContent` snippet.

---

## 2. Foundational Fixes (do these first — they fix many screens at once)

Ordered. All live in `app/rokkit.config.js` + `app/src/app.css` + `lib/tokens.css`.

**F1 — Fix the ink-ramp collapse (the one real darkening defect).** In `rokkit.config.js:70-71`, `ink-mute` and `ink-faint` are both `oklch(0.500 0.010 50)` in light. Give `ink-faint` a distinct lighter value (and formally scope it to large/decorative/disabled text) OR push `ink-mute` darker, so `faint > mute` stays ordered in **both** modes. A 4-step ramp that renders as 3 must not ship. Re-verify dark mode too (faint currently too dim where mute was meant).

**F2 — Adopt the alpha `-soft` model (Theme B).** Override the six softs — `accent/success/warning/danger/error/info-soft` — to `oklch(var(--color-<role>) / <a>)` with the mockup alphas (accent .12, success .14, warning .15, danger .10, dark variants .16–.20). Remove the hardcoded solid `.100/.900` shades and the dark-mode workaround comments. Keep the AA-darkened status **text** bases. This makes every callout/chip/pill surface-adaptive and dark-safe.

**F3 — Re-converge AA values into `lib/tokens.css` (Theme A).** Write the app's corrected `ink-mute`, status-text, and (post-F1) `ink-faint` values back into the mockup's `lib/tokens.css` so the two stop forking. Reconcile the smaller palette drifts while here: light `paper-edge` (app 0.850/hue70 → mockup 0.880/hue85), and the dark status **chroma** the app dropped (success 0.075→0.090, accent 0.130→0.150) if brand vividness matters.

**F4 — Single-source the app-only tokens.** `error/error-soft/info/info-soft` have no mockup source. Add them to `lib/tokens.css` (or document them) so the named-token vocabulary has one authority. Confirm whether failures should read `danger` or `error` app-wide (activity-logs uses a dedicated red `--error`; the mockup uses `accent` vermillion as its failure signal — decide once).

**F5 — Establish the shared header sizing.** Add/confirm a `PageHeader variant` that pairs a **40px (`3xl`) kanji** with the correct title size, and a `right`-slot for stats/actions. Nearly every screen needs the 40px kanji + eyebrow + description + right-action; fixing `PageHeader` once resolves the header findings on projects, libraries, traceability, impact, share-review, dojo-connections, dojo-sharing, settings-*, proj-memories, proj-impact.

**F6 — Fix the shared `Eyebrow` primitive.** Widen tracking to ~0.18em (zs-eyebrow) — but verify against sibling mockups since it's global (`app/src/lib/components/Eyebrow.svelte:15`).

**F7 — Reserve `paper-mute` for fills only.** A recurring bug: `border-paper-mute` on a `bg-paper-mute` card renders an **invisible** border (settings-general, settings-extensions, settings-instruments, settings-projects, libraries, learnings, share-review, proj-about, proj-memories). Sweep: any hairline/border → `border-paper-edge`; any sunken well/track → `paper-mute`. This is a lint-able rule.

**F8 — Add the shared `ScreenState` scaffold** (loading skeleton + empty + error-with-Retry) and stop `null`-on-failure in loaders (consolidation `api.ts:1035`, libraries `getLibs`, dojo-sharing `+page.ts`, share-review). A failure must be distinguishable from honest-empty.

**F9 — Resolve `primary` skin = ink vs vermillion.** `rokkit.config` maps `primary → shu` (vermillion) in places; the design system says **primary = ink** (ink-on-paper CTA). Primary Buttons on proj-impact, dojo-connections, settings-inference render vermillion. Fix the skin so `[data-button][data-variant="primary"]` resolves to `var(--ink)`/`var(--on-primary)`, or override per-use. Ration vermillion.

---

## 3. Rokkit Component Migration Plan (consolidated)

**Universal gotchas:**
- v1.3.1 Toggle/Select/Switch **drop arbitrary `data-*`/`aria-*`** — scope overrides on a wrapper `<div data-component="...">`, keep test/aria ids on the wrapper, render composite cell content via `itemContent`/`groupContent` snippets.
- Precedent to copy: `app.css:107-123` `[data-component="observatory-sidebar"] [data-list] [data-list-item]`.

### Button — mostly migrated; only geometry/variant overrides needed
Reusable snippets (scope to a `data-component` wrapper):
```css
/* ink primary CTA (matches mockup btnPrimary) */
[data-component] [data-button][data-variant="primary"]{background:var(--ink);color:var(--on-primary);border-radius:var(--radius);padding:8px 16px;font-size:var(--text-sm)}
[data-component] [data-button][data-variant="primary"]:hover{background:var(--ink-soft)}
/* ghost / flat */
[data-component] [data-button][data-style="ghost"]{background:transparent;color:var(--ink-soft);border:0}
[data-component] [data-button][data-style="ghost"]:hover{background:var(--paper-mute);color:var(--ink)}
/* icon-only (kanji glyph buttons: 留/消/⋯) */
[data-component] [data-button][data-icon-only]{background:var(--paper-mute);color:var(--ink-mute);padding:var(--space-2);border-radius:var(--radius);font-family:var(--font-kanji);line-height:1}
```
Screens needing new Buttons: projects (`+ new project`), libraries (`+ add library`), traceability (Fix all / Fix drift), consolidation (accept/keep/edit/undo), impact & proj-impact (Revert/Revise/Keep-monitoring/Dismiss), upgrades (Defer, icon Pin/Mute), insights (Undo, Violation actions), settings-inference (pull/reorder), memories (anatomy actions + `···`).

### Toggle — the most common miss (segmented single-choice). Two reusable looks:
```css
/* A. borderless ink-chip filter row (projects filter, insights project-filter, dojo-connections, learnings scope) */
[data-component] [data-toggle]{background:transparent;border:0;padding:0;gap:8px}
[data-component] [data-toggle-option]{border:1px solid var(--paper-edge);border-radius:var(--radius-full);background:transparent;color:var(--ink-soft);padding:4px 12px;font-size:var(--text-xs)}
[data-component] [data-toggle-option][data-selected]{background:var(--ink);color:var(--on-primary);border-color:var(--ink)}
/* B. sunken paper-mute track w/ raised paper selected pill (projects view, sessions chart tabs, obs-sidebar All|Focus, atlas granularity, settings segments) */
[data-component] [data-toggle]{background:var(--paper-mute);border-radius:5px;padding:3px;gap:0}
[data-component] [data-toggle-option]{border-radius:3px;padding:4px 12px;font-size:var(--text-xs);color:var(--ink-mute);background:transparent}
[data-component] [data-toggle-option][data-selected]{background:var(--paper);color:var(--ink);box-shadow:var(--shadow-sm)}
```
Migrate: projects (filter+view — already Toggle, zero override), insights project-filter, sessions range+chart, libraries kind+lang+mcp-tools, upgrades/dojo cadence, activity-logs since-range, settings-general segments (correction/digest), settings-extensions kind filter, atlas granularity, learnings/memories scope, health window, share-review nothing new.

### Select — dropdown pickers. One reusable trigger/dropdown skin:
```css
[data-component] [data-select-trigger]{background:var(--paper-soft);border:1px solid var(--paper-edge);border-radius:var(--radius);padding:6px 10px;font-size:var(--text-sm);color:var(--ink)}
[data-component] [data-select-trigger]:focus-within{border-color:var(--ink)}
[data-component] [data-select-dropdown]{background:var(--paper);border:1px solid var(--paper-edge);border-radius:var(--radius);box-shadow:var(--shadow)}
[data-component] [data-select-option][data-selected]{background:var(--paper-mute);color:var(--ink)}
```
Migrate/fix: activity-logs level filter (native `<select>` → Select), instruments enum input (migrated but unstyled), settings-projects role picker (chip-style variant: transparent trigger, accent value text), settings-inference role picker (already migrated, needs skin), insights project search (Select combobox), share-review scope (blocked on wire).

### List — hairline row lists ("hairlines, not boxes"). Reusable base:
```css
[data-component] [data-list]{gap:0}
[data-component] [data-list-item]{border-bottom:1px solid var(--paper-edge);padding:12px 16px;border-radius:0}
[data-component] [data-list-item]:last-child{border-bottom:none}
[data-component] [data-list-item]:hover,[data-component] [data-list-item][data-selected]{background:var(--paper-soft)}
/* nav-rail selected variant: add left accent bar */
[data-component] [data-list-item][data-selected]{border-left:2px solid var(--accent)}
```
Migrate (hand-rolled today): today insights + recent-sessions, projects list-view, insights (none new), learnings rail, sessions/proj-sessions session rows, libraries list, impact & proj-impact verdict list, traceability drift/docs lists, consolidation proposal rail, share-review item list, instruments session picker + timeline, dojo-connections memberships, proj-overview recent rows, proj-shell section nav, proj-memories memory cards, proj-about repos. **obs-sidebar & settings sidebar already on List** (need color/badge overrides only).

### Tabs — underline tab strips:
```css
[data-component] [data-tabs-list]{border-bottom:1px solid var(--paper-edge);padding-inline:48px;gap:0}
[data-component] [data-tabs-trigger]{padding:12px 16px;font-size:var(--text-sm);color:var(--ink-mute);border-bottom:2px solid transparent}
[data-component] [data-tabs-trigger][data-selected]{color:var(--ink);border-bottom-color:var(--accent)}
```
Migrate: learnings tab bar (if triage retained), instruments-obs tab strip (only if tabbed shell kept — the `-simple` design removes it), settings-inference tab strip (if restored), insights/consolidation subtabs (obs-sidebar recommendation).

### Table — data grids with header strip:
```css
[data-component] [data-table-header-cell]{font-size:var(--text-xs);text-transform:uppercase;letter-spacing:0.14em;color:var(--ink-mute);font-weight:400;background:var(--paper-soft)}
[data-component] [data-table-cell]{font-family:var(--font-mono);font-size:var(--text-xs)}
[data-component] [data-table-row][data-selected]{background:var(--paper-mute)}
```
Migrate: instruments health per-tool table, activity-logs background-tasks table (two-line worker cell may stay a snippet).

### Menu — action popovers:
```css
[data-component] [data-menu]{background:var(--paper-soft);border:1px solid var(--paper-edge);border-radius:var(--radius);box-shadow:var(--shadow)}
[data-component] [data-menu-item]{padding:6px 12px;font-size:var(--text-sm);color:var(--ink)}
[data-component] [data-menu-item][data-selected]{background:var(--paper-mute)}
```
Migrate: learnings/memories `···` overflow, settings-projects repo-move + merge, dojo-connections scopes, memories scope-ladder.

### Tree — collapsible hierarchies:
Migrate: instruments MCP groups→tools tree (chevron + kanji + count).

### Genuine keep-hand-rolled (with reason)
| Widget | Reason |
|---|---|
| Free-text `<input>` / `<textarea>` / search boxes | rokkit has **no** text-input/textarea primitive (Select/MultiSelect are option-pickers). Fix tokens only (`bg-paper-soft`, `border-paper-edge`, focus→ink, `placeholder text-ink-soft`). |
| Chart/sparkline/HealthBar/StrengthBar SVGs | data-viz; no rokkit primitive. Keep fills on named tokens. |
| Static badges/chips/pills (DocChip, ProjPill, DojoChip, status pills, kind chips) | display-only; rokkit has no Badge/Chip. Fix to alpha-soft tokens + `.04em` tracking + `py-1`. |
| Boolean sliding switches (dojo-connections enable, share-review checkbox) | rokkit Toggle is a segmented radiogroup, not a sliding knob/checkbox; rokkit has no Checkbox. Use `$lib/Switch` (wraps rokkit Switch) for on/off rows, or a native `<button aria-pressed>` for checkbox-shape controls. |
| Composite content cards (adopted teachings, retro lanes, project cards, MoE/anatomy panels, stat tiles, hero cards) | 2-D bespoke composites; a List/Table would fight the layout. Keep as token-styled markup. |
| Native window chrome / drag regions (proj-shell titlebar) | OS chrome, not a widget. |
| Inline expand/disclosure wells (traceability signature-delta) | rokkit exposes no accordion/disclosure part. |
| Brand wordmark lockup | not data-driven. |

---

## 4. Per-Screen Punch List

Findings ordered high→low. `file:line` is where to apply the fix.

### `today` — observatory-today.jsx
- **[high] Ink hierarchy inverted in recent rows** — title `text-ink-mute` sits *lighter* than its own `text-ink-soft` metadata. Swap: title→`text-ink-soft`, project/duration→`text-ink-mute`, time→`text-ink-faint`. `RecentSessions.svelte:70-88`.
- **[high] Pinned header + hairline gone** — greeting/FTR strip scrolls with body. Restore ScreenShell `shrink-0 header` with `border-b border-paper-edge bg-paper`. `+page.svelte:21-46`, `+layout.svelte:81`.
- **[med] Meta text 1–2 tiers too dark** (`text-ink-soft`→`text-ink-mute`) across FTR %, section links, adopted `when`, hero source. `+page.svelte:38,103`, `RecentSessions.svelte:52`, `AdoptedCard.svelte:12`, `HeroKoan.svelte:51`.
- **[med] Body copy a tier too light** (`text-ink-mute`→`text-ink-soft`) + line-height 1.6/1.5. `HeroKoan.svelte:37`, `InsightCard.svelte:25`.
- **[med] Insight tag pills** use solid `bg-paper-mute`+border → alpha `bg-<role>-soft text-<role>` no border (F2). `today-view.svelte.ts:55-63`, `InsightCard.svelte:27`.
- **[med] "Recent sessions" heading** `text-base`→`text-lg` + `tracking-tight`. `RecentSessions.svelte:51`.
- **[med] Gutter** `px-7`→`px-12`. `+page.svelte:21`.
- **[med] Greeting drops the name** — append display name. `today-view.svelte.ts:71-74`.
- **[med] Corrections copy/color** — "3× rework"+tint → bare "3×" neutral `text-ink-mute`. `RecentSessions.svelte:15-19,75-77`.
- **[med] Hero under-padded** — `px-6 py-6 gap-5 rounded-lg`→`px-8 py-8 gap-6 rounded-[12px]`. `HeroKoan.svelte:22`.
- **[low] grid gap-6/mt-6→gap-8/mt-8** `+page.svelte:52`; **FTR "%"** `text-xs ink-soft`→`text-sm ink-mute` `:38`; **row separators** `paper-mute`→`paper-edge` `RecentSessions.svelte:61`; **adopted empty-state** kanji→faint, copy→ink-mute, py-6 `+page.svelte:84-87`; **display tracking** -0.02→-0.01em; **early-mode dim palette** `FtrStrip.svelte:16-17`.
- **Rokkit:** hero CTA Button ✓ (geometry pin). **Migrate** insights list + recent-sessions → List (snippets, `href`). Keep hand-rolled: section text-links, adopted cards, FTR sparkline.

### `projects` — navigation.jsx (ProjectsIndexA)
- **[high] `+ new project` primary action missing** — add ink Button in header-right; wire ⌘N. `+page.svelte:69-72`.
- **[med] `⌘K to jump` pill** is from discarded Browser-C — remove/relocate. `+page.svelte:70-72`.
- **[med] Both Toggles unstyled** — add filter (borderless ink-chip) + view (sunken track) overrides. `+page.svelte:77-104`.
- **[med] View glyphs** solar icons → kanji 田/≣. `+page.svelte:55-58`.
- **[med] Gutter** `px-7`→`px-12`. `+page.svelte:63,76,130`.
- **[med] Header kanji** 28→40px `:64`; **title** 22→28px + -0.02em `:67`.
- **[low] Counter/search kanji** `ink-soft`→`ink-mute` `:107,124`; eyebrow tracking→0.18em (F6); ProjPill `py-0.5`→`py-1`; card/row hover transition; empty-state copy token.
- **Rokkit:** filter+view Toggle ✓ (need overrides). **Migrate** list-view → List; add `+ new project` Button. Keep hand-rolled: search input, card composite.

### `insights` — learnings-v2.jsx (LearningsTriage)
- **[med] Header KPIs wrong** — shows column counts (all mono/tinted) instead of to-act/memories/ftr-lift; extend wire or fix mock; stop mono-ing/tinting all three. `+page.svelte:39-49`.
- **[med] Active filter chip** `bg-accent-soft`→solid ink pill (`bg-primary text-on-primary`). `ProjectFilterStrip.svelte:43`.
- **[med] Applied/dismissed recs deleted** — render collapsed resolved one-liner + Undo instead of removing. `insights-board.svelte.ts:273`.
- **[med] RecCard shows project chip** not recommendation-kind descriptor (needs wire `kind`). `RecCard.svelte:34`.
- **[med] ViolationCard static** — add Reinforce/Review/Mute row + focal "do first" + timestamp. `ViolationCard.svelte:12`.
- **[med] Soft callouts solid** (F2). `ViolationCard.svelte:15`, `MemoryRow.svelte:26`, `+page.svelte:182`.
- **[med] Project refetch has no loading feedback** — consume `board.loading`. `+page.svelte:189`.
- **[low] "project" eyebrow missing** `ProjectFilterStrip.svelte:33`; search box lacks magnifier/clear/right-align; header kanji 今→学 `:157`; eyebrow/sub copy; column sub-caption copy + `proposed`→`challenged` `MemoryRow.svelte:27`; captions `ink-soft`→`ink-mute`; uppercase tracking 0.05→0.14em; padding one stop tight; Settled count/patterns extra; strength-bar track `paper-mute`→`paper-edge`; card radius 6 vs 5.
- **Rokkit:** RecCard actions Button ✓ (scoped ink/outline/ghost). **Migrate** filter→Toggle, search→Select, ViolationCard actions + Undo→Button. Keep hand-rolled: cards.

### `memories` (route `/learnings`) — learnings-anatomy-v2.jsx — ⚠ WRONG FEATURE
- **[high] App implements the triage manager (sibling learnings-v2 → /insights), NOT the anatomy reader.** Whole screen structurally mismatched. **Decide routing:** move triage to /insights, rebuild /learnings as anatomy reader (toolbar + 244px calm rail + reading stage), OR update MOCKUP-INDEX. `+page.svelte:19`.
- **[high] Missing:** L2Hero (覚 + HealthChart); toolbar (project-pill filter + memory search + "N of M"); 244px surface-glyph rail; AnatomyStageV2 reading pane; Promote/Enrich/Apply verbs.
- **[med] REAL BUGS:** undefined `var(--text-muted)` (`ActiveList.svelte:74`, `ArchiveList:24`, `TriageList:37`) → `text-ink-mute`; hardcoded `#553/#fff` via undefined `--warning-bg/--warning-fg` (`TriageList.svelte:35`) → `bg-warning-soft text-warning`.
- **[med] Hairlines** `paper-mute`→`paper-edge` (`+page.svelte:65,70`, `ActiveList:68`); off-scale rem fonts (0.75/0.85rem)→`text-xs/text-sm`; detail title `text-xl` sans→display `text-3xl`; rem paddings→4px grid utilities; eyebrow chips→zs-eyebrow; bad-example danger-soft chip→`mono text-accent`; selected rail full-border→left accent bar + soft fill.
- **[low] Surface classification missing; StrengthBar→text metric; no auto-select strongest.**
- **Rokkit:** everything hand-rolled → Tabs (if triage kept), Toggle (scope), List (rail), Button (actions), Menu (overflow). Keep hand-rolled: search, badges→eyebrow, strength meter.

### `sessions` — sessions-zen.jsx
- **[high] Retrospective 3-lane section (primary content) absent** — app renders a session list (the *discarded* variant's element). Decide + reconcile. `+page.svelte:229-251`.
- **[med] Session list is extra** vs current zen mockup. `+page.svelte:229-251`.
- **[med] Zen scroll-collapse interaction missing** — cycler is always-visible manual toggle. `+page.svelte:126-161`.
- **[med] Hero stat numbers in Fraunces** → UI-light for counts, mono for median. `+page.svelte:134,142,257`.
- **[med] `all` range missing** `sessions-digest.ts:28-29`; **full Pulse chart+chip missing** `:19-20`.
- **[med] TrendChart drops checkpoints/baselines/ftr-label/caption** `TrendChart.svelte:71-124`; **ConstellationChart drops x-ticks/avg-markers/ring/caption** `ConstellationChart.svelte:41-58`.
- **[med] ink-faint==ink-mute collapse** flattens captions (F1). `+page.svelte:91,98`.
- **[low] chip labels/heading capitalized**→lowercase `:183,192`; StreamChart annotations; extra totals strip `:162-177`; hero padding one step tight; mini-cycler/bands annotations.
- **Rokkit:** **Migrate** range→Toggle, chart-variant→Toggle, cycler→Button, session rows→List. Keep hand-rolled: retro lanes, chart SVGs.

### `libraries` — libraries.jsx (LibrariesVariantA)
- **[high] Whole layout reduced** — centered max-w-960 column vs full-bleed pinned two-pane 1fr/1fr. `+page.svelte:59`.
- **[high] Filter toolbar missing** (kind pills + lang pills + "N of M"). `+page.svelte:62`.
- **[high] Rows stripped** — no icon tile/version/source/DocChip/usage/lastIndexed. `+page.svelte:87`.
- **[high] Detail MCP-example explorer + tagline/summary/top-symbols/rules missing** (partly wire). `+page.svelte:107`.
- **[high] Header wrong** — kanji 書→庫, title, sub, `+ add library`. `+page.svelte:58`.
- **[high] DocChip docs-health badge absent everywhere.** `+page.svelte:93`.
- **[med] No loading/error state (masks failure as empty).** `+page.svelte:71`.
- **[med] Surface tokens** — `paper-mute` borders/detail → `paper-edge`/`paper-soft` (F7). `+page.svelte:88`.
- **[med] No rokkit components; header padding/kanji; detail 340px vs 1fr/1fr.**
- **[low] search lacks 探/clear; repo chips lowercased on paper-mute.**
- **Rokkit:** **Migrate** list→List, kind+lang+mcp-tools→Toggle, `+ add library`→Button. Keep hand-rolled: search, DocChip/repo chips.

### `impact` — impact.jsx (ObsImpact)
- **[high] 4-axis before→after grid reduced to 1 FTR cell + 3 redundant metadata cells.** Render honest cells only. `+page.svelte:147`.
- **[high] Action bar (Revert/Revise/Keep-monitoring/Dismiss) missing.** `+page.svelte:186`.
- **[med] Negative verdict amber not accent/vermillion.** `impact.ts:68`.
- **[med] Verdict pill solid bg-paper-soft** → alpha soft (F2). `+page.svelte:133`.
- **[med] Header counts mono + negative amber** → UI font, negative ink. `+page.svelte:47`.
- **[med] Header/detail padding + no 800px cap; secondary text `ink-mute`→`ink-soft`.**
- **[low] aside row/MoE `paper-mute`→`paper-soft`; MoE simplification (justified #109); tool-usage delta omitted; loading/error.**
- **Rokkit:** **Migrate** aside→List (tone bar in snippet), action bar→Button. Keep hand-rolled: stat tiles, before/after grid, MoE chips.

### `traceability` — traceability.jsx — ⚠ FULL REBUILD
- **[high] Whole 3-tier drill-down missing** — flat grouped list vs rollup strip + 300px/1fr. `+page.svelte:57-70`.
- **[high] Missing:** per-project health rollup strip; Documents nav aside; DocDetail panel; per-reference detail (line/quote/target + expected-vs-actual diff + confidence chip — drops typed `file/confidence/expectedSignature/actualSignature`); Fix drift / Fix all actions; HealthBar.
- **[med] No loading/error (297-call fan-out); header kanji 28→40px.**
- **[low] gutter px-8; status soft-fill chip → plain colored label; copy; ink-ramp.**
- **Rokkit:** **Migrate** drift/docs → List, rollup → Toggle, Fix actions → Button. Keep hand-rolled: signature-delta expander.

### `consolidation` — consolidation.jsx — ⚠ DIFFERENT FEATURE
- **[high] App implements governance ruleset consolidation; mockup is MEMORY consolidation.** Confirm intent; author correct artboard or rebuild. `+page.svelte:59`.
- **[high] Missing:** master/detail 320px rail; sources→merged visualization + arrow spine; per-source MemorySnippet cards; loading/error (null-on-failure masks). `+page.ts:11`.
- **[med] Missing decision banner + undo; diff headline muted (`toneClass('ink')`→`text-ink`); hero desc `ink-mute`→`ink-soft`; diff cell `paper-mute`→`paper-soft`; merged card `paper-soft`→`paper`; stats mono/tinted; action buttons drop kanji; empty-state copy.**
- **[low] Edit affordance; hero + detail padding (px-7).**
- **Rokkit:** **Migrate** proposal rail→List, actions→Button (ink/secondary/ghost/sm). Keep hand-rolled: diff strip.

### `upgrades` — dojo-inapp.jsx (InappDownstream)
- **[high] Precedence ladder bar (序 Org›Team›Global›Personal) missing** (static, no wire needed). `+page.svelte:196`.
- **[high] Lane 2 is analyzer recs not "Collective · public"** — confirm/relabel. `+page.svelte:228`.
- **[med] Action set diverges** — add Defer; icon-only Pin(留)/Mute(消); Adopt vs Apply. `+page.svelte:92`.
- **[med] Dōjō cards lose accent-edge border** (use `border-accent-edge`). `+page.svelte:53`.
- **[med] Lane headers drop kanji + uppercase-accent eyebrow.** `+page.svelte:199`.
- **[med] Card vertical vs horizontal 3-col grid; chips solid soft (F2).**
- **[low] Header copy (keep precedence sentence); impact/supersedes chips (wire); "N new" accent vs faint; px-7→px-8.**
- **Rokkit:** Apply/Pin/Mute Button ✓ (icon face); add Defer/Review Button. Keep hand-rolled: analyzer card grid, chips.

### `share-review` — dojo-inapp.jsx (InappShare)
- **[high] Boxed cards vs single hairline panel** → List. `ShareReviewScreen.svelte:68-104,240-259`.
- **[med] Policy bar swapped** (org-floor chips + edit-policy gone). `:169-201`.
- **[med] "Still forming — below the bar" section missing** (partly wire). `:249-259`.
- **[med] CTA out of header** → PageHeader `right` slot. `:136-142`.
- **[med] Header glyph 送→共; type pills colored/uppercase → neutral mono; softs solid (F2); no preview-redaction link; header kanji 2xl→3xl; rows omit origin/confidence/scope (wire).**
- **[low] no per-item selection (defensible); copy/period; title font-medium; chip py + tracking; px-7→px-8; CTA glyph paper→accent.**
- **Rokkit:** 3 Buttons ✓ (markup tweaks). **Migrate** item list→List, scope→Select (wire), edit-policy→Button. Keep hand-rolled: chips, checkbox.

### `instruments-obs` — instruments-simple.jsx
- **[high] Internal 3-tab strip retained** — `-simple` removed it in favor of sibling sidebar routes. Split to /instruments, /replay, /health or document. `+page.svelte:454`.
- **[med] Hero kanji text-4xl→3xl (40px); Playground copy from old variant; hero padding/align/border; hairlines `paper-mute`→`paper-edge`; enum Select unstyled; idle Response `—` vs example; Health missing 7d/30d/90d window; Signals demoted L1→drill.**
- **[low] health table columns; coverage bar solid vs segmented; replay stats; dead kind-filter state + unused imports; footer copy.**
- **Rokkit:** **Migrate** tab bar→Tabs (if kept), MCP groups→Tree, session picker+timeline→List, health table→Table, window→Toggle; enum Select ✓ (style). Keep hand-rolled: search, text/number inputs, health/KPI/signal cards.

### `activity-logs` — project-logs.jsx — ⚠ DIFFERENT FEATURE (raw log viewer vs session/trace explorer)
- **[high] Whole two-pane session/trace explorer absent** (feature re-scope; update MOCKUP-INDEX or give it its own artboard). `+page.svelte:48`.
- **[med] Level filter native `<select>`** → rokkit Select `:93`; **since control hand-rolled** → Toggle `:143`; **level chips/payload solid soft** (F2); **app red `--error` vs mockup accent-as-failure** (F4).
- **[low] header literal px→scale; 2px off-grid margin; no loading/error scaffold; empty-state kanji.**
- **Rokkit:** **Migrate** level→Select, since→Toggle, tasks→Table. Selects for source/module/limit ✓ (add mono override). Keep hand-rolled: search input, expandable log rows.

### `dojo-connections` — dojo-inapp.jsx (InappConnection)
- **[high] "Connected server" hero (Identity·access vs Attribution-only + offline token) absent.** `+page.svelte:104-217`.
- **[high] Per-membership enable/disable toggle missing** (wire has `enabled`). `+page.svelte:42-84`.
- **[high] Client-precedence footnote missing.** `+page.svelte:214-217`.
- **[med] Stacked boxes → single List; auth-method chip missing (`authenticated_via`); header kanji 28→40px; `+ Add connection` hand-rolled outline → rokkit filled-ghost Button; chips solid soft (F2); register form has no mockup counterpart (wire-driven, reframe).**
- **[low] scopes ▾ disclosure; chip py/tracking; px-7→px-8; client chip accent; header copy; ink-ramp; glyph 連 correct per spec.**
- **Rokkit:** **Migrate** `+ Add`→Button, memberships→List; form Select ✓/submit ✓. Keep hand-rolled: enable switch, text inputs, scopes Menu (blocked).

### `dojo-sharing` — collective-settings.jsx — ⚠ RE-SCOPED
- **[high] Missing:** sharing-mode picker (auto/review/off); Sharing-history section; Lifetime-contribution card+callout; Privacy-controls row group. **[high] Extra:** Destination two-toggle (wire-driven — fold into mock).
- **[med] Extra attribution radio section; hero stats omitted; active cadence chip accent-soft → ink pill; categories single-col switches → 2-col checkbox grid; category set 7 vs 8; section subs 11→13px; content width/padding.**
- **[low] hero kanji 28→40px; eyebrow/title copy; no ScreenState; accent/status AA (F3).**
- **Rokkit:** **Migrate** cadence→Toggle, attribution radios→Toggle. Keep hand-rolled: destination + category switches (no rokkit Switch/Checkbox fit).

### `obs-sidebar` — observatory.jsx (ObsSidebar)
- **[high] Review cluster has extra flattened items + Dōjō moved out of Settings** — restore grouping, surface Consolidation/Share-review/Dōjō-Sharing as subtabs. `observatory-nav.ts:85-103`.
- **[med] Rail surface inverted** (rail `bg-paper` on `bg-paper-soft`) → rail `paper-soft` on `paper` base. `ObservatorySidebar.svelte:39`, `+layout.svelte:71`.
- **[med] Active row weak** — kanji never vermillion (styled `[data-item-icon]` but glyph is `[data-item-icon-literal]`), label never full ink. Add overrides. `app.css:110-115`.
- **[med] Kanji lose Mincho face** — add `[data-item-icon-literal]{font-family:var(--font-kanji)}`. 
- **[med] All|Focus Toggle unstyled** (default dark pill) → paper-pill on mute track. `ObservatorySidebar.svelte:48`.
- **[med] Intake anchor removed; count badges stripped (wire real counts); rail 220 vs 240; idle labels ink-mute→ink-soft; active bg paper-soft→paper-mute; badge pill→plain mono.**
- **[low] header eyebrow size; footer heartbeat mono; "Settings"→"Preferences"; wordmark size; py-5→py-6; cluster labels ink-mute vs faint.**
- **Rokkit:** List ✓ + Toggle ✓ (both need the override additions above). Keep hand-rolled: wordmark, status footer.

### `settings-general` — setup-wizard.jsx (WizPreferences)
- **[high] Three kanji sections (名/師/守) collapsed to one flat card.** Reintroduce sections. `+page.svelte:101`.
- **[high] Correction tone + digest are Select dropdowns → should be segmented Toggle.** `+page.svelte:131`.
- **[med] Invisible `border-paper-mute` on `bg-paper-mute` card (F7); display-name input tokens + focus-accent; row order (tone before digest); labels/hints copy; save-error `text-warning`→`text-danger`; header tagline dropped + "Profile"→"General".**
- **[low] telemetry/welcome copy; digest hint; gap-8/pl-12/760; group subs.**
- **Rokkit:** **Migrate** correction+digest → Toggle; boolean rows Switch ✓; settings sidebar List ✓. Keep hand-rolled: name input.

### `settings-inference` — inference-settings.jsx — ⚠ WHOLESALE REIMPL
- **[high] 3-tab structure (Models/Routing/MOE) collapsed to 2 panels; hero stat pills + description absent; MOE tab absent; Routing tab + External providers absent (split to /providers).**
- **[med] Header kanji disagreement 智/推/想 (use 推); eyebrow/title copy; panels sunken `paper-mute`+invisible border → `paper-soft`+`paper-edge` (F7); local-model rows drop size/capability/default; primary chain full ink row vs accent circle; captions 13→11px `ink-soft`→`ink-mute`.**
- **[low] arbitrary px literals; header kanji 28→40; panel copy.**
- **Rokkit:** role Select ✓ (skin); reorder/pull/tab/window → Button/Tabs/Toggle. Keep hand-rolled: cards.

### `settings-projects` — setup-wizard.jsx (WizProjects)
- **[high] Action cluster (merge/edit-roles/exclude/per-repo move/split) collapsed to confirm-toggle-only 4-col header.** `ProjectsSection.svelte:93,114-141`.
- **[med] "Excluded from scan" section missing; per-repo file counts missing; stack-chips extra; role Select persistent vs segmented editor; card hairline invisible (F7); card fill a step dark; avatar/pills tokens; multi-repo badge solid accent (AA-justified — reconcile mock); name field no focus affordance.**
- **[low] confirm-check radius/border; secondary text ink-soft→ink-mute; text-[11px]→text-xs; dead nested max-w; header tagline; error callout danger-edge/soft.**
- **Rokkit:** role Select ✓ (chip style); expanded role editor→Toggle, move/merge→Menu. Keep hand-rolled: name input, confirm check.

### `settings-extensions` — ⚠ NO LIVE MOCKUP (judged vs design language + settings/general)
- **[high] Enable control is native `<input type=checkbox>` + hand-rolled on/off label** → rokkit Switch. `+page.svelte:101`.
- **[med] Invisible row dividers (F7)→`divide-y divide-paper-edge`; `<style>` hardcodes `color:var(--success)` (rule violation)→`text-success` util; bare `<p>` empty state→`EmptyState`; no kind filter/grouping.**
- **[low] kind plaintext→kanji glyph; redundant on/off label; card copy 4 vs 6 kinds; loading `<p>`→Spinner; card p-7.**
- **Rokkit:** **Migrate** enable→Switch; add kind filter→Toggle.

### `settings-instruments` — setup-wizard.jsx (WizRegistry/McpRow)
- **[high] MCP rows are boxed cards + gaps → single hairline container** (List). `InstrumentsSection.svelte:84-91,120-126`.
- **[med] Column-5 badge (matches/kind) dropped (wire has `kind`); summary drops "N recommended" chip + reworded; leading accent checkbox→trailing Switch + installed static pill (not toggleable); kanji tile circle+border vs rounded square; panels `paper-mute`+invisible border (F7); verified bare ✓ vs pill.**
- **[low] stack chip tokens; name weight/publisher; header tagline into `description`; off-state opacity 55 vs 60.**
- **Rokkit:** **Migrate** row lists→List; Switch ✓ (move to leading, checkbox style). Keep hand-rolled: tag/status chips.

### `proj-overview` — project-lite-panes.jsx (ProjOverviewLite)
- **[med] Hero adds Accept+Reject (two ink primaries) vs single send action** — demote one to ghost. `+page.svelte:98-124`.
- **[med] Hero padding 20→24px; "Recent" heading over-tracked 0.18→0.01em; recent-row mono meta ink-faint collapse (F1); all-quiet 静+watermark vs 聴 solid accent.**
- **[low] repos/role pills extra (reflect in mock); link+hover+empty rows extra; stat radius 8; letter-spacing; 4-row cap/max-width.**
- **Rokkit:** hero Buttons ✓ (geometry, demote one). **Migrate** recent rows→List. Keep hand-rolled: ProjPill.

### `proj-memories` — project-lite-panes.jsx (ProjMemoriesLite)
- **[high] Hero whole-card success-soft green vs neutral paper (success only on 送 glyph).** `+page.svelte:133`.
- **[high] Rows are hairline checkbox list, drop per-row kanji + "kind·source" subtext + "N× used".** `+page.svelte:210-243`.
- **[med] PageHeader bare (kanji 覚 + count eyebrow, h1); "Active memories" MiniHeading gone; no loading/error; drawer arbitrary px→scale; `border-paper-mute`→`paper-edge` (F7); opacity hacks→ink tokens; hero glyph/headline/body/meta drift.**
- **[low] empty-state kanji tone; chips solid soft (F2).**
- **Rokkit:** hero/batch/propose/generalise/confirm Buttons ✓; close/widen→Button; **Migrate** memory list→List, scope-ladder→Menu. Keep hand-rolled: checkbox, status chip.

### `proj-about` — project-lite-panes.jsx (ProjAboutPane/ProjSettingsV2)
- **[high] Read-first mode bar (識 + About + TOC + Edit/Done) absent; 280px summary rail (icon editor/quick-facts/anchor-nav) absent; section titles tiny eyebrows/text-base vs display 22px.**
- **[med] Links/Guidelines/Backlog sections missing (wire-defer OK) but Stack drops Runtimes/Services + grouping; stack chips non-mono paper-mute vs mono paper-soft/paper-edge; repo rows drop size·lang/remove/add; `border-paper-mute`→`paper-edge` + drop card fill (F7); Identity missing Name+Icon fields, adds Status/Description.**
- **[low] px-7/mt-5 rhythm; repo path opacity-50→ink-faint; save-error warning→danger; binding chips solid soft (F2); Bindings section extra; header copy.**
- **Rokkit:** maturity Select ✓ (skin), Confirm Button ✓; **Migrate** repos→List, add mode-toggle Button, anchor-nav→List. Keep hand-rolled: text inputs/textareas, stack chips.

### `proj-sessions` — sessions-zen.jsx (via ProjectPageSidebar)
- **[high] Retrospective 3-lane section omitted; hero subtitle still promises "A retrospective".** `+page.svelte:233,97`.
- **[med] `all` range chip missing (mock+spec); hero numbers Fraunces→UI-light/mono; gutters px-6→px-8/px-10; scroll-collapse hero missing; mini-view headline hardcoded; chart annotations dropped.**
- **[low] chips capitalized + no tracking; extra totals strip; eyebrow/headline copy; dot 6→7px + stroke paper→paper-soft; cycler aria-label; hero kanji 刻 vs 録 (spec says 刻).**
- **Rokkit:** **Migrate** range→Toggle, chart chips→Toggle, cycler→Button, session rows→List. Keep hand-rolled: chart SVGs, EmptyState.

### `proj-shell` — project-pages.jsx (ProjectSidebarRouted)
- **[high] Sidebar identity header + "⇆ switch project" absent (no way to switch); per-section count badges missing; Health block reduced to FTR-only (Sessions·7d + Drift dropped, sessions7d already loaded); nav hand-rolled `<a>` + `<style>` color block instead of rokkit List.**
- **[med] 8/11 kanji differ from mock; rail 180→220px; active row full-accent label → ink label + accent-only kanji; inactive full-ink → ink-soft; active bg paper→paper-mute in `<style>` (rule violation); right border paper-mute→paper-edge; shell surface (base paper + sidebar paper-soft); FTR text-2xl font-bold off-scale; full-bleed bars vs inset pills; no shell empty/loading/error; Intake extra.**
- **[low] independent scroll; titlebar subtitle ink-faint→ink-mute; px-4→px-2 inset.**
- **Rokkit:** **Migrate** section nav→List (kanji/label/badge snippet, `[data-component="project-sidebar"]` scope), add switch-project→Button. Keep hand-rolled: Health readout, titlebar.

### `proj-atlas` — project-atlas.jsx
- **[high] Docs layer (drift/broken nodes+edges, toggle, inspector callout, legend, drift stat) — the atlas's defining feature — absent.** `+page.svelte:137-241`.
- **[med] 2 wire levels + Scope dropdown vs 4 authored levels + breadcrumb drill; nodes drop kanji glyph + 2-line label (9px off-scale); header captions ink-faint (dark-mode too dim)→ink-mute; active segment lost shadow; inspector action button missing; header kanji 40→30px; header padding.**
- **[low] extra description+stats block; controls hairline; legend ring ink-faint→ink-mute; chip radius 6→4; literal px; no loading/error; nodes not keyboard-reachable (tabindex -1).**
- **Rokkit:** granularity→Toggle, scope Select ✓ (skin), inspector action + docs toggle→Button. Keep hand-rolled: chip cluster, graph SVG.

### `proj-impact` — project-lite-panes.jsx (ProjImpactLite)
- **[high] App reimplements observatory ObsImpact list+detail vs project ProjImpactLite pane; HeroCard "loop closed" summary absent; PaneHeader (56px kanji + eyebrow) absent; negative verdict warning→accent (`impact.ts:68`).**
- **[med] Verdict pill solid soft + full-sat border → alpha (F2); raised panels paper-mute→paper-soft; before/after grid padded with Status/Verdict filler; detail title 22→28px display-light; impact-log input/textarea `outline-none` (a11y); primary Button vermillion→ink (F9); pane padding 24 vs 32/40/48; no detail action row.**
- **[low] text-[13px]→text-sm; manual Impact-log section extra; empty-state jargon copy; ink-faint labels (F1).**
- **Rokkit:** **Migrate** verdict list→List (tone strip in snippet), decide pills + action row→Button; Log button ✓ (ink override). Keep hand-rolled: inputs, count tiles, verdict pill.

---

## 5. Suggested Fix Order (execute one screen at a time)

**Phase 0 — Foundational tokens/config (blocks everything; do all before touching screens):**
1. **F1** ink-ramp collapse (`rokkit.config.js:70-71`) — the one real darkening defect.
2. **F2** alpha `-soft` model + delete solid-shade hardcodes.
3. **F7** `paper-mute`→`paper-edge` border sweep (lint rule).
4. **F9** `primary` skin = ink.
5. **F3/F4** re-converge AA values + single-source error/info into `lib/tokens.css`.
6. **F5** shared `PageHeader` 40px-kanji variant + `right` slot; **F6** `Eyebrow` tracking; **F8** shared `ScreenState` scaffold + stop null-on-failure loaders.

**Phase 1 — Rokkit override snippets (build the reusable §3 CSS blocks once):** Button, Toggle (A+B), Select, List, Tabs, Table, Menu. Land them scoped so subsequent screens just add a `data-component` wrapper.

**Phase 2 — Screens, by impact.** Group A first (real bugs / feature-level decisions needing an owner), then B (high structural drift), then C (token/type polish).

**Group A — decisions + real bugs (surface to owner before rebuilding):**
7. `memories`/`learnings` (wrong feature; fix `--text-muted` + `#553/#fff` bugs immediately)
8. `consolidation` (different feature)
9. `activity-logs` (different feature / re-scope)
10. `traceability` (full rebuild)
11. `dojo-sharing` (re-scoped) · `settings-inference` (wholesale reimpl)
12. `proj-shell` (`<style>` color-block bug + no switch-project)

**Group B — high structural drift:**
13. `libraries` 14. `today` 15. `insights` 16. `sessions` 17. `proj-sessions` 18. `impact` 19. `proj-impact` 20. `share-review` 21. `dojo-connections` 22. `upgrades` 23. `instruments-obs` 24. `proj-atlas` 25. `proj-memories` 26. `settings-projects`

**Group C — token/type/rokkit polish:**
27. `projects` 28. `obs-sidebar` 29. `settings-general` 30. `settings-instruments` 31. `settings-extensions` 32. `proj-about` 33. `proj-overview`

**Per-screen loop (every screen):** (a) add the `data-component` wrapper + apply the matching §3 override; (b) migrate the flagged widgets to rokkit; (c) fix drift high→low from that screen's §4 list; (d) add loading/error state if missing; (e) verify light **and** dark mode; (f) run zero-errors-policy before commit.
