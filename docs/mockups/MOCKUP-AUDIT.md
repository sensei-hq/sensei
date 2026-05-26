# Sensei Mockup Audit — Design-System & Component Inventory

> Source: `/Users/Jerry/Developer/sensei-hq/sensei/docs/mockups/` (React 18 + Babel-in-browser JSX, ~40 files).
> Aesthetic: "Zen-Sumi" — washi paper, sumi ink, vermillion accent. Restraint over ornament; hairlines over shadows; air over density.
> Stylesheet of record: `lib/tokens.css` (a custom utility CSS, *not* real Tailwind — but uses Tailwind-style class names so the JSX reads like Tailwind).
> Scope class: any descendant of `.sensei`, `.zs`, or `.artboard-shell` activates the utility classes.

---

## Tokens

### Color palette (CSS variables, `lib/tokens.css:13-122`)

All colours are `oklch()` so they scale into dark mode cleanly. Semantic aliases are mandatory — JSX should never inline raw oklch.

| Variable | Light value (paraphrased) | Role |
|---|---|---|
| `--paper`, `--paper-2`, `--paper-3` | washi-50/100/200/300 (off-white) | Backgrounds, ascending elevation |
| `--edge` | washi-400 | All hairline borders |
| `--ink`, `--ink-2`, `--ink-3`, `--ink-4` | sumi 900 → 300 | Foreground text, descending emphasis |
| `--accent` (= `--shu-500`) | vermillion oklch(0.58 0.15 35) | Primary accent · kanji · 1-pixel left-borders on cards |
| `--accent-soft` | accent @ 12 % | Soft-fill pills/badges |
| `--success` (= `--jade-500`) | jade oklch(0.62 0.08 160) | Positive verdicts, "good" status |
| `--success-soft` | jade @ 14 % | Soft fills |
| `--warning` (= `--amber-500`) | amber | Caution, dormant, drift |
| `--warning-soft` | amber @ 15 % | Soft fills |
| `--danger` | oklch(0.55 0.18 28) | Reserved — rarely used; the negative verdict reuses `--accent` |
| `--focus-ring` | = accent | Outline for `:focus-visible` |
| `--on-primary` | = paper | Text on dark-ink button |

Dark theme (`[data-theme="dark"]`, `tokens.css:125-145`) swaps paper ↔ ink with calibrated muted lightness; **all named tokens preserve their semantic role**. The Tweaks panel in `Sensei Observatory.html` toggles `document.documentElement.dataset.theme`.

**Usage frequency (count of inline `var(--*)` references across all JSX):**

```
817× --ink-3    432× --ink     399× --accent    333× --ink-2
313× --paper   310× --ink-4   250× --paper-2   179× --success
 91× --warning  56× --paper-3  40× --edge
```

→ Pattern: most copy is `--ink-2` or `--ink-3`; primary text is `--ink`; meta/mono text is `--ink-3`/`--ink-4`; accents are deeply rationed (`--accent` shows up only on kanji glyphs, hero-card left-borders, primary buttons, focus dots).

### Spacing scale (4 px base, 10 stops; `tokens.css:91-100`)

```
--space-0..9 = 0 · 4 · 8 · 12 · 16 · 24 · 32 · 48 · 64 · 96
```

Utility classes (`p-N`, `px-N`, `py-N`, `pt/pb/pl/pr-N`, `m-N`, `mx/my-N`, `mt/mb/ml/mr-N`, `gap-N`) all reference these. JSX consistently uses utility classes for spacing (rarely `style={{ padding: … }}`).

**Top spacing utilities seen:**

| Class | Count | Typical use |
|---|---|---|
| `px-2` / `px-3` / `px-4` | 147/141/108 | Card body, button, list row |
| `py-1` / `py-2` / `py-3` | 83/71/58 | Button, chip, row |
| `px-6` / `px-7` | 43/40 | Screen content margin |
| `gap-1` / `gap-2` / `gap-3` | 19/41/22 | Inline group, list, columns |
| `m-0` / `mt-0` / `mb-0` | 84/41/28 | Heading resets |
| `mt-1`, `mb-1` | 80, 65 | Eyebrow → title gap |
| `mb-3`, `mb-5` | 45, 42 | Section internal rhythm |

**Canonical rhythm**

- Page header strip: `pt-5 pb-4 px-6` (or `px-7` on wider pages).
- Card padding: `py-3 px-3` or `py-3 px-4` (denser) / `py-5 px-5` (looser).
- List row: `py-3 px-4 gap-3` with `border-bottom: var(--hairline)`.
- Sidebar nav item: `py-2 px-2 gap-2`.
- Section gap inside main content: `gap-5` / `gap-6` between cards.

> The intent is a strict 4 px grid. There **is no recurring `spacing-sm/md/lg` semantic alias** in the new mockups — utility classes are used directly. This is a deliberate departure from the `semantic-styles` skill's recommendation, but here it works because the scale itself is the vocabulary.

### Type scale (8 stops, `tokens.css:72-79`)

```
--text-xs   11px    --text-sm   13px    --text-base 15px    --text-lg   17px
--text-xl   22px    --text-2xl  28px    --text-3xl  40px    --text-4xl  56px
```

**Observed font-size frequencies** (inline `style.fontSize` numbers — JSX usually uses inline numbers, not the utility classes):

