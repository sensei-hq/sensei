# Checkpoint — 2026-11-10 (lanes/merge on develop)

**Slice:** Relay autonomous queue & adversarial planning — plan written, implementation not started.
Design: `docs/design/relay-autonomous-queue-and-planning.md`. Plan: `docs/plan/2026-11-10-relay-autonomous-queue/`
(README = estimate + grounding corrections + issue triage; `1-db.md` step-level; `2-daemon.md`, `3-mcp.md`,
`4-app.md` task-level).

**Done:**
- Design grounded against develop @ 58b66e9fa: F03 narrower than stated (heartbeat clears a pause only when
  `resumes_at` is null/past); F01 lives in `relay_envelope`; F04 real for `report_md`; F12 already holds; F09 is the
  props allowlist; F10 is `grant_next_waiter`.
- Phase order fixed: DB additive (green on the slice-1 daemon) → daemon incl. DDL cutover → MCP → app → docs/issues.
- Estimate: 9.5–14 days sequential without §10.1–10.3 host health/janitor/shared build; 12.5–18 with.

**Remaining:** every phase. Phase 1 is executable as written (DDL + SQL pins + `make test-template` + daemon suite
green + one commit); the live daemon DB is NOT converged until phase 2 task 2.10.

**Next command:** open `docs/plan/2026-11-10-relay-autonomous-queue/1-db.md` and run Task 1.1 step 1
(`cd database/tests && ./run.sh relay/relay_enums.sql` after editing the pins — expect red).

**Open questions (user):** none blocking. Deviations from the draft design are listed in the plan README
(`task_queue` view name, enums for run state/executor, `released` action kind, labels appended, claim lands `working`,
Task 0 claimable by any lane, auto-merge as a broker task, #405 fix folded into phase 1).

**Known broken:** none on develop.
