# Relay lanes · data model (proposal r2 for discussion, 2026-11-05)

**For:** [lanes.md](lanes.md). **Screen and interaction contract:** `coord/mockups/relay-lanes/CONTRACT.md` (Design r1, with
the "Message the crew" addendum, settled with the user). Nothing here is built yet.

**r2 (user, 2026-11-05):**
- Reports, reviews, findings, decisions and messages are all **actions on a task**: one table, categorised by kind.
- What is shared is **Markdown**; the numbers and structure behind it are **props**, which feed metrics.
- Items **reference the tracker's tickets** rather than copying them.
- One leases table.

## Principles
- **Reuse.**
  - A crew **is** an `activity.runs` row (new `kind = 'crew'`).
  - Tickets **are** `sensei.tracker_tasks` (the TG5 integration). Sprints come from `sensei.tracker_task_sprints`, and
    "an assistant fetched this ticket" from `sensei.session_task_links`.
  - Gates stay M21's.
- **One log for a crew: `relay.actions`.** Every hand-off, report, review, question, answer, message, merge and lease change
  is a row. It is the bus, the history and the audit trail. Drive runs keep `activity.run_events`; nothing is written twice.
- **Markdown for people, props for machines.** `body_md` is what a reader sees: a summary, a review, a question, a message.
  `props` (jsonb) holds structured facts: telemetry, files, gates, findings, options. Its keys are fixed per action kind
  and validated by the daemon, and metrics read them through views.
- **Nullable means "not reported" (D83).** A missing prop is absent, never 0. Totals say how many turns reported.

## Tables (six)

### `activity.runs` (existing) + `kind`
`kind run_kind ('drive','crew')`, default `'drive'`. The crew's workspace, goal, status, heartbeat and timestamps are the
existing columns.

### `relay.lanes`: who is registered
`id` uuid pk · `run_id` → runs · `code` (`L1`…, unique per run) · `title` null (the objective, e.g. "Diagrams") · `role`
(`'coordinator','contributor'`; one coordinator per run) · `family` / `version` null / `model` null · `capabilities` text[]
· `repo`, `worktree` null (the broker creates it) · `token_hash` · `checkpoint` jsonb null (for `reregister`) ·
`registered_at` / `unregistered_at` null.
Liveness is **not** a column here; it's the lane's lease (below).

### `relay.leases`: one table for every lease
`id` · `run_id` · `lane_id` · `kind` (`'lane','resource'`) · `resource` text null (`rust-build`, `e2e`…; null for a lane
lease) · `state` (`'queued','granted','released','expired','refused'`) · `expires_at` null · `granted_at`,
`released_at` null · `props` jsonb (disk/swap at grant, refusal reason).
- A lane lease is renewed by every MCP call. When it expires, the lane is suspended.
- A resource lease is the build slot: at most one granted per resource, machine-wide (partial unique).

### `relay.items`: the queue (triage and backlog)
`id` · `workspace_id` · `tracker_task_id` → `sensei.tracker_tasks` null (**the ticket; never copied**) · `title`, `kind`
(`'bug','feature','gap','issue','finding'`), `tags` text[] (**only for manual items**; a ticketed item reads its title, type
and sprint from the tracker) · `state` (`'backlog','planned','closed'`) · `position` int null (backlog order) ·
`added_by`, `added_at`.
- **Triage is a view, not rows:** tracker tasks in an active sprint (`tracker_task_sprints`, primary sprint) for the
  workspace's connected trackers that have no item yet, plus manual triage items (`state = 'triage'`, the one extra state).
- **`+` (add to backlog)** creates the item with `tracker_task_id` set and `state = 'backlog'`.
- The ticket button is the tracker task's key and URL.

