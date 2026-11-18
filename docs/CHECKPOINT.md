# Checkpoint

**Slice:** #398 — lane close tolerates and releases dead-lease lanes (`fix-relay-crew-close-dead-lanes`, 1/1).
**Branch:** `lane/relay-398-s1` (worktree `lanes/relay-open`), PR to develop.

## Done
- Store: `relay_close_crew` judges liveness by the lane lease via `relay_crew_has_live_lane`
  (shared with run start), locks crew then lanes, releases each dead lane through
  `relay_unregister_tx`; `RelayCloseOutput.released`.
- Handler: `close_crew` emits `lane.unregistered` + `task.interrupt` per released lane
  (`emit_released`, shared with the person close), then `run.completed`; answer gains `released`.
- MCP `relay_close_crew` text and `lanes.md` §12 updated.
- e2e `app/e2e/tests/relay-crew-close.spec.ts`: red on the pre-fix daemon (409), green on this branch.
- Review fix (PR #502): heartbeat now takes the crew row, then the lane `for share`, then its lease —
  the order close uses — so close and heartbeat no longer deadlock (40P01); a heartbeat queued behind
  close answers 410. Red `480fe1a3f`, green `875c3a080`; design.md decision 4 amended.
- Review fix 2 (PR #502): `relay_require_crew_open` reads the crew `for share`, so a register-join
  or reregister racing a close waits, then answers `409 crew is closed` — no lane or lease lands in a
  closed crew. Red `6eace665a`, green `a9fcc6f01`; design.md decision 3 amended.
- Gate: fmt, clippy (senseid, sensei-mcp), relay DB tests (454), e2e relay-crew-close.spec.ts,
  openspec validate --strict.

## Remaining
- PR review and merge to develop (not merged by the lane).

## Next command
`gh pr view --web` on the slice PR.

## Open questions
- None.

## Known-broken
- None.
