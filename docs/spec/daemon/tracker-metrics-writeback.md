---
name: Tracker metrics write-back
status: draft
date: 2026-10-26
parent: docs/spec/daemon/tracker-jira-linkage.md (R4, R5, R7, R8, R11, "Out of scope")
analysis: docs/analysis/2026-10-26-tracker-metrics-writeback.md
---

# Tracker metrics write-back

## Purpose

After every successful sync, write each ticket's R7 usage back to the tracker, but
only for tickets **assigned to the token owner**. The numbers go into the custom fields the
user mapped on the tracker, or, when none are mapped, into one sensei comment per ticket
that is edited in place. Jira is the first adapter. The engine is provider-neutral.

## Overrides of parents

| Parent | Said | Now | Why (user, 2026-10-26) |
|---|---|---|---|
| linkage spec "Out of scope" | write-back to any tracker | in scope, metrics only | the ask |
| `design/integrations-and-gating-requirements.md` | sync is read-only (2026-10-14, "no export") | reads stay read-only; this slice adds a separate, metrics-only write path | the ask |
| `backlog.md` Jira deviation (3) | no assignee synced or stored; no per-person figure | the assignee is compared, never stored; only `assigned_to_me` is kept; the figure written is the assignee's own | assignee-only writer resolves the multi-writer conflict |
| `tracker_tasks` table comment | "mirrored read-only … No assignee or other person data" | adds `assigned_to_me` and write-back state; still no other person's identity | as above |
| `screen/workspace-integrations.md` sub-line | "Read-only: sensei never changes anything in Jira." | the copy states what is written and where (W-UI) | the copy would be false |
| linkage spec R4 / `GetTransport` | "GET-only … no other verb can be sent" | still true of `GetTransport`; writes use a **separate** `WriteTransport` that only the write-back engine holds | keeps sync and discovery GET-only by construction |

## Resolved design

### W1 · Who writes

- At connect, `validate` already calls `GET /rest/api/3/myself`. It now also returns the
  token owner's **`owner_ref`**: the response's `accountId`, but only when `accountType`
  is `"atlassian"`. Any other account type (`app`, `customer`) or a missing field gives
  `None`. It's stored in `trackers.owner_ref`.
- A tracker connected before this slice has `owner_ref = NULL`. The sync then calls
  `adapter.owner_ref(t)` (the same request) once, before the fetch. On `Ok(Some)` it stores the value.
  On `Ok(None)` it stores nothing, and the tracker's write-back state is `owner_not_a_person`.
  On `Err` it carries on with the sync and leaves `owner_ref` NULL. That error is never
  recorded as a sync failure, because the read sync doesn't depend on it.
- `TaskRow` gains `assignee_ref: Option<String>` (Jira: `fields.assignee.accountId`,
  `None` when unassigned). Both issue reads request `assignee`: `ISSUE_FIELDS` for the
  board snapshot, and `search_keys` for R11 discovery. `assignee_ref` is **never
  persisted and never logged**.
- When the snapshot or a discovery is applied, each task's `assigned_to_me` is set
  provider-neutrally:
  - `NULL` when the tracker's `owner_ref` is NULL (unknown)
  - else `assignee_ref == owner_ref` (`false` covers unassigned)
- Only tasks with `assigned_to_me = true` are written. A ticket reassigned away is no
  longer written. Its existing comment or field values stay as they were (W5 records the
  time of the last change), and sensei never deletes what it wrote.

### W2 · When

- Write-back runs at the end of `runtime::sync_stored` when the sync returned `Ok`. That
  covers the scheduled tick (`tracker_sync`, 1800 s), "Sync now" and the first sync.
  It doesn't run after a discovery.
- It takes the tracker's `TrackerLocks` guard again, so it can't overlap a sync or
  another write-back of the same tracker. If the guard is busy, it skips this round.
  It runs under a deadline of `WRITEBACK_DEADLINE = 5 min`. Overrunning it stops the
  pass, and tasks not yet written keep their previous state.
- Skip the whole pass when `writeback_enabled = false` or `owner_ref IS NULL`.

### W3 · What (the metrics)

The enum `sensei.tracker_metric` lists the metrics in this order, each derived from the
existing R7 `Usage` that `tracker::usage::ticket_usages` already computes:

