# Project Management — Detailed Requirements (M16)

> **Status:** Requirements gathering — feeds the M16 module spec, its `.feature`
> set, and implementation.
> **Sources (visual, never edited):** `mockups/Sensei/lib/project/project-pages.jsx`,
> `perspective-split.jsx`, `project-lite-panes.jsx`, `project-shared.jsx`,
> `project-metrics.jsx`, `mockups/Sensei/lib/observatory/sessions-zen.jsx`,
> `mockups/Sensei/lib/data/project-data.js`.
> **How to read:** every *mockup fact* below cites its symbol; anything marked
> **(proposed)** is a requirement this doc introduces, not something the mockup
> shows. Data-model claims are grounded in `database/ddl/`.
> **Companion:** [`diagrams-zoom-pan-detail-requirements.md`](diagrams-zoom-pan-detail-requirements.md)
> (the diagram surfaces the window embeds), [`integrations-and-gating-requirements.md`](integrations-and-gating-requirements.md) (the T3 tracker/target layer).

---

## 1 · The four layout variations

`project-pages.jsx` header: *"Three variations of the Project page — Top-tabs /
Left-rail / Long-scroll."* A fourth artboard follows it.

| Variation | Function | Shape |
|---|---|---|
| **A — Top tabs** | *(classic)* | A horizontal tab bar over the content; scannable |
| **B — Left inner rail** | *(more content real estate)* | A narrow inner rail beside the content |
| **C — Long scroll** | *(most zen)* | One page, anchored section links |
| **D — Own left sidebar** | `ProjectPageSidebar` | A full project sidebar (identity · sections · diagrams · health) |

**Decision: Variation D is implemented.** Everything from §2 on describes D.

---

## 2 · Variation D — the project window shell

`ProjectPageSidebar({ initialSection = "overview", embedded = false, onBack, onSwitchProject, projectId, state = "ready" })`:

- **Non-ready states** render a `<window.ScreenState>` (empty/error) before the window.
- **`embedded` mode** hides the `PerspectiveChrome` (this is how the Observatory
  mounts the same window via `<ProjectPageSidebar embedded …/>`).
- **Chrome:** `PerspectiveChrome title="先生  ·  {project.name}" subtitle="project window"`.
- **Grid:** `grid-template-columns: 220px 1fr` — sidebar + main.
- **Main pane:** if the active id is a diagram id (`PROJ_DIAGRAMS.some(d => d.id === sec)`)
  the main pane is `overflow-hidden` and renders `ProjDiagramFrame`; otherwise it
  scrolls and renders the section pane.

### 2.1 Sidebar composition (`ProjectSidebarRouted`)

Top to bottom:

1. **Project identity** — `ProjIdentityCard`.
2. **"This project"** — the section list (§3).
3. **"Diagrams"** — `PROJ_DIAGRAMS`, grouped Architecture / Code (§5).
4. **"Health"** — `ProjHealthSpark`.
5. A flexible spacer.
6. **`ProjMarkSwitch`** (§6).
7. A footer line, `scoped to this project`.

### 2.2 Project identity card (`ProjIdentityCard`, `perspective-split.jsx`)

- Kanji mark, a **status dot**, the project name, and a client pill.
- Dot colour: `warn` → warning; else `ftr ≥ 0.8` → success; `ftr ≥ 0.6` → ink-3;
  else accent.
- A **`⇆ switch`** button calls `onSwitchProject`.

