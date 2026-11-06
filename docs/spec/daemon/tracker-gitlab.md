---
name: GitLab tracker adapter
status: reviewed (r2, gate passed 2026-11-06)
date: 2026-11-06
parent: docs/spec/daemon/tracker-jira-linkage.md ("Integration seam", R1–R11, "Future adapters")
analysis: docs/analysis/2026-11-06-forge-trackers.md (FT1–FT6, §3)
also: docs/spec/tracker-levels.md (D40), docs/spec/daemon/tracker-metrics-writeback.md (W8, W10), docs/spec/daemon/tracker-sdlc-metrics.md (S4, S5)
---

# GitLab tracker adapter

> **r2 (2026-11-06):** rewritten after the claims verification and five reviewers.
> See "Review record" at the end for what changed and why.

## Purpose

Connect a sensei workspace to a **GitLab group or project**, on gitlab.com or a
**self-managed** instance on any tier, through the daemon path (FT1, amending D49
as D91). Sprints are **iterations** where the instance has them and
**milestones** otherwise (FT2). The adapter seam carries the rest:

- **Reused unchanged:** session linking, per-ticket and per-sprint **usage**
  (`usage.rs`), discovery (R11), the Tickets screen, and the Integrations screen's
  board list.
- **Built here:** the adapter, the key logic shared by the forges (`forge.rs`),
  `get_ticket` for GitLab keys (D63), the host policy, and kind-aware copy.
- **Reported as unsupported, never as an error or a placeholder:** write-back
  (W1–W10), history (S4), and everything history feeds. That covers sprint SDLC
  metrics, estimates and points, and status-category timings (§G8).

GitHub is slice 2 and reuses §G6.

## Resolved design

### G1 · What a GitLab board is

| Concept | Stored as |
|---|---|
| Board | A **group** (with its subgroups) or a **project** |
| `board_ref` | **`project:<id>`** or **`group:<id>`**: GitLab's numeric id from validate. A rename or transfer keeps it, so one board can never be connected twice under two paths. Project ids and namespace ids are separate sequences, hence the prefix. |
| `external_scope` | The board's `full_path` **at connect** (the column is written only by connect, `db/pg_store/trackers.rs:1091`). The **current** path is `params.full_path`, refreshed by every sync (§G4.6), and everything after connect reads that one. |
| `account` | The token's `username` from `GET /user` at validate |
| `params` | `{"board_name", "full_path", "version", "sprint_source": "iteration"\|"milestone", "sprint_source_changed_at"?, "allow_private_network": bool}` |

**Connect body** (`POST /api/workspaces/{id}/trackers`):

```json
{ "kind": "gitlab", "level": "sprint",
  "board_url": "https://gitlab.example.com/acme/platform/-/boards",
  "confirm_host": "gitlab.example.com",
  "api_token": "glpat-…",
  "allow_private_network": false }
```

**`connect_params`** (pure, no network). Every failure is a **422** with the code
(`api/handlers/trackers.rs:159`):
1. Parse `board_url`. Drop a leading `groups/` segment, and everything from the first `/-/` segment on. What remains is the **typed path**. An empty path → `invalid_base_url`.
2. Base URL = `normalize_base_url(board_url, host_rule)` (§G5.1). The shared rule discards a URL's path. A GitLab served under a subpath (`https://host/gitlab/…`) is therefore read as a board at `gitlab/…`, and validate then returns `board_not_found`. That copy says subpath installs are not supported (OPEN-G1).
3. `confirm_host` must equal the normalised host, compared case-insensitively. It may be omitted only for `gitlab.com`. Missing or different → **`host_not_confirmed`**.
4. `api_token` missing → `token_required`. `email_required` is never returned for GitLab.
5. `ConnectParams.board_ref` = the typed path (lowercased) and `account` = `""`. Both are **provisional**: validate replaces them (§G10 `tracker/mod.rs` `Validated`).

**Validate** (network; at most **5 GETs**). Every `:path` is **percent-encoded as one
segment** (`acme/platform` → `acme%2Fplatform`), as GitLab requires:
1. `GET /api/v4/user` → `username` (the account) and the owner (§G8). 401 → `unauthorized`. 403 → `forbidden` (the PAT lacks `read_api`).
2. `GET /api/v4/projects/:path` → 200 means a **project** board with `board_ref = project:<id>`. On 404, `GET /api/v4/groups/:path` → 200 means a **group** board with `board_ref = group:<id>`. 404 on both → `board_not_found`.
3. The probes of §G3 (`/version`, iterations).

The handler's copy of the provisional `board_ref` (`api/handlers/trackers.rs:169`, used by `connect_failure` at `:185` for the `board_already_connected` body) is replaced by the validated one, carried on the error.
`connect()` runs its board-owner check **after** validate, with the validated
`board_ref`, and uses the validated `account` for **all three** of its uses: the
pending tombstone, the keyring entry and the row (`connect.rs:61, :66-68, :77`).
Nothing is persisted before validate succeeds (C11). This is unchanged.

### G2 · Auth

A new `AuthScheme::Header { name: &'static str, token: ApiToken }`. GitLab uses
`PRIVATE-TOKEN`. Its `Debug` prints `Header(PRIVATE-TOKEN, ***)`. The outgoing
`HeaderValue` is built with **`set_sensitive(true)`**, as `basic_auth` already does
for `Basic`, so neither hyper's nor reqwest's `Debug` or trace output shows it. A
test asserts that `format!("{:?}", request)` does not contain the token. The PAT
needs `read_api`. OAuth is out of scope.

### G3 · Version and capability probes

Run at validate and at the start of every snapshot. All requests address the
board by **numeric id** (`/projects/<id>`, `/groups/<id>`), so no encoding is
needed after connect.

| Probe | Endpoint | Outcome |
|---|---|---|
| Version | `GET /api/v4/version` | `params.version` |
| Iterations | `GET /{projects\|groups}/<id>/iterations?per_page=1` | **200** → `iteration`. **403 or 404** → `milestone`. **Anything else** (429 after retries, 5xx, transport, decode) → the validate or snapshot **fails with that `TrackerError`**, and nothing is written. A failed probe never picks a sprint source. |

