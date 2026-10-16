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
// crates/senseid/src/tracker/mod.rs
#[async_trait] pub trait TrackerAdapter: Send + Sync {
    fn kind(&self) -> TrackerKind;                       // `tracker_kind` enum value
    /// Parse + validate provider connect params (JSON body) and normalise base_url.
    fn connect_params(&self, body: &serde_json::Value) -> Result<ConnectParams, TrackerError>;
    /// Auth scheme for the shared GET transport (Basic, Bearer, or a named header).
    fn auth(&self, account: &str, token: &ApiToken) -> AuthScheme;
    /// Prove the credential and the board/scope exist; returns the canonical `account`.
    async fn validate(&self, t: &dyn GetTransport, p: &ConnectParams) -> Result<String, TrackerError>;
    /// Everything the coordinator needs, in common shapes. `prev` lets the adapter skip
    /// immutable data (e.g. closed sprints' members).
    async fn fetch_snapshot(&self, t: &dyn GetTransport, p: &ConnectParams, prev: &PrevState)
        -> Result<Snapshot, TrackerError>;
    /// Pure: which known tickets this session's text refers to.
    fn match_links(&self, session: &SessionText, known: &KnownTasks) -> Vec<Link>;
}
pub struct Snapshot { pub sprints: Vec<SprintRow>, pub tasks: Vec<TaskRow>,
                      pub members: Vec<(SprintExtId, Option<Vec<TaskExtId>>)> } // None = unchanged
pub struct SessionText { pub branches: Vec<String>, pub prompts: Vec<String>,
                         pub repo_key: Option<String> }    // GitLab scopes `#12` by repo
fn adapters() -> Vec<Box<dyn TrackerAdapter>>;               // registry, like transcript/mod.rs
fn adapter_for_kind(kind: TrackerKind) -> Option<Box<dyn TrackerAdapter>>;
```

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
| `sensei.trackers` | `id uuid pk`, `project_id uuid not null → projects on delete cascade`, `kind tracker_kind not null` (enum: `jira`; future `gitlab`), `base_url text not null` (adapter-normalised), `account text not null` (adapter-defined: Jira = email; GitLab = username), `external_scope text not null` (Jira project key; GitLab group/project path), `board_ref text not null` (Jira board id; GitLab board / iteration-cadence / milestone scope), `params jsonb not null default '{}'` (adapter-specific extras, never secrets), `secret_service text not null`, `last_attempt_at`, `last_success_at`, `last_error tracker_error` (enum, nullable), `created_at` | `unique(project_id)`; `unique(kind, base_url, board_ref)`. No secret in the row. `after delete` and `after update of secret_service` triggers → tombstone of the old value (R3), defined with its function in `database/ddl/function/sensei/tracker_secret_tombstone.ddl` (the repo's trigger convention, e.g. `historize_memories.ddl`). |
| `sensei.tracker_secret_tombstones` | `secret_service text pk`, `account text not null`, `not_before timestamptz not null`, `created_at` | drained by disconnect and by the tick (R3) |
| `sensei.sprints` | `id uuid pk`, `tracker_id → trackers cascade`, `external_id text not null`, `seq bigint not null` (adapter-supplied creation order — Jira: the numeric sprint id; never compare `external_id` text), `name text not null`, `state text not null` (common states `future`/`active`/`closed`; adapters map theirs onto these), `start_at`, `end_at`, `complete_at` (nullable), `members_fetched_state text` (state at last membership fetch), `removed_at`, `last_seen_at` | `unique(tracker_id, external_id)` |
| `sensei.tracker_tasks` | `id uuid pk`, `tracker_id → trackers cascade`, `external_id text not null` (provider issue id), `key text not null` (provider's canonical display key — Jira `AUTH-420` upper-cased; GitLab `group/project#12`), `scope text` (nullable; GitLab project path the key is relative to), `summary text`, `issue_type text`, `status text`, `current_sprint_id uuid → sprints on delete set null`, `removed_at`, `last_seen_at` | `unique(tracker_id, external_id)`; index `(tracker_id, key)` |
| `sensei.tracker_task_sprints` | `task_id → tracker_tasks cascade`, `sprint_id → sprints cascade` | `pk(task_id, sprint_id)`; index `(sprint_id)` — which sprints list the issue |
| `sensei.session_task_links` | `session_id → activity.sessions cascade`, `task_id → tracker_tasks cascade`, `source link_source not null` (enum: `branch`, `prompt`, `commit` — `commit` never written in v1), `matched_key text not null`, `mentions integer not null check (mentions > 0)`, `linked_at` | `pk(session_id, task_id, source)`; index `(task_id)` |