```
1179× 11    599× 13    96× 22    84× 15    60× 17    51× 28    37× 40    32× 56
```

Maps 1-to-1 onto the scale. **Strong convention: `11 / 13` carry 99 % of UI copy.**

Line-height tokens: `--leading-tight 1.2` · `--leading-snug 1.4` · `--leading-normal 1.6` · `--leading-loose 1.75`. JSX uses inline numbers (1.2 / 1.4 / 1.5 / 1.55 / 1.65 — close to tokens; small drift).

**Type-pair recipes (`tokens.css:194-202`):**

| Class | Family | Size | Weight | Letter-spacing | Use |
|---|---|---|---|---|---|
| `.zs-h1` (= screen titles) | Fraunces display | 28 | 400 | -0.02em | Page/section big title |
| `.zs-h2` | Fraunces | 22 | 400 | -0.02em | Sub-page / dialog title |
| `.zs-h3` | Fraunces | 17 | 400 | snug | Column heading |
| `.zs-hero` | Fraunces | 56 | 300 | -0.02em, leading 1.05 | Bootstrap & landing hero |
| `.zs-display-lg` | Fraunces | 40 | 300 | -0.02em | Big numeric (FTR) |
| `.zs-body` | Inter | 15 | 400 | leading 1.6 | Body paragraph |
| `.zs-body-sm` | Inter | 13 | 400 | leading 1.6 | Cards & sub-copy |
| `.zs-meta` | JetBrains Mono | 11 | 400, tabular | — | Timestamps · ids · numbers |
| `.zs-eyebrow` | Inter | 11 | 500 | 0.18em wide, UPPERCASE | Every section header eyebrow |

> Observed: most files **do not** apply `.zs-h1/.zs-h2` directly. They write inline styles that match the recipe (`fontSize: 22, fontWeight: 400, letterSpacing: '-0.01em'`). The Svelte port should keep the recipes but expose them as component variants, not loose CSS classes.

**Three typographic families (`tokens.css:66-69`):**

- `--font-display` Fraunces — every title, every big number, "display" affordance
- `--font-ui` Inter — body, buttons, labels (default body)
- `--font-mono` JetBrains Mono — IDs, durations, percents, timestamps, code (`tabular-nums` on by default via `.mono`)
- `--font-kanji` Yu Mincho / Hiragino Mincho — the `.kanji` glyph next to every heading

### Border radius (`tokens.css:103-106`)

```
--radius-sm   4px    --radius   6px    --radius-lg   10px    --radius-full   9999px
```

**Observed inline `borderRadius:` values** (JSX rarely uses `.rounded-*` — almost always inline numbers):

```
131× 5    123× 6    86× 4    66× 3    35× 8    35× 10    14× 2    12× 999    9× 7    9× 12
```

→ Most are between 3–10. The Svelte port should standardise to `--radius-sm` (4), `--radius` (6), `--radius-lg` (10), and `--radius-full` (999). The 3 / 5 / 7 / 8 drift is incidental — round to the nearest token.

### Shadow & hairlines (`tokens.css:108-115`)

```
--hairline   1px solid var(--edge)           ← used everywhere as the canonical separator
--ink-line   1px solid oklch(0.22 0.012 50 / 0.12)
--shadow-sm  0 1px 2px ink/4%
--shadow     0 1px 3px ink/6%, 0 8px 24px ink/6%
--shadow-lg  0 24px 60px ink/18%
```

The design *prefers hairlines to shadows*. Shadows are rare (the FirstEntry toast, popovers, modals). Most "cards" are just `border: var(--hairline); border-radius: 10`.

### Motion (`tokens.css:118-121`)

```
--ease    cubic-bezier(0.2, 0.6, 0.2, 1)
--dur-fast 120ms     --dur 180ms     --dur-slow 280ms
```

Used for hover/focus colour transitions and tab indicator slides.

---

## Components

The mockups define ~25 visually-recurring shapes. The naming below is the suggested Svelte component name. For each: where it lives, props to extract, variants, and a canonical short snippet.

### 1. `KanjiHeader` — page/section header (the master pattern)

Defined once in `lib/primitives.jsx:194-226`, but most screens *re-inline* the same shape rather than calling it. The Svelte port should consolidate.

**Recipe:** kanji glyph (40px accent) · eyebrow (11px uppercase 0.18em tracking ink-3) · display title (22 or 28 px Fraunces) · optional body paragraph (13 px ink-2) · optional right slot for action/stats. Wrap in `border-bottom: var(--hairline); display: flex; gap: 20; pt-5 pb-4 px-6`.

**Variants observed in the wild:**

- `h1` — kanji 40, title 28 — page hero. Seen in `bootstrap-simple.jsx:188`, `direction-merged.jsx`, `navigation.jsx:50`.
- `h2` — kanji 28, title 22 — sub-page (Inference settings, Collective settings, Impact, Consolidation, Sharing, Traceability). The vast majority.
- `h3` — kanji 22, title 17 — column heading inside split panes.

Canonical inline form, repeated literally 20+ times (`learnings-v2.jsx:20-46`, `impact.jsx:28-54`, `consolidation.jsx:24-50`, `inference-settings.jsx:17-40`, `instruments-simple.jsx:23-58`, `sharing-review.jsx`, `collective-settings.jsx`, `traceability.jsx`, `extensions-browser.jsx:139-177`, ...):

