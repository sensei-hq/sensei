---
name: GitHub tracker adapter (Projects iterations)
status: r2.3 (design gate passed 2026-11-09)
date: 2026-11-09
parent: docs/spec/daemon/tracker-jira-linkage.md ("Integration seam", R1–R11, "Future adapters")
sibling: docs/spec/daemon/tracker-gitlab.md (§G5 host policy, §G6 `forge.rs`, §G8 capabilities reused; §G9 `get_ticket` extended by H5)
analysis: docs/analysis/2026-11-06-forge-trackers.md (slice 2), its GitHub design superseded by D92
also: docs/spec/tracker-levels.md (D40), docs/spec/daemon/tracker-sdlc-metrics.md (S4, S5), docs/spec/daemon/tracker-metrics-writeback.md (W10, gate 13)
---

# GitHub tracker adapter

> **r2 (2026-11-09):** rewritten after the claims verification and five reviewers.
> See "Review record" at the end.

## Purpose

Connect a sensei workspace to a **GitHub Project (v2)** on github.com, through the
daemon path (D91). Sprints are the iterations of the project's **Iteration field**.
Tickets are the project's **issues**, from any repository the project holds.

The user changed the analysis's GitHub design on 2026-11-09, recorded as **D92**,
which amends D71, D91, FT3, FT5, O5 and O6:

| Was | Now |
|---|---|
| FT3 / D91: GitHub milestones are sprints; O5 merge by title; O6 `start_at` NULL | A **GitHub Project's Iteration field** is the sprint source. Milestones are not read. An iteration has a real start date and duration, so O6 no longer arises. |
| FT5 / D91: GitHub uses the person's forge sign-in | The tracker authenticates with **a GitHub token given at connect**, kept in the local keychain like Jira's and GitLab's. |
| D71: live GitHub ticket reads (`get_ticket`) use the forge sign-in | They use **the tracker's token** (H5). The forge sign-in lacks `read:project` (its scopes are `read:org`, `user:email`, `repo`, `dojo/kavach.config.js:52`), and adding the scope would change the dōjō's login for every user. The dōjō sign-in itself is unchanged. |

Reused unchanged from the GitLab slice: session linking through `tracker/forge.rs`
(GitLab §G6), usage (`usage.rs`), discovery (R11), the Tickets screen, the address
policy inside `ReqwestRaw` (§G5), and the `unsupported_by_tracker` capability
guards (§G8). Changed: `get_ticket`'s tracker selection (H5) and the read seam (H2).

## Resolved design

### H1 · What a GitHub board is

| Concept | Stored as |
|---|---|
| Board | One **Project (v2)**, owned by an organization or a user |
| `base_url` | **`https://api.github.com`**, always. github.com only; GitHub Enterprise Server is out of scope. |
| `board_ref` | **`project:<node id>`**: the project's GraphQL node id (`PVT_…`) from validate. A renamed project or owner keeps it, so one project can never be connected twice (the existing `board_already_connected` check on `board_ref`). |
| `external_scope` | `<owner login>/<project number>`, lowercased, **at connect** (written only by connect, `db/pg_store/trackers.rs:1119`) |
| `account` | The token's `login` from `GET /user` at validate |
| `params` | `{"board_name", "owner", "owner_kind": "organization"\|"user", "number", "url", "iteration_field": <name>\|null}`. **`iteration_field` is set at validate** and refreshed by every snapshot. |

**Connect body** (`POST /api/workspaces/{id}/trackers`):

```json
{ "kind": "github", "level": "sprint",
  "board_url": "https://github.com/orgs/sensei-hq/projects/3",
  "api_token": "github_pat_…",
  "iteration_field": "Sprint" }
```

`iteration_field` is optional (H3.2). The keychain stores the **bare token**;
`Bearer ` is added only in `auth()`.

**`connect_params`** (pure, no network). Every failure is a **422** with its code:
1. `board_url` is parsed with `url::Url`. The scheme must be `https`, the host must
   equal `github.com` exactly after the parser's lowercasing (so `github.com.evil.com`,
   `api.github.com`, an IDN host, and `GITHUB.COM` → `github.com` are told apart),
   with no port and no username or password. The path must be
   `/orgs/<owner>/projects/<n>` or `/users/<owner>/projects/<n>`, optionally followed
   by `/views/<v>` or `/`. A query or fragment is ignored. `<owner>` must match
   `^[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})$` (no `%`-encoded bytes), `<n>` `^[0-9]{1,9}$`.
   Anything else → **`invalid_base_url`**.
2. `api_token` missing or blank → `token_required`. `email_required` and
   `host_not_confirmed` are never returned: the host is fixed.
3. `ConnectParams.base_url` = `https://api.github.com`; `board_ref` = the typed
   `orgs/<owner>/<n>` or `users/<owner>/<n>`, lowercased; `account` = `""`. Both are
   **provisional**: validate replaces them. As with GitLab (§G1), the provisional
   `board_ref` reaches only the `board_not_found` body (`api/handlers/trackers.rs:268`, from `connect_failure`),
   which fires before validate succeeds. It holds only what the user typed.

**Validate** (network; at most **3 requests** when the project has ≤ 100 fields):
1. `GET /user` → `login`, `id`, `type`. 401 → `unauthorized`. 403 → `forbidden`.
2. **`Q_VALIDATE_ORG`** or **`Q_VALIDATE_USER`** (H2.4), as the URL said. A
   `NOT_FOUND` error or a null project → `board_not_found`. `INSUFFICIENT_SCOPES` or
   `FORBIDDEN` → `forbidden` (the token lacks `read:project`). More than 100 fields
   (`fields.pageInfo.hasNextPage`) → `http_error`: a project with more than 100
   fields is out of scope, and a silently truncated field list could hide the
   iteration field.
3. The iteration field, from the project's `ProjectV2IterationField`s:
   - a name was given → that field; no field of that name → ambiguous (below);
   - no name, **exactly one** iteration field → that field;
   - no name, **none** → `iteration_field = null` (no sprints, H3.5);
   - no name, **more than one** → ambiguous.

   The choice is stored as `params.iteration_field` and every later snapshot uses
   only that (H3.2). Ambiguous → **422 `iteration_field_ambiguous`**, body
   `{"error": "iteration_field_ambiguous", "fields": [<names, in the project's order>]}`.