`tracker_error` enum: `unauthorized`, `forbidden`, `not_found`, `rate_limited`,
`http_error`, `transport`, `decode`, `redirect`, `token_missing`,
`credential_store`, `unsupported_tracker`. `last_error` and sync/connect failures caused by the provider or the
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
- The request type holds the token in a newtype `ApiToken(String)` whose `Debug`
  prints `ApiToken(***)`; request structs are never logged.
- **Every keyring entry that no tracker row references has a tombstone** in
  `sensei.tracker_secret_tombstones(secret_service pk, account, not_before)`. The
  keyring can't be listed, so the tombstone is the only record of an entry to clean
  up. Two triggers on `sensei.trackers` write tombstones for the **old** value, with
  `not_before = now()`: `after delete`, which covers disconnect and the project
  cascade, and `after update of secret_service`, which covers every re-connect,
  including concurrent ones. No code path writes the old-value tombstone by hand.
- **Draining** (each `tracker_sync` tick, and best-effort right after a commit) runs
  per tombstone in its **own transaction**:
  `select … where not_before <= now() for update skip locked limit 1` →
  `keychain_delete` → delete the row → commit. A keyring failure rolls back and
  leaves the row for the next tick.
- **Connect / re-connect** (new tracker, same board, new account or new board):
  1. Commit a **pending** tombstone for the new `(service, account)` with
     `not_before = now() + 10 min`. It's a grace period far longer than steps 2–3
     take (they're bounded by the 30 s client timeout), so a tick can't drain an
     entry that's still being connected.
  2. `keychain_set(new service, account, token)`. On failure → 500
     `{error:"credential_store"}`. The pending tombstone is drained once its grace
     period ends.
  3. One transaction: `select … from sensei.projects where id = $1 for update`
     (serialises connects for the project). Then:
     - **Same board / new tracker:** upsert the row with the new `secret_service`
       and `account`; the update trigger tombstones the previous secret.
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

- Base-URL policy common to every adapter: parse as URL; scheme `https`; no
  userinfo, no query/fragment; host not an IP literal; path discarded. Each
  adapter then applies its **host allow-rule** (Jira: host ends in
  `.atlassian.net`, no port). Anything else → 422 `{error:"invalid_base_url"}`.
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
- **429:** wait `Retry-After` seconds if it parses as an integer, else 5 s; cap 60 s;
  at most 3 retries per request.
- The coordinator and adapters are written against `&dyn GetTransport`; tests use an
  in-memory fake.

**Jira adapter (`tracker/jira.rs`):** auth = HTTP Basic `account(email):token`;
connect params `{base_url, email, api_token, project_key, board_id}`
(`external_scope = project_key`, `board_ref = board_id`); `validate` also checks the
board's project key equals `project_key` (422 `board_project_mismatch`). Endpoints (Jira Cloud; paths from Atlassian's Agile REST board-API operations index,
2026-10-15; response field names confirmed by the build-start fixture, C11):

| Purpose | Call |
|---|---|
| validate credential | `GET /rest/api/3/myself` |
| validate board | `GET /rest/agile/1.0/board/{boardId}` |
| sprints | `GET /rest/agile/1.0/board/{boardId}/sprint?startAt&maxResults=50` |
| board issues | `GET /rest/agile/1.0/board/{boardId}/issue?fields=summary,issuetype,status&startAt&maxResults=100` |
| sprint members | `GET /rest/agile/1.0/board/{boardId}/sprint/{sprintId}/issue?fields=summary&startAt&maxResults=100` |

Pagination: advance `startAt` by the number of items returned; stop when
`isLast == true`, or `startAt >= total`, or a page returns 0 items.

### R5 · Sync coordinator — `tracker::sync(&TrackerRow, &dyn TrackerAdapter, &dyn GetTransport, &PgStore)`

The coordinator is provider-neutral; steps 2–3 are the adapter's `fetch_snapshot`
(for Jira as described; another adapter decides its own fetches but must return the
same `Snapshot` and honour "all-or-nothing").

