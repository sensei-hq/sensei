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
  - It returns a **`guidance` block** (F9): `version` (schema, currently `"1"`) + `generic`, the
    self-sufficient join-time contract (300s lease/heartbeat cadence, poll paths, report → review → merge
    flow, re-offer/`wait` pickup, no-transcript rule) + `family_notes`, the override for the request's
    `family`/`model`. Four families have notes: `muse`, `claude` (prefix match, so `claude-code` counts),
    `opencode`, `freebuff`; the family wins, else the model id is matched the same way, else `family_notes`
    is null and the block is generic-only. MCP `relay_register` surfaces the block verbatim.
- **Leases, not just unregister.** A lane holds a lease renewed **only by `heartbeat`** (no other call touches
  it). When a lease expires (crash, restart, closed session) the lane is **suspended**, its in-flight task is
  marked `interrupted` with its last checkpoint, and after a grace period it goes back to the queue.
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
   Live means holding a granted, unexpired lease: assigning onto a dead lease is refused (409),
   because nothing would ever recover the task — the sweep only interrupts leases it sees expire,
   and cancel refuses while the holder is still registered while re-assign refuses a held task.
   Wake the lane first, then assign. Crews are isolated at assign: a lane or task from another
   crew reads as not found, and a lane's role never changes — swapping roles means registering
   a new lane.

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
- **Reviewer pick and re-pick.** `assign_task` may name a `reviewer_lane`; the name is recorded as
  the task's reviewer. The first `report_task` keeps a named reviewer as-is; with none named, the
  broker picks the least-loaded live lane that is not the author (ties to the smallest id, else the
  coordinator unless it is the author). Leases die after the pick: the lease sweep (60 s cadence)
  re-picks dead reviewers over the live lanes and clears the slot when nobody is live (Q17-REC).
  The re-pick is silent — no history row names it and the task view shows no reviewer — so the
  coordinator learns the reviewer from the report response, and after a re-pick only from the filed
  review itself (round-1 R3 healed L3→L4→L5 unheard and merged as "approved by L4" while L5 filed it).
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
- **One UI, two places (user, 2026-11-05).** The desktop app's **Relay screen replaces Intake**. It shows everything:
  the lane board (who, family/version/model, current task, state, lease), the task timeline (report → review →
  rulings → merge), the decision inbox, and per-workspace and per-sprint roll-ups. The **dōjō renders the same
  screen from the same components**. It receives task-level outlines (titles, states, rounds, verdicts, timings,
  telemetry numbers) by default, and **report and review bodies only when the person allows it** (daemon-controlled,
  per crew/workspace). Anything not exposed reads as "not shared", never as empty.

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
`release_resource` · `plan_item` (coordinator) · `assign_task` (coordinator) · `merge_task` (coordinator) ·
`handover_task` (coordinator, F1: pull a held task off its lane mid-turn).
The existing `start_run` / `register_plan` / `update_task_status` / `report_run_outcome` /
`run_status` / `pause_run` are reused where they already fit, and not cloned.

## 13. Gaps and open questions

1. **Pull, with a bounded long-poll (user, 2026-11-05).** Not every assistant gives MCP a push hook, so lanes pull.
   `wait_for_task(lane_token, timeout_s)` centralises the polling loop. The daemon holds the call open on its
   internal event bus and answers **at once** when work arrives (returning the work), or answers "nothing yet, call
   again" when `timeout_s` runs out. MCP tool calls are request/response, and clients time out a long call, by
   default and per client. So `timeout_s` is short (≤ 50 s by default) and the assistant simply calls again. Where a
   client resets its timeout on MCP progress notifications, the daemon may send keep-alive progress and allow a longer
   hold. SessionStart and prompt hooks (Claude Code) still inject the queue on start. A session that idles is told to
   loop on `wait_for_task`.
