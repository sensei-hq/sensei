# 場 · Observatory · Projects

**Segment:** 03 · Observatory — daily use
**Route:** `/projects`
**Source mockup:** [`lib/shared/navigation.jsx`](../../mockups/Sensei/lib/shared/navigation.jsx) → `ProjectsIndexA` (grid + filter + search) and `ProjectCard` (row-1 pills + row-2 description + stat grid)
**App file:** `app/src/routes/(observatory)/projects/+page.svelte`
**Card mockup last changed:** 2026-07-07 (single-row header with pills + description row + repos/libs/last-session strip)

## Purpose

Jerry (or any user) has multiple projects going. This is the index — the
place to jump from *"I want to look at X"* to *the project window for
X*. Dense enough that everything the user works on today is visible
in one glance, sortable/filterable so a growing list of 40+ projects
doesn't drown the ones that matter this week. Kanji is 場 — *place*.

The card is quiet and dense — one row for the identity (kanji · name ·
client · optional status pill), one full-width description row when the
project has a vision, one row of three stats. No wasted vertical space.

## Data invariants

- `sensei.projects` has ≥ 1 row.
- `GET /api/projects` returns projects with:
  - `id`, `name`, optional `client`, optional `vision`, `maturity`,
    `stack.languages`, optional `icon: { kind, value }`,
    `repos_count`, `libs_count`, `sessions7d` (via join),
    `last_session_at` (ISO for dormant/archived).
- **Daemon extension required before this screen builds.** `list_projects`
  (`crates/senseid/src/db/pg_store.rs`) today SELECTs only
  `id, name, description, client, maturity, tags, modified_at`. Extend it to
  also return (all read-only query joins — no schema change):
  - `icon` (`sensei.projects.icon` jsonb) + `stack` (`sensei.projects.stack`
    jsonb) — existing columns, just add to the SELECT.
  - `vision` — `sensei.projects.goal` aliased (see the vision bullet below).
  - `repos_count` — COUNT of the project's **repo-root** folders in
    `sensei.folders` (filter to roots/repo-kind, not every nested subfolder —
    that table holds 10k+ rows including sub-directories).
  - `libs_count` — COUNT over `sensei.project_libraries` for the project.
  - `last_session_at` — MAX(`activity.sessions.started_at`) grouped by
    `project_id`; `sessions7d` — COUNT of the same within 7 days. (`sessions7d`
    may instead reuse the existing per-project `/api/projects/{id}/ftr` fanout
    in `+page.ts` if a list-endpoint join is undesirable.)
- `GET /api/projects/{id}/ftr` returns `{ ftr14d, sessions7d }`.
- `GET /api/projects/{id}/quality-signals` returns
  `{ open_drift_count, ftr_7d }` — powers the `warn` dot.
- **`icon` is a small union.** `kind: "kanji"` and `value` is a
  single kanji glyph (the default when nothing else is known).
  `kind: "image"` and `value` is a URL/path resolved via
  [[pipeline/project-icon]] — README logo, favicon, or a repo
  logo file. See that spec for the inference chain and cache.
- **`vision` is sourced from `sensei.projects.goal`** (nullable "purpose"
  text). There is no `vision` column — `goal` is the semantic match; the
  separate `description` column is the short summary. The API exposes it as
  `vision` (`SELECT goal AS vision`). Editable from Project › About (which
  edits `goal`). Empty when absent — the description row isn't rendered.

## Signals shown

### Header + toolbar

| Element | Value | Meaning | Example |
|---|---|---|---|
| Header kanji + title | 場 · "All the places you work." | Fixed voice statement | — |
| Filter chip: All (全) | integer count | Total projects | `12` |
| Filter chip: Active (動) | integer count | Projects with `sessions7d > 0` | `4` |
| Filter chip: Dormant (眠) | integer count | Projects with `sessions7d == 0 && maturity != "archived"` | `7` |
| Filter chip: Archived (蔵) | integer count | Projects with `maturity == "archived"` (there is no `archived` boolean — it is the `maturity` enum value) | `1` |
| **View toggle** | 2-way: grid (田) / list (≣) | User-persisted preference. Defaults to `grid`. | selected: `list` |
| Search input (探) | text | Matches `name` OR `client` case-insensitively | Type `sen` → `sensei` |
| Running tally | `{filtered} of {total}` | After chip + search filter | `3 of 12` |

### Card — new layout (2026-07-07)

The card is three vertical bands (row 1, optional row 2, stat grid).

**Row 1 — identity strip**

| Element | Value | Meaning |
|---|---|---|
| Kanji or image | `p.icon.value` (18px) | Kanji glyph OR small image from [[pipeline/project-icon]] |
| Status dot | success / warning / ink-mute | `warning` if `p.warn`, `success` if `!warn && ftr14d >= 0.8`, else ink-mute |
| Name | text (13px) | Truncates with ellipsis |
| Client pill (`ProjPill`) | 10px uppercase mono | Always renders; empty client shows `—` or omits |
| Status pill (`ProjPill tone="dormant"`) | 10px uppercase mono | Only when `p.status !== "active"` (dormant / archived) |

**Row 2 — description (optional)**

| Element | Value |
|---|---|
| Vision text | `p.vision`, 12px, ink-2, `textWrap: pretty`. Row omitted entirely if empty. |

**Stat grid — 3 columns, top-border hairline**

Active card (`hasStats`, `sessions7d > 0`):

| Slot | Label | Value | Notes |
|---|---|---|---|
| 1 | `ftr` | `round(ftr14d * 100)` | Warn tone when `p.warn` |
| 2 | `repos` | `p.repos_count` | |
| 3 | `libs` | `p.libs_count` | |

