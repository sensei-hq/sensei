# Checkpoint — 2026-11-12 (lanes/relay-queue on lane/relay-autonomous-queue, cut from develop @ 598045196)

**Slice:** Relay autonomous queue — phase 2 (daemon). Tasks 2.1–2.7 SHIPPED as red/green pairs; task 2.8
(deterministic gate) next. Plan: `docs/plan/2026-11-10-relay-autonomous-queue/2-daemon.md` (each task has ticked
steps + "Executed" deviations; 2.8's steps are expanded).

**Done:**
- Phase 1 (DB additive) on develop @ `b9eb57ac2`; phase-1 checkpoint docs @ `598045196`.
- 2.1 DDL cutover `75bb808c9`+`ddaf9be49`; 2.2 vocabulary/state machine/props/errors; 2.3 plan parse
  (`relay/plan.rs`); 2.4 claims/release/checkpoint/limit_resume; 2.5 runs/Task 0/unpack/complete
  (`df4fb2787`+`99cb93dfb`); 2.6 store cutover `d0db800bd`+`f1a50fa39` (deps gate, F01/F03/F04/F05/F08/F10,
  nothing-to-merge → done, #402 join-active-crew, `RegisterMode::{Mint,Join,JoinActive}`); 2.7 sweeper
  `d5f9c1952`+`220f216a0` (pause end at resumes_at, abandoned review claims after 60 s, waiter eviction, nudges per
  900 s window, ready broker-task selection, `LIMIT_PAUSE_CLEAR`).
- Proof at `220f216a0`: `cargo test -p senseid relay -- --test-threads=1` 343/343; clippy `-D warnings` + fmt clean;
  `make test-fast` green (hooks ran on both commits).

**Remaining:** 2.8 deterministic gate (`relay/gate.rs` GateSpec/GateRunner/GitGateRunner, gate task minted on a
branched approval in a gated run, outcome write merged/rework/blocked, `retry|skip_gate` answers, tick wiring, twelve
finding classes via `dbd import` + `make test-template`), 2.9 handlers/routes/events/guidance v2, 2.10 live converge
(`dbd reconcile --allow-destructive`) + `make install-service` + smoke; then phases 3–5.

**Next command:** in `lanes/relay-queue`: finish the 2.8 red (`crates/senseid/src/relay/gate.rs` exists untracked with
its pins; add `pub mod gate;`, `TaskEvent::GateFailed`, store stubs + pins, the ticker `pass`, the SQL twelve-class pin),
`cargo test -p senseid gate -- --test-threads=1`.

**Open questions (user):** none blocking. Deviations recorded per task in `2-daemon.md`.

**Known broken:** none on the branch. `sensei_test` (Rust template) is stamped for THIS tree — rebuild with
`make test-template` from whichever worktree runs the Rust suite next (#142). Live daemon DB NOT converged (2.10).