| metric | from `Usage` | unit written | null when |
|---|---|---|---|
| `sessions` | `sessions` | count | never (0 sessions → the task isn't written, W4) |
| `active_hours` | `active_secs / 3600` | hours, 2 dp | `active_n = 0` |
| `claude_hours` | `claude_secs / 3600` | hours, 2 dp | `claude_n = 0` |
| `user_hours` | `user_secs / 3600` | hours, 2 dp | `user_n = 0` |
| `wall_hours` | `wall_secs / 3600` | hours, 2 dp | `wall_n = 0` |
| `cycle_hours` | `(last_at − first_at) / 3600` | hours, 2 dp | `last_at` or `first_at` null |
| `tokens_fresh` | `tokens_fresh` | count | `tokens_fresh_n = 0` |
| `cache_read` | `cache_read` | count | `_n = 0` |
| `cache_write` | `cache_write` | count | `_n = 0` |
| `tokens_out` | `tokens_out` | count | `_n = 0` |
| `tokens_reasoning` | `tokens_reasoning` | count | `_n = 0` |
| `metered_cost` | `metered_cost` | USD, 2 dp | `metered_cost_n = 0` |
| `api_cost` | `api_cost` | USD, 2 dp | `api_cost_n = 0` |

`by_source` and `shared` are linkage diagnostics, not ticket metrics, so they're not
written. A null is **never written as 0**. A comment shows it as "—" and a mapped field
is cleared (W6).

### W4 · Which tasks, and change detection

- Candidates are tasks of the tracker with `assigned_to_me = true`, `removed_at IS NULL`,
  `writeback_status IS DISTINCT FROM 'comment_removed'`, and `Usage.sessions ≥ 1`. The
  usage comes from one `ticket_usages(pg, tracker_id, Some(&ids))` call per pass.
- The **payload** is the ordered list of `(metric, Option<value>)` after unit
  conversion and rounding, plus the surface (`fields` with the sorted mapping, or `comment`). `writeback_hash` =
  lowercase hex SHA-256 (`sha2`) of its canonical JSON. When it equals the stored hash the
  task is skipped: **no request is sent**. The "updated" time in a comment is not part of
  the payload.
- After a successful write, `writeback_hash`, `writeback_at = now()` and
  `writeback_status = 'ok'` are set in one statement. After a failed write, only
  `writeback_status` is set and the hash keeps its old value, so the next pass retries.

### W5 · Surface: comment (no mappings)

- If `writeback_comment_ref` is NULL, call `adapter.create_comment` and store the returned id.
  Otherwise call `adapter.update_comment(ref)`. If the update returns 404, set
  `writeback_status = 'comment_removed'`, keep the ref, and **stop** writing this
  task. Sensei doesn't post again.
- A `comment_removed` status is cleared, so writing resumes, for every task of the
  tracker when `writeback_enabled` goes from `false` to `true` (W8). It's also cleared by
  a re-connect.
- Provider-neutral content, built by a pure `writeback::render(&Payload, updated_at) -> Report`
  (a title plus ordered lines). The adapter turns it into the provider's format (Jira:
  ADF `doc` → one `paragraph` with the title in `strong`, then a `bulletList`):
  - Title: `Sensei: AI usage from the assignee's sessions (updated <YYYY-MM-DD HH:MM> UTC)`
  - `Sessions: N`
  - `Active time: X h (Claude X h, you X h)`
  - `Wall-clock time: X h`
  - `Cycle time: X h`
  - `Tokens: fresh N · cache read N · cache write N · out N · reasoning N`
  - `Cost: metered $X · API $X`
  
  Each null value is shown as `—`. When a metric's `_n` < `sessions`, it's suffixed
  ` (N of M sessions)`. When `api_cost_partial > 0`, the cost line ends
  ` (API cost partly priced for N sessions)`. No summary, prompt, branch, repo or session id
  ever appears.
- Jira: `POST /rest/api/3/issue/{external_id}/comment` with `{"body": <adf>}`. The update is
  `PUT /rest/api/3/issue/{external_id}/comment/{ref}?notifyUsers=false` (X2).

### W6 · Surface: mapped fields

