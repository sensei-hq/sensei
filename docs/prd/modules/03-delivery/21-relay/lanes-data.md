# Relay lanes · data model (proposal for discussion, 2026-11-05)

**For:** [lanes.md](lanes.md). **Screen and interaction contract:** `coord/mockups/relay-lanes/CONTRACT.md` (Design r1, settled
with the user). This turns that contract's §2–§4 into tables. Nothing here is built yet.

## Principles
- **Reuse the run engine.** A crew **is** an `activity.runs` row (a new `kind = 'crew'` beside today's drive runs). Every
  event **is** an `activity.run_events` row; its `kind` enum gains the crew event names (CONTRACT §3i). Gates stay M21's.
- **New tables live in a new `relay` schema** in the sensei DB (daemon-owned), for crew-only concepts: lanes, items, tasks,
  reports, reviews, findings, decisions, leases.
- **Nullable means "not reported" (D83).** Self-reported numbers are never defaulted to 0; totals say how many turns
  reported.
- **Enums for closed sets, a seeded table for the growing one** (finding classes: the learning loop adds to it).
- **The dōjō gets outlines** (slice 2): the same shapes minus `report_md`, which goes only where sharing is on.

## Tables

### `activity.runs` (existing) + one column
| column | type | note |
|---|---|---|
| `kind` | `run_kind` enum `('drive','crew')`, default `'drive'` | new; a crew is a run |

The crew's workspace, goal, status, heartbeat and timestamps are the existing columns.

### `relay.lanes`: who is registered (CONTRACT §3a, §3f)
| column | type | note |
|---|---|---|
| `id` | uuid pk | |
| `run_id` | uuid → `activity.runs` | the crew |
| `code` | text | `L1`, `L2`…; unique per run |
| `title` | text null | the objective ("Diagrams"), set at first assignment |
| `role` | `lane_role` enum `('coordinator','contributor')` | one coordinator per run (partial unique) |
| `family` / `version` / `model` | text / text null / text null | self-reported, recorded on every report too |
| `capabilities` | text[] | `rust`, `sql`, `ts`, `mockups`, `can-push`, `can-run-hooks` |
| `repo` / `worktree` | text / text null | the broker creates the worktree (lanes.md §13.2) |
| `token_hash` | text | the lane token is shown once; only its hash is stored |
| `lease_state` | `lease_state` enum `('live','suspended','gone')` | |
| `lease_expires_at` / `heartbeat_at` | timestamptz | any MCP call renews |
| `checkpoint` | jsonb null | resumed by `reregister` |
| `registered_at` / `unregistered_at` | timestamptz / null | |

### `relay.items`: triage and backlog in one table (CONTRACT §1d, §3h)
| column | type | note |
|---|---|---|
| `id` | uuid pk | |
| `workspace_id` | uuid | |
| `state` | `item_state` enum `('triage','backlog','planned','closed')` | `+` moves triage → backlog |
| `position` | int null | backlog order; the coordinator takes the lowest |
| `kind` | `item_kind` enum `('bug','feature','gap','issue','finding')` | |
| `title` / `tags` | text / text[] | |
| `source_provider` | `item_source` enum `('jira','ado','github','manual')` | |
| `source_key` / `source_url` / `sprint` | text null | the ticket button; the sprint pill |
| `created_by` / `created_at` | text / timestamptz | |

### `relay.tasks`: what a lane works on (CONTRACT §3b)
| column | type | note |
|---|---|---|
| `id` | uuid pk | |
| `run_id` / `item_id` | uuid / uuid null | the item it came from |
| `lane_id` | uuid null | **one active task per lane** (partial unique where state is active) |
| `reviewer_lane_id` | uuid null | set when the task enters review; never the author's lane (check) |
| `title` / `tags` | text / text[] | |
| `phase` | `task_phase` enum `('tests','impl','docs','mockup','merge')` | |
| `state` | `task_state` enum `('ready','assigned','in_progress','reported','in_review','changes_requested','approved','merged','done','blocked','interrupted')` | |
| `round` | int | +1 per `changes_requested` |
| `ticket_provider` / `ticket_key` / `ticket_url` | null | inherited from the item, or set by the coordinator |
| `brief_summary` / `brief_docs` | text / text[] | **frozen at assignment**; a change is `task.brief_updated` |
| `acceptance` / `owned_paths` / `resources` | text[] | |
| `deps` | uuid[] | forward-only (the coordinator checks) |
| `assigned_at` / `state_changed_at` / `completed_at` | timestamptz null | for "time on it" and "Completed today" |

A review is **a state of the task, not a separate task**: `get_tasks` for a reviewer returns tasks where
`reviewer_lane_id = me` and `state = 'in_review'`, ahead of new work.

### `relay.reports`: a work or rework turn (CONTRACT §3c)
| column | type | note |
|---|---|---|
| `id` / `task_id` / `lane_id` | uuid | |
| `round` / `phase` | int / `task_phase` | |
| `summary_changed` / `summary_look_at` / `summary_flags` | text / text / text null | ≤ 600 chars each, leak-guarded |
| `report_md` | text | shared to the dōjō only when sharing is on |
| `base` / `branch` / `commits` | text / text / text[] | |
| `files` | jsonb `[{path, added, removed, outside_owned}]` | **the broker computes `outside_owned`** |
| `gates` | jsonb `[{cmd, exit, counts}]` | |
| `family` / `version` / `model` | text | a snapshot at report time |
| `tokens_in` / `tokens_out` / `tokens_cache` | bigint null | not reported = null |
| `started_at` / `ended_at` | timestamptz null | |
| `context_tokens` / `compactions` / `peak_memory_mb` | int null | |
| `subagents` | jsonb `[{role, model}]` | |
| `created_at` | timestamptz | |

### `relay.reviews` + `relay.findings` (CONTRACT §3d)
`reviews`: `id, task_id, report_id, round, verdict` (enum `('approved','changes_requested')`), `summary` (≤ 600),
`reviewer_lane_id`, the reviewer's `family`/`version`/`model`, `subagents` jsonb, `probes` int null, the same telemetry
columns as reports, `created_at`.
`findings`: `id, review_id, severity` (enum `('high','medium','low','info')`), `class` → `relay.finding_classes(code)`,
`text, file null, line null, answered_in_report_id null` (set when the next round answers it).
`finding_classes`: seeded with `vacuous-test, missing-failure-path, single-tenant-fixture, role-untested,
aggregate-direction, secret-in-output, skipped-live-test, stale-claim, fabricated-value, parity-loss, …` (the README
checklist). The coordinator may add one.

### `relay.decisions` (CONTRACT §3e)
`id, run_id, task_id null, question, context, options` (jsonb `[{id,label,detail}]`), `recommended, blocks uuid[]`,
`state` (enum `('open','answered','withdrawn')`), `answer_option, answer_note, answered_by`, `answered_surface` (enum
`('desktop','dojo')`), `asked_at, answered_at`.

### `relay.resource_leases` (lanes.md §8)
`id, run_id, lane_id, resource` (text: `rust-build`, `e2e`…), `state` (enum `('queued','granted','released','refused')`),
`requested_at, granted_at, released_at`, `disk_free_bytes, swap_free_bytes` (measured at grant), `refusal_reason null`.
One `granted` row per resource machine-wide (partial unique).

### `activity.run_events` (existing): kinds and two columns
- `kind` gains: `lane.registered`, `lane.suspended`, `lane.resumed`, `task.assigned`, `task.brief_updated`,
  `task.started`, `task.reported`, `review.submitted`, `decision.asked`, `decision.answered`, `task.interrupted`,
  `task.reoffered`, `merge.done`, `task.completed`, `triage.added`, `backlog.added`, `resource.granted`,
  `context.compacted`.
- New nullable `task_id` and `lane_id` columns (indexed), so a task's history and a lane's timeline are one query each.
  `detail` jsonb keeps the payload.

## Derived reads (views, security-invoker; the screen never sums in TS)
- `relay.now`: decisions open, the triage count by source, one row per lane with its active task, completed today.
- `relay.task_history`: one task's events, reports, reviews and findings, newest first.
- `relay.lane_metrics`: first-pass approval, rounds, finding classes by family/version/model.

## Integrity
- The coordinator is unique per run, an active task is unique per lane, and a granted lease is unique per resource.
- `reviewer_lane_id <> lane_id` (no self-review).
- Every state change writes a `run_events` row in the same transaction: the events are the bus and the audit trail.
- Lane tokens are hashed. No transcript text in any column. Every text field is leak-guarded before any push.