`Validated` = `board_ref: Some("project:<id>")`, `account: Some(login)`,
`external_scope: "<owner>/<n>"` (lowercased), `board_name: title`, `params` as H1
(including the chosen `iteration_field`). The owner check, the pending tombstone,
the keyring entry and the row use the validated values (`connect.rs:45-91`, as for
GitLab). A **closed** project connects; its iterations still read.

### H2 · Transport: GraphQL as a guarded read

Reads go through `GetTransport` (`tracker/mod.rs:207`), which today has one method,
`get_json`, documented "GET-only … One method, so no other verb can be sent"
(`mod.rs:205`). GitHub's Projects data is read with **GraphQL**, a `POST` to
`https://api.github.com/graphql`. GitHub's REST Projects API was considered and
rejected: its pages are linked only through the `Link` header, which `RawResponse`
does not carry (`transport.rs:11`), and its docs give no shape for an iteration value.

1. **A validated query type.** `tracker/graphql.rs` (new) defines
   `pub struct GraphqlQuery { document: &'static str, operation: &'static str, variables: serde_json::Value }`.
   Its only constructor, `GraphqlQuery::new(document: &'static str, variables: Value) -> Result<Self, TrackerError>`,
   **lexes** the document (skipping `#` comments, `"…"` strings and `"""…"""` block
   strings) and requires **exactly one operation definition**, written
   `query <Name>` (no anonymous `{ }` shorthand), and no `mutation`, `subscription`
   or second operation anywhere at the top level. Fragments are allowed. Anything
   else → `BadRequest`. `operation` = `<Name>`, sent as `operationName`. The
   document is a `&'static str` constant, so owner, number, cursor and field name
   travel **only as variables**, never interpolated.
2. **The seam.** `GetTransport` gains
   `async fn graphql(&self, q: &GraphqlQuery) -> Result<serde_json::Value, TrackerError>`
   with a default body `Err(TrackerError::UnsupportedTracker)`. `RawGet` gains a
   provided method `async fn post_graphql(&self, body: &[u8]) -> Result<RawResponse, ()>`
   whose default is `Err(())`. The existing `impl<R: RawGet> GetTransport for PolicyTransport<R>`
   (`transport.rs:106`) **keeps its bound** and overrides `graphql`: serialize
   `{"query", "operationName", "variables"}`, call `raw.post_graphql` through the same
   `with_retries` and `map_response` as `get_json`, then H2.3. **Only `ReqwestRaw`
   overrides `post_graphql`**, and it POSTs to `<base_url>/graphql` (a fixed path;
   no path argument exists) **only when it was built with `graphql: true`**;
   otherwise it returns `Err(())` with nothing sent. `TrackerAdapter` gains
   `fn reads_graphql(&self) -> bool` (default `false`; `GithubAdapter` → `true`).
   `TransportFactory::transport` gains a `graphql: bool` argument, passed as
   `adapter.reads_graphql()` at its four call sites (`api/handlers/trackers.rs:168`,
   `tracker/history.rs:64`, `tracker/writeback.rs:593`, `tracker/runtime.rs:87`).
   `ReqwestRaw` gains a `graphql: bool` field, `false` from `new` and `with_resolver`
   (so every existing caller and test is unchanged), set only by a new
   `allow_graphql(self)`, which the production factory calls when the flag is true.
   `write_transport` never sets it. A Jira or GitLab credential therefore never leaves in a POST
   to `/graphql`, whatever code holds its `&dyn GetTransport`. Every RawGet-only test backend (`transport.rs:246` `Ref`)
   and every `GetTransport` test impl (`FakeBoard`, `RecordedGitlab`, `Pages`) is
   unchanged.
3. **GraphQL errors are failures.** A 200 whose body has a non-empty `errors` array
   never yields data, even with `data` partly filled. The first error's `type`:
   `NOT_FOUND` → `NotFound`; `FORBIDDEN`, `INSUFFICIENT_SCOPES` → `Forbidden`;
   `RATE_LIMITED` → `RateLimited`; anything else → `HttpError`. A body with neither
   `data` nor `errors` → `Decode`. Error messages are never copied into a
   `TrackerError` (a unit enum).
   **One exception, for `Q_ISSUE` only:** GitHub answers a repository or issue that
   does not exist, or that the token cannot see, with `null` and a `NOT_FOUND` error
   whose `path` is `["repository"]` or `["repository", "issue"]`. When **every** error
   is such a `NOT_FOUND`, the read is **absence** (`Ok(None)` / not in the result),
   not a failure. Any other error, or a `NOT_FOUND` elsewhere, stays an `Err`.
4. **The queries**, constants in `tracker/github.rs`, each a single named `query`:

   ```graphql
   # Q_VALIDATE_ORG (Q_VALIDATE_USER: `user(login: $owner)` in place of `organization`)
   query ValidateOrg($owner: String!, $number: Int!) {
     organization(login: $owner) { projectV2(number: $number) { ...Board } } }

   # Q_BOARD: the board and its fields, by node id (snapshot step 1–2)
   query Board($id: ID!) { node(id: $id) { ... on ProjectV2 { ...Board } } }

   fragment Board on ProjectV2 {
     id title url number closed
     owner { __typename ... on Organization { login } ... on User { login } }
     fields(first: 100) {
       pageInfo { hasNextPage }
       nodes { __typename
         ... on ProjectV2IterationField { id name
           configuration {
             iterations { id title startDate duration }
             completedIterations { id title startDate duration } } } } } }

   # Q_ITEMS: one page of items (snapshot step 4)
   query Items($id: ID!, $field: String!, $after: String) {
     node(id: $id) { ... on ProjectV2 {
       items(first: 100, after: $after) {
         pageInfo { hasNextPage endCursor }
         nodes { ...Item } } } } }

   fragment Item on ProjectV2Item {
     id type isArchived
     fieldValueByName(name: $field) { __typename
       ... on ProjectV2ItemFieldIterationValue { iterationId } }
     content { __typename ... on Issue {
       databaseId number title body state url createdAt updatedAt
       issueType { name }
       repository { nameWithOwner url }
       assignees(first: 1) { nodes { databaseId login } } } } }

   # Q_ISSUE: one issue and its project items (discovery H4, get_ticket H5)
   query Issue($owner: String!, $name: String!, $number: Int!, $field: String!, $after: String) {
     repository(owner: $owner, name: $name) { issue(number: $number) {
       databaseId number title body state url createdAt updatedAt
       issueType { name } repository { nameWithOwner url }
       assignees(first: 1) { nodes { databaseId login } }
       projectItems(first: 50, after: $after, includeArchived: true) {
         pageInfo { hasNextPage endCursor }
         nodes { isArchived project { id }
           fieldValueByName(name: $field) { __typename
             ... on ProjectV2ItemFieldIterationValue { iterationId title } } } } } } }
   ```

   With no iteration field (`iteration_field` null), `Q_ITEMS` and `Q_ISSUE` are sent
   with `$field` = `""`, for which GitHub returns `fieldValueByName: null`.
   The recorded fixture (Build order 1) confirms every field name above. If a name
   differs, the fixture wins and this section is corrected before code is written.