There is no tier lookup: `/license` needs an admin token, and the probe answers
the actual question.

**When the source changes** (an upgrade or downgrade): the snapshot carries only
the new source's sprints. The old ones are absent, so the existing vanish rule
gives them `removed_at` (`db/pg_store/trackers.rs:602-604`). Their `external_id`s
are prefixed by source (§G4.2), so no row is overwritten. The sync coordinator
compares the row's stored `params.sprint_source` with the snapshot's. When they
differ it adds `sprint_source_changed_at = now()` to the patch. Integrations then
shows, for 14 days after that timestamp: **"Sprints now come from iterations.
Earlier milestone sprints are marked removed."** (or the reverse). This is the
one-line notice decided in FT6/O1.

### G4 · Snapshot

Every page of every listing (§G7). **Any failed request fails the whole snapshot**
(parent R4, unchanged).

1. **Board**: `GET /projects/<id>` or `/groups/<id>` → `board_name` (`name`) and `full_path`. A 404 → `not_found` (the board was deleted).
2. **Sprints**: `external_id` = **`iteration:<id>`** or **`milestone:<id>`**. `seq` = the numeric `id`.
   - *Iterations*: `GET …/iterations?state=all&include_ancestors=true`. **`state` is an integer**: `1 → future`, `2 → active`, `3 → closed`, and anything else → `Decode`. `start_at` = `start_date`; `name` = `title`, or when the title is null `"<start_date> – <due_date>"` (both real fields; GitLab shows an untitled iteration by its dates).
   - *Milestones*: `GET …/milestones?state=all&include_ancestors=true`. On a **group** board, `include_descendants=true` is sent as well, so the listing holds parent-group, group and project milestones. Rows are de-duplicated by `id`.

     | milestone | → state |
     |---|---|
     | `closed` | `closed` |
     | `active`, `start_date` after today (UTC date) | `future` |
     | `active`, otherwise (overrunning or undated) | `active` |

   - **Dates** for both sources: `start_at` = `start_date` at 00:00 UTC. `end_at` = **`due_date` + 1 day** at 00:00 UTC, because GitLab's due date is inclusive and `usage.rs::sprint_window` treats the end as exclusive. `complete_at` = NULL. An absent date stays NULL. **An undated milestone is kept** as a sprint with NULL dates. Its usage reads `no_dates` (`usage.rs:69-74`), which is the truth.
3. **Issues**: `GET …/issues?scope=all&state=all&per_page=100`. GitLab serves merge requests from a separate `/merge_requests` resource (to be confirmed by the fixture, C26). If the Free fixture shows otherwise, items with a `merge_request` reference are dropped, as GitHub's pull requests are in slice 2.

   | TaskRow field | from |
   |---|---|
   | `external_id` | `id` (global) |
   | `key` | `references.full` with its path **lowercased** (`Acme/Platform#12` → `acme/platform#12`). One canonical case, so discovery's exact comparison (`discover.rs:169-170`) and `known_tasks` (`link.rs:36-39`) hold |
   | `scope` | `normalize_repo_key("<base_url>/<path part of key>")`. The path comes from `references.full`, not from `web_url`, whose route differs by issue type (`/-/issues/`, `/-/work_items/`) |
   | `summary` / `issue_type` | `title` / `issue_type` |
   | `status` | `state` (`opened` / `closed`) |
   | `status_category` | `opened → "new"`, `closed → "done"` |
   | `assignee` | `AccountRef(assignees[0].id)`, or None |
   | `created_at` / `updated_at` | as returned |
   | `points` | **`None`** ("not asked", `mod.rs:319-321`). Snapshot `points_field` = `None`. Estimates are unsupported (§G8). |
4. **Membership**: from each issue's `iteration.id` or `milestone.id`, whichever is the source, mapped to that sprint's prefixed `external_id`. `members` lists **every** sprint with `Some(ids)` (an empty vec when it has none). A ticket sits in the one sprint GitLab currently assigns it to. Past membership is history (out of scope).
5. **No sprints**: `sprints: []`. Every task gets `current_sprint_id = NULL` (`sprint.rs::no_sprints_means_backlog`). The tracker keeps the level the user chose (FT4). They are the Tickets screen's existing **Backlog** filter ("Backlog · N tickets", `tickets-state.svelte.ts:309`, strip at `:323-329`), and appear under the default All filter (`:248`). Nothing is synthesised.
6. **Params**: `Snapshot` gains `params_extra: serde_json::Map<String, Value>`. The adapter fills `full_path`, `version` and `sprint_source`. The coordinator merges it into the patch it already builds (`sync.rs:151-153`, `{"board_name": …}`), and the existing `params = params || $2` writes it in the same transaction (`db/pg_store/trackers.rs:707`). Jira returns an empty map, so its patch is byte for byte the same.

### G5 · Host policy (SSRF)

The threat: a user-supplied URL makes the daemon send a request carrying the
token to a host the user did not mean, or to a service on the machine's own
network. Applies to **every tracker transport** (all kinds) unless stated.

1. **Name rule** (`host_rule`, a `fn(&str) -> bool` over the shared `normalize_base_url`, which already refuses IP literals, userinfo, any explicit port other than `:443`, and non-https):
   - `gitlab.com` is always allowed.
   - Otherwise a DNS name containing a dot, excluding `localhost`, `*.localhost`, `*.local`, `*.internal`, `*.lan` and `*.home.arpa`.
   - With `allow_private_network: true`, those reserved suffixes are allowed too (`localhost` and `*.localhost` never are). `connect_params` selects between two `fn` rules by the flag.
   - **A trailing dot is refused** by the shared `normalize_base_url` (`localhost.` would otherwise pass every suffix test). Jira's existing test already rejects `acme.atlassian.net.`, so Jira is unaffected. The build confirms whether today's refusal comes from the Jira predicate or the shared rule, and puts it in the shared rule.
