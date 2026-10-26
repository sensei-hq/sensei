---
name: Tracker metrics write-back
description: Analysis — write each ticket's R7 usage metrics back to the tracker (Jira first), from the assignee's daemon only
date: 2026-10-26
stage: analysis
depends_on:
  - analysis/2026-10-15-jira-ticket-linkage.md
  - spec/daemon/tracker-jira-linkage.md
next: /sensei:design
---

# Tracker metrics write-back (analysis)

**Ask (verbatim):** *"we need to write back the ticket level metrics to ticket adaptor"*

**Restated from what exists:** the local daemon already computes per-ticket usage at
read time (spec R7: sessions, active / Claude / user / wall time, cycle time, the
token split, metered cost, each with its `_n`). The tracker adapter is **read-only**.
This slice adds a write path: on every sync tick, push those metrics to the ticket
in the tracker, into mapped custom fields or else one sensei-owned comment.

## 1 · What was read (grounding)

| Source | What it says |
|---|---|
| [`spec/daemon/tracker-jira-linkage.md`](../spec/daemon/tracker-jira-linkage.md) "Out of scope" | **"write-back to any tracker"** is out of scope for the linkage slice |
| same, R7 | Metrics computed at read, nothing materialised; a sum over zero non-NULL values is `null`, never `0`, with `<field>_n` |
| same, isolation gate 2b | Jira logic (endpoints, auth, formats) lives only in `tracker/jira.rs` |
| [`design/integrations-and-gating-requirements.md`](../design/integrations-and-gating-requirements.md) | Sync is **read-only** (decision 2026-10-14, "no export") |
| [`prd/modules/04-team/03-dojo-project-planning`](../prd/modules/04-team/03-dojo-project-planning/README.md) | Hours/tokens are insights, never gates, never per-author gates |
| `backlog.md`, Jira linkage deviation (3) | "no assignee synced/stored (no per-person figure)" |
| `crates/senseid/src/tracker/mod.rs:96,254` | `GetTransport` only has `get_json`; `TrackerAdapter` has `validate`, `fetch_snapshot`, `match_links`, `unknown_keys`, `fetch_tasks`, with no write method |
| `database/ddl/table/sensei/trackers.ddl` | `account` = the Atlassian **email** (keyring account); no account id |
| `database/ddl/table/sensei/tracker_tasks.ddl` | Mirror of the board's issues; `summary` is client-confidential and never leaves the machine |

## 2 · Surprises (the assumptions the ask carried)

1. **"Ticket-level" is really "one developer's".** A daemon only sees its own
   machine's sessions. Two people working one ticket would each push their own
   figure and overwrite each other.
2. **The ask reverses two recorded decisions:** read-only sync (2026-10-14) and
   deviation (3), no assignee or per-person figure (2026-10-15). Both reversals are
   approved by the user here (2026-10-26) and must be recorded in `backlog.md` and the
   integrations doc, not only in this analysis.
3. **Jira identifies the assignee by `accountId`, not email.** `trackers.account`
   (email) can't be compared with the assignee, so the token owner's `accountId` is
   needed (`GET /rest/api/3/myself`).
4. **An edited comment notifies watchers.** Writing every tick without a change
   check would spam everyone on the ticket.

## 3 · Requirement

### 3.1 Decisions (user, 2026-10-26)

| # | Decision |
|---|---|
| W1 | **Assignee-only writer.** A daemon writes a ticket only when the ticket's assignee is the token owner. This resolves the multi-writer conflict: one writer per ticket. |
| W2 | **Surface:** if the tracker has custom-field mappings, mapped metrics go to their fields; otherwise one sensei comment per ticket, edited in place. |
| W3 | **Cadence:** every sync tick, writing only when the payload changed (W8). |
| W4 | **Content:** all R7 metrics, including tokens and metered cost. |
| W5 | **Mapping model:** one-to-many, tracker → `sensei.tracker_fields` (one row per metric → field). The daemon calls an integration a *tracker*, hence the name. |
| W6 | **On by default:** `trackers.writeback_enabled default true`. |
| W7 | Only a boolean `assigned_to_me` is stored per task, never the assignee's identity, so no other person's identity lands in the DB. |
| W8 | Change detection: store a hash of the last written payload per task; skip when equal. |
| W9 | With mappings present, **unmapped metrics are not written** (no fallback comment), so a ticket never has two places holding sensei data. |
| W10 | A missing metric (`null`, `_n = 0`) is written as "—" in a comment and left empty in a field. Never `0` (no-fabrication rule). |
| W11 | The label on the tracker says whose sessions it counts ("Sensei (assignee's sessions)"), so the figure isn't read as the full ticket total. |

### 3.2 Data model (sketch, settled in design)

**`sensei.tracker_fields`** (new)

| column | meaning |
|---|---|
| `tracker_id` | FK → `trackers`, on delete cascade |
| `metric` | enum `tracker_metric`: `sessions`, `active_secs`, `claude_secs`, `user_secs`, `wall_secs`, `cycle_secs`, `tokens_fresh`, `cache_read`, `cache_write`, `tokens_out`, `tokens_reasoning`, `metered_cost` |
| `field_ref` | provider field id (Jira: `customfield_10050`) |
| unique `(tracker_id, metric)` | at most one field per metric |

