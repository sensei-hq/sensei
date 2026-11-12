# Checkpoint — 2026-11-12 (lanes/relay-queue on lane/relay-autonomous-queue, cut from develop @ 598045196)

**Slice:** Relay autonomous queue — phase 2 (daemon). Tasks 2.1–2.9 SHIPPED as red/green pairs; task 2.11 (now +
task view serve the sprint surface — added, phase 4 reads it) next, then 2.10 converge/install/smoke. Plan:
`docs/plan/2026-11-10-relay-autonomous-queue/2-daemon.md` (each task has ticked steps + "Executed" deviations).

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
- 2.9 routes `4d8bca384`+`bc583372f` (claim_next/claim/checkpoint/release/limit_resume, GET /api/relay/queue,
  POST /relay/runs 201, register role default + suggested_name, strict wait queries + timeout_seconds, waiter row,
  unpack + run.activated events, `relay_run(change, run_id, crew_id)`, guidance v2).
- Proof at `bc583372f`: `cargo test -p senseid relay -- --test-threads=1` 371/371; mcp green; clippy `-D warnings` +
  fmt clean; fabrication checker clean; `make test-fast` green.

**Remaining:** 2.11 now/task-view sprint surface (`now.runs`, `now.floating_reviews`, lane `reviewing` +
`suggested_name`; task header `run_id`/`deps_status`/`unmet_deps`), 2.10 live converge (`dbd reconcile
--allow-destructive` — dry-run read: 3 tables, 2 enums, tasks/lanes alters, reviewer_lane_id drop, plus two unrelated
`sensei.*` enum alters from develop drift; then `dbd import`) + `make crates && make install-service` +
`scripts/relay-queue-smoke.sh` (drafted, untracked); then phases 3–5.

**Next command:** in `lanes/relay-queue`: write the 2.11 reds in `crates/senseid/src/db/pg_store/relay.rs`
(`now_serves_runs_floating_reviews_and_the_reviewing_lane`, `task_view_serves_run_id_and_deps_status`),
`cargo test -p senseid -- --test-threads=1 now_serves task_view_serves`.

**Open questions (user):** none blocking. Deviations recorded per task in `2-daemon.md`.

**Known broken:** none on the branch. `sensei_test` (Rust template) is stamped for THIS tree — rebuild with
`make test-template` from whichever worktree runs the Rust suite next (#142). Live daemon DB NOT converged (2.10).
