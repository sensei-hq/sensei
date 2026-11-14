---
name: Comment write-back for GitLab and GitHub
status: reviewed (r3)
date: 2026-11-13
parent: docs/spec/daemon/tracker-metrics-writeback.md (W1, W5, W7, W8, W9, W10, "Out of scope")
analysis: docs/analysis/2026-11-13-forge-comment-writeback.md
also: docs/spec/daemon/tracker-gitlab.md (§G8), docs/spec/daemon/tracker-github.md (capabilities, token, §H6), docs/spec/daemon/tracker-jira-linkage.md (R8, Future adapters)
---

# Comment write-back for GitLab and GitHub

## Purpose

Write-back (D50) writes figures to tickets only on Jira. GitLab and GitHub trackers report
`writeback: false` and refuse the toggle with 409 `writeback_unsupported`. This spec gives
both forges the **comment surface** (parent W5): one sensei comment per ticket assigned to
the token owner, created once and then edited in place. Every tracker kind then supports at
least the comment (user, 2026-11-13). GitLab and GitHub ship in **one slice**.

Mapped fields (W6, W7) stay **Jira-only**.

Forge issues are often public, and Jira tickets are not. A sensei comment carries the
assignee's AI cost, tokens and time, so it is **never written to a ticket in a public or
internal repository or project** (F7, user, 2026-11-13).

| Overrides | Was | Now |
|---|---|---|
| parent "Out of scope" `:686` | "adapters other than Jira" | "mapped fields on adapters other than Jira" |
| parent W7 step 4 `:387-389` | `capabilities().writeback = false` → 409 `writeback_unsupported` | `capabilities().fields = false` → 409 `fields_unsupported` (F1) |
| parent W8 `:431-433` | `capabilities().writeback = false` → 409 `writeback_unsupported` | `capabilities().comment = false` → 409 `writeback_unsupported` (F1) |
| parent W9 status table and `status_for` ops `:484-486`, `failing` (W8), W11 enum `:561`, W-UI `:592-640` | 7 statuses; 5 ops; `failing` = assigned with status ≠ `ok`; "Write back to Jira" copy; `active` N = `assigned` | adds `not_private` (not a failure; F7) and op `Audience` (its own arms, F7); a 404 on a comment create goes through the issue check (`forbidden` or `not_found`, F6a), where it was `not_found`; a 403 that names a rate limit, on a read or a write, is `rate_limited` (F8); kind-neutral copy and `active` N = `assigned − not_private` (F10) |
| linkage `tracker-jira-linkage.md` R8 `:528` (GET shape), `:531`, `:533`, `:548`; Future adapters `:707` | PATCH and PUT `/fields` → `writeback_unsupported`; forges `{history:false, writeback:false}` | PATCH as W8 above; PUT `/fields` → `fields_unsupported`; a 409 row for it; forges `{history:false, comment:true, fields:false}` |
| `tracker-gitlab.md` `:29`, §G8 `:193-199`, `:83` (token), Out of scope `:249`, D10 `:274` | write-back unsupported; PAT `read_api` | comment supported, fields unsupported; `api` scope needed only for write-back |
| `tracker-github.md` `:341`, `:366`, token `:220-225`, Out of scope `:413`, E13 `:435` | write-back unsupported; read-only token | comment supported; Issues: read and write needed only for write-back (F9) |
| `decisions.md` D50, D91 (GitLab `read_api`), D92 (GitHub read-only token) | as stated | D93 (F11) amends all three |

## Resolved design

### F1 · Capabilities: comment and fields

`tracker/mod.rs:171` `Capabilities { history: bool, writeback: bool }` becomes
`Capabilities { history: bool, comment: bool, fields: bool }`. The `writeback` field is
removed, so the compiler finds every reader and every struct literal.

| Adapter | history | comment | fields |
|---|---|---|---|
| Jira | true | true | true |
| GitLab | false | true | false |
| GitHub | false | true | false |

