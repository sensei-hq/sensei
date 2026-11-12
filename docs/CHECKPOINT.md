# Checkpoint — 2026-11-12 (lanes/relay-queue on lane/relay-autonomous-queue, cut from develop @ 598045196)

**Slice:** Relay autonomous queue — phase 2 (daemon). Task 2.1 (DDL cutover + rename blast radius) SHIPPED;
task 2.2 next. Plan: `docs/plan/2026-11-10-relay-autonomous-queue/2-daemon.md` (2.1 expanded to steps S1–S5).

**Done:**
- Phase 1 (DB additive) on develop @ `b9eb57ac2`; phase-1 checkpoint docs @ `598045196`.
- Task 2.1 as a red/green pair: `75bb808c9` (pins) + `ddaf9be49` (cutover). `tasks` drops `reviewer_lane_id`,
  defaults `ready`, `assigned_at` nullable, `tasks_lane_coherence`, `tasks_one_active_worker_per_lane`; the daemon
  gains `RelayTaskState::Ready`, `RelayActionKind::ReviewClaimed`, `relay_claim_review` +
  `POST /api/relay/tasks/{id}/claim_review` (pulled in from 2.4/2.9 so a review can be submitted at this
  checkpoint); report floats, review authorises on the claim, re-offer lands `ready`, the sweeper leaves
  `in_review` alone and drops the reviewer re-pick; the task view serves `reviewer {id, code}`.
- Proof: 14/14 relay SQL on a fresh `sensei_test_relay_autonomous_queue`; `cargo test -p senseid relay` 297/297;
  `cargo test -p sensei-mcp` green; clippy `-D warnings` + fmt clean; `make test-fast` green (hooks ran on both
  commits).

**Remaining:** 2.2 vocabulary/state machine/props/errors (guidance v2 moves to 2.9 with its routes), 2.3 plan parse,
2.4 claims/release/checkpoint/limit_resume, 2.5 runs/Task 0/unpack, 2.6 report gate/envelope/heartbeat/reregister,
2.7 sweeper, 2.8 gate, 2.9 handlers/routes/events, 2.10 live converge + install + smoke; then phases 3–5.

**Next command:** in `lanes/relay-queue`: write the 2.2 reds in `crates/senseid/src/relay/mod.rs`
(transition_queue_events, validate_props_queue_kinds, F09 keys, error codes, constants), `cargo test -p senseid relay::`.

**Open questions (user):** none blocking. Deviations recorded under 2.1 in `2-daemon.md`.

**Known broken:** none on the branch. `sensei_test` (Rust template) is stamped for THIS tree — rebuild with
`make test-template` from whichever worktree runs the Rust suite next (#142). Live daemon DB NOT converged (2.10).
