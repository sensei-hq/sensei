# Relay lanes · data model (proposal r2 for discussion, 2026-11-05)

**For:** [lanes.md](lanes.md). **Screen and interaction contract:** `coord/mockups/relay-lanes/CONTRACT.md` (Design r1, with
the "Message the crew" addendum, settled with the user). Relay DDL merged; the daemon store and screens are under construction (slice 1).

**r2 (user, 2026-11-05):**
- Reports, reviews, findings, decisions and messages are all **actions on a task**: one table, categorised by kind.
- What is shared is **Markdown**; the numbers and structure behind it are **props**, which feed metrics.
- Items **reference the tracker's tickets** rather than copying them.
- One leases table.

## One `relay` schema, deployed to both the daemon and the dōjō (user, 2026-11-05)
The same DDL runs on the desktop's sensei DB and on the dōjō, and the daemon syncs rows across (D89 targets, per-target
marks). For that to work without breaking:
- **Every table carries `user_id`**: the machine-local daemon user (`relay.daemon_user`), stable per machine, set on the
  desktop when the crew is created — never the dōjō principal (local identity, user ruling 2026-11-09; Q19 superseded).
  Sync maps local→principal at the dōjō boundary, so on the dōjō the mapped principal is the RLS key (a row is readable
  by its owner, and by others only by an explicit share). On the desktop it qualifies rows when more than one local
  identity runs crews. Every unique constraint and every lookup includes it. Relay rows are never re-keyed: there is no
  adoption migration.
- **Ids are minted on the desktop and are stable everywhere.** `uuid` v7 (time-ordered) for every table, `actions`
  included (no `bigserial`), so a row has the same id on both sides and an upsert on `(user_id, id)` is idempotent.
- **The crew lives in `relay.crews`, not `activity.runs`.** `activity` is desktop-only, so a shared schema can't point at it.
  `relay.crews (user_id, id, workspace_id null, repo_key null, goal, status, heartbeat_at, started_at, completed_at)`. A crew
  may name no repository (Q6): every **task** carries its own, so one crew can span repos, and a crew whose `repo_key` is null
  reads "no repository" on screen (Q18). The daemon may link a crew to a drive run later; nothing depends on it.
- **No foreign key leaves the `relay` schema.** The ticket is referenced by its **portable identity**,
  `ticket_provider` + `ticket_key` (e.g. `jira` / `SEN-52`). Each side resolves it in its own tracker tables: the desktop in
  `sensei.tracker_tasks`, the dōjō in its read-only integration. Only that identity travels, never the ticket's content.
- **What syncs:**
  - `crews`, `lanes` (without `token_hash` / `checkpoint`), `items`, `tasks`, `actions`. An action's `report_md` goes only
    when sharing is on.
  - `leases` never leave the machine: they're local resources.
  - `daemon_user` never leaves the machine either: it is the local side of the sync boundary's local→principal map.
- **The dōjō is read-mostly.** The person can answer a decision and post a message there (`answered`, `message` actions).
  Those sync back to the desktop through the existing pull path, with the same ids, so neither side ever overwrites the
  other's rows.

## Repository required, workspace optional (user, 2026-11-05)
On this Mac, **67 of 157 repositories belong to no workspace**, and 5 belong to more than one. So the workspace can't be the
key, or work on those repositories would never appear:
- **Every task names its repository** (required): `repo_key`, the portable identity (`github.com/org/repo`), resolved on
  each side. No foreign key leaves `relay`.
- **The workspace is optional** on `crews`, `items` and `tasks` (`workspace_id` null). It's a tag and a filter, never a
  gate.
- **Scope is the crew and the user,** not the workspace. Reading a task is `(user_id, crew_id, task_id)`. Another user's
  task, or one from a crew you're not in, is not found.
- **The Relay screen lives in the main window** (all crews, filterable by repository or workspace). The workspace window
  shows the same screen pre-filtered to its workspace.

## Principles
- **Reuse.**
    - Tickets **are** `sensei.tracker_tasks` (the TG5 integration). Sprints come from `sensei.tracker_task_sprints`, and
    "an assistant fetched this ticket" from `sensei.session_task_links`.
  - Gates stay M21's.
- **One log for a crew: `relay.actions`.** Every hand-off, report, review, question, answer, message, merge and lease change
  is a row. It is the bus, the history and the audit trail. Drive runs keep `activity.run_events`; nothing is written twice.
