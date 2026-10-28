---
name: Tracker metrics write-back
status: reviewed
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
| linkage spec Done gate 2 :717 and seam list :73 ("GET-only transport"); `tracker/mod.rs:94` doc comment; `tracker/transport.rs:1` module doc | no `post/put/…` call anywhere in `tracker/` | `GetTransport` stays GET-only, and sync, discover and connect only ever get one. Gate 2 is restated as this spec's gate 13. `mod.rs:94` stays true. `transport.rs:1` is reworded to "The shared GET transport, and the write seam that only `writeback.rs` uses: status → error mapping and the 429 policy over one-method raw HTTP seams". |
| `backlog.md` :106 Jira deviation (3) | no assignee synced or stored; no per-person figure | as the linkage row above |
| `database/ddl/table/sensei/{trackers,tracker_tasks}.ddl` comments and `database/design.dbml` :2781, :4135 | "Read-only mirror source"; "mirrored read-only … No assignee or other person data" | W11 wording, in both DDL and dbml |
| `screen/workspace-integrations.md` sub-line :46-48, wrong gate :172; `screen/workspace-tickets.md` sub-line :45-47, wrong gates :133-134; `app/…/integrations/IntegrationsScreen.svelte:71`; `app/…/tickets/TicketsScreen.svelte:40` | "Read-only: sensei never changes anything in Jira."; "Read from Jira; nothing is changed there."; "any write route called from this screen"; no per-person data shown | W-UI copy for both screens. The screen wrong gates gain: "`assigned_to_me` and the write-back state are not person data; showing them is allowed". The Tickets wrong gate :134 stays: the Tickets screen itself calls no write route. |
| `prd/modules/04-team/README.md` :22, :28-29; `prd/modules/04-team/05-releases-and-dora/README.md` :24-25 | "external systems (trackers, CI) are read-only"; "tracker APIs are read, never written" | the canonical sentence (CI stays read-only) |

## Resolved design

### W1 · Who writes

- **Owner lookup.** The adapter method `owner(t) -> Result<OwnerLookup, TrackerError>`
  returns one of three outcomes. For Jira it reads `GET /rest/api/3/myself`, the request
  `validate` already makes. The rules are applied **in order**, and the first match wins:
  1. `accountType` is a JSON string whose lowercased value isn't `"atlassian"` (e.g.
     `app`, `customer`) → `NotAPerson`. `accountId` is ignored.
  2. `accountType` lowercased is `"atlassian"` and `accountId` is a non-empty JSON string
     → `Person(AccountRef(accountId))`. The id is kept byte for byte, untrimmed.
  3. Anything else → `Unknown`. That covers a missing or non-string `accountType`, and
     `atlassian` with a missing, empty or non-string `accountId`.
  
  Only rule 1 is sticky (W1 backfill). Rule 3 is retried, so a real person is never
  shut out permanently.
- `validate` returns the lookup in `Validated.owner`. Connect stores it:
  - `Person(r)` → `owner_ref = r`, `owner_state = NULL`
  - `NotAPerson` → `owner_ref = NULL`, `owner_state = 'not_a_person'`
  - `Unknown` → `owner_ref = NULL`, `owner_state = 'unknown'`
- **Backfill.** Step 1 of the sync (after the guard, before the fetch) runs this only
  when `owner_ref IS NULL AND owner_state IS DISTINCT FROM 'not_a_person'`.
  - **Deadline.** The whole-sync deadline `until` is computed **before** this step.
    The lookup runs under `timeout_at(until, adapter.owner(t))`, so its 429 sleeps count
    against `SYNC_DEADLINE` and the sync stays bounded by it (C37). A timeout is treated
    as an `Err(Transport)`.
  - **Ok.** `set_tracker_owner(id, secret_service, &lookup)` stores the result as
    connect does and clears `owner_error`. It's one UPDATE
    `… WHERE id = $1 AND secret_service = $2 AND owner_ref IS NULL AND owner_state IS DISTINCT FROM 'not_a_person'`,
    bound to the `secret_service` the sync started with. If it touches 0 rows, a re-key
    or re-connect got there first and nothing is stored.
  - **Err.** `set_tracker_owner_error(id, secret_service, code)` sets
    `trackers.owner_error` to the `TrackerError` code, with the same `secret_service`
    condition. `last_error` and the sync's result are unchanged. The board shows the code
    (W8 `owner_lookup_failed`), so a lookup that fails every time can be seen without
    reading the logs.
  - **Store failure.** If either store fails, the error is logged with `tracker_id`
    only, with no `?`, and the sync carries on. `assigned_to_me` then stays NULL for
    this sync, because the apply reads the unchanged `owner_ref`.
  - The read sync never fails because of the lookup. A `not_a_person` tracker never asks
    again until a re-connect. A failing or `Unknown` lookup costs at most one `/myself`
    call per sync (up to `SYNC_RETRIES` + 1 with 429s), inside the deadline.
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

- **Spawned, not awaited.** `runtime::sync_stored` is the one path for the scheduled tick
  (`tracker_sync`, 1800 s), "Sync now" and the first sync. After its sync returns `Ok`, it
  calls `writeback::spawn_after_sync(pg, rt, tracker_id, adapter)`, which:
  - takes `rt.locks.try_lock(tracker_id)` synchronously (busy → no pass this round, no error)
  - `tokio::spawn`s the pass with the owned guard moved into it, and returns at once.

  So `sync_stored`, the "Sync now" response and the tick never wait for a pass. The tick's
  next tracker starts immediately (C40). A second "Sync now" during a pass gets 409 `busy`,
  because the guard is held. Discovery during a pass gets `Busy`, as it does against a sync.
  Discovery doesn't trigger a pass. The outcome is visible only through `GET …/trackers` and
  the task reads (W8).