Dormant/archived card (`!hasStats`):

| Slot | Label | Value | Tone |
|---|---|---|---|
| 1 | `repos` | `p.repos_count` | ink-3 |
| 2 | `libs` | `p.libs_count` | ink-3 |
| 3 | `last session` | relative label from `p.last_session_at` | ink-3 |

**Notes on the delta from the old card:**
- `7d` (sessions-in-7d) is dropped from the active card — it moved
  out to the filter-chip count and is redundant on the card.
- Dormant now shows THREE stats (repos / libs / last session), not
  a terse two-item line.
- Client + status moved from muted-subtitle to pill.

### List view (`ProjectRow`)

When the view toggle is set to **list (≣)**, the grid is replaced
by a single bordered container of rows. Each row is a 4-column
grid: kanji · [name + pills + optional vision one-liner (truncated
to one line)] · ftr%-or-last-session · repos/libs. Same data as
the card, one screen-line per row, no borders between rows except
a hairline separator.

| Row column | Value | Notes |
|---|---|---|
| 1. Kanji or image | as in card | 18px |
| 2. Name block | status-dot · name · ProjPill(client) · ProjPill(status) | Row 2 (vision) truncates to a single line with ellipsis; hidden when `p.vision` is empty |
| 3. Signal cell | active → `{ftr}% ftr`; dormant/archived → `last · {relative}` | Warn tone when `p.warn` |
| 4. Repos·libs cell | `{repos_count} repos · {libs_count} libs` | Mono, right-aligned, `min-width: 78px` |

**Row behaviour:**
- Same `onClick → openProjectWindow` as the card.
- Hover: row background lifts to `bg-paper-mute`.
- Archived rows carry opacity `0.6`, matching the card variant.
- No stat grid — the row is one line by design; drill in for full stats.

**When to reach for list vs grid:**
- Grid — the default. Good for ≤ 40 projects, best for a browsing
  glance.
- List — better at scale (100+ projects), better on narrow windows,
  and better when the user wants to eyeball FTR across many
  projects at once.

The user preference is **persisted** in `localStorage` under the key
`sensei:projects:view` (value `"grid"` | `"list"`; no settings-row endpoint
exists today) so a return visit lands in the same view. No per-project override.

## Done gate

- Loading `/projects` on Jerry's live data shows chips with counts
  summing to `all`.
- Search filters in real time; typing `sen` narrows to sensei;
  clearing restores.
- Clicking any card opens the project window (Tauri capability
  regression check).
- Cards render both variants: **active** shows the ftr/repos/libs
  strip; **dormant/archived** show repos/libs/last-session with the
  status pill and muted tone.
- The view toggle switches the layout without refetching data;
  filter chip state and search query survive the toggle.
- List-view rows show the same identity elements as cards (kanji,
  status dot, name, pills) plus a single FTR-or-last-session cell
  and a repos·libs cell.
- User's grid/list preference is remembered across sessions.
- `p.vision` when present renders as row 2 with `pretty` wrapping;
  when absent the row is omitted (no empty gap).
- Project icons inferred via [[pipeline/project-icon]] render for
  projects that have a repo-derived image; others fall back to the
  kanji glyph. The kanji is never rendered on top of an image.
- Dark-mode: all text remains readable; pill backgrounds shift with
  theme.

Optional check (passes only after the `list_projects` daemon extension above):
```
curl -s http://localhost:7744/api/projects | jq '.[0] | {name, icon, vision, repos_count, libs_count, last_session_at}'
# expected: icon is a { kind, value } union; vision may be null (sourced from goal)
# before the extension, icon/vision/repos_count/libs_count are absent
```

## Wrong gate

- **Chip counts don't sum to All.** Bucket derivation disagrees with
  the chip filter.
- **Row 2 shows an empty box for projects without a vision.**
  Should be omitted entirely, not rendered empty.
- **Active card still shows a `7d` stat.** Delta not applied.
- **Client pill is missing on projects that HAVE a client.** Pill
  render skipped when it shouldn't be.
- **Kanji glyph AND inferred image both render on the same card.**
  Fallback logic OR-collapsed both.
- **Repo/lib counts differ from Project › Overview for the same
  project.** Two derivations of the same aggregate.
- **`last session` says `1 minute ago` for a dormant card whose
  Sessions view shows 0 sessions in the last 30 days.** Wrong join
  — likely reading created-at rather than last-session-at.
- **Card can't be opened** — Tauri capability regression.
  `app/src-tauri/capabilities/default.json` must include
  `project-*` in `windows` and `core:webview:allow-create-webview-window`
  in permissions.
- **Search misses a project whose name contains the query.** Hay
  string omitted the field.
- **View toggle switches back to grid on every page load.** User
  preference not being persisted; the mockup's default is `grid`
  but the toggle must respect the last-chosen value.
- **List-view row wraps to two visual lines when the window is
  narrow.** Row is meant to stay one screen-line — vision text
  must truncate to a single line with ellipsis, not wrap.
- **Vision text renders untruncated in list view.** Ruins the
  one-row-per-project promise.

## Related

- [[pipeline/analyzer]] — where `ftr14d` and `sessions7d` come from
- [[pipeline/ftr]] — the FTR calculation
- [[pipeline/project-icon]] — README/favicon/logo → project icon
- [[pipeline/narration-cache]] — `vision` text (if daemon-proposed)
- [[screen/project-overview]] — where clicking a card lands
- [[screen/project-about]] — where `vision` is edited
- [[screen/observatory-today]] — peer that reads the same projects list