5. **Auth**: `AuthScheme::Header { name: "Authorization", token }` with the value
   `Bearer <token>`, built in `auth()`. `authenticate` marks it sensitive
   (`transport.rs:189`).
6. **User-Agent**: GitHub refuses API requests without one. `ReqwestRaw::with_resolver`,
   the shared constructor, sets `.user_agent(concat!("sensei/", env!("CARGO_PKG_VERSION")))`
   for every tracker client.
7. The address policy, `.no_proxy()` and redirects-off apply unchanged.
8. **403**: the policy maps 403 → `Forbidden` (`transport.rs:98`). GitHub's
   secondary rate limit also answers 403. v1 accepts that it reads as `forbidden`;
   the next sync retries. Recorded as out of scope.

**Token**: the documented, recommended token is **fine-grained**, read-only:
Projects: read on the owner (organization permission), and Issues: read and
Metadata: read on the project's repositories. A classic token with `repo` and
`read:project` also works; the CLI help and the Integrations form say that a
classic `repo` token can also **write** to every repository it reaches.
`gh auth token` is a classic token.

### H3 · Snapshot

`fetch_snapshot` calls a pure `snapshot_from(board, item_pages, now)`; `now` is
`chrono::Utc::now()` in production and a fixed instant in tests (as GitLab's
`milestone_row(…, today)`, `gitlab.rs:186`). **Any failed request fails the whole
snapshot** (R4).

1. **Board** (`Q_BOARD`): `title` → `board_name`; `owner`, `owner_kind`, `number`,
   `url` → `params_extra`. A null node → `NotFound`. `fields.pageInfo.hasNextPage`
   → `HttpError`.
2. **Iteration field**, from `fields.nodes` of type `ProjectV2IterationField`:
   - `params.iteration_field` set → that field. Missing from the project →
     `NotFound` (the snapshot fails; it never falls back to another field).
   - `params.iteration_field` null (validate found none) → the project must still
     have none, else `Decode`.
     A field added after connect is never adopted by a sync: the sync fails and
     Integrations shows the error. Only a person's reconnect or Replace token adopts
     it, by the H1 validate rule (a sole field is taken; several need a name).
   - `params_extra.iteration_field` = the field used, or null.
3. **Sprints**: every iteration of `configuration.iterations` and
   `configuration.completedIterations`.
   - `external_id` = **`iteration:<id>`**. `name` = `title`.
   - `start_at` = `startDate` at 00:00 UTC. `end_at` = `startDate + duration` days at
     00:00 UTC: exclusive, as `usage.rs::sprint_window` reads it (`[from, to)`).
   - `seq` = `startDate` as days since 1970-01-01. `seq` is read only to order
     sprints: `tracker_usage.rs:196, :310`, `tracker_history.rs:213`,
     `sdlc/sprint_replay.rs:75` (the sprints later than this one), and as the
     tie-break for a ticket's current sprint (`trackers.rs:346` → `sprint.rs:58`).
     GitHub iterations of one field do not overlap, so two never share a start date.
     The `sprints.seq` column comment changes from "creation order" to "ordering
     key" (H7).
   - **State**: in `completedIterations` → `closed`. In `iterations`: `start_at > now`
     → `future`; otherwise `active`. One still in `iterations` past its end stays
     `active`, as GitHub shows it.
   - `complete_at` = NULL.
4. **Items** (`Q_ITEMS`), `items(first: 100, after)` until `hasNextPage` is false.
   - Kept: `type == ISSUE`, `isArchived == false`, `content.__typename == "Issue"`.
     **Dropped**: `PULL_REQUEST`, `DRAFT_ISSUE` (no repository, no number), `REDACTED`
     (the token cannot see it), archived items. An `ISSUE` item with null `content`
     or null `repository` → `Decode`.

   | TaskRow field | from `content` |
   |---|---|
   | `external_id` | `databaseId`, as text. GitHub allows an issue once per project, so it is unique within a board. |
   | `key` | `<repository.nameWithOwner>#<number>`, **lowercased** (`Sensei-HQ/sensei#12` → `sensei-hq/sensei#12`) |
   | `scope` | `normalize_repo_key(repository.url)` → `github.com/sensei-hq/sensei` (`repo_key.rs:54` asserts this shape) |
   | `summary` | `title` |
   | `issue_type` | `issueType.name`, else **None** |
   | `status` | `state` lowercased (`open` / `closed`) |
   | `status_category` | `open → "new"`, `closed → "done"`, anything else → `Decode` |
   | `assignee` | `AccountRef(assignees.nodes[0].databaseId)`, or None |
   | `created_at` / `updated_at` | `createdAt` / `updatedAt` |
   | `points` | **None** (not asked). Snapshot `points_field` = None. |
