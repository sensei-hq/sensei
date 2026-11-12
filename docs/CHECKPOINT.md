# Checkpoint — 2026-11-12 (lanes/relay-queue on lane/relay-autonomous-queue, cut from develop @ 598045196)

**Slice:** Relay autonomous queue — phase 2 (daemon). Tasks 2.1–2.6 SHIPPED as red/green pairs; task 2.7
(sweeper) next. Plan: `docs/plan/2026-11-10-relay-autonomous-queue/2-daemon.md` (each task has ticked steps +
"Executed" deviations).

**Done:**
- Phase 1 (DB additive) on develop @ `b9eb57ac2`; phase-1 checkpoint docs @ `598045196`.
- 2.1 DDL cutover `75bb808c9`+`ddaf9be49`; 2.2 vocabulary/state machine/props/errors; 2.3 plan parse
  (`relay/plan.rs`); 2.4 claims/release/checkpoint/limit_resume; 2.5 runs/Task 0/unpack/complete
  (`df4fb2787`+`99cb93dfb`); 2.6 store cutover `d0db800bd`+`f1a50fa39` (deps gate, F01/F03/F04/F05/F08/F10,
  nothing-to-merge → done, #402 join-active-crew, `RegisterMode::{Mint,Join,JoinActive}`).
- Proof at `f1a50fa39`: `cargo test -p senseid relay -- --test-threads=1` 338/338; `cargo test -p sensei-mcp` green;
  clippy `-D warnings` + fmt clean; `make test-fast` green (hooks ran on both commits).

**Remaining:** 2.7 sweeper (end pauses at resumes_at → `Resumed(LimitPauseEnded)`, abandoned review claims after 60 s
→ `released {kind: review}`, waiter eviction, nudges after 900 s, ready broker-task selection, report fields),
2.8 deterministic gate (`relay/gate.rs`, finding classes via `dbd import`), 2.9 handlers/routes/events/guidance v2,
2.10 live converge (`dbd reconcile --allow-destructive`) + `make install-service` + smoke; then phases 3–5.

**Next command:** in `lanes/relay-queue`: write the 2.7 reds (report fields + `RelaySweptTask::ReviewReleased` stubs,
store pins `sweep_ends_a_pause_at_resumes_at_not_before`, `sweep_releases_an_abandoned_review_claim_after_the_grace`,
`sweep_evicts_waiters_of_dead_lanes`, `sweep_nudges_everyone_about_a_floating_review_once_per_window`,
`sweep_reports_a_ready_broker_task_whose_deps_are_done_one_per_pass`), `cargo test -p senseid sweep -- --test-threads=1`.

**Open questions (user):** none blocking. Deviations recorded per task in `2-daemon.md`.

**Known broken:** none on the branch. `sensei_test` (Rust template) is stamped for THIS tree — rebuild with
`make test-template` from whichever worktree runs the Rust suite next (#142). Live daemon DB NOT converged (2.10).
