---
name: Tracker metrics write-back
status: draft
date: 2026-10-26
parent: docs/spec/daemon/tracker-jira-linkage.md (R4, R5, R7, R8, R11, "Out of scope")
analysis: docs/analysis/2026-10-26-tracker-metrics-writeback.md
---

# Tracker metrics write-back

## Purpose

After every successful sync, write each ticket's R7 usage back to the tracker, but only
for tickets **assigned to the token owner**. The numbers go into the custom fields the user
mapped on the board, or, when none are mapped, into one sensei comment per ticket that is
edited in place. Jira is the first adapter. The engine is provider-neutral.

## Overrides of parents (reversals approved by the user, 2026-10-26)

The user was shown the PRD, `decisions.md` and design §2.6 wording and chose to
**reverse** the read-only rule for metrics, automatically after every sync, on by default
for every board including existing ones. Every document in the W-DOCS file set is updated
**in this slice** by one rule, so no document keeps contradicting the code. The table lists the
load-bearing statements, and W-DOCS covers the rest.

**The canonical narrowed sentence** (used wherever a document says trackers are read-only,
never written, or that sync never writes):

> Tracker reads are read-only: sync never writes sprints, statuses or memberships, and the
> dōjō never writes to a tracker. The one write path is usage-metrics write-back by the
> local daemon, to tickets assigned to the token owner (`spec/daemon/tracker-metrics-writeback.md`).

| Surface | Said | Now |
|---|---|---|
| `prd/modules/04-team/03-dojo-workspace-planning/README.md` :36 (resolved 2026-10-21), :58 (Out), :105 (Lead rule), :221-222 (wrong gate) | "adapters are strictly read-only"; "An adapter never writes to the tracker"; "a tracker adapter writes back" is a regression | The canonical sentence. The wrong gate becomes "a tracker adapter writes anything other than usage metrics, or to a ticket not assigned to the token owner". |
| `decisions.md` :69, :76 "Read-only means read-only"; `roadmap/09-dojo-expansion.md` D3 (:14) | "Tracker adapters never write back. There is no export-back path."; "Jira / ADO / GitHub stay read-only" | A new `decisions.md` row dated 2026-10-26 supersedes both for metrics write-back. The old rows stay, marked "superseded 2026-10-26 (tracker-metrics-writeback)". |
| `design/integrations-and-gating-requirements.md` :23-24, §2.6 :136-144, :401-403 (2026-10-14 decision), §6.2, §8 :457 ("Sync never writes; after a sync, the tracker is unchanged") | sync never writes; an export would be an explicit action | Metrics write-back **is** a side effect of every successful sync, on by default (user, 2026-10-26); otherwise the canonical sentence. The §8 row becomes "Sync never writes sprints, statuses or memberships; after a sync the only tracker change is metrics write-back". |
| `prd/modules/04-team/03-dojo-workspace-planning/scenarios/dojo-workspace-planning-gaps.feature` :29-32; `…/dojo-workspace-planning.feature` :5, :14 | "no export action and no write path" | the scenarios are narrowed to "sync never writes sprints, statuses or memberships" |
| `spec/dojo/daemon-sync.md` :20 | "the dōjō reads; nothing writes back" | the dōjō still never writes to a tracker; the local daemon writes metrics (this spec) |
| linkage spec Purpose :17-21 ("read-only", "never keyed by person"), Overrides row T3/D9 (:95 "no assignee"), "Out of scope" :657-659 ("write-back to any tracker", "assignee or any per-person figure"), Wrong gate :753 | read-only; no assignee; no per-person figure | The assignee is **compared, never stored**. The figure written is the assignee's own; usage **API output** still carries no person attribute. `assigned_to_me` is a property of the ticket relative to this machine's token, not a person attribute (W1). |
| linkage spec Done gate 2 :717 and seam list :73 ("GET-only transport"); `tracker/mod.rs:94` doc comment; `tracker/transport.rs:1` module doc | no `post/put/…` call anywhere in `tracker/` | `GetTransport` stays GET-only, and sync, discover and connect only ever get one. Gate 2 is restated as this spec's gate 13. |
| `backlog.md` :106 Jira deviation (3) | no assignee synced or stored; no per-person figure | as the linkage row above |
| `database/ddl/table/sensei/{trackers,tracker_tasks}.ddl` comments and `database/design.dbml` :2781, :4135 | "Read-only mirror source"; "mirrored read-only … No assignee or other person data" | W11 wording, in both DDL and dbml |
| `screen/workspace-integrations.md` sub-line :46-48, wrong gate :172; `screen/workspace-tickets.md` sub-line :45-47, wrong gates :133-134; `app/…/integrations/IntegrationsScreen.svelte:71`; `app/…/tickets/TicketsScreen.svelte:40` | "Read-only: sensei never changes anything in Jira."; "Read from Jira; nothing is changed there."; "any write route called from this screen"; no per-person data shown | W-UI copy for both screens. The screen wrong gates gain: "`assigned_to_me` and the write-back state are not person data; showing them is allowed". The Tickets wrong gate :134 stays: the Tickets screen itself calls no write route. |
| `prd/modules/04-team/README.md` :22, :28-29; `prd/modules/04-team/05-releases-and-dora/README.md` :24-25 | "external systems (trackers, CI) are read-only"; "tracker APIs are read, never written" | the canonical sentence (CI stays read-only) |

## Resolved design

### W1 · Who writes

- **Owner lookup.** The adapter method `owner(t) -> Result<OwnerLookup, TrackerError>`
  returns one of three outcomes. For Jira it reads `GET /rest/api/3/myself`, the request
  `validate` already makes:
  - `Person(AccountRef)` when `accountType == "atlassian"` and `accountId` is a non-empty string
  - `NotAPerson` when `accountType` is present and is anything else (`app`, `customer`)
  - `Unknown` when either field is missing
- `validate` returns the lookup in `Validated.owner`. Connect stores it:
  - `Person(r)` → `owner_ref = r`, `owner_state = NULL`
  - `NotAPerson` → `owner_ref = NULL`, `owner_state = 'not_a_person'`
  - `Unknown` → `owner_ref = NULL`, `owner_state = 'unknown'`