5. **Membership**: `fieldValueByName.iterationId` → `iteration:<id>`. `members` lists
   every sprint with `Some(ids)` (an empty vec when it has none). A null
   `fieldValueByName` → backlog. An `iterationId` naming no listed iteration →
   `Decode`. **No iteration field** (or one with no iterations): `sprints: []`, every
   task `current_sprint_id = NULL`, the level stays as chosen (FT4).
6. **Pages**: at most **200** item pages. The 200th page with `hasNextPage` true →
   `HttpError` (never a partial board as whole).

### H4 · Keys, links, discovery

`tracker/forge.rs` unchanged (GitLab §G6): full references `owner/repo#12`, bare
`#12` resolved only through the session's `repo_key`, branch `12-…`, and
`unknown_keys` returning full references whose path a task on the board already uses.

**GitHub key shape.** Before any request, a key's path must be exactly
`<owner>/<name>`: owner as H1, name `^[A-Za-z0-9._-]{1,100}$`, and the number
1 ≤ n ≤ 2147483647 (GraphQL `Int`). `forge::is_key_path` accepts deeper paths for
GitLab subgroups; for GitHub any other shape is **absent, with no request sent**.

**`fetch_tasks(keys)`**: per key, `Q_ISSUE`. A null repository or issue (including
a pull-request number), signalled by H2.3's `NOT_FOUND` exception → absent. **Containment**: the issue is on the board when a
`projectItems` node has `project.id` = the board's node id and `isArchived == false`.
`projectItems` is paged (50 per page, at most 10 pages; the 10th page with
`hasNextPage` true and no match → `HttpError`). Not contained → absent. Contained →
mapped as H3.4, membership from that node's `iterationId`.

### H5 · `get_ticket` for GitHub keys (D63, D71 as amended by D92)

**Keys keep their host.** `forge_key` (`targets/http.rs:201`) builds a bare `#12`
from the folder's `repo_key` and drops the host (`:215`). It changes to return
`(canonical_key, Option<host>)`: the host from the `repo_key` for a bare key, None
for a typed full reference.

**Selection** (`tracker::forge::tracker_for_key(trackers, key, host)`, replacing
`gitlab::tracker_for_key` at `targets/http.rs:101`):
1. **Host known** (a bare key): only trackers on that host. `github.com` → GitHub
   trackers. Any other host → GitLab trackers whose `base_url` host equals it, by
   the longest `params.full_path` prefix (the existing GitLab rule).
2. **Host unknown** (a typed `owner/repo#12`): GitLab's path-prefix rule first; if no
   GitLab tracker matches, the GitHub trackers.
3. **GitHub candidates, in order**: those whose mirrored tasks include one with
   `scope = github.com/<owner>/<repo>`, then the workspace's other GitHub trackers,
   each group in tracker list order (`ORDER BY level, created_at, id`,
   `trackers.rs:473`). A newly added repository's issue is still found by the
   second group.
4. **The loop**: each candidate's `GithubTicketReader` runs. `Ok(Some)` → answer.
   `Ok(None)` → next candidate. `Ok(None)` covers **every** way a ticket is
   unavailable from a board alike: no such repository or issue, not visible to the
   token, a pull-request number, and off the board. So the answer never tells a
   caller which of those it was. **Any `Err` stops the loop** → 502 with that
   error's code and no issue content. All `Ok(None)` → 404 `ticket_not_found`. No candidate at all (no GitHub tracker in the workspace) →
   404 `no_tracker_for_key`.

**`GithubTicketReader`** (`tracker/github.rs`, implementing `TicketReader`): `Q_ISSUE`.
Containment as H4 is checked **before** anything is mapped or returned. Maps `title`,
`body` → description, `state`, `issueType.name`, the iteration title → `sprint`,
`assignees[0].login`, `url`; `acceptance_criteria = []`,
`AcceptanceCriteriaSource::NotConfigured`. A null issue or not contained → `Ok(None)`.

### H6 · Capabilities

`GithubAdapter::capabilities()` = `{history: false, writeback: false}`. The §G8
guards read `capabilities()` and name no kind (`history.rs:46`, `writeback.rs:516`,
`api/handlers/trackers.rs:376, :461-466`, `sdlc/ticket.rs:562`,
`sdlc/sprint_replay.rs:415`), so they apply with no change. The write-back trait
methods are required, so `github.rs` returns `Err(UnsupportedTracker)` from each and
`false` from `field_ref_shape`, as `gitlab.rs:553-599` does. `owner()`: `GET /user`
→ `Person(AccountRef(id))`, or `NotAPerson` when `type == "Bot"`.

### H7 · Changes, by surface