### `relay.tasks`: what a lane works on
`id` · `run_id` · `item_id` null · `lane_id` null (**one active task per lane**, partial unique) · `reviewer_lane_id`
null (never the author's lane; checked) · `title` · `phase` (`'tests','impl','docs','mockup','merge'`) · `state`
(`'ready','assigned','in_progress','reported','in_review','changes_requested','approved','merged','done','blocked','interrupted'`)
· `round` · `owned_paths`, `acceptance`, `resources` text[] · `deps` uuid[] · `assigned_at`, `state_changed_at`,
`completed_at` null.
- The ticket comes through `item_id → tracker_task_id`.
- The **brief is the `assigned` action's `body_md`**, frozen. A change is a `brief_updated` action.
- A review is a state of the task: the reviewer's queue is `reviewer_lane_id = me and state = 'in_review'`.

### `relay.actions`: everything that happens
`id` bigserial · `run_id` · `task_id` null (null for crew-level ones, e.g. a message "about no particular task") ·
`lane_id` null (who acted; null = the person) · `kind` (`action_kind`, below) · `round` null · `phase` null ·
`body_md` text null (the shared text, ≤ 600 chars for summaries, leak-guarded) · `report_md` text null (a full report;
shared to the dōjō only when sharing is on) · `props` jsonb · `reply_to` bigint null (an answer → its question; a
review → the report it reviews) · `created_at`.

| `kind` | `body_md` is | `props` keys |
|---|---|---|
| `assigned` | the hand-off brief | `docs[]`, `owned_paths[]`, `acceptance[]` |
| `brief_updated` | the new brief | `docs[]` |
| `started` | — | — |
| `reported` | the summary sent with the request for review | `look_at`, `flags`, `base`, `branch`, `commits[]`, `files[{path,added,removed,outside_owned}]`, `gates[{cmd,exit,counts}]`, `family`, `version`, `model`, `tokens_in`, `tokens_out`, `tokens_cache`, `started_at`, `ended_at`, `context_tokens`, `compactions`, `peak_memory_mb`, `subagents[{role,model}]` |
| `reviewed` | the review summary | `verdict` (`approved` / `changes_requested`), `findings[{severity,class,text,file,line}]`, `probes`, `subagents[]`, the same telemetry keys |
| `asked` | the question | `options[{id,label,detail}]`, `recommended`, `blocks[task_id]`, `asked_of` (`person` / lane) |
| `answered` | an optional note | `option`, `surface` (`desktop` / `dojo`) |
| `message` | the message | `to` (`coordinator` / lane / `everyone`), `surface`; delivery is tracked by `delivered` / `read` actions with `reply_to` |
| `merged` | the merge note | `commits[]`, `into`, `gates[]` |
| `interrupted` / `reoffered` | — | `checkpoint`, `to_lane` |
| `compacted` | — | `context_tokens_before`, `context_tokens_after` |

- **Decisions are actions** (`asked` and `answered`, linked by `reply_to`). An open decision is an `asked` with no answer,
  and its task reads "needs your answer" while it is open. The same applies to messages.
- **Finding classes:** a lookup table `relay.finding_classes (code, label, added_by)`, seeded from the README checklist. The
  daemon rejects an unknown class in `props.findings`. The learning loop adds classes.

## Derived reads (security-invoker views; the screen never adds up numbers itself)
- `relay.now`: open `asked` actions for the person · the triage count by tracker and sprint · one row per lane with its
  active task and lease · today's completed tasks.
- `relay.task_history`: a task's actions, newest first, with props unpacked for the folds.
- `relay.lane_metrics`: per family/version/model: rounds, first-pass approval, finding classes (unnested from
  `reviewed.props.findings`), tokens, time working vs waiting for review, how accurate claims were.

## Integrity
- One coordinator per run, one active task per lane, at most one granted lease per resource, no self-review.
- Every task state change and its action are written in **one transaction**.
- Lane tokens are hashed. No transcript text anywhere. `body_md` and `report_md` pass the leak guard before any push.

## Decided (user, 2026-11-05)
- Actions over separate report/review/finding/decision/message tables.
- Markdown for what is shared; props for what metrics read.
- Items reference tracker tickets; triage is a view over active sprints.
- One `leases` table.