Guards (they read capabilities, never the kind's name):

- **Pass** (`writeback.rs:516` `run`): skips with no request when `!comment`.
- **`PATCH …/trackers/{t}` `{"writeback_enabled": true}`**: 409 `writeback_unsupported` when
  `!comment`. `false` is always allowed (unchanged).
- **`PUT …/trackers/{t}/fields`**: 409 **`fields_unsupported`** when `!fields`. The check
  stays where `refusal_for` is called today (`api/handlers/trackers.rs:385`). That is after
  the body checks (422 `invalid_body`, `unknown_metric`, `duplicate_mapping`) and the
  tracker lookup (404), and before the lock, any network call and any DB write. An empty
  `[]` reaches it.
- `refusal_for(caps, enabling)` (`trackers.rs:471`) is replaced by two pure functions,
  `toggle_refusal(caps, enabling)` and `fields_refusal(caps)`.

`tracker_fields` rows are **created** only by `replace_tracker_fields`
(`db/pg_store/trackers.rs:1652`). Its only production caller is the PUT handler
(`trackers.rs:439`), which now refuses a `fields: false` tracker. The pass only deletes
rows (`delete_retired_fields`). So a forge tracker never has a mapped row, and the pass's
surface rule (`writeback.rs:626` `on_fields = !mapped.is_empty()`) needs no change.

### F2 · The comment methods take the ticket's address

GitLab and GitHub address a ticket's comments by **project and number**. The stored
`external_id` (GitLab's global issue `id`, GitHub's `databaseId`) is neither, but both are
derivable from the stored, lowercased `key`.

- `tracker/mod.rs` gains `pub struct TaskRef<'a> { pub external_id: &'a str, pub key: &'a str }`.
- `find_comment`, `create_comment`, `update_comment` and `issue_exists` take
  `task: &TaskRef<'_>` instead of `task_external_id: &str`. `write_fields` is unchanged
  (Jira-only).
- `WritebackTask` (`db/pg_store/trackers.rs:101`) gains `key: String`. `writeback_tasks`
  (`:1405`) also selects `key`, so the tuple grows from 7 fields to 8.
- `write_comment` (`writeback.rs`) builds the `TaskRef` from the task row for all four calls.
- Jira reads only `task.external_id`. Its requests do not change by a byte (gate 1).

### F3 · Shared helpers, reused, not duplicated

- `jira::is_jira_id` (`jira.rs:784`, `^[0-9]{1,18}$`) moves to `tracker/mod.rs` as
  `pub fn is_numeric_id`, and Jira calls it from there. Both forges use it for
  `comment_ref`.
- GitLab's existing `encoded(path)` (`gitlab.rs:55`, `/` → `%2F`) builds the F5 paths.
  `fetch_tasks` (`gitlab.rs:485-490`) and the ticket reader (`:590-593`) already *send*
  `{API}/projects/{encoded(lowercased path)}/issues/{iid}`. Both swallow a 404, so the
  code alone does not prove GitLab answers it. The live probe X1 does (2026-11-13):
  `GET /api/v4/projects/GitLab-Org%2FGitLab` → 200, `path_with_namespace: gitlab-org/gitlab`.
  GitLab resolves project paths case-insensitively.
- `WriteMethod` (`mod.rs:230`) gains `Patch`, which `ReqwestRaw`'s write send
  (`transport.rs:255-257`) maps to `reqwest::Method::PATCH`. GitHub edits a comment with
  PATCH. The write seam is unchanged otherwise (parent gate 13).

### F4 · The Markdown comment, shared by both forges

`writeback.rs` gains:

- `pub const COMMENT_MARKER: &str = "<!-- sensei:writeback -->";`
- `pub fn markdown(report: &Report) -> String`, pure. The body is exactly:

  ```
  <!-- sensei:writeback -->
  **<title>**

  - <line 1>
  - <line 2>
  …
  ```

  The parts are joined with `\n`, with no trailing newline. Each title and line is
  **sanitised** in this order:
  1. Every Unicode line terminator (`\r`, `\n`, U+0085, U+2028, U+2029) and every `\t` is
     replaced by a space. A line can never start a new Markdown line. This blocks GitLab
     quick actions (`/close`), headings and lists.
  2. Every ASCII punctuation character gets a `\` in front of it. CommonMark allows a
     backslash before any ASCII punctuation, and both forges render the character itself.
     The exception is `.`, `,`, `'`, `-` and `/`: on their own, inside a line, they form
     no construct. This covers:
     - mentions (`@`)
     - images and links (`!`, `[`, `]`, `(`, `)`)
     - GitHub math (`$`)
     - emoji (`:`)
     - HTML and entities (`<`, `>`, `&`)
     - GitLab references (`&`, `%`, `~`, `#`)
     - GitLab inline diffs (`{`, `}`, `+`)
     - emphasis, code, heading and table syntax

     3. (Amended at build, W5 review.) Leading space is trimmed, a leading `-` is escaped,
     and the `.` after a leading number is escaped, so text right after `- ` starts no
     list, thematic break or indented code. An empty title is left out, not rendered as
     `****`.

  A bare `www.` autolink cannot be escaped. Report text holds no hostnames today; the
     guard is for future text.

  Report strings come only from `writeback::render`. Its output is digits, units, `·`, `—`,
  `$`, brackets and parentheses (parent W5). The sanitising is a guard for future text, and
  is not needed for today's.
  - **Golden example 1** (parent W5): the title becomes
    `Sensei · AI usage from the assignee's sessions \(updated 2026-10-26 14\:30 UTC\)`.
    `Sessions: 3 (1 shared with other tickets)` becomes
    `Sessions\: 3 \(1 shared with other tickets\)`, and `API $4.32` becomes `API \$4.32`.
    The checked-in golden strings (gate 4) are produced by applying exactly these two steps.
  - **Golden example 2**: `cache read 900000 [2 of 3 sessions]` becomes
    `cache read 900000 \[2 of 3 sessions\]`.
- `pub fn is_marked(body: &str) -> bool`, defined as `body.starts_with(COMMENT_MARKER)`.

Both forge adapters call these, and neither has a renderer of its own. The comment hash
(`comment_hash(&report)`, `writeback.rs:428`, which hashes `report.lines`) does not change.

### F5 · GitLab comments (notes)

**Address.** (Amended at build: a path with no `/` is refused too, since GitLab would
read a bare `12345` as a project id.) `forge::parse_key(task.key)` gives `(path, iid)`; it already validates `path`
with `is_key_path` (`forge.rs:26`). Any of these gives `Err(BadRequest)` **before any
request**, as Jira's `issue_path` does:
- `parse_key` returns `None`
- `iid` is 0
- `iid` is greater than `i64::MAX`

Let `I = {API}/projects/{encoded(path)}/issues/{iid}`.

| Method | Request | Result |
|---|---|---|
| `find_comment` | `GET I` first: its `id` as text must equal `task.external_id` (F6b), otherwise `NotFound`. Then `GET I/notes?sort=desc&order_by=created_at&per_page=50` | Not an array → `Decode`. Otherwise the first note, newest first, with `system != true`, `author.id` as text `== owner`, and `is_marked(body)`. Returns its integer `id` as text, or `None`. |
| `create_comment` | `POST I/notes {"body": markdown(report)}` | the integer `id` as text; missing → `Decode` |
| `update_comment` | `comment_ref` fails `is_numeric_id` → `BadRequest`, no request. Otherwise `PUT I/notes/{comment_ref} {"body": markdown(report)}` | `Ok(())` |
| `issue_exists` | `GET I` | 2xx → `true`; `NotFound` → `false`; any other error → `Err` |

The window is the 50 newest notes, and system notes count against it. On a busy issue,
adoption can miss sensei's note and post a second one. That needs a lost create response
plus more than 50 later notes, the same bounded window as Jira's `maxResults=50`, and is
accepted.

### F6 · GitHub comments

**Address.** `(owner, name, n) = github_key(task.key)` (`github.rs:119`). If it returns
`None`, the call gives `Err(BadRequest)` before any request. Let `R = /repos/{owner}/{name}`.
Everything here is REST over the existing transports; no GraphQL is used, so §H2's
query-only seam does not change.

**`find_comment`**:
1. `GET R/issues/{n}`. Its `id` as text must equal `task.external_id` (F6b), otherwise
   `NotFound`. Then read `comments` (a u64).
   - Missing → `Decode`.
   - Any error on this GET propagates. Through `status_for(FindComment, …)`, `NotFound`
     gives `not_found`.
2. If `comments` is 0, return `None` with no second request.
3. Otherwise:
   - Let `last = ceil(comments / 50)`.
   - `GET R/issues/{n}/comments?per_page=50&page={last}`.
   - When `last > 1`, also GET `page={last-1}`.

   Together those pages hold at least `min(comments, 50)` of the newest comments.
4. Scan newest first: the later page first, each page in reverse. Take the first comment
   whose `user.id` as text `== owner` and where `is_marked(body)`. Return its integer `id`
   as text, or `None`.

| Method | Request | Result |
|---|---|---|
| `create_comment` | `POST R/issues/{n}/comments {"body": markdown(report)}` | the integer `id` as text |
| `update_comment` | `comment_ref` fails `is_numeric_id` → `BadRequest`, no request. Otherwise `PATCH R/issues/comments/{comment_ref} {"body": markdown(report)}` | `Ok(())` |
| `issue_exists` | `GET R/issues/{n}` | 2xx → `true`; `NotFound` → `false`; any other error → `Err` |

A comment posted between the count read and the page read can shift the pages, so adoption
can miss a sensei comment and post a second one. This is accepted. It needs a lost create
response *and* a concurrent comment.

### F6a · A 404 on create (both forges)

GitHub answers 404, not 403, when a token cannot write to a private repository.
`write_comment` already follows a 404 on **update** with `issue_exists` (parent W5). A 404
on **create** now does the same:
- the issue exists → `forbidden`, because the token can read the ticket but not comment on it
- the issue does not exist → `not_found`
- the issue check errors → `status_for(IssueCheck, e)`

Jira's create path gets the same rule. Jira answers 403 when it may not comment, so in
practice its behaviour does not change.

Which code a provider sends is external behaviour this repository cannot check. The rule
gives a correct status either way: a 403 that names no rate limit (F8) is `forbidden`
directly, and a 404 becomes
`forbidden` or `not_found` through the issue check. No status depends on guessing it.

### F6b · The address must still be the same ticket

The comment methods address a ticket by its stored `key` path. After a rename, transfer or
deletion, another project or repository can take over that path, and its `#n` is then an
unrelated ticket. So `find_comment` checks identity before any comment is adopted or
created:
- its first GET reads the issue
- that issue's `id` must equal `task.external_id` (GitLab's global `id`; GitHub's REST `id`,
  which equals the `databaseId` that sync stores)
