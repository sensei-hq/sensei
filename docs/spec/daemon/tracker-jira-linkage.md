---
name: Jira tracker linkage (local)
status: reviewed
date: 2026-10-15
parent: docs/prd/modules/04-team/03-dojo-project-planning/README.md (T3) · docs/design/integrations-and-gating-requirements.md §2, §5
analysis: docs/analysis/2026-10-15-jira-ticket-linkage.md
---

# Tracker integrations — Jira first (T-1a + T-1d, local slice)

> File kept at its original path; the scope is now the **tracker integration seam**
> with **Jira as the first adapter**. GitLab is the next adapter (see "Future
> adapters").

## Purpose

Connect a sensei **project** to a Jira Cloud board **read-only**, mirror its sprints
and issues into the local DB, link the project's AI sessions to those issues by the
issue key found in the session's **branch** or **prompts**, and report per-ticket and
per-sprint **active AI-session time** and **token usage**. Insight only — never a
gate, never keyed by person.

## Integration seam (applies to every section below)

Trackers follow the house adapter pattern (`TranscriptAdapter`, `ManifestAdapter`,
`ConfigAdapter`, `LanguageAdapter`): a trait whose methods carry **all**
provider-specific knowledge and return **common structs**, a coordinator that owns
every DB write, and a registry.

```rust
// crates/senseid/src/tracker/mod.rs (as built; `match_links` lands with F3)
#[async_trait] pub trait TrackerAdapter: Send + Sync {
    fn kind(&self) -> TrackerKind;                       // `tracker_kind` enum value
    /// Parse + canonicalise the provider's connect body (URL policy, board id).
    fn connect_params(&self, body: &serde_json::Value) -> Result<ConnectParams, ParamError>;
    /// Auth scheme for the shared GET transport.
    fn auth(&self, account: &str, token: &ApiToken) -> AuthScheme;
    /// Prove the credential and the board; learn the scope and board name.
    async fn validate(&self, t: &dyn GetTransport, p: &ConnectParams) -> Result<Validated, ValidateError>;
    /// Everything the coordinator needs, in common shapes. `prev` lets the adapter skip
    /// immutable data (e.g. closed sprints' members).
    async fn fetch_snapshot(&self, t: &dyn GetTransport, board: &BoardScope, prev: &PrevState)
        -> Result<Snapshot, TrackerError>;
    /// F3: pure — which known tickets this session's text refers to.
    fn match_links(&self, session: &SessionText, known: &[KnownTask]) -> Vec<Link>;
    /// R11: pure — keys in this session's text shaped like this board's tickets (a
    /// prefix the board already uses) that match no known ticket. Every returned key
    /// is in the adapter's validated key shape (Jira: `[A-Z][A-Z0-9_]*-[0-9]+`).
    fn unknown_keys(&self, session: &SessionText, known: &[KnownTask]) -> Vec<String>;
    /// R11: fetch these tickets, scoped to the board — a key not on the board is
    /// simply absent from the result, never an error.
    async fn fetch_tasks(&self, t: &dyn GetTransport, board: &BoardScope, keys: &[String])
        -> Result<Discovered, TrackerError>;
}
pub struct Discovered { pub tasks: Vec<TaskRow>,
                        pub members: Vec<(String, String)> }   // (sprint ext id, task ext id)
pub struct Snapshot { pub board_name: Option<String>, pub sprints: Vec<SprintRow>,
                      pub tasks: Vec<TaskRow>,
                      pub members: Vec<(String, Option<Vec<String>>)> } // sprint ext id → member ext ids; None = unchanged
pub struct SessionText { pub session_id: Uuid, pub branches: Vec<String>, pub prompts: Vec<String>,
                         pub repo_key: Option<String> }    // F3; GitLab scopes `#12` by repo
pub struct KnownTask { pub external_id: String, pub key: String,       // one row per KEY ALIAS
                       pub scope: Option<String>, pub current: bool }  // current: the task's key now
pub struct Link { pub session_id: Uuid, pub task_external_id: String, pub source: LinkSource,
                  pub matched_key: String, pub mentions: u32 }
fn adapters() -> &'static [&'static dyn TrackerAdapter];     // registry, like adapters/manifest.rs
fn adapter_for_kind(kind: TrackerKind) -> Option<&'static dyn TrackerAdapter>;
```

The coordinator stores each `TaskRow.key` **exactly as the adapter gives it**; key
form is the adapter's (Jira upper-cases; GitLab keeps `group/proj#12`).

- **Provider-neutral (written once):** tables (R2), credentials (R3), the GET-only
  transport + error mapping + 429 policy (R4), sync coordinator (R5), relink
  orchestration (R6), usage (R7), routes (R8), status (R9), scheduling.
- **Per adapter:** connect params and URL policy, auth scheme, endpoints and
  pagination, what a "sprint" is, ticket-key matching.
- v1 registers **one** adapter, `JiraAdapter` (`tracker/jira.rs`).

## Overrides of parents

Rows marked **T3** override the parent design and are recorded in
`docs/backlog.md` (Jira entry). Rows marked **A** refine this slice's own analysis;
the analysis carries a "superseded in part" note pointing here, and they are not
backlog items.

| Src | Parent says | This spec | Why |
|---|---|---|---|
| T3 | §2.2 / PRD: a project may have several trackers; one owns sprints, the others still supply issues | **one tracker per project** in v1; secondary (issue-only) trackers deferred | v1 is Jira-only; a board spanning several Jira projects already covers multi-project issue sources |
| T3 | tracker connections configured in Dōjō, project-scoped | **local daemon first**; Dōjō rollup behind F-3 | analysis D1 |
| T3 | integrations §2.3: OAuth where offered, PAT fallback | **API token only** | v1 scope; OAuth later |
| T3 | T3 §5.1 `sensei.tracker_links` (repo → key regex, cursor) | **dropped**; repo binding = `sensei.repositories_in_projects` (R1); cursor/status on `trackers` | the junction already is "THE one place a project's membership lives" |
| T3 | T3 PRD: key pattern **per repo**, sources **all three selectable** | key prefixes **derived from synced keys** (R6); sources fixed to `branch` + `prompt` | matching against synced keys makes a user regex redundant; toggles deferred to T-1c |
| T3 | T3 §2.5 sources branch · commit · prompt | `commit` deferred (enum value reserved) | needs new capture (analysis D4) |
| T3 | T3 §5.1 `tracker_tasks.assignee` | no assignee | analysis D9 |
| A | Analysis D10 window `[start, end)` | R7 window by sprint **state** | overrunning active sprints; future sprints with planned dates |
| A | Analysis D11 issues "via JQL search" | Agile board endpoints (R4) | board scope = the configured board; sprint membership comes from the same API |
| A | Analysis §3.2 `…/usage` sub-routes, `sessions_missing_*` fields | usage embedded in list/detail responses (R8); per-field `_n` counts (R7) | one read per screen; per-field `n` is more precise than one "missing" count |
| A | Analysis §3.2 #1 "≥1 bound repo" at connect | binding is the project's existing membership (R1) | follows from the `tracker_links` row |
| A | Analysis §4 "current sprint from the Agile API" | sprint **membership set** from per-sprint issue lists (R5) | no reliance on a per-issue sprint field |
| A | Analysis D11 statuses `connected`/`stale`/`error`, "message kept" | adds `pending` (never synced); `last_error` is a fixed code, never the raw message (R2, R9) | a raw reqwest error carries the URL and may carry issue text |

## Resolved design

### R1 · Data model and scope

```
sensei project ──< repositories        (existing: repositories_in_projects; grouped by
      │                                 README front matter `project:` or the API)
      └── 1 tracker  (one board: Jira today, GitLab next)
              └──< sprints
                      └──< tickets      (tracker_task_sprints; a ticket carried over
                              │          sits in each sprint that lists it; its
                              │          current_sprint_id is the one it is in now)
                              └──< session links ──> AI sessions from ANY of the
                                                     project's repos
```

- A **sensei project** groups one or more repositories (existing
  `sensei.repositories_in_projects`). Several repos join one project through README
  front matter `project: <name>` (`tasks/handlers/repo_scan.rs` `resolve_project`)
  or the projects API.
- A project connects to **exactly one** tracker board (its **tracker**), and a board is
  connected to **exactly one** project: `trackers.project_id` is unique, and so is
  `(kind, base_url, board_ref)`. Connecting a board that another project already uses
  → 409 `board_already_connected` (naming that project). This **narrows** T3 §2.2,
  which allows extra issue-only trackers per project (deferred; see overrides).