- **Backfill.** Step 1 of the sync (after the guard, before the fetch) runs this only
  when `owner_ref IS NULL AND owner_state IS DISTINCT FROM 'not_a_person'`. It makes one
  `owner(t)` call and stores the result as connect does. On `Err` it stores nothing,
  which leaves the state `unknown` (or NULL), and the sync carries on. The read sync never
  fails because of the lookup. A `not_a_person` tracker never asks again until a re-connect.
- **The comparison uses the owner as of step 1**, so the snapshot applied in the same sync uses
  the value just stored.
- **Assignee.** `TaskRow` gains `assignee: Option<AccountRef>` (Jira:
  `fields.assignee.accountId`, `None` when unassigned or absent). The board issue read
  (`ISSUE_FIELDS`) and the R11 `search_keys` read request `assignee`. The sprint-member
  read only collects ids, so it doesn't.
  - `AccountRef` is a newtype whose `Debug` prints `AccountRef(***)`. It's never
    persisted from a `TaskRow`, never logged, and never part of an API response.
- **`assigned_to_me`** is set in `upsert_tasks` (the function both the snapshot apply and
  `apply_discovered` use) from the tracker's `owner_ref`, read in the same transaction:
  - `NULL` when `owner_ref` is NULL
  - `true` when `assignee == owner_ref`
  - `false` otherwise (unassigned included)
- Only tasks with `assigned_to_me = true` are written. A ticket reassigned away isn't
  written again. Whatever was last written stays and shows its "updated" time, and sensei
  never deletes what it wrote.

### W2 · When, lock, deadline

- `runtime::sync_stored`, the one path for the scheduled tick (`tracker_sync`, 1800 s),
  "Sync now" and the first sync, calls `writeback::run(pg, rt, &row, adapter)` after the
  sync returns `Ok`. Its outcome doesn't change `sync_stored`'s return value or the "Sync
  now" response. The result is visible through `GET …/trackers` and the task reads (W8).
  Discovery doesn't trigger it.
- `run` takes `rt.locks.try_lock(tracker_id)`. If the lock is busy, it returns at once with
  no request and no error. This guard is the sync's, re-taken after the sync dropped it, so
  a manual sync can't run during a pass.
- The whole pass runs under `WRITEBACK_DEADLINE = 5 min`, including every 429 sleep.
  Writes use `WRITE_RETRIES = 3`, the same budget as `SYNC_RETRIES`. When the deadline is
  hit, tasks not yet written keep their state.
- Discovery takes the same guard, so it is also locked out during a pass. A discovery
  that finds the guard busy returns `Busy`, as it does today against a sync.
- **The pass's own credential.** `run` builds its write transport with
  `rt.transports.write_transport(...)`. If the keyring read fails (`TokenMissing`,
  `CredentialStore`), it sets `trackers.writeback_error` to that code and returns, with
  no task status written. A pass that gets a transport clears `writeback_error` first.
  This is the only place `writeback_error` is written, and `last_error` is never touched.
- **Before every task's write**, one read (`writeback_target(task_id)`) re-checks four things:
  - the tracker still exists
  - `writeback_enabled` is true
  - `secret_service` is unchanged since the pass started
  - the task is still `assigned_to_me`
  
  If any fails, the pass stops (tracker gone, disabled, re-keyed) or skips the task
  (reassigned). A switch-off or disconnect therefore takes effect before the next request.
- The pass is skipped entirely, with no request, when `writeback_enabled = false` or
  `owner_ref IS NULL`.

### W3 · What (the metrics)

The enum `sensei.tracker_metric` lists these in this order. Each comes from the R7 `Usage`
returned by `tracker::usage::ticket_usages`. A metric is **null exactly when its `Usage`
value is `None`**, never decided by an `_n` count (an unpriced session has
`api_cost_n ≥ 1` and `api_cost = None`).

| metric | value | kind |
|---|---|---|
| `sessions` | `sessions` | integer |
| `active_hours` | `active_secs / 3600` | decimal |
| `claude_hours` | `claude_secs / 3600` | decimal |
| `user_hours` | `user_secs / 3600` | decimal |
| `wall_hours` | `wall_secs / 3600` | decimal |
| `cycle_hours` | `(last_at − first_at) / 3600`, null if either is `None` | decimal |
| `tokens_fresh` | `tokens_fresh` | integer |
| `cache_read` | `cache_read` | integer |
| `cache_write` | `cache_write` | integer |
| `tokens_out` | `tokens_out` | integer |
| `tokens_reasoning` | `tokens_reasoning` | integer |
| `metered_cost` | `metered_cost`, in the source's own currency units, with no currency named (`sessions.metered_cost` comment) | decimal |
| `api_cost` | `api_cost`, US dollars (`model_prices`) | decimal |

- **Decimals** are rounded to 2 dp with `f64::round` (half away from zero) on
  `x * 100.0`. The **canonical value** is the string `format!("{:.2}", rounded)`.
  **Integers** are written as JSON integers, with the canonical value `i64::to_string`.
  Both the hash and the request body use the canonical value (a field body parses the
  decimal string back into a JSON number).
- `by_source` isn't written. `shared` is shown in the comment only (W5). It isn't a mapped metric.
- **Mapped fields aren't additive across tickets.** A session linked to two tickets
  counts in full on both, the R7 rule. The comment states the overlap, and W-UI's Fields
  section says so under the editor.

### W4 · Which tasks, payload, change detection

- **Candidates for a pass** are the tracker's tasks with `assigned_to_me = true`,
  `removed_at IS NULL`, and either `Usage.sessions ≥ 1` or `writeback_hash IS NOT NULL`.
  The second case is a ticket that has lost all its sessions, for example its repo left
  the workspace. It is written as `sessions = 0` with every other metric null, so the
  ticket stops showing stale figures. The usage comes from one
  `ticket_usages(pg, tracker_id, Some(&ids))` call per pass.
- **Sticky statuses are skipped.** A candidate is skipped while its `writeback_status` is
  `forbidden`, `not_found` or `field_invalid`, or `comment_removed` on the comment surface
  only. Those clear as W9 says. `rate_limited` and `error` retry on the next pass.
- **The payload, and its hash, is exactly what would be sent, minus the "updated" time:**
  - comment surface: the rendered `Report` lines (W5)
  - fields surface: the mapped `(field_ref, canonical value)` pairs in `tracker_metric`
    enum order
  
  `writeback_hash` is the lowercase hex SHA-256 (`sha2`, `hex`) of those strings joined
  with `\n`, prefixed by the surface name. A change to an unmapped metric therefore sends
  nothing on the fields surface. A changed `_n`, `shared` or `api_cost_partial` that
  changes a comment line does send.