1. Take `pg_try_advisory_xact_lock(hashtext('tracker_sync:' || tracker_id))` in the
   sync transaction; if not acquired → return `Busy` (manual sync → 409; scheduled
   sync → skip this tracker this tick). Set `last_attempt_at = now()` (own statement).
2. Fetch all sprints and all board issues (all pages).
3. Fetch the member list of every sprint that **needs it**: state ≠ `closed`, or
   closed and (`members_fetched_state` is NULL or ≠ `closed`). A closed sprint whose
   members were fetched after it closed is never re-fetched (closed sprints don't
   change).
4. **Only if every request in 2–3 succeeded**, in the transaction:
   - upsert sprints and tasks by `(tracker_id, external_id)`; a changed key
     overwrites `key`; set `last_seen_at = now()`; clear `removed_at` on seen rows;
   - `removed_at = now()` on rows not seen **and `removed_at IS NULL`**;
   - for each fetched sprint, replace its `tracker_task_sprints` rows; set its
     `members_fetched_state = state`;
   - `current_sprint_id` per task = among sprints listing it: `active` > `future` >
     `closed`; tie → later `start_at` (NULL last), then higher `seq`; none → NULL;
   - **relink** (R6);
   - `last_success_at = now()`, `last_error = NULL`.
5. Any failure → transaction rolled back (no data rows change); `last_error = <code>`
   written in its own statement; error returned. A partial fetch is never committed.

Scheduling: new task **`tracker_sync`** — entry in `tasks::schedule::SCHEDULABLE`,
in `DESCRIPTIONS` (`api/handlers/scheduled_tasks.rs`), a line in
`database/import/staging/schedules.jsonl` (`interval_secs: 1800`), and a `tick()` in
`tasks/tracker_sync.rs`, whose `pub fn spawn(pg)` calls
`tasks::ticker::run_scheduled(pg, "tracker_sync", …)` like `forge_token_check.rs`,
**and** a `crate::tasks::tracker_sync::spawn(...)` call at daemon start in
`api/server.rs` next to `forge_token_check::spawn`. `SCHEDULABLE` stays sorted
(test `schedulable_names_are_unique_and_sorted`). A tick drains secret tombstones
(R3), then syncs every tracker in turn. One tracker's failure is recorded on its
row and does not stop the others.

### R6 · Linking (adapter-owned matching + coordinator-owned relink)

