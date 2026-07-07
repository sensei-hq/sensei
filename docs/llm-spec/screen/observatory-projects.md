# 場 · Observatory · Projects

**Segment:** 03 · Observatory — daily use
**Route:** `/projects`
**Source mockup:** [`lib/navigation.jsx`](../../mockups/Sensei/lib/navigation.jsx) → `ProjectsIndexA` (variant A — the current wire target)
**Rebuild landed:** 2026-07-07 (this session)
**App file:** `app/src/routes/(observatory)/projects/+page.svelte`

## Purpose

Jerry (or any user) has multiple projects going. This is the index — the
place to jump from *"I want to look at X"* to *the project window for
X*. It has to be dense enough that everything the user works on today
is visible in one glance, and sortable/filterable so a growing list of
40+ projects doesn't drown the ones that matter this week. Kanji is
場 — *place* — because this is the map of where you work.

The user's mental model is not "here are three buckets I have to
scroll past" — it's "everything is here, filtered by what I care about
right now."

## Data invariants

- `sensei.projects` has ≥ 1 row.
- `GET /api/projects` returns projects with `id`, `name`, optional
  `client`, `maturity`, `stack.languages`, optional `icon.value`.
- `GET /api/projects/{id}/ftr` returns `{ ftr14d, sessions7d }` — the
  card-strip metrics. If the analyzer never ran, both are 0. That is
  honest, not a bug.
- `GET /api/projects/{id}/quality-signals` returns `{ open_drift_count,
  ftr_7d }` — used to compute the `warn` dot on the card.
- **No project shape hides behind `archived`** unless the daemon
  actually persists an archived flag. The UI already tolerates absent
  archived — it just shows `all: N, active: M, dormant: L, archived: 0`.

## Signals shown

| Element | Value | Meaning | Correct example |
|---|---|---|---|
| Header count | `{active} · {recent} · {archived}` totals | (Removed in rebuild — replaced by chip counts + running "N of M"). Kept in this spec so we notice if it comes back. | — |
| Filter chip: All (全) | integer count | Total projects | `12` |
| Filter chip: Active (動) | integer count | Projects with `sessions7d > 0` | `4` |
| Filter chip: Dormant (眠) | integer count | Projects with `sessions7d == 0 && !archived` | `7` |
| Filter chip: Archived (蔵) | integer count | Projects with `archived == true` | `1` |
| Search | filter text | Matches `name` OR `client` case-insensitively | Type `sen` → shows `sensei` |
| Running tally | `{filtered} of {total}` | After chip + search filter | `3 of 12` |
| Card kanji | `p.icon.value` or `場` fallback | Domain kanji chosen at project setup | `雲` |
| Status dot | success / warning / ink-mute | success if `!warn && ftr14d >= 0.8`, warning if `warn`, ink-mute otherwise | warning dot on `sensei` while FTR is low |
| FTR stat (active only) | integer percent | `Math.round(ftr14d * 100)` | `63` |
| 7d stat (active only) | integer | `sessions7d` | `18` |
| stack strip | first 2 of `stack.languages` | Not a full inventory — just orientation | `rust / typescript` |
| Dormant/archived subline | `stack · maturity` | Honest terse row when there's no recent activity to summarise | `python · exploratory` |

## Done gate

- Loading `/projects` with the sensei binary on Jerry's live data shows
  chips with correct counts summing to `all`.
- Search field filters in real time; typing `sen` narrows to a single
  card; clearing restores all.
- Clicking any card opens the project window (Tauri capability
  regression check).
- No section headings — filters replace them. The layout is a single
  grid, sorted active-first by FTR desc, then dormant, then archived.
- Cards render two variants correctly: **active** shows the FTR/7d/stack
  stat strip; **dormant/archived** show the terse `stack · maturity`
  row and an uppercase status chip.
- Dark-mode: all text remains readable; card backgrounds shift with
  theme; no color-hardcoded values.

Optional check:
```
curl -s http://localhost:7744/api/projects | jq 'length'
# expected: > 0
curl -s http://localhost:7744/api/projects/sensei/ftr | jq '.ftr14d'
# expected: >= 0.4 (once analyzer has run for sensei)
```

## Wrong gate

- **Chip counts don't sum to All.** Means bucket derivation disagrees
  with the chip filter — pick one truth.
- **Search reveals no match on a project that IS in the list.** Means
  the search hay-string doesn't include what the user sees on the card.
- **FTR chip shows a number but the project window shows 0.** Means the
  `/api/projects/{name}` name-vs-UUID resolution regressed.
- **Cards show client=`—` for every project.** Means the API isn't
  returning `client`, or the mockup is over-promising a field the
  daemon doesn't set. The daemon should be the source of truth.
- **Any card cannot be opened** — Tauri capability regression. See
  `app/src-tauri/capabilities/default.json` — the `windows` list must
  include `project-*` and the permissions must include
  `core:webview:allow-create-webview-window`.
- **⌘K jump-to-project hint appears but ⌘K does nothing.** Either wire
  the shortcut or remove the hint.

## Related

- [[pipeline/analyzer]] — where `ftr14d` and `sessions7d` come from
- [[pipeline/ftr]] — the FTR calculation itself
- [[screen/project-overview]] — where clicking a card lands
- [[screen/observatory-today]] — the peer that reads the same projects list