**`sensei.trackers`** (new columns): `writeback_enabled bool not null default true`;
`account_ref text`, the provider's id of the token owner (Jira: `accountId` from
`/myself`), fetched at connect and re-connect.

**`sensei.tracker_tasks`** (new columns): `assigned_to_me bool`; `writeback_hash`;
`writeback_comment_ref`; `writeback_status` (enum, below); `writeback_at`.

### 3.3 Flow (per tick, after the read sync commits)

1. The sync fetches `assignee` with the issues and sets `assigned_to_me` (assignee
   `accountId = trackers.account_ref`). The assignee value itself is discarded.
2. For each task with `assigned_to_me` and `writeback_enabled`: compute R7 ticket
   usage → build the payload → hash it; equal to `writeback_hash` → skip.
3. With mappings, write the fields (Jira: `PUT /rest/api/3/issue/{id}` with
   `fields`). Without them, update the comment `writeback_comment_ref`
   (`PUT …/comment/{id}`) or create it (`POST …/comment`, in the rich-text format
   Jira's v3 API uses for comments) and store the id.
4. Record `writeback_status` / `writeback_hash` / `writeback_at` per task.

### 3.4 Failure rules

- A write failure **never fails the read sync**. It sets the task's
  `writeback_status`: `ok`, `forbidden` (403: the token lacks Edit Issues / Add
  Comments), `field_invalid` (field missing, wrong type, or not on the edit screen),
  `comment_removed` (404 on update), `not_a_person` (no `account_ref` match is
  possible: service/bot token), `error`.
- **`comment_removed` stops writes to that ticket.** We don't quietly re-post:
  deleting the comment is a signal to stop. Re-enabling it is an explicit action (design).
- A tracker whose token owner is never an assignee shows a tracker-level
  "write-back inactive" state, so the silence is explained.
- Unassigned tickets are never written (fail closed).
- Field mappings are validated at connect/mapping time (`GET /rest/api/3/field`:
  the field exists and has a writable type) → 422.

### 3.5 Done looks like (observable)

- A ticket assigned to the token owner, with linked sessions, shows the sensei comment
  (or mapped fields) in Jira after one tick; the values equal `GET …/usage` for it.
- A second tick with no new sessions makes **no** write request (transport mock
  records zero PUT/POST).
- A ticket assigned to someone else, or unassigned, gets no write request.
- A metric with `_n = 0` appears as "—" / an empty field, never `0`.
- A 403 on write leaves the sync `ok` and the task `writeback_status = forbidden`.
- `rg -i 'jira|atlassian'` over the adapter-agnostic write-back engine returns nothing
  outside tests (gate 2b extended).

### 3.6 Constraints (from `CLAUDE.md`)

- A new **write transport** trait beside `GetTransport`, injectable and mocked in
  tests. One `write_metrics` (or similar) method on `TrackerAdapter`; payload
  formatting lives in the adapter.
- Reuse the existing R7 usage computation (`tracker/usage.rs`), not a second one.
- Reuse the existing sync runtime and guard (R5) for the tick; no second scheduler.
- `summary` and other client-confidential fields are never sent back.

### 3.7 Out of scope

Team-total figures (Dōjō rollup, F-3); adapters other than Jira; a per-ticket opt-out
from the UI beyond the tracker toggle; backfilling history for reassigned tickets;
writing to epic-level aggregates.

## 4 · Remaining ambiguities

1. **Notifications.** Can a comment update or field edit run without notifying
   watchers? Jira's `notifyUsers=false` on issue edit needs admin rights. Check whether
   the comment endpoint supports it. If it can't be suppressed, W8 is the only
   mitigation.
2. **Fields that aren't on the edit screen** only fail when we write, not at
   validation. Check against a real tenant; maybe use `editmeta` per issue.
3. **Reassignment.** With a comment, A's comment stays (B can't edit it) and B
   creates their own. With fields, B overwrites A. Is that acceptable as designed, or
   should A's daemon mark its comment stale when the ticket leaves A?
4. **Which tracker levels write back.** Task and sprint trackers carry tickets. Does
   epic level write too?
5. **Field types.** Should durations go to a number field as seconds or as hours? Is
   cost a number in the tracker's currency?
6. **The comment's layout and label wording** (W11), to settle in design along with
   the desktop integrations screen (field mapping UI, toggle, status).

## 5 · Risks

- **Per-person exposure.** The number is the assignee's own AI time and cost,
  visible to everyone who can see the ticket. It's on by default (W6), so the connect
  screen must say this plainly.
- **Undercount read as truth.** Teammates' sessions are invisible (W11 mitigates).
- **Rate limits.** Every tick across many owned tickets. W8 bounds writes to
  changed tickets; the existing 429 handling must cover the write transport too.
- **Token scope creep.** A read-only token now needs edit/comment permissions;
  a 403 must stay visible, not a silent no-op.
