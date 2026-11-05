# Relay · lanes — sensei brokers work between assistant sessions

**Status:** requirement (draft for decision, 2026-11-05). **Extends:** [Relay · M21](README.md).
**Origin:** the user, 2026-11-05, after two days of running five assistant lanes plus a coordinator by hand
through `~/Developer/sensei-hq/coord/` (reports, reviews, tracker, briefs). Every pain point below is one we hit.

## 1. The idea in one paragraph

A person opens a few assistant sessions (Claude Code, Muse, Freebuff, Cowork… in any mix) and tells each one
"register with sensei". One registers as **coordinator**, the rest as **contributors**. Each contributor
takes the first open **lane**. The coordinator turns a backlog item, ticket or spec into **ready tasks**: deep,
testable, with a written definition of done. It then puts each task on a lane. Contributors pull their
next task (`get_tasks`), do it, and file a **completion report** (Markdown, readable by people and assistants).
The coordinator, or another lane, reviews it, and the review lands back in the author's queue **ahead of new
work**. Sensei is the **broker**: every hand-off is an event on one bus, recorded in the daemon and pushed to
the dōjō. There, a person sees what every lane is doing, what it reported and what review it got, and how
well each assistant did. Two sessions are enough: one coordinator and one worker.

## 2. What it replaces (observed 2026-11-03 → 05)

| Today, by hand | Cost we paid | In relay lanes |
|---|---|---|
| The user copy-pastes one-line status between sessions | Lines didn't reach lanes, so lanes idled "waiting" for reviews that existed (L1, L4, L3, 11-04) | `get_tasks` returns the review; nobody relays it |
| The coordinator scans `reports/` against `reviews/` | A scan by mtime hid a report for 3 h | Every report is an event; the review task exists the moment the report does |
| Lane identity by worktree name (L9 = L4, L2 = L1) | Wrong-lane work, mismatched names | A lane is a registered record; the worktree is an attribute |
| Owned paths in a plan table | Grant requests by inbox file | Each task carries its paths; a write outside them is flagged at report time |
| Disk and memory managed by the coordinator by hand | 168 → 17 GB disk, swap 40/42 GB, a reviewer stopped mid-run | Resource leases: one heavy build machine-wide, scheduled by the broker |
| Review lessons in a README checklist | The same misses recurred in every lane | Findings are typed; the recurring ones become rules served to the next task (closes the learnings gap, D66) |
| Merge done by the coordinator after re-running gates | Correct, but manual | Merge is a coordinator task with its gates recorded |

## 3. Roles

- **Coordinator** (one per crew). Analyses and plans, runs the depth gate, assigns tasks, reviews (or assigns
  reviews), merges, and manages resources. It never implements a task it also reviews.
- **Contributor.** Implements tasks and may review tasks it didn't author.
- **Reviewer (spawned).** A coordinator may hand a review to a subagent it spawns. The review is still filed
  under the coordinator's lane, with the subagent recorded.
- **Person.** Opens sessions, answers **decisions**, approves outward or destructive actions, and watches.
- **Minimum crew.** Two (coordinator + one contributor). One session alone isn't a crew: self-review isn't
  allowed, so a solo session's work waits for a reviewer.

## 4. Registration (assistants come and go)

- `register(role, family, version, model, repo, worktree?, capabilities)`:
  - `family` is e.g. `claude-code`, `muse`, `freebuff`, `cowork`, `codex`.
  - `version` is the assistant's own version and is recorded on every execution, since versions change often.
  - `model` is the model id in use.
  - `capabilities` is e.g. `rust`, `sql`, `ts`, `mockups`, `can-push`, `can-run-hooks`.
  - It returns a **lane token** and the lane it was given: the first open lane whose requirements its
    capabilities meet.
- **Leases, not just unregister.** A lane holds a lease renewed by every MCP call (or a `heartbeat`). When a
  lease expires (crash, restart, closed session) the lane is **suspended**, its in-flight task is marked
  `interrupted` with its last checkpoint, and after a grace period it goes back to the queue.
- `re-register(lane_token)` resumes the same lane after a restart or a context clear, and gets the task's
  checkpoint back. `unregister` releases the lane cleanly.
- **Fewer workers → rebalance.** Tasks queued for a lane that's gone are re-offered to live lanes whose
  capabilities match. A task never moves while its lane is live.