- **`writeback::run(pg, io, tracker_id, secret_service, adapter, deadline, guard)`** is the
  pass itself; tests call it directly. `secret_service` is the token the sync ran on, and
  `io` (`PassIo`) carries the keyring and transport factory. It re-reads the tracker row
  **at pass start** for `owner_ref` and `writeback_enabled`, because the sync may have just
  backfilled the owner (W1).
  - It skips with no request when the tracker is gone, `writeback_enabled = false`, or
    `owner_ref IS NULL`.
  - It ends superseded when the row's token is no longer the sync's.
  - If the row read fails, it records `writeback_error = 'db'` under the sync's token.
- **Credential, once.** The pass reads the token once (bounded by `KEYRING_TIMEOUT`, as
  `transport_for` does). It builds both transports from it:
  - a GET transport, `SYNC_RETRIES`, for `find_comment` and `issue_exists`
  - a write transport, `WRITE_RETRIES = 3`

  If the read or either build fails, it sets `writeback_error` to the code and ends.
- **Every pass write is bound to the pass's token.** Each statement the pass makes carries
  `AND secret_service = $pass_service` on the tracker, through a join for task rows. That
  covers recording a write, recording a status, and setting or clearing `writeback_error`.
  A statement that touches 0 rows means a re-connect or re-key happened, so the pass ends
  **superseded** and writes nothing more (C22). So a write in flight during a re-connect can
  never land the old owner's comment id, hash or status on the reset row.
- **Deadline.** The whole pass runs under `WRITEBACK_DEADLINE = 5 min`, including every 429
  sleep. When it fires:
  - the task in flight is recorded `error` (its request may or may not have reached Jira;
    the next pass's `find_comment` adopts a comment that did land)
  - tasks not yet started keep their status
  - the pass sets `writeback_error = 'transport'`
- **Fair order.** Candidates are processed by `writeback_tried_at`, the time of the last
  attempt, landed or not: never-tried (NULL) first, then oldest attempt, then task id.
  Every recorded attempt sets it (`record_write_ok` and `record_write_status`). A ticket
  that keeps failing or being rate-limited therefore goes behind the others rather than
  starving them. (`writeback_at` records only successes, so ordering by it would put a
  failing ticket first again on every pass.)
- **Pass-level failure channel: `trackers.writeback_error`** (`tracker_error` codes). It is
  set when the pass can't run or ends early:
  - `token_missing` / `credential_store`: the credential
  - `db`: any pass read, or a record after a 2xx fails. The pass stops at the first record
    failure, so a write that landed but couldn't be recorded is never re-sent across the
    rest of the pass.
  - `rate_limited`: five consecutive `rate_limited` tasks
  - `transport`: the deadline

  A pass that gets both transports clears it first, under the token condition. It is the
  only writer of `writeback_error`, and `last_error` is never touched.
- **Before every task's write**, one read (`writeback_target(task_id, pass_service)`)
  re-checks four things:
  - the tracker still exists
  - `writeback_enabled` is true
  - `secret_service` is unchanged
  - the task is still `assigned_to_me`

  If any fails, the pass stops (tracker gone, disabled, re-keyed) or skips the task
  (reassigned). A switch-off or disconnect therefore takes effect before the next request.

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

- **Decimals** are rounded to 2 dp, half away from zero, with the division last, so an
  exact half stays exact (C41):
  - hours: `(secs * 100.0 / 3600.0).round() / 100.0`, so 3618 s gives `1.01` and 522 s
    gives `0.15`
  - costs: `(x * 100.0).round() / 100.0`
  - `-0.0` is normalised to `0.0`. A negative span (`cycle_hours` when `last_at < first_at`,
    by however little, or any negative duration sum) is **null**: hours elapsed are never
    negative. One `cycle_secs(&Usage)` computes it for both surfaces, testing the sign of
    the duration itself.

  The **canonical value** is the string `format!("{:.2}", rounded)`.
  **Integers** are written as JSON integers, with the canonical value `i64::to_string`.
  Both the hash and the request body use the canonical value (a field body parses the
  decimal string back into a JSON number).
- `by_source` isn't written. `shared` is shown in the comment only (W5). It isn't a mapped metric.
- **Mapped fields aren't additive across tickets.** A session linked to two tickets
  counts in full on both, the R7 rule. The comment states the overlap, and W-UI's Fields
  section says so under the editor.

### W4 · Which tasks, payload, change detection

- **Candidates for a pass** are the tracker's tasks with `assigned_to_me = true`,
  `removed_at IS NULL`, and either `Usage.sessions ≥ 1` or `writeback_at IS NOT NULL` (written
  before: a mapping change clears the hash, never `writeback_at`).
  The second case is a ticket that has lost all its sessions, for example its repo left
  the workspace. It is written as `sessions = 0` with every other metric null, so the
  ticket stops showing stale figures. The usage comes from one
  `ticket_usages(pg, tracker_id, Some(&ids))` call per pass. A candidate **absent from
  that map** has no candidate sessions, the genuinely empty set, so it takes
  `Usage::default()` (`sessions = 0`, every value `None`), as `backlog_usage` does (C42).
- **Sticky statuses are skipped.** A candidate is skipped while its `writeback_status` is
  `forbidden`, `not_found` or `field_invalid`, or `comment_removed` on the comment surface
  only. Those clear as W9 says. `rate_limited` and `error` retry on the next pass.
- **The payload, and its hash, is exactly what would be sent, minus the "updated" time:**
  - comment surface: the rendered `Report` lines (W5)
  - fields surface: the mapped `(field_ref, canonical value)` pairs in `tracker_metric`
    enum order
  
  `writeback_hash` is the lowercase hex SHA-256 (`sha2`, `hex`) of those strings joined
  with `\n`, prefixed by the surface name. On the fields surface each pair is the string
  `{field_ref}={canonical value}`, or `{field_ref}=null`. A change to an unmapped metric therefore sends
  nothing on the fields surface. A changed `_n`, `shared` or `api_cost_partial` that
  changes a comment line does send.
- When the hash equals the stored one **and the last write was `ok`**, the task is skipped
  and **no request is sent**. A failed write keeps the old hash, so a hash match alone
  doesn't mean the tracker shows it.
- After a write succeeds, `writeback_hash`, `writeback_at = now()` and
  `writeback_status = 'ok'` are set in one statement. After a failure, only
  `writeback_status` is set, so the hash stays as it was and a transient failure retries.
  Both are bound to the pass's token (W2).

### W5 · Surface: comment (no mappings)

- **Rendering.** The pure function `writeback::render(&Usage, updated_at) -> Report`
  produces `Report { title: String, lines: Vec<String> }`. The adapter turns a `Report`
  into the provider's format. For Jira that is an ADF `doc` with one `paragraph` (the title
  in `strong`) and one `bulletList` with a `listItem` per line. Every string goes in as a
  `text` node through `serde_json`, never by interpolation.
