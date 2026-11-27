# #527 parity — observatory pages off legacy `Screen` (D87 §9)

16 files migrated, zero legacy `Screen` importers left under `(observatory)/`.
Every existing spec and e2e is unedited and green (4110 unit tests), except the
one sanctioned structural edit noted at the bottom. `bun run check` clean.

View ids are `observatory.<route>` (one per kind of view, §3). Every page keeps
its data hooks, empty states and error states; shell search stays off except
where the page had none to begin with (all 16 keep bespoke or no search).

## Per-screen moves

- `help/` — header mark/title/eyebrow → shell mark/title/context; static body
  unchanged. No page specs. [verified: suite]
- `learnings/` — header → shell; tabs → body top; list → body; detail aside →
  sidebar slot; page-owned `overflow-y` removed (shell scrolls). Hooks kept
  (`tab-*`, `list-*`, `detail-pane`, `learnings-page`). [page.spec,
  MemoryDetail.spec, e2e learnings-triage]
- `upgrades/` — header + description → shell; dojo + analyzer lanes unchanged
  (incl. `data-lane`, `data-upgrades-total`). [page.spec, buckets,
  dojo-upgrades-state, upgrades-load specs]
- `workspaces/` — header → shell; ⌘K hint chip → shell description (same text,
  shell styling); filter bar (status toggle, view toggle, search, count) →
  filters snippet with bindings untouched, still hidden on load error; cards
  and rows unchanged. [page.spec, card/row/glyph/view/load specs]
- `consolidation/` — header → shell; version/sections/status trio → stats band
  (hidden until a ruleset reads); body unchanged. [state + view specs]
- `impact/` — header → shell; verdict counts → stats band (hidden on read
  error, per the error-state spec); body unchanged. [page.spec]
- `traceability/` — header → shell; body unchanged. [page.spec, load, state]
- `instruments/` — tab-reactive hero → shell mark/title/context/description;
  tabs → body top (roles, labels, `instrument-tab-*` kept); tab bodies
  unchanged; page-read glyph mode passed to the shell so title and rows agree.
  [page.spec, state/view/groups/replay/marks specs, e2e
  instruments-observatory]
- `operations/` — header → shell; seven sections unchanged (children snippet
  kept). [suite]
- `libraries/` — header → shell; bespoke search + lists unchanged. [page.spec,
  read, state specs]
- `+page` (today) — greeting/date → shell title/context; FTR block (value,
  delta, strip, all states) → body top, right-aligned, hooks kept
  (`data-today-header`, `data-ftr-header`, `ftr-bar-today`); hero, columns and
  sessions unchanged. [today-page, HeroKoan, InsightCard, AdoptedCard specs,
  e2e observatory-daily]
- `insights/` — header → shell; now/soon/settled counts → stats band;
  workspace strip → filters snippet; action notice → body top; columns
  unchanged; orphaned `stat` snippet removed. [page.spec + card specs]
- `knowledge-sources/` — header → shell; add-source form + cards unchanged.
  [page.spec (13), read spec]
- `share-review/` — header → shell; review body unchanged (`data-share-review`
  kept). [screen + state specs, 44 tests]
- `dojo/connections/` — header → shell; sign-in link → toolbar `add` button
  (still an anchor to `/settings/dojo` with `data-sign-in`); cards unchanged.
  [page.spec incl. the `a[data-sign-in]` assertion, state spec]
- `dojo/sharing/` — header → shell; preferences form unchanged. [state spec]

## Sanctioned structural edit (one)

`page-header-icons.spec.ts` (#133) scanned `PageHeader` tags only and required
more than 20 of them — the migration retires exactly that. It now scans
`PageHeader` tags AND `ShellScreen` marks with the invariant unchanged (every
kanji mark names its icon); today gained its `KANJI_ICONS` icon so the total
holds with zero exceptions.
