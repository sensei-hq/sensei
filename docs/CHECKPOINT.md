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
- Review fixes 1-4 (PR #502): heartbeat, open-crew gate, active-crew join and sweep take the crew row
  before lanes and leases (40P01 / closed-crew joins). Greens `875c3a080`, `a9fcc6f01`, `7517eeb76`,
  `5ef857718`; design.md decisions 3-4.
- Review fix 5 (PR #502, round 5): lane task writes (checkpoint, report, holder start, review, release,
  ask, answer, hand-over) take the crew row `for key share` first (`relay_hold_crew`/`relay_hold_lane_crew`,
  410 for a lane the close released); they deadlocked a close (40P01). Ask and hand-over write only from
  the state they read (409). Red `d48982306`/`9733fe5a1`, green `c91cf752d`/`c17278409`; design.md 3.
- Review fix 6 (PR #502, round 6): the lane's own unregister takes the crew row first (40P01 with
  close; 410 once released). No writer acts on a task state a racer moved on: unregister_tx, sweep
  (expiry/pause/re-offer), report, review, answer read `for update`; merge, task cancel, assign write
  only from the state+holder read (409). Red `f0ba39396` (11 race tests), green `9205e06fd`; design.md 3.
- Gate: fmt, clippy (senseid), relay DB tests (476), e2e relay-crew-close.spec.ts,
  openspec validate --strict.

## Remaining
- PR review and merge to develop (not merged by the lane).

## Next command
`gh pr view --web` on the slice PR.

## Open questions
- None.

## Known-broken
- None. Noted, out of slice: `relay_unregister_tx` grants freed resource slots without the
  per-resource advisory lock (pre-existing).