- a mismatch returns `NotFound`, which `status_for(FindComment, …)` maps to `not_found`

`not_found` is sticky, so nothing is posted. The next sync re-keys or removes the task.

`find_comment` runs before every create (`writeback.rs:789-822`), so no create skips this.
An update addresses a comment sensei already holds:
- GitLab's `PUT I/notes/{id}` is scoped to the issue, so a moved path answers 404, which
  becomes `comment_removed` or `not_found`.
- GitHub's PATCH by comment id edits sensei's own comment wherever the issue now lives.

Jira addresses by `external_id` and needs no check.

### F7 · Never comment on a public or internal ticket

The pass gets a new per-ticket check, implemented once in `writeback.rs` and fed by two new
adapter methods:

- `fn audience_key(&self, task: &TaskRef<'_>) -> Option<String>`, pure.
  - GitHub: `Some("{owner}/{name}")` from `github_key`.
  - GitLab: `Some(path)` from `parse_key`.
  - Jira: `None`.
  - A key the adapter cannot parse gives `None` on a forge too. That is safe, because every
    comment method then refuses with `BadRequest` before any request.
- `async fn audience(&self, t: &dyn GetTransport, audience_key: &str) -> Result<Audience, TrackerError>`.
  `Audience` is `Private` or `Visible`.
  - GitHub: `GET /repos/{owner}/{name}`. `visibility == "private"` gives `Private`. Any
    other value, or a missing `visibility` (which includes `public` and `internal`), gives
    `Visible`.
  - GitLab: `GET {API}/projects/{encoded(path)}`. `visibility == "private"` gives `Private`.
    Anything else (`public`, `internal`, missing) gives `Visible`.
  - Jira: never called; its trait body returns `Err(UnsupportedTracker)`.

**In the pass**, after the unchanged-hash check and once `writeback_target` returns `Go`
(amended at build, 2026-11-14: `writeback_target` only reads, and checking after it means
a reassigned ticket or a stopped pass costs no visibility request):
- `audience_key(task)` is `None` → write as today (this covers Jira).
- Otherwise read the per-pass cache, a `HashMap<String, Audience>`. On a miss, call
  `audience` once per key per pass.
  - `Private` → write as today.
  - `Visible` → record status **`not_private`** and write nothing. This status is not
    sticky: every pass re-checks it, so a repository made private later gets its comment.
  - `Err(e)` → record `status_for(WriteOp::Audience, e)` (below), and write nothing. The
    check fails closed.

The cache lives for one pass, which is bounded by the pass deadline. A repository made
public during a pass can still receive that pass's remaining writes for it. This is
accepted, because the next pass skips it.

The status is recorded through the existing `record_write_status`. A ticket that already
has a comment when its repository goes public keeps that comment, but sensei stops editing
it. Deleting comments stays out of scope (parent).

**Status and counts.**
- `sensei.tracker_writeback_status` gains `'not_private'`, appended after `'error'`, with
  the comment `-- the ticket's repository or project is public or internal; nothing written
  (re-checked each pass)`. The existing comments change too:
  - `forbidden` becomes `401/403 other than a rate limit: …`
  - `rate_limited` becomes `429 after the retry budget, or a 403 (read or write) that
    names a rate limit (F8)`

  `database/design.dbml:826-834` gets the same value.
- `status_for` (`writeback.rs:89`) gains `WriteOp::Audience`, with explicit arms placed
  before the generic ones (amended at build, 2026-11-14, W4 review HIGH):
  - `NotFound` → `not_found`. Both forges answer 404 for a repository the token cannot
    see, and the ticket cannot be written then.
  - `RateLimited` → `rate_limited`, so `RATE_LIMITED_STOP` counts it
  - anything else, `Forbidden` and `Unauthorized` included → `error`, retried next pass.
    The read is a GET, and a GET's 403 cannot be told from GitHub's rate-limit 403
    (as first written, F8 mapped only writes; it now maps reads too). A sticky `forbidden` would let one throttled read hold every
    ticket of the repository until write-back is toggled.

  A failed read is cached for the pass like an answer, so each ticket it holds back
  counts once; a cached rate limit therefore ends the pass after `RATE_LIMITED_STOP`
  tickets. A non-rate-limit failure resets that run, as a write's does; a `not_private`
  skip leaves it as it was.
  Its op × `TrackerError::ALL` test covers the new op.
- `WritebackStatus::ALL` gains `NotPrivate` in the same position. A test pins that order.
- `database/apply/before/sensei_tracker_writeback_status_not_private.sql` adds the value to
  an existing type. It follows `sensei_tracker_kind_github.sql`: a `to_regtype` guard and
  `add value if not exists`. It is registered under `apply.before` in
  `database/design.yaml` (beside `:192`). Gate 10 tests it with the existing hook-test
  pattern (`tracker/mod.rs:1641-1670`).
- `writeback_counts` (`db/pg_store/trackers.rs:1679`) returns `(assigned, failing, not_private)`:
  - `failing` excludes `not_private`, because it is not a failure.
  - `not_private` counts assigned live tasks with that status.
- The card's `writeback` object gains `"not_private": <n>`.
- The TS `TrackerWritebackStatus` (`app/src/lib/types.ts:1961`) gains `'not_private'`.

### F8 · A rate-limit 403 is `rate_limited`, on a write or a read

