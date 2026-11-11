# Relay Autonomous Queue & Adversarial Planning — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace coordinator-push with a self-service pull queue (FCFS claims, floating peer reviews, claim-early DAGs, in-turn checkpoints), add sprint runs with an adversarially reviewed planning task, and close the twelve broker findings F01–F12 — in the order **db → daemon → mcp → app**, each phase a green, committed checkpoint.

**Design:** `docs/design/relay-autonomous-queue-and-planning.md` (draft, 2026-11-10). This plan is grounded against `develop` @ `58b66e9fa` in `lanes/merge`; where the design's claims about existing code were wrong, the plan follows the code (see *Grounding corrections*).

**Architecture:** The relay schema (`database/ddl/*/relay/`) gains three tables (`runs`, `run_items`, `lane_waiters`), two enums, task columns for runs/review claims/checkpoints/broker tasks, and a read-only `task_queue` view. The daemon (`crates/senseid/src/relay/`, `db/pg_store/relay.rs`, `api/handlers/relay.rs`, `tasks/relay_lease_sweep.rs`) gains atomic `claim_next`/`claim`/`claim_review`/`release`/`checkpoint`/`limit_resume` and the person's `POST /relay/runs`; reads stop mutating; reviews float; the sweeper heals abandoned claims, re-offers to the pool, ends quota pauses, evicts dead waiters, unpacks approved plans, runs the deterministic merge gate, and completes runs. MCP mirrors the new lane calls. The app gains Create Sprint, the sprint band, floating reviews and reviewing-lane indicators over the existing SSE seam.

**Tech stack:** PostgreSQL DDL via dbd 0.27.9 (pre-release project → `dbd reconcile`), SQL tests under `database/tests/relay/` (run.sh, rolled-back transactions), Rust (axum + sqlx, `PgStore` tests cloning `sensei_test`), MCP tool table in `crates/mcp/src/lib.rs`, SvelteKit + Svelte 5 + rokkit in `app/`, vitest + Playwright e2e.

---

## Phase files