```jsx
<div className="gap-5 pt-5 pb-4 px-6"
     style={{ borderBottom: 'var(--hairline)', display: 'flex', alignItems: 'center' }}>
  <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>{kanji}</div>
  <div style={{ flex: 1, minWidth: 0 }}>
    <div className="mb-1" style={{ fontSize: 11, letterSpacing: '0.18em',
         color: 'var(--ink-3)', textTransform: 'uppercase' }}>{eyebrow}</div>
    <h1 className="display m-0" style={{ fontSize: 22, fontWeight: 400, color: 'var(--ink)' }}>{title}</h1>
    <p className="mt-1 mb-0" style={{ fontSize: 13, color: 'var(--ink-2)',
         maxWidth: 720, lineHeight: 1.55 }}>{description}</p>
  </div>
  {right /* often <MiniStat/> trio behind a border-left */}
</div>
```

**Suggested Svelte API:** `<PageHeader kanji eyebrow title description variant="h1|h2|h3" right={Snippet}/>`.

### 2. `Eyebrow` — uppercase 11 px label

Used >100 times across files. The shape is always:

```jsx
<div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
              textTransform: 'uppercase' }}>{label}</div>
```

The CSS class `.zs-eyebrow` (`tokens.css:202`) does this, but JSX doesn't use it. Same value with 0.14em / 0.16em / 0.12em tracking shows up too — drift to ignore.

**Suggested Svelte API:** `<Eyebrow tone="ink-3"/>` with children.

### 3. `Kanji` — glyph token

Sizes scale-matched to type scale (xs/sm/base/lg/xl/2xl/3xl/4xl); `lib/primitives.jsx:176-185`. Recurs as inline `<span className="kanji" style={{ fontSize: N, color: 'var(--accent)' }}>{glyph}</span>`. Colour defaults to `--accent` but switches to `--warning` / `--success` for tone (e.g. dormant project = warning).

**Suggested Svelte API:** `<Kanji size="xl" tone="accent">観</Kanji>`.

### 4. `MiniStat` (a.k.a. `UgMini` / `IfMini` / `Stat` / `Mini`)

Same shape, defined under different names in `impact.jsx:55`, `inference-settings.jsx:45`, `learnings-v2.jsx:184`, `navigation.jsx:193`, etc.:

```jsx
<div style={{ textAlign: 'center' }}>
  <div className="display" style={{ fontSize: 17, fontWeight: 300,
       color: accent ? 'var(--accent)' : 'var(--ink)' }}>{value}</div>
  <div className="mt-1" style={{ fontSize: 11, letterSpacing: '0.12em',
       color: 'var(--ink-4)', textTransform: 'uppercase' }}>{label}</div>
</div>
```

Grouped in a flex row behind a `border-left: var(--hairline); padding-left: 24`. Used as the right slot on page headers and inside cards.

**Variants:** `accent` (colour), `mono` (font), size (display 17 default / 22 for hero).

### 5. `StatusDot` — coloured pill

Defined once (`primitives.jsx:240-252`) and re-inlined elsewhere (`navigation.jsx:6-12`, `observatory.jsx:725`). Tones: accent / success / warning / ink-3. Sizes 5–8 px.

```jsx
<span style={{ width: 7, height: 7, borderRadius: '50%',
               background: 'var(--success)', display: 'inline-block' }}/>
```

Often paired with a heading or list-row to communicate health.

### 6. `Button` (`zs-btn` family)

CSS classes exist (`tokens.css:474-498`) but JSX usually inlines:

```jsx
<button className="py-2 px-4 gap-2"
        style={{ fontSize: 13, background: 'var(--ink)', color: 'var(--paper)',
                 borderRadius: 5, display: 'inline-flex', alignItems: 'center' }}>
  {label} →
</button>
```

**Variants seen:**

| Variant | Style |
|---|---|
| `primary` (default = "ink") | bg `--ink`, text `--paper`, radius 5–6, padding 9×16 |
| `secondary` | transparent bg, border 1 px `--ink`, text `--ink` |
| `ghost` | transparent bg, text `--ink-2`, hover bg `--paper-3` |
| `accent` | bg `--accent`, text `--paper` (rare — used for "publish" / strong CTA) |
| `danger` / negative | text `--accent`, transparent (action-drawer "revert", e.g. `impact.jsx`) |
| size `sm` | font 11, padding 6×12 |
| size `lg` | font 15, padding 11×18 |

**Suggested Svelte API:** `<Button variant="primary|secondary|ghost|accent" size="sm|md|lg" leading trailing>`.

### 7. `Badge` / `Pill` (`zs-badge`)

Two shapes coexist:

1. **Capsule pill** (verdicts, kind chips): rounded 16 / 18 / 4, padded 4×12. (`impact.jsx:147-156`, `inference-settings.jsx:62-71`, `extensions-browser.jsx:27-43`)
2. **Inline tag** (mono uppercase 11 px badge with soft background): `bg --paper-3` / `--accent-soft` / `--success-soft` / `--warning-soft`, radius 3, padded 2×8. (`observatory.jsx:611-633`)

**Tone classes:** `accent`, `success`, `warning`, `default`. Each tone has matching `*-soft` background + border at ~25 % opacity of the foreground.

