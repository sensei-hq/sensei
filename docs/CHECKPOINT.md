# Checkpoint

**Slice:** v0.11.0 shipped and merged; graph re-indexing on the new binary.

## Done

- **v0.11.0 released, merged to main, CI green.** `brew fetch` verifies formula
  and cask against the published assets. Suites: senseid 3,312 · bootstrap 175 ·
  app 1,700 · dojo 1,535; clippy `-D warnings` and fmt clean.
- **Repo REPLACED.** `sensei-hq/sensei-archive` is private+archived and **must
  stay so** — unscrubbed history and issues. Backup `~/sensei-history-backup/`.
- **Production dōjō reconciled** — 7 created, 13 altered, `dbd diff` → 0. Applied
  attended: the 5 constraint drops were safe only because all three tables were
  empty (0 rows, verified).
- **`dojo-prod` now has `required_reviewers`** (was `protection_rules=NONE`) —
  #200's actual complaint.
- **Architecture metrics are computing live**: graph_confidence 72 rows,
  public_surface_ratio 76, symbol_size_p95 68.

## In flight

Re-index after `TRUNCATE sensei.files, sensei.nodes, sensei.edges CASCADE`.
Truncating `files` was REQUIRED — `pipeline.rs:121` skips any file whose stored
mtime matches. At 592k nodes / 2.35M edges (was 1.18M / 4.08M).
**90.8% of edges carry a verdict**, written at insert — so
`scripts/backfill-edge-verdicts.sh` is moot here, kept for un-reindexed DBs.

## Next command

    gh run view 36457949929 --repo sensei-hq/sensei   # deploy-dojo awaits approval

## Open

- **deploy-dojo is `waiting` on a human** — deliberately not approved by the
  agent; that is the only step the gate exists to require.
- **#200 / #156** can close once that run is green.
- **#187** — `app/e2e/**` outside every static gate. Pre-existing, untouched.
- **Flip to released?** Deferred. It governs BOTH scopes, so it also ends
  in-place reconcile for local dev. Revisit when DDL churn settles.
- CLAUDE.md's release checklist names two things absent from this tree: the
  `docs_match_code` website gate and dual `docs/skills/` copies.