| Phase | File | Checkpoint |
|---|---|---|
| 1 · DB (additive) | [`1-db.md`](1-db.md) | DDL + SQL tests green on a fresh lane DB, `make test-template` rebuilt, daemon relay suite still green on the new schema, one commit |
| 2 · Daemon (incl. DDL cutover) | [`2-daemon.md`](2-daemon.md) | New lane/person routes + sweeper + gate; cutover DDL; all reds green; live DB converged; `make install-service`; two-lane smoke loop |
| 3 · MCP | [`3-mcp.md`](3-mcp.md) | Seven new tools + register changes + guidance v2; tool-list pins green; `make install-service` |
| 4 · App | [`4-app.md`](4-app.md) | Create Sprint, sprint band, floating reviews, reviewing lanes; specs + e2e pins green; `make install-app` |
| 5 · Docs, issues, release | [below](#phase-5--docs-issue-triage-release) | Docs synced, superseded issues closed, release checklist run |

Phases 2–4 are written at task level with the core code and test names; expand each to step level (red → run → green → run → commit) when the phase starts. Phase 1 is at step level and executes now.

---

## Estimate

Assumes one engineer working the TDD discipline this repo enforces (red first, SQL + Rust + vitest pins, review gates), on the current codebase. Ranges are working days.

| Phase | Scope | Estimate |
|---|---|---|
| 1 · DB additive | 2 enums, 3 tables, 6 task/lane columns, `task_queue` view, `now` sections, policies, 7 SQL test files updated/added, cutover reds pending, template rebuild | 0.5–1 |
| 2 · Daemon core | DDL cutover + rename blast radius (49 refs), state machine, `claim_next`/`claim`/`claim_review`/`release`/`checkpoint`/`limit_resume`, run create + Task 0, plan parse/validate/unpack, report dependency gate, floating reviews, envelope read-only, heartbeat pause fix, reregister re-attach, waiters + nudges, sweeper (abandoned claims, re-offer to pool, pause expiry, waiter eviction, run completion), F04/F08/F09/F11 fixes, register defaults + active-crew join, SSE events, store + handler tests | 5–7 |
| 2 · Deterministic gate | broker-executed tasks (`executor='broker'`), clean-merge check via `git merge-tree`, verification command in a scratch worktree, auto-merge into the base checkout, `build_failure`/`merge_conflict` findings | 2 |
| 2 · Host health, janitor, shared build (§10.1–10.3) | disk/swap sentinel with claim throttling (503), cleanup cascade, `/resources/execute` | 3–4 (separable; schedule after phase 4 if the queue must ship first) |
| 3 · MCP | 7 tools, register args, guidance v2, dispatch + list pins | 0.5–1 |
| 4 · App | types, state, Create Sprint form with backlog multi-select, sprint band, floating review rows, reviewing indicator, timeline labels, fixtures, component/state specs, 2 e2e no-refresh pins | 2–3 |
| 5 · Docs, issue triage, release | PRD + lanes-data + architecture + journeys + features docs, CHANGELOG, 6 issue closures with notes, release checklist, live verification | 1 |
| **Total (queue + gate, without §10.1–10.3)** | | **9.5–14 days** |
| **Total with §10.1–10.3** | | **12.5–18 days** |

With three lanes in parallel after phase 1 (daemon; mcp + docs; app against fixtures), wall time is bounded by the daemon phase: roughly 7–9 working days.

---

## Grounding corrections (design vs. code on `develop`)

The design's §1.2 findings were checked against the live code. The plan targets what exists:

1. **F03 is narrower than stated.** `relay_heartbeat` clears a pause only when `limit_resumes_at is null or limit_resumes_at <= now` (`pg_store/relay.rs:1221-1233`), not unconditionally. The thrash case is a pause with unknown resume time, cleared by the next routine beat. Fix stands: heartbeat never touches the pause; the sweeper clears it at `limit_resumes_at`; `POST /api/relay/limit_resume` clears it explicitly. `reregister` keeps clearing a stale pause (lanes.md §19, user ruling).
2. **F01 lives in the envelope read.** `relay_envelope` (`pg_store/relay.rs:3787-3815`) moves `assigned → working` and writes a `started` action on delivery. Both `GET /api/relay/tasks` and `GET /api/relay/tasks/wait` call it. The receipts it writes (`delivered`/`read`) are kept — they are delivery facts, not work state.
3. **F04 is real for `report_md`.** `check_summary` runs the leak guard over summaries, notes and message text; `report_md` is bound straight into `relay.actions` (`pg_store/relay.rs:2290`). Review finding texts are likewise unguarded.
4. **F12 already holds.** No task endpoint touches `leases.expires_at`; the phase-2 work adds a pin, nothing else.
5. **F09 is the props allowlist.** `validate_props` for `reported`/`reviewed` rejects unknown keys; a telemetry object carrying `family`/`version`/`model` is a 422 today. Fix: add the three keys to both allowlists.
6. **F10 is in `grant_next_waiter`** (`pg_store/relay.rs:719`); the liveness predicate is added there.
7. **Enum labels append, never insert.** The design's `add value 'ready' before 'assigned'` is replaced by appending (`ready` last in `task_state`, `planning` last in `task_phase`, the four new kinds last in `action_kind`). No code orders by these enums; the `relay_enums.sql` pins are updated to the appended order, so a converged database and a fresh one agree.
8. **`relay.queue` is named `relay.task_queue`.** The person route `GET /relay/queue` already means the items queue (triage + backlog); a view called `queue` would collide in every conversation about it.
9. **House-style enums instead of string-set CHECKs:** `relay.run_state` (`planning|active|completed|cancelled`) and `relay.task_executor` (`agent|broker`).
10. **`released` is a fourth new action kind.** A claim surrendered back to the pool is a hand-off; lanes.md §15 says every hand-off is a history row.
11. **Two-step DDL.** Phase 1 is additive only. `reviewer_lane_id`, `tasks_one_active_per_lane` (which counts `in_review` as lane-holding), `state default 'assigned'`, `assigned_at not null` and the lane section's `in_review` join all stay until the daemon switches (phase 2 task 2.1). Reason: the daemon reads `reviewer_lane_id` in 49 places and reads `assigned_at` as non-null; `relay_plan_item` writes `assigned` with a null lane, which the ready-coherence check would refuse. The cutover reds are committed in phase 1 as `relay_tasks_cutover.sql.pending` (run.sh skips `*.pending`).
12. **No hand-written migration script.** The project is pre-release (no `project.released`, no `snapshots/`): converge with `dbd reconcile` (additive now, `--allow-destructive` at cutover), rebuild the Rust template with `make test-template`, and let run.sh mint a fresh lane database from the tree. Index-predicate changes are the one thing reconcile is known not to converge (memory: dbd reconcile gaps); the cutover task names the manual `drop index` + `create index` for `tasks_one_active_per_lane` and `lanes_one_coordinator` on an already-converged database.
13. **A claim lands the task `working`.** There is no start tool; the envelope's delivery-starts-work is what F01 removes. `assigned` stays for the coordinator's `assign` path.
14. **Task 0 is claimable by any live lane.** The design's "planning capabilities" gate is not built in v1; the adversarial check is the review, which a different lane must claim (the no-self-review check).
15. **Auto-merge is a broker task.** Approving a task inserts a `executor='broker'` gate task (deps = the approved task); the sweeper claims and runs it (`git merge-tree` clean check → verification command in a scratch worktree → merge into the base checkout). Gate inputs ride `runs.config` (`base_checkout`, `base_branch`, `verify_cmd`, `auto_merge`), captured by the Create Sprint form. A failing gate files a `reviewed` action with class `build_failure` or `merge_conflict` and moves the task to `rework`.
16. **#405 fix folded into phase 1.** `lanes_one_coordinator` becomes `where role = 'coordinator' and unregistered_at is null` (the coordinator ruling on the issue). One line, one pin.

## Risks to carry

- **Upgrade path on other machines.** `sensei upgrade` deploys DDL with embedded dbd-core, which does not ALTER-add columns (memory: `dbd reconcile = incremental DDL`). Until the upgrade path runs a reconcile for `relay`, a machine upgrading from slice 1 keeps the old relay schema. Raise as its own item in phase 5 if not already tracked.
- **The dōjō deploys the relay schema too** (`design.yaml` dojo scope includes `relay` and lists its enums explicitly). Phase 1 adds `relay.run_state` and `relay.task_executor` to that list so `dojo-prod` deploys cleanly. The dōjō app's relay reads (`dojo/src/lib/relay-*.ts`) are untouched by this plan; `review_claimed_by` reaches them only when the dōjō relay screen is next worked.
- **Live crews during cutover.** The live DB holds 2 crews, 13 lanes, 23 tasks, 0 `in_review`. The cutover copies `reviewer_lane_id → review_claimed_by` for any `in_review` rows before dropping the column; with zero such rows today this is a no-op, but the step stays.

---

## Phase 5 — Docs, issue triage, release

### Task 5.1: Sync the four documentation surfaces

**Files:**
- Modify: `docs/prd/modules/03-delivery/21-relay/lanes.md` (§3 roles → coordinator optional; §5 flow → sprint run + Task 0; §6 queue → FCFS claims, floating reviews, claim-early; §7 reviewer pick → floating claim; §12 MCP surface; §13 gap 1 → waiters table; §15 nudges → floating-review nudge; §19 → `limit_resume`)
- Modify: `docs/prd/modules/03-delivery/21-relay/lanes-data.md` (tables 7 → 10; `tasks` columns; *Stages* gains `ready`; derived reads gain `task_queue` and the two `now` sections)
- Modify: `docs/architecture/relay.md`, `docs/features/06-relay.md`, `docs/journeys/relay.md` (the lane loop is claim_next → work → checkpoint → report → float; the person's loop is register lanes → Create Sprint → watch)
- Modify: `docs/design/relay-autonomous-queue-and-planning.md` frontmatter `status: draft` → `status: implemented` plus a "Deviations" section pointing at this README's *Grounding corrections*
- Modify: `CHANGELOG.md`

- [ ] **Step 1:** Edit each file; every claim names the route or function it describes.
- [ ] **Step 2:** `make check-doc-links` → PASS.
- [ ] **Step 3:** Commit: `docs(relay): autonomous queue — PRD, data model, architecture, journeys synced with the shipped surface`.

### Task 5.2: Close the superseded issues

Run after phases 2–4 are installed and the smoke loop passed. Each closure comment cites the design file, this plan, and the commit range.

| Issue | Verdict | Comment to post |
|---|---|---|
| #403 coordinator cannot re-pick a reviewer | **Superseded** | Reviews no longer belong to a lane: a report floats on the review queue, any non-author lane claims it (`claim_review`/`claim_next`), a claimant can `release` it, and the sweeper clears a claim whose lane lease lapsed 60 s ago. The re-pick endpoint is unnecessary. |
| #401 parked/waiting tasks should free the lane | **Superseded** | `in_review` no longer holds the lane (`tasks_one_active_worker_per_lane` excludes it): a lane that reported can claim a review or a ready task at once. Claim-early covers "start the next thing while waiting". `blocked` still holds the lane by design (the ask resumes where it was). The `parked` state is not built. |
| #405 departed coordinator holds `lanes_one_coordinator` | **Fixed** (phase 1) | Index is partial on `unregistered_at is null`; pin in `relay_crews_lanes.sql`. |
| #402 register should join the active crew without `crew_id` | **Fixed** (phase 2, task 2.9) | `crew_id` optional; zero active crews → 409 `relay_no_active_crew`; more than one → 409 `relay_crew_ambiguous`. |
| #404 `tasks/wait` ignores `timeout_seconds` | **Fixed** (phase 2, task 2.9) | `timeout_seconds` accepted (`timeout_ms` wins when both), `WaitQuery` and `TasksQuery` are `deny_unknown_fields` (F11), guidance names the param. |
| #400 register guidance omits the limit-exhaustion flow | **Fixed** (phase 3) by guidance v2 — unless `lane/400-limit-flow-guidance` merges first, in which case rebase guidance v2 on it. |
| #399 document text caps | **Already merged** (`217a56643` on develop) — close with the commit. |
| #415 one-command crew join | **Narrowed**, stays open: the register response now answers the whole loop (guidance v2); the `/sensei:register` slash command remains. Retitle to the slash-command remainder. |
| #398 crew with dead lanes can never close | **Stays open.** Runs complete themselves; crews still close by the existing route. Note on the issue that a coordinator is no longer required, so the fix is "close tolerates dead-lease lanes". |
| #396 crew management | **Stays open**; note that the roster now shows `suggested_name` and that register defaults to contributor. |
| #393 archive completed runs | **Stays open**; note that `relay.runs` now exists and is the archival unit. |
| #389, #414, #378, #387, #382, #385, #386, #388, #380, #381, #383, #384, #379, #395, #312, #314, #299 | **Unaffected**; no comment. |

- [ ] **Step 1:** For each *Superseded* / *Fixed* / *Already merged* row: `gh issue close <n> --comment "<comment>"`.
- [ ] **Step 2:** For each *Narrowed* / *Stays open* row with a note: `gh issue comment <n> --body "<note>"`; retitle #415 with `gh issue edit 415 --title "Relay: /sensei:register slash command (register already answers the full lane loop)"`.
- [ ] **Step 3:** Record the closures in `docs/backlog.md` under the relay entry.

### Task 5.3: Release checklist

Follow `~/.claude/CLAUDE.md` → *Release Checklist* (full suite green; clippy + fmt clean; CHANGELOG; docs/skills/agents/website synced; shipped artifact reproduces the smoke loop; merge to main, CI green). The `release` skill runs it.