2. **Host confirmation** (§G1.3): the user types the host twice.
3. **One shared classifier.** `indexer/lib_indexer.rs:250` already has `is_public_ip` (it canonicalises IPv4-mapped IPv6 and has its own tests at `:1186-1189`). It moves to a new shared module, `crates/senseid/src/net/addr.rs`, as `classify(ip) -> AddrClass { Public, PrivateLan, Never }`, with the ranges below. `is_public_ip(ip)` becomes `classify(ip) == Public`, and the library fetcher keeps calling it. The fetcher's refusals only get stricter (6to4, Teredo, NAT64, the extra IPv4 blocks, and IPv6 outside `2000::/3`), and its existing tests stay green. The tracker resolver calls `classify` too, so there is no second list.
   **Address rule**, enforced in a custom `reqwest` DNS resolver (`ClientBuilder::dns_resolver`) on the tracker client. It filters every address of every resolution, so every new connection is re-checked (DNS rebinding included), and pooled connections reuse an address that was already checked.
   - Each address is **canonicalised first**: an IPv4-mapped or IPv4-compatible IPv6 address (`::ffff:a.b.c.d`) is classified as its IPv4 address.
   - **Allowed by default: global unicast only**, defined positively.
     - IPv4: refused are unspecified, `0.0.0.0/8`, loopback, RFC 1918, CGNAT `100.64.0.0/10`, link-local `169.254.0.0/16`, `192.0.0.0/24`, `198.18.0.0/15`, `240.0.0.0/4`, `192.88.99.0/24`, broadcast, multicast, and the documentation ranges.
     - IPv6: **only `2000::/3` is allowed**, and within it these are refused: 6to4 `2002::/16`, Teredo `2001::/32`, NAT64 `64:ff9b::/96` and `64:ff9b:1::/48`, ORCHID `2001:10::/28`, benchmarking `2001:2::/48` and documentation `2001:db8::/32`. Everything outside `2000::/3` is refused, which covers ULA `fc00::/7` (and so `fd00:ec2::254`), link-local, site-local `fec0::/10`, multicast and SIIT.
   - **`allow_private_network: true`** additionally allows **only** RFC 1918, ULA `fc00::/7` and CGNAT `100.64.0.0/10`. Even then, the known cloud metadata addresses inside those ranges stay refused: **`100.100.100.200`** (Alibaba) and **`fd00:ec2::254`** (AWS IPv6). Loopback, link-local (so `169.254.169.254`), unspecified and multicast also stay refused. Other services inside RFC 1918 or CGNAT cannot be enumerated. The checkbox's explanation says so: "Lets sensei send your GitLab token to addresses on your private network. Only tick this for a GitLab you run there."
   - No address left → `TrackerError::Transport`.
4. **No proxy**: the tracker client calls `.no_proxy()`. Otherwise reqwest honours `HTTPS_PROXY` / `ALL_PROXY`, the proxy resolves the host, and step 3 never runs.
5. **The flag travels with the row.** `TransportFactory::transport` and `::write_transport` gain a `NetPolicy { allow_private: bool }`. **All five** production call sites pass it: `runtime.rs:87` (`transport_for`, from the row), `history.rs:60` and `writeback.rs:590-591` (from the row they already hold), and `api/handlers/trackers.rs:162` (connect, from the body). Jira rows pass `allow_private: false`. "Private network allowed" shows on Integrations, and changing it needs a reconnect.
6. **Redirects stay off** (unchanged, C6).
7. **CSRF on connect**: no new surface. The tracker routes carry `local_caller_guard` (`route_layer` at `api/routes.rs:71`, defined at `api/origin_guard.rs:93-101`), which refuses a foreign Origin on reads and writes, inside the full router's `caller_guard` (`routes.rs:581`).

Explicit ports and subpaths stay refused or unsupported in v1 (OPEN-G1).

### G6 · Keys and links: `tracker/forge.rs`, provider-neutral, shared with GitHub

Pure functions with no endpoint and no auth. A **path character** is `[A-Za-z0-9_.\-/]`.
A path with an empty, `.` or `..` segment (`acme//x`, `acme/./x`, `acme/../x`) is
**not a key**. It is never matched, returned by `unknown_keys`, or fetched.

- **Full reference** `<path>#<n>`: a path not preceded by a path character, then `#`, then digits **not followed by a digit**. The path is compared case-insensitively against the task's lowercased key. It matches in prompts and branches, in any session, whatever its `repo_key`.
- **Bare `#<n>`** in a prompt: `#` **not preceded by a path character**, digits not followed by a digit. **Branch** `<n>-…` at the start of the branch name. A bare key resolves **only** against tasks whose `scope == session.repo_key`, and a NULL `repo_key` resolves nothing. The coordinator does not do this filtering; the adapter does (C12).
- So `acme/platform#12` counts **once** (the `#12` inside it is preceded by a path character), and `#123` never matches `#12`.
- `mentions` = matches for one task in one source. `matched_key` = the text as found.
- `unknown_keys` returns **full references only**, lowercased, whose path is the path of a task already on the board (the R11 "prefix the board already uses" rule). A bare `#n` is never sent to discovery.
- `fetch_tasks(keys)`: per key, `GET /projects/:path/issues/:iid` (path encoded). A 404 → absent. The project must be inside the board: for a project board, its id equals the board's id. For a group board, its `namespace.full_path` equals or starts with the group's `full_path` + `/`. A project outside the board → absent, not fetched further.

Jira's `match_keys` / `unknown_keys_in` are unchanged.

### G7 · Pagination

`RawResponse` carries no pagination headers (C8). Pages `page=1,2,…` are read with
`per_page=100` until a page has **fewer than 100** items. At most **201** pages per
listing: page 201 is read only to prove that 200 full pages were the end. A
non-empty page 201 fails the snapshot with `http_error` instead of returning a
partial board as whole. So a board of exactly 20,000 items syncs.

### G8 · Unsupported capabilities

`TrackerAdapter` gains `fn capabilities(&self) -> Capabilities { history: bool, writeback: bool }`.
Jira returns `{true, true}`. GitLab returns `{false, false}`.