The coordinator gathers `SessionText` (branches, prompts, the session's `repo_key`)
and `KnownTasks` (all of the tracker's tasks incl. removed, with `key`/`scope`), calls
`adapter.match_links`, and writes the result. The rules **below are the Jira
adapter's** `match_links`; the source semantics (`branch`, `prompt`, `mentions`
meaning) and "link only to a known task" are **common to all adapters**.

- `known` = **every** task of this tracker, **including removed ones** (links to a
  removed issue survive and stay attributable; reads flag them).
- Prefixes `P` = distinct prefixes (text before the last `-`) of the keys in `known`
  (a board can span projects; Jira keys may contain `_`).
- For each prefix `p`: find case-insensitive occurrences of `p` + `-` + `[0-9]+`
  (digits greedy). Accept an occurrence only if the character before it is not an
  ASCII letter or digit (or it is at the start). `_`, `/`, `-`, space and
  punctuation are all valid separators. Key = `UPPER(p)-digits`, exactly as
  written (so `AUTH-0420` is a different string from `AUTH-420` and does not
  link).
- Link iff key ∈ `known`. With no `known` keys, nothing links — `UTF-8` and
  `AES-256` cannot match unless `UTF`/`AES` is a synced prefix *and* that issue
  exists.
- `branch`: each **distinct** `git_branch` across the session's transcript turns;
  `mentions` = number of distinct branches containing the key.
- `prompt`: each turn's `user_text`; `mentions` = total accepted occurrences.
- Matching is case-insensitive because branch names are free-form
  (`fix/auth-420-store`).

Inputs per candidate session: `activity.transcript_turns WHERE session_id =
sessions.client_session_id`. No `client_session_id` or no turns → no links.

**Relink** (inside the sync transaction): delete every `session_task_links` row whose
task belongs to this tracker, then insert `match_links` output for all candidate
sessions. Full recompute — deterministic and idempotent.

### R7 · Metrics (computed at read; nothing materialised)

**Usage** of a distinct session set `S`:

| Field | Definition |
|---|---|
| `sessions` | `|S|` |
| `by_source.branch`, `by_source.prompt` | sessions in `S` with ≥1 link of that source (to the ticket, for ticket usage; to any sprint member, for `linked_sprint`) — may sum past `sessions` |
| `shared` | sessions in `S` linked to ≥2 distinct tasks of this tracker |
| `active_secs`, `active_n` | `Σ extract(epoch from duration)` over sessions with `duration IS NOT NULL`; `active_n` = that count |
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

**Sprint usage:**
- `state = 'future'` → `usage: null, reason: "not_started"`.
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
| `GET /api/projects/{id}/trackers` | `[{id, kind, base_url, account, external_scope, board_ref, status, last_success_at, last_error, counts:{tasks, sprints}}]`; `[]` when none. Unknown project → 404. |
| `POST /api/projects/{id}/trackers/{kind}/connect` | `kind` resolved by `adapter_for_kind`; unknown or unregistered → 422 `{error:"unsupported_tracker"}`. Body = the adapter's connect params (Jira: `{base_url, email, api_token, project_key, board_id}`). Validate and normalise the URL (R4) → 422. The normalised `(kind, base_url, board_ref)` already connected to **another** project → 409 `{error:"board_already_connected", project:{id, name}}`. The project already connected (same or different board) → **re-connect**: fields and token replaced after validation; a different board drops the old board's sprints/tickets/links (old tracker row replaced). Then `adapter.validate`: credential rejected → 401 `{error:"unauthorized"}`; board/scope missing → 404 `{error:"board_not_found"}`; adapter-specific mismatches → 422 with the adapter's code (Jira: `board_project_mismatch`). Then persist (R3), run the first sync, return **201** `{tracker, first_sync:{ok:true, counts} \| {ok:false, error:<code>}}`. A failed first sync keeps the connection (credential was valid); `status` shows `error`. |
| `DELETE /api/projects/{id}/trackers` | R3 disconnect; 204; 404 if the project has no tracker. |
| `POST /api/projects/{id}/trackers/sync` | R5 now. 200 `{counts:{sprints, tasks, removed, links}}`; 409 `{error:"busy"}`; 502 `{error:<code>}`; 404 if no tracker. |
| `GET /api/projects/{id}/sprints` | the project's sprints, newest first: `[{id, name, state, start_at, end_at, complete_at, candidates_in_window, linked_sprint, linked_other, unlinked}]` (each part a usage object; `null` + `reason` for future). Removed sprints excluded. |
| `GET /api/projects/{id}/tracker-tasks?sprint={uuid}&include_removed=bool` | the sprint's tickets (or all the project's tickets without `sprint`): `[{key, summary, issue_type, status, current_sprint, sprints:[{id, name}], removed, usage}]`. `sprint` filters to its members; unknown sprint → 404. |
| `GET /api/projects/{id}/tracker-tasks/{key}` | `{task, usage, sessions:[{id, started_at, sources, matched_key}]}`; case-insensitive key; 404 if absent. |
| `GET /api/projects/{id}/sessions/unlinked?sprint={uuid}` | the sprint's `unlinked` sessions `[{id, started_at, task}]`; sprint required; unknown → 404. |

DB errors → 500 via `.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?`. No list
pagination in v1 (bounded by one board). Errors are logged with the code only.

### R9 · Status (pure fn of row + `interval_secs`, unit-tested)

In order: `error` if `last_error IS NOT NULL`; `pending` if `last_success_at IS NULL`;
`stale` if `now − last_success_at > 2 × interval_secs`; else `connected`.
`interval_secs` is read from the `tracker_sync` schedules row whether or not it is
enabled; a missing row is a 500 (the row is seeded and test-enforced).

### R10 · CLI (`crates/cli`, via `daemon_url()` + the blocking client)

- `sensei tracker connect <kind> …` — one subcommand per registered adapter; v1: `jira`.
  `sensei tracker connect jira --project <p> --url <site> --email <e> --project-key <K> --board <id>`
  — token from `SENSEI_JIRA_TOKEN` if set, else one line read from stdin (the help
  text notes env vars are visible to same-user processes). Never an argv flag. The
  CLI sends the URL as typed; the server normalises it (R4). On 409
  `board_already_connected` it prints which project owns the board.
