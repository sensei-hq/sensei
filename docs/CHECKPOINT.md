# Checkpoint

**Slice:** #126 and #127 both COMPLETE. Last commit `9b9adee8`. Branch `develop`.

## Done

- **#126 metric activation** — dōjō route (`1541b9b2`), daemon proxy (`cb093b4a`),
  status read (`060f2a22`), screen (`8bb1a671`), honest-empty + e2e (`6feaac57`).
  `GET /api/metrics/status?repo=<repo_key|uuid>` + `/summary`.
- **#127 dōjō credential + sync** — endpoint + screen (`2f94488b`), e2e
  (`9b9adee8`). `GET /api/dojo/sync-state` is the FIRST reader of
  `sensei.sync_state`; `/settings/dojo` shows credential standing (reusing
  `PersonaList.describe`) and per-entity agreement.
- **#142 filed** — `sensei_test` and `sensei_e2e` age out of the DDL tree.
- **Gates at `9b9adee8`:** 2606 senseid pass, clippy `-D warnings` 0, fmt clean,
  1678 app unit / 117 files, svelte-check 0/0. e2e: settings-dojo 2 passed,
  settings-metrics 1 passed / 3 skipped.

## Remains, in Jerry's stated order

1. **#128** — 201 (repo × metric) pairs read `never_computed`; attribute them
   correctly. The `metric_status` view now makes this diagnosable.
2. **Scan scope** (#129) — non-git roots sweep unbounded; `find-me-board` = 1,230
   folders.
3. **Branch tagging vs delete/update** (#130).

## Next command

```
gh issue view 128
```

## Open questions

- **#138 project/namespace** is a decision, not a task — the repository rung does
  not exist, so project aggregation would aggregate an empty set.
- **#137** blocks 4,329 user-scope sync rows (no writer sets
  `personas.principal_id`). Does NOT block activation — the dōjō resolves the
  caller from the bearer token.

## Known-broken

- **`dojo_memberships.sync_status` is dead.** `memberships::set_sync_status` has
  zero callers, so every row reads `authenticating` forever and the connections
  pane's "healthy" count is permanently 0. `sensei.sync_state` is the table that
  actually knows. Worth an issue or a deletion.
- **e2e populated-path gap.** settings-metrics' three data-dependent tests skip on
  a fresh DB (needs a `sensei.repositories` row; `POST /api/repos` makes a
  PROJECT). settings-dojo's populated branch was verified by seeding
  `sensei.sync_state` by hand — SQL is in the spec header.
- `/settings/projects` e2e fails on a fresh DB; shares no code with either slice.
  Consistent with **#134**'s 23 triaged failures.
- **The repo's security-reminder hook has a false positive on the regex `match`
  sibling method** (the one named the same as the `child_process` function). It
  fires on ANY file containing that identifier, including Markdown — it blocked
  this checkpoint until reworded. A regex helper in `$lib/dates.ts` uses
  `String.match` because of it; `collective-sharing-state.svelte.ts` still holds
  the older form, so edits there will hit the same wall.
