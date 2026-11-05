# Relay lanes · data model (proposal r2 for discussion, 2026-11-05)

**For:** [lanes.md](lanes.md). **Screen and interaction contract:** `coord/mockups/relay-lanes/CONTRACT.md` (Design r1, with
the "Message the crew" addendum, settled with the user). Nothing here is built yet.

**r2 (user, 2026-11-05):**
- Reports, reviews, findings, decisions and messages are all **actions on a task**: one table, categorised by kind.
- What is shared is **Markdown**; the numbers and structure behind it are **props**, which feed metrics.
- Items **reference the tracker's tickets** rather than copying them.
- One leases table.

## One `relay` schema, deployed to both the daemon and the dōjō (user, 2026-11-05)
The same DDL runs on the desktop's sensei DB and on the dōjō, and the daemon syncs rows across (D89 targets, per-target
marks). For that to work without breaking:
- **Every table carries `user_id`**: the owning person's dōjō principal id, set on the desktop when the crew is created. On
  the dōjō it is the RLS key (a row is readable by its owner, and by others only by an explicit share). On the desktop it
  qualifies rows when more than one signed-in identity runs crews. Every unique constraint and every lookup includes it.
- **Ids are minted on the desktop and are stable everywhere.** `uuid` v7 (time-ordered) for every table, `actions`
  included (no `bigserial`), so a row has the same id on both sides and an upsert on `(user_id, id)` is idempotent.
- **The crew lives in `relay.crews`, not `activity.runs`.** `activity` is desktop-only, so a shared schema can't point at it.
  `relay.crews (user_id, id, workspace_id, goal, status, heartbeat_at, started_at, completed_at)`. The daemon may link a crew
  to a drive run later; nothing depends on it.
- **No foreign key leaves the `relay` schema.** The ticket is referenced by its **portable identity**,
  `ticket_provider` + `ticket_key` (e.g. `jira` / `SEN-52`). Each side resolves it in its own tracker tables: the desktop in
  `sensei.tracker_tasks`, the dōjō in its read-only integration. Only that identity travels, never the ticket's content.
- **What syncs:**
  - `crews`, `lanes` (without `token_hash` / `checkpoint`), `items`, `tasks`, `actions`. An action's `report_md` goes only
    when sharing is on.
  - `leases` never leave the machine: they're local resources.
- **The dōjō is read-mostly.** The person can answer a decision and post a message there (`answered`, `message` actions).
  Those sync back to the desktop through the existing pull path, with the same ids, so neither side ever overwrites the
  other's rows.

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

## Tables (six: crews, lanes, leases, items, tasks, actions; all with `user_id`)

### `relay.crews`
See the section above. `user_id`, `id` (uuid v7) · `workspace_id` · `goal` · `status` · `heartbeat_at` · `started_at` /
`completed_at`.

### `relay.lanes`: who is registered
`user_id` · `id` uuid v7 · `crew_id` · `code` (`L1`…, unique per crew) · `title` null (the objective, e.g. "Diagrams") · `role`
(`'coordinator','contributor'`; one coordinator per crew) · `family` / `version` null / `model` null · `capabilities` text[]
· `repo`, `worktree` null (the broker creates it) · `token_hash` · `checkpoint` jsonb null (for `reregister`) ·
`registered_at` / `unregistered_at` null.
Liveness is **not** a column here; it's the lane's lease (below).

### `relay.leases`: one table for every lease
`user_id` · `id` · `crew_id` · `lane_id` · `kind` (`'lane','resource'`) · `resource` text null (`rust-build`, `e2e`…; null for a lane
lease) · `state` (`'queued','granted','released','expired','refused'`) · `expires_at` null · `granted_at`,
`released_at` null · `props` jsonb (disk/swap at grant, refusal reason).
- A lane lease is renewed by every MCP call. When it expires, the lane is suspended.
- A resource lease is the build slot: at most one granted per resource, machine-wide (partial unique).