```jsx
<span className="mono py-1 px-2" style={{ fontSize: 11, color: 'var(--warning)',
      background: 'var(--warning-soft)', borderRadius: 3, whiteSpace: 'nowrap' }}>
  {x.tag}
</span>
```

**Suggested Svelte API:** `<Badge tone="accent|success|warning|neutral" shape="pill|tag" mono?/>`.

### 8. `Card` (`zs-card`)

Defined once (`tokens.css:512-516`); JSX reproduces it inline as one of two shapes:

- **Standard card** — `background: var(--paper-2); border: var(--hairline); border-radius: 10; padding: 24` (`observatory.jsx:520-526` for the hero card).
- **Accent-edged card** — same plus `border-left: 2px solid var(--accent)` (the "System has learned" rows, `observatory.jsx:666-670`).

Empty-state variant: `border: 1px dashed var(--edge); border-radius: 8; text-align: center; padding: 20` (`observatory.jsx:637-644` and `observatory.jsx:688-700`).

**Suggested Svelte API:** `<Card variant="default|accent-edge|dashed-empty" padding="md|lg"/>`.

### 9. `ListRow` — uniform divider-separated row

The single most common shape in the codebase. Seen in `observatory.jsx:719-744` (recent sessions), `extensions-browser.jsx:46-128` (extension row), `inference-settings.jsx:97-130` (model row), `impact.jsx:72-103` (report list), `consolidation.jsx:71-104` (proposal list).

```jsx
<button style={{
  display: 'grid', gridTemplateColumns: 'auto 1fr auto',
  alignItems: 'start', textAlign: 'left',
  borderBottom: 'var(--hairline)',
  background: active ? 'var(--paper-2)' : 'transparent',
  borderLeft: active ? '2px solid var(--accent)' : '2px solid transparent',
  cursor: 'pointer'
}} className="gap-3 py-3 px-4">
  {/* lead glyph or status dot */}
  {/* main: title + meta */}
  {/* trailing: badge / count */}
</button>
```

**Suggested Svelte API:** `<ListRow leading title meta trailing active selectable={Snippet|boolean}/>`. The "selected" treatment (`border-left: 2px solid accent` + `bg paper-2`) is universal — bake it in.

### 10. `Sidebar` (`ObsSidebar`, `WizRail`, `CollectiveSidebar`)

Same structural shell in `observatory.jsx:266-435`, `setup-wizard.jsx:193-330`, `perspective-split.jsx:87-180`, `direction-merged.jsx`. Recipe:

```jsx
<aside className="py-5 px-3 gap-4"
       style={{ borderRight: 'var(--hairline)', background: 'var(--paper-2)',
                display: 'flex', flexDirection: 'column', overflow: 'auto' }}>
  {/* brand row: kanji 22 + display 17 */}
  {/* group: SectionLabel + NavItem[] */}
  {/* group: SectionLabel + NavItem[] */}
  {/* spacer: flex 1 */}
  {/* footer: daemon heartbeat */}
</aside>
```

Width: **240 px** (Observatory) or **260 px** (Wizard) or **300 px** (Instruments / Impact list-rail). The sidebar **is collapsible to icon-only** in the merged direction (`direction-merged.jsx`) — chevron toggles wide ↔ icon-only.

**`NavItem` sub-component** is the most-duplicated shape across the sidebars. Grid `auto 1fr auto`, gap-2, py-2 px-2, font 13, kanji-left + label + mono badge:

```jsx
<button className="gap-2 py-2 px-2" onClick={...}
        style={{ display: 'grid', gridTemplateColumns: 'auto 1fr auto',
                 background: active ? 'var(--paper-3)' : 'transparent',
                 color: active ? 'var(--ink)' : 'var(--ink-2)', fontSize: 13 }}>
  <span className="kanji" style={{ fontSize: 13, width: 14,
        color: active ? 'var(--accent)' : 'var(--ink-3)' }}>{kanji}</span>
  <span>{label}</span>
  {badge != null && <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{badge}</span>}
</button>
```

**Suggested Svelte primitives:** `<Sidebar width=240 collapsible>`, `<SidebarGroup label>`, `<NavItem id kanji label badge active onClick/>`.

### 11. `TabBar` — horizontal tabs with underline indicator

`inference-settings.jsx:53-72`, `instruments.jsx` (Playground / Replay / Insights), `learnings.jsx`. Recipe:

```jsx
<div className="px-6" style={{ display: 'flex', borderBottom: 'var(--hairline)',
                                background: 'var(--paper-2)' }}>
  {tabs.map(([id, kanji, label]) => (
    <button onClick={() => setTab(id)} className="py-3 px-4 gap-2"
            style={{ display: 'inline-flex', alignItems: 'center', border: 'none',
                     borderBottom: tab === id ? '2px solid var(--accent)' : '2px solid transparent',
                     background: 'transparent', fontSize: 13,
                     color: tab === id ? 'var(--ink)' : 'var(--ink-3)' }}>
      <span className="kanji" style={{ fontSize: 13,
            color: tab === id ? 'var(--accent)' : 'var(--ink-3)' }}>{kanji}</span>
      {label}
    </button>
  ))}
</div>
```

**Suggested Svelte API:** `<TabBar tabs={Array<{id, kanji?, label, badge?}>} active onChange/>`.