- When the hash equals the stored one, the task is skipped and **no request is sent**.
- After a write succeeds, `writeback_hash`, `writeback_at = now()` and
  `writeback_status = 'ok'` are set in one statement. After a failure, only
  `writeback_status` is set, so the hash stays as it was and a transient failure retries.

### W5 · Surface: comment (no mappings)

- **Rendering.** The pure function `writeback::render(&Usage, updated_at) -> Report`
  produces `Report { title: String, lines: Vec<String> }`. The adapter turns a `Report`
  into the provider's format. For Jira that is an ADF `doc` with one `paragraph` (the title
  in `strong`) and one `bulletList` with a `listItem` per line. Every string goes in as a
  `text` node through `serde_json`, never by interpolation.
- **Golden example.** 3 sessions; active 7380 s over 3 sessions, split Claude 4680 s and
  you 2700 s; wall 14400 s; first-to-last 2 days; tokens fresh 120000, cache read
  900000, cache write 30000, out 45000, reasoning null; metered null; API 4.317 with
  `api_cost_n = 3` and `api_cost_partial = 1`; `shared = 1`. It renders exactly:
  ```
  Sensei · AI usage from the assignee's sessions (updated 2026-10-26 14:30 UTC)
  Sessions: 3 (1 shared with other tickets)
  Active time: 2.05 h (Claude 1.30 h, you 0.75 h)
  Wall-clock time: 4.00 h
  Cycle time: 48.00 h
  Tokens: fresh 120000 · cache read 900000 · cache write 30000 · out 45000 · reasoning —
  Cost: metered — · API $4.32 (partly priced for 1 session)
  ```
  The rules:
  - A null value is `—`.
  - A value whose `_n` satisfies `0 < _n < sessions` is followed by ` (N of M sessions)`,
    attached to that value, not to the line.
  - ` (N shared with other tickets)` appears when `shared > 0`.
  - The `partly priced` note appears when `api_cost_partial > 0`.
  - Only `api_cost` carries `$`.
  - Numbers have no thousands separator.
  - No summary, prompt, branch, repo, session id or person name ever appears.
- **Create.** When `writeback_comment_ref` is NULL, the adapter first **looks for a
  comment sensei already wrote**. That covers a create whose response was lost.
  - Jira: `GET /rest/api/3/issue/{id}/comment?orderBy=-created&maxResults=50` over the
    `GetTransport`. It adopts the newest comment whose `author.accountId == owner_ref` and
    whose first text node starts with `Sensei · AI usage`.
  - If it adopts one, it updates that comment. Otherwise it creates one with
    `POST /rest/api/3/issue/{id}/comment` `{"body": <adf>}`.
  - The adopted or returned id is stored.
- **Update.** `PUT /rest/api/3/issue/{id}/comment/{ref}?notifyUsers=false` `{"body": <adf>}`.
- **A 404 on update** is followed by one `GET /rest/api/3/issue/{id}?fields=none` (GET
  transport) to see which object is gone:
  - 404 → `not_found`
  - 2xx → `comment_removed`
  - any other error → `error`
  
  The comment ref is kept with `comment_removed`, so the status explains it. Sensei
  doesn't post again on its own.

### W6 · Surface: mapped fields

- `sensei.tracker_fields (tracker_id, metric, field_ref)`, unique on `(tracker_id, metric)`
  and on `(tracker_id, field_ref)`, FK → `trackers` on delete cascade. When a tracker has
  ≥ 1 row, only mapped metrics are written, in **one** request per task. No comment is
  created or edited.
- Jira: `PUT /rest/api/3/issue/{id}` `{"fields": {"<field_ref>": <number|null>, …}}`. A
  null is JSON `null`, which clears the field. No `notifyUsers` parameter is sent (X3). A
  2xx (Jira: 204) is success.
- Switching comment → fields leaves an existing comment untouched. Switching fields →
  comment edits the same `writeback_comment_ref` again (W7 nulls the hashes, so it is
  rewritten).

### W7 · Field mapping

- `PUT /api/workspaces/{id}/trackers/{tracker}/fields` with body
  `{"fields": [{"metric": "...", "field_ref": "..."}…]}` **replaces** the whole set, and
  `[]` clears it. It's accepted in every write-back state (disabled included). The checks
  run in this order, stopping at the first failure:
  1. The body isn't an object with a `fields` array of `{metric, field_ref}` strings → 422
     `invalid_body`.
  2. Unknown metric → 422 `unknown_metric` `{metric}`.
  3. A metric or `field_ref` appears twice → 422 `duplicate_mapping` `{metric|field_ref}`.
  4. `field_ref` fails `adapter.field_ref_shape` (Jira: `^customfield_[1-9][0-9]{0,9}$`)
     → 422 `invalid_field_ref` `{field_ref}`. No network request has been made yet.
  5. `adapter.check_fields(t, refs) -> Result<Vec<FieldCheck>, TrackerError>`, where
     `FieldCheck { field_ref, exists: bool, numeric: bool }`. Jira does this with one
     `GET /rest/api/3/field` and checks `schema.type == "number"`. Missing → 422
     `field_not_found` `{field_ref}`. Not numeric → 422 `field_not_number` `{field_ref}`.
  6. **Fields already in use.** For each `field_ref` not already mapped on this tracker,
     `adapter.field_in_use(t, board, field_ref) -> Result<bool, TrackerError>`. Jira: the
     board issue endpoint with `jql=cf[N] is not EMPTY&maxResults=1&fields=none`. `true` →
     422 `field_in_use` `{field_ref}`, so a mapping can't take over Story Points or any
     field people fill in.
  7. Transport and credential failures use the statuses connect returns for the same
     `TrackerError` (C26): `Unauthorized` → 401 `unauthorized`, `Forbidden` → 403
     `forbidden`, `TokenMissing`/`CredentialStore` → 500 with their code, and any other →
     502 with its code. The handler maps `TrackerError` directly. It doesn't go through
     `connect_failure`, which matches `ConnectError`. The status table is factored into
     one `fn tracker_error_status(TrackerError) -> StatusCode` that both use.