- A tracker holds **many sprints**; a sprint holds **many tickets**. A ticket carried
  over between sprints appears in each sprint that lists it (Jira's own model), with
  `current_sprint_id` naming the one it is in now (R5).
- **Candidate sessions** of a tracker: sessions whose repo anchor belongs to the
  tracker's project, from **any** of its repos:
  `select s.id from activity.sessions s join sensei.folder_projects fp on
  fp.folder_id = s.repo_folder_id where fp.project_id = tracker.project_id`.
  `folder_projects` is the existing view over `folders ⨝ repositories_in_projects`.
- Joined on `repo_folder_id`, not the raw `folder_id`, because the sessions DDL
  says "Attribution keys off repo_folder_id, not this": `folder_id` is the cwd,
  which is often a subfolder; `repo_folder_id` is the repo anchor. **Both** are
  nullable and `on delete set null` — a pruned anchor un-candidates its sessions
  (next bullet). Existing reads are
  mixed: most filter the single-valued `s.project_id`; some (`sessions.rs`
  `reset_project_sessions_for_reenrichment`, `get_project_tool_breakdown`;
  also `metrics.rs`, `api/routes.rs`) join
  `folder_projects` on `s.folder_id`. The single-valued column can't serve a
  repository that belongs to two projects (the junction's DDL comment measured 4
  such repos, 2026-09-30), so this spec uses the view.
- A session with NULL `repo_folder_id`, or whose anchor folder has NULL
  `repository_id` (not yet resolved by the scanner), is not a candidate for any
  tracker (it appears in no partition).

### R2 · Tables (DDL under `database/ddl/table/sensei/`, enums under `database/ddl/enum/sensei/`)

| Table | Columns | Keys / notes |
|---|---|---|
| `sensei.trackers` | `id uuid pk`, `project_id uuid not null → projects on delete cascade`, `kind tracker_kind not null` (enum: `jira`; future `gitlab`), `base_url text not null` (adapter-normalised), `account text not null` (adapter-defined: Jira = email; GitLab = username), `external_scope text not null` (Jira project key; GitLab group/project path), `board_ref text not null` (Jira board id; GitLab board / iteration-cadence / milestone scope), `params jsonb not null default '{}'` (adapter-specific extras, never secrets), `secret_service text not null`, `last_attempt_at`, `last_success_at`, `last_error tracker_error` (enum, nullable), `discovered_through timestamptz` (R11 watermark: transcript turns ingested up to here have been checked for unknown keys; set to `now()` at connect, since the first full sync covers the past), `created_at` | `unique(project_id)`; `unique(kind, base_url, board_ref)`. No secret in the row. `after delete` and `after update of secret_service` triggers → tombstone of the old value (R3), defined with its function in `database/ddl/function/sensei/tracker_secret_tombstone.ddl` (the repo's trigger convention, e.g. `historize_memories.ddl`). |
| `sensei.tracker_secret_tombstones` | `secret_service text pk`, `account text not null`, `not_before timestamptz not null`, `created_at` | drained by disconnect and by the tick (R3) |
| `sensei.sprints` | `id uuid pk`, `tracker_id → trackers cascade`, `external_id text not null`, `seq bigint not null` (adapter-supplied creation order — Jira: the numeric sprint id; never compare `external_id` text), `name text not null`, `state text not null` (common states `future`/`active`/`closed`; adapters map theirs onto these), `start_at`, `end_at`, `complete_at` (nullable), `members_fetched_state text` (state at last membership fetch), `removed_at`, `last_seen_at` | `unique(tracker_id, external_id)` |
| `sensei.tracker_tasks` | `id uuid pk`, `tracker_id → trackers cascade`, `external_id text not null` (provider issue id), `key text not null` (provider's canonical display key — Jira `AUTH-420` upper-cased; GitLab `group/project#12`), `scope text` (nullable; GitLab project path the key is relative to), `summary text`, `issue_type text`, `status text`, `current_sprint_id uuid → sprints on delete set null`, `removed_at`, `last_seen_at` | `unique(tracker_id, external_id)`; index `(tracker_id, key)` |
| `sensei.tracker_task_sprints` | `task_id → tracker_tasks cascade`, `sprint_id → sprints cascade` | `pk(task_id, sprint_id)`; index `(sprint_id)` — which sprints list the issue |
| `sensei.session_task_links` | `session_id → activity.sessions cascade`, `task_id → tracker_tasks cascade`, `source link_source not null` (enum: `branch`, `prompt`, `commit` — `commit` never written in v1), `matched_key text not null`, `mentions integer not null check (mentions > 0)`, `linked_at timestamptz not null default now()` | `pk(session_id, task_id, source)`; index `(task_id)` |
| `sensei.tracker_unknown_keys` | `tracker_id → trackers cascade`, `key text not null`, `checked_at timestamptz not null default now()` | `pk(tracker_id, key)` — R11's negative cache: keys the board was asked for and does not have. Re-asked only after `UNKNOWN_KEY_TTL` (6 h); a full sync deletes the rows of every key it now has |
| `sensei.tracker_task_keys` | `task_id → tracker_tasks cascade`, `key text not null`, `first_seen_at timestamptz not null default now()` | `pk(task_id, key)`; index `(key)` — **every key a ticket has ever had** (its current key included), so a renamed or moved issue still matches text written with its old key |

`tracker_error` enum: `unauthorized`, `forbidden`, `not_found`, `rate_limited`,
`http_error`, `transport`, `decode`, `redirect`, `token_missing`,
`credential_store`, `unsupported_tracker`, `bad_request` (a 400: the provider rejected
the request, e.g. R11's search for a key that exists nowhere; 5xx stays `http_error`), `db` (a database failure during a sync,
recorded like the others so status can't read `connected` while every sync fails). `last_error` and sync/connect failures caused by the provider or the
credential store carry only these codes — never a raw error string (a reqwest error
carries the URL; a body may carry issue text). R8 adds request-validation codes of
its own (`invalid_base_url`, `board_not_found`, adapter mismatch codes such as Jira's `board_project_mismatch`,
`board_already_connected`, `busy`),
which are likewise fixed strings.

The `dojo` deploy scope is an explicit `includes:` list; the `default` (daemon)
scope `excludes: [dojo, …]`. None of these tables reach the Dōjō DB.

### R3 · Credential

- Token stored via `gateway_keys::keychain_set(service, account, token)`, with
  `service = "com.sensei.tracker.{kind}.{credential_id}"` (a **fresh uuid on every
  connect and re-connect**) and `account` = the adapter's canonical account. A keyring entry is
  never overwritten in place, so a failure part-way through a swap can't destroy
  the token currently in use.
- `keychain_get` mapping (all five `KeychainError` variants): `NotFound` →
  `token_missing`; `Unavailable`, `Store`, `CommandFailed`, `Io` →
  `credential_store`. No fallback value.
- **Every keyring call is blocking** (`gateway_keys` docs: "callers in an async
  context must wrap them in `tokio::task::spawn_blocking`"). The production
  `SecretStore` runs each call in `spawn_blocking` under `tokio::time::timeout(5 s)`.
  A timeout (e.g. a locked Secret Service waiting on an unlock prompt) →
  `credential_store`. **No keyring call is ever made while a DB transaction is
  open.**
- The request type holds the token in a newtype `ApiToken(String)` whose `Debug`
  prints `ApiToken(***)`; request structs are never logged.
- **Every keyring entry that no tracker row references has a tombstone** in
  `sensei.tracker_secret_tombstones(secret_service pk, account, not_before)`. The
  keyring can't be listed, so the tombstone is the only record of an entry to clean
  up. Two triggers on `sensei.trackers` write tombstones for the **old** value, with
  `not_before = now()`: `after delete`, which covers disconnect and the project
  cascade, and `after update of secret_service`, which covers every re-connect,
  including concurrent ones. No code path writes the old-value tombstone by hand.
- **Draining** (each `tracker_sync` tick, and best-effort right after a commit),
  per due tombstone (`not_before <= now()`), with **no transaction open across the
  keyring call**:
  1. If a `trackers` row still references its `secret_service`, **delete the
     tombstone row** without touching the keyring (defence in depth: any later
     change of that secret re-tombstones it through the triggers); otherwise
  2. `keychain_delete` (idempotent: a missing entry counts as success; a bounded
     call, see above);
  3. `delete from tracker_secret_tombstones where secret_service = $1` (autocommit).
  A keyring failure is logged with the service name and **pushes `not_before` back
  15 min** (`DELETE_RETRY`), so a permanently failing entry can't hold the head of
  the queue; the tick then reports failure (`"<n> secret deletions failed"`). A
  drain takes at most 500 due rows, oldest `not_before` first, over services with
  the `com.sensei.tracker.` prefix. The best-effort drain after connect/disconnect
  runs **in the background**, never on the request path. Two drains racing on one
  tombstone are harmless, because both steps are idempotent.
- **Connect / re-connect** (new tracker, same board, new account or new board):
  1. Commit a **pending** tombstone for the new `(service, account)` with
     `not_before = now() + 10 min`. It's a grace period far longer than steps 2–3
     take (the keyring call is bounded at 5 s; the step-3 transaction makes no
     network or keyring calls), so a drain can't remove an entry that's still being
     connected.
  2. `keychain_set(new service, account, token)`. On failure → 500
     `{error:"credential_store"}`. The pending tombstone is drained once its grace
     period ends.
  3. One transaction: `select … from sensei.projects where id = $1 for update`
     (serialises connects for the project). Then:
     - **Same board / new tracker:** upsert the row with the new `secret_service`
       and `account`, and **reset `last_success_at` and `last_error` to NULL**, so
       the status is `pending` until the new credential's first sync. The update
       trigger tombstones the previous secret.
     - **Different board:** delete the old row (the delete trigger tombstones its
       secret; the cascade drops the old board's sprints/tickets/links), then insert
       a new row with a new `id`.
     - `delete from tracker_secret_tombstones where secret_service = $new`. The
       transaction requires **exactly one** row deleted, else roll back and return
       500, so the row never points at a secret a drain already removed.
  4. Commit. On failure the old row and secret are untouched and in use, and the new
     secret keeps its pending tombstone. On success, the old secret's tombstone is
     drained as a best-effort step.
- Credential calls go through a narrow `SecretStore` trait (`set/get/delete`) with
  one production impl delegating to `gateway_keys::keychain_*` and an in-memory impl
  for tests. It is an adapter, not a second store.

### R4 · Transport (shared) and the Jira adapter

**Shared, provider-neutral (`tracker/http.rs`):**

- Base-URL policy common to every adapter, `tracker::http::normalize_base_url(raw,
  host_rule)`. It **builds on the existing `api::util::require_secure_url`** (parse +
  https), which it calls, rather than parsing again. On top of that:
  - Refuse that helper's loopback-`http` exemption: trackers are https-only.
  - Reject userinfo, IP-literal hosts, and an explicit port other than 443.
  - Discard the path, query and fragment. This rule is for an explicit `base_url`
    only; a `board_url` keeps its query until the adapter has read the board id
    from it.
  - **Canonical output:** `"https://" + host` (lowercased; IDN in punycode, as
    `reqwest::Url` yields it), with no trailing slash, port or path.
  - The adapter's **host allow-rule** comes last. Jira: the host has the form
    `<label>.atlassian.net` or `<label>.jira.com` (older Jira Cloud sites, e.g.
    `senecaglobal.jira.com`), with at least one label before the domain. So bare
    `atlassian.net`/`jira.com`, `x.atlassian.net.evil.com` and the trailing-dot form
    `acme.atlassian.net.` are all rejected. (Amended 2026-10-17: the user's own site
    is on `jira.com`, which the first rule refused.)
  - Anything else → 422 `{error:"invalid_base_url"}`.
- Transport:

  ```rust
  #[async_trait] pub trait GetTransport: Send + Sync {
      async fn get_json(&self, path: &str, query: &[(&str, String)]) -> Result<serde_json::Value, TrackerError>;
  }
  ```

  One method; the adapter's `AuthScheme` is applied by the production impl. Production impl: a `reqwest::Client` built from a new shared
  `http::bounded_builder()` (connect 5 s, total 30 s — extracted from
  `federation::http_client()`, which then calls it) **plus**
  `.redirect(reqwest::redirect::Policy::none())`. A build failure is an error, not
  a fallback client. Any 3xx → `TrackerError::Redirect`.
- `TrackerError` → `tracker_error`: 401 `unauthorized`, 403 `forbidden`, 404
  `not_found`, 429-exhausted `rate_limited`, other non-2xx `http_error`, network
  `transport`, JSON shape `decode`, 3xx `redirect`.
- **429:** wait time from `Retry-After`. A decimal integer (as `u64`) means
  seconds. An HTTP-date means `max(0, date − now)`. Anything else, including a
  negative number or overflow, means 5 s. The result is clamped to [0, 60] s. At most
  3 retries per request during sync; **none during connect's `validate`** (fail fast
  with `rate_limited`, so connect stays interactive).
- The coordinator and adapters are written against `&dyn GetTransport`; tests use an
  in-memory fake.

**Jira adapter (`tracker/jira.rs`):** auth = HTTP Basic `account(email):token`;
connect params `{board_url?, base_url?, board_id?, email, api_token, project_key?}`:
- `board_url` (what the desktop app sends) is the board's address, e.g.
  `https://acme.atlassian.net/jira/software/projects/AUTH/boards/42` or
  `…/secure/RapidBoard.jspa?rapidView=42`. The adapter derives `base_url` (common URL
  policy) and `board_id` (the path segment after `/boards/`, or the `rapidView`
  query parameter in any position; trailing segments like `/backlog` and other query
  params are ignored). It is an alternative to `base_url` + `board_id`.
  - Neither form present, no id found, or both forms present → 422
    `invalid_base_url`.
  - **Board id canonicalisation** (every source: `/boards/`, `rapidView`, the
    `board_id` field): parse as `u64` and reject `0` or a non-number (422). Store
    the decimal form, so `042` and `42` are the same `board_ref` and the unique key
    holds.
- `email` is **required**: Jira Cloud API tokens authenticate only as Basic
  `email:token`.
- `project_key` is **optional**. If absent, `external_scope` = the board's
  `location.projectKey` from `GET …/board/{id}`, or the board name when the board
  has no single project. If present, it must match (422 `board_project_mismatch`).
- `board_ref = board_id`; the board's name from `GET …/board/{id}` is stored as `params.board_name` and returned as `board_name` (refreshed on each sync). Endpoints (Jira Cloud; paths from Atlassian's Agile REST board-API operations index,
2026-10-15; response field names are the spec's **assumption until the F1.1 fixture** from a real tenant confirms them — C11, still UNCHECKABLE):

| Purpose | Call |
|---|---|
| validate credential | `GET /rest/api/3/myself` |
| validate board | `GET /rest/agile/1.0/board/{boardId}` |
| sprints | `GET /rest/agile/1.0/board/{boardId}/sprint?startAt&maxResults=50` |
| board issues | `GET /rest/agile/1.0/board/{boardId}/issue?fields=summary,issuetype,status&startAt&maxResults=100` |
| sprint members | `GET /rest/agile/1.0/board/{boardId}/sprint/{sprintId}/issue?fields=summary&startAt&maxResults=100` |

Pagination: advance `startAt` by the number of items returned; stop when
`isLast == true`, or `startAt >= total`, or a page returns 0 items.

### R5 · Sync coordinator — `tracker::sync::sync_tracker(pg, locks, tracker_id, adapter, transport, deadline)`

The coordinator is provider-neutral; step 2 is the adapter's `fetch_snapshot`
(for Jira as described; another adapter decides its own fetches but must return the
same `Snapshot` and honour "all-or-nothing").

**No DB transaction or connection is held across network or keyring calls.** The
sync runs in five steps (guard → fetch → compute links → apply → outcome):

1. **Guard.** Try the per-tracker in-process lock (`TrackerLocks`, a map of
   `tokio::sync::Mutex` keyed by tracker id, in `AppState` and shared with the
   scheduled tick; the daemon is the only writer). If it's held → `Busy` (manual
   sync → 409; a tick skips this tracker without recording an error). Then write
   `last_attempt_at = now()` (autocommit).
2. **Fetch, with no DB connection checked out.** Steps 2 **and 3** run under one
   whole-sync deadline (`tokio::time::timeout(10 min)`); overrunning maps to
   `transport`. For Jira: all sprints and all board issues (every page), then the
   member list of every sprint that **needs it**: state ≠ `closed`, or closed and
   (`members_fetched_state` is NULL or ≠ `closed`). A closed sprint whose members
   were fetched after it closed is never fetched again.
3. **Compute links, with no transaction open** (F3), under the same deadline:
   - `known` = every **key alias** of the tracker's stored tasks
     (`tracker_task_keys`, removed tasks included) **plus** the snapshot's tasks'
     current keys, as `KnownTask { external_id, key, scope, current }`. Two tasks claiming
     one key: the snapshot's task wins, then the non-removed one, then the later
     `last_seen_at`.
   - Candidate sessions are read in **chunks of 200** (`load_link_inputs(tracker_id,
     after, limit)`, keyset-paged by session id). Per session: `SELECT DISTINCT
     git_branch` and each turn's `left(user_text, 4096)` — never `assistant_text`.
   - `adapter.match_links` runs on a blocking thread (`spawn_blocking`) and returns
     links that name tasks by `external_id`. Newly synced tickets have no row id
     yet, and don't need one.
4. **Apply: one short transaction** in `db/pg_store/trackers.rs`
   (`apply_tracker_snapshot(id, secret_service, snapshot, params_patch, links)`),
   only if steps 2–3 fully succeeded. It sets `SET LOCAL statement_timeout =
   '60s'` and makes no network or keyring calls:
   - `select … from sensei.trackers where id = $1 and secret_service = $2 for
     update`, `$2` being the credential the sync started with. If the row is gone
     (disconnected, or re-connected to another board, mid-sync) **or re-connected
     with a new credential** → `Gone` (manual sync → 404), and nothing is written:
     a superseded sync never records its success or failure on the new connection.
     `last_error` writes are conditional on the same `secret_service`.
   - Upsert sprints and tasks by `(tracker_id, external_id)`. A changed key
     overwrites `key`, and **every current key is added to `tracker_task_keys`**
     (`on conflict do nothing`), so old keys are kept as aliases. Set
     `last_seen_at = now()` and clear `removed_at` on seen rows.
   - Set `removed_at = now()` on rows not seen **and `removed_at IS NULL`**.
     **Delete the `tracker_task_sprints` rows of every removed sprint.**
   - For each fetched sprint, replace its `tracker_task_sprints` rows and set
     `members_fetched_state = state`. A member whose `external_id` has no task row
     (not among the board's issues) is **skipped and counted** (`skipped_members`),
     never inserted, and never an FK error.
   - `current_sprint_id` per task = among **non-removed** sprints listing it,
     `active` > `future` > `closed`. Ties go to the later `start_at` (NULL sorts
     last), then the higher `seq`. No such sprint → NULL.
   - **Relink** (R6): `SELECT id FROM activity.sessions WHERE id = ANY($links'
     sessions) FOR KEY SHARE` — the sessions still present, locked against a
     concurrent prune — then delete this tracker's `session_task_links` and insert
     the step-3 links for those sessions only, in one `INSERT … SELECT FROM
     unnest(…)` that resolves `external_id → id` in SQL. A session pruned since
     step 3 is skipped, never an FK error.
   - Store `params.board_name`, set `last_success_at = now()` and
     `last_error = NULL`, then commit.
   Sprints, tasks, memberships and links commit **together or not at all**.
5. **Outcome.** Success is recorded inside step 4. Any failure in steps 2–4 →
   `last_error = <code>` (autocommit, conditional on the starting `secret_service`):
   the provider's or credential's code, or `db` for a database failure. No data
   rows change.

Scheduling: new task **`tracker_sync`** — entry in `tasks::schedule::SCHEDULABLE`,
in `DESCRIPTIONS` (`api/handlers/scheduled_tasks.rs`), a line in
`database/import/staging/schedules.jsonl` (`interval_secs: 1800`), and a `tick()` in
`tasks/tracker_sync.rs`, whose `pub fn spawn(pg)` calls
`tasks::ticker::run_scheduled(pg, "tracker_sync", …)` like `forge_token_check.rs`,
**and** a `crate::tasks::tracker_sync::spawn(...)` call at daemon start in
`api/server.rs` next to `forge_token_check::spawn`. `SCHEDULABLE` stays sorted
(test `schedulable_names_are_unique_and_sorted`). A tick drains secret tombstones
(R3), then syncs every tracker in turn. One tracker's failure is recorded on its
row and does not stop the others. The tick returns `Err("<n>/<m> trackers failed")`
when any tracker failed, so `ticker` records a failed run instead of a clean one.
Module wiring: `mod tracker;` in `main.rs`, `pub mod tracker_sync;` in
`tasks/mod.rs`.

### R6 · Linking (adapter-owned matching + coordinator-owned relink)

The coordinator gathers `SessionText` (session id, branches, prompts, `repo_key`) and
`known: &[KnownTask]` (R5 step 3), calls `adapter.match_links`, and writes the
result. The rules **below are the Jira adapter's** `match_links`; the source
semantics (`branch`, `prompt`, `mentions`) and "link only to a known key" are
**common to all adapters**.

- `known` holds **every key alias** of every task, removed ones included (links to
  a removed or renamed issue survive and stay attributable; reads flag removed).
- Prefixes `P` = distinct prefixes (text before the last `-`) of the keys in
  `known`. Jira project keys never contain `-` but may contain `_`.
- **Matching is on the original bytes, ASCII-only case-insensitive**
  (`eq_ignore_ascii_case`): no Unicode case folding, so no lookalike match (the
  Kelvin sign `K` is not `K`) and no byte-offset shift (`ß`, `İ` cannot panic).
- At each position, try prefixes **longest first**. A match is `p` + `-` +
  `[0-9]+` (digits greedy), accepted only if the byte before it is not an ASCII
  letter or digit (or it is the start). `_`, `/`, `-`, space, punctuation and
  non-ASCII bytes separate, so `feature_AUTH-420` links. Positions are scanned left
  to right and a matched span is consumed, so with `MY_PROJ` known `MY_PROJ-12`
  never also yields `PROJ-12` (with only `PROJ` known it yields `PROJ-12`).
- The key is `UPPER(p)-digits` with the digits **as written** (`AUTH-0420` is not
  `AUTH-420` and does not link). That string is `matched_key`.
- Link iff the key ∈ `known`. With no `known` keys nothing links (`UTF-8`, `AES-256`
  can't match unless `UTF`/`AES` is a synced prefix *and* that issue exists).
- `branch`: each **distinct** `git_branch` of the session; `mentions` = number of
  distinct branches containing a key of that task.
- `prompt`: each turn's `user_text` (first 4 KB); `mentions` = total accepted
  occurrences.
- One `Link` per (session, task, source). If several aliases of one task match
  under one source, `matched_key` is the alias with the most mentions (ties: the
  task's current key, then lexical order), and `mentions` is their sum.

No `client_session_id` or no turns → no links.

**Relink** (computed in R5 step 3, written in R5 step 4): a full recompute of this
tracker's links over all candidate sessions, deterministic and idempotent.
*Incremental relinking (a per-tracker watermark; full recompute only when the
known-key prefix set changes) is deferred — backlog.*

### R7 · Metrics (computed at read; nothing materialised)

**Usage** of a distinct session set `S`:

| Field | Definition |
|---|---|
| `sessions` | `|S|` |
| `by_source.branch`, `by_source.prompt` | sessions in `S` with ≥1 link of that source (to the ticket, for ticket usage; to any sprint member, for `linked_sprint`) — may sum past `sessions` |
| `shared` | sessions in `S` linked to ≥2 distinct tasks of this tracker |
| `active_secs`, `active_n` | `Σ extract(epoch from duration)` over sessions with `duration IS NOT NULL`; `active_n` = that count |
| `claude_secs`, `claude_n` / `user_secs`, `user_n` | the same rule over `claude_time` / `user_time` (R12): active time split into Claude working and you interacting |
| `wall_secs`, `wall_n` | `Σ (completed_at − started_at)` over sessions with `completed_at IS NOT NULL` — each session's clock time, idle inside it included |
| `first_at`, `last_at` | `min(started_at)` over `S`; `max(completed_at)` over sessions that have one (`null` when none has). The **cycle time** shown is `last_at − first_at`: from the first session's start to the last one's end, the gaps between sessions included |
| `tokens_fresh`, `cache_read`, `cache_write`, `tokens_out`, `tokens_reasoning` | each `Σ field` over sessions where **that field** IS NOT NULL, each with `<field>_n` |
| `metered_cost`, `metered_cost_n` | same rule |

A sum over zero non-NULL values is `null` (not `0`); its `_n` is `0`. `tokens_in` is
not reported (folded total incl. cache).

**Ticket usage:** `S` = distinct sessions with any link to the task **that are
currently candidates of the tracker (R1)**, i.e. from any repo currently in the
project. The read-time intersection means removing a repo from the project takes
effect immediately, without waiting for the next relink. Also returns `by_repo:
[{repository_id, name, usage}]` (each session has exactly one repo anchor, so these
parts sum to the total).

**Backlog usage:** the **backlog** is the tracker's non-removed tickets that no
non-removed sprint lists (a ticket left only in a closed sprint is that sprint's, not
the backlog's). `S` = distinct candidate sessions (R1) with a link to any backlog
ticket; usage as above, `by_source` counting links to backlog tickets. No window: the
backlog has no dates, so its usage is all-time.

**Sprint usage:**
- `state = 'future'` → `usage: null, reason: "not_started"`. A non-future sprint without a `start_at`, or a closed one with neither `complete_at` nor `end_at`, has no window → `reason: "no_dates"` (never a `0`).
- Window: `active` → `[start_at, now())`; `closed` → `[start_at, coalesce(complete_at, end_at))`.
- `candidates_in_window` = candidate sessions (R1) with `started_at` in the window.
- Partition: `linked_sprint` = links to ≥1 task in this sprint's
  `tracker_task_sprints`; `linked_other` = links to tasks of this tracker but none in
  this sprint; `unlinked` = **no link to any task of this tracker** (links to another
  tracker's tasks don't count). Each part carries a usage object.
- Candidates span all the project's repos.

### R8 · API (`api/handlers/trackers.rs`)

All routes below — **reads included**, because they return client issue summaries —
sit behind one guard layer built from **existing** code:

- **Origin:** reuse `origin_may_mutate` + `ALLOWED_ORIGINS` from
  `api/handlers/repositories.rs` (no Origin → allow; the Tauri app and its dev
  origins → allow; any other Origin → 403). Extract both into a shared
  `api/origin_guard.rs` and have `repositories.rs` call it from there — one
  allow-list, not two.
- **Host (new, added to the same module):** 403 unless `Host` is `127.0.0.1`,
  `localhost` or `[::1]` (any port). Closes DNS rebinding, where a page's request
  is same-origin to the browser and so carries no `Origin` on a GET.
- Exposed as an axum middleware `local_caller_guard` applied to the tracker route
  group; the existing `repositories.rs` behaviour is unchanged.

| Route | Behaviour |
|---|---|
| `GET /api/projects/{id}/trackers` | `[{id, kind, base_url, account, external_scope, board_ref, board_name, status, last_success_at, last_error, counts:{tasks, sprints}}]`; `[]` when none. Unknown project → 404. |
| `POST /api/projects/{id}/trackers/{kind}/connect` | `kind` resolved by `adapter_for_kind`; unknown or unregistered → 422 `{error:"unsupported_tracker"}`. Body = the adapter's connect params (Jira: R4, e.g. `{board_url, email, api_token}`). Validate and normalise the URL (R4) → 422. The normalised `(kind, base_url, board_ref)` already connected to **another** project → 409 `{error:"board_already_connected", project:{id, name}}`. The project already connected (same or different board) → **re-connect**: fields and token replaced after validation; a different board drops the old board's sprints/tickets/links (old tracker row replaced). Then `adapter.validate`: credential rejected → 401 `{error:"unauthorized"}`; board/scope missing → 404 `{error:"board_not_found", base_url, board_ref}` (the normalised values, so a UI that sent only `board_url` can name them); adapter-specific mismatches → 422 with the adapter's code (Jira: `board_project_mismatch`). Then persist (R3). Return **201** `{tracker, first_sync:{pending:true}}` straight away, and start the first sync in the background (`tokio::spawn`, under the R5 guard; if a sync is already running it waits it out, polling once a second for up to 15 min, then leaves it to the tick). Connect never waits on a sync, so it finishes in the time of `validate` (2 requests with no 429 retries) plus one keyring call. Clients poll `GET …/trackers` until `status` leaves `pending`. A failed first sync keeps the connection (the credential was valid), and `status` shows `error`. |
| `DELETE /api/projects/{id}/trackers` | R3 disconnect; 204; 404 if the project has no tracker. |
| `POST /api/projects/{id}/trackers/sync` | R5 now. 200 `{counts:{sprints, tasks, removed, links, skipped_members}}`; 409 `{error:"busy"}`; 502 `{error:<code>}`; 404 if no tracker. |
| `GET /api/projects/{id}/sprints` | the project's sprints, newest first (`start_at` desc, NULL last, then `seq` desc): `[{id, name, state, start_at, end_at, complete_at, tasks, candidates_in_window, linked_sprint, linked_other, unlinked, reason}]` — `tasks` = tickets the sprint lists; each part a usage object; without a window the count and parts are `null` and `reason` says why (`not_started`, `no_dates`). Removed sprints excluded. 404 `not_connected` without a tracker. |
| `GET /api/projects/{id}/backlog` | `{tickets, usage}` — the backlog's ticket count and its usage (R7 backlog usage). 404 `not_connected` without a tracker. |
| `GET /api/projects/{id}/tracker-tasks?sprint={uuid}&backlog=bool&include_removed=bool` | the sprint's tickets, the backlog's (`backlog=true`), or all the project's tickets without either — both given → 422 `sprint_or_backlog`: `[{key, summary, issue_type, status, current_sprint, sprints:[{id, name}], removed, usage}]`. `sprint` filters to its members; unknown sprint → 404 `sprint_not_found`. A ticket with no linked sessions has a zero-session usage object (sums `null`, counts `0`). |
| `GET /api/projects/{id}/tracker-tasks/{key}` | `{task, usage, by_repo:[{repository_id, name, usage}], sessions:[{id, started_at, links:[{source, matched_key, mentions}]}]}`; `key` matched case-insensitively against current keys **and aliases**; sessions are the ticket's current candidates (R1), as for usage; 404 `task_not_found` if absent or the project has no tracker. A miss in the board's key shape runs R11 discovery first: 404 `task_not_on_board` (+ `key`, `board_ref`) when the board does not have it. |
| `GET /api/projects/{id}/sessions/unlinked?sprint={uuid}` | the sprint's `unlinked` sessions `[{id, started_at, task}]`, oldest first; no `sprint` → 422 `sprint_required`; unknown → 404 `sprint_not_found`; a sprint without a window → `[]`. |

Error codes the routes return (body `{error: <code>}`, extra fields where noted):

| Status | Codes |
|---|---|
| 401 | `unauthorized` (Jira rejected the credential at connect) |
| 403 | `forbidden` (the account can't read the board, at connect); `forbidden_caller` (the local-caller guard: foreign `Origin` or non-loopback `Host`) |
| 404 | `project_not_found`; `not_connected` (sync/disconnect with no tracker); `board_not_found` (+ `base_url`, `board_ref`); `task_not_found` (no ticket with that key or alias); `task_not_on_board` (+ `key`, `board_ref`; R11); `sprint_not_found` |
| 409 | `board_already_connected` (+ `project {id, name}`); `busy` |
| 422 | `unsupported_tracker`; `invalid_base_url`; `email_required`; `token_required`; `board_project_mismatch`; `sprint_required`; `sprint_or_backlog` |
| 500 | `credential_store`; `internal` (database error) |
| 502 | a `tracker_error` code from the provider (connect validation or sync): `rate_limited`, `transport`, `http_error`, `redirect`, `decode`, `not_found`, `token_missing`, … |

DB errors → 500 `internal`. No list pagination in v1 (bounded by one board). Errors
are logged with the code only.

### R9 · Status (pure fn of row + `interval_secs`, unit-tested)

In order: `error` if `last_error IS NOT NULL`; `pending` if `last_success_at IS NULL`;
`stale` if `now − last_success_at > 2 × interval_secs`; else `connected`.
`interval_secs` is read from the `tracker_sync` schedules row whether or not it is
enabled; a missing row is a 500 (the row is seeded and test-enforced).

### R10 · CLI (`crates/cli`, via `daemon_url()` + the blocking client)

- `sensei tracker connect <kind> …` — one subcommand per registered adapter; v1: `jira`.
  `sensei tracker connect jira --workspace <w> (--board-url <url> | --url <site> --board <id>) --email <e> [--project-key <K>]`
  — token from `SENSEI_JIRA_TOKEN` if set, else one line read from stdin (the help
  text notes env vars are visible to same-user processes). Never an argv flag. The
  CLI sends the URL as typed; the server normalises it (R4). On 409
  `board_already_connected` it prints which workspace owns the board. Connect uses
  `client_with_timeout(90)` (two validation requests at up to 30 s each, plus the
  keyring call). After a 201 it polls `GET …/trackers` every 2 s for up to 120 s,
  printing the first-sync result. On a polling timeout it says the sync is still
  running in the daemon; it never reports a connect failure.
- `sensei tracker sync --workspace <w>` · `sensei tracker disconnect --workspace <w>`
- `sensei tracker sprints --workspace <w>` — the workspace's sprints with ticket counts.
- `sensei tracker usage --workspace <w> [--sprint <s> | <KEY>]` — giving both is an
  error. Table: key · summary · sessions (branch/prompt) · **"active AI time"** `h:mm` ·
  fresh / cache-read / out tokens · cost (blank when unknown, not `0`), then for a
  sprint the `unlinked` count. `--by-repo` adds one row per repository.
- `<p>`: a uuid, or an exact project name from `GET /api/projects`; 0 or >1 matches
  → error listing the candidates. `<s>`: uuid or exact sprint name; same rule.

### R11 · On-demand ticket discovery (`tracker/discover.rs`)

The full sync (R5) runs every 30 min. A ticket created since then, and first named in
a Claude Code session, is fetched **when sensei reads that session** and its session
linked, without waiting for the next full sync. The full sync stays the authority:
discovery only ever **adds** (tickets, aliases, memberships, links, negative-cache
rows); it never marks anything removed and never deletes a link.

**Triggers.**
- **Ingest.** After `ingest_capture` writes turns (`upsert_transcript_turns` > 0) for a
  session that is a candidate (R1) of a tracker, enqueue `TaskKind::TrackerDiscover`
  for that tracker unless one is already pending (coalescing:
  `TaskQueue::has_pending_kind_path`, path = tracker id). The seam is
  `tracker::discover::after_ingest(pg, queue, client_session_id)`, called from
  `transcript::run_ingest_capture`. Latency: within the
  `ingest_captures` interval (300 s) plus seconds. Live session hooks are not a
  trigger in this slice.
- **Lookup.** `GET …/tracker-tasks/{key}` that misses (R8), for a key in the board's
  key shape and prefix and not in the negative cache, runs one discovery for that key
  (bounded 30 s) before answering.

**`discover(pg, locks, tracker_id, adapter, transport, only_keys)`** — steps, in the
R5 shape (no DB transaction or connection across network calls):
1. **Guard.** `TrackerLocks::try_lock` (shared with R5). Held → skip: the watermark is
   not advanced, so the next trigger retries; a running full sync covers it anyway.
   Lookup path → 409 `busy`.
2. **Read** `known` (R5 step 3 rule, aliases included), the negative cache (rows with
   `checked_at > now() − UNKNOWN_KEY_TTL`), and — ingest path — the candidate sessions
   with transcript turns ingested after `discovered_through` (chunks of 200, the R5
   read: distinct branches + `left(user_text, 4096)`, never `assistant_text`).
3. **Find:** `adapter.unknown_keys` per session (on a blocking thread), minus the
   negative cache. None → advance the watermark; **no network call.** At most
   `DISCOVER_MAX_KEYS` (50) keys per run; when more are found the watermark stays,
   so the next run continues.
4. **Fetch:** `adapter.fetch_tasks(board, keys)` under a 60 s deadline. Jira:
   `GET /rest/agile/1.0/board/{id}/issue?jql=key in (K1,…)&fields=summary,issuetype,status,sprint,closedSprints`
   — board-scoped, so only issues on the board come back. If Jira answers 400 for the
   batch with `bad_request` (claim C33: a key that exists nowhere fails the whole
   `key in` query), ask one key at a time; a key whose own request is `bad_request` is
   "not on the board". Any other failure (5xx, transport, 401…) fails the run and
   negative-caches nothing. JQL is built
   **only** from keys that passed the adapter's key-shape check (no quoting or
   escaping is ever needed, and none is attempted).
5. **Apply** — one short transaction, `apply_discovered(id, secret_service,
   discovered, links, not_found, watermark)` in `db/pg_store/trackers.rs`, conditional
   on `secret_service` (as R5 step 4): upsert the tasks (`removed_at = NULL`,
   `last_seen_at = now()`), add their keys to `tracker_task_keys`, add memberships for
   sprints sensei already has (an unknown sprint is skipped and counted; the next full
   sync brings it), recompute `current_sprint_id` for those tasks (R5 rule), insert the
   links of **every candidate session** to these tasks — computed before the
   transaction by the R5 step 3 read with `known` = the new tasks only, so a session
   ingested before the watermark (or a lookup's) links too (`match_links`;
   `on conflict (session_id, task_id, source) do update` of `matched_key`/`mentions`/
   `linked_at`), upsert `not_found` into `tracker_unknown_keys`, advance
   `discovered_through`.
6. **Failure** (provider, credential, deadline, DB): logged with its fixed code; no
   `last_error` write (R9 status stays the full sync's), nothing applied, watermark
   not advanced. The tracker's discovery then pauses `DISCOVER_BACKOFF` (5 min,
   in-memory) so a failing Jira isn't asked on every ingest.

**Full sync interplay (R5 step 4).** The full apply also deletes the
`tracker_unknown_keys` rows of every key now among its tasks, and its relink (a full
replace) supersedes any discovered link.

**Lookup answers (R8).** Found → 200 as before. Not on the board → 404
`{error:"task_not_on_board", key, board_ref}`. Wrong shape/prefix, or within the
negative cache → 404 `task_not_found` (no network call). Busy → 409 `busy`; a
provider failure → 502 with its code.

### R12 · Active time split (`tasks/handlers/analyze.rs`)

The analyzer already derives `sessions.duration` (active time) from the session's hook
events: the gaps between consecutive events, each counted when `0 < gap ≤ 30 min`.
R12 attributes each counted gap to **who the session was waiting on** when it began,
by the event that opened it:

| Event opening the gap | Gap is |
|---|---|
| `UserPromptSubmit` | Claude's (it is working on the prompt) |
| `PreToolUse`, `PostToolUse`, `SubagentStop`, `PreCompact` and any other event inside a turn | Claude's, unless a `Notification` came after the turn's latest tool event |
| `Notification` | yours (Claude is waiting for an approval or an answer) |
| `Stop`, `SessionStart`, `SessionEnd`, or any event before the first prompt | yours (reading, analysing, typing the next prompt) |

- Stored on `activity.sessions` as `claude_time interval` and `user_time interval`
  beside `duration`, and `duration = claude_time + user_time` for every session the
  analyzer writes.
- A session with **no hook events** (transcript only) gets `NULL` in all three. Before
  R12 it got `0`, a fabricated zero.
- Sessions analysed before R12 (`duration IS NOT NULL AND claude_time IS NULL`) are
  picked up again by the enrichment pass, once, so their split is filled without a
  data migration.
- Time outside Claude Code (editing in the IDE with no session open) is not seen and
  not counted.

### Out of scope

Any adapter other than Jira (GitLab is next — below); write-back to any tracker; Dōjō UI / rollup / upload (F-3); ADO / GitHub Issues; task-type
mapping; per-repo key patterns and source toggles (T-1c); commit linkage; OAuth;
Jira Server/DC; assignee or any per-person figure; discovery from live session hooks
(R11 triggers on transcript ingest and lookup only); daemon-wide route auth/CORS
(backlog). The desktop **connection** and **sprints** screens are in scope:
[`screen/project-integrations.md`](../screen/project-integrations.md),
[`screen/workspace-tickets.md`](../screen/workspace-tickets.md).

## Future adapters (not built in this slice)

**GitLab** — known differences the seam already accommodates; each needs a decision
in its own slice (recorded in `docs/backlog.md`):

| Concern | GitLab reality | Seam hook |
|---|---|---|
| What is a sprint | **Iterations** (Premium/Ultimate, cadence-based) or **milestones** (all tiers) | adapter's `fetch_snapshot` maps either onto `SprintRow`; `board_ref` names which |
| Ticket key | `#12` is per GitLab project; `group/project#12` is global; branches default to `12-title` | `match_links` gets the session's `repo_key` to resolve `#12`. `tracker_tasks.scope` is stored as `normalize_repo_key(<project web URL>)` (`db/pg_store/repo_key.rs:14`, the same lowercased `host/path` form as `repositories.repo_key`), and matched by exact equality. A session with NULL `repo_key` never resolves a bare `#12`. |
| Host | gitlab.com **or self-managed** | adapter host allow-rule: gitlab.com plus a host the user explicitly confirms at connect (SSRF review needed) |
| Auth | PAT / OAuth, `PRIVATE-TOKEN` or `Bearer` header | `AuthScheme::Header` / `Bearer` |
| States | `opened`/`closed` issues; iteration `upcoming`/`current`/`closed` | mapped onto common sprint states |

## Prerequisites (build gate)

1. **P0 — sessions exist on this machine:**
   ```sql
   select (select count(*) from activity.sessions) > 0
      and (select count(*) from activity.transcript_turns where git_branch is not null) > 0;
   ```
   Must return `t`. On 2026-10-17 it was `f` for **setup** reasons, not code:
   - sessions are synthesised only for watched folders (4 of 28 hook sessions);
   - transcripts are read only with consent (`transcripts.consent.claude_code`
     unset, so 0 turns).

   Fix: consent to Claude Code transcripts (Settings → Transcripts) and watch the
   repos to be linked.
2. **Jira tenant (human):** the user supplies a Jira Cloud site, board id and API
   token for the live gates and the C11 fixture. Build task 1 records one response
   per R4 endpoint (secrets and issue text scrubbed) under
   `crates/senseid/tests/fixtures/jira/`. Until then the build halts at that task.

## Done gate

Unattended (CI):
1. `cargo test -p senseid tracker` passes, including:
   a. `match_links` with `known = {AUTH-420, AUTH-421, MY_PROJ-12}`: `UTF-8`, `AES-256`, `CVE-2026` → none; `fix/auth-420-store` → AUTH-420/branch; `AUTH-420_fix`, `feature_AUTH-420` → AUTH-420; `xAUTH-420`, `AUTH-4201`, `AUTH-0420` → none; `MY_PROJ-12` → MY_PROJ-12; a prompt with AUTH-420 twice → `mentions = 2`; AUTH-420 and AUTH-421 → two links; empty `known` → none. With `PROJ-12` also known: `MY_PROJ-12` → only MY_PROJ-12. `\u{212A}EY-1` with `KEY-1` known → none. `ßİ AUTH-420` → AUTH-420, no panic. A renamed issue (AUTH-420 → IAM-5, same id) still links a session on `fix/auth-420` after the rename sync (`matched_key` AUTH-420). Two tasks claiming one key → the live one wins.
   b. Sync on the fake transport (fixture-shaped): 2 issue pages + 2 sprints → all rows; a second identical sync changes nothing but `last_seen_at`; an issue missing on the third sync → `removed_at` set once (unchanged on a fourth sync) **and its links still present**; a renamed key → same row, new key.
   c. Page 2 errors → zero data rows changed, `last_error` set, `last_success_at` unchanged.
   d. Issue listed by a closed and an active sprint → `current_sprint_id` = active; both rows in `tracker_task_sprints`; a closed sprint fetched after closing is not re-fetched. A task in closed S1 and future S2, then S2 is deleted in Jira → S2 gets `removed_at`, its membership rows are deleted, and `current_sprint_id` = S1. A sprint member that isn't among the board issues → skipped, `skipped_members = 1`, no error.
   d2. Failure modes: a fake transport sleeping 5 s per page → no `pg_stat_activity` row for this test's connection is `idle in transaction` during the fetch. The tracker row is deleted between fetch and apply → `Gone`, no `last_error` write, no FK error. A fake `SecretStore` that blocks → `credential_store` after 5 s (timeout injected), and the connect handler returns. A tick with one failing and one healthy tracker → the healthy one syncs and the tick returns `Err("1/2 trackers failed")`. Connect returns 201 with `first_sync.pending` before the (slow, fake) first sync finishes. A concurrent manual sync → 409 `busy`.
   e. R9 status: never synced → `pending`; failed first sync → `error`; `last_error` set together with an old success → `error`; success exactly `2×1800` s ago → `connected`; `2×1800+1` s → `stale`.
   f. 429 + `Retry-After: 1` then 200 → ok; `Retry-After` absent → 5 s, `-5` → 5 s, `120` → 60 s, `0` → 0 s, an HTTP-date 30 s ahead → 30 s, `99999999999999999999` → 5 s (clock injected); 4× 429 during sync → `rate_limited`; one 429 during connect `validate` → `rate_limited` with no retry; 302 → `redirect`.
   g2. Board-URL parsing: `https://acme.atlassian.net/jira/software/projects/AUTH/boards/42` → (`https://acme.atlassian.net`, `"42"`); `…/boards/042` → `"42"`; `…/boards/0` → 422; `…/boards/42/backlog?selectedIssue=AUTH-1` → `"42"`; `…/secure/RapidBoard.jspa?rapidView=7` → `"7"`; `…/RapidBoard.jspa?view=planning&rapidView=7` → `"7"`; both `board_url` and `base_url`+`board_id` sent → 422; `https://acme.atlassian.net/jira/software/projects/AUTH` (no board) → 422; `https://evil.com/…/boards/42` → 422. No `project_key` → `external_scope` from board `location.projectKey`.
   g. URL validation: `http://x.atlassian.net`, `http://localhost`, `https://evil.com`, `https://1.2.3.4`, `https://u:p@x.atlassian.net`, `https://x.atlassian.net:8443`, `https://atlassian.net`, `https://x.atlassian.net.evil.com`, `https://acme.atlassian.net.` → rejected; `https://acme.atlassian.net/jira/` and `https://ACME.atlassian.net:443/jira/?x=1#y` → `https://acme.atlassian.net`. `normalize_base_url` calls `require_secure_url` (no second parser).
   h. `local_caller_guard`: `Origin: https://evil.com` → 403; `Origin: tauri://localhost` → passes; `Host: evil.com` → 403; no Origin + `Host: 127.0.0.1:7744` → passes. The existing `repositories.rs` origin tests still pass after the extraction.
   i. Sentinel-token test: run connect (fake), failed sync and status with token `SENTINEL-TOKEN-XYZ`; captured logs, `last_error`, and every response body do not contain it.
   j. Sprint usage against seeded DB rows: `candidates_in_window` equals an independent `count(*)` written in the test; a session linked only to another tracker's task lands in `unlinked`; a future sprint → `not_started`; an active sprint past `end_at` includes a session started after `end_at`.
   k. Project hierarchy (seeded DB, fake transport, fake `SecretStore`): a project with 3 repos; sessions in each repo on `fix/AUTH-420-x` → AUTH-420 usage `sessions = 3`, `by_repo` has 3 entries summing to the total. Removing one repo from the project → AUTH-420 drops to 2 immediately (no resync). `GET …/sprints` returns the board's sprints; `GET …/tracker-tasks?sprint=S` returns exactly S's tickets; a ticket carried from S1 to S2 appears under both, with `current_sprint` = S2. Connecting the same board from a second project → 409 `board_already_connected`, including when typed as `https://ACME.atlassian.net/jira/software`. Re-connecting the first project to a different board removes the old board's sprints and tickets, creates a new tracker id, and tombstones only the old secret (after one tick the new one is still readable). A tick run while a connect is paused between `keychain_set` and step 3 does not delete the new secret (grace period). Two interleaved re-connects of one project leave exactly one live secret, and every other keyring entry they wrote has a tombstone. Re-connecting to the same board with the same email: the live token is never deleted, even when the commit is made to fail (fake store returns the old token). Re-connect from `e1` to `e2` with a fake store whose delete fails → a tombstone for `(old service, e1)` remains for the tick. Deleting the project removes its tracker and writes a tombstone; the next tick deletes the secret and the tombstone. A keyring delete that fails leaves the tombstone in place.
   l. **Seam test:** a test-only `FakeAdapter` (non-Jira key format `#12` scoped by `repo_key`, its own sprint shape) runs through the coordinator (sync, relink, usage) with no change to coordinator code. `repositories.repo_key` is seeded through the production `normalize_repo_key` from remotes like `git@gitlab.com:Group/Sub/Repo.git`, and the fake's `scope` comes from `https://gitlab.com/Group/Sub/Repo`. Two sessions in different repos both saying `#12` link to different tasks. A third repo with no matching scope gives 0 links, and a session with NULL `repo_key` gives 0 links.
   n. **Backlog:** a ticket no sprint lists, and one listed only by a sprint Jira deleted, are backlog; a ticket in a closed sprint is not; a removed ticket is excluded unless `include_removed`. `GET …/backlog` counts a session linked to two backlog tickets once (`sessions = 1`) and reports the ticket count; `?sprint=X&backlog=true` → 422 `sprint_or_backlog`.
   m. **Discovery (R11):** `unknown_keys` with known `{P-1}`: `P-900` → `[P-900]`; `P-1`, `UTF-8`, `Q-5` (prefix the board doesn't use), `P-9x;DROP` → none beyond the valid `P-9`; a prompt naming `P-900` twice → once. A session ingested after the watermark naming `P-900` → one board-scoped request with `key in (P-900)`, the task inserted, the session linked, the watermark advanced; nothing marked removed, no other link touched. A session with no unknown key → no request. A key the board lacks → a `tracker_unknown_keys` row, and a second discovery within 6 h makes no request. A 400 batch (fake) → one request per key; the bad key negative-cached, the good ones inserted. Lock held → skipped, watermark unchanged. A provider failure → nothing applied, no `last_error`, watermark unchanged, a second trigger within 5 min makes no request. A full sync after discovery: the discovered task stays (it is on the board), its links are recomputed, and its negative-cache row (if any) is gone. `GET …/tracker-tasks/P-900` before any ingest → discovers, 200; `…/P-404` → 404 `task_not_on_board`; `…/UTF-8` → 404 `task_not_found` with no request. Ingest wiring: `ingest_capture` writing turns for a candidate session enqueues exactly one `TrackerDiscover` per tracker while one is pending.
2. `rg -n 'Method::|\.request\(|client\.(post|put|patch|delete)\(' crates/senseid/src/tracker/` → no output (the HTTP client is GET-only; `SecretStore::delete` is a keyring call, not an HTTP verb).
2a. `rg -n -i '\b(select|insert|update|delete)\b.*\b(from|into|set|where)\b' crates/senseid/src/tracker` → no hits **outside `#[cfg(test)]` modules** (test fixtures may seed rows, as `check-sql-in-layer` allows; `make check-sql-in-layer` only matches `sqlx_core::query*` call sites, so SQL text in a `const` would slip past it).
2b. Outside `#[cfg(test)]` modules: `rg -n -i 'jira|atlassian'` over `crates/senseid/src/tracker/{http,sync,status,sprint,transport,runtime,secret,connect}.rs` (and `link.rs`/`usage.rs` once F3/F4 add them) → no output; in `tracker/mod.rs` the only hits are `mod jira;`, the `JiraAdapter` entry in `adapters()`, the `TrackerKind::Jira` variant with its `"jira"` label (the `tracker_kind` enum), and doc comments. Naming the adapter in the registry, the CLI's `jira` subcommand and `SENSEI_JIRA_TOKEN` is expected; Jira **logic** (key shape, endpoints, auth, host rule) lives only in `tracker/jira.rs`.
3. `rg -n 'api_token|SENSEI_JIRA_TOKEN' crates/senseid/src crates/cli/src` hits only the connect request type, its keyring call, and the CLI token reader; no DDL column holds a token.
3b. `rg -n 'discover::after_ingest' crates/senseid/src/transcript/mod.rs` → 1 hit (the R11 ingest trigger; its unit test can't see a missing call).
4. `tracker_sync` is in `SCHEDULABLE`, `DESCRIPTIONS` and `schedules.jsonl`; `every_schedulable_worker_has_a_description`, `every_schedulable_worker_has_a_schedule_row`, `every_schedule_row_names_a_real_worker` and `schedulable_names_are_unique_and_sorted` pass; `rg -n 'tracker_sync::spawn' crates/senseid/src/api/server.rs` → 1 hit (the tests above can't see a missing spawn).
5. `rg -n 'const ALLOWED_ORIGINS|fn origin_may_mutate' crates/senseid/src` → exactly one definition each, in `api/origin_guard.rs`.

Human (after both prerequisites; needs the user's tenant):
6. Group the project's repos (README `project:`), connect via CLI → `GET …/trackers` shows `connected`; sprint and issue counts equal the Jira board UI; a ticket worked on in two of the project's repos shows both in `by_repo`.
7. For one ticket, `active_secs` and `tokens_fresh` from the API equal a hand-written `select sum(...)` over its linked session ids (recorded in Evidence).
8. A real session on a branch carrying a real key appears under that ticket.
9. Precision sample: ≥10 real linked sessions reviewed; 0 links to non-existent keys; precision per source recorded in Evidence.
10. Bad token on connect → CLI prints the `unauthorized` error; `GET …/trackers` returns no row.

## Wrong gate

- Discovery marking a ticket removed, deleting a link, or writing `last_error`.
- A JQL string containing text that did not pass the adapter's key-shape check.
- A Jira request for a key the board's prefixes don't use, or for a negative-cached key within its TTL.

- A link to a key not in `tracker_tasks`, or a regex hit treated as a key.
- Links to a removed issue disappear on the next sync.
- `tokens_in` shown as "tokens"; a NULL field summed as `0`; a field without its `_n`.
- Ticket sums added to make a sprint total (double-counts shared sessions).
- The same tracker board connected to two projects, or a project with two boards (two sprint calendars).
- A ticket's usage missing sessions from one of the project's repos.
- A failed/partial sync changes data rows (including committing sprints/tasks without their links), or a never-successful tracker shows `connected`; a re-connect showing the previous credential's status before its own first sync.
- Any non-GET request to a tracker, a request to a host outside the adapter's allow-rule, or a followed redirect.
- Jira logic (key shape, endpoints, auth scheme, host rule) outside `tracker/jira.rs`; a coordinator change needed to add an adapter.
- A drain deleting a secret whose tombstone is still within its grace period; a tracker row pointing at a secret with no keyring entry.
- A DB transaction open across a network or keyring call; a keyring call outside `spawn_blocking` plus timeout; connect blocking on a sync; a tick reporting success when any tracker failed.
- `042` and `42` stored as different boards; a removed sprint chosen as a ticket's current sprint.
- The token in a DB row, log line, API response, `last_error`, or argv.
- A tracker route answering a foreign `Origin` or a non-loopback `Host`; a second copy of `ALLOWED_ORIGINS`.
- A session from a repo outside the tracker's project counted in any partition.
- Assignee, author, or any person attribute in usage output.
- "hours" / "time spent" without the "active AI time" label.
- A future sprint showing `0` usage; an overrunning active sprint dropping sessions after `end_at`.

## Evidence

| Gate | Result | Date |
|---|---|---|
| 7 hand-SQL equality | — | — |
| 9 precision (branch / prompt) | — | — |

## Claims (verified 2026-10-15 — independent re-derivation by `sensei-claims-verifier`, re-checked after rewrite)

| # | claim | check | expect | actual | verdict |
|---|---|---|---|---|---|
| C1 | No tracker tables/enums exist | `ls database/ddl/table/sensei \| rg -c 'tracker\|sprint'`; `to_regclass`/`to_regtype` on live DB | 0 / null | 0; all null | CONFIRMED |
| C2 | `repositories_in_projects` pk(project_id, repository_id), "THE one place a project's membership lives" | read DDL | as stated | as stated, both FKs cascade | CONFIRMED |
| C3 | session→repo via `repo_folder_id → folders.repository_id` in `session_outcomes.rs` | `rg -c 'rf.id = s.repo_folder_id'` | ≥1 | 8 | CONFIRMED |
| C4 | `transcript_turns.session_id` (text, no FK) joins `sessions.client_session_id` | DDL; `logs.rs:255`; `usage.rs:36` | as stated | as stated | CONFIRMED |
| C5 | turns have `git_branch`, `user_text`; sessions has no branch column | DDL + `information_schema` | as stated | as stated (`folders.branch` is checkout grain) | CONFIRMED |
| C6 | `keychain_set/get/delete(service, account, …)` public and generic | `rg 'pub fn keychain_'` | 3 | 3; already used by `dojo_client/session.rs` | CONFIRMED |
| C7 | `KeychainError` has `NotFound`, `Unavailable`, `Store`, `CommandFailed`, `Io` | read `gateway_keys/mod.rs:35-49` | 5 | 5 | CONFIRMED *(was MISLEADING: draft R3 mapped only 2 — fixed)* |
| C8 | `federation::http_client()` = 5 s / 30 s, falls back to an untimed client on build failure, sets no redirect policy | read `federation/mod.rs:23-29`; `rg -c 'redirect::Policy' crates/senseid/src` | as stated; 0 | as stated; 0 | CONFIRMED |
| C9 | Schedule registration = `SCHEDULABLE` (sorted, tested) + `DESCRIPTIONS` (+ `every_schedulable_worker_has_a_description`) + `schedules.jsonl` + `spawn` → `ticker::run_scheduled` + a start-up `spawn` call in `server.rs` | `rg SCHEDULABLE\|DESCRIPTIONS\|run_scheduled`; `forge_token_check.rs:158-160`; `server.rs:555`; `schedule.rs:365` | all present | all present | CONFIRMED *(MISLEADING twice: draft 1 omitted `DESCRIPTIONS`/`tick()`, draft 2 omitted the `server.rs` spawn — both fixed)* |
| C10 | `dojo` scope is `includes:` + `deps: include`; `default` scope `excludes: [dojo, …]` | read `database/design.yaml` | as stated | as stated | CONFIRMED |
| C11 | Agile endpoints paginate by `startAt`/`maxResults` with `isLast`/`total` | recorded live response | present | not run | UNCHECKABLE until prerequisite 2 — build task 1 makes it checkable; build halts there |
| C12 | `duration` gap-aware; `tokens_in` folded total, ~8× cost | `sessions.ddl` comments | as stated | as stated | CONFIRMED |
| C13 | No per-ticket/session-set usage aggregation exists (only day×repo in `session_outcomes.rs`, `usage.rs`); `tokens_fresh` is never read | `rg sum(…tokens\|duration…)`; `rg tokens_fresh` | day×repo only | day×repo only; `tokens_fresh` written only at `sessions.rs:1410` | CONFIRMED |
| C14 | CLI uses `daemon_url()` + `reqwest::blocking` | `rg 'fn daemon_url\|fn client\('` | 2 | 2 | CONFIRMED |
| C15 | No HTTP-mock dev-dependency; `async-trait`, `regex` already deps | `rg` Cargo.toml | 0 / present | 0 / present | CONFIRMED |
| C16 | Handler idiom `.map_err(\|_\| StatusCode::INTERNAL_SERVER_ERROR)?` | `rg -c` | many | 130 in 13 files | CONFIRMED |
| C17 | No trackers/sprints/tracker-tasks/sessions-unlinked routes | `rg -i 'tracker\|sprint\|unlinked' crates/senseid/src/api/` | 0 routes | 0 | CONFIRMED |
| C18 | Daemon CORS is `allow_origin(Any)` with all methods; no auth middleware; loopback bind is the **default**, overridable by `SENSEI_BIND_HOST` | read `api/server.rs:91-120,148-165`; `rg 'from_fn\|route_layer'` | as stated | as stated; 0 middleware | CONFIRMED *(was MISLEADING: "loopback only" — fixed)* |
| C19 | An Origin guard exists — `ALLOWED_ORIGINS` + `origin_may_mutate` in `api/handlers/repositories.rs`, used on `PATCH /api/repositories/{repo_key}`, tested; **no Host guard** exists | `rg -n 'ALLOWED_ORIGINS\|origin_may_mutate\|header::HOST' crates/senseid/src` | guard in repositories.rs; 0 Host | `repositories.rs:39,53,83`; 0 Host | CONFIRMED *(was FALSE: draft claimed no guard and designed a new `local_cli_guard` — R8 now reuses the existing one)* |
| C20 | Project-scoped session reads are mixed: most filter `s.project_id`; some join `sensei.folder_projects` (a view over `folders ⨝ repositories_in_projects`) on `s.folder_id` | `rg 'project_id\s*=\s*\$'`; `rg folder_projects`; `pg_get_viewdef('sensei.folder_projects')` | mixed | `usage.rs:38` etc.; `sessions.rs:547,575,1029`, `metrics.rs:600`, `routes.rs:5522`; view as stated | CONFIRMED *(was FALSE: draft said no read uses the junction — R1 now reuses the view)* |
| C21 | The `tracker_links` drop + project-membership binding is recorded in `docs/backlog.md` | `rg -n 'tracker_links' docs/backlog.md` | ≥1 | 0 at review; 1 after fix (`backlog.md:84`) | CONFIRMED *(was FALSE — fixed)* |
| C22 | Prerequisite P0 returns `f` today | P0 SQL | f | f (0 / 0) | CONFIRMED |
| C23 | `folders.repository_id` is set on **every** folder (D-REPO-ANCHOR, 2026-10-01; supersedes I16), NULL only until the scanner resolves it | `folders.ddl` column comment; `select count(*), count(repository_id) from sensei.folders` | as stated | as stated; 2237 / 2237 | CONFIRMED *(`repositories.ddl:24-25` still states the superseded I16 rule — inherited doc defect, backlog)* |
| C25 | The repo's adapter pattern = trait carrying provider knowledge + common struct + registry fns: `TranscriptAdapter` (`transcript/mod.rs:330`, `adapters()` :418, `adapter_for_source` :439); also `ManifestAdapter`, `ConfigAdapter`, `LanguageAdapter` | `rg -n 'pub trait [A-Za-z]*Adapter' crates/senseid/src`; read `transcript/mod.rs:320-440` | 4 traits; registry fns | 4 traits; `adapters()`, `adapter_for_source()` | CONFIRMED |
| C26 | No GitLab API client exists in senseid (GitLab appears only in remote-URL normalisation and a SwiftPM manifest adapter) | `rg -li gitlab crates/senseid/src` | no client | `repo_key.rs`, `tests.rs`, `adapters/manifest/swiftpm.rs` | CONFIRMED |
| C27 | `normalize_repo_key(remote) -> Option<String>` exists in `db/pg_store/repo_key.rs` and yields the lowercased `host/path` form stored in `repositories.repo_key` | read `repo_key.rs:14-80`; `repositories.ddl` repo_key comment | as stated | as stated | CONFIRMED |
| C28 | Project-window sections are registered in `project-sidebar-view.ts` (intake…about); app secret entry precedent is `ProvidersSection.svelte` (`type="password"`); screen specs live in `docs/spec/screen/project-*.md` | read those files | as stated | as stated | CONFIRMED |
| C29 | `api::util::require_secure_url(raw, resource) -> Result<reqwest::Url, String>` exists (https or exact loopback) and is shared | read `api/util.rs:9-17` | as stated | as stated | CONFIRMED |
| C30 | `gateway_keys` calls are blocking and must be wrapped in `spawn_blocking` | `gateway_keys/mod.rs:14-17` | as stated | as stated | CONFIRMED |
| C31 | CLI blocking client default timeout is 30 s (`client()` → `client_with_timeout(30)`); DB pool max 24, acquire timeout 10 s | `crates/cli/src/main.rs:291-297`; `crates/bootstrap/src/config.rs:93,109` | as stated | as stated | CONFIRMED |
| C32 | `scripts/check-sql-in-layer.py` matches only `sqlx_core::query*` call sites and exempts test modules | read the script | as stated | `CALL` regex line 38; test exemptions lines 21–37 | CONFIRMED |
| C33 | Jira Cloud answers 400 for a whole `jql=key in (…)` when one key exists nowhere | needs the tenant | R11 falls back to one key per request | 2026-10-19, senecaglobal.jira.com board 333: `GET …/tracker-tasks/P057542601-999999` → 404 `task_not_on_board`, key negative-cached; Jira's raw status (400 or an empty result) is not logged, and R11 handles both identically | CONFIRMED (behaviour); raw status unobserved, not load-bearing |
| C34 | `GET /rest/agile/1.0/board/{id}/issue` returns `fields.sprint` (current) and `fields.closedSprints` when asked for them | needs the tenant | R11 builds memberships from them | — | UNCHECKED (verify in F8; without them discovery inserts the task with no membership and the next full sync adds it) |
| C35 | Transcript turns are written by `upsert_transcript_turns` from `ingest_capture`, and `ingest_captures` runs every 300 s | `transcript/mod.rs` (~L500); `database/import/staging/schedules.jsonl` | the R11 ingest trigger and its latency | as stated | CONFIRMED 2026-10-19 |
| C36 | The task queue can tell whether a task kind is already pending (`TaskQueue::has_pending_kind`) | `tasks/transcript_scheduler.rs` `enqueue_backfill` | R11 coalescing | as stated | CONFIRMED 2026-10-19 |
| C24 | `activity.sessions.folder_id` and `repo_folder_id` are both nullable `→ folders on delete set null`; the DDL designates `repo_folder_id` for attribution | `sessions.ddl:4-5` + column comment; `pg_constraint` | as stated | as stated | CONFIRMED *(was MISLEADING: implied only `folder_id` could be nulled — fixed)* |

**Contested ground:** C19 and C20 were FALSE in the second draft and reversed two
design choices: a new guard became reuse of the existing one, and a hand-built join
became the existing `folder_projects` view. C21 was FALSE — the draft claimed a backlog record that did
not exist, for the deviation the analysis called "the client-confidentiality
guard". C7 and C9 were MISLEADING and would have produced an unmapped credential
error and a failing test respectively.

## Related

- Parents: T3 PRD module; integrations requirements §2.5, §5.1/§5.2 (overrides in
  the table above).
- Analysis: `docs/analysis/2026-10-15-jira-ticket-linkage.md` (amended to point here).
- Gaps: T-1 (partially), F-3 (rollup, later); daemon-wide CORS/auth (backlog).