### 12. `SegmentedControl` / `ChipRow` — filter chips

Used for status filters, scope filters, time-range filters (`navigation.jsx:66-91`, `extensions-browser.jsx:27-43`, `sessions-zen.jsx` time-range). Compact ink-filled active state:

```jsx
<button className="py-1 px-3 gap-2"
        style={{ fontSize: 11, borderRadius: 4, display: 'inline-flex', alignItems: 'center',
                 background: on ? 'var(--ink)' : 'transparent',
                 color: on ? 'var(--paper)' : 'var(--ink-2)' }}>
  <span className="kanji" style={{ fontSize: 11 }}>{kanji}</span>
  {label}
  <span className="mono" style={{ fontSize: 11, opacity: 0.85 }}>{count}</span>
</button>
```

Two shapes coexist: **square chip** (radius 4–5, ink-on-paper) and **capsule chip** (radius 16, paper-2 with hairline border — used in `project-filter.jsx`).

**Suggested Svelte API:** `<ChipRow value onChange options={[{value, label, kanji?, count?}]} shape="square|capsule"/>`.

### 13. `Input` / `SearchField` (`zs-input`)

`navigation.jsx:93-106`, `project-filter.jsx:87-117`, `learnings-anatomy-v2.jsx:68-76`, `instruments-simple.jsx:114-127`. Recipe:

```jsx
<div className="gap-2 py-1 px-2"
     style={{ display: 'flex', alignItems: 'center',
              background: 'var(--paper-2)', borderRadius: 5,
              border: 'var(--hairline)', minWidth: 260 }}>
  <span className="kanji" style={{ fontSize: 11, color: 'var(--ink-3)' }}>探</span>
  <input value={query} onChange={...} placeholder="search…"
         style={{ border: 'none', outline: 'none', background: 'transparent',
                  fontSize: 13, flex: 1, color: 'var(--ink)' }}/>
  {query && <button onClick={...} style={{ fontSize: 11, color: 'var(--ink-4)' }}>×</button>}
</div>
```

Border-radius varies (5 default, 16 for capsule variant). Focus state recolours border to `--ink`.

**Suggested Svelte API:** `<TextField bind:value placeholder leading={Snippet} trailing={Snippet} variant="default|capsule"/>`. Subtype `<SearchField/>` wraps with the magnifier glyph + clear button.

### 14. `Checkbox` / `Toggle` — minimal

`observatory.jsx:828-830`, `collective-settings.jsx:168-172`, wizard preferences. Just `<input type="checkbox" accentColor: var(--accent)>` plus a label. No fancy custom widget seen — port as plain accessible `<Checkbox bind:checked label/>`.

Radio is similar; segmented control is preferred over radio sets.

### 15. `Select` (native)

Used in `setup-wizard.jsx:2048`, `learnings.jsx:340`, `direction-shoji.jsx:394`. Native `<select>` with `style={{ fontSize: 13, border: 'var(--hairline)', borderRadius: 5, background: 'var(--paper-2)', padding: '6px 10px' }}` — no custom listbox.

### 16. `Toast` — animation-in floating banner

Single instance in `observatory.jsx:805-840` (first-entry welcome). Recipe:

```jsx
<div className="gap-4 py-3 pl-4 pr-5"
     style={{ position: 'absolute', top: 24, left: '50%', transform: 'translateX(-50%)',
              background: 'var(--ink)', color: 'var(--paper)', borderRadius: 10,
              display: 'flex', alignItems: 'center',
              boxShadow: '0 6px 24px rgba(0,0,0,0.2)', zIndex: 20,
              animation: 'toast-in .45s ease-out' }}>
  <Kanji color="--accent" size={22}/> <Title/> <Sub/> <DismissCheckbox/> <Close/>
</div>
```

**Suggested Svelte API:** `<Toast kanji title body actions dismissible/>`.

### 17. `Drawer` / `SidePanel` (memory drawer, action drawer)

`learnings.jsx` has the memory drawer; `project-shared.jsx` has the action drawer / detail pane. The pattern is a fixed-width column (320–420 px) on the right edge with its own hairline border-left and a scrolling body. **Not a true modal** — it slides next to the list, never over it. There is no real "modal/dialog" component in the entire mockup set — every detail view is a side panel.

### 18. `MasterDetail` / `SplitPane`

The split is so common it's worth a named primitive. Patterns:

- `300px 1fr` — list rail + detail (Impact, Consolidation, Extensions, Skill editor, Inference settings tabs)
- `240px 1fr` — sidebar + main (Observatory, Setup)
- `1.4fr 1fr` — Insights ↔ Adopted teachings (`observatory.jsx:508`)
- `1fr 1fr` or `1fr 1fr 1fr` — Triage columns (`learnings-v2.jsx:134`)
- `2fr 1fr` — Project-window two-column (Enso layout)

**Suggested Svelte API:** `<SplitPane left right gutter="hairline" leftWidth="300px" />` and `<ListDetail items selectedId onSelect/>` as the higher-level smart version.

### 19. `EmptyState`

Repeats in observatory (`ObsPlaceholder`, `observatory.jsx:751-779`), adopted-teachings empty (`observatory.jsx:689-700`), instruments-simple, library detail. Recipe:

