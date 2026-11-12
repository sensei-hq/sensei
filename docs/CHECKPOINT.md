# Checkpoint — 2026-11-12 (lanes/relay-queue on lane/relay-autonomous-queue, cut from develop @ 598045196)

**Slice:** Relay autonomous queue — phase 2 (daemon). Tasks 2.1–2.8 SHIPPED as red/green pairs; task 2.9
(handlers/routes/events/guidance v2) next. Plan: `docs/plan/2026-11-10-relay-autonomous-queue/2-daemon.md` (each
task has ticked steps + "Executed" deviations).

**Done:**
- Phase 1 (DB additive) on develop @ `b9eb57ac2`; phase-1 checkpoint docs @ `598045196`.
- 2.1 DDL cutover `75bb808c9`+`ddaf9be49`; 2.2 vocabulary/state machine/props/errors; 2.3 plan parse
  (`relay/plan.rs`); 2.4 claims/release/checkpoint/limit_resume; 2.5 runs/Task 0/unpack/complete
  (`df4fb2787`+`99cb93dfb`); 2.6 store cutover `d0db800bd`+`f1a50fa39` (deps gate, F01/F03/F04/F05/F08/F10,
  nothing-to-merge → done, #402 join-active-crew, `RegisterMode::{Mint,Join,JoinActive}`); 2.7 sweeper
  `d5f9c1952`+`220f216a0` (pause end at resumes_at, abandoned review claims after 60 s, waiter eviction, nudges per
  900 s window, ready broker-task selection, `LIMIT_PAUSE_CLEAR`); 2.8 gate `b2aae4702`+`537a5b814`
  (`relay/gate.rs` GateSpec/GateRunner/GitGateRunner over `agent_spawn::run_agent`, gate task minted on a branched
  approval in a gated run, outcome write merged/rework/blocked + `retry|skip_gate`, ticker `pass`, twelve finding
  classes seeded — `make test-template` rebuilt).
- Proof at `537a5b814`: `cargo test -p senseid relay -- --test-threads=1` 357/357; `database/tests/run.sh relay` 14/14;
  clippy `-D warnings` + fmt clean; `make test-fast` green (the fabrication checker refused a log-and-continue arm in
  the pass once; errors now propagate).

**Remaining:** 2.9 handlers/routes/events (claim_next, claim, checkpoint, release, limit_resume, GET /api/relay/queue,
POST /relay/runs 201, RegisterBody role optional + suggested_name, WaitQuery/TasksQuery deny_unknown_fields +
timeout_seconds, waiter row in wait_for_task, `relay_run(change, run_id, crew_id)` + Created/Activated, guidance v2),
2.10 live converge (`dbd reconcile --allow-destructive`, `dbd import`) + `make install-service` + smoke; then phases 3–5.

**Next command:** in `lanes/relay-queue`: write the 2.9 reds in `crates/senseid/src/api/handlers/relay.rs` (handler
pins), `crates/senseid/src/api/events.rs` (run event shape), `crates/senseid/src/relay/mod.rs` (guidance v2 pins),
`cargo test -p senseid -- --test-threads=1 claim_next checkpoint release limit_resume queue create_run guidance`.

**Open questions (user):** none blocking. Deviations recorded per task in `2-daemon.md`.

**Known broken:** none on the branch. `sensei_test` (Rust template) is stamped for THIS tree — rebuild with
`make test-template` from whichever worktree runs the Rust suite next (#142). Live daemon DB NOT converged (2.10).
