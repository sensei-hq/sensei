# 場 · Observatory · Projects

**Segment:** 03 · Observatory — daily use
**Route:** `/projects`
**Source mockup:** [`lib/navigation.jsx`](../../mockups/Sensei/lib/navigation.jsx) → `ProjectsIndexA` (grid + filter + search) and `ProjectCard` (row-1 pills + row-2 description + stat grid)
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
- `GET /api/projects/{id}/ftr` returns `{ ftr14d, sessions7d }`.
- `GET /api/projects/{id}/quality-signals` returns
  `{ open_drift_count, ftr_7d }` — powers the `warn` dot.
- **`icon` is a small union.** `kind: "kanji"` and `value` is a
  single kanji glyph (the default when nothing else is known).
  `kind: "image"` and `value` is a URL/path resolved via
  [[pipeline/project-icon]] — README logo, favicon, or a repo
  logo file. See that spec for the inference chain and cache.
- **`vision` is daemon-owned.** Nullable text on `sensei.projects`.
  Editable from Project › About. Empty when absent — the description
  row simply isn't rendered.

## Signals shown

### Header + toolbar

| Element | Value | Meaning | Example |
|---|---|---|---|
| Header kanji + title | 場 · "All the places you work." | Fixed voice statement | — |
| Filter chip: All (全) | integer count | Total projects | `12` |
| Filter chip: Active (動) | integer count | Projects with `sessions7d > 0` | `4` |
| Filter chip: Dormant (眠) | integer count | Projects with `sessions7d == 0 && !archived` | `7` |
| Filter chip: Archived (蔵) | integer count | Projects with `archived == true` | `1` |
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
- `p.vision` when present renders as row 2 with `pretty` wrapping;
  when absent the row is omitted (no empty gap).
- Project icons inferred via [[pipeline/project-icon]] render for
  projects that have a repo-derived image; others fall back to the
  kanji glyph. The kanji is never rendered on top of an image.
- Dark-mode: all text remains readable; pill backgrounds shift with
  theme.

Optional check:
```
curl -s http://localhost:7744/api/projects | jq '.[0] | {name, icon, vision, repos_count, libs_count}'
# expected: icon is a { kind, value } union; vision may be null
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

## Related

- [[pipeline/analyzer]] — where `ftr14d` and `sessions7d` come from
- [[pipeline/ftr]] — the FTR calculation
- [[pipeline/project-icon]] — README/favicon/logo → project icon
- [[pipeline/insight-copy]] — `vision` text (if daemon-proposed)
- [[screen/project-overview]] — where clicking a card lands
- [[screen/project-about]] — where `vision` is edited
- [[screen/observatory-today]] — peer that reads the same projects list