```jsx
<div className="p-8 mx-auto" style={{ textAlign: 'center', maxWidth: 520 }}>
  <div className="kanji mb-3" style={{ fontSize: 56, color: 'var(--accent)', opacity: 0.5 }}>{kanji}</div>
  <h1 className="display mt-0 mb-2" style={{ fontSize: 28, fontWeight: 300 }}>{title}</h1>
  <p style={{ fontSize: 13, color: 'var(--ink-3)' }} className="mt-0 mb-5">{subtitle}</p>
  <Button variant="secondary">← back</Button>
</div>
```

Quieter inline variant uses `border: 1px dashed var(--edge); border-radius: 8; padding: 16` for cell-level empties.

**Suggested Svelte API:** `<EmptyState kanji title body action variant="full|inline"/>`.

### 20. `Avatar`

`primitives.jsx:124-135` — single-letter, circular, `--paper-3` bg, `--ink-2` text, ink-line border. Sizes 18 / 24 / 32. Pass `name` only.

### 21. `Sparkline` + `EnsoRing` + `BarRow` + `ObsFtrStrip`

The visualisation primitives:

- **`Sparkline`** (`primitives.jsx:7-30`) — fixed-size SVG line, optional area fill, optional last-point dot. Used everywhere a trend is shown (FTR 14-day, project micro-trend, sessions per day).
- **`EnsoRing`** (`primitives.jsx:34-81`) — open-arc circular progress, displays a centre number. Used in Enso direction only.
- **`BarRow`** (`primitives.jsx:84-96`) — single-row bar chart, opacity-encoded.
- **`ObsFtrStrip`** (`observatory.jsx:133-163`) — 14-day FTR bar strip with baseline rule and last-bar highlight. Re-used in the Sessions and project pages.

**Suggested Svelte ports:** Keep each as a standalone primitive (`Sparkline.svelte`, `EnsoRing.svelte`, `BarRow.svelte`, `BarStrip.svelte`). Each takes a number[] and renders pure SVG.

### 22. `TauriChrome`

Top window chrome — height 38, traffic-light buttons left, title centre. Defined once (`primitives.jsx:113-121`) and styled by `tokens.css:543-562`. Already the right abstraction — port straight as `<TauriChrome title/>`.

### 23. `EventGlyph` — minimal vector icons

`primitives.jsx:145-158`. Six tiny SVG glyphs (start / end / context / edit / test / correction) for session timelines. Stroke-based, 16×16 viewBox, currentColor. **The codebase deliberately avoids lucide / Phosphor / emoji** — small bespoke SVGs only. (See *Iconography* below.)

### 24. `Stepper` — wizard left rail

`setup-wizard.jsx:193-330`. Each stage row is either:

- **Collapsed chip** (completed): kanji + truncated title, ink-3 colour, smaller.
- **Expanded current**: kanji 28 + title 17 + sub paragraph.
- **Pending**: same as chip but ink-4 opacity.

Includes a numeric progress strip "03 / 11" at the bottom of the rail.

**Suggested Svelte API:** `<Stepper stages currentIdx onSelect/>` with stage `{id, kanji, title, sub}`.

### 25. `BottomBar` — wizard action bar

`setup-wizard.jsx` — pinned bottom strip with secondary "back" left, primary "next" right, mono progress middle. Hairline border-top. Standard 56 px tall.

### 26. `KeyValueRow` (a.k.a. `ExtProp`)

For property grids and detail panels. Uppercase 11 px label above 13 px ink value:

```jsx
<div>
  <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                textTransform: 'uppercase' }} className="mb-1">{label}</div>
  <div style={{ fontSize: 13, color: 'var(--ink)' }}>{children}</div>
</div>
```

### 27. `BeforeAfter` (impact-specific)

`impact.jsx` — 4-column grid with before / after / delta blocks separated by 1 px paper gaps over an `--edge`-coloured background (so the gaps read as hairlines):

```jsx
<div className="gap-1 mb-5" style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)',
                                      background: 'var(--edge)', borderRadius: 6, overflow: 'hidden' }}>
  <BeforeAfter .../>
</div>
```

Lovely trick — "rule lines as 1 px gaps over a coloured backplate." Worth adopting as a `<HairlineGrid/>` primitive.

### 28. `ActivityTimeline` / `EventStream`

Vertical list of events with a left-rule, kanji glyph or `EventGlyph`, time/mono, body. Used in Sessions zen (`sessions-zen.jsx`), Logs (`project-logs.jsx`), Replay (`mcp-replay-insights.jsx`). Repeats the `ListRow` pattern but with a vertical thread line on the left and tone-coloured glyph.

### 29. `Hairline` / `Rule`

A simple `<hr/>` token — `height: 1px; background: var(--edge); border: none` (`tokens.css:519`). Sometimes inline `<span style={{ flex: 1 }}>·</span>` for inline separators (`impact.jsx:128`).

---

## Layouts

Five layout shells repeat across every screen:

### L1 · Full app shell — TauriChrome + sidebar + main

```jsx
<div className="sensei" style={{ width: '100%', height: '100%',
     display: 'flex', flexDirection: 'column', background: 'var(--paper)' }}>
  <TauriChrome title="…"/>
  <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '240px 1fr', minHeight: 0 }}>
    <Sidebar/>
    <main style={{ overflow: 'auto' }}>…</main>
  </div>
</div>
```