`RawResponse` (`transport.rs:11`) gains `rate_exhausted: bool`. `ReqwestRaw::execute`
(`:195-205`) sets it when the response carries `x-ratelimit-remaining: 0`.
`map_write_response` treats a **403** as `RateLimited` when any of these holds:
- `retry_after.is_some()`
- `rate_exhausted`
- the body's `message` (JSON) contains `rate limit`, ignoring case. This is the case for
  GitHub's secondary limit, which can send no header.

Any other 403 stays `Forbidden`.

**Amended at build (2026-11-14, W5 review):** the rule lives in `map_response`, so it
applies to reads too. GitHub's comment-list GETs are limited with 403, and the
comment-list read's `forbidden` is sticky, so one throttled read would have held the
ticket for good. A 410 (GitHub: a deleted issue) maps to `NotFound`.

The policy loop still retries only 429. The pass counts the write as `rate_limited`, which
is not sticky, and its existing `RATE_LIMITED_STOP` rule ends the pass after a run of them.
The four `RawResponse { … }` literals (`transport.rs:205, :322`, `mod.rs:1422`,
`gitlab.rs:750`) gain the field.

### F9 · Token scope: needed when write-back is used, not at connect

Connect stays read-only for both forges. A token without write permission connects and
syncs as before. When write-back is on, its writes fail with 403 → `forbidden` (or 404 →
F6a → `forbidden`), which is sticky (parent W9). Replace token, which is a re-connect, or
turning write-back off and on again clears it (parent W8).

| Kind | Needed to write comments |
|---|---|
| GitLab | PAT scope `api` |
| GitHub fine-grained | Issues: **read and write** on each repository whose tickets are on the project |
| GitHub classic | `repo`, which is already needed to read private repositories |

### F10 · API and app

- **Card.** `GET …/trackers` element: `writeback` gains
  `"supports": {"comment": <bool>, "fields": <bool>}` from the adapter's capabilities. It
  also gains `"not_private"` (F7). The existing `surface` (which surface is active) is
  unchanged. The TS `TrackerWriteback` interface (`types.ts:1902`) gains both fields.
- **`IntegrationsScreen.svelte`:**
  - `:126` `{#if tracker.kind === 'jira'}` becomes `{#if tracker.writeback.supports.comment}`.
  - The field-mapping section shows only when `supports.fields`.
  - The `data-writeback="unsupported"` line (`:205-207`) shows when `!supports.comment`.
    No registered adapter is in that state today.
  - The toggle label `:140` "Write back to Jira" becomes "Write back to {TRACKER_NAME[kind]}".
  - The page description `:73` becomes: "Connect this workspace's Jira, GitLab or GitHub
    boards, each at a level, so sensei can link AI sessions to tickets and report time,
    tokens and cost per ticket. On tickets assigned to you, sensei writes these numbers back
    to the tracker: as a comment, or on Jira in the fields you map. It never comments on a
    public or internal GitLab project or GitHub repository."
  - The state line `active`: "Writing to N tickets assigned to you", with
    N = `assigned − not_private`. It was N = `assigned`, which counted skipped tickets.
  - The GitLab and GitHub connect-form token notes (`:332`, `:384`) gain: "Write-back also
    needs the **api** scope" (GitLab) or "Write-back also needs **Issues: read and write**"
    (GitHub). The same sentence goes on every other surface that states the token
    requirement:
    - `tracker-errors.ts:67-68` (GitLab) and `:84` (GitHub `token_required`)
    - the CLI help and prompts (`crates/cli/src/tracker.rs:135-136, :629, :643`)
    - the CLI unauthorized text (`:465-469`) Connect itself still needs only read access (F9).
  - Under the toggle, for a tracker with `!supports.fields`:
    - GitLab: "Sensei posts one comment on each ticket assigned to you, in private projects
      only, and keeps it up to date. The token needs the **api** scope."
    - GitHub: "Sensei posts one comment on each ticket assigned to you, in private
      repositories only, and keeps it up to date. The token needs **Issues: read and write**
      on each repository (or a classic `repo` token)."
    - Both: "The first comment notifies the ticket's participants; updates don't."
  - When `not_private > 0`, the card shows the sub-line "{n} tickets are in public or
    internal repositories; sensei doesn't comment there."
- **`app/src/lib/tracker-errors.ts`:**
  - `fields_unsupported`: "Field mapping isn't available for this tracker; sensei writes a
    comment instead."
  - `WRITEBACK_STOPPED` (`:100-103`) becomes a function `writebackStopped(code, kind)` that
    names the tracker through `TRACKER_NAME` (`tracker-errors.ts:19`):
    - `token_missing` and `credential_store`: "Couldn't read the {name} token to write back.
      Replace the token."
    - `rate_limited`: "{name} is rate-limiting writes. Write-back resumes on the next sync."
  - `writebackText(w)` (`integrations-state.svelte.ts:141-151`) takes the tracker's `kind`.
    It passes `kind` to `writebackStopped`, and to `lastErrorText` for
    `owner_lookup_failed`; today that call defaults to `'jira'` and would say "Jira
    rejected…" on a forge card. Its callers pass `tracker.kind`.
- **`TicketsScreen.svelte:50`** sub-line: "On tickets assigned to you, sensei writes these
  numbers back, as set on Integrations."
- **`api/handlers/scheduled_tasks.rs:71`** description: "… writes usage metrics back to
  tickets assigned to you (and SDLC metrics on Jira), and drains removed credentials".
- **Specs that change in meaning, not only in signature:**
  - `IntegrationsScreen.spec.svelte.ts:135, :178`: the GitLab and GitHub cards show the
    toggle and no field mapping.
  - `:137`: the GitLab card still matches no `/Jira/`.
  - `:353`: the new page description. (`:373`, the Jira card's "Write back to Jira", does
    not change.)
  - `integrations-state.spec.svelte.ts:677-680`: the `writebackStopped` text, with a kind.
  - `TicketsScreen.spec.svelte.ts:156`.
  - Rust handler tests:
    - `api/handlers/trackers.rs:899-908`: the pure `refusal_for` / `writeback_refusal` unit
      test, rewritten for `toggle_refusal` and `fields_refusal`.
    - `:1101`: real `GithubAdapter`; PATCH `true` → 200.
    - `:1366`: real `GitlabAdapter`; PATCH `true` → 200.
    - `:1376`: real `GitlabAdapter`; PUT `/fields` → 409 `fields_unsupported`.
  - The test fake `FakeAdapter::gitlab_like()` (`tracker/mod.rs:865-871`) has every
    capability today. It becomes `{history: true, comment: true, fields: true}`, so the
    tests that use it (`writeback.rs:2529`, `history.rs:583`, `connect.rs:382`,
    `sync.rs:731`, `mod.rs:860`) keep their meaning.