- **History task**: skips a tracker with `history: false`, leaving `history_through`, `terms_at` and the history error NULL.
- **SDLC reads**: today a task whose `tracker_tasks.history_through` is NULL gets `history_gap_from = i64::MIN` (`tracker_history.rs:309-313`), so its sprints read `history_pending` (`sdlc/sprint_replay.rs:190`, `:387-388`). A tracker whose estimates are not `learned` (`terms_at IS NOT NULL AND points_field IS NOT DISTINCT FROM coalesce(estimation_field,'none')`, `tracker_history.rs:261-263`) gives `not_synced` for `estimated_points` (`sdlc/ticket.rs:172`) and `velocity` (`sprint_replay.rs`, asserted at `:822`). Both say the data is coming. (`lead_hours` → `not_synced` at `sdlc/ticket.rs:362` comes from a missing `issue_created_at`, not from history. GitLab supplies `created_at`, so it is unaffected and unchanged.) For a tracker whose adapter has `history: false`, every reason that depends on history (sprint SDLC keys, estimates, points, status timings) reads **`unsupported_by_tracker`** instead. It is a **registered reason code**: a new `sdlc_metrics` row in `database/import/staging/reason_codes.jsonl`, with a precedence not yet used in the domain (`unique (domain, precedence)`, `database/ddl/table/sensei/reason_codes.ddl:32`), kind `normal`, summary **"Not available from this tracker"**, and detail "This tracker's adapter doesn't read ticket history, so figures built from it can't be computed." The wording is kind-neutral because one jsonl summary serves every kind. The Tickets screen shows it through its existing `GET /api/reason-codes/sdlc_metrics` lookup (`tickets-state.svelte.ts:84`). The `registered_reasons` checks (`sdlc/ticket.rs:972`) gain a `history: false` input.
- **Write-back**: `PATCH …/trackers/{tracker} {"writeback_enabled": true}` and `PUT …/trackers/{tracker}/fields` on a `writeback: false` tracker → **409 `writeback_unsupported`**. The write-back pass skips such trackers. Integrations shows "Write-back: not available for GitLab yet" in place of the toggle.
- The GitLab adapter's write-back methods (`find_comment`, `create_comment`, `update_comment`, `check_fields`, `field_in_use`, `write_fields`, `issue_exists`) return `Err(UnsupportedTracker)`, and `field_ref_shape` returns false. They are unreachable behind the guards above, and the guards are tested (D10).
- **`owner()`**: `GET /user` → `Person(AccountRef(id))`, or `NotAPerson` when `bot: true`.

### G9 · `get_ticket` for GitLab keys (D63/D71)

`targets/http.rs:88-111` picks a tracker only for Jira keys and reads with
`JiraTicketReader`. It becomes **kind-dispatched**:

- A key containing `#` (`acme/platform#12`, case-insensitive) is a forge key. It is **not upper-cased**: today the route upper-cases every key (`targets/http.rs:79`) and passes that to `start_work` (`:146`), and from there to `tracker_task_detail`, `discover_on_miss` and `record_fetch_link`. A forge key instead takes its canonical form, lowercased path plus `#n` (§G4.3), on all of those paths. Jira keys keep upper-casing.
- **A bare `#12`**: the route receives only `folder` (the MCP cwd, `targets/http.rs:22`) and resolves the folder and workspace (`:43-74`). The new step goes from the folder's `repository_id` to `repositories.repo_key`, through the existing `repo_key_for_repository` (`db/pg_store/metrics.rs:940`). The key becomes `<path of that repo_key>#12`. A folder with no repository or no `repo_key` → 404 `no_tracker_for_key`.
- **Which tracker:** the workspace's GitLab trackers whose `params.full_path` is the key's path or a path prefix of it (on `/` boundaries). The **longest** matching `full_path` wins. Ties go by `TrackerLevel` order (`epic`, `sprint`, `task`), then `created_at`, the order trackers already list in. No candidate → 404 `no_tracker_for_key`. This textual prefix only **selects** the tracker. It is not the access check.
- **Containment, before anything is mapped or returned:** read the issue, then the issue's project by `project_id` (`GET /projects/<project_id>`, one extra request). For a project board, `project_id` must equal the board's id. For a group board, the project's `namespace.full_path` must equal the group's **current** `full_path` (read from `/groups/<id>`, not the stored one) or start with it + `/`. A project outside the board → 404 `ticket_not_found`, and nothing from the issue is returned. A PAT often sees more than the board, and this check is what stops `get_ticket` from becoming a read of anything the token can see.
- `GitlabTicketReader` (in `tracker/ticket.rs`, implementing the existing `TicketReader`) maps: `title`, `description`, `state`, `issue_type`, the current iteration or milestone title as `sprint`, `assignees[0].username`, `web_url` as `url`. It reads `acceptance_criteria = []` with `AcceptanceCriteriaSource::NotConfigured`: GitLab has no criteria field, and parsing them out of the description is out of scope.
- 404 → `Ok(None)`. Any other failure → `Err`, never a placeholder (the trait's contract).

### G10 · Changes, by surface

| Surface | Change |
|---|---|
| `tracker/mod.rs` | `TrackerKind::Gitlab` (`"gitlab"`); `AuthScheme::Header`; `ParamError::HostNotConfirmed` (`host_not_confirmed`); `Validated` gains `account: Option<String>`, `board_ref: Option<String>` and `params: serde_json::Map`; `Snapshot.params_extra`; `Capabilities` + `capabilities()`; `adapters()` registers `GitlabAdapter` |
| `tracker/connect.rs` | Owner check after validate; validated `board_ref` and `account` for the tombstone, the keyring and the row; the row's `params` = `{board_name} ∪ validated.params` |
| `tracker/sync.rs` | Merges `params_extra` into the existing patch; `sprint_source_changed_at` (§G3) |
| `net/addr.rs` (new; `is_public_ip` moved from `indexer/lib_indexer.rs:250`, extended to `classify`), `tracker/transport.rs`, `runtime.rs`, `http.rs` (`crates/senseid/src/http.rs`) | `Header` auth; the resolver; `.no_proxy()`; `NetPolicy` at all 5 sites |
| `tracker/history.rs`, `writeback.rs`, `sdlc/*`, `db/pg_store/tracker_history.rs`, `api/handlers/trackers.rs` | §G8 guards and the `unsupported_by_tracker` reason |
| `database/import/staging/reason_codes.jsonl` | the `unsupported_by_tracker` row (§G8); `make test-template` |
| `tracker/forge.rs`, `tracker/gitlab.rs` (new) | §G6, the adapter |
| `tracker/ticket.rs`, `targets/http.rs` | §G9 |
| DDL `enum/sensei/tracker_kind.ddl` | `'gitlab'` added, following the add-value convention of `enum/…/assistant_family.ddl:20`; `make test-template`. The column comments in `table/sensei/trackers.ddl:47-54` name GitLab's values. |
| `database/design.dbml` | `Enum tracker_kind` gains `"gitlab"`; the tracker notes (`:2955-2958`) gain GitLab |
| CLI `crates/cli/src/tracker.rs` | `sensei tracker connect gitlab --workspace <w> --level <l> --board-url <url> --confirm-host <host> [--allow-private-network]`. The token comes from `SENSEI_GITLAB_TOKEN`, else the first line of stdin (Jira's pattern: `SENSEI_JIRA_TOKEN`, `read_token`). The connect client timeout becomes **180 s** (5 validate requests × 30 s + the keyring call) for GitLab. |
| App: Integrations | A **GitLab** radio (replacing "GitLab, coming soon"); board URL, confirm host, token, and a private-network checkbox with one line of explanation; `Sprints: iterations\|milestones`; the §G3 notice; "Write-back: not available for GitLab yet"; **Replace token** for GitLab re-sends `board_url` built from `base_url` + `params.full_path`, `confirm_host` = the stored host, `allow_private_network` = the stored `params.allow_private_network`, and the new token |
| App: kind-aware copy | Every string naming Jira on a shared path takes the tracker's kind: `app/src/lib/tracker-errors.ts` (`:14` `CREATE_TOKEN_URL`, `:17, :23, :24, :26` (`token_required`, which GitLab returns), `:28, :29, :30, :49, :56, :57, :59, :69, :104`, plus `host_not_confirmed` and `writeback_unsupported`; SDLC reasons do not live here); `tickets-state.svelte.ts:220` (`no_dates`); `integrations-state.svelte.ts:256`; `IntegrationsScreen.svelte:72, :132, :164, :382` (shared), with the Jira-only form copy at `:286, :320, :324, :333` shown only for the Jira radio; `integrations-state.svelte.ts:375` (connect kind hard-coded to `'jira'`); `TicketsScreen.svelte:49`; `api/handlers/scheduled_tasks.rs:67` |
| Docs | **Parent** `tracker-jira-linkage.md`: `:80` (one adapter), `:157` (`board_ref` for GitLab = `project:<id>`/`group:<id>`, with the source in `params`), R4 `:280-283` (Header auth, resolver, NetPolicy), R5 `:388`, R8 `:511, :530-531` (`host_not_confirmed` 422, `writeback_unsupported` 409, GitLab's validate count), R10 `:546-553`, `:662`, and the "Future adapters" table (iteration `state` is an integer; `board_ref` does not name the source). `tracker-metrics-writeback.md` W7 `:375` (PUT `/fields` → 409), W8 `:426-428` (PATCH → 409) and W10 `:514-524` (client and `write_transport`: resolver, `.no_proxy()`, `NetPolicy`). `tracker-sdlc-metrics.md` S4/S5, S6 (the reason table) and S8 (`estimated_points` reasons): `unsupported_by_tracker`. `tracker-levels.md:27, :48, :65-67`: forge milestones are **sprints** on a sprint-level board, distinct from T3/V11 planning milestones read from an epic tracker. Screen specs `workspace-integrations.md` (`:43, :90-99, :121, :132-139, :151-180, :216, :218`) and `workspace-tickets.md` (`:77, :105-106`). `decisions.md` D91. `backlog.md:207-212` (closed by this spec). |

### Build order

0. **Security sign-off** of §G5 (`sensei-security-reviewer` on the rewritten section) before step 3.
1. **Fixtures**, recorded and scrubbed, under `crates/senseid/tests/fixtures/gitlab/`:
   - **Free**: a gitlab.com free group, which has milestones and no iterations. It also covers a merge request absent from `/issues`, an undated milestone, a parent-group milestone on a subgroup issue, and a `task`-type issue.
   - **Premium**: a gitlab.com Premium/Ultimate trial group, with iterations including an untitled one.
   - **Human prerequisite** for both. Fixtures are recorded with `curl`, not hand-written.
2. **Shared seams**, nothing GitLab-specific: `AuthScheme::Header`, the resolver, `.no_proxy()`, `NetPolicy` at 5 sites, `Validated` and `Snapshot` fields, the `params_extra` merge, `capabilities()` and the §G8 guards with `unsupported_by_tracker`. The full Jira suite stays green.
3. `forge.rs` (pure, unit-tested).
4. **The `gitlab` kind lands as one unit:** the DDL enum value and `make test-template`; `TrackerKind::Gitlab`; `gitlab.rs` against the fixtures; registration in `adapters()`; replacing the three "gitlab is unsupported" tests (`mod.rs:1274`, `api/handlers/trackers.rs:998-1003`, and the app's `IntegrationsScreen.spec.svelte.ts:94-98` "coming soon" in step 6). These land together because `mod.rs:1274` pins the variant, and `every_registered_kind_is_a_label_of_the_ddl_enum` (`mod.rs:1278`, which iterates `adapters()`) pins registration to the DDL.
5. §G9 `get_ticket`.
6. CLI, app, copy, docs (§G10).
7. Live (D18).

### Out of scope

Write-back, history, SDLC sprint metrics, estimates and points for GitLab (all
`unsupported_by_tracker`); OAuth; custom CA and self-signed TLS (FT6/O4); explicit
ports and subpaths (OPEN-G1); scoped `workflow::` labels as statuses; acceptance
criteria parsed from descriptions; past sprint membership; epics; GitHub (slice 2).

## Open for the user

| # | Question | Default |
|---|---|---|
| OPEN-G1 | Self-managed GitLab on a non-443 port or under a subpath: support in v1? | Not supported |
| OPEN-G2 | The private-network opt-in as specified (§G5.1, §G5.3: relaxes the reserved-name suffixes; allows RFC 1918, ULA and CGNAT; never loopback, link-local or metadata) | As specified |

## Done gate

| # | Check | Pass |
|---|---|---|
| D1 | `cargo test -p senseid tracker::` and `targets::` | 0 failed; every existing Jira test passes unmodified (except the two of D12) |
| D2 | `connect_params` tests | `…/groups/acme/-/boards` → typed path `acme`; `…/acme/platform/-/issues` → `acme/platform`; `confirm_host` mismatch or missing → 422 `host_not_confirmed`; `https://1.2.3.4`, `:8443`, `http://`, `localhost`, `x.internal` (flag off) → `invalid_base_url`; `x.internal` with the flag → Ok; missing token → `token_required` |
| D3 | Validate (fake transport) | requests `/api/v4/projects/acme%2Fplatform`; project 200 → `board_ref = project:<id>`; project 404 + group 200 → `group:<id>`; both 404 → `board_not_found`; `account` = `/user` username; **at most 5 requests** |
| D4 | Board identity (DB) | connect `acme/platform` (id 123) in workspace A, then `acme/core` (id 123) in workspace B → `board_already_connected` |
| D5 | Probes | iterations 200 → `iteration`; 403 → `milestone`; 404 → `milestone`; 500 and timeout → the snapshot fails with `http_error` / `transport`, and **no sprint row's `removed_at` changes** |
| D6 | Sprint mapping | iteration `state` 1/2/3 → future/active/closed; `"current"` (a string) → `decode`; milestone table rows; `due_date 2026-11-13` → `end_at 2026-11-14T00:00Z`; undated → NULL dates; untitled iteration named by its dates |
| D7 | Free fixture snapshot | sprint count = the fixture's distinct milestones (ancestor milestone included); a subgroup issue on a parent-group milestone has that sprint as current; the merge request is not a task; task-type issue scope = `normalize_repo_key("git@<host>:acme/platform.git")` |
| D8 | Premium fixture, source switch (DB) | milestone snapshot, then iteration snapshot with a colliding numeric id 53 → two sprint rows, the milestone row has `removed_at IS NOT NULL`, and `params.sprint_source_changed_at` is set |
| D9 | No sprints | empty fixture → `sprints` empty, every task `current_sprint_id IS NULL`, `last_error IS NULL`; a failing milestone page → `last_error = 'http_error'` and the tasks untouched |
| D10 | Guards | PATCH `writeback_enabled:true` and PUT `/fields` on a GitLab tracker → 409 `writeback_unsupported`; after a history run, the GitLab tracker's `history_error`, `history_at` and `terms_at` (`trackers.ddl:29-31`) are NULL; its `/sprints` SDLC reasons (commitment and scope, `velocity`) and a task's `estimated_points` reason are `unsupported_by_tracker` (not `history_pending` / `not_synced`); `lead_hours` is computed |
| D11 | `forge.rs` | two repos with `#12` each: a bare `#12` links only the session's repo's task; NULL `repo_key` → none; `fix acme/platform#12` → mentions 1; `#123` → no link to `#12`; branch `12-fix` links; `unknown_keys` never returns a bare key; discovery for `acme/platform#99` against `Acme/Platform#99` → not_on_board empty |
| D12 | Kind tests | `mod.rs:1274`, `trackers.rs:998-1003` and `IntegrationsScreen.spec.svelte.ts:94-98` replaced by positive `gitlab` cases; `every_registered_kind_is_a_label_of_the_ddl_enum` green; the dbml enum lists `gitlab` |
| D13 | Resolver, through the **production `ReqwestRaw`** with a stub resolver | refused by default: `10.0.0.5`, `127.0.0.1`, `::1`, `::ffff:127.0.0.1`, `::ffff:169.254.169.254`, `169.254.169.254`, `100.64.0.1`, `0.1.2.3`, `64:ff9b::a00:1`, `fd00:ec2::254`; with `allow_private`: `10.0.0.5` and `fd12::1` pass, while `127.0.0.1`, `169.254.169.254` and `fd00:ec2::254` are still refused; a public address passes; with `HTTPS_PROXY` set, the request still goes direct (the proxy stub receives nothing) |
| D14 | `get_ticket` | `/api/tickets/acme%2Fplatform%2312/fetch` on a GitLab workspace → 200 with title, url, `acceptance_criteria_source = not_configured`; an unknown iid → 404; a 500 from GitLab → 502 with the code; **a key whose issue's project is outside the board** (the fake returns it, the token can see it) → 404 `ticket_not_found` with no title in the body; `acme/../x#1` → 404, no request sent |
| D19 | Header redaction | `format!("{:?}", request)` for a `Header`-auth request does not contain the token; `localhost.` and `x.local.` → `invalid_base_url`; with `allow_private`, `100.100.100.200` is refused and `2002:7f00:1::` (6to4 of 127.0.0.1) is refused |
| D15 | Pagination | 200 full pages + an empty page 201 → Ok with 20,000 items; a non-empty page 201 → `http_error` |
| D16 | App and copy | component tests: a GitLab tracker with `unauthorized` and an undated sprint render no "Jira"; the GitLab radio is enabled; the source-change notice shows within 14 days of `sprint_source_changed_at` |
| D17 | Gates | `make test-fast`, the app's tests, and `cargo clippy` green |
| D18 | Live (human-assisted) | connect the Free group and the Premium group; after one sync, `select count(*) from sensei.sprints where tracker_id = $1 and removed_at is null` equals what GitLab's UI lists; a Claude session on branch `12-…` in a repo of the board links to `#12`; MCP `get_ticket` on that key returns it |

## Wrong gate

Ways this passes the done gate and is still wrong:

- **A hand-written fixture.** Iteration `state` was a string in the draft because nobody had looked at a real response. D6 and D8 run on **recorded** fixtures, or they prove nothing.
- **The probe fell through on a timeout.** Iterations would flip to milestones, and every iteration sprint would get `removed_at`. D5 must include a timeout, not just 200 and 403.
- **The resolver tested beside the client, not inside it.** D13 goes through `ReqwestRaw` with the proxy variable set. A unit test of the classifier alone can pass while the client never calls it.
- **`NetPolicy` threaded through `transport_for` only.** History and write-back would build Jira-default transports for a private GitLab and fail with `transport`. Grep the 5 sites.
- **Bare `#12` linked across repos.** A single-repo test cannot catch it. D11 needs two repos.
- **"Not available" shown where the real answer is "pending" on Jira.** The `unsupported_by_tracker` reason must be driven by the adapter's capability, never by the kind's name in SQL, and Jira's reasons must be unchanged (D1).
- **A path-keyed board.** Any code path that looks a board up by `full_path` after connect reintroduces the rename bug D4 guards against.
- **Invented dates.** A `start_at` or `end_at` not derived from a real `start_date` / `due_date` (D83).

## Claims (about the existing code; r1 ledgered and verified 2026-11-06 by `sensei-claims-verifier`; r2 claims marked †)

| # | Claim | Check | Actual | Verdict |
|---|---|---|---|---|
| C1 | `TrackerKind` has only `Jira` | `tracker/mod.rs:28-43` | only Jira | CONFIRMED |
| C2 | DDL `tracker_kind` holds only `'jira'` | `enum/sensei/tracker_kind.ddl` | `'jira'` | CONFIRMED |
| C3 | `AuthScheme` has only `Basic` | `mod.rs:161-171` | Basic; Debug redacts | CONFIRMED |
| C4 | `normalize_base_url` refuses IP, explicit port, userinfo, non-https, and **discards** the path | `tracker/http.rs:21-42` | as stated | CONFIRMED |
| C5 | its host predicate is a `fn(&str) -> bool` | `http.rs:24` | fn pointer | CONFIRMED |
| C6 | redirects off; 3xx → `Redirect` | `transport.rs:142`, `:95`, test `:372` | as stated | CONFIRMED |
| C7 | nothing in the tracker transport checks resolved addresses; the client honours proxy env vars | `rg 'dns_resolver\|no_proxy' crates/senseid/src` | 0 hits; `http.rs:13-15` sets timeouts only | CONFIRMED |
| C8 | `RawResponse` = status, retry_after, body | `transport.rs:11-15` | 3 fields | CONFIRMED |
| ~~C9~~ | ~~`params` written only at connect, no sync updates it~~ | `sync.rs:150-163`, `db/pg_store/trackers.rs:707`, `:1092` | **every sync merges `params \|\| patch`** | **FALSE (r1)**. Cost: r1 invented a second merge path. r2 extends the existing patch (§G4.6). |
| C10 | `BoardScope` = base_url + board_ref | `mod.rs:287-290` | 2 fields | CONFIRMED |
| C11 | `connect()` validates before any secret or row write | `connect.rs:35-85` | validate `:55` precedes tombstone `:61`, keyring `:68`, row `:84` | CONFIRMED |
| C11a† | `connect()` uses `params.account` in three places: tombstone, keyring, row | `connect.rs:61, :66-68, :77` | 3 | CONFIRMED (r1 said two: MISLEADING, fixed) |
| C12 | the coordinator delegates matching to `adapter.match_links` and does **no** scope filtering itself; the scope filter in sync's tests lives in a test FakeAdapter | `sync.rs:200-226`, `:766`, `:1427-1493` | as stated | CONFIRMED (r1 credited the coordinator: MISLEADING, rewritten) |
| C13 | `SessionText.repo_key` = `repositories.repo_key`, written through `normalize_repo_key` | `db/pg_store/trackers.rs:770-805`; `folders.rs:854`, `:1481` | as stated | CONFIRMED |
| C14 | zero sprints → NULL current sprint | `sprint.rs:81`; `db/pg_store/trackers.rs:351-353` | as stated | CONFIRMED |
| C15 | the default `board_terms` leads history to record `not_found` | `mod.rs:532-538`, `history.rs:80-84` | as stated | CONFIRMED |
| ~~C16~~ | ~~exactly 4 production transport call sites~~ | `rg '\.transport\(\|\.write_transport\(' crates/senseid/src` | **5**: `runtime.rs:87`, `history.rs:60`, `writeback.rs:590`, `:591`, `api/handlers/trackers.rs:162`; history and write-back bypass `transport_for` | **FALSE (r1)**. Cost: `NetPolicy` would have missed history and write-back. Fixed in §G5.5. |
| C17 | `trackers.rs` asserts `gitlab` → `unsupported_tracker` | `trackers.rs:998-1003` | present | CONFIRMED |
| C17a† | `mod.rs` asserts `TrackerKind::parse("gitlab") == None` | `mod.rs:1274` | present | CONFIRMED |
| C18 | connect = `POST /api/workspaces/{id}/trackers`; ParamErrors → **422** | `trackers.rs:135-159` | as stated | CONFIRMED (r1 said 400: FALSE, fixed) |
| C19 | Integrations has one radio, `jira` | `IntegrationsScreen.svelte:271` | as stated | CONFIRMED |
| C20 | the CLI's connect has only `ConnectKind::Jira`; its token comes from `SENSEI_JIRA_TOKEN` or stdin, never a prompt | `cli/src/tracker.rs:10, :64-89, :333-342` | as stated | CONFIRMED (r1 said "prompted": FALSE, fixed) |
| C21 | Jira: a failed page fails the snapshot; an unknown sprint state → Decode | `jira.rs:1424`, `:1435` | both | CONFIRMED |
| C22 | a sprint absent from a snapshot gets `removed_at`; the upsert on `(tracker_id, external_id)` clears it | `db/pg_store/trackers.rs:579-618` | as stated | CONFIRMED. Hence the prefixed `external_id` (§G4.2). |
| C23 | PATCH body is exactly `{"writeback_enabled": bool}` | `trackers.rs:449-460` | as stated | CONFIRMED |
| C24 | one key in two scopes = two tasks | `link.rs:152` | present | CONFIRMED |
| ~~C25~~ | ~~`points_field = None` means "no estimation field"~~ | `mod.rs:319-337`, `jira.rs:340-344`, `db/pg_store/trackers.rs:624` | `None` = **not asked** (column untouched); no field = `Some("none")`; points trusted only once `terms_at` is set by history | **FALSE (r1)**. Cost: r1's weight mapping would never have been trusted. r2 reports no points (§G4.3, §G8). |
| C26† | GitLab's `/issues` excludes merge requests | GitLab API (external) | **UNCHECKABLE in repo**; settled by the Free fixture (D7) | load-bearing only for D7, which tests it |
| C27† | `sprint_window` treats `end_at` as an **exclusive** end: `[from, to)` with `to = complete_at.or(end_at)` | `usage.rs:59-76` | as stated | CONFIRMED (by me, re-read) |
| C28† | `discover.rs` compares asked and fetched keys by exact string | `discover.rs:169-170` | `got.contains(asked)` | CONFIRMED |
| C29† | `history_pending` comes from per-task `history_through` NULL (`sprint_replay.rs:190`, `:387-388`); `not_synced` for estimates and velocity from `learned` (`tracker_history.rs:261-263`, `sdlc/ticket.rs:172`); `ticket.rs:362` is `lead_hours`, unrelated | as cited | as stated | CONFIRMED (r2 cited `ticket.rs:362` as history-driven and `history_through` as a tracker column: MISLEADING, fixed in §G8 and D10) |
| C30† | `get_ticket`'s route picks only Jira trackers and reads with `JiraTicketReader`; `TicketReader` is a trait | `targets/http.rs:88-111`; `tracker/ticket.rs:39` | as stated | CONFIRMED (re-read) |
| C31† | there is no "No sprints yet" copy; the Tickets screen groups sprintless tickets as "N in no sprint" | `rg -i 'no sprints\|sprints yet' app dojo crates`; `tickets-state.svelte.ts:329` | as stated | CONFIRMED (r1 claimed an existing "No sprints yet": FALSE, fixed in §G4.5) |
| C32† | the tracker routes carry `local_caller_guard` (foreign Origin refused on reads and writes) inside `caller_guard` | `api/routes.rs:71`, `:581`; `origin_guard.rs:93-101` | as stated | CONFIRMED (r2 cited `:622`, the degraded router: MISLEADING, fixed) |
| C33† | `assistant_family.ddl:20` documents the enum add-value convention | `database/ddl/enum/*/assistant_family.ddl:20` | per the spec-doc reviewer | to re-check at build step 4 |

## Review record (r1 → r2, 2026-11-06)

Reviewers: `sensei-claims-verifier`, `sensei-plan-depth-reviewer`, `spec-doc-reviewer`,
`sensei-data-correctness-reviewer`, `sensei-spec-conformance-auditor`,
`sensei-security-reviewer`. None was `NOT RUN`. I verified every HIGH finding
against the code myself before acting on it.

| Sev | Finding | Corroborated by | r2 |
|---|---|---|---|
| HIGH | `board_ref` = path, which a rename can move or reuse | data | `project:<id>` / `group:<id>` (§G1) |
| HIGH | sprint `external_id` unspecified; iteration and milestone ids collide, and the upsert overwrites | data, verifier (C22) | prefixed ids (§G4.2) |
| HIGH | `due_date` at midnight drops a sprint's last day | data | `+1 day` (§G4.2) |
| HIGH | iteration `state` is an integer (inherited from the parent) | data, conformance | §G4.2; parent amended (§G10) |
| HIGH | no history means `history_pending` / `not_synced` forever | conformance | `unsupported_by_tracker` (§G8) |
| HIGH | `get_ticket` is Jira-only | conformance | §G9 |
| HIGH | the build order is self-contradictory and has no DDL step | depth, spec-doc, conformance | reordered (step 4 lands as one unit) |
| MED | proxy bypass; mapped IPv6 / NAT64 / incomplete deny list; the opt-in reaches metadata | security | §G5.3–5.4 |
| MED | paths not URL-encoded | depth, conformance | §G1, §G6 |
| MED | ancestor milestones dropped; `web_url` scope; key case; match boundaries | data | §G4.2–4.3, §G6 |
| MED | C9, C16, C25 false | verifier | see the ledger |
| MED | O1 notice, security gate, D49 decision, docs, copy, Replace token, tracker-levels conflict, validate count vs the CLI timeout | conformance, spec-doc | §G3, Build order 0, D91, §G10 |
| MED | the opt-in cannot reach `.internal` / `.local`; how the live gate (D18) reaches CE | depth | §G5.1; live on gitlab.com Free + Premium |
| LOW | 422 not 400; no prompt; the tombstone account; PUT /fields; `issue_exists` signature; D11 removed rows; 20,000 cap; points | all | fixed |

**r2 security sign-off round** (`sensei-security-reviewer`, SIGN-OFF: NO, then fixed):
the CGNAT opt-in reached Alibaba's metadata `100.100.100.200` (MED → refused, §G5.3);
`get_ticket`'s containment order, tracker tie-break and `..` segments were unspecified
(MED → §G9, §G6); the re-review signed off (SIGN-OFF: YES, residual ORCHID/benchmarking/192.88.99.0/24 added); IPv6 "global unicast" was not positive (6to4, Teredo, SIIT) and a
trailing dot passed the name rule (LOW → §G5.1, §G5.3); the `PRIVATE-TOKEN` header was
not marked sensitive (LOW → §G2). The reviewer verified against reqwest 0.12.28's source
that `.no_proxy()` disables env proxies and that a custom resolver sees every new connection.

**r2 claims re-verification** (`sensei-claims-verifier`): 0 FALSE among the † claims. MISLEADING, all fixed: G9's "the route has the caller's repository" (it has only the folder; resolved via `repo_key_for_repository`, and forge keys are no longer upper-cased); C29's reason sites and D10's `history_through` column; C32's `:622`; the app copy list (missing `:26 token_required` and 6 more lines); the step-4 test count (3, not 2); the handler's provisional `board_ref`; the Backlog wording; `:443`; the precedence citation. FALSE citation fixed: W10 `:510-512` → `:514-524`, and the parent `:79` → `:80` and `:389` → `:388`. **New: an existing `is_public_ip` (`lib_indexer.rs:250`) had been left out, so §G5.3 would have duplicated it. It is now extracted and shared (DRY).**

**Corrected in the parent's own text** (not this spec's error, owned there): the
iteration `state` type, and `board_ref` "names which" source. These are listed in
§G10 Docs, to be amended in build step 6.