### `relay.items`: the queue (triage and backlog)
`user_id` · `id` · `workspace_id` · `ticket_provider` / `ticket_key` null (**the ticket's portable identity, resolved on each side; never its content**) · `title`, `kind`
(`'bug','feature','gap','issue','finding'`), `tags` text[] (**only for manual items**; a ticketed item reads its title, type
and sprint from the tracker) · `state` (`'backlog','planned','closed'`) · `position` int null (backlog order) ·
`added_by`, `added_at`.
- **Triage is a view, not rows:** tracker tasks in an active sprint (`tracker_task_sprints`, primary sprint) for the
  workspace's connected trackers that have no item yet, plus manual triage items (`state = 'triage'`, the one extra state).
- **`+` (add to backlog)** creates the item with `ticket_provider` / `ticket_key` set and `state = 'backlog'`.
- The ticket button is the tracker task's key and URL.

### `relay.tasks`: what a lane works on
`user_id` · `id` · `crew_id` · `item_id` null · `lane_id` null (**one active task per lane**, partial unique) · `reviewer_lane_id`
null (never the author's lane; checked) · `title` · `phase` (`'analysis','tests','impl','docs','mockup'`) · `state`
(`'assigned','working','in_review','rework','approved','done','blocked','interrupted','cancelled'`; see *Stages*)
· `round` · `owned_paths`, `acceptance`, `resources` text[] · `deps` uuid[] · `assigned_at`, `state_changed_at`,
`completed_at` null.
- The ticket comes through `item_id → ticket_provider / ticket_key`.
- The **brief is the `assigned` action's `body_md`**, frozen. A change is a `brief_updated` action.
- A review is a stage of the task: the reviewer's queue is `reviewer_lane_id = me and state = 'in_review'`.

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
| `reported` | the summary sent with the request for review | `look_at`, `flags`, `base`, `branch`, `commits[]`, `files[{path,added,removed,outside_owned}]`, `gates[{cmd,exit,counts}]`, `family`, `version`, `model`, `tokens_in`, `tokens_out`, `tokens_cache`, `started_at`, `ended_at`, `context_tokens`, `compactions`, `peak_memory_mb`, `subagents[{role,model}]` |
| `reviewed` | the review summary | `verdict` (`approved` / `changes_requested`), `findings[{severity,class,text,file,line}]`, `probes`, `subagents[]`, the same telemetry keys |
| `asked` | the question | `context` (one line: what is waiting on it), `options[{id,label,detail}]`, `recommended`, `blocks[task_id]`, `asked_of` (`person` / lane id) |
| `answered` | an optional note | `option`, `surface` (`desktop` / `dojo`) |
| `message` | the message | `to` = `{kind: 'coordinator' \| 'lane' \| 'everyone', lane_id?}` (a lane by its **uuid**; the screen shows its code), `surface` |
| `delivered` / `read` | — | none; `reply_to` = the message (receipts, one per recipient) |
| `merged` | the merge note | `commits[]`, `into`, `gates[]` |
| `interrupted` / `reoffered` | — | `checkpoint`, `to_lane` |
| `compacted` | — | `context_tokens_before`, `context_tokens_after` |

- **Decisions are actions** (`asked` and `answered`, linked by `reply_to`). An open decision is an `asked` with no answer,
  and its task reads "needs your answer" while it is open. The same applies to messages.
- **Finding classes:** a lookup table `relay.finding_classes (code, label, added_by)`, seeded from the README checklist. The
  daemon rejects an unknown class in `props.findings`. The learning loop adds classes.

## Stages of a task (user question, 2026-11-05)
`assigned → working → in_review → (rework → in_review)* → approved → done`
- **assigned:** handed to a lane. The brief is frozen.
- **working:** the lane has started. On `report`, it goes to in_review.
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
  active task and lease · today's completed tasks.
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
