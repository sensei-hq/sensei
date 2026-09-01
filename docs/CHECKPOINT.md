# Checkpoint

**Slice:** #126 · #127 · #128 · #129 · #143 all COMPLETE and closed.
Last commit `ea6fcbd8`. Branch `develop`.

## Done this run

| issue | what landed | commits |
|---|---|---|
| #126 | metric activation: read, write, screen | `060f2a22` `8bb1a671` `6feaac57` |
| #127 | dōjō credential + sync-state surface | `2f94488b` `9b9adee8` |
| #128 | 12 pairs read never_computed while computing | `3d3f0790` |
| #129 | per-root scan exclusions, reachable at last | `9d1aca8b` `3265a355` |
| #143 | the scan now honours those exclusions | `ea6fcbd8` |

- **Gates at `ea6fcbd8`:** 2607 senseid pass / 0 fail / 6 ignored, clippy
  `-D warnings` 0, fmt clean, 1698 app unit / 118 files, svelte-check 0/0.
- **Live result:** `find-me-board` 1,230 → 18 folders; total 8,984 → 7,772; and
  it survives a re-scan. `metric_status` 1,943 rows / 67 repositories intact.

## Remains, in Jerry's stated order

1. **#130** — watcher: a branch switch forces a full reconcile; should tagging
   replace re-walking?
2. Then the mockup-alignment pair (#131 sensei, #132 dōjō) and #133
   (kanji ↔ icon display mode), which is where "clean up the product" was headed.

## Next command

```
gh issue view 130
```

## Filed this run, not started

- **#142** `sensei_test` / `sensei_e2e` age out of the DDL tree.
- **#144** repo-less projects should say `scope=repo` metrics cannot exist.

## Open questions

- **#138 project/namespace** is a decision, not a task — the repository rung does
  not exist, so project aggregation would aggregate an empty set.
- **#137** blocks 4,329 user-scope sync rows (no writer sets
  `personas.principal_id`). Does not block activation.

## Known-broken

- **`dojo_memberships.sync_status` is dead.** `memberships::set_sync_status` has
  zero callers, so every row reads `authenticating` and the connections pane's
  "healthy" count is permanently 0. `sensei.sync_state` is the table that knows.
- **e2e populated-path gap.** settings-metrics' three data-dependent tests skip on
  a fresh DB (needs a `sensei.repositories` row; `POST /api/repos` makes a
  PROJECT). settings-dojo's populated branch was verified by hand-seeding
  `sync_state` — SQL is in that spec's header.
- `/settings/projects` e2e fails on a fresh DB, unrelated to these slices (#134).
- **Removing an exclusion enqueues a re-scan that races the prune.** Observed
  while applying one: clear-then-set in quick succession let the re-scan restore
  rows the prune had just removed. Harmless now that the scan honours exclusions,
  but the sequencing is still unguarded.
- **The security-reminder hook false-positives** on the regex `match` sibling
  method, in ANY file including Markdown — it blocked a checkpoint until reworded.
  `$lib/dates.ts` uses `String.match` because of it.