- `sensei tracker sync --project <p>` · `sensei tracker disconnect --project <p>`
- `sensei tracker sprints --project <p>` — the project's sprints with ticket counts.
- `sensei tracker usage --project <p> [--sprint <s> | <KEY>]` — giving both is an
  error. Table: key · summary · sessions (branch/prompt) · **"active AI time"** `h:mm` ·
  fresh / cache-read / out tokens · cost (blank when unknown, not `0`), then for a
  sprint the `unlinked` count. `--by-repo` adds one row per repository.
- `<p>`: a uuid, or an exact project name from `GET /api/projects`; 0 or >1 matches
  → error listing the candidates. `<s>`: uuid or exact sprint name; same rule.

### Out of scope

Any adapter other than Jira (GitLab is next — below); write-back to any tracker; Dōjō UI / rollup / upload (F-3); ADO / GitHub Issues; task-type
mapping; per-repo key patterns and source toggles (T-1c); commit linkage; OAuth;
Jira Server/DC; assignee or any per-person figure; desktop-app screen; daemon-wide
route auth/CORS (backlog).

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
   Must return `t`. Today `f` (0 / 0).
2. **Jira tenant (human):** the user supplies a Jira Cloud site, board id and API
   token for the live gates and the C11 fixture. Build task 1 records one response
   per R4 endpoint (secrets and issue text scrubbed) under
   `crates/senseid/tests/fixtures/jira/`. Until then the build halts at that task.

## Done gate

Unattended (CI):
1. `cargo test -p senseid tracker` passes, including:
   a. `match_links` with `known = {AUTH-420, AUTH-421, MY_PROJ-12}`: `UTF-8`, `AES-256`, `CVE-2026` → none; `fix/auth-420-store` → AUTH-420/branch; `AUTH-420_fix`, `feature_AUTH-420` → AUTH-420; `xAUTH-420`, `AUTH-4201`, `AUTH-0420` → none; `MY_PROJ-12` → MY_PROJ-12; a prompt with AUTH-420 twice → `mentions = 2`; AUTH-420 and AUTH-421 → two links; empty `known` → none.
   b. Sync on the fake transport (fixture-shaped): 2 issue pages + 2 sprints → all rows; a second identical sync changes nothing but `last_seen_at`; an issue missing on the third sync → `removed_at` set once (unchanged on a fourth sync) **and its links still present**; a renamed key → same row, new key.
   c. Page 2 errors → zero data rows changed, `last_error` set, `last_success_at` unchanged.
   d. Issue listed by a closed and an active sprint → `current_sprint_id` = active; both rows in `tracker_task_sprints`; a closed sprint fetched after closing is not re-fetched.
   e. R9 status: never synced → `pending`; failed first sync → `error`; success then 2×interval+1s → `stale`; just inside → `connected`.
   f. 429 + `Retry-After: 1` then 200 → ok; 429 with no header → 5 s wait (clock injected); 4× 429 → `rate_limited`; 302 → `redirect`.
   g. URL validation: `http://x.atlassian.net`, `https://evil.com`, `https://1.2.3.4`, `https://u:p@x.atlassian.net`, `https://x.atlassian.net:8443` → rejected; `https://acme.atlassian.net/jira/` → `https://acme.atlassian.net`.
   h. `local_caller_guard`: `Origin: https://evil.com` → 403; `Origin: tauri://localhost` → passes; `Host: evil.com` → 403; no Origin + `Host: 127.0.0.1:7744` → passes. The existing `repositories.rs` origin tests still pass after the extraction.
   i. Sentinel-token test: run connect (fake), failed sync and status with token `SENTINEL-TOKEN-XYZ`; captured logs, `last_error`, and every response body do not contain it.
   j. Sprint usage against seeded DB rows: `candidates_in_window` equals an independent `count(*)` written in the test; a session linked only to another tracker's task lands in `unlinked`; a future sprint → `not_started`; an active sprint past `end_at` includes a session started after `end_at`.
   k. Project hierarchy (seeded DB, fake transport, fake `SecretStore`): a project with 3 repos; sessions in each repo on `fix/AUTH-420-x` → AUTH-420 usage `sessions = 3`, `by_repo` has 3 entries summing to the total. Removing one repo from the project → AUTH-420 drops to 2 immediately (no resync). `GET …/sprints` returns the board's sprints; `GET …/tracker-tasks?sprint=S` returns exactly S's tickets; a ticket carried from S1 to S2 appears under both, with `current_sprint` = S2. Connecting the same board from a second project → 409 `board_already_connected`, including when typed as `https://ACME.atlassian.net/jira/software`. Re-connecting the first project to a different board removes the old board's sprints and tickets, creates a new tracker id, and tombstones only the old secret (after one tick the new one is still readable). A tick run while a connect is paused between `keychain_set` and step 3 does not delete the new secret (grace period). Two interleaved re-connects of one project leave exactly one live secret, and every other keyring entry they wrote has a tombstone. Re-connecting to the same board with the same email: the live token is never deleted, even when the commit is made to fail (fake store returns the old token). Re-connect from `e1` to `e2` with a fake store whose delete fails → a tombstone for `(old service, e1)` remains for the tick. Deleting the project removes its tracker and writes a tombstone; the next tick deletes the secret and the tombstone. A keyring delete that fails leaves the tombstone in place.
   l. **Seam test:** a test-only `FakeAdapter` (non-Jira key format `#12` scoped by `repo_key`, its own sprint shape) runs through the coordinator (sync, relink, usage) with no change to coordinator code. `repositories.repo_key` is seeded through the production `normalize_repo_key` from remotes like `git@gitlab.com:Group/Sub/Repo.git`, and the fake's `scope` comes from `https://gitlab.com/Group/Sub/Repo`. Two sessions in different repos both saying `#12` link to different tasks. A third repo with no matching scope gives 0 links, and a session with NULL `repo_key` gives 0 links.