The card deliberately matches a *projects-index* card (*"the same shape … so opening
a project keeps the object you clicked"*). There is **no repo-count badge, no
quick-actions row** here — a thing the earlier draft of this doc invented.

---

## 3 · The section list (authoritative)

Variation D's sidebar is the inline list in `ProjectSidebarRouted`
(`project-pages.jsx`). **This is the list to build.**

| # | id | Kanji | Icon | Label | Badge |
|---|---|---|---|---|---|
| 1 | `overview` | 全 | `home-2` | Overview | — |
| 2 | `sessions` | 録 | `history` | Sessions | 28 |
| 3 | `metrics` | 測 | `chart-2` | Metrics | 12 |
| 4 | `memories` | 覚 | `lightbulb` | Memories | 11 |
| 5 | `traceability` | 巻 | `link` | Traceability | 4 |
| 6 | `libraries` | 庫 | `library` | Libraries | 5 |
| 7 | `instruments` | 具 | `tuning-2` | Instruments | 7 |
| 8 | `patterns` | 紋 | `layers` | Patterns | 3 |
| 9 | `impact` | 果 | `target` | Impact | 2 |
| 10 | `about` | 識 | `info-circle` | About | — |

Badges are **hardcoded strings** in the mockup. **(Proposed):** each badge is a live
count from its section endpoint (§7.1), and a zero badge is hidden.

> **Finding — two section lists exist.** `perspective-split.jsx`
> (`PROJ_SIDEBAR_SECTIONS`) is a *different* list: it adds `atlas` (図, badge 4),
> spells Sessions 刻 and the last item `settings` (設), and omits `about`. It is
> used by variations A–C's `ProjectSidebar`. Variation D uses the inline list
> above. These must be reconciled to one list — and per
> [the Atlas/Diagrams decision](../decisions.md), Atlas is the Graph UI surface,
> so an `atlas` *section row* duplicates the Diagrams group. **Recommendation:**
> the 10-row list above wins; Atlas stays in the Diagrams group.

> **Finding — a third, smaller list.** `project-pages.jsx` `PROJ_SECTIONS` (used by
> variations A–C, not D) lists only five: `overview`, `graph`, `patterns`,
> `sessions`, `settings` (labelled *About*). It should not drive implementation.

---

## 4 · Section panes

Variation D routes each id to a *lite* pane (a calm summary with one highlighted
insight and a single primary action) or, for diagrams, to `ProjDiagramFrame`.

### 4.1 Overview — `ProjOverviewLite`

- **Header** (`PaneHeader`): eyebrow `Project · {client|internal}`, title = project
  name; right side = **FTR · 14d** as a rounded percent (warning colour when
  `project.warn`).
- **Hero** (`HeroCard`, kanji 聴): the top recommendation —
  `headline = title`, `body = why`, action = **`send to {defaultAcp}`**,
  meta = `evidence.join(" · ")`. Empty state: *"All quiet — no urgent
  recommendations."*
- **Three stat blocks:**
  - `Sessions · 7d` → `project.sessions7d`, sub = count of recent sessions that
    were corrected;
  - `Memories` → *"2 to share · 1 to merge"*;
  - `Doc drift` → *"3 of 18 referenced docs"*, **warning** tone.
- **"Recent in this project"** (`MiniHeading` 今): up to 4 rows — title; sub
  `{id} · {duration} · {n corrections | "first-try right"}`; right = time; right
  tone success when the session was first-try-right.

### 4.2 Sessions — `SessionsDigestZen` (`observatory/sessions-zen.jsx`)

The Project window reuses the Observatory's sessions digest, filtered by project
(`projectFilter`, `projectLabel`).

- **Charts:** `trend` · `stream` · `constellation` · `bands` (and a `pulse`
  fallback) — selected by the `chart` state.
- **Range:** `daysForRange(range)`; the hero owns range + a **mini-mode cycler**
  over `numbers | trend | stream | constellation | bands | pulse`.
- **Totals:** count, projects, good/bad/ugly quality, median minutes.
- **Row/detail:** opens the session (the doc's earlier claim of a
  `goto('/instruments?tab=replay&session={id}')` route is **not** in this mockup;
  replay lives in M18 — treat the exact route as **(proposed)**).

### 4.3 Metrics — `ProjMetricsLite` (`project-metrics.jsx`)

- Non-ready → `ScreenState` (kanji 測). Otherwise it renders either `MtList`
  (metric list) or `MtDetail` (one metric), driven by project `MT_METRICS`.
- **(Proposed):** the list carries the seven gating/tracked metrics with a
  gating badge (`none`/`delta`/`ratchet`) per
  [`integrations-and-gating-requirements.md`](integrations-and-gating-requirements.md) §3.5.

### 4.4 Memories — `ProjMemoriesLite` (`project-lite-panes.jsx`)

- **Header:** eyebrow `This project · 11 memories`, title *Memories*.
- **Hero** (kanji 送, success tone): *"2 ready to share with the collective"* →
  action **`review next batch`**, meta *"reviewed batches · 47 / 47"*.
- **Active list** (`MiniHeading` 覚, right *"9 active · 2 sharing"*): each row is
  kanji · title · `{kind} · {source}` · `{places}× used`; share rows are
  success-accented with a left border.

The *promotion-ladder* labels (proposed → reinforced → battle-tested …) are **not**
in this pane; that lifecycle belongs to M9 Memory & Knowledge, not M16.

### 4.5 Traceability — `ProjTraceabilityLite`

- **Hero** (kanji 繕, warning): *"3 docs drifted · 1 broken"* → action
  **`run fix-drift prompt`**, meta *"checked nightly · 18 docs tracked"*.
- **Drift cards:** doc reference; `↪ {symbol}`; right badge `synced {n}d ago` or
  **`⚠ broken link`** (broken shows accent); severity via the left border
  (`high` → accent, `medium` → warning); a one-line note.

### 4.6 Libraries — `ProjLibrariesLite`

- **Hero** (kanji 繋): *"One library is unwrapped — tailwindcss"* → action
  **`wrap tailwindcss`**, meta *"4 of 5 wrapped"*.
- **Rows** (kanji 庫, or 入 for an imported lib): name; sub = source
  (`detected · package.json` / `detected · Cargo.toml` / `imported · llms.txt`);
  right = `{uses}× used · {具 wrapped | — unwrapped}` with success/warning tone.

The earlier draft's *version-pin chip*, *lockfile resolution* and
*`usage_count_14d ≥ 25` generated-skill threshold* are **not** in this pane — they
are M12 Library requirements.

### 4.7 Instruments — `ProjInstrumentsLite`

- **Header:** *"This project · 7 instruments"*.
- **Hero** (kanji 調, warning): the weakest tool — *"patterns.find-similar is your
  weakest tool here"* → action **`open replay for {tool}`**, meta *"164 calls · 7d"*.
- **Rows:** tool name; sub `scope · {scope}`; right `{calls7d} calls · {n}% FTR`,
  success when ≥ 95%.

The tabs *Playground / Replay / Health* and the used/partial/ignored verdict
vocabulary belong to **M18 Instruments**, not this summary pane.

### 4.8 Patterns — `ProjPatterns` (`project-shared.jsx`)

- A two-way toggle (`PatSideBtn`):
  - **"Patterns in use"** (紋, count = `patterns.followed`) — *"Constructive patterns
    sensei detects across your code — promoted to rules once adopted."*
  - **"Anti-patterns"** (避, warning, count = `patterns.antiPatterns`) —
    *"Duplication, god-nodes, monoliths. Where fixable, sensei suggests a pattern
    that would resolve it."*
- Two columns: a list (`FollowRow` / `AntiRow`) and a detail (`FollowDetail` /
  `AntiDetail`); clicking a row sets focus; an anti-pattern can jump to the
  resolving pattern. Adoption/promotion goes through `openAction` (the action
  drawer).

It is **two** modes, not three — the earlier draft's "Design / Anti / Opt" is wrong.

### 4.9 Impact — `ProjImpactLite`

- **Header:** eyebrow includes the pending count; title *Impact*.
- **Hero** (kanji 勝, success): *"2 wins this month"* → action **`see all impact
  reports`**, meta *"2 positive · 0 negative · 1 pending"*.
- **Verdict cards:** title; `accepted {n}d ago`; when before/after exist, a
  `before FTR → after FTR` line with `+{n} pts`; a note. Verdict tone:
  `positive` → success, `negative` → accent, `pending` → ink-3.

The earlier draft's *"C-8 full-screen alert"* and *"FTR-chart trend markers at
`applied_at`"* are **not** in this pane — drop or re-scope them.

### 4.10 About — `ProjAboutPane` → `ProjSettingsV2`

- `ProjAboutPane` is the **original settings layout in read mode**, with a single
  top-right **`✎ Edit` / `✓ Done`** toggle; in read mode the edit affordances are
  hidden by an injected style block (`[data-editing="false"]`).
- It renders `ProjSettingsV2` (§5.1) — *"identity, stack, repos, links,
  guidelines, backlog"*.

---

## 5 · About / Settings document

### 5.1 `ProjSettingsV2` sections (authoritative)

`const sections = ["identity","stack","repos","links","guidelines","backlog"]` — a
`280px 1fr` grid: a sticky left summary + a document of `V2Block`s.

**Left summary:** icon swatch (change icon), project name, client, goal; quick
facts *(repos, skills on/of, links, backlog)*; anchor nav.

**Blocks:**

| id | Fields |
|---|---|
| `identity` | Name · Client · Goal (multiline) · Icon (`kanji · {value}`, change) |
| `stack` | Languages · Frameworks · Runtimes · Services (each with `+ add`) |
| `repos` | rows: id, path, `size · lang`, `remove`; `+ add repo` |
| `links` | the project's links (kind/label/url) |
| `guidelines` | the project's guidelines (rule text) |
| `backlog` | the project's backlog (task, added) |

> **Finding.** The earlier draft listed nine settings tabs (*General · Folders ·
> Sessions · Assistants · Inference · Integrations · Diagrams · Notifications ·
> Advanced*) and an "Integrations" tab with JQL/WIQL/webhooks. **None of that is
> in `ProjSettingsV2`.** Tracker configuration is a Dōjō/T3 concern
> (`dojo2-project.jsx` *Work trackers*), see
> [`integrations-and-gating-requirements.md`](integrations-and-gating-requirements.md).

---

## 6 · Diagrams in the window

### 6.1 Inventory (`PROJ_DIAGRAMS`, `project-pages.jsx`)

| Group | Label | `tab` | `view` | Badge |
|---|---|---|---|---|
| Architecture | Zones | calls | zones | — |
| Architecture | Dependency matrix | calls | dsm | — |
| Architecture | Layers | calls | layers | — |
| Architecture | Cycles | calls | cycles | — |
| Architecture | Hidden coupling | calls | cochange | — |
| Architecture | Complexity | calls | poly | — |
| Architecture | Ownership | calls | own | — |
| Code | Structure | calls | structure | — |
| Code | World | calls | world | — |
| Code | Neighbourhood | calls | hood | — |
| Code | Schema | schema | — | 10 |

### 6.2 Frame (`ProjDiagramFrame`)

- **src:** `encodeURI(DIAGRAM_DOC) + "?embed=1&tab={tab}" + (view ? "&view={view}" : "")`.
- **Parent → child:** `{type:"sensei-diagram", tab, view}` on mount/each frame, and
  `{type:"sensei-theme", theme: documentElement[data-theme] || "light"}` on load and
  on every `data-theme` mutation (`MutationObserver`).
- **Child → parent:** `{type:"sensei-diagram-head", sub, meta}` — the frame uses
  `sub` as a subtitle and `meta` in the `PaneHeader` slot
  (`eyebrow = "Diagrams · {group}"`, `title = label`, kanji 表 for `schema` else 構).
- Zoom/pan/detail are specified separately in
  [`diagrams-zoom-pan-detail-requirements.md`](diagrams-zoom-pan-detail-requirements.md).

> **Finding — the embedded document is missing.** `DIAGRAM_DOC =
> "Sensei Schema and Call Graph v8.dc.html"`, but no `v8` file exists under
> `docs/mockups/Sensei/` (the versions present run to `experiments/Schema and Call
> Graph v7.dc.html`, plus `Sensei Schema and Call Graph.dc.html`). Decide the
> canonical embedded document before wiring the frame.

---

## 7 · Marks — kanji ↔ icons

`ProjMarkSwitch` toggles **Kanji** ↔ **Icons** through `useMarkStyle()`.

- localStorage key **`sensei.iconStyle`**, default `"kanji"`, values
  `"kanji"` / `"icons"`.
- Writes dispatch `sensei-iconstyle` and listen to `storage`, so every `<Mark>`
  across the window repaints.
- `<Mark>` falls back to the kanji when the icon is unknown or fails to load
  (`marks.jsx`), so a missing translation never leaves a gap.

The earlier draft's `kanjiMode` state field is a reasonable renamed equivalent;
keep the storage key and event names as above.

---

## 8 · Data model

### 8.1 Mockup shape (`data/project-data.js`)

`window.PROJECT_DATA`:

```
projects[id] = {
  id, kanji, name, client, goal,
  icon:  { kind: "kanji", value, bg, fg },
  stack: { languages[], frameworks[], runtimes[], services[] },
  repos: [ { id, path, size, lang } ],
  ftr, ftrPrev, sessions7d, preferredAcp,
  ftr14: number[]            // 14-day spark series
}
settings = {
  links:      [ { id, kind, label, url } ],
  guidelines: [ { id, rule } ],
  backlog:    [ { id, task, added } ],
  skills:     [ { id, name, on } ]
}
active
recommendations: [ { id, title, why, evidence[], defaultAcp, … } ]
recentSessions:  [ { id, title, duration, corrections, ftr, time, project, … } ]
patterns = { followed[], antiPatterns[] }
```

### 8.2 What the schema already has (`database/ddl/`)

- `sensei.folders.role` is `folder_role` — **architectural**
  (`backend · frontend · library · tool · docs · infra · website · desktop · mobile ·
  config · packaging`), *not* a membership role.
- `sensei.folder_projects` is a **view**, not a table:
  `(folder_id, folder_abs_path, repository_id, project_id, project)`, and it is
  **multi-valued by design** (a canonical repo keyed on its remote can belong to
  two projects). A consumer needing one must name which.

> **Finding — no project membership model exists.** The earlier draft's
> Owner / Contributor / Viewer table, attributed to `sensei.folder_projects`, is
> fabricated: the view has no roles and the enum is architectural. Project-window
> permissions are an **open design question**, not a spec-ready requirement.

---

## 9 · Proposed requirements (introduced here, marked per the rule in the header)

1. **Live badges.** Section badges come from endpoints, not literals; zero
   badges are hidden. §3.
2. **One section list.** Ship the 10-row list (§3); delete/redirect the two
   duplicates (`PROJ_SIDEBAR_SECTIONS`, `PROJ_SECTIONS`).
3. **Section endpoints** (proposed):
   ```
   GET /api/projects/{id}/overview
   GET /api/projects/{id}/sessions?range=7d
   GET /api/projects/{id}/metrics
   GET /api/projects/{id}/memories?scope=project
   GET /api/projects/{id}/drift?state=open
   GET /api/projects/{id}/libraries?wrapped=true
   GET /api/projects/{id}/instruments/tool-stats
   GET /api/projects/{id}/patterns
   GET /api/projects/{id}/impact/verdicts
   GET /api/projects/{id}/about          # identity/stack/repos/links/guidelines/backlog
   PATCH /api/projects/{id}/about
   POST /api/projects/{id}/icon
   POST /api/projects/{id}/switch
   ```
4. **Repo / project binding.** Multi-repo membership uses the existing
   `folder_projects` semantics (multi-valued; name-one where required); the header
   shows a `{n} repos` chip when `repos.length > 1` **(proposed)**.
5. **Project membership / permissions (open).** Distinct from `folder_role`; needs
   its own design (roles, who may edit About, who may switch/merge). Do **not**
   reuse `folder_role`.
6. **Dōjō binding.** About carries an opt-in Dōjō binding; the mechanism is T1's,
   surfaced here **(proposed)**.
7. **Keyboard & responsive (proposed).** Sidebar tab order, `Enter`/`Space` to
   activate, `Escape` to leave the embedded diagram; sidebar collapses below
   ~1200 px. Not present in the mockup — product requirements, to confirm.

---

## 10 · Mockup inconsistencies to resolve

| # | Finding | Action |
|---|---|---|
| 1 | Two/three section lists disagree (§3) | Ship the 10-row variation-D list; retire the others |
| 2 | `DIAGRAM_DOC` names a `v8` file that isn't in the repo (§6.2) | Choose the canonical embedded doc |
| 3 | `renderProjectSection` handles `about` but not `settings`; `PROJ_SECTIONS` uses `settings` | Standardise the About section id to `about` |
| 4 | Earlier source citation `project-sidebar.jsx` | Does not exist — sidebar lives in `project-pages.jsx` (`ProjectSidebarRouted`, `ProjectPageSidebar`) + `perspective-split.jsx` (`PROJ_SIDEBAR_SECTIONS`, `ProjIdentityCard`, `ProjHealthSpark`) |
| 5 | No membership model in the schema (§8.2) | Design it; don't borrow `folder_role` |

---

## 11 · Acceptance criteria (proposed)

| Scenario | Pass condition |
|---|---|
| Open the project window | Sidebar shows identity card, the 10 sections with live badges, the Diagrams group, and the health spark; main pane renders Overview |
| Click a section | Main pane swaps to the section; the sidebar row is active; the URL reflects the section |
| Click a diagram | The frame loads `?embed=1&tab=…&view=…`; `sensei-diagram-head` fills the pane subtitle/meta; theme stays in sync |
| Toggle Kanji/Icons | Every mark repaints; the choice survives reload (`sensei.iconStyle`) |
| Embed in the Observatory | `embedded` hides the chrome; the window still routes |
| Multi-repo project | Repos render with `size · lang`; membership resolves through `folder_projects` (multi-valued) |
| About edit mode | Read mode hides every edit affordance; `Done` returns to read mode |
| Non-ready project | The window shows the empty/error `ScreenState`, not a broken shell |

---

## 12 · Related

- [`../prd/modules/03-delivery/16-project-window/README.md`](../prd/modules/03-delivery/16-project-window/README.md) — the M16 spec this feeds.
- [`diagrams-zoom-pan-detail-requirements.md`](diagrams-zoom-pan-detail-requirements.md) — the diagram detail/zoom requirements.
- [`integrations-and-gating-requirements.md`](integrations-and-gating-requirements.md) — the T3 trackers/targets the window surfaces.
- [`../prd/modules/04-team/03-dojo-project-planning/README.md`](../prd/modules/04-team/03-dojo-project-planning/README.md) — tracker sync + gates.
- [`../prd/modules/02-insight/09-memory-and-knowledge/README.md`](../prd/modules/02-insight/09-memory-and-knowledge/README.md) — the memory promotion lifecycle.
- [`../prd/modules/03-delivery/17-atlas/README.md`](../prd/modules/03-delivery/17-atlas/README.md) — the shared diagram embed, in full.
- Mockups: `project-pages.jsx` · `perspective-split.jsx` · `project-lite-panes.jsx` · `project-shared.jsx` · `project-metrics.jsx` · `observatory/sessions-zen.jsx` · `data/project-data.js`.