- On success, one transaction replaces the set, sets `writeback_hash = NULL` on every task
  of the tracker, and clears `field_invalid`. The route returns 200 with the
  `GET …/fields` body.
- `GET …/fields` → 200 `{"fields": [{metric, field_ref}]}` in enum order.
- Whether a field is on an issue's edit screen can't be checked in advance. A write
  rejected with 400 is `field_invalid` (X4).

### W8 · Toggle, status, API

- `trackers.writeback_enabled boolean not null default true` applies to every row,
  including trackers connected before this slice. The user confirmed this on 2026-10-26
  after being shown that existing boards were connected under the "Read-only" copy.
- `PATCH /api/workspaces/{id}/trackers/{tracker}`: the body must be exactly
  `{"writeback_enabled": <bool>}`. Anything else (missing, non-bool, extra key) → 422
  `invalid_body`. It returns 200 with the tracker in the `GET …/trackers` element shape.
  Going false → true clears `comment_removed`, `forbidden` and `not_found` for that
  tracker's tasks, and with `comment_removed` it also sets `writeback_comment_ref = NULL`
  and `writeback_hash = NULL`. The next pass therefore posts a new comment.
- `GET …/trackers`: each tracker gains
  `"writeback": {"enabled", "state", "surface": "comment"|"fields", "mapped": <rows>, "assigned": <live tasks with assigned_to_me>, "failing": <tasks in a non-ok status>, "error": <writeback_error|null>}`.
  `state` is the first of these that applies:
  1. `disabled`
  2. `credential_failed` (`writeback_error` not NULL)
  3. `owner_not_a_person` (`owner_state = not_a_person`)
  4. `owner_unknown` (`owner_ref` NULL otherwise)
  5. `nothing_assigned` (`assigned = 0`)
  6. `active`
- `GET …/tasks` and `GET …/tracker-tasks/{key}`: each task gains `assigned_to_me`,
  `writeback_status`, `writeback_at`.
- Both new routes are added inside `api::routes::tracker_routes()`, before its
  `route_layer(local_caller_guard)`, so they inherit the guard (C12).

### W9 · Write statuses (`sensei.tracker_writeback_status`)

The mapping covers every `TrackerError` and every operation:

| outcome | status | sticky until |
|---|---|---|
| 2xx | `ok` | n/a |
| `Forbidden` (403), `Unauthorized` (401) on any write | `forbidden` | toggle on, re-connect |
| `BadRequest` (400) on a field write | `field_invalid` | mapping PUT, toggle on, re-connect |
| `NotFound` on a field write or a comment create; the W5 404 check | `not_found` | toggle on, re-connect |
| the W5 404 check finds the issue | `comment_removed` | toggle on, re-connect (both clear the ref) |
| `RateLimited` after `WRITE_RETRIES` | `rate_limited` | not sticky |
| `BadRequest` on a comment write, `HttpError`, `Transport`, `Decode`, `Redirect`, `Db`, deadline, an id failing W10's shape check | `error` | not sticky |
| `TokenMissing`, `CredentialStore` (no write transport) | no task status; `trackers.writeback_error` = the code (W2) | cleared by the next pass that gets a transport |

- A write failure **never** writes `trackers.last_error` or changes the sync's result. The
  success apply clears `last_error` before the pass runs (C17). One task's failure doesn't
  stop the others.
- Five consecutive `rate_limited` results end the pass.
- No response body, URL or account id is stored or logged. A log line carries only
  `tracker_id`, `task_id` and the status code.

### W10 · Seams (provider-neutral)

```rust
pub enum WriteMethod { Post, Put }

#[async_trait] pub trait WriteTransport: Send + Sync {
    async fn send_json(&self, method: WriteMethod, path: &str, query: &[(&str, String)],
                       body: &serde_json::Value) -> Result<serde_json::Value, TrackerError>;
}
#[async_trait] pub trait RawWrite: Send + Sync {            // transport.rs, beside RawGet
    async fn send(&self, method: WriteMethod, path: &str, query: &[(&str, String)],
                  body: &[u8]) -> Result<RawResponse, ()>;
}
```

- `PolicyTransport<R>` implements `WriteTransport` for `R: RawWrite`. The 429 loop
  is **one** generic function used by both the GET and the write impls, not a copy. It
  takes the response mapper as a parameter.
  - GET keeps `map_response` unchanged: an empty 2xx is still `Decode` (C10).
  - Writes use `map_write_response`, which returns `Value::Null` for an empty 2xx and
    otherwise delegates to `map_response`.
  
  No existing GET caller changes behaviour (C30).
- `ReqwestRaw` implements `RawWrite` on **the same client** it builds for GET:
  `crate::http::bounded_builder()` with `redirect::Policy::none()`. A 3xx on a write is
  `Redirect`, never followed. The base URL is the stored, normalised `trackers.base_url`,
  so no new host is possible.
- `TransportFactory` gains
  `write_transport(base_url, auth, retries) -> Result<Arc<dyn WriteTransport>, TrackerError>`,
  and `FakeFactory` gains a recording fake. `write_transport(` is called **only** from
  `tracker/writeback.rs`.
- `TrackerAdapter` gains these methods. `t` is `&dyn GetTransport` and `w` is
  `&dyn WriteTransport`:
  - `owner(t) -> Result<OwnerLookup, TrackerError>`
  - `field_ref_shape(&str) -> bool`
  - `check_fields(t, &[String]) -> Result<Vec<FieldCheck>, TrackerError>`
  - `field_in_use(t, &BoardScope, &str) -> Result<bool, TrackerError>`
  - `find_comment(t, task_ext_id, &AccountRef) -> Result<Option<String>, TrackerError>`
  - `create_comment(w, task_ext_id, &Report) -> Result<String, TrackerError>`
  - `update_comment(w, task_ext_id, comment_ref, &Report) -> Result<(), TrackerError>`
  - `issue_exists(t, task_ext_id) -> Result<bool, TrackerError>`
  - `write_fields(w, task_ext_id, &[(String, FieldValue)]) -> Result<(), TrackerError>`,
    where `FieldValue = Int(i64) | Dec(String) | Null`
- **Path safety.** Before any path is built, each adapter write method checks
  `task_ext_id` and `comment_ref` against the adapter's id shape (Jira: `^[0-9]{1,18}$`)
  and every `field_ref` against `field_ref_shape`. Anything that fails returns
  `BadRequest`, mapped to `error`, without sending a request.