- `sensei.tracker_fields (tracker_id, metric, field_ref)`, unique on `(tracker_id, metric)`
  and on `(tracker_id, field_ref)`. When a tracker has ≥ 1 row, only the mapped metrics are
  written, in **one** request per task. Unmapped metrics aren't written anywhere, and no
  comment is created or edited.
- Jira: `PUT /rest/api/3/issue/{external_id}` with `{"fields": {"<field_ref>": <number|null>}}`.
  A null sends JSON `null`, which clears the field. The request omits `notifyUsers`
  (X3). A 204 is success.
- Switching from comment to fields leaves an existing comment in place and stops
  editing it. Removing every mapping switches back, and the same `writeback_comment_ref`
  is edited again.

### W7 · Field mapping (validation)

- `PUT /api/workspaces/{id}/trackers/{tracker}/fields` with body
  `{"fields": [{"metric", "field_ref"}…]}` **replaces** the whole set. `[]` clears it.
  - Unknown metric → 422 `unknown_metric`. A repeated metric or `field_ref` → 422
    `duplicate_mapping`.
  - A `field_ref` that fails the adapter's shape check (Jira: `^customfield_[0-9]+$`) →
    422 `invalid_field_ref`, before any network call.
  - Then `adapter.check_fields(t, refs)` (Jira: `GET /rest/api/3/field`, one request)
    answers for each ref: missing → 422 `field_not_found` `{field_ref}`; `schema.type ≠
    "number"` → 422 `field_not_number` `{field_ref}`; a transport failure maps to the
    tracker error (R8 vocabulary).
  - On success, the set is replaced and `writeback_hash` is set to NULL for every task of the
    tracker, in one transaction. The next pass rewrites them.
- `GET …/fields` → `{"fields": [{metric, field_ref}]}` in enum order.
- Whether a field is **on the issue's edit screen** can't be checked here. A write
  rejected with 400 sets `writeback_status = 'field_invalid'` (X4).

### W8 · Toggle and status

- `trackers.writeback_enabled boolean not null default true`. The default also applies
  to trackers connected before this slice (user decision W6, analysis §3.1).
  `PATCH /api/workspaces/{id}/trackers/{tracker}` with `{"writeback_enabled": bool}` →
  200 with the tracker. Any other key → 422 `unknown_field`. Going from false to true
  clears `comment_removed` (W5).
- `GET …/trackers`: each tracker gains
  `"writeback": {"enabled", "state", "surface": "comment"|"fields", "mapped": n}`. Its
  `state` is the first that applies:
  1. `disabled`: `enabled` is false
  2. `owner_unknown`: `owner_ref` is NULL and the account type wasn't the cause
  3. `owner_not_a_person`
  4. `nothing_assigned`: no live task has `assigned_to_me`
  5. `active`
  
  `owner_unknown` and `owner_not_a_person` both have `owner_ref` NULL, so the
  difference is persisted as `trackers.owner_state` (`unknown | not_a_person`).
- `GET …/tasks` and `GET …/tracker-tasks/{key}`: each task gains `assigned_to_me`,
  `writeback_status`, `writeback_at`.

### W9 · Write statuses (`sensei.tracker_writeback_status`)

| status | cause |
|---|---|
| `ok` | last write succeeded |
| `forbidden` | 403: the token can't edit the issue or add or edit comments |
| `field_invalid` | 400 on a field write (field not on the screen, wrong type) |
| `comment_removed` | 404 on a comment update |
| `not_found` | 404 on the issue itself (the ticket was deleted or moved out of reach) |
| `rate_limited` | 429 after the retry budget |
| `error` | anything else (transport, 5xx, decode, deadline) |

A write failure **never** changes `trackers.last_error` or the sync's result. One task's
failure doesn't stop the others. Five consecutive `rate_limited` results end the pass
early. No raw response body or URL is stored or logged (R4 rule).

### W10 · Seams (provider-neutral)

- `tracker::WriteTransport`: `async fn send_json(&self, method: WriteMethod /* Post | Put */, path, query, body: &Value) -> Result<Value, TrackerError>`.
  `transport::PolicyTransport` implements it over a new `RawWrite` trait, with the **same**
  status mapping and 429 policy as GET. The retry loop is factored out once, not
  copied. A 2xx with an empty body decodes to `Value::Null`, since Jira's issue edit returns 204.
