# Checkpoint — 2026-11-23 (perf-indexer-storm landed)

**Slice:** P1–P5 storm follow-ups + 4.4 harness fix.
**Branch:** `develop` (pushed, 8 commits past v0.26.0 checkpoint).
Tag `v0.26.0` on main; deploy-dojo still waits on user approval.

## Done
- P1 governor (cap 2 + dedup; tsconfig skip reverted, regressed).
- P2 stale prune (live-folder gate in community + git_history).
- P3 phase timing (phases_ms JSONB; task-local outbox; spans).
- P4 list caps (200 + overflow in 4 ops sections).
- P5 clear redelivery (pending + timeout + ack-cancel + gens).
- 4.4 root-caused (harness-side webview sleep; keep-awake read).
- Gates: unit 4102, senseid exit-0, clippy clean, check clean,
  crash 5/5 + ops-center 5/5 Tauri matrices green.

## Remaining
- User: approve deploy-dojo in GitHub.
- Follow-ups (in tasks.md, not this change): 1.5 ts-driver hang,
  5.5 event-driven alert derive (poll is sleep-prone backstop).

## Next command
None — verify live, then pick a follow-up or cut 0.27.0.

## Open questions
- 6 process.rs fmt spots: local 1.99.1/1.100.0 flag what CI
  stable accepts. Left untouched (fixing flips CI red).

## Known-broken
- None.