- `tracker/writeback.rs` holds the pass, the payload, the hash, `render` and the status
  mapping, and knows nothing about Jira. Its SQL lives in `db::pg_store`, in `trackers.rs`
  for tracker and task state and in `tracker_usage.rs` for usage reads.

### W11 · DDL

- New enums, each pinned to its Rust `ALL` by a test as `tracker_error` is:
  - `tracker_metric` (W3 order)
  - `tracker_writeback_status` (`ok, forbidden, field_invalid, comment_removed, not_found, rate_limited, error`)
  - `tracker_owner_state` (`unknown, not_a_person`)
- New table `tracker_fields` (W6).
- `trackers` gains `owner_ref text`, `owner_state tracker_owner_state`,
  `writeback_enabled boolean not null default true` and `writeback_error tracker_error`. The comment's "Read-only mirror source
  for …" becomes "Mirror source for …; writes back usage metrics only, to tickets
  assigned to the token owner (spec tracker-metrics-writeback)".
- `tracker_tasks` gains `assigned_to_me boolean`, `writeback_hash text`,
  `writeback_comment_ref text`, `writeback_status tracker_writeback_status` and
  `writeback_at timestamptz`. The header line and table comment change "mirrored
  read-only" to "mirrored from the tracker", and "No assignee or other person data, by
  design" to "No assignee identity: only whether the token owner is the assignee".
- `database/design.dbml` gets the same comments, columns and table.
- **Re-key and re-connect.** `connect_tracker` writes the connected board (an UPDATE on
  re-connect, an INSERT on a new board) and then, on **every** connect, re-keys each
  sibling board on the same site onto the new token (C22, C31). On the connected board
  and every re-keyed sibling it:
  - sets `owner_ref` and `owner_state` (the connected board from its `validate`; siblings
    to NULL, so their next sync backfills) and `writeback_error = NULL`
  - sets `writeback_comment_ref`, `writeback_hash`, `writeback_status` and
    `assigned_to_me` to NULL for their tasks, since a new token owner can't edit the old
    owner's comments
  
  The connect caller already re-syncs siblings.

### W-UI · Integrations screen

- **Sub-line** (based on the shipped D40 text in `IntegrationsScreen.svelte:71`): "Connect this
  workspace's Jira boards, each at a level, so sensei can link AI sessions to tickets and
  report time, tokens and cost per ticket. On tickets assigned to you, sensei writes these
  numbers back to Jira, as a comment or in the fields you map."
- **Each board card** gets:
  - **Write back to Jira**, a toggle (`data-action="writeback"`) that sends the PATCH.
    While pending, the toggle is disabled. On error, it reverts and shows the code text.
  - **A state line** (`data-writeback-state=<state>`):
    - `active`: "Writing to N tickets assigned to you." N is `writeback.assigned`. With
      `failing > 0` it adds " K couldn't be written."
    - `disabled`: "Write-back is off."
    - `credential_failed`: "Couldn't read the Jira token to write back. Replace the token."
    - `owner_not_a_person`: "This token belongs to an app account, so sensei can't tell
      which tickets are yours. Nothing is written."
    - `owner_unknown`: "Checking which tickets are yours on the next sync."
    - `nothing_assigned`: "No tickets on this board are assigned to you."
  - **Fields** (`data-section="fields"`): lists the mappings in enum order, adds a row
    (metric select + field id input), removes a row, and saves with `PUT …/fields`. Under
    the editor: "Each ticket counts its own sessions in full; don't sum these fields across
    tickets." W7 errors are shown inline on the row they name:
    - `unknown_metric` "Unknown metric."
    - `duplicate_mapping` "Already mapped."
    - `invalid_field_ref` "Use a custom field id like customfield_10050."
    - `field_not_found` "No such field on this Jira site."
    - `field_not_number` "That field isn't a number field."
    - `field_in_use` "That field already has values on this board. Pick an empty field."
    - `invalid_body` "Couldn't save the mapping."
- **Per-ticket write status** on the Tickets screen stays out of this slice. The state
  line's `failing` count is the only aggregate.
- **Tickets screen sub-line** (`TicketsScreen.svelte:40`, `workspace-tickets.md:45-47`):
  "… Read from Jira. On tickets assigned to you, sensei writes these numbers back, as set
  on Integrations."
- **App wrappers** go on `senseiApi` in `app/src/lib/api.ts`, beside `tryListTrackers`
  and the others, following their `try*` convention (C29): `trySetTrackerWriteback`,
  `tryGetTrackerFields`, `tryPutTrackerFields`. They're tested in
  `app/src/lib/api-trackers.spec.ts`. The W7/W8 codes and `credential_failed` are added to
  `tracker-errors.ts`.
- **Disconnect copy** stays "Jira itself is not changed." It's still true: disconnect never
  deletes what was written.

### W-DOCS · Documents updated in this slice

**The file set:**
- `docs/prd/modules/04-team/` (all files)
- `docs/decisions.md`
- `docs/roadmap/09-dojo-expansion.md`
- `docs/design/integrations-and-gating-requirements.md`
- `docs/spec/dojo/daemon-sync.md`
- `docs/spec/daemon/tracker-jira-linkage.md`
- `docs/spec/screen/workspace-integrations.md` and `workspace-tickets.md`
- `docs/backlog.md`
- the two tracker DDL files and `database/design.dbml`

**The rule.** In those files, every paragraph that says trackers are read-only, never
written, or that sync never writes is either:
- rewritten to the "Now" wording in "Overrides of parents", or
- given the canonical sentence.

Statements about CI, or about the dōjō never writing, stay as they are, but their
paragraph gains the spec reference. The shipped app copy changes as W-UI says. The `/api/projects/{id}/trackers`
path in the screen spec's Data invariants (:142) is inherited staleness, so it's
corrected in passing and noted as such.

### Build order

1. DDL and enums
2. Seams: `RawWrite`, shared retry loop, `map_response` empty body, factory
3. Owner lookup and backfill, `assigned_to_me`
4. Payload, render and hash (pure, unit-tested)
5. Comment surface
6. Fields surface and the mapping route
7. Toggle and status API
8. UI
9. W-DOCS

The X1–X8 fixture checks run against a real tenant before step 5 is merged.

### Out of scope