- **Coordinator failover.** Coordinator state lives in the daemon (DB), not in a session. A new session can
  register as coordinator for the crew, and it resumes from the recorded state.

## 5. The coordinator's flow: nothing reaches a lane until it's airtight

1. **Intake.** It takes items from the backlog (grouped by feature/area), a ticket (TG5 `get_ticket`) or a
   spec. Each item carries **workspace, repository, sprint/epic, issue** tags.
2. **Analyse and clarify.** It does a depth assessment (the existing `sensei-plan-depth-reviewer` bar).
   Every ambiguity becomes either an answered assumption or a **decision task for the person**, with options
   and a recommendation.
3. **Design.** It sets out the candidate approaches and picks one with its reasons. The pick is recorded as a
   decision (D-number) when it binds other work.
4. **Break down.** The item becomes tasks with **definition of ready**: observable acceptance criteria, owned
   paths, dependencies (forward-only), risk tier, worker-tier hint, resource needs (e.g. a Rust build), and
   the test-design checklist that applies.
5. **Two phases per task**, as this week proved:
   - **tests** (red, disabled, each failing for its own reason) → **review** → **impl** → **review** → **merge**.
   - Each step is its own task with a dependency on the one before.
6. **Assign.** The task goes to a live lane whose capabilities fit, so work stays even across lanes.

## 6. The task queue

- **States:**
  - `backlog` → `ready` (passed the depth gate) → `assigned` → `in_progress` → `reported` → `in_review`;
  - then either `changes_requested`, which loops back to `in_progress`, or `approved` → `merged` / `done`;
  - plus `blocked` (on a decision or a dependency) and `interrupted` (lease lost).
- **`get_tasks(lane_token)`** returns the lane's queue in priority order:
  1. a **review result** on the lane's own work;
  2. a **review to perform**;
  3. a **decision answer** it was waiting for;
  4. new work.
- "Wait for confirmation": a lane with a task in review gets `waiting` with what it is waiting on. It may take
  an independent next task only if the coordinator marked one as safe to start.
- **Claiming is atomic.** Two sessions can't take the same lane or task.
- **Path ownership.** A task lists its paths. The report's diff is checked against them, and a write outside
  them is a finding, not a silent merge. A **grant** is a coordinator action recorded on the task.

## 7. The completion report and the review (Markdown, structured)

- **Report** (`report_task`): a Markdown body (today's `templates/report.md`) plus structured fields:
  - repository, branch, commits, base;
  - phase and round;
  - gates run, each with its command, exit code and counts;
  - deviations, questions and known-broken;
  - **telemetry:** family, version, model, tokens in/out (and cache), start and end, wall time, context size at
    report, context clears or compactions during the task, peak memory if the assistant can tell.

  Telemetry complements transcript scanning, and the two are reconciled (a mismatch is a finding).
- **Review** (`submit_review`):
  - verdict: approved / changes requested;
  - **typed findings**, each with severity, file:line and class (`vacuous-test`, `missing-failure-path`,
    `single-tenant-fixture`, `role-untested`, `aggregate-direction`, `secret-in-output`, `skipped-live-test`,
    `stale-claim`…);
  - mutation probes run, plus who reviewed and with which model.

  Rulings are separate events, so they're never lost in an appended file.
- **Decisions** (`ask_decision` / `answer_decision`): options, a recommendation, and who answered. A person
  answers on the desktop app or the dōjō (mobile relay).

## 8. Resources and safety (learned the hard way)

- **Resource leases.**
  - Heavy jobs (a Rust build or test run, a full e2e) take a machine-wide lease, one at a time by default.
  - The broker queues them and reports disk and swap with every grant, and refuses below set floors (e.g. 25 GB
    disk, 1 GB swap free).
  - Idle build caches are cleaned by policy.
- **Merges** are coordinator tasks. They re-run the gates on the merged tree and record them, and never merge a
  branch head that carries unreviewed commits; they take the reviewed commits only.
- **Gates (existing M21).** Outward or destructive actions (push to protected branches, publish, release,
  deploy, credential changes, contacting third parties) raise a relay gate for the person, as today.
- **Content.** Reports and reviews pass the leak guard before they reach the dōjō. Transcripts never leave the
  machine. Report bodies leave only when the person allows it (per crew / workspace).
- **No fabrication (D83).** A missing telemetry field is recorded as unknown, never as 0.

## 9. The bus, the daemon and the dōjō

- Every action is an **event** (`lane.registered`, `task.assigned`, `task.reported`, `review.submitted`,
  `decision.asked`, `resource.granted`, `merge.done`, `context.compacted`…). It's written once to the daemon
  and fanned out:
  - to waiting lanes (pull via MCP; push where the assistant supports hooks);
  - to the desktop app;
  - to the dōjō, per D89 target, with per-target marks.
- **Reuse, don't duplicate.** A crew is a relay run (`activity.runs`), tasks are segments, events are
  `activity.run_events`, gates stay M21's, and the dōjō side extends `dojo.relay_*`. New tables only where the
  run model has no equivalent (lanes and leases; typed findings).
- **Dōjō views.** A board of lanes (who, family/version/model, current task, state, lease); a task timeline
  (report → review → rulings → merge); the decision inbox; and per-workspace and per-sprint roll-ups.

## 10. Metrics (what this makes measurable)

- Per task: rounds to approval, by phase; time in each state; tokens; reviewer effort.
- Per lane or family/version/model: first-pass approval rate; finding classes per round; regressions
  introduced; how accurate its report claims are (claims the review contradicted).
- Per crew: throughput, review latency, idle time, resource wait.
- **Learning loop.** A finding class that recurs across lanes becomes a rule. The next task's brief carries the
  rules that apply, which is the manual checklist automated.

## 11. UX (to mock — Design lane, on the D87 shell)

- **Backlog:** grouped by feature/area, with each item's depth status (`needs analysis` / `needs decision` /
  `ready`).
