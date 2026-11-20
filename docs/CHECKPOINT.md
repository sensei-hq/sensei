# Checkpoint — 2026-11-20 (relay open items — #398, PR #502 landing)

**Slice:** #398 — lane close tolerates and releases dead-lease lanes (`fix-relay-crew-close-dead-lanes`, 1/1).
**Branch:** `lane/relay-398-s1` (worktree `lanes/relay-open`), PR #502 to develop; origin/develop merged in.

## Done
- Store: `relay_close_crew` judges liveness by the lane lease, locks crew then lanes, releases each
  dead lane through `relay_unregister_tx`; `RelayCloseOutput.released`.
- Handler: `close_crew` emits `lane.unregistered` + `task.interrupt` per released lane, then
  `run.completed`; answer gains `released`.
- e2e `app/e2e/tests/relay-crew-close.spec.ts`: red on the pre-fix daemon (409), green on this branch.
- Review fixes 1-7 (PR #502): every crew/lane/task writer takes the crew row first; writers act only
  from the state they read; the sweep skips closed crews. design.md 3.
- Gate: fmt, clippy (senseid), relay DB tests (478), e2e relay-crew-close.spec.ts, openspec validate --strict.
- #389 (PR #508) landed on develop separately; its rokkit upstream issues rokkit#205-#209 stay open.

## Remaining
- PR #502 CI green, merge to develop, close #398.

## Next command
`gh pr checks 502 --watch`

## Open questions
- None.

## Known-broken
- None. Noted, out of slice: `relay_unregister_tx` grants freed resource slots without the
  per-resource advisory lock (pre-existing).