### F11 · Docs

- **Parent `tracker-metrics-writeback.md`:**
  - Out of scope `:686`.
  - W5: a per-kind table (Jira ADF; forges F4–F6), the marker rule, F6a.
  - W7 step 4 `:387-389`.
  - W8 `:431-433`, the card shape (`supports`, `not_private`), `failing`.
  - W9: `not_private`, the F8 403 row, the F6a create-404 row, F6b; `status_for` ops
    `:484-486` gain `Audience`.
  - W10: `TaskRef`, `is_numeric_id`, the forge key shape checks.
  - W11 `:561`: the enum gains `not_private`.
  - W-UI `:592-640`: the label, sub-line, stop texts, `active` N, the `not_private`
    sub-line, the scheduled-task text and the Tickets sub-line, all as in F10.
  - Gate 13: `Patch`.
- **`tracker-jira-linkage.md`:** `:528` (`writeback` gains `supports`, `not_private`), `:531`,
  `:533`, `:548` (a `fields_unsupported` row), `:707`.
- **`tracker-gitlab.md`:** `:29`, §G8 `:193-199`, `:83` (`api` needed only for write-back),
  the G10 App row (drop "Write-back: not available"), Out of scope `:249`, D10 `:274`.
- **`tracker-github.md`:** `:341`, `:366`, token text `:220-225`, Out of scope `:413`, E13 `:435`.
- **`decisions.md` D93:** comment write-back on every tracker kind; never on a public or
  internal forge repository; fields stay Jira-only; the forges' write permission is needed
  only for write-back. Add amendment notes on D50, D91 (GitLab `read_api`) and D92 (GitHub
  read-only token).
- **Screen specs:**
  - `workspace-integrations.md` `:51-54, :58-62, :64, :68-70, :72-75, :121, :132, :170,
    :216-221, :247, :288-289, :297`. The wrong gate becomes "a Fields section on a GitLab or GitHub
    card", and the token notes follow F10.
  - `workspace-tickets.md` `:77`.