- team totals (Dōjō rollup, F-3)
- adapters other than Jira
- a per-ticket opt-out
- deleting or archiving sensei comments
- text-type custom fields
- writing `by_source`
- backfilling a reassigned ticket's history
- write-back after discovery
- a per-ticket write status on the Tickets screen
- a write budget shared across trackers (each pass is bounded by its own deadline and 429 policy)
- `list_cost_usd`, which isn't in R7 `Usage`

## External assumptions (Jira Cloud API: confirmed against a tenant fixture before build step 5)

| # | assumption | used by | if false (decided) |
|---|---|---|---|
| X1 | `/rest/api/3/myself` returns `accountId` and `accountType` (`atlassian` for a person) | W1 | `Unknown`, so nothing is written |
| X2 | a comment update accepts `notifyUsers=false` from a non-admin author | W5 | drop the parameter; W4's hash is the only guard |
| X3 | issue edit's `notifyUsers=false` needs admin, so it's never sent | W6 | stays omitted (decided; not revisited at build) |
| X4 | a field not on the edit screen → 400 | W9 | map the observed status to `field_invalid` |
| X5 | board issues return `fields.assignee.accountId` when `assignee` is requested | W1 | `assigned_to_me` false, so nothing is written |
| X6 | `/rest/api/3/field` lists custom fields with `id` and `schema.type` | W7 | the mapping PUT fails closed (`field_not_found`) |
| X7 | issue comments list `author.accountId`, and `orderBy=-created` is accepted | W5 | drop `orderBy` and scan the last page |
| X8 | the board issue endpoint accepts `jql=cf[N] is not EMPTY` | W7 | `field_in_use` check falls back to the `/rest/api/3/search/jql` endpoint scoped to the board's project |

## Done gate

Every gate runs against the fake transports (`FakeBoard` plus the recording write fake)
and the test DB unless marked *(tenant)*.

1. **DDL**: `information_schema.columns` shows the W11 columns and none named `*assignee*`.
   `tracker_fields` has both unique constraints. Each of the three enums equals its Rust
   `ALL` (unit test). A `trackers` row inserted without `writeback_enabled` has `true`.
2. **Owner, three ways**: `/myself` returning `{accountId:"A1",accountType:"atlassian"}`
   → `owner_ref = 'A1'`. `accountType:"app"` → `not_a_person` and
   `writeback.state = owner_not_a_person`. `{}` → `unknown` and `owner_unknown`.
3. **Backfill bounds**: a pre-slice tracker (`owner_ref` and `owner_state` NULL) whose
   `/myself` says `app` makes exactly **1** `/myself` request across two syncs. A failing
   `/myself` leaves the sync `Ok` and `last_error` NULL.
4. **Assignee only**: board with AUTH-1→A1, AUTH-2→`zz-sentinel-b2`, AUTH-3 unassigned, all linked.
   One pass sends write requests for AUTH-1 only, and `assigned_to_me` is `t/f/f`. Then
   reassign AUTH-1 to `zz-sentinel-b2` and sync: the next pass sends 0 requests.
5. **No identity stored or logged**: after gate 4, no row in any `sensei` table contains
   `zz-sentinel-b2` (`pg_dump --data-only -n sensei | grep -c zz-sentinel-b2` → 0). `format!("{:?}", task_row)` and
   `format!("{:?}", validated)` contain neither `A1` nor `zz-sentinel-b2`.
6. **Comment golden**: `render` of the W5 golden `Usage` equals the seven golden lines
   exactly (unit test).
7. **Comment lifecycle**:
   - The first pass makes 1 comment-list GET, then 1 POST with an ADF body whose text nodes
     are the rendered lines, and stores the id.
   - A second pass with no new sessions sends 0 requests.
   - A new linked session → exactly 1 PUT to `…/comment/{id}?notifyUsers=false`.
8. **Adopt, no duplicate**: if the list GET returns a comment by `A1` starting with
   `Sensei · AI usage`, the pass PUTs to that comment, doesn't POST, and stores its id.
9. **Null is not 0**: metered and API cost both `None` → `Cost: metered — · API —`. A
   session whose only messages are unpriced (`api_cost_n = 1`, `api_cost = None`) gives
   `API —`. On a mapped field the body has `null`.
10. **Change detection**: two `Usage`s differing only in `tokens_reasoning_n` (2 vs 3, same
    sum) give different comment hashes. With only `metered_cost` mapped, adding a cost-less
    session sends 0 requests.
11. **Lost sessions**: after the only repo leaves the workspace, the next pass sends 1
    write with `Sessions: 0` and every other value `—`, or `null` in mapped fields.
12. **Comment removed**:
    - A 404 on the comment PUT, then a 2xx on the issue check → `comment_removed`, and the
      next pass sends 0 requests for that task.
    - Toggle off and on → the next pass sends 1 list GET and 1 POST.
    - A 404 on both → `not_found`.
13. **GET-only stays GET-only** (replaces linkage gate 2):
    - `rg -n 'write_transport\(' crates/senseid/src` → only the trait method and impls in
      `tracker/runtime.rs`, and calls in `tracker/writeback.rs`.
    - `rg -n 'reqwest::Method|\.(post|put|patch|delete)\(' crates/senseid/src/tracker` →
      only the `RawWrite` impl in `transport.rs`.
    - `rg -n 'WriteMethod::' crates/senseid/src` → only `tracker/transport.rs` and
      `tracker/jira.rs`, plus their `#[cfg(test)]` modules.
    - `sync.rs`, `discover.rs` and `connect.rs` contain no `WriteTransport` and no
      `write_transport`.
14. **Isolation from Jira wording**: `rg -n -i 'jira|atlassian' crates/senseid/src/tracker/writeback.rs`
    outside `#[cfg(test)]` → no output.
15. **Fields path**: with `active_hours → customfield_10050`, a pass sends exactly
    `PUT /rest/api/3/issue/{id}` with body `{"fields":{"customfield_10050":2.05}}` for the
    golden usage, and no comment request.
16. **Mapping validation**:
    - Each W7 code is returned for its input, in the W7 order (a body with an unknown
      metric and a bad ref → `unknown_metric`).
    - `invalid_field_ref` makes 0 network requests.
    - A field with values on the board → `field_in_use`, and re-PUTting an already-mapped
      field doesn't make that check.
    - Success → 200, every task's hash NULL, and `field_invalid` cleared.
