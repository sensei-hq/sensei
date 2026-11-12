# Checkpoint — 2026-11-12 (lanes/merge on develop)

**Slice:** Relay autonomous queue & adversarial planning — phase 1 (DB additive) SHIPPED; phase 2 (daemon) next.
Design: `docs/design/relay-autonomous-queue-and-planning.md`. Plan: `docs/plan/2026-11-10-relay-autonomous-queue/`.

**Done:**
- Phase 1 committed on develop @ `b9eb57ac2`: `run_state` + `task_executor` enums, six appended labels (`ready`,
  `planning`, `claimed`/`review_claimed`/`released`/`checkpointed`); `relay.runs`/`run_items`/`lane_waiters`;
  `tasks` gains `run_id`, `review_claimed_by/_at`, `executor`, `deterministic`, `checkpoint` + three indexes + two
  checks; `lanes` gains `suggested_name`, `role default 'contributor'`, the #405 partial index; `relay.task_queue`
  view; `relay.now` sections `sprint_summary` + `floating_review` and lane `suggested_name`/`reviewing`; owner-read
  policies; dōjō scope lists the two enums.
- Proof: 13/13 relay SQL files on a fresh `sensei_test_develop` (dbd deploy) and on a no-relay scratch DB; the four
  DDL lint gates + `dbd inspect` clean; `make test-template` rebuilt + stamped; `cargo test -p senseid relay`
  296/296; `make test-fast` green (one `check-no-fabrication` S1 hit fixed by `array(subquery)`).
- Cutover reds committed as `database/tests/relay/relay_tasks_cutover.sql.pending` (proved red by direct psql).
- Reconcile preview: all additive except the `lanes_one_coordinator` predicate — dbd plans DROP+CREATE, so the
  live converge at 2.10 needs `--allow-destructive`.

**Remaining:** phase 2 (daemon incl. DDL cutover, tasks 2.1–2.10), phase 3 (MCP), phase 4 (app), phase 5
(docs, issue triage, release).

**Next command:** `git worktree add ../relay-queue -b lane/relay-autonomous-queue develop`, then open
`docs/plan/2026-11-10-relay-autonomous-queue/2-daemon.md` task 2.1 — first red is renaming
`relay_tasks_cutover.sql.pending` → `.sql`.

**Open questions (user):** none blocking. Execution deviations are recorded at the end of `1-db.md`.

**Known broken:** none. The live daemon DB is NOT converged — by design, until phase 2 task 2.10.