2. `rg -n '\.(post|put|patch|delete)\(' crates/senseid/src/tracker/` → no output.
2b. `rg -n -i 'jira|atlassian' crates/senseid/src/tracker/{http,sync,link,usage,status}.rs` → no output; in `tracker/mod.rs` the only hits are `mod jira;` and the `JiraAdapter` entry in `adapters()`. Naming the adapter in the registry, the CLI's `jira` subcommand and `SENSEI_JIRA_TOKEN` is expected; Jira **logic** (key shape, endpoints, auth, host rule) lives only in `tracker/jira.rs`.
3. `rg -n 'api_token|SENSEI_JIRA_TOKEN' crates/senseid/src crates/cli/src` hits only the connect request type, its keyring call, and the CLI token reader; no DDL column holds a token.
4. `tracker_sync` is in `SCHEDULABLE`, `DESCRIPTIONS` and `schedules.jsonl`; `every_schedulable_worker_has_a_description`, `every_schedulable_worker_has_a_schedule_row`, `every_schedule_row_names_a_real_worker` and `schedulable_names_are_unique_and_sorted` pass; `rg -n 'tracker_sync::spawn' crates/senseid/src/api/server.rs` → 1 hit (the tests above can't see a missing spawn).
5. `rg -n 'const ALLOWED_ORIGINS|fn origin_may_mutate' crates/senseid/src` → exactly one definition each, in `api/origin_guard.rs`.

Human (after both prerequisites; needs the user's tenant):
6. Group the project's repos (README `project:`), connect via CLI → `GET …/trackers` shows `connected`; sprint and issue counts equal the Jira board UI; a ticket worked on in two of the project's repos shows both in `by_repo`.
7. For one ticket, `active_secs` and `tokens_fresh` from the API equal a hand-written `select sum(...)` over its linked session ids (recorded in Evidence).
8. A real session on a branch carrying a real key appears under that ticket.
9. Precision sample: ≥10 real linked sessions reviewed; 0 links to non-existent keys; precision per source recorded in Evidence.
10. Bad token on connect → CLI prints the `unauthorized` error; `GET …/trackers` returns no row.

## Wrong gate

- A link to a key not in `tracker_tasks`, or a regex hit treated as a key.
- Links to a removed issue disappear on the next sync.
- `tokens_in` shown as "tokens"; a NULL field summed as `0`; a field without its `_n`.
- Ticket sums added to make a sprint total (double-counts shared sessions).
- The same tracker board connected to two projects, or a project with two boards (two sprint calendars).
- A ticket's usage missing sessions from one of the project's repos.
- A failed/partial sync changes data rows, or a never-successful tracker shows `connected`.
- Any non-GET request to a tracker, a request to a host outside the adapter's allow-rule, or a followed redirect.
- Jira logic (key shape, endpoints, auth scheme, host rule) outside `tracker/jira.rs`; a coordinator change needed to add an adapter.
- A drain deleting a secret whose tombstone is still within its grace period; a tracker row pointing at a secret with no keyring entry.
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
