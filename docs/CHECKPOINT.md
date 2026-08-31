# Checkpoint

**Slice:** #126 metric activation — COMPLETE, all three halves.
Last commit `6feaac57`. Branch `develop`.

## Done

- **#126 read + write + screen.** Dōjō route (`1541b9b2`), daemon activation
  proxy (`cb093b4a`), daemon status read (`060f2a22`), Settings · Metrics
  (`8bb1a671`), honest-empty + e2e (`6feaac57`).
- **New endpoints:** `GET /api/metrics/status?repo=<repo_key|uuid>` and
  `GET /api/metrics/status/summary`. `repo` is REQUIRED — the view cross-joins,
  so the estate is unbounded (10.9M rows in `sensei_test`).
- **`sensei.reason_codes` has a store module** (`pg_store/reasons.rs`). It is
  cross-domain by design; sharing and schedules resolve there next rather than
  writing a second reader.
- **Gates at `6feaac57`:** 2605 senseid pass, clippy `-D warnings` 0, fmt clean,
  1652 app unit pass / 115 files, svelte-check 0/0.
- **Live-verified** against the 67-repository install: summary merges Acolytes'
  three reason codes into one entry (2+3+24=29); no-repo → 400; unknown → 404;
  uuid → 200. e2e `settings-metrics` 1 passed / 3 skipped.

## Remains, in Jerry's stated order

1. **#127** dōjō connections + forge-token expiry screen, then **#128**
   (201 never_computed pairs).
2. **Scan scope** (#129) — non-git roots sweep unbounded; `find-me-board` = 1,230
   folders.
3. **Branch tagging vs delete/update** (#130).

## Next command

```
gh issue view 127
```

## Open questions

- **#138 project/namespace** is a decision, not a task — the repository rung does
  not exist, so project aggregation would aggregate an empty set.
- **#137** blocks 4,329 user-scope sync rows: no production writer sets
  `personas.principal_id`. Does NOT block activation (the dōjō resolves the
  caller from the bearer token).

## Known-broken

- **`sensei_test` and `sensei_e2e` age out of the DDL tree.** Both were missing
  `metric_computation` reason codes; `sensei_e2e` was missing the whole
  `metric_status` view. A FRESH e2e DB deploys correctly — an existing one is
  never re-deployed. Fixed by hand this round; worth an issue.
- **e2e metric fixture gap.** The three data-dependent `settings-metrics` tests
  skip on a fresh DB: seeding needs a `sensei.repositories` row and no daemon
  endpoint creates one (`POST /api/repos` makes a PROJECT).
- `/settings/projects` e2e fails on a fresh DB (shares no code with #126);
  consistent with the 23 triaged-unfixed failures in **#134**.
- **#140** repo-scope compute skip is now OBSERVABLE via `reason_code`.