- **Golden example 1.** `sessions = 3`, `shared = 1`:
  - active 7380 s, `active_n = 3`, split Claude 4680 s (`claude_n = 3`) and you 2700 s
    (`user_n = 3`)
  - wall 14400 s, `wall_n = 3`
  - first-to-last 2 days
  - tokens fresh 120000, cache read 900000, cache write 30000, out 45000, each with `_n = 3`
  - reasoning `None` (`_n = 0`), metered `None` (`_n = 0`)
  - API 4.317, `api_cost_n = 3`, `api_cost_partial = 1`

  It renders exactly:
  ```
  Sensei · AI usage from the assignee's sessions (updated 2026-10-26 14:30 UTC)
  Sessions: 3 (1 shared with other tickets)
  Active time: 2.05 h (Claude 1.30 h, you 0.75 h)
  Wall-clock time: 4.00 h
  Cycle time: 48.00 h
  Tokens: fresh 120000 · cache read 900000 · cache write 30000 · out 45000 · reasoning —
  Cost: metered — · API $4.32 (partly priced for 1 session)
  ```
- **Golden example 2** (the coverage marks). `sessions = 3`, `shared = 0`:
  - active 7380 s (`_n = 3`), Claude 4680 s (`claude_n = 2`), you 2700 s (`user_n = 3`)
  - wall 14400 s, `wall_n = 1`
  - first-to-last 2 days
  - tokens fresh 120000 (`_n = 3`), cache read 900000 (`_n = 2`), cache write `None`,
    out 45000 (`_n = 3`), reasoning `None`
  - metered 1.5 (`_n = 1`)
  - API 30.0, `api_cost_n = 2`, `api_cost_partial = 2`

  It renders exactly:
  ```
  Sessions: 3
  Active time: 2.05 h (Claude 1.30 h [2 of 3 sessions], you 0.75 h)
  Wall-clock time: 4.00 h [1 of 3 sessions]
  Cycle time: 48.00 h
  Tokens: fresh 120000 · cache read 900000 [2 of 3 sessions] · cache write — · out 45000 · reasoning —
  Cost: metered 1.50 [1 of 3 sessions] · API $30.00 [2 of 3 sessions] (partly priced for 2 sessions)
  ```
  (The title line is as in example 1.)

  The rules:
  - A null value is `—`, with no mark or note after it.
  - A value whose `_n` satisfies `0 < _n < sessions` is followed by ` [N of M sessions]`
    (square brackets, so it never nests inside the active line's parentheses), right after
    that value.
  - ` (N shared with other tickets)` follows the session count when `shared > 0`.
  - The ` (partly priced for N session|sessions)` note follows the API value, after any
    coverage mark, when `api_cost_partial > 0` **and** `api_cost` is a value. An unpriced
    `API —` carries no note. "session" is used for N = 1, "sessions" otherwise.
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

- `sensei.tracker_fields (tracker_id, metric, field_ref, retired_at)`, unique on
  `(tracker_id, metric)` and on `(tracker_id, field_ref)`, FK → `trackers` on delete
  cascade. A row with a `metric` is **mapped**. A row with `metric` NULL is **retired**: a
  field sensei wrote before that is no longer mapped, kept so sensei can clear it.
  `retired_at` is when it was retired.
- When a tracker has ≥ 1 mapped row, the surface is **fields**: only mapped metrics are
  written, in **one** request per task, and no comment is created or edited. Otherwise
  the surface is **comment**.
- **Retired fields are cleared.** A ticket has a clear **pending** for a retired row
  (`clear_pending(task, retired_at)`) while nothing has landed on it since the field was
  retired and it was tried at all: its `writeback_at` is older than `retired_at`, or it
  is NULL and `writeback_tried_at` is not. A write that was tried may have landed even if
  it failed, and a failed clear stays pending, so it is retried. A pending ticket gets
  `null` written to that field on its next write:
  - on the fields surface, in the same request
  - on the comment surface, in one extra field write after the comment
  
  That cleanup is not part of the hash. A ticket with a field still to clear is written
  even when its hash matches. A failure of the extra field write is recorded as the
  ticket's status (`status_for(WriteFields, e)`), as any failed write is, except a 400.
- **A field the tracker refuses to clear** (`BadRequest`: deleted in Jira, say) is
  dropped, not a poison pill. On the comment surface the comment is recorded as written
  (`ok`, its id kept). On the fields surface a 400 on a request that carries clears is
  retried once with the mapped fields alone: if that lands, the ticket is `ok`; if not,
  its status is that retry's. When the ticket ends `ok`, the retired rows it carried are
  deleted (`delete_retired_fields`, under the token condition), and no later ticket
  tries them.
- Jira: `PUT /rest/api/3/issue/{id}` `{"fields": {"<field_ref>": <number|null>, …}}`. A
  null is JSON `null`, which clears the field. No `notifyUsers` parameter is sent (X3). A
  2xx (Jira: 204) is success. A task id that fails the id shape is `Decode` (→ `error`)
  before any request, so a real Jira 400 stays the only `field_invalid`.
- Switching comment → fields leaves an existing comment untouched. Switching fields →
  comment edits the same `writeback_comment_ref` again (W7 nulls the hashes, so it is
  rewritten) and retires every field, so each one is cleared.

### W7 · Field mapping

sensei never creates a Jira field: that, and putting it on the board's screens, needs Jira
admin rights. A Jira admin can run `scripts/jira-create-sensei-fields.py` once per board.
It finds or creates one Number field per metric ("sensei <label>", with a global
context), adds the fields to the edit and view screens of the board's company-managed
projects, and with `--sensei` saves the mapping through the PUT below. A field missing
from the edit screen is refused with a 400, which is `field_invalid` (W4).

- `PUT /api/workspaces/{id}/trackers/{tracker}/fields` with body
  `{"fields": [{"metric": "...", "field_ref": "..."}…]}` **replaces** the mapped set, and
  `[]` clears it. The body must be JSON; a non-JSON body gets axum's own 400/415. It's
  accepted in every write-back state (disabled included). The checks run in this order,
  each over the whole body, stopping at the first failure:
  1. The body isn't an object with a `fields` array of `{metric, field_ref}` strings → 422
     `invalid_body`.
  2. Unknown metric → 422 `unknown_metric` `{metric}`.
  3. A metric or `field_ref` appears twice → 422 `duplicate_mapping` `{metric|field_ref}`
     (the second occurrence's value).
  4. Unknown workspace or tracker → 404 (`workspace_not_found` / `tracker_not_found`). This
     comes first because the tracker's adapter owns the field-id shape; it is a database
     read, not a network call.
  5. `field_ref` fails `adapter.field_ref_shape` (Jira: `^customfield_[1-9][0-9]{0,9}$`)
     → 422 `invalid_field_ref` `{field_ref}`. No network request has been made yet.
     Then the Save takes the board's lock (`TrackerLocks`, the one sync and the pass
     hold) until it has replaced the set, or answers 409 `busy`. A mapping never changes
     under a running pass, and two Saves never race.
  6. `adapter.check_fields(t, refs) -> Result<Vec<FieldCheck>, TrackerError>`, where
     `FieldCheck { field_ref, exists: bool, numeric: bool }`. Jira does this with one
     `GET /rest/api/3/field` and checks `schema.type == "number"`. Missing → 422
     `field_not_found` `{field_ref}`. Not numeric → 422 `field_not_number` `{field_ref}`.
  7. **Fields already in use.** For each `field_ref` whose values aren't sensei's own, `adapter.field_in_use(t, board, field_ref) -> Result<bool, TrackerError>`.
     Jira: the board issue endpoint with `jql=cf[N] is not EMPTY&maxResults=1&fields=none`.
     `true` → 422 `field_in_use` `{field_ref}`, so a mapping can't take over Story Points or
     any field people fill in. A field's values are sensei's own while it is mapped, or
     while it is retired and some ticket on the board (assigned or not) still has a clear
     pending for it. Such a field is never refused. A retired field with no pending clear
     holds no value of sensei's, so it is checked like a new one.
  8. Transport and credential failures map through `fn tracker_error_status(TrackerError)
     -> StatusCode`: `Unauthorized` → 401 `unauthorized`, `Forbidden` → 403 `forbidden`,
     `TokenMissing`/`CredentialStore` (the route's own keyring read, `transport_for`) → 500
     with their code, and any other → 502 with its code. The checks use a transport with
     **0 retries**, as connect does, so Save never waits out a 429. Connect's
     `Validate(Tracker(e))` arm uses the same function; the keyring codes can't reach it,
     so connect's answers don't change.
- On success, one transaction:
  - replaces the mapped set: a field no longer mapped is **retired** (`metric` NULL,
    `retired_at = now()`); a retired field mapped again is un-retired
  - sets `writeback_hash = NULL` on every task of the tracker, and clears `field_invalid`
  
  The route returns 200 with the `GET …/fields` body.
- `GET …/fields` → 200 `{"fields": [{metric, field_ref}]}`: the mapped rows only, in enum order.
- Whether a field is on an issue's edit screen can't be checked in advance. A write
  rejected with 400 is `field_invalid` (X4).

### W8 · Toggle, status, API

- `trackers.writeback_enabled boolean not null default true` applies to every row,
  including trackers connected before this slice. The user confirmed this on 2026-10-26
  after being shown that existing boards were connected under the "Read-only" copy.
- `PATCH /api/workspaces/{id}/trackers/{tracker}`: the body must be exactly
  `{"writeback_enabled": <bool>}`. Anything else (missing, non-bool, extra key) → 422
  `invalid_body`. It returns 200 with the tracker in the `GET …/trackers` element shape.
  Going false → true clears `comment_removed`, `forbidden`, `not_found` and `field_invalid` for that
  tracker's tasks, and with `comment_removed` it also sets `writeback_comment_ref = NULL`
  and `writeback_hash = NULL`. The next pass therefore posts a new comment.
- `GET …/trackers`: each tracker gains
  `"writeback": {"enabled", "state", "surface": "comment"|"fields", "mapped": <n>, "assigned": <n>, "failing": <n>, "error": <code|null>}`:
  - `mapped`: the number of mapped `tracker_fields` rows; `surface` is `fields` when it is > 0
  - `assigned`: live tasks (`removed_at IS NULL`) with `assigned_to_me = true`
  - `failing`: the subset of `assigned` whose `writeback_status` is set and isn't `ok`.
    Never-written (NULL) and reassigned or removed tickets don't count, so
    `failing ≤ assigned`.

  Both come from one `PgStore::writeback_counts(tracker_id) -> (assigned, failing)`.
  `state` is the first of these that applies:
  1. `disabled`
  2. `pass_failed` (`writeback_error` not NULL)
  3. `owner_not_a_person` (`owner_state = not_a_person`)
  4. `owner_lookup_failed` (`owner_ref` NULL and `owner_error` not NULL; `error` carries the code)
  5. `owner_unknown` (`owner_ref` NULL otherwise)
  6. `nothing_assigned` (`assigned = 0`)
  7. `active`
  
  `error` is `writeback_error` when the state is `pass_failed`, `owner_error` when
  it's `owner_lookup_failed`, and otherwise `null`.
- `GET …/tasks` and `GET …/tracker-tasks/{key}`: each task gains `assigned_to_me`,
  `writeback_status`, `writeback_at`.
  The linkage spec's route table (R8) and the app's `TrackerTaskRow` / `TrackerTaskDetail`
  types in `app/src/lib/types.ts` list every field the routes return. Each feature that adds
  a field updates both (F1 `assigned_to_me`; F2 `writeback_status`, `writeback_at`; F2/F3 the
  `writeback` object on `GET …/trackers`).
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
| on the W5 comment-list read (`find_comment`): `Unauthorized`/`Forbidden` → `forbidden`, `NotFound` → `not_found`, any other → `error` | as named | as named |
| the W5 404 check finds the issue | `comment_removed` | toggle on, re-connect (both clear the ref) |
| `RateLimited` after `WRITE_RETRIES` | `rate_limited` | not sticky |
| `BadRequest` on a comment write, `HttpError`, `Transport`, `Decode`, `Redirect`, the task in flight at the deadline, an id failing W10's shape check | `error` | not sticky |
| a pass-level failure: credential, a failed read or record (`Db`), five `rate_limited` in a row, the deadline | no extra task status; `trackers.writeback_error` = the code (W2) | cleared by the next pass that gets both transports |
| an update's `NotFound` never stands alone: the issue check above decides it | (`status_for(UpdateComment, NotFound)` is `error`, unreachable from the pass) | n/a |
| `Db`, `TokenMissing`, `CredentialStore`, `UnsupportedTracker` from a single request | `error` | not sticky |

`status_for(op, TrackerError) -> WritebackStatus` is total. `op` is one of `FindComment`,
`CreateComment`, `UpdateComment`, `IssueCheck` or `WriteFields`, and its test iterates every
`op` × `TrackerError::ALL` with no wildcard arm.

- A write failure **never** writes `trackers.last_error` or changes the sync's result. The
  success apply clears `last_error` before the pass runs (C17). One task's failure doesn't
  stop the others.
- Five consecutive `rate_limited` results end the pass (W2: `writeback_error = 'rate_limited'`).
- No response body, URL, account id or database error text is stored or logged. A log
  line carries only `tracker_id`, `task_id` and a fixed status or error code.

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

- `PolicyTransport<R>` loses its `R: RawGet` struct bound. The bound moves onto the impls,
  so a write-only `RawWrite` fits (C43). It implements `WriteTransport` for `R: RawWrite`. The 429 loop
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
  and `FakeFactory` gains a recording fake. That fake is a `RawWrite` with scripted
  responses and a call log, wrapped in the real `PolicyTransport` with the `retries` it is
  given, so the 429 loop and `Retry-After` are exercised and `WRITE_RETRIES` is asserted.
  `MemoryStore` gains a per-call `get` failure injection (gate 22a). `write_transport(` is
  called **only** from `tracker/writeback.rs`.
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
  `Decode`, mapped to `error`, without sending a request, so a real 400 stays the only
  `field_invalid`.
- `TrackerRow.owner_ref` becomes `Option<AccountRef>`, so a `{:?}` of the row the pass
  carries can't print the owner's id (gate 5).
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
  `owner_error tracker_error` (F1), plus `writeback_enabled boolean not null default true`
  and `writeback_error tracker_error` (F2). `TrackerRow` is read through a
  struct with a by-name `FromRow` impl (no `sqlx` derive crate is a dependency), not the positional `TrackerTuple`. With these five columns
  the tuple would reach 19 fields, past sqlx 0.8's 16-field `FromRow` tuple limit (C38). The comment's "Read-only mirror source
  for …" becomes "Mirror source for …; writes back usage metrics only, to tickets
  assigned to the token owner (spec tracker-metrics-writeback)".
- `tracker_tasks` gains `assigned_to_me boolean`, `writeback_hash text`,
  `writeback_comment_ref text`, `writeback_status tracker_writeback_status`,
  `writeback_at timestamptz` (last success) and `writeback_tried_at timestamptz` (last
  attempt, the pass order). The header line and table comment change "mirrored
  read-only" to "mirrored from the tracker", and "No assignee or other person data, by
  design" to "No assignee identity: only whether the token owner is the assignee".
- `database/design.dbml` gets the same comments, columns and table.
- **Re-key and re-connect.** `connect_tracker` writes the connected board (an UPDATE on
  re-connect, an INSERT on a new board) and then, on **every** connect, re-keys each
  sibling board on the same site onto the new token (C22, C31). On the connected board
  and every re-keyed sibling it:
  - sets `owner_ref` and `owner_state` (the connected board from its `validate`; siblings
    to NULL, so their next sync backfills) and `owner_error = NULL`, `writeback_error = NULL`
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
    - `pass_failed`, by `error` code:
      - `token_missing` / `credential_store`: "Couldn't read the Jira token to write back. Replace the token."
      - `db`: "Couldn't record what was written. Write-back retries on each sync."
      - `rate_limited`: "Jira is rate-limiting writes. Write-back resumes on the next sync."
      - `transport`: "Writing didn't finish in time. Write-back resumes on the next sync."
    - `owner_not_a_person`: "This token belongs to an app account, so sensei can't tell
      which tickets are yours. Nothing is written."
    - `owner_lookup_failed`: "Couldn't check which tickets are yours: <error text>.
      Retrying on each sync." The `<error text>` is `tracker-errors.ts`'s words for the code.
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

    These texts are their own map in `tracker-errors.ts` (`FIELD_MAPPING_ERRORS`), apart from
    `WORDS`, where `invalid_body` reads "sensei refused that change." (the PATCH). A failure
    that names no row (401/403/500/502, `tracker_not_found`) shows as the card alert with
    `errorFor`'s text. A duplicate marks the second row.
- **Per-ticket write status** on the Tickets screen stays out of this slice. The state
  line's `failing` count is the only aggregate.
- **Scheduled tasks**: the `tracker_sync` description (`api/handlers/scheduled_tasks.rs:63`)
  becomes "Tracker sync — mirrors each connected Jira board's sprints and tickets, writes
  usage metrics back to tickets assigned to you, and drains removed credentials".
- **Tickets screen sub-line** (`TicketsScreen.svelte:40`, `workspace-tickets.md:45-47`):
  "… Read from Jira. On tickets assigned to you, sensei writes these numbers back, as set
  on Integrations."
- **App wrappers** go on `senseiApi` in `app/src/lib/api.ts`, beside `tryListTrackers`
  and the others, following their `try*` convention (C29): `trySetTrackerWriteback`,
  `tryGetTrackerFields`, `tryPutTrackerFields`. They're tested in
  `app/src/lib/api-trackers.spec.ts`. The W7/W8 codes and the `pass_failed` texts are added to
  `tracker-errors.ts`.
- **Disconnect copy** stays "Jira itself is not changed." It's still true: disconnect never
  deletes what was written.

### W-DOCS · Documents updated in this slice

**The file set:**
- `docs/prd/modules/04-team/` (every `.md` and `.feature`)
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
| X8 | the board issue endpoint accepts `jql=cf[N] is not EMPTY` | W7 | `field_in_use` check falls back to the `/rest/api/3/search/jql` endpoint scoped to the board's project; a board spanning several projects (no single key) fails closed: the mapping is refused with the tracker error |

## Done gate

Every gate runs against the fake transports (`FakeBoard` plus the recording write fake)
and the test DB unless marked *(tenant)*.

1. **DDL**: `information_schema.columns` shows the W11 columns and none named `*assignee*`.
   `tracker_fields` has both unique constraints. Each of the three enums equals its Rust
   `ALL` (unit test). A `trackers` row inserted without `writeback_enabled` has `true`.
2. **Owner, three ways.**
   - Unit table over the W1 rules:
     - `{accountType:"atlassian",accountId:"A1"}` → `Person("A1")`
     - `{accountType:"app"}` (no `accountId`) and `{accountType:"customer",accountId:"C1"}` → `NotAPerson`
     - `{}`, `{accountId:"A1"}`, `{accountType:"atlassian"}`,
       `{accountType:"atlassian",accountId:""}`, `{accountType:"atlassian",accountId:123}`
       and `{accountType:null,accountId:"A1"}` → `Unknown`
     - `{accountType:"Atlassian",accountId:"A1"}` → `Person("A1")`
   - Stored on connect: `owner_ref = 'A1'`; `not_a_person`; `unknown`.
   - *(F2)* `writeback.state` is `owner_not_a_person` and `owner_unknown` for the last two.
3. **Backfill bounds**:
   - A pre-slice tracker (`owner_ref` and `owner_state` NULL) whose `/myself` says `app`
     makes exactly **1** `/myself` request across two syncs.
   - A `/myself` that 403s every time leaves each sync `Ok`, `last_error` NULL and
     `owner_error = 'forbidden'`. *(F2)* `writeback.state = owner_lookup_failed` with
     `error: "forbidden"`.
   - A later `/myself` that succeeds clears `owner_error`.
3a. **Backfill under re-key**: `/myself` returns `A1`, but an `on_call` hook on `/myself`
   re-connects the board with a new token mid-call. Afterwards `owner_ref` isn't `'A1'`;
   it's NULL, or the new connect's owner.
3b. **Backfill inside the deadline** (`start_paused`): `/myself` answers 429 with
   `Retry-After: 60` four times, and the fetch never resolves. The sync returns within
   `SYNC_DEADLINE` of its start.
3c. **Store failure**: if `set_tracker_owner` fails (a store fake or a revoked
   privilege), the sync returns `Ok` and the tasks' `assigned_to_me` is NULL.
4. **Assignee only**: board with AUTH-1→A1, AUTH-2→`zz-sentinel-b2`, AUTH-3 unassigned, all linked.
   The fake board strips `fields.assignee` from its responses unless the request's
   `fields` query names `assignee`. That way a read that forgets to ask gives `f/f/f`
   and fails, for both the snapshot read and the discovery (`search_keys`) read.
   One pass sends write requests for AUTH-1 only, and `assigned_to_me` is `t/f/f`. Then
   reassign AUTH-1 to `zz-sentinel-b2` and sync: the next pass sends 0 requests.
5. **No identity stored or logged**: after gate 4, no row in any `sensei` table contains
   `zz-sentinel-b2` (`pg_dump --data-only -n sensei | grep -c zz-sentinel-b2` → 0). `format!("{:?}", task_row)` and
   `format!("{:?}", validated)` contain neither `A1` nor `zz-sentinel-b2`. *(F2)* Neither
   does `format!("{:?}", tracker_row)`.
6. **Comment golden**: `render` of each W5 golden `Usage` equals its lines exactly (unit
   tests). Unit tests also cover:
   - `canonical` for 3618 s → `1.01`, 522 s → `0.15`
   - a negative cycle → `—`
   - `-0.0` → `0.00`
7. **Comment lifecycle**:
   - The first pass makes 1 comment-list GET, then 1 POST with an ADF body whose text nodes
     are the rendered lines, and stores the id.
   - A second pass with no new sessions sends 0 requests.
   - A new linked session → exactly 1 PUT to `…/comment/{id}?notifyUsers=false`.
8. **Adopt, no duplicate**: if the list GET returns a comment by `A1` starting with
   `Sensei · AI usage`, the pass PUTs to that comment, doesn't POST, and stores its id.
9. **Null is not 0**: metered and API cost both `None` → `Cost: metered — · API —`. A
   session whose only messages are unpriced (`api_cost_n = 1`, `api_cost = None`,
   `api_cost_partial = 1`) gives exactly `Cost: metered — · API —`, with no note.
   *(F4)* On a mapped field the body has `null`.
10. **Change detection**: two `Usage`s differing only in `tokens_reasoning_n` (2 vs 3, same
    sum) give different comment hashes. With only `metered_cost` mapped, adding a cost-less
    session sends 0 requests.
11. **Lost sessions**: after the only repo leaves the workspace, the next pass sends 1
    write with `Sessions: 0` and every other value `—`. *(F4)* On mapped fields, `null`.
12. **Comment removed**:
    - A 404 on the comment PUT, then a 2xx on the issue check → `comment_removed`, and the
      next pass sends 0 requests for that task.
    - Toggle off and on → the next pass sends 1 list GET and 1 POST.
    - A 404 on both → `not_found`.
13. **GET-only stays GET-only** (replaces linkage gate 2):
    - `rg -n 'write_transport\(' crates/senseid/src` → only the trait method and impls in
      `tracker/runtime.rs`, and calls in `tracker/writeback.rs`.
    - `rg -n -U 'client\s*\.\s*(post|put|patch|delete|request)\(|reqwest::Method' crates/senseid/src/tracker`
      → only the `RawWrite` impl in `transport.rs`. `-U` and `\s*` catch a call rustfmt
      splits across lines. The keyring `s.delete(` at connect.rs:177 doesn't match, because
      it isn't a `client.` call.
    - `for f in sync discover connect; do sed -E '/^(pub(\(crate\))? )?mod tests/q' crates/senseid/src/tracker/$f.rs | rg -n 'WriteMethod|WriteTransport|write_transport'; done; rg -n 'WriteMethod|WriteTransport|write_transport' crates/senseid/src/api`
      → no output. These names are forbidden in the production code of sync, discover and
      connect and in the API. Their test modules implement the adapter trait, so they
      name `WriteTransport`.
14. **Isolation from Jira wording**:
    `sed '/#\[cfg(test)\]/q' crates/senseid/src/tracker/writeback.rs | rg -n -i 'jira|atlassian'`
    → no output. The pass's tests may use `JiraAdapter`.
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
    - After a PUT of n rows, `GET …/trackers` shows `surface: "fields"` and `mapped: n`;
      after `[]`, `comment` and `0`.
    - A field unmapped after sensei wrote it is retired, its next write sends it `null`,
      and mapping it again is accepted (no `field_in_use`).
    - A 429 on `/rest/api/3/field` → 502 `rate_limited` after exactly 1 request.
17. **Status table**:
    - Unit: `status_for` over every `op` × `TrackerError::ALL`.
    - Pass, comment rows *(F2)*, fake returns each in turn:
      - 401, 403, 400 on a comment, 404 on create, 500, a 302, a malformed id
      - 429×4, which is exactly 4 raw sends, sleeping `Retry-After`, with `WRITE_RETRIES`
      - a 403 / 404 on the comment-list read

      Each yields the W9 status. Sticky ones send 0 requests on the next pass; transient
      ones retry. Five 429 tasks in a row end the pass with `writeback_error = 'rate_limited'`.
    - *(F4)* The field rows (400 on a field, 404 on a field write).
18. **Isolation from the sync**: a 403 on a write leaves `trackers.last_error` NULL and the
    sync `Ok`. That task is `forbidden` and the others are written.
19. **Redirect**: a 307 on a write is `error` with exactly 1 request to the base URL, and no
    request elsewhere (the `ReqwestRaw` test server records it).
20. **Mid-pass switch-off**: `writeback_enabled` set false after task 1 of 3 is written
    (F2 sets the column directly; F3 repeats it through PATCH) → tasks 2 and 3 get 0
    requests. A disconnect mid-pass → 0 further requests.
20a. **Re-connect mid-write**: a hook on the fake's POST re-connects the board with a new
    token. Afterwards the task's `writeback_comment_ref`, hash and status are all NULL, and
    `writeback_error` is NULL. The pass recorded nothing and ended superseded.
20b. **Record failure**: with the record refused (a trigger in the test's database) and 2
    candidates, the pass sends exactly 1 write. `writeback_error = 'db'` is set if the
    database allows, and `writeback.state = pass_failed`.
20c. **Spawned**: `sync_stored`, with a write fake that sleeps 4 min (paused clock), returns
    in under 1 s of virtual time, and a second sync of the same tracker during the pass is
    `Busy`. (The tick awaits only `sync_stored`, so this also shows it never waits for a
    pass before syncing the next tracker.)
20d. **Backfilled owner writes in the same sync**: a tracker with no owner whose `/myself`
    gives `A1`, with AUTH-1 assigned to A1 → one `sync_stored` leads to exactly 1 comment POST.
21. **Lock, deadline and order**:
    - A pass while a sync holds the guard → 0 requests and no error.
    - A pass whose fake sleeps past `WRITEBACK_DEADLINE` on task 1 of 3: task 1 `error`;
      tasks 2 and 3 keep their prior status; `writeback_error = 'transport'`.
    - The next pass starts with task 2 or 3, oldest `writeback_at` first.
21a. **Counts**: with A `ok` + assigned, B NULL + assigned, C `forbidden` + reassigned, and
    D `error` + assigned + removed → `assigned = 2`, `failing = 0`. Set A to `error` →
    `failing = 1`.
22. **Re-connect and re-key**: for both a re-connect and a **new** board on a site with an
    existing sibling board, the connected and sibling rows have
    `owner_ref`/`owner_state`/`writeback_error` reset as W11 says, and each of their tasks'
    `writeback_comment_ref`, hash, status and `assigned_to_me` are NULL.
22a. **Pass credential**: `MemoryStore` fails only the pass's `get` → no request,
    `writeback_error = credential_store`, `writeback.state = pass_failed`, and `last_error`
    still NULL. The next pass that gets both transports clears it.
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
    - the toggle calls `senseiApi.trySetTrackerWriteback` and reverts on error
    - a `field_in_use` response shows its text on the named row
26. **Docs**:
    - `scripts/check-tracker-readonly-docs.sh <W-DOCS file set>` → no output. The build
      adds it (plan task) and it works like this:
      - **Unit:** each Markdown table row (a line starting with `|`) on its own, and
        otherwise each blank-line-delimited paragraph. Checking per row means one
        reference elsewhere in a table can't excuse a stale row.
      - **Flagged when** the unit, lowercased, matches
        `read-only|read only|never writ|writes? back|no write path|nothing is changed|never changes anything|never written`,
        **and** a tracker context
        `tracker|jira|adapter|azure board|github issues|export|sync never|sync is|sync only`,
        **and** doesn't contain `tracker-metrics-writeback`.
      - **Baseline:** prototyped on `d9b708894`, it reports **39** units over the
        Markdown and `.feature` files of the set. Billing "read-only members" and the
        workspace-viewer paragraphs aren't flagged, and the roadmap D3 row is flagged
        on its own.
    - `rg -n 'Read-only mirror|mirrored read-only' database/ddl/table/sensei/tracker*.ddl database/design.dbml`
      → no output.
    - `rg -n '\(read-only\)' crates/senseid/src/api/handlers/scheduled_tasks.rs` → no output.
    - `rg -n -g '!*.spec.*' 'nothing is changed there|never changes anything in Jira' app/src`
      → no output. (The screen specs assert that the old copy is absent.)
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
| C13 | `…/trackers/{tracker}` has only `DELETE` | routes.rs:54 | as stated | CONFIRMED *(F3 adds PATCH, W8)* |
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
| C34 *(rewrite 3)* | Gate 13's two commands give no output on today's tree; the keyring `s.delete(` (connect.rs:177) isn't an HTTP call | ran both at `64f10fe46` | no output | CONFIRMED *(was FALSE: the earlier pattern `\.(post\|put\|patch\|delete)\(` matched connect.rs:177)* |
| C35 *(rewrite 3)* | Today `Read-only mirror\|mirrored read-only` appears in both tracker DDL files and design.dbml, and `scheduled_tasks.rs:63` says "(read-only)" | rg at `64f10fe46` | tracker_tasks.ddl 2, trackers.ddl 1, design.dbml 3; scheduled_tasks.rs:63 | CONFIRMED |
| C36 *(rewrite 3)* | The docs check as defined in gate 26 reports 39 units on the set's Markdown and `.feature` files, with no billing or viewer paragraph | scratch prototype run at `d9b708894` | 39; 0 billing/viewer | CONFIRMED |
| C37 *(build F1)* | `until` is computed after `mark_tracker_attempt`, so a step-1 call made before it isn't covered by `SYNC_DEADLINE`; the 429 loop sleeps up to `MAX_RETRY_AFTER` = 60 s per retry with `SYNC_RETRIES` = 3 | sync.rs:103-111; transport.rs 429 loop; http.rs; runtime.rs:15 | as stated | CONFIRMED *(found by the F1 failure-mode pre-audit; W1 now starts the deadline first)* |
| C38 *(build F1)* | `TrackerTuple` has 14 fields; sqlx-core 0.8.6 implements `FromRow` for tuples up to 16 | trackers.rs:130-145; sqlx-core-0.8.6 `from_row.rs` | as stated | CONFIRMED *(W11 switches `TrackerRow` to a derived struct)* |
| C39 *(build F1)* | `FakeBoard::get_json` ignores the `fields` query; `FakeBoard::standard`'s `/myself` has no `accountType` | tracker/mod.rs:519, 540-573 | as stated | CONFIRMED *(gate 4's fake now honours `fields`; the standard fixture gains `accountType`)* |
| C40 *(build F2)* | `tracker_sync` loops trackers sequentially and "Sync now" awaits `sync_current` inline, so a pass awaited inside `sync_stored` would delay both | tracker_sync.rs:49-50; api/handlers/trackers.rs:211, 279 | as stated | CONFIRMED *(F2 pre-audit; W2 now spawns the pass)* |
| C41 *(build F2)* | `(x*100.0).round()` rounds exact decimal halves of an hour down (e.g. 3618 s gives 1.00); `secs*100.0/3600.0` keeps them exact | scratch Rust run (F2 pre-audit): 135 of 2000 half-hundredth durations differ | as stated | CONFIRMED *(W3 now divides last)* |
| C42 *(build F2)* | `ticket_usages` has no entry for a task with no sessions; `backlog_usage` uses `unwrap_or_default()` for the genuinely empty set | usage.rs:228-229, 560 | as stated | CONFIRMED *(W4 maps absent to `Usage::default()`)* |
| C43 *(build F2)* | `PolicyTransport` is declared `struct PolicyTransport<R: RawGet>` with `impl<R: RawGet>` | transport.rs:25, 30 | as stated | CONFIRMED *(W10 moves the bound to the impls)* |
| C44 *(build F2)* | `MemoryStore::get` can only fail with `TokenMissing`; it has no get-failure injection | secret.rs:124-131 | as stated | CONFIRMED *(W10 adds one)* |
| C45 *(build F2)* | No `PgStore` method counts a tracker's assigned or failing tasks (`tracker_counts` returns tasks and sprints) | pg_store/trackers.rs `tracker_counts` | as stated | CONFIRMED *(W8 adds `writeback_counts`)* |

## Related

- Analysis: [`analysis/2026-10-26-tracker-metrics-writeback.md`](../../analysis/2026-10-26-tracker-metrics-writeback.md)
- Parent: [`spec/daemon/tracker-jira-linkage.md`](tracker-jira-linkage.md)
- Screen: [`spec/screen/workspace-integrations.md`](../screen/workspace-integrations.md)