- `runtime::TransportFactory` gains `write_transport(base_url, auth, retries)`. Only
  `tracker::writeback` calls it. `sync`, `discover` and `connect` never hold a `WriteTransport`.
- `TrackerAdapter` gains:
  - `owner_ref(t) -> Result<Option<String>, TrackerError>`
  - `field_ref_shape(&str) -> bool`
  - `check_fields(t, &[String]) -> Result<Vec<FieldCheck>, TrackerError>`
  - `create_comment(w, task_external_id, &Report) -> Result<String, TrackerError>`
  - `update_comment(w, task_external_id, comment_ref, &Report) -> Result<(), TrackerError>`
  - `write_fields(w, task_external_id, &[(String, Option<f64>)]) -> Result<(), TrackerError>`
  
  `Validated` gains `owner_ref: Option<String>` and `owner_state`.
- `tracker::writeback` holds the pass, the payload, the hash, render and the status
  mapping, and has no Jira knowledge. Every SQL statement lives in
  `db::pg_store::trackers` (parent rule).

### W11 · DDL

- New enums `tracker_metric` (W3 order), `tracker_writeback_status` (W9 order), and
  `tracker_owner_state` (`unknown`, `not_a_person`). Each is pinned to its Rust `ALL` by
  a test, as `tracker_error` is.
- New table `tracker_fields` (W6), FK → `trackers` on delete cascade.
- `trackers` + `owner_ref text`, `owner_state tracker_owner_state`,
  `writeback_enabled boolean not null default true`. The table comment loses
  "Read-only mirror source" and says "mirror source; writes back metrics only (W)".
- `tracker_tasks` + `assigned_to_me boolean`, `writeback_hash text`,
  `writeback_comment_ref text`, `writeback_status tracker_writeback_status`,
  `writeback_at timestamptz`. The table comment's "No assignee or other person data"
  becomes "No assignee identity: only whether the token owner is the assignee".
- A re-connect clears `owner_ref`/`owner_state` before storing the new `validate` result.
  It also clears `comment_removed` (W5).

### W-UI · Integrations screen (`screen/workspace-integrations.md`, updated in this slice)

- The sub-line becomes: "Connect this project's Jira board so sensei can link AI sessions
  to tickets and report time, tokens and cost per ticket. On tickets assigned to you,
  sensei writes these numbers back to Jira (a comment, or the fields you map)."
- Each board card gets a **Write back to Jira** toggle (`data-action="writeback"`) and a
  state line from W8 `state`, using the texts in the screen spec's error-message table. It also
  gets a **Fields** section that lists the mappings, edits them (metric select + field id
  input) and saves with `PUT …/fields`, showing the W7 errors inline.
- The Disconnect body still says Jira isn't changed. That stays true: disconnect never
  deletes what was written.

### Out of scope

Team totals (Dōjō rollup, F-3); adapters other than Jira; per-ticket opt-out; deleting
or archiving sensei comments; text-type custom fields; writing `by_source`/`shared`;
backfilling a reassigned ticket's history; write-back after discovery.

## External assumptions (Jira Cloud API: to confirm against a tenant fixture in build)

| # | assumption | used by | if false |
|---|---|---|---|
| X1 | `GET /rest/api/3/myself` returns `accountId` and `accountType` (`atlassian` for a person) | W1 | `owner_state = unknown`; never write |
| X2 | comment update accepts `notifyUsers=false` for a non-admin author | W5 | drop the param; W4's change check is the only spam guard |
| X3 | issue edit's `notifyUsers=false` needs admin rights, so it's omitted | W6 | send it if a fixture shows a non-admin may |
| X4 | a field not on the edit screen → 400 | W7/W9 | map whichever status is observed to `field_invalid` |
| X5 | the board issue endpoint returns `fields.assignee.accountId` when asked for `assignee` | W1 | `assigned_to_me` stays false; nothing written |
| X6 | `GET /rest/api/3/field` lists custom fields with `id` and `schema.type` | W7 | validation can't run → refuse mappings (fail closed) |

## Done gate

1. **DDL**: `\d sensei.tracker_fields` shows the two unique constraints. The three enums'
   labels equal their Rust `ALL` (unit test). `trackers.writeback_enabled` defaults to `true`
   for a row inserted without it.
