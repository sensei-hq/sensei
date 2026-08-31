# Checkpoint

**Slice:** Product cleanup — trail filed as issues, metrics first.
Last commit `cb093b4a` (daemon activation proxy). Branch `develop`, 2 unpushed.

## Done

- **#126 metric activation, both write halves.** Dōjō route + service
  (`1541b9b2`), daemon proxy `PATCH /api/dojo/metric-activation` (`cb093b4a`).
  Gates at `cb093b4a`: 2603 senseid pass, clippy `-D warnings` 0, fmt clean,
  1622 app unit pass. Two mutation probes red (401/503 collapse; dropped trim).
- **`sensei.metric_status` view** — one row per (repository × metric) with
  watermark + status reason. Reads cadence off `w.last_sha`, not a hardcoded
  group list. 402 live rows.
- **Trail filed: #126–#140.** Nothing left only in conversation.

## Remains, in Jerry's stated order

1. **Metrics** — `/settings/metrics` page (#126's UI half): rows from
   `sensei.metric_status`, reason + watermark beside each toggle, PATCH on click.
   Then #127 (dōjō connections + token expiry), then #128.
2. **Scan scope** (#129) — non-git roots sweep unbounded; `find-me-board` = 1,230
   folders.
3. **Branch tagging vs delete/update** (#130).

## Next command

```
# the daemon side is done and live; build the settings page against this view
cat database/ddl/view/sensei/metric_status.ddl
```

## Open questions

- **#138 project/namespace** is a decision, not a task — the repository rung does
  not exist, so project aggregation would aggregate an empty set.
- **#137** blocks 4,329 user-scope sync rows: no production writer sets
  `personas.principal_id`.

## Known-broken

- Repo-scope compute skip (#140) never observed live — dbd has no commits after
  its sealed day, so no row appears either way.
- 23 e2e failures triaged but unfixed (#134); no baseline recorded yet.