17. **Status table**: the fake returns each W9 outcome in turn: 401, 403, 400 on a field,
    400 on a comment, 404 on field/create, 429×4, 500, a 302 and a malformed id. Each yields
    the W9 status. Sticky ones send 0 requests on the next pass, and transient ones retry.
    Five 429 tasks in a row end the pass.
18. **Isolation from the sync**: a 403 on a write leaves `trackers.last_error` NULL and the
    sync `Ok`. That task is `forbidden` and the others are written.
19. **Redirect**: a 307 on a write is `error` with exactly 1 request to the base URL, and no
    request elsewhere (the `ReqwestRaw` test server records it).
20. **Mid-pass switch-off**: PATCH `writeback_enabled:false` after task 1 of 3 is written →
    tasks 2 and 3 get 0 requests. A disconnect mid-pass → 0 further requests.
21. **Lock and deadline**: a pass while a sync holds the guard → 0 requests and no error.
    A pass whose fake sleeps past `WRITEBACK_DEADLINE` stops, and unwritten tasks keep
    their prior status.
22. **Re-connect and re-key**: for both a re-connect and a **new** board on a site with an
    existing sibling board, the connected and sibling rows have
    `owner_ref`/`owner_state`/`writeback_error` reset as W11 says, and each of their tasks'
    `writeback_comment_ref`, hash, status and `assigned_to_me` are NULL.
22a. **Pass credential**: a keyring failure when building the write transport → no
    request, `writeback_error = credential_store`, `writeback.state = credential_failed`,
    and `last_error` still NULL. The next pass that gets a transport clears it.
22b. **GET unchanged**: an empty 200 on a GET is still `Decode` (the existing transport
    test is extended), and an empty 204 on a write is `Ok(Value::Null)`.
23. **Toggle and PATCH**: `{writeback_enabled:false}` → 200 and the next sync sends 0
    writes. `{"foo":1}`, `{}` and `{"writeback_enabled":"no"}` → 422 `invalid_body`.
24. **Guard**: PATCH and PUT `…/fields` with `Origin: https://evil.example` → 403, and with
    no Origin → allowed (the existing guard rule).
25. **UI** (component tests, `IntegrationsScreen.spec.svelte.ts`):
    - the sub-line equals the W-UI text
    - for each `writeback.state` the line has the W-UI text (`active` with `assigned: 2`
      renders "Writing to 2 tickets assigned to you.")
    - `TicketsScreen` renders the W-UI Tickets sub-line
    - the toggle sends `setTrackerWriteback` and reverts on error
    - a `field_in_use` response shows its text on the named row
26. **Docs**:
    - Over the W-DOCS file set, every paragraph that matches the read-only or no-write
      pattern also names this spec. This command → no output:
      `for f in <W-DOCS markdown files>; do awk -v RS= -v F="$f" 'tolower($0) ~ /read-only|never writ|writes? back|nothing is changed|never changes anything/ && $0 !~ /tracker-metrics-writeback/ {print F": "substr($0,1,80)}' "$f"; done`
    - `rg -n 'nothing is changed there|never changes anything in Jira' app/src` → no output.
    - Each superseded `decisions.md` row carries "superseded 2026-10-26".
27. *(tenant)* X1–X8 confirmed or their decided fallback applied, recorded under Evidence.

## Wrong gate

- On a board where the owner is the assignee of several tickets, **no** write request is
  sent at all. That's the sign `assigned_to_me` was derived from `trackers.account`, an
  email Jira's assignee doesn't carry.
- Every pass rewrites every comment, so the second-pass request count in gate 7 isn't 0:
  the hash included the "updated" time.
- A metric with no data is written as `0` or `$0.00`, from an `unwrap_or(0.0)` or a
  null decided by `_n`.
- The board card shows the sync as failed after only a write failed, because write
  failures went to `last_error`.
- A deleted comment is re-posted without the user turning write-back back on.
- A second comment appears on a ticket after a timeout on create: the lost-response
  adoption (gate 8) wasn't used.
- A mapping silently overwrites a field the team fills in, because `field_in_use` was skipped.
- The 429 loop exists twice and the write copy ignores `Retry-After`.
- An account id shows up in a log line, an API response or a `Debug` print.
- Sibling boards keep the old owner after a re-key and write to the new owner's tickets
  based on the old owner's assignments.

## Claims

Verified 2026-10-26 by `sensei-claims-verifier`, re-deriving each claim against `908140029`.
The rows marked *(rewrite)* were added by the rewrite and verified in a second pass.

