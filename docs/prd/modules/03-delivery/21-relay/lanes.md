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

- **Coordinator** (at most one per crew, optional since the queue cutover of 2026-11-12). Mints the crew
  (`POST /api/relay/register` with `role: coordinator` and no `crew_id`), plans items into the pool
  (`POST /api/relay/tasks/plan`), may still `assign`, `cancel`, `handover` and `merge` by hand, and manages
  resources. It never reviews a task it authored. A crew of contributors alone runs on the queue: a sprint run
  (`POST /relay/runs`) plans itself through Task 0.
- **Contributor** (the default role: `register` without `role` is a contributor, and without `crew_id` it joins the
  one active crew — 409 `relay_no_active_crew` / `relay_crew_ambiguous` otherwise). Claims ready tasks and floating
  reviews from the crew's queue (`POST /api/relay/tasks/claim_next`), works, checkpoints, reports, and reviews what
  it did not author.
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
6. **Queue it** (queue cutover, 2026-11-12). A planned task waits `ready` in the pool with no lane; the next live
   lane takes it with `claim_next` (`relay_claim_task` for a named one). A **sprint run** does steps 1–5 for the
   person: `POST /relay/runs` groups backlog items under a goal and mints **Task 0** (phase `planning`, ready in the
   pool); the lane that claims it writes the plan as a ```json block in `report_md` (`tasks[{key, title, phase,
   owned_paths, acceptance, deps}]`), a peer claims the review, and the approval **unpacks** the plan into ready
   tasks in topological order (`relay_unpack_plan`) and turns the run `active`; a plan that cannot unpack is refused
   (409 `relay_plan_invalid`) and the review stays open for a `changes_requested`. The run completes itself when
   every task is done or cancelled (the sweeper's pass, `run.completed`).
7. **Assign** (the manual path, coordinator only). The task goes to a live lane whose capabilities fit, so work stays even across lanes.
   Live means holding a granted, unexpired lease: assigning onto a dead lease is refused (409),
   because nothing would ever recover the task — the sweep only interrupts leases it sees expire,
   and cancel refuses while the holder is still registered while re-assign refuses a held task.
   Wake the lane first, then assign. Crews are isolated at assign: a lane or task from another
   crew reads as not found, and a lane's role never changes — swapping roles means registering
   a new lane.

## 6. The task queue

- **States** (`relay.task_state`): `ready` (the pool: minted, unclaimed, on no lane) → `working` (a claim lands
  working at once — there is no start tool; an `assigned` task starts on its holder's first write, F01) →
  `in_review` (the report **floats** on the review queue and frees the lane) → `rework` → `in_review` … →
  `approved` → `done`; plus `blocked`, `interrupted` (lease lost; re-offered to the pool as `ready` after the grace)
  and `cancelled`. `tasks_lane_coherence` keeps the shape honest: `ready` has no lane, a laneless task is `ready`,
  `cancelled` or the broker's.
- **FCFS claims** (`POST /api/relay/tasks/claim_next`, MCP `relay_claim_next`; one transaction, `for update skip
  locked`): a **floating review** first (never the lane's own report), then a ready task whose dependencies are
  done, then a ready task the lane may **claim early** (`deps_status: early_claim_gated`, `unmet_deps` listed) — its
  full report is refused with 409 `relay_upstream_deps_unmet` until every dependency is done. A named task or
  review goes through `/{id}/claim` and `/{id}/claim_review`; `/{id}/release` gives a claim back (a task returns to
  the pool with its checkpoint, a review floats again). A lane holds one worker task
  (`tasks_one_active_worker_per_lane`) and one review claim (`tasks_one_active_review_per_lane`) at a time, and
  reviews beside its own work.
- **Reading never starts work.** `GET /api/relay/tasks` / `tasks/wait` answer the envelope (messages, answers, the
  review you claimed, the task you hold); `GET /api/relay/queue` reads the crew's claimable work off
  `relay.task_queue` (`review_floating` · `review_claimed` · `ready_unblocked` · `ready_claim_early`).
- **In-turn saves.** `POST /api/relay/tasks/{id}/checkpoint` merges `current_step`, `notes` and checklist `steps`
  into the task without moving it; the checkpoint survives interrupt, re-offer and reregister (`resumed`).
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
- **The review floats; the gate merges** (queue cutover). A full report clears any previous claim and floats the
  task on the review queue; the lane holding the claim (`review_claimed_by`) is the only one that may file the
  verdict, and the sweeper releases a claim whose lane lease lapsed 60 s ago or whose lane left
  (`released {kind: review, reason: "lease lapsed"}`). An approval whose latest report names a `branch`, in a run
  whose config names `base_checkout` and `base_branch` (and does not set `auto_merge: false`), mints a **broker gate
  task** (`executor: broker`, `deterministic: {action: gate, …}`): the sweeper hands it to `GitGateRunner`, which
  refuses a dirty or wrong-branch base, probes with `git merge-tree --write-tree`, runs `verify_cmd` in a throwaway
  worktree with the base merged in, then merges `--no-ff`. Merged → the task is `done` with a `merged` row; a
  conflict or a failed verify → `rework` with one high finding of class `merge_conflict` / `build_failure` (the
  tail in the review's `body_md`); the gate's own failure blocks the gate task behind an ask (`retry` |
  `skip_gate`). With nothing to merge (no branch) an approval ends the task `done` at once; outside a gated run it
  stays `approved` for the coordinator's `merge`.
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

## 12. MCP surface (`sensei-mcp`, `relay_*`)

`relay_register` (role and crew_id optional; `suggested_name`) · `relay_heartbeat` · `relay_reregister` ·
`relay_unregister` · `relay_get_tasks` · `relay_wait_for_task` (`timeout_ms` or `timeout_seconds`) ·
`relay_get_queue` · `relay_claim_next` · `relay_claim_task` · `relay_claim_review` · `relay_checkpoint_task` ·
`relay_release_task` · `relay_report_task` · `relay_submit_review` · `relay_ask_decision` ·
`relay_answer_decision` · `relay_send_message` · `relay_limit_pause` · `relay_limit_resume` ·
`relay_request_resource` · `relay_release_resource` · `relay_plan_item`, `relay_assign_task`,
`relay_merge_task`, `relay_cancel_task`, `relay_handover_task`, `relay_close_crew` (coordinator) ·
`relay_post_step`. A sprint run is the person's (`POST /relay/runs`), not a lane tool. The guidance `register`
answers (v2) spells the loop out.
The existing `start_run` / `register_plan` / `update_task_status` / `report_run_outcome` /
`run_status` / `pause_run` are reused where they already fit, and not cloned.

## 13. Gaps and open questions

1. **Pull, with a bounded long-poll (user, 2026-11-05) — built.** `GET /api/relay/tasks/wait` holds up to 50 s
   (`timeout_ms`, or `timeout_seconds` when it is absent; unknown query keys are refused, F11) and answers at once
   on any relay event; the hold is a row in `relay.lane_waiters` for its duration (the sweeper nudges by
   `waiting_since` and evicts dead lanes' rows, F06). The rest of the note is the reasoning that led there.
   Not every assistant gives MCP a push hook, so lanes pull.
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
- **Nudge (as built, F07).** A review floating past 900 s with nobody claiming it gets one `message` to
  everyone in its crew per window — `props: {nudge: true, reason: review_floating, waiting_min, task_id}`, no lane
  (the daemon speaks) — on the sweeper's pass, delivered on every lane's next `wait_for_task` / `get_tasks`. There
  is no party to wait on any more: any non-author lane claims the review (`claim_next` offers it first). The person
  may still message the crew at any time.
- **The person's nudge (as built, #419).** `POST /relay/messages` with `nudge: true`, a `reason` (`review_unassigned`
  · `verdict_unread` · `merge_pending` · `no_work` · `decision` · `resource` · `review_floating`) and optionally
  `waiting_min` writes the message with those props. A reason or a wait without `nudge: true` is refused (422), and so
  is a nudge to the person. The task's history shows the flag, the reason and the wait on the message row.
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

## 18. Step progress (F8; run-02 round-1 verdict)

Steps lived in transcripts and the broker never received them, so no screen could show progress. The holder
narrates its turn step by step: `POST /api/relay/tasks/{id}/steps` with `{key, title, state}` (holder only;
MCP `relay_post_step`), working/rework turns only. Each step carries a key stable within the task (upserts by
it), a title, and a state: `open` advances to `done`, or to `skipped` with a reason (required on skipped,
forbidden elsewhere). Done/skipped never reopen — re-posting the same state is a no-op, and a new turn posts a
new key. The broker stores the flat checklist on the task in insertion order and serves it on the task view;
the screen renders it as a checklist (completed checked, the first open step highlighted as current). Flat only:
no dependencies, no graphs, no cross-task aggregation.

## 19. Limit exhaustion (user, 2026-11-09)

Assistants hit daily/weekly usage limits, usually visible only as transcript warnings with a resume time. A lane
that goes quiet on quota used to read exactly like a dead lane. Now the lane reports it:
`POST /api/relay/limit_pause` with `scope` (`daily` | `weekly` | `other`), `resumes_at` (the warning's reset
time; null when unknown), and an optional note (MCP `relay_limit_pause`). Re-pausing overwrites.

The pause is recorded on the lane and is **distinct from lease expiry**: the lease may stay live (the heartbeat
loop is not quota work) while the lane cannot work. The sweep suspends a paused lane's held task for re-offer
elsewhere on the normal grace **without** expiring the lane or counting it failed. A beat **never** ends the pause
(F03, queue cutover): the sweeper ends it at `resumes_at` (`lane.resumed` with reason `limit_pause_ended`), the lane
ends it early with `POST /api/relay/limit_resume` (MCP `relay_limit_resume`; an unknown resume time ends only this
way), and leaving and resuming clears any stale pause.

The screen reads a paused lane as paused-with-resume-time, never "lease lost": the lanes list shows the scope
plus `until HH:MM` (or `resume unknown`), and the task view's facts line carries the same. A dead lease still
reads lease-lost even with a stale pause on the row. Transcript mining for limit warnings stays out of scope:
the explicit report path works for every assistant, and an adapter may automate it later.

## 20. Local identity (user, 2026-11-09)

Local relay works always; sign-in is only needed when the user wants relay on the dōjō. The daemon user
behind register and the person routes is a STABLE machine-local id (`relay.daemon_user`, minted once, never
changes) — never the dōjō principal, with no precedence and no fallback: linked or not, relay rows are keyed
by the local id on every machine. This supersedes the Q19 fail-closed ruling, which 401'd
(`relay_signin_required`) without a linked default persona — a gate a GitHub-signed-in machine couldn't satisfy,
since sign-in sets no linked default. The `relay_signin_required` code stays for the dōjō-sync operations that
will need it; relay routes never raise it. Sync maps local→principal at the boundary and never re-keys relay
rows, so there is no adoption migration. (The enrollment fix — sign-in producing a linked default persona for
the dōjō side — is a separate squash goal.)