- **Crew board:** lanes as columns, tasks as cards with state, round and lease; the review queue pinned first.
- **Task detail:** the report rendered, the review and typed findings, rulings, gates and telemetry.
- **Decision inbox:** desktop and dōjō.
- **Resources strip:** current lease, queue, disk and swap.
- **Assistant onboarding:** one line to paste into any session: "register with sensei as contributor".

## 12. MCP surface (sketch)

`register` · `heartbeat` · `reregister` · `unregister` · `get_tasks` · `claim_task` · `checkpoint_task` ·
`report_task` · `get_reviews` · `submit_review` · `ask_decision` · `answer_decision` · `request_resource` ·
`release_resource` · `plan_item` (coordinator) · `assign_task` (coordinator) · `merge_task` (coordinator).
The existing `start_run` / `register_plan` / `update_task_status` / `report_run_outcome` /
`run_status` / `pause_run` are reused where they already fit, and not cloned.

## 13. Gaps and open questions

1. **Push vs pull.** MCP is pull. Without push, a lane must poll, and idle sessions don't poll. So: SessionStart
   and prompt hooks inject the queue (Claude Code); a "check sensei" convention for the rest; the desktop app
   notifies the person. Whether to add a long-poll `wait_for_task`?
2. **Worktrees.** Does the broker create and assign worktrees (one per lane), or does the person? We recommend
   the broker, from the lane's repo plus branch, inside the disk floor.
3. **Base drift.** When develop moves, the broker tells affected lanes to rebase before they report.
4. **Trust.** Can a contributor's claimed telemetry be trusted? We reconcile with transcript scanning where
   possible; family/version are self-reported.
5. **Cost caps.** Per crew and per lane token budgets, with M21's limits (daily/weekly/monthly).
6. **Privacy in org dōjōs.** What a viewer may see of a lane's report body (D46/D86 access ladder).
7. **Relation to the Agentic Execution Team design** ([design/agentic-execution-team.md](../../../../design/agentic-execution-team.md)).
   That design has sensei *drive* model workers; this one has sensei *broker* between assistant sessions a
   person opened. We recommend building lanes first, because they work with today's assistants and no
   driver. The execution-team then becomes "the coordinator can also spawn a worker" later.

## 14. First slice (to build next, and use for the work after it)

1. Register, lease and re-register, with lanes and capabilities. Lane-scoped `get_tasks` with review-first
   priority. `report_task` / `submit_review` with the Markdown body plus telemetry fields.
2. Coordinator tools: `plan_item` (with the depth gate checklist recorded) and `assign_task`.
3. The event log in the daemon; a minimal desktop board (crew, lanes, queue). The dōjō push comes in slice 2.
4. A resource lease for heavy builds.
5. Dogfood: run this repository's next item through it, with one coordinator and two contributors.