Used by Observatory, Setup wizard, the merged direction, Project window. Every full screen.

### L2 · Hero + content (no sidebar)

Single column, `max-width: 1060` / `1100` / `820`, `mx-auto`, internal padding `pt-6 pb-7 px-7`. Used inside the home view (`ObsHome`), Bootstrap-simple. The hero is a `PageHeader` with a right-side stats group; below is freeform.

### L3 · List + Detail (split pane)

```jsx
<div style={{ flex: 1, display: 'grid', gridTemplateColumns: '300px 1fr', minHeight: 0 }}>
  <aside style={{ borderRight: 'var(--hairline)', overflow: 'auto' }}>…rows…</aside>
  <main style={{ overflow: 'auto' }}>…detail…</main>
</div>
```

Used by Impact, Consolidation, Extensions browser, Skill editor (form-style), Inference settings (tabs use this internally), Libraries, Sharing review.

### L4 · Three-column triage

`grid-template-columns: 1fr 1fr 1fr` with each column getting its own hairline border-right + header + scrollable body. Used by `LearningsTriage`.

### L5 · Stage with footer-actions (wizard)

Sidebar (260) · main (`pt-7 pb-6 px-8`) · pinned `BottomBar`. Used by setup wizard. The wizard is also the only place where the layout owns its own `<TauriChrome/>` rather than letting a host wrap it.

### Plus: bootstrap centred-card layout

Vertical centring with `max-width: 640`, used only by `bootstrap-simple.jsx`. Effectively a more constrained L2.

---

## Iconography

- **Kanji glyphs** are the primary "icon" — every page header, nav item, badge tone, list-row leading slot. Rendered via `<span className="kanji">{glyph}</span>` with the kanji-mincho font stack. The full kanji vocabulary is enumerated in `summary.md` (≈ 40 glyphs).
- **Vector glyphs**: bespoke 16×16 SVG `EventGlyph` (start / end / context / edit / test / correction). Hand-rolled, stroke-based, `currentColor`. (`primitives.jsx:145-158`)
- **No icon library** — no lucide-react, no Phosphor, no Heroicons. Every other icon (chevrons, ×, ▶, search magnifier, traffic lights) is inline SVG or a literal Unicode character.
- **Status dots** are 5–8 px coloured circles via `<span style={{ width, height, borderRadius: '50%', background }}/>`.

→ The Svelte port should keep this discipline: **expose a `<Kanji>` primitive and a tiny `<Icon>` set hand-drawn as Svelte components**. Do not pull in lucide.

---

## Color-usage patterns

- **Semantic vs raw**: 99 % of the code uses CSS-variable tokens (`var(--ink-2)`). The Svelte port must reuse the same variable names so themes carry over. Raw `oklch()` or hex appears only in `Sensei Observatory.html` body (canvas backdrop) and the `tauri-traffic` light dots.
- **Three-tone messaging**: positive = `--success` (jade), warning = `--warning` (amber), accent / urgent = `--accent` (vermillion). The negative-verdict screen in `impact.jsx` reuses `--accent`, not `--danger`. The `--danger` token is defined but unused; **do not introduce a fourth signal colour**.
- **Backgrounds form an elevation ladder**: `--paper` (base) < `--paper-2` (cards, sidebars) < `--paper-3` (active nav row, hover, search input bg). Dark theme keeps the same relative order.
- **Soft fills** for badges and toned highlights use the `*-soft` variants (`--accent-soft` 12 %, `--success-soft` 14 %, `--warning-soft` 15 %). Border on the soft pill is 25–30 % of the foreground.
- **Inline left-borders** (`borderLeft: 2px solid var(--accent)`) are the "this is the focal / active / canonical row" treatment. Recurs in the hero "System has learned" card, the active list-row in master/detail, accent-edged cards. Worth codifying as `<Card edge="accent"/>`.

---

## Open questions

1. **`zs-*` CSS classes vs inline styles.** `tokens.css` defines `.zs-btn`, `.zs-badge`, `.zs-card`, `.zs-input` — but the JSX bodies inline the same recipe instead. For the Svelte port, decide whether to:
   - (A) Port `zs-*` to Svelte CSS class wrappers and call them from `class:` directives, or
   - (B) Bake the recipes into Svelte components so consumers never see the classes.
   Recommendation: **B**. Components own the recipe; tokens.css ships only the variables + minimal utilities.

2. **Tailwind vs utility classes.** The mockups use Tailwind-named utility classes (`py-2 px-4 gap-3`) but these are handwritten in `tokens.css` (scoped to `.sensei`). The Svelte app likely already has real Tailwind — confirm whether the port should:
   - swap `.sensei .py-2` to actual Tailwind config (cleaner), or
   - keep tokens.css as a global stylesheet (faster, less risk during port).
   Recommendation: ship `tokens.css` as the canonical token sheet and let real Tailwind generate the utilities from a matching theme config (single source of truth for the scale; utility classes "just work" everywhere).

3. **Numeric `borderRadius:` drift (3 / 5 / 7 / 8).** Standardise on the four tokens (4 / 6 / 10 / 999) during port; map 3→4, 5→6, 7→6, 8→10. Confirm with design before flattening.

4. **No real modal/dialog component anywhere.** Confirm the desktop app should treat detail views as side panels, not overlays. If a true modal is needed (e.g. confirm-destructive), it will be the first one — define it deliberately.

