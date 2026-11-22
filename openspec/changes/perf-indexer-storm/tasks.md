## 1. Compile governor (test-first)

- [x] 1.1 Global semaphore (2 permits) around compiler subprocess spawn;
  concurrency test with the Counting engine proves max overlap
- [x] 1.2 Same-(engine, root) in-flight sharing; duplicate compiles run once
- [x] 1.3 tsconfig-less skip tried and REVERTED (regressed working
  packages to unresolved); governor contains storms without skipping
  compilable work
- [x] 1.4 Live check: storm cleared, zero grinders recur; cap unit-proven
  (machine load since is qlty's, not the daemon's)
- [ ] 1.5 Follow-up (not this change): root-cause the ts-driver hang on
  some tsconfig-less input

## 2. Stale prune (test-first)

- [x] 2.1 Shared live-folder gate: vanished dir deletes its tree + Ok skip
- [x] 2.2 `detect_communities` + `scan_git_history` use the gate; no retry
- [x] 2.3 Live check: zero new ghost failures; rows converged; the one
  live-path/no-row case errors per spec; delete branch unit-proven

## 3. Phase timing (test-first)

- [x] 3.1 DDL `phases_ms` JSONB on `activity.task_executions` (full-table
  file, `dbd` deploy, test-template rebuild)
- [x] 3.2 Task-local outbox carries splits; executor persists on completion
- [x] 3.3 `process_file` records parse/world/compile-wait/resolve/write + spans
- [x] 3.4 Live check: kit/types.ts attributes 48s to write_ms;
  skips stay NULL (avg 3ms)

## 4. List caps (test-first)

- [x] 4.1 Alerts/failures/queue/history render at most 200 + overflow row
- [x] 4.2 Component specs pin cap + honest counts; full unit green

## 5. Clear redelivery (test-first)

- [x] 5.1 Pending set: uncleared drops redeliver every refresh till written
- [x] 5.2 Per-PUT timeout unblocks the flush; in-flight guard stops pile-up
- [x] 5.3 Ack cancels pending; generations drop stale late responses
- [x] 5.4 4.4 green — root cause was harness-side, not app-side: the
  backgrounded e2e webview suspends timers, so a DB-only wait starved
  the 30s alert poll and the write never fired. The wait now keeps the
  webview awake with a per-iteration section read (2/2 pass with it,
  0/11 without); app code unchanged by the diagnosis
- [ ] 5.5 Follow-up (not this change): derive alerts on feed updates,
  not only on poll/mount — a backgrounded prod window sleeps the same
  way, delaying clears until foreground; the poll should be a backstop

## 6. Gates + land

- [ ] 6.1 Full unit + senseid suites green; clippy/fmt/fabrication clean
- [ ] 6.2 Tauri matrices green (crash + ops-center, no regressions)
- [ ] 6.3 Commit, push, checkpoint, report