- **Markdown for people, props for machines.** `body_md` is what a reader sees: a summary, a review, a question, a message.
  `props` (jsonb) holds structured facts: telemetry, files, gates, findings, options. Its keys are fixed per action kind
  and validated by the daemon, and metrics read them through views.
- **Nullable means "not reported" (D83).** A missing prop is absent, never 0. Totals say how many turns reported.

## Tables (seven: daemon_user, crews, lanes, leases, items, tasks, actions; all but `daemon_user` with `user_id`)

### `relay.daemon_user`: the machine-local daemon user (singleton)
`singleton` (boolean PK, fixed true — exactly one row) · `user_id` (uuid v7, minted once) · `minted_at`.
- The stable id behind register and the person routes, on every machine, signed in or not. Never the dōjō
  principal; persona/principal state never affects it (local identity, user ruling 2026-11-09; Q19 superseded).
- Never synced, never re-keyed (see *What syncs*).

### `relay.crews`
See the section above. `user_id` · `id` (uuid v7) · `workspace_id` null · `repo_key` null (**the founding repository**, Q6; tasks
carry their own, so a crew spans repos, and null is read as "no repository", Q18) · `goal` · `status` · `heartbeat_at` ·
`started_at` / `completed_at`.

### `relay.lanes`: who is registered
`user_id` · `id` uuid v7 · `crew_id` · `code` (`L1`…, unique per crew) · `title` null (the objective, e.g. "Diagrams") · `role`
(`'coordinator','contributor'`; one coordinator per crew) · `family` / `version` null / `model` null · `capabilities` text[]
· `repo`, `worktree` null (the broker creates it) · `token_hash` · `checkpoint` jsonb null (for `reregister`) ·
`registered_at` / `unregistered_at` null · `limit_paused_at` / `limit_scope` (`'daily','weekly','other'`) /
`limit_resumes_at` / `limit_note` (all null when working; paused ⟺ scope set).
Liveness is **not** a column here; it's the lane's lease (below). A limit pause is not liveness either: the
lane may hold a live lease while paused, and the sweep suspends its held task without expiring it.

### `relay.leases`: one table for every lease
`user_id` · `id` · `crew_id` · `lane_id` · `kind` (`'lane','resource'`) · `resource` text null (`rust-build`, `e2e`…; null for a lane
lease) · `state` (`'queued','granted','released','expired','refused'`) · `expires_at` null · `granted_at`,
`released_at` null · `props` jsonb (disk/swap at grant, refusal reason).
- A lane lease is renewed only by `heartbeat`. When it expires, the lane is suspended.
- A resource lease is the build slot: at most one granted per resource, machine-wide (partial unique).