5. **Font loading.** Fraunces + Inter + JetBrains Mono are pulled from Google Fonts via `@import` in `tokens.css:11`. Tauri apps should ship these locally (no runtime CDN). Plan to vendor + `@font-face` them.

6. **Kanji fallback fonts.** `--font-kanji: 'Yu Mincho', 'Hiragino Mincho ProN', 'Songti SC', serif`. On Linux there's no Mincho — kanji will render as Songti/Noto Serif CJK. Verify on each target platform.

7. **The `.sensei` / `.zs` / `.artboard-shell` triple scope.** Mockups need all three for the design-canvas (`.artboard-shell` wraps each artboard, `.sensei` wraps app roots, `.zs` is an alias). In the real Svelte app, only one is needed — recommend keeping `.sensei` as the canonical scope class and deleting `.zs` + `.artboard-shell` from the ported tokens.css.

8. **Component naming.** The mockups re-define the same shape under file-local names (`UgMini`, `IfMini`, `Mini`, `Stat`) — these are all the same `<MiniStat/>`. The port should land on one canonical name per pattern. The names suggested above (`PageHeader`, `Eyebrow`, `Kanji`, `MiniStat`, `StatusDot`, `Button`, `Badge`, `Card`, `ListRow`, `Sidebar`, `NavItem`, `TabBar`, `ChipRow`, `TextField`, `SearchField`, `Toast`, `Drawer`, `SplitPane`, `ListDetail`, `EmptyState`, `Avatar`, `Sparkline`, `EnsoRing`, `BarRow`, `BarStrip`, `TauriChrome`, `EventGlyph`, `Stepper`, `BottomBar`, `KeyValueRow`, `HairlineGrid`, `ActivityTimeline`, `Hairline`) cover the full set.

9. **Source-of-truth file**. The mockup folder contains both `lib/tokens.css` and `site/tokens.css` (duplicates). The HTML loads only `lib/tokens.css`. Verify the two are identical before porting (a quick `diff` step) and treat `lib/` as canonical.

10. **Dark mode parity.** Every screen renders under both themes via the `[data-theme="dark"]` block (`tokens.css:125-145`). Visual QA needed in dark mode for each ported component — some inline `rgba(0,0,0,…)` shadows (e.g. `FirstEntryToast`) will need to switch to ink-relative shadows so they remain visible against ink backgrounds.

---

## File index (relevant to the port, by pattern)

| Pattern | Canonical file(s) |
|---|---|
| Tokens | `lib/tokens.css` |
| Primitives | `lib/primitives.jsx` |
| PageHeader (h1) | `lib/bootstrap-simple.jsx`, `lib/navigation.jsx`, `lib/instruments-simple.jsx` |
| PageHeader (h2 with stats) | `lib/impact.jsx`, `lib/consolidation.jsx`, `lib/inference-settings.jsx`, `lib/sharing-review.jsx`, `lib/collective-settings.jsx`, `lib/traceability.jsx`, `lib/learnings-v2.jsx` |
| Sidebar + NavItem | `lib/observatory.jsx`, `lib/perspective-split.jsx`, `lib/direction-merged.jsx` |
| Sidebar (wizard rail / stepper) | `lib/setup-wizard.jsx` |
| TabBar | `lib/inference-settings.jsx`, `lib/instruments.jsx`, `lib/learnings.jsx` |
| ChipRow | `lib/navigation.jsx`, `lib/extensions-browser.jsx`, `lib/sessions-zen.jsx` |
| TextField / SearchField | `lib/navigation.jsx`, `lib/project-filter.jsx`, `lib/learnings-anatomy-v2.jsx`, `lib/instruments-simple.jsx` |
| ListRow + selected accent | `lib/observatory.jsx`, `lib/extensions-browser.jsx`, `lib/impact.jsx`, `lib/consolidation.jsx`, `lib/inference-settings.jsx` |
| Card | `lib/observatory.jsx` (hero), `lib/navigation.jsx` (project card), `lib/sessions-zen.jsx` (retro card) |
| EmptyState | `lib/observatory.jsx` (ObsPlaceholder, adopted empty) |
| MiniStat group | `lib/impact.jsx` (UgMini), `lib/inference-settings.jsx` (IfMini), `lib/learnings-v2.jsx` (Mini), `lib/navigation.jsx` (Stat) |
| Toast | `lib/observatory.jsx` (FirstEntryToast) |
| Drawer / side panel | `lib/learnings.jsx` (memory drawer), `lib/project-shared.jsx` (action drawer) |
| Sparkline / EnsoRing / BarRow | `lib/primitives.jsx` |
| ObsFtrStrip / multi-series area / scatter / bands / pulse | `lib/observatory.jsx`, `lib/sessions-zen.jsx` |
| Stepper + BottomBar | `lib/setup-wizard.jsx` |
| EventGlyph | `lib/primitives.jsx` |
| TauriChrome | `lib/primitives.jsx` + `lib/tokens.css:543-562` |
| Activity timeline | `lib/sessions-zen.jsx`, `lib/project-logs.jsx`, `lib/mcp-replay-insights.jsx` |
| Design-canvas wrapper (mockup-only, NOT to port) | `lib/design-canvas.jsx` |