2. **Worktrees: the broker creates and assigns them** (user, 2026-11-05): one per lane, from the lane's repo plus
   branch, inside the disk floor.
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
5. Dogfood: run this repository's next item through it, with one coordinator and two contributors. First dogfood
   item (user, 2026-11-06): first-class Muse support in Sensei — an assistant adapter for Muse (MCP registration,
   sensei-skills install and peers of the existing adapters' duties), using the direct session connection where
   available, plus transcript scan and/or a background sensei↔Muse channel. Decomposed when slice 1 lands.

## 15. Waiting, nudges and the hand-off trail (user, 2026-11-06)
Today the person carries every hand-off between lanes by pasting messages. Relay must make each of those hand-offs a
row, and must make waiting visible.
- **Every hand-off is an action in the task's history.** A report, a review assigned, a verdict, a rebase note, a
  merge, and a "standing by" are all actions on the timeline (`reported`, `reviewed`, `message`, `merged` …). The person
  never relays a status line by hand. A lane re-sending a status it has already sent is a no-op: the broker
  de-duplicates on (lane, task, kind, round).
- **Waiting is a state with a clock.** Each task shows how long it has waited and on whom. The cases:
  - a report with no reviewer assigned;
  - a verdict the author has not picked up;
  - an approval not yet merged;
  - a lane idle with nothing assigned.
- **The waiting record.** A waiting task carries `task.waiting {since, reason, waiting_on}`, cleared into
  `task.waiting_ended` on resume. Reasons: `review_unassigned` / `merge_pending` / `no_work` → the coordinator;
  `verdict_unread` → the author's lane; `decision` → the person; `resource` → the broker. (From the approved
  addendum-2 mockup, gap 17.)
- **"Waiting" is a stat on the Relay screen** (ShellStat). Clicking it filters to the waiting tasks, oldest first.
- **Nudge.** The person, or the broker after a threshold (default 15 min, per crew), nudges the party being waited on:
  - the coordinator, for an unassigned report or an unmerged approval;
  - a lane, for an unread verdict.

  A nudge is a `message` action with `props.nudge = true`, delivered on the lane's next `wait_for_task`. The
  coordinator's queue lists unreviewed reports **before** new assignments.
- **Why (observed 2026-11-05/06):** four reports sat without a reviewer for 30 min to 3 h, because the coordinator
  scanned by time instead of matching reports to reviews. Two lanes idled on verdicts that had already been filed. One
  lane got a stale review of a round it had already replaced. Each was a lost hand-off that a row and a clock would have
  shown.
- **Coordinator hand-off.** The coordinator role must be transferable mid-crew: `reregister` as coordinator with the
  crew's checkpoint, for example when one assistant's quota runs out. Until Relay ships,
  `coord/HANDOFF-COORDINATOR.md` is the manual form.

## 16. Lane codenames (slice 2; user, 2026-11-06)
Lanes read better as names than as codes. The broker mints each lane a stable codename at register, in the
"colour + celestial" style the sessions already use (e.g. "Beige Aster", "Gold Antares"), unique per crew. Screens
show the codename as the primary human handle next to the objective title ("Beige Aster · Diagrams"); the code (L1)
stays for compact and technical contexts. Slice 1 ships code + objective title as contracted; codenames land as a
tests-first slice-2 item (model column + minting rule + uniqueness, in `lanes-data.md` and `relay-api.ts` then).
Handshake is negotiated, not assumed. At register the broker probes what the assistant family supports: the probe
itself, session-minted codenames, and direct session-to-session handshakes (as Muse sessions do today). Where an
assistant (freebuff, opencode, claude, cowork, chatgpt, …) lacks direct-handshake support, traffic falls back to the
Sensei-mediated channel — the daemon brokers, exactly as slice 1 does for every lane. The dogfood adapter (§14.5)
proves the direct path; the fallback keeps every other family working unchanged. Direct traffic stays visible: whenever
a direct handshake carries an interaction, a copy is fed into Sensei so the daemon tracks it, tagged with the mode of
communication (direct vs Sensei-mediated). The broker never goes blind on lanes that talk around it.

## 17. Mid-turn handover (F1; run-02 round-1 verdict)

Two gaps made mid-task handover impossible: every report moved to `in_review`, and `assign` 409'd from
`in_review` / `rework` / `interrupted`. F1 closes both halves.

**Partial reports.** `report` takes a partial marker. A partial report keeps the task `working` — it records a
`reported` action with `props.partial = true` and picks no reviewer. A later full report moves to `in_review`
normally. The history reads a partial as progress so far ("Partial · Work … rN", still working), never as a
request for review.

**Force-handover.** `POST /api/relay/tasks/{id}/handover` with `{to_lane}` (coordinator only; MCP
`relay_handover_task`) pulls a task off its holder from `working` / `in_review` / `rework`. Effects: the task lands
`assigned` on `to_lane`, the round bumps, a fresh `assigned` brief freezes the prior brief plus the handoff note, and
a `handed_over` event names both lanes. Guards, each a 409 with a specific error: the from-set is exactly the held
three (terminal tasks, `assigned` with nothing held, `blocked`, and `approved` all refuse; `interrupted` stays the
sweeper's re-offer job and says so); `to_lane` must hold a live lease in the same crew, must not be the current
holder, must not be the picked reviewer (a task never reviews itself), and must not already hold a task.