| # | claim | check | actual | verdict |
|---|---|---|---|---|
| C1 | Jira `validate` calls `GET /rest/api/3/myself` and discards the body | `jira.rs` `validate` | jira.rs:119, result never bound | CONFIRMED |
| C2 | Tracker transport traits: `GetTransport` (1 method), `TransportFactory`, and the raw seam `RawGet` | `rg 'trait .*(Transport\|Raw)'` | mod.rs:96, runtime.rs:18, transport.rs:18 | CONFIRMED *(was MISLEADING: draft said "the only transport trait")* |
| C3 | No issue read requests `assignee`; there are 3 issue reads | `rg -i assignee crates/` | 0 hits; reads at jira.rs:162, 177, 320 | CONFIRMED *(draft said "both"; there are three)* |
| C4 | No DDL column holds an assignee or provider account id | `information_schema` / DDL columns | no such column (one comment mentions "assignee") | CONFIRMED *(the draft's check was wrong: the comment matches the grep)* |
| C5 | `trackers.account` is the Atlassian email | trackers.ddl comment; jira.rs:99 | as stated | CONFIRMED |
| C6 | `ticket_usages(pg, tracker_id, Some(ids))` returns per-task `Usage` with every W3 input, plus `shared`, `api_cost_partial` | usage.rs:19-55, 101-105 | as stated | CONFIRMED |
| C7 | `sync_stored` is the one path for tick, Sync now and first sync | tracker_sync.rs:50; trackers.rs:185, 211, 279 | as stated | CONFIRMED |
| C8 | `tracker_sync` runs every 1800 s | schedules.jsonl:16; live `sensei.schedules` | 1800 | CONFIRMED |
| C9 | `try_lock` → `None` when held; one instance shared by API and tick; the guard drops before `sync_stored` returns | sync.rs:31-40, 97; server.rs:562 | as stated | CONFIRMED |
| C10 | `map_response` makes a 2xx empty body a `Decode` error | transport.rs:40; scratch `from_slice(b"")` | Err | CONFIRMED |
| C11 | `sha2`, `hex` are `senseid` deps | Cargo.toml:101-102 | present | CONFIRMED |
| C12 | All tracker routes sit in `tracker_routes()` behind one `route_layer(local_caller_guard)` | routes.rs:47-65 | as stated | CONFIRMED |
| C13 | `…/trackers/{tracker}` has only `DELETE` | routes.rs:54 | as stated | CONFIRMED |
| C14 | Tracker SQL lives in `db::pg_store` (`trackers.rs` and `tracker_usage.rs`); none in `tracker/` outside tests | `rg sqlx crates/senseid/src/tracker`; tracker_usage.rs | as stated | CONFIRMED *(was FALSE: draft said all of it is in `trackers.rs`; W10 corrected)* |
| C15 | `tracker_error` labels are pinned to `TrackerError::ALL` by a test | mod.rs tests | as stated | CONFIRMED |
| C16 | Discovery builds `TaskRow`s with `task_row` and inserts via `apply_discovered` → `upsert_tasks`, the function the snapshot apply also uses | jira.rs:334; discover.rs:161, 204; trackers.rs:531, 847 | as stated | CONFIRMED |
| C17 | Sync failures write `last_error` (`record_failure`, `mark_tracker_error`); the success apply sets it NULL | sync.rs:220-247; runtime.rs:89; trackers.rs:604 | as stated | CONFIRMED *(was MISLEADING: draft said nothing else writes it)* |
| C18 | The screen sub-line says "Read-only: sensei never changes anything in Jira."; the app ships the same sentence | workspace-integrations.md:48; IntegrationsScreen.svelte:71 | as stated | CONFIRMED |
| C19 | `sessions.metered_cost` is "whole currency units", no currency named; `api_cost` is USD | sessions.ddl:134-135; model_prices.ddl:30 | as stated | CONFIRMED *(was MISLEADING: draft said metered_cost is USD; W3/W5 corrected)* |
| C20 | `sync`/`connect` core fns take `&dyn GetTransport`; `discover.rs::run_task` holds the whole `TrackerRuntime` (factory included) | discover.rs:241 | as stated | CONFIRMED *(was MISLEADING: draft said none holds a factory; gate 13 now checks `write_transport(` callers)* |
| C21 | `TransportFactory` has one method, with `FakeFactory` for tests | runtime.rs:18-25, 130-154 | as stated | CONFIRMED |
| C22 | A re-connect updates the board in place **and** re-keys every sibling board on the site to the new account and token | trackers.rs:962-968, 1005-1009 | as stated | CONFIRMED *(was MISLEADING: draft said single upsert; W11 now resets siblings)* |
| C23 *(rewrite)* | A session whose messages are all unpriced yields `api_cost = NULL` with `api_cost_n ≥ 1` | `message_api_cost.ddl`, `session_api_cost.ddl` `sum(c.cost)`; tracker_usage.rs:122-124 | sum of NULLs is NULL; row exists so n=1 | CONFIRMED |
| C24 *(rewrite)* | The board issue endpoint is already called with a `jql` parameter (R11) | jira.rs:313-323 `search_keys` | `{board_path}/issue` with `jql=key in (…)` | CONFIRMED |
| C25 *(rewrite)* | `ReqwestRaw::new` builds its client with `bounded_builder()` and `Policy::none()` | transport.rs:85-89 | as stated | CONFIRMED |
| C26 *(rewrite)* | Connect's statuses for a `TrackerError`: 401 `unauthorized`, 403 `forbidden`, keyring 500 `credential_store`, other 502 with code; `connect_failure` matches `ConnectError`, not `TrackerError` | api/handlers/trackers.rs:219-252; test `a_keyring_failure_is_500_credential_store` :833 | as stated | CONFIRMED *(was FALSE: draft said keyring → 503 and reuse of `connect_failure`; W7 step 7 corrected)* |
| C27 *(rewrite)* | The lines quoted in "Overrides of parents" exist with that wording | sed each cited line | all match; mod.rs doc comment is :94 | CONFIRMED *(was MISLEADING on completeness: the table read as exhaustive; now a canonical sentence + W-DOCS file-set rule + gate 26; gaps.feature path and mod.rs line corrected)* |
| C28 *(rewrite)* | `IntegrationsScreen.spec.svelte.ts` exists as the screen's component-test file | `find app/src -iname 'IntegrationsScreen*'` | present | CONFIRMED |
| C29 *(rewrite)* | The tracker app wrappers are `try*` methods on `senseiApi` in `app/src/lib/api.ts`; `app/src/lib/api-trackers.ts` does not exist (only its spec); `tracker-errors.ts` holds the code→words map | api.ts:756-785; `ls app/src/lib` | as stated | CONFIRMED *(was FALSE: draft placed wrappers in a non-existent `api-trackers.ts`; W-UI corrected)* |
| C30 *(rewrite)* | Changing `map_response` to accept an empty 2xx would alter two GET callers (`validate`'s `/myself`, the board-name read) | jira.rs:119, 152 | as stated | CONFIRMED *(was MISLEADING: draft said GET callers unaffected; W10 now scopes the empty-body rule to writes)* |
| C31 *(rewrite)* | The sibling re-key UPDATE runs after a first-time INSERT as well as after a re-connect UPDATE | pg_store/trackers.rs:960-1009 | as stated | CONFIRMED *(W11 and gate 22 now cover a new-board connect)* |
| C32 *(rewrite)* | A keyring failure inside the write-back pass would be recorded nowhere (the sync before it succeeded and cleared `last_error`) | runtime.rs:76-92; trackers.rs:604 | as stated | CONFIRMED *(was MISLEADING in W9: "the sync already recorded it"; now `writeback_error`)* |
| C33 *(rewrite)* | Discovery takes the same per-tracker guard as sync | discover.rs:97 | as stated | CONFIRMED |

## Related

- Analysis: [`analysis/2026-10-26-tracker-metrics-writeback.md`](../../analysis/2026-10-26-tracker-metrics-writeback.md)
- Parent: [`spec/daemon/tracker-jira-linkage.md`](tracker-jira-linkage.md)
- Screen: [`spec/screen/workspace-integrations.md`](../screen/workspace-integrations.md)