| Surface | Change |
|---|---|
| `tracker/mod.rs` | `TrackerKind::Github` (`"github"`); `GetTransport::graphql` (default `UnsupportedTracker`); the `GetTransport` doc comment (`:205`) becomes "GET, plus validated GraphQL queries (H2)"; `ValidateError` **drops `Copy`** and gains `IterationFieldAmbiguous(Vec<String>)`; `TrackerAdapter::reads_graphql` (default `false`); `adapters()` registers `GithubAdapter` |
| `tracker/mod.rs` (build amendment, G3) | `ConnectParams` gains `sprint_field: Option<String>` (the connect body's `iteration_field`, read by validate step 3; `None` for Jira and GitLab). `BoardScope` gains `params: serde_json::Value` (the tracker row's stored `params`, so `fetch_snapshot`, `fetch_tasks` and `GithubTicketReader` read `params.iteration_field`) and a `BoardScope::of(&TrackerRow)` constructor used at every production site (`sync.rs:117, :1162`, `discover.rs:158`, `history.rs:73`, `api/handlers/trackers.rs:426`, `targets/http.rs:145`); it derives `Default` for test literals. Found at build: the r2.3 design had no path for either value. |
| `tracker/runtime.rs`, `api/handlers/trackers.rs:168`, `tracker/history.rs:64`, `tracker/writeback.rs:593` | `TransportFactory::transport(base_url, auth, retries, graphql)`; the callers pass `adapter.reads_graphql()`; the production factory calls `ReqwestRaw::allow_graphql` when true; test factories (`FakeFactory`) take and ignore it |
| `tracker/gitlab.rs` | `:409-417` rebuilds the `decode` error per use (no `Copy`); `tracker_for_key` moves to `forge.rs` with the host argument (H5) |
| `tracker/graphql.rs` (new) | `GraphqlQuery` and its lexer (H2.1) |
| `tracker/transport.rs` | `RawGet::post_graphql` (default `Err`), `ReqwestRaw`'s override (refuses unless built with `graphql: true`), `PolicyTransport`'s `graphql`; `user_agent`; the module doc (`:1`) names the GraphQL read |
| `tracker/github.rs` (new) | H1–H6, the queries, `GithubTicketReader`, `RecordedGithub` test support |
| `tracker/forge.rs` | `tracker_for_key(trackers, key, host)` (H5) |
| `api/handlers/trackers.rs` | `connect_failure` (`:256-282`) gains the `IterationFieldAmbiguous` arm, 422 with `{"error", "fields"}`; the tracker list (`:83-98`) also returns `url` and `iteration_field` from `params` (null for Jira and GitLab) |
| `targets/http.rs` | `forge_key` returns the host; H5 selection and loop; the `Github` read arm |
| DDL | `enum/sensei/tracker_kind.ddl` (`'github'`, and its comment at `:7`); `database/apply/before/sensei_tracker_kind_github.sql` (as `…_gitlab.sql`: dbd does not add a value to an existing enum) registered in `design.yaml`; the column comments in `table/sensei/trackers.ddl:47-54` and `tracker_tasks.ddl:43` name GitHub's values; `sprints.ddl:27-28` (`seq` is an ordering key); `design.dbml` enum and tracker notes; `make test-template` |
| CLI `crates/cli/src/tracker.rs` | `sensei tracker connect github --workspace <w> --level <l> --board-url <url> [--iteration-field <name>]`; token from `SENSEI_GITHUB_TOKEN`, else stdin; `Provider::Github`; GitHub words in `describe_error_for`; `iteration_field_ambiguous` prints the names; the help names the fine-grained token and the classic token's write reach |
| App `lib/types.ts`, `lib/tracker-errors.ts` | `TrackerKind` gains `'github'`; `TrackerConnection` (`types.ts:1773-1800`) gains `url` and `iteration_field` (nullable); `TRACKER_NAME.github = 'GitHub'`; `errorFor`'s early `board_not_found` and `task_not_on_board` branches (`:111-153`) gain a GitHub case ("No GitHub project at that address, or the token can't see it"); a `GITHUB_WORDS` map for `unauthorized` and `forbidden` ("The token needs Projects: read"); the create-token link for GitHub opens `https://github.com/settings/personal-access-tokens/new` |
| App Integrations (`integrations-state.svelte.ts`, `IntegrationsScreen.svelte`) | the GitHub radio; board URL and token; the optional iteration field, and on `iteration_field_ambiguous` a select of the returned names; card title **"GitHub · <owner>/<n> · <board_name>"** (the title at `:294-301` no longer falls to Jira's); **Replace token** re-sends `board_url = params.url`, `iteration_field = params.iteration_field` and the new token (`startReplace` `:470-484` gains a GitHub branch); "Write-back: not available for GitHub yet"; the token-reach note; and "A ticket in a repository the token can't read shows as not found" (H5.4 treats invisible and missing alike) |
| Docs | `decisions.md` D92, and amendment notes on D71 and D91; the analysis's frontmatter, §2 FT3/FT5, §4, §5's milestone mapping, §6's GitHub cells, §7's PR row, §8 and §9 O5/O6 marked superseded by D92; parent `tracker-jira-linkage.md` `:75` and R4 (`:277-283`, the read seam and "One method"), `:80-83` (adapters), `:115` (one board: Jira, GitLab or GitHub), R2 `:160` and `:162` (`seq` is an ordering key), R8's GET shape `:524` (`url`, `iteration_field`), R8 `:525, :545` (`iteration_field_ambiguous`), R10 `:561`, Out of scope `:682`, Future adapters `:701` (`Bearer` is built); `tracker-metrics-writeback.md` `:47` and gate 13 (`GetTransport` is GET plus validated queries); `tracker-levels.md` (a project's iterations are sprints); `tracker-gitlab.md` §G9 `:208-209` (the host filter for a bare key; `tracker_for_key` now in `forge.rs`); `docs/spec/dojo/targets-and-gates.md:707-713` (the GitHub reader uses the tracker's token per D92, the kind is `Github`, and the forge-sign-in and `forge_token_unavailable` text goes); screen specs `workspace-integrations.md`, `workspace-tickets.md` |

### Build order

1. **Fixture** (human-assisted: needs a token with `project` scope; the run halts and
   asks if none is given). Under `crates/senseid/tests/fixtures/github/`, recorded by
   `scripts/record-github-fixtures.py` (token from `GITHUB_TOKEN`, never written):
   - `project`: a sensei-hq project with one Iteration field: a completed, the
     current and a future iteration. Items: issues from **two** repositories, one with
     no iteration, one with no issue type; a pull request; a draft issue; an archived
     issue. Records `Q_VALIDATE_ORG`, `Q_BOARD`, every `Q_ITEMS` page.
   - `issue-on`, `issue-off`, `pr`: `Q_ISSUE` for an issue on the project, an issue of
     the same repository not on it, and a pull-request number.
   - `no-field`: a second project with no Iteration field (`Q_BOARD`, `Q_ITEMS`).
   - Two iteration fields, `errors` bodies, paging caps and `RATE_LIMITED` are
     **scripted** on the fake transport, since they cannot be recorded on demand.

   **Scrub, by allowlist**, consistently within a fixture set:
   - kept as recorded: `__typename`, `type`, `state`, `isArchived`, `closed`,
     `duration`, `hasNextPage`, `null`, booleans;
   - **remapped to stable fakes**: node ids (`PVT_1` project, `PVTI_1` item, `PVTIF_1` iteration field), every
     `databaseId` per object kind (issues `1001…`, users `9001…`), issue `number`s
     (`1, 2, …`, the same mapping in keys and queries), iteration ids (`it1…`),
     cursors (`c1…`);
   - **shifted**: every date and timestamp by one constant offset per set (whole
     days), so order and durations hold and real dates do not;
   - **rewritten**: owners → `acme`, repositories → `platform` / `core`, logins →
     `user1…`, and every other string (titles, bodies, names, urls) to a neutral value
     built from the replaced parts.

   `fixtures_are_scrubbed` is **structural**: it walks every JSON leaf and fails unless
   each string is a kept enum value or matches a fake pattern (`^PVT(I|IF)?_\d+$`, `^it\d+$`, `^c\d+$`, `^user\d+$`, `acme`, `platform`, `core`, the
   neutral templates, a date or timestamp), each `databaseId` is in a fake range,
   and no string matches `gh[pousr]_` or `github_pat_`. A real login or org name
   fails it without being listed.
2. Shared seams: `GraphqlQuery`, `graphql` on the read seam, User-Agent, `ValidateError`
   without `Copy`, host-aware `tracker_for_key`. The Jira and GitLab suites stay green.
3. **The `github` kind lands as one unit**: DDL value and hook, `TrackerKind::Github`,
   `github.rs` against the fixture, registration.
4. H5 `get_ticket`.
5. CLI, app, docs.
6. Live (human-assisted): connect the sensei workspace to the sensei project.

### Out of scope

Milestones; GitHub Enterprise Server; projects with more than 100 fields; the
project's Status field as ticket status; write-back; history and SDLC metrics
(`unsupported_by_tracker`); OAuth or the forge sign-in as the credential; estimates
(a Number field as points); sub-issues; past iteration membership; reading GitHub's
secondary rate limit (403) as `rate_limited`; a token-scope warning (needs response
headers in `RawResponse`).

## Done gate

| # | Check | Pass |
|---|---|---|
| E1 | `cargo test -p senseid` | 0 failed; every Jira and GitLab assertion holds; the `transport.rs` tests over `Ref` compile unchanged |
| E2 | `connect_params` | `…/orgs/Sensei-HQ/projects/3/views/1` → `orgs/sensei-hq/3`; `…/users/bob/projects/2/` → `users/bob/2`; `invalid_base_url` for `http://github.com/orgs/a/projects/1`, `https://github.example.com/…`, `https://github.com.evil.com/…`, `https://api.github.com/orgs/a/projects/1`, `https://u:p@github.com/…`, `https://github.com:444/…`, `…/projects/x`, `…/orgs/a`, `…/orgs/a%2fb/projects/1`; `https://GITHUB.COM/orgs/a/projects/1` → ok; no token → `token_required` |
| E3 | Validate (fake transport) | ≤ 3 requests; board_ref `project:PVT_…`; account = login; NOT_FOUND → `board_not_found`; INSUFFICIENT_SCOPES → `forbidden`; 101 fields → `http_error`; two iteration fields none named, or a missing named one → 422 `iteration_field_ambiguous` whose body lists both names; one named that exists → ok, `params.iteration_field` set; **one iteration field, no name given → ok, `params.iteration_field` = its name**; none → ok, null; a closed project → ok |
| E4 | `GraphqlQuery::new` | refused: `mutation M { x }`, `{ x }`, `query A { x } mutation B { y }`, `# c\nmutation M { x }`, `query A { x } query B { y }`, `subscription S { x }`; accepted: every query constant, `query A { x(s: "mutation") }`, `query A { x } fragment F on T { y }`; operationName = the name |
| E5 | GraphQL error mapping and scope | a 200 with `errors` and partial `data` → error, no data used; `RATE_LIMITED` → `rate_limited`; `SOMETHING_ELSE` → `http_error`; `{}` → `decode`; `Q_ISSUE` with only `NOT_FOUND` at `repository` or `repository.issue` → absent, `NOT_FOUND` elsewhere → `not_found`; `PolicyTransport::graphql` over a `RawGet` whose `post_graphql` is the default → `transport`, not a send; `ReqwestRaw` built with `graphql: true` posts to `/graphql`; a transport the factory built for a Jira or a GitLab tracker → `graphql` sends nothing (its listener receives no connection) |
| E6 | Sprint mapping (recorded fixture, fixed `now`) | completed → `closed`; current → `active`; future → `future`; `startDate 2026-11-02, duration 14` → `end_at 2026-11-16T00:00Z`; `seq` = days since epoch |
| E7 | Items (recorded fixture) | the PR, the draft and the archived item are not tasks; issues from two repos: keys lowercased `owner/repo#n`, scopes `github.com/owner/repo`; no iteration → `current_sprint_id IS NULL`; `issue_type` None where GitHub has none; `status`, `status_category`, `assignee`, `external_id` as H3.4 |
| E8 | Snapshot failures (DB) | a failing second items page → `last_error = 'http_error'` and the tasks untouched; 200 pages + `hasNextPage` → `http_error`; an unknown `iterationId` → `decode`; a named field gone → `not_found`; null name with two fields → `decode` |
| E9 | No iteration field (recorded `no-field`) | `sprints` empty, every task `current_sprint_id IS NULL`, `last_error IS NULL` |
| E10 | Board identity (DB) | the same project connected to workspace A, then (as `orgs/acme/3` and after a rename as `orgs/acme2/3`) to workspace B → `board_already_connected` |
| E11 | Discovery (recorded `issue-on/off`, `pr`) | on the project → found with its sprint; off → absent; a PR number → absent; a missing repository (scripted `NOT_FOUND`) → absent; `acme/x/y#1`, `acme/platform#2147483648`, `a_b/platform#1` → absent with **no request sent**; containment on page 2 of `projectItems` → found; 10 pages, no match → `http_error` |
| E12 | `get_ticket` | `acme/platform#12` → 200; an issue the token sees but off the project → 404 `ticket_not_found`, no title in the body; a bare `#12` from a folder with `repo_key github.com/acme/platform`, in a workspace with a GitLab tracker `full_path acme` and a GitHub tracker → the **GitHub** tracker answers; a bare key from a `gitlab.example.com` folder never reaches GitHub; two GitHub trackers, the first `Ok(None)` → the second answers; the first `Err(Forbidden)` → 502 `forbidden`, no content; a nonexistent issue and an off-board issue → the **same** 404 `ticket_not_found` body; a repository the first tracker's token cannot see (`NOT_FOUND`) → the second tracker still answers; a GitHub tracker with no mirrored task in the repo → still found; no GitHub tracker → 404 `no_tracker_for_key` |
| E13 | Guards and owner | PATCH write-back on a GitHub tracker → 409 `writeback_unsupported`; SDLC reasons `unsupported_by_tracker`; `/user` `type: "Bot"` → `NotAPerson`, `"User"` → `Person(id)` |
| E14 | Kinds and DDL | `every_registered_kind_is_a_label_of_the_ddl_enum` green; the upgrade hook adds `github` to an enum holding `jira, gitlab`; the dbml enum lists `github` |
| E15 | User-Agent | a request from `ReqwestRaw` carries `user-agent: sensei/<version>` |
| E16 | Fixtures | `fixtures_are_scrubbed` green |
| E17 | App and CLI | a GitHub tracker's card title, `forbidden` and `board_not_found` render no "Jira"; `GET …/trackers` on a GitHub tracker includes `url` and `iteration_field`, and Replace token sends both with the new token; `iteration_field_ambiguous` shows a select of the names; CLI `connect github` reads `SENSEI_GITHUB_TOKEN` |
| E18 | Gates | `make test-fast`, the app's tests, `cargo clippy` green |
| E19 | Live (human-assisted) | the sensei workspace connected to the sensei project; sprints = the project's iterations; tasks = the project's **non-archived issue items**; a session on a `12-…` branch of sensei links to `sensei-hq/sensei#12`; MCP `get_ticket` returns it |

## Wrong gate

- **A hand-written fixture.** GraphQL field names and nesting are easy to imagine. E6, E7, E9, E11 run on recorded responses.
- **GraphQL errors read as an empty board.** A 200 with `errors` must fail (E5, partial `data`).
- **A pull request or draft counted as a ticket** (E7 needs all item types).
- **A query that can write.** E4 sends hostile documents to the constructor; the path is fixed, so no other endpoint is reachable.
- **Containment by repository instead of by project** (E11, E12 use an issue off the project).
- **The tracker chosen by path alone** (E12's mirrored-repo case).
- **An error read as "not found"** in the candidate loop (E12's `Forbidden` case).
- **The ambiguous field silently resolved**, at validate or later (E3, E8).
- **Invented dates.** `end_at` comes from `startDate + duration`, never a default length.
- **A real org in a committed fixture** (E16).

## Claims (about the existing code; r1 verified 2026-11-09 by `sensei-claims-verifier`; r2 claims marked †)

| # | Claim | Check | Actual | Verdict |
|---|---|---|---|---|
| C1 | `GetTransport` has one method, `get_json` | read `tracker/mod.rs:205-213` | one, `:207` | CONFIRMED |
| C2 | `RawResponse` carries status, retry_after, body only | read `transport.rs:9-15` | those three | CONFIRMED |
| C3 | `ReqwestRaw` implements `RawGet` and `RawWrite` | `rg 'impl.*Raw(Get\|Write) for ReqwestRaw'` | 2; but `GetTransport` is `impl<R: RawGet>` and `Ref` (`:246`) is RawGet-only, so r1's `R: RawGet + RawWrite` broke tests | r1 MISLEADING → r2 H2.2 keeps the bound |
| C4 | the tracker client sets no User-Agent | `rg user_agent crates/senseid/src/tracker crates/senseid/src/http.rs` | 0 | CONFIRMED |
| C5 | `Header` auth is marked sensitive in `authenticate` | read `transport.rs:179-193` | `set_sensitive(true)` `:189`, tested | CONFIRMED |
| C6 | `seq` is i64 / `bigint not null` and read only to order | `rg '\bseq\b'` | 6 readers, all ordering, "later than" or the current-sprint tie-break (`sprint_replay.rs:75` found by the r2 audit) | r1 citation incomplete → r2.2 H3.3 cites all |
| C7 | `gitlab::tracker_for_key` filters GitLab trackers, by path, no host | read `gitlab.rs:670-686` | yes | CONFIRMED |
| C8 | every `#` key goes to `gitlab::tracker_for_key` | `rg tracker_for_key targets/http.rs` | 1 call, `:101` | CONFIRMED |
| C9 | `forge_key` drops the `repo_key`'s host | read `targets/http.rs:199-230` | `split_once('/')`, `:215` | CONFIRMED; r2 H5 keeps the host |
| C10 | dbd does not add a value to an existing enum; GitLab needed an apply/before hook | `ls database/apply/before/`, `design.yaml:169-172`, commit cab891ef8 | present | CONFIRMED (from the live failure, not dbd's source) |
| C11 | `ValidateError` carries no data for a 422 body, and is `Copy` | read `mod.rs:320-326` | `Tracker(TrackerError)`, `BoardNotFound`, `BoardProjectMismatch`; derives `Copy`, relied on at `gitlab.rs:409-417` | r1 MISLEADING → r2 H7 drops `Copy`, fixes gitlab.rs |
| C12 | `normalize_repo_key("https://github.com/Owner/Repo")` → `github.com/owner/repo` | `repo_key.rs:54` test | asserted | CONFIRMED |
| C13 | the sensei repo's `repo_key` is `github.com/sensei-hq/sensei` | psql | that value | CONFIRMED |
| C14 | the §G8 guards read `capabilities()` and name no kind | `rg "kind\s*(=\|<>\|in)\s*'"` over src, ddl | 0 outside test inserts | CONFIRMED |
| C15 | app `TrackerKind` is `'jira' \| 'gitlab'`; `errorFor(status, body, kind)` | `types.ts:1771`, `tracker-errors.ts:111` | yes; `board_not_found` and `task_not_on_board` return early by ternary | r1 MISLEADING (a words map would not reach them) → r2 H7 |
| C16 | sensei-hq/sensei has no milestones | `gh api …/milestones?state=all` | `[]` | CONFIRMED |
| C17 | the forge sign-in scopes are `read:org, user:email, repo` | `dojo/kavach.config.js:52` | yes | CONFIRMED |
| C18 † | trackers list `ORDER BY level, created_at, id` | `trackers.rs:473` | yes | CONFIRMED (verifier) |
| C19 † | `external_scope` is written only by connect | `rg external_scope db/pg_store/trackers.rs` | `:1119, :1142`, connect only | CONFIRMED (verifier) |
| C20 † | `connect_failure` (`api/handlers/trackers.rs:256-282`) is an exhaustive match whose `fail()` gives only `{"error"}` | read | yes | CONFIRMED (verifier) |
| C21 † | the write-back trait methods have no default body | read `mod.rs:501-549` | required; GitLab stubs them `gitlab.rs:553-599` | CONFIRMED (verifier) |
| C22 † | the Integrations card title and `startReplace` treat every non-GitLab kind as Jira | read `integrations-state.svelte.ts:294-301, :470-484` | yes | CONFIRMED (conformance auditor) |
| C23 † | the GitLab adapter judges sprint state against a passed-in `today` | `gitlab.rs:186-195, :456` | `milestone_row(m, today)`, `Utc::now()` at the call | CONFIRMED |
| C24 † | the factory builds every kind's transport as `PolicyTransport::new(ReqwestRaw::new(base_url, auth)?, retries)` | read `tracker/runtime.rs:44` | yes (security reviewer, r2) | CONFIRMED |
| C27 † | the tracker list returns only `board_name`, `sprint_source`, `sprint_source_changed_at`, `full_path` from `params` | read `api/handlers/trackers.rs:83-98` | yes | CONFIRMED (r2 audit) |
| C28 † | `targets-and-gates.md:707-713` says GitHub reads use the forge sign-in and a `GithubIssues` kind | read | yes | CONFIRMED (r2 audit); amended per H7 |
| C26 † | `TransportFactory::transport` is called at exactly four production sites | `rg '\.transport\(' crates/senseid/src` | `trackers.rs:168`, `history.rs:64`, `writeback.rs:593`, `runtime.rs:87` | CONFIRMED |
| C25 † | `forge::is_key_path` accepts paths of any depth, and the number parses as `u64` | read `forge.rs:19-31` | yes (security reviewer, r2) | CONFIRMED |

## Review record (r1 → r2, 2026-11-09)

Claims: 23 ledgered, 0 FALSE, 4 MISLEADING (C3, C6, C11, C15), each changed the
design as noted. Reviewers: plan-depth, spec-doc, data-correctness, conformance,
security, none `NOT RUN`.

| Sev | Finding | Reviewers | Change |
|---|---|---|---|
| HIGH | `query_json` could not be added under `impl<R: RawGet>`; the read seam lost its GET-only property; the first-token guard was bypassable | spec-doc, security, conformance, verifier | H2.1–2.2: `GraphqlQuery` with a lexer, `RawGet::post_graphql` default `Err`, fixed path |
| HIGH | Tracker selection by host-less key: a GitHub `#12` answered by GitLab | data | H5 keeps the host; selection by host |
| HIGH | The candidate loop's error rule undefined (an error could read as 404) | depth, security | H5.4 |
| HIGH | D71 misread; D92 did not amend D71, D91 | conformance | Purpose table; D92 and notes |
| HIGH | H7 missed parent, write-back spec, code doc comments | conformance | H7 Docs row |
| HIGH | The fixture list did not cover the gates | depth | Build order 1 |
| MED | `ValidateError` is `Copy` | verifier | H7 |
| MED | `projectItems`, `fields` truncated silently | depth, data | H1 validate 2, H4 paging |
| MED | A board unmirrored for a repo unreachable | data, depth | H5.3 second group |
| MED | State depends on the clock | depth | H3 `now` |
| MED | Queries not given | depth | H2.4 |
| MED | Rules with no gate | spec-doc | E3, E5, E8, E10, E12, E13 |
| MED | Fixture scrub unspecified | security | Build order 1, E16 |
| MED | Classic `repo` token can write | security | H2 Token, CLI and form note |
| MED | Card title and Replace token fall to Jira | conformance | H7 App |
| MED | `GITHUB_WORDS` would not reach `board_not_found` | verifier | H7 App |
| MED | E15 counted every item | spec-doc | E19 |
| MED | DDL comments, analysis sections not listed | spec-doc, conformance | H7 |
| LOW | `seq` citation; `iteration_field` pinned at validate; owner charset; 403 secondary limit; null `content`; frontmatter "§G9 reused" | various | H3.3, H1, H2.8, H3.4, frontmatter |

**r2 security sign-off (BLOCKED → fixed in r2.1):** `NOT_FOUND` contradicted H4 and
made 404/502 an existence oracle (MED → H2.3 exception, H5.4 uniform `Ok(None)`,
E5, E11, E12); the scrub kept real `databaseId`s, numbers and dates, and its test
was a deny list (MED → remap, shift, structural test); a Jira or GitLab credential
could be POSTed to `/graphql` (LOW → `reads_graphql`, E5); GitHub key shape
undefined (LOW → H4 key shape, E11).

**r2 conformance re-audit (→ r2.2):** `targets-and-gates.md` still described the
forge-sign-in reader (HIGH → H7 Docs); Replace token read fields the list API did
not return (MED → H7 api and types rows, E17); one unnamed iteration field had no
rule (MED → H1 validate 3, E3); parent `:115`, `:161`, R8 `:524` and GitLab §G9 not
listed (MED → H7 Docs); D92 named the classic token (MED → D92 rewritten); four
citations (LOW → fixed).

**r2.2 narrow re-audit (→ r2.3):** four LOW: parent citations `:162`, `:277-283`; `types.ts:1773-1800`; the iteration-field node id gets its fake `PVTIF_n` and pattern; H3.2's remedy agrees with H1 validate 3.

**Build amendment (G3, 2026-11-09):** `ConnectParams.sprint_field` and `BoardScope.params` (H7): validate and the snapshot had no way to read the chosen iteration field. Also observed while recording: GitHub omits archived items from `items` entirely (H3.4's filter stays as a second guard), and every H2.4 query ran unchanged against the live API.