### `relay.items`: the queue (triage and backlog)
`user_id` · `id` · `workspace_id` null · `repo_key` null (a manual item may not name one yet) · `ticket_provider` / `ticket_key` null (**the ticket's portable identity, resolved on each side; never its content**) · `title`, `kind`
(`'bug','feature','gap','issue','finding'`), `tags` text[] (**only for manual items**; a ticketed item reads its title, type
and sprint from the tracker) · `state` (`'backlog','planned','closed'`) · `position` int null (backlog order) ·
`added_by`, `added_at`.
- **Triage is a view, not rows:** tracker tasks in an active sprint (`tracker_task_sprints`, primary sprint) for the
  workspace's connected trackers that have no item yet, plus manual triage items (`state = 'triage'`, the one extra state).
- **`+` (add to backlog)** creates the item with `ticket_provider` / `ticket_key` set and `state = 'backlog'`.
- The ticket button is the tracker task's key and URL.

### `relay.tasks`: what a lane works on
`user_id` · `id` · `crew_id` · `item_id` null · `repo_key` (**required**) · `workspace_id` null · `lane_id` null (**one active task per lane**, partial unique) · `reviewer_lane_id`
null (never the author's lane; checked) · `title` · `phase` (`'analysis','tests','impl','docs','mockup'`) · `state`
(`'assigned','working','in_review','rework','approved','done','blocked','interrupted','cancelled'`; see *Stages*)
· `round` · `owned_paths`, `acceptance`, `resources` text[] · `deps` uuid[] · `assigned_at`, `state_changed_at`,
`completed_at` null · `steps` jsonb (flat checklist, F8: `[{key, title, state, reason, updated_at}]`, insertion order).
- The ticket comes through `item_id → ticket_provider / ticket_key`.
- The **brief is the `assigned` action's `body_md`**, frozen. A change is a `brief_updated` action.
- A review is a stage of the task: the reviewer's queue is `reviewer_lane_id = me and state = 'in_review'`.

### Waiting on a task (lanes.md §15, user 2026-11-06)
Waiting is a record beside a task's state, not a seventh state: a task is `in_review` **and** waiting on a reviewer,
`approved` **and** waiting on a merge. §15 gives it `task.waiting {since, reason, waiting_on}`, closed on resume into
`task.waiting_ended {at, by}`; CONTRACT §3f-bis fixes the six reasons and who each one waits on. (`no_work` is the one
case with no task: a lane idle with nothing assigned, `task_id` null.)

| `reason` | opens when | `waiting_on` |
|---|---|---|
| `review_unassigned` | a report lands with no reviewer assigned | `{role: 'coordinator'}` |
| `merge_pending` | approved, not yet merged | `{role: 'coordinator'}` |
| `no_work` | a lane is idle with nothing assigned (`task_id` null: a lane fact) | `{role: 'coordinator'}` |
| `verdict_unread` | a verdict is filed and its author has not picked it up | `{lane_id}` (the author's, a uuid) |
| `decision` | an `asked` is open | `{person: true}` |
| `resource` | a resource lease is queued | `{broker: true}` |

- **The clock** starts at `since` and stops at `waiting_ended.at`; `by` names the action that closed it (`review.assigned`,
  `verdict.read`, `merge.done`, `task.assigned`, …), so "waited 47 min for a reviewer" stays a fact in the history instead
  of a recalculation.
- **One threshold per crew** (`wait_threshold_min`, default **15**, §15): at or past it (`>=`) the screen marks the wait and
  the broker sends the nudge itself (`auto_nudge`, on by default); the person may nudge at any time.
- **A nudge is a `message`**, not a new kind: `props.nudge = true` with `reason` and `waiting_min`, to the party being waited
  on, delivered first by that party's next `wait_for_task` / `get_tasks`. The coordinator's queue lists unreviewed reports
  before new assignments.
- **De-duplication:** a lane re-sending a status it already sent is a no-op — the broker drops it on
  `(lane, task, kind, round)`. "Standing by" is a `message` with `props.standing_by`, a history row like any other.

**Stored or derived — for L0 and L4, not decided here.** §15 says the task *carries* the record; it does not say where it
lives, and no `waiting_*` column (nor the crew's threshold) exists in the DDL today. Stored on `relay.tasks`, the screen
reads it without a join, but the row holds a second truth beside `state` / `state_changed_at`. Derived in `relay.now` from
state and the last action, nothing new to keep in sync, but `no_work` is a lane fact and `resource` a lease fact, so the
view also reads `lanes` and `leases`. A `waiting` / `waiting_ended` action pair puts it in `relay.actions`, where
the history already is, and the open record is "an unclosed `waiting` action". I do not choose: the DDL is L4's and the
model is L0's. Whichever lands, this file and `relay-api.ts` change in the same round, and no `waiting_*` column goes into
the DDL before the ruling.

### `relay.actions`: everything that happens
`user_id` · `id` uuid v7 · `crew_id` · `task_id` null (null for crew-level ones, e.g. a message "about no particular task") ·
`lane_id` null (who acted; null = the person) · `kind` (`action_kind`, below) · `round` null · `phase` null ·
`body_md` text null (the shared text, ≤ 600 chars for summaries, leak-guarded) · `report_md` text null (a full report;
shared to the dōjō only when sharing is on) · `props` jsonb · `reply_to` uuid null (an answer → its question; a
review → the report it reviews) · `created_at`.

| `kind` | `body_md` is | `props` keys |
|---|---|---|
| `assigned` | the hand-off brief | `docs[]`, `owned_paths[]`, `acceptance[]` |
| `brief_updated` | the new brief | `docs[]` |
| `started` | — | — |
| `reported` | the summary sent with the request for review | `look_at`, `flags`, `base`, `branch`, `commits[]`, `files[{path,added,removed,outside_owned}]`, `gates[{cmd,exit,counts}]`, `family`, `version`, `model`, `tokens_in`, `tokens_out`, `tokens_cache`, `started_at`, `ended_at`, `context_tokens`, `compactions`, `peak_memory_mb`, `subagents[{role,model}]`, `partial?` (bool; true = progress only, task keeps working, no reviewer picked) |
| `reviewed` | the review summary | `verdict` (`approved` / `changes_requested`), `findings[{severity,class,text,file,line}]`, `probes`, `subagents[]`, the same telemetry keys |
| `asked` | the question | `context` (one line: what is waiting on it), `options[{id,label,detail}]`, `recommended`, `blocks[task_id]`, `asked_of` (`person` / lane id) |
| `answered` | an optional note | `option`, `surface` (`desktop` / `dojo` / `lane`; `lane` is the daemon's on a lane-sent answer and is never sent, Q12) |
| `message` | the message | `to` = `{kind: 'coordinator' \| 'lane' \| 'everyone' \| 'person', lane_id?}` (a lane by its **uuid**; the screen shows its code; `person` is Q2's target, slice 2), `surface` (`desktop` / `dojo` / `lane`, same split), and for a nudge `nudge` (`true`) + `reason` + `waiting_min`, or `standing_by` (`true`) (§15) |
| `delivered` / `read` | — | none; `reply_to` = the message (receipts, one per recipient) |
| `merged` | the merge note | `commits[]`, `into`, `gates[]` |
| `interrupted` / `reoffered` | — | `checkpoint`, `to_lane` |
| `compacted` | — | `context_tokens_before`, `context_tokens_after` |
| `handed_over` | the handoff note (or the move itself when no note was sent) | `from_lane`, `from_state` (the state word the task left), `to_lane` (lanes by uuid; the screen shows codes) |

- **Decisions are actions** (`asked` and `answered`, linked by `reply_to`). An open decision is an `asked` with no answer,
  and its task reads "needs your answer" while it is open. The same applies to messages.
- **Finding classes:** a lookup table `relay.finding_classes (code, label, added_by)`, seeded from the README checklist. The
  daemon rejects an unknown class in `props.findings`. The learning loop adds classes.

## Stages of a task (user question, 2026-11-05)
`assigned → working → in_review → (rework → in_review)* → approved → done`
- **assigned:** handed to a lane. The brief is frozen.
- **working:** the lane has started. On `report`, it goes to in_review — unless the report is partial
  (`props.partial`), which records progress and keeps the task working with no reviewer picked.
- A held task (`working` / `in_review` / `rework`) can be **handed over** mid-turn: the coordinator pulls it off its
  holder onto another lane, where it lands `assigned` with the round bumped and the brief frozen from the prior brief
  plus a handoff note. `interrupted` stays the sweeper's re-offer job.
- **in_review:** a reviewer lane (never the author) holds it.
- **rework:** changes were requested. The round increments, and the next report returns it to in_review.
- **approved:** the review passed, waiting for the coordinator's merge with gates. This is its own stage because this week
  "approved, not yet merged" was a real wait.
- **done:** merged, or completed with nothing to merge (a mockup, an analysis).
- **Side states:**
  - **blocked**, on a decision or a dependency; it resumes where it was;
  - **interrupted**, when the lane's lease is lost; after the grace period it goes back to the backlog;
  - **cancelled**, by the coordinator, with a reason.
- **Phases ride inside one task:** `phase` goes `tests → impl` (or `analysis`, `mockup`, `docs`). Each phase runs its own
  work → review → rework loop, and the task's round counts per phase. Approval of the tests phase moves it to impl with
  state `working`. Approval of the last phase moves it to approved. So one ticket is one task, and "rounds by phase" is one
  query.

## Derived reads (security-invoker views; the screen never adds up numbers itself)
- `relay.now`: open `asked` actions for the person · the triage count by tracker and sprint · one row per lane with its
  active task and lease · the waiting tasks, oldest first, with who each waits on and for how long (the **Waiting** stat's
  filter, §15; this section lands with the waiting ruling) · today's completed tasks.
- `relay.task_history`: a task's actions, newest first, with props unpacked for the folds.
- `relay.lane_metrics`: per family/version/model: rounds, first-pass approval, finding classes (unnested from
  `reviewed.props.findings`), tokens, time working vs waiting for review, how accurate claims were.

## Integrity
- One coordinator per crew, one active task per lane, at most one granted lease per resource, no self-review.
- Every task state change and its action are written in **one transaction**.
- Lane tokens are hashed. No transcript text anywhere. `body_md` and `report_md` pass the leak guard before any push.

## Decided (user, 2026-11-05)
- Actions over separate report/review/finding/decision/message tables.
- Markdown for what is shared; props for what metrics read.
- Items reference tracker tickets (by provider + key); triage is a view over active sprints.
- One `relay` schema in both the daemon and the dōjō: `user_id` on every table, ids minted on the desktop (uuid v7),
  no foreign key outside `relay`.
- One `leases` table.