- **Code doc comments:** `gitlab.rs:5` and `github.rs:5` ("History and write-back are
  unsupported") become "History and mapped fields are unsupported", and the `Capabilities`
  field docs in `mod.rs`.

### Build order

1. F1, F2, F3 refactors with Jira unchanged. Gate 1 must be green before any forge code.
2. F4 renderer (pure, golden tests).
3. F8 transport mapping, then F6a.
4. F7: DDL, upgrade hook, status, counts, the pass check (fake adapter tests).
5. F5 GitLab and F6 GitHub (recorded-fixture tests per method, plus `audience`).
6. F10 API and app.
7. F11 docs.

### Out of scope

Mapped fields on GitLab or GitHub; GitHub Projects fields; a scope probe at toggle time;
deleting sensei comments, including one left on a ticket whose repository later went
public; GitHub Enterprise Server; writing SDLC figures to a forge (the comment carries
usage only, parent S9 note); retrying a rate-limit 403 inside the transport; a per-tracker
override to comment on public repositories.

**Accepted risk (W5 review):** a GitHub issue transferred from a private repository to a
public one keeps its comments. Until the next sync re-keys the ticket, the visibility read
still names the old, private repository, and the PATCH by comment id may reach the comment
on its new, public issue. The pass runs right after each sync, which re-keys a transferred
issue, so the window is between a transfer and the next sync. Probe X6 checks whether
GitHub scopes a comment edit to the repository in its path.

**Accepted risk:** `update_comment` edits by the stored `comment_ref` without re-reading the
comment's author. The ref is only ever written by sensei, from its own create or from an
adoption that matched author and marker. GitHub's `PATCH …/issues/comments/{id}` is not
scoped to the issue. Re-reading before each update would cost one GET per changed ticket.
Recorded in D93.

## Done gate

1. **Jira unchanged.** Every existing `writeback.rs` and `jira.rs` write-back test passes
   with only signature edits. A recorded Jira pass sends exactly the same method, path,
   query and body sequence as before. A Jira pass makes no `audience` call.
2. **Capabilities.**
   - `cargo build --tests -p senseid` is clean with `writeback` removed from `Capabilities`.
   - Each adapter's tests assert Jira `{true,true,true}` and GitLab and GitHub
     `{false,true,false}`.
   - `rg -n 'kind\s*==\s*"(jira|gitlab|github)"' crates/senseid/src/tracker/writeback.rs crates/senseid/src/api/handlers/trackers.rs`
     → 0 hits. It is 0 today; this is a regression guard, not proof of the change.
3. **Guards.** On GitLab and GitHub:
   - PATCH `writeback_enabled: true` → 200, and the tracker is enabled.
   - `PUT /fields` with `[]`, and with one valid entry → 409 `fields_unsupported`. The fake
     transport records 0 requests, and `tracker_fields` has 0 rows for the tracker.
   - `PUT /fields` with a malformed body → 422 `invalid_body`, as today.

   On Jira, PUT `/fields` is unchanged.
4. **Renderer.**
   - `markdown(golden_1)` and `markdown(golden_2)` exactly equal checked-in strings that
     contain the F4 examples (`14\:30`, `\$4.32`, `\[2 of 3 sessions\]`).
   - A title with each escaped character renders with a `\` before it.
   - A line `"x\n/close"` renders as one line, `x /close`.
   - A line `"@team"` renders as `\@team`.
   - The line `"a\u{2028}b\t&123 {+x+}"` renders as `a b \&123 \{\+x\+\}`.
   - `is_marked(markdown(r))` is true.
5. **GitLab lifecycle** (fake transport, recorded fixtures):
   - First pass on assigned `acme/platform#12` (private), in this order:
     - 1 GET `/api/v4/projects/acme%2Fplatform` (audience)
     - 1 GET `/api/v4/projects/acme%2Fplatform/issues/12` (identity, F6b)
     - 1 GET `…/issues/12/notes?sort=desc&order_by=created_at&per_page=50`
     - 1 POST to `…/notes` with the F4 body

     The returned id is stored.
   - Changed report → exactly [GET project (audience), PUT `…/notes/{id}`]. Unchanged
     report → 0 requests.
   - `acme/sub/platform#3` → path `acme%2Fsub%2Fplatform`.
   - A key path with `.` and `-` stays unencoded apart from `/`.
   - Not adopted: an owner note with the marker but `system: true`, another user's marked
     note, an owner note without the marker.
   - A matching note → adopted: PUT, no POST.
   - A non-array notes body → `error` (`Decode`).
   - The issue GET returns an `id` different from `external_id` → `not_found`, with no
     notes GET and no POST (F6b).
6. **GitHub lifecycle:**
   - `comments: 0` (private repo), in this order:
     - repo GET `R` (audience)
     - issue GET `R/issues/12`
     - 1 POST `R/issues/12/comments`
   - `comments: 120` → exactly [GET `R` (audience), GET `R/issues/12`, GET page 3, GET page 2
     at `per_page=50`]. A marked owner comment on page 2 is adopted, then PATCH
     `R/issues/comments/{id}`.
   - Changed report with a stored ref → exactly [GET `R`, PATCH `R/issues/comments/{id}`].
   - `comments: 50` → only page 1. `comments: 51` → pages 2 then 1. `comments: 30` → only
     page 1.
   - Missing `comments` → `error`.
   - Not adopted: a bot's or another user's comment with the marker.
   - First issue GET 404 → `not_found`.
   - The issue GET returns an `id` different from `external_id` → `not_found`, with no page
     GET and no POST (F6b).
7. **404 handling.** On each forge:
   - update 404, then issue GET 200 → `comment_removed`; issue GET 404 → `not_found`
   - create 404, then issue GET 200 → `forbidden`; issue GET 404 → `not_found`
8. **Shape refusals.** These give `BadRequest` with **0 requests** on the fake transport:
   - a `key` that `parse_key` or `github_key` rejects
   - GitLab `iid` 0
   - a `comment_ref` of `abc`
   - a `comment_ref` of 19 digits
9. **Rate-limit 403.**
   - These writes give `rate_limited`:
     - 403 with `retry-after: 30`
     - 403 with `x-ratelimit-remaining: 0`
     - 403 with no rate headers and body `{"message":"You have exceeded a secondary rate limit"}`
   - A bare 403 on a write → `forbidden`.
   - A **read** with 403 and `retry-after`, `x-ratelimit-remaining: 0` or a rate-limit
     message → `RateLimited` too (amended at build); any other read 403 → `Forbidden`.
   - Any 410 → `NotFound`.
10. **Public skip** (fake adapter and fixtures):
    - Two tickets in one public GitHub repo: 1 `GET /repos/o/r` (cached), 0 comment
      requests, both `not_private`.
    - The same with `"visibility": "internal"`, and with GitLab `"visibility": "public"` →
      the same result.
    - A repo-visibility GET that errors → that ticket's status per `status_for(Audience, …)`,
      and 0 comment requests:
      - 404 → `not_found` (sticky)
      - 403 or 401 → `error` (retried next pass)
      - 429 → `rate_limited`
      - malformed JSON → `error`
    - Two repositories in one pass, one private and one public: two reads, and only the
      private ticket is commented.
    - A ticket already commented whose repository goes public: no update, status
      `not_private`, the comment ref kept.
    - The fixture switched to private → the next pass comments.
    - `writeback_counts`: a `not_private` ticket is in `not_private`, not in `failing`.
    - The DDL enum labels equal `WritebackStatus::ALL`, with `not_private` last.
    - The upgrade hook adds `not_private` to a database whose enum lacks it, and is a no-op
      otherwise.
11. **Patch.** `ReqwestRaw` sends `PATCH` for `WriteMethod::Patch` to a local listener.
12. **API.** `GET …/trackers`:
    - `writeback.supports` is `{comment:true, fields:true}` for Jira and
      `{comment:true, fields:false}` for GitLab and GitHub.
    - `writeback.not_private` is present.
13. **App.**
    - The Integrations specs show `[data-action="writeback"]` and the F10 copy on GitLab and
      GitHub cards, with no `[data-action="fields"]` (`IntegrationsScreen.svelte:151`, the
      always-present Fields control).
    - No card text matches `/Jira/` on a forge card.
    - `writebackText` for a GitHub `rate_limited` stop does not mention Jira.
    - The Tickets sub-line does not contain "On Jira tickets".
    - The Jira card is unchanged.
    - The write-back block and field section in `IntegrationsScreen.svelte` contain no
      `kind ===` test.
14. **Stubs gone.** `rg -n UnsupportedTracker crates/senseid/src/tracker/{gitlab,github}.rs`
    shows only `check_fields`, `field_in_use` and `write_fields`. No comment method,
    `issue_exists` or `audience` returns it. Only Jira's `audience` does, and it is never
    called.
    `rg -n 'write-back are unsupported' crates/senseid/src/tracker` → 0 hits.
15. **Docs.**
    - `rg -n 'writeback_unsupported' docs/spec/daemon/tracker-jira-linkage.md docs/spec/daemon/tracker-metrics-writeback.md`
      pairs it with no `PUT /fields`.
    - `rg -n 'not available for (GitLab|GitHub)' docs/spec app/src` → 0 hits.
    - D93 exists, and D50, D91 and D92 carry its amendment notes.

**Manual check after merge** (not an unattended gate; recorded under "Probes" below by
whoever runs it): with test projects on gitlab.com and github.com, confirm that:
- (X2) the hidden marker comes back verbatim in `body`
- (X3) editing a comment sends no notification
- (X5) a private test repo is commented and a public one is not

If X2 fails, the fix is to make the marker the visible first line `Sensei ·` and match on
that (`is_marked` changes, the design does not). If X3 fails, only copy changes.

## Wrong gate

- **Gated by kind name.** A `kind == "gitlab"`, `'github'` or `'jira'` test decides
  write-back in Rust or TS, instead of capabilities and `supports`.
- **Jira changed silently.** The `TaskRef` or F6a refactor changes a Jira request. Gate 1's
  recorded sequence catches it.
- **Adopting someone else's comment.** Matching on the marker alone, or on the author
  alone.
- **Unsanitised Markdown.** A report string reaches the body without F4 sanitising. A test
  that passes only on today's lines does not prove sanitising; gate 4's title, newline and
  `@` cases do.
- **Commenting on someone else's ticket.** A create or adoption that does not compare the
  fetched issue's `id` with `external_id` (F6b).
- **Commenting on a public ticket.** Any path that writes a comment without a `Private`
  audience for its key: a cache keyed wrongly, an `Err` treated as private, or `internal`
  treated as private.
- **`not_private` counted as failing.** The card would then show a red failure for a
  deliberate skip.
- **Encoding the key wrongly.** Building the GitLab path from anything but the stored
  lowercased `key`, or encoding more than `/`.
- **A rate-limit 403 left as `forbidden`.** On a read or a write, a 403 that names a
  rate limit must be `rate_limited`, never the sticky `forbidden` (F8, amended at build).
- **Fields shown for a forge.** The mapping UI, or `PUT /fields`, reachable on GitLab or
  GitHub.
- **A stub left reachable.** A forge comment method or `issue_exists` still returns
  `UnsupportedTracker` behind `comment: true`.
- **Duplicated helpers.** A second `/`→`%2F` encoder, or a second numeric-id check.

## Probes

| # | Claim | Status |
|---|---|---|
| X1 | GitLab resolves a project path case-insensitively | **settled live 2026-11-13**: `GET https://gitlab.com/api/v4/projects/GitLab-Org%2FGitLab` → 200, `gitlab-org/gitlab`. (r2 said "production already does it", which the verifier showed was MISLEADING: both callers swallow 404.) |
| X2 | An HTML comment as the first line comes back verbatim in `body` | manual, after merge; fallback defined |
| X3 | Editing a comment sends no notification | manual, after merge; copy only |
| X4 | A GitHub token without write access gets 403 or 404 on `POST …/comments` | **resolved by design**: F6a maps both to `forbidden` when the issue exists |
| X5 | Public and private repos behave per F7 against the real APIs | manual, after merge |
| X6 | `PATCH /repos/A/issues/comments/{id}` after the issue moved to repo B answers 404 or 301 (scoped to A) | manual, after merge; if it edits the comment in B, add an identity GET before each GitHub update |

## Claims (about the existing code; r1 verified 2026-11-13 by `sensei-claims-verifier`; r2 claims marked †)

| # | claim | check | actual | verdict |
|---|---|---|---|---|
| C1 | `Capabilities` has exactly `history` and `writeback` | read `tracker/mod.rs:169-176` | struct at `:171` | CONFIRMED |
| C2 | The only non-test readers of `.writeback` are `writeback.rs:516` and `trackers.rs:472`. Tests build whole struct literals | `rg -n '\.writeback\b'`; `rg 'Capabilities \{'` | as stated (literals at `mod.rs:717, :870, :1606`, `github.rs:867`, `history.rs:582`, `writeback.rs:2528`, `trackers.rs:899, :905`) | CONFIRMED |
| C3 | PUT `/fields` refuses via `refusal_for(caps, true)` at `:385`, after the body checks and the tracker lookup, and before the lock and the network | read `trackers.rs:334-446` | as stated | CONFIRMED |
| ~~C3a~~ | ~~refused "whatever the body is"~~ | read `trackers.rs:345-385` | the body checks come first | **FALSE (r1)**. F1 now states today's order. |
| C4 | `tracker_fields` rows are created only by `replace_tracker_fields`, whose only production caller is `trackers.rs:439`; the pass only deletes | `rg` callers | INSERT `pg_store/trackers.rs:1652`; DELETE `:1529` | CONFIRMED (r1 "only writer" was MISLEADING, reworded) |
| C5 | The four comment methods and `write_fields` take `task_external_id: &str` | read `mod.rs:538-590` | yes | CONFIRMED |
| C6 | `WritebackTask` has no `key`; `writeback_tasks` selects 7 columns | read `pg_store/trackers.rs:99-112, :1405-1447` | yes | CONFIRMED |
| C7 | `tracker_tasks.key` is NOT NULL and lowercased `path#n` on both forges | DDL; `gitlab.rs`, `github.rs` | yes | CONFIRMED |
| C8 | `WriteMethod` has only `Post` and `Put` | `mod.rs:229-233` | yes | CONFIRMED |
| C9 | Every forge write-back method returns `Err(UnsupportedTracker)`; stubs at `gitlab.rs:515-569` and `github.rs:595-650` | read | yes (r1 GitHub range `594-630` was MISLEADING) | CONFIRMED |
| C10 | Both forges' `owner()` store the numeric user `id` as text | `gitlab.rs:263-268`, `github.rs:357-363` | yes | CONFIRMED |
| C11 | `parse_key → (String, u64)` and already calls `is_key_path`; `github_key → (String, String, i64)` with `1 ≤ n ≤ i32::MAX` | `forge.rs:19-31`, `github.rs:119-126` | yes | CONFIRMED |
| C12 | The pass builds both transports through `ReqwestRaw::new` (PublicOnly resolver, `.no_proxy()`, redirects off) | `writeback.rs:578-598`, `runtime.rs:41-59`, `transport.rs:158-180` | yes | CONFIRMED |
| C13 | `RawResponse` has `status`, `retry_after` and `body`, and is constructed literally at **4** sites | `rg 'RawResponse\s*\{'` | 4 plus the definition | CONFIRMED (r1 said 5: **FALSE**, corrected) |
| C14 | `map_write_response` maps 403 → `Forbidden` via `map_response` | `transport.rs:91-110` | yes | CONFIRMED |
| C15 | The comment hash is computed from the `Report`'s lines | `writeback.rs:428-432` | yes | CONFIRMED |
| C16 | The app gates the write-back block on `tracker.kind === 'jira'` | `IntegrationsScreen.svelte:126, :205-207` | yes | CONFIRMED |
| C17 | The card's `writeback` object has no capability field; it has `surface` | `trackers.rs:104-112` | yes | CONFIRMED |
| C18 | GitLab `API` is `/api/v4`, and paths are relative to `base_url` | `gitlab.rs:19` | yes | CONFIRMED |
| C19 | Only `writeback.rs` and the factory hold a `WriteTransport` | `rg 'write_transport\('` | yes | CONFIRMED |
| C20 | The comment surface calls `write_fields` only for retired-field clears | `writeback.rs:716-726` | yes | CONFIRMED |
| ~~C21~~ | ~~golden 1 renders `[2 of 3 sessions]`~~ | `rg '2 of 3 sessions'` | golden 2 only | **FALSE (r1)**. F4 now credits golden 2. |
| C22 † | `gitlab::encoded` exists and is `/`→`%2F`; production GETs use it with the lowercased path | `gitlab.rs:55, :485-490, :590-593` | yes | CONFIRMED (verifier) |
| C23 † | `jira::is_jira_id` is `^[0-9]{1,18}$` | `jira.rs:784-786` | yes | CONFIRMED |
| C24 † | `tracker_writeback_status` has 7 labels ending in `error`, pinned to `WritebackStatus::ALL` by a test | `database/ddl/enum/sensei/tracker_writeback_status.ddl`; `writeback.rs:38` | yes | CONFIRMED (r2 verifier) |
| C25 † | Enum values are added to existing installs by a `database/apply/before/*.sql` hook | `sensei_tracker_kind_github.sql` | yes | CONFIRMED (r2 verifier) |
| C26 † | `writeback_counts` computes `failing` as assigned with status ≠ `ok` | `pg_store/trackers.rs:1679-1680` | yes | CONFIRMED (r2 verifier) |
| C27 † | The TS write-back type is `TrackerWriteback` (`types.ts:1902`), and `TrackerWritebackStatus` is at `:1961` | read | yes | CONFIRMED (r2 verifier) |
| C28 † | The Jira copy is at `IntegrationsScreen.svelte:73, :140`, `tracker-errors.ts:100-103` and `TicketsScreen.svelte:50` | read | yes | CONFIRMED (conformance, re-read 2026-11-13) |
| C29 † | Linkage R8 names `writeback_unsupported` for PUT `/fields` at `:533` and `:548`, and the forge row is at `:707` | grep | yes | CONFIRMED (re-read 2026-11-13) |
| C30 † | `write_comment` follows only an **update** 404 with `issue_exists`; a create error goes straight to `status_for(CreateComment, …)` | `writeback.rs:789-822` | yes | CONFIRMED (r2; range corrected) |
| C31 † | The pass's unchanged-hash check comes before `writeback_target` | `writeback.rs:687-700` | yes | CONFIRMED (r2 verifier) |
| C32 ‡ | GitLab sync stores the issue's REST global `id` as `external_id`; GitHub sync stores the issue's `databaseId`, which GitHub's REST issue `id` equals | `gitlab.rs:164, :240`; `github.rs:281-286` | code: `external_id: id(v)` (GitLab REST `id`), `databaseId` (GitHub); REST issue `id` = GraphQL `databaseId` is GitHub's documented identity, exercised by gate 6's recorded fixture | CONFIRMED (r3, author re-read) |
| C33 ‡ | `write_comment` calls `find_comment` before every create (no create path skips it) | `writeback.rs:789-822` | `comment_ref` NULL → `find_comment`, and `create_comment` only on `Ok(None)` | CONFIRMED (r3, author re-read) |

## Review record (r1 → r2, 2026-11-13)

Reviewers: claims verifier, plan-depth, spec-doc, data-correctness, conformance auditor,
security. All six ran, and none was NOT RUN.

| Sev | Finding | By | Where fixed |
|---|---|---|---|
| HIGH | PUT `/fields` 409 code: parent W7 and linkage R8 not named | conformance, verifier | Overrides, F11 |
| HIGH | Live probes left design forks open, and gate 13 could not run unattended | depth | X1 settled, X4 by design (F6a), X2/X3/X5 manual after merge |
| MED | Usage figures exposed on public forge issues | security | **F7** (user: skip public, 2026-11-13) |
| MED | F1 "whatever the body is" FALSE | verifier | F1 |
| MED | Golden 1/2 bracket example FALSE | verifier, data, conformance | F4 |
| MED | Header-less GitHub secondary limit becomes sticky `forbidden` | data | F8 body match |
| MED | Jira-named copy on forge cards, stop text, Tickets sub-line, screen-spec wrong gate | conformance | F10, F11 |
| MED | GitLab `api` scope missing from D91 and `tracker-gitlab.md:83` | conformance | F11 |
| MED | No executable gate for "kind name" or "stub left" | spec-doc | gates 2, 13, 14 |
| MED | Escape set unjustified; newline, `@`, leading `/` | depth, security | F4 sanitising |
| MED | GitLab window and system notes not stated as accepted | depth | F5 |
| LOW | Duplicate encoder and id check | verifier | F3 |
| LOW | C13 count, GitHub stub range, "only writer" | verifier | ledger, F1 |
| LOW | `surfaces` beside `surface`; TS type name | verifier | `supports`, `TrackerWriteback` |
| LOW | Boundary, `Decode`, nested-group and read-`remaining` gates; handler tests change in meaning | spec-doc, depth | gates 5, 6, 9; F10 |
| LOW | GitHub first-GET 404 unspecified | depth | F6 step 1, gate 6 |
| LOW | Update not re-bound to author | security | accepted risk, D93 |

## Review record (r2 → r3, 2026-11-13)

| Sev | Finding | By | Where fixed |
|---|---|---|---|
| MED | Key path could point at a different ticket after a rename or transfer; posting to an unrelated private issue | security (r2) | F6b, gates 5 and 6 |
| LOW | Visibility cache is stale within a pass | security (r2) | accepted, stated in F7 |
| LOW | Sanitiser missed `& % { } ( ) + ^`, Unicode line terminators and tabs | security (r2) | F4: all ASCII punctuation but `. , ' - /`; gate 4 |
| HIGH | Linkage R8 GET shape `:528` not named | conformance (r2) | Overrides, F11 |
| MED | `status_for` op list misses `Audience` | conformance (r2) | F7, F11 |
| MED | W11 enum list and `design.dbml` not named | conformance (r2) | F7, F11 |
| MED | Parent W-UI copy and the `active` count (counted skipped tickets) not named | conformance (r2) | F10, F11 |
| MED | Scheduled-task description says Jira-only and SDLC | conformance (r2) | F10 |
| LOW | Page description said "public" but not "internal" | conformance (r2) | F10 |
| LOW | Connect-form, CLI and error token copy not named | conformance (r2) | F10, F11 |
| LOW | DDL enum comments stale under F8 | conformance (r2) | F7 |
| FALSE | F10 handler tests "use `gitlab_like`" (they use real adapters; `gitlab_like` has every capability) | verifier (r2) | F10 test list; `gitlab_like` → `{true,true,true}` |
| FALSE | F10 spec lines `:353, :373` called `WRITEBACK_STOPPED` text | verifier (r2) | F10 |
| FALSE | `transport.rs:272-275` (the verb match is `:255-257`) | verifier (r2) | F3 |
| MISLEADING | `WRITEBACK_STOPPED` "uses `TRACKER_NAME` like the rest of the file"; `writebackText` has no kind; `lastErrorText` defaults to Jira | verifier (r2) | F10: `writebackStopped(code, kind)`, `writebackText` takes `kind` |
| MISLEADING | X1 "proven by production" (both callers swallow 404) | verifier (r2) | **live probe run 2026-11-13: settled** |
| MISLEADING | Audience errors "as `IssueCheck`" made 403 an `error` and 429 uncounted | verifier (r2) | F7: explicit `Audience` arms; gate 10 |
| INFO | Hook registration in `design.yaml`; gate 2 vacuous today; gate 13 `add-field` vacuous | verifier (r2) | F7, gate 2 note, gate 13 uses `data-action="fields"` |
| UNCHECKABLE | Which code GitHub or Jira sends for a write it refuses | verifier (r2) | F6a: the status is correct for either answer |

## Review record (r3 → r4, 2026-11-13: final conformance pass on the r3 edits)

| Sev | Finding | Where fixed |
|---|---|---|
| MED | Overrides W9 row still said `Audience` (= `IssueCheck`); the F6a create-404 reversal was not in it | Overrides |
| LOW | GitHub, CLI and screen-spec token-requirement surfaces not named | F10, F11 |
| LOW | Gates 5 and 6 omitted the audience and identity GETs on updates | gates 5, 6 |
| LOW | F6a "a 403 is `forbidden` directly" contradicted F8 | F6a |

No new claims about existing code were introduced in r4. Every line cited came from the
auditor's own read.