2. **Owner**: connecting with a fake `/myself` `{accountId:"A1", accountType:"atlassian"}`
   stores `owner_ref = 'A1'`. With `accountType:"app"` it stores `owner_ref NULL`,
   `owner_state = not_a_person`, and `GET …/trackers` says `writeback.state = owner_not_a_person`.
3. **Backfill**: a tracker with `owner_ref NULL, owner_state NULL` makes exactly one
   `/myself` request on its next sync and then stores it. A failing `/myself` leaves the
   sync `ok`.
4. **Assignee only**: on a fake board with AUTH-1 assigned to A1, AUTH-2 assigned to B2,
   AUTH-3 unassigned, all with linked sessions, one pass sends write requests for **AUTH-1
   only** (the write transport's call log). `assigned_to_me` is `t / f / f`.
5. **No identity stored**: `rg -n 'assignee' database/ddl` shows no column holding an
   assignee value, and after a sync no row in any table contains the string `B2`.
6. **Comment path**: the first pass POSTs a comment whose rendered lines match W5 for
   the ticket's `GET …/tracker-tasks/AUTH-1` usage, value for value, after unit
   conversion. It stores the returned id. A second pass with no new sessions sends **0**
   requests. A new linked session → exactly 1 PUT to `…/comment/{id}?notifyUsers=false`.
7. **Null is not 0**: a ticket whose sessions have no cost → `Cost: metered — · API —`
   in the comment, or `null` in a mapped field's body. Never `0`.
8. **Comment removed**: a 404 on update → `comment_removed`, and the next pass sends 0
   requests for that task. Toggling write-back off then on → the next pass POSTs a new comment.
9. **Fields path**: with `active_hours → customfield_10050` mapped, a pass sends one
   `PUT /rest/api/3/issue/{id}` whose body is exactly `{"fields":{"customfield_10050": <active_secs/3600 rounded to 2 dp>}}`,
   and **no** comment request.
10. **Mapping validation**: each W7 422 code is returned for its input. A `field_ref` failing the shape check
    makes **no** network request. A successful PUT nulls every task's `writeback_hash`.
11. **Isolation**: a write failure (403) leaves `trackers.last_error` NULL and the sync's
    result `Ok`, and sets that task `forbidden` while other tasks are written.
12. **GET-only stays GET-only**: `sync.rs`, `discover.rs` and `connect.rs` contain no
    `WriteTransport` (`rg -n WriteTransport` outside `mod.rs`, `transport.rs`,
    `runtime.rs`, `writeback.rs` and `jira.rs` → none).
13. **Gate 2b extended**: `rg -n -i 'jira|atlassian'` over `tracker/writeback.rs` outside
    `#[cfg(test)]` → no output.
14. **Disabled**: `PATCH {writeback_enabled:false}` → the next sync sends 0 write requests.
    `{"foo":1}` → 422 `unknown_field`.
15. **Lock**: a write-back while a manual sync holds the guard sends 0 requests and
    returns without error.

## Wrong gate

- The comment is written for every linked ticket, not just the assignee's: gate 4
  passes only on a board where every ticket happens to be assigned to the owner.
- `assigned_to_me` is derived from `trackers.account` (email) instead of `owner_ref`.
  Jira's assignee carries no email, so it's always false and nothing is ever written,
  and that still "passes" the identity gate.
- The hash includes the "updated" time, so every pass rewrites every comment (gate 6's
  second pass is non-zero) and every watcher gets a notification each tick.
- A null metric is written as `0` because `Option::unwrap_or(0.0)` was used in unit
  conversion.
- A write failure is recorded as `trackers.last_error`, so the board card shows the
  sync as broken when only write-back failed.
- On a 404, a new comment is POSTed instead of `comment_removed`, undoing the user's deletion.
- The retry loop is copied into a second impl instead of shared (DRY), and the copy
  drifts (no `Retry-After`).
- `assignee_ref` appears in a `tracing` line or the `Debug` of a persisted struct.

## Claims

Each row is a claim about the code as of `f81a9d487`, with the check that would disprove it.

| # | claim | check | expect | actual | verdict |
|---|---|---|---|---|---|
| C1 | Jira `validate` already calls `GET /rest/api/3/myself` and discards its body | read `tracker/jira.rs` `validate` | one call, result unused | | |
| C2 | `GetTransport` is the only tracker transport trait and has one method, `get_json` | `rg -n 'trait .*Transport' crates/senseid/src/tracker` | 1 trait, 1 method | | |
| C3 | No issue read requests `assignee` today (`ISSUE_FIELDS`, `search_keys`, sprint member reads) | `rg -n 'assignee' crates/senseid/src` | 0 hits | | |
| C4 | No table stores an assignee or a provider account id | `rg -n -i 'assignee\|account_id\|accountid' database/ddl` | 0 hits | | |
| C5 | `trackers.account` is the Atlassian email (the keyring account), not an account id | `trackers.ddl` column comment; `ConnectParams.account` | email | | |
| C6 | `tracker::usage::ticket_usages(pg, tracker_id, Some(ids))` returns a per-task `Usage` carrying every W3 input (`*_secs`/`*_n`, token fields, `metered_cost`, `api_cost`, `api_cost_partial`, `first_at`, `last_at`) | read `tracker/usage.rs` `Usage`, `ticket_usages` | all present | | |
| C7 | `runtime::sync_stored` is the one path the scheduled tick, "Sync now" and the first sync all take | `rg -n 'sync_stored\|sync_current' crates/senseid/src` | the 3 paths go through it | | |
| C8 | The `tracker_sync` schedule runs every 1800 s | `database/import/staging/schedules.jsonl` | 1800 | | |
| C9 | `TrackerLocks::try_lock` returns `None` when held, and one instance is shared by the API and the tick | read `tracker/sync.rs`, `runtime.rs` | as stated | | |
| C10 | `transport::map_response` treats a 2xx with an empty body as `Decode` | read `map_response` | `from_slice(b"")` → Err | | |
| C11 | `sha2` and `hex` are already dependencies of `senseid` | `crates/senseid/Cargo.toml` | both present | | |
| C12 | Every tracker route sits behind `origin_guard::local_caller_guard`, so new routes added to that router inherit it | read `api/routes.rs` around the tracker routes | one `route_layer` covers them | | |
| C13 | `/api/workspaces/{id}/trackers/{tracker}` has only `DELETE` today (no `PATCH`) | `api/routes.rs` | delete only | | |
| C14 | Every tracker SQL statement lives in `db::pg_store::trackers` (no SQL in `crates/senseid/src/tracker/`) | `rg -n 'sqlx::query' crates/senseid/src/tracker` | 0 outside tests | | |
| C15 | `tracker_error`'s labels are pinned to `TrackerError::ALL` by a test (the pattern W11 copies) | `tracker/mod.rs` tests | test exists | | |
| C16 | R11 discovery builds `TaskRow`s with the same `task_row` and inserts them through `apply_discovered` | read `jira.rs` `fetch_keys`, `pg_store/trackers.rs` `apply_discovered` | as stated | | |
| C17 | A sync failure is recorded in `trackers.last_error` (`record_failure` / `mark_tracker_error`), and nothing else writes it during a sync | read `sync.rs`, `runtime.rs` | as stated | | |
| C18 | The integrations screen's sub-line says "Read-only: sensei never changes anything in Jira." | `rg -n 'never changes anything' docs/spec/screen` | 1 hit | | |
| C19 | `activity.sessions.metered_cost` is in US dollars | `sessions.ddl` column comment | USD | | |
| C20 | `sync.rs`, `discover.rs` and `connect.rs` take only `&dyn GetTransport`, never a factory that could build another transport | read signatures | as stated | | |
| C21 | `TransportFactory` has one method returning `Arc<dyn GetTransport>`, with a test fake (`FakeFactory`) | `runtime.rs` | as stated | | |
| C22 | A re-connect of the same board updates the existing tracker row in place (level and token replaced), so W11's "re-connect clears `owner_ref`" has one place to live | read `tracker/connect.rs` | single upsert path | | |

## Related

- Analysis: [`analysis/2026-10-26-tracker-metrics-writeback.md`](../../analysis/2026-10-26-tracker-metrics-writeback.md)
- Parent: [`spec/daemon/tracker-jira-linkage.md`](tracker-jira-linkage.md)
- Screen: [`spec/screen/workspace-integrations.md`](../screen/workspace-integrations.md)
