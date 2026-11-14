---
name: Comment write-back for GitLab and GitHub
description: Analysis — what it takes for the GitLab and GitHub adapters to write the W5 usage comment back to tickets, so every tracker kind gets at least the comment surface
date: 2026-11-13
stage: analysis
depends_on:
  - spec/daemon/tracker-metrics-writeback.md
  - spec/daemon/tracker-gitlab.md
  - spec/daemon/tracker-github.md
  - analysis/2026-11-06-forge-trackers.md
next: /sensei:design
---

# Comment write-back for GitLab and GitHub

## Problem

Write-back (D50) works for Jira only. GitLab and GitHub report
`capabilities() = {history: false, writeback: false}` and answer
409 `writeback_unsupported` (`tracker-gitlab.md` §G8, `tracker-github.md:341`), and
Integrations shows "Write-back: not available for … yet". A team on a forge tracker gets
no sensei figures on its tickets, while a team on Jira does. The user wants the **comment
surface** on every tracker kind, for consistency between projects (user, 2026-11-13).

**Scope decided (user, 2026-11-13):** GitLab and GitHub in **one slice**, comment surface
only. Mapped fields (W6/W7) stay Jira-only.

## 1 · What was read (grounding)

| Source | Finding |
|---|---|
| `tracker/mod.rs:170-177` `Capabilities` | One `writeback: bool` covers both surfaces. Guards read it, never the kind name. |
| `tracker/mod.rs:538-560` | `find_comment`, `create_comment`, `update_comment` and `issue_exists` take only `task_external_id` (plus owner / comment ref / report). |
| `tracker/mod.rs:230-233` `WriteMethod` | `Post`, `Put` only. |
| `tracker-metrics-writeback.md` W5 (:253-327) | `render(&Usage) -> Report { title, lines }` is kind-neutral; the adapter turns a `Report` into the provider's format. Adopt-before-create: the newest comment by `owner_ref` whose first text starts `Sensei · AI usage`. A 404 on update → `issue_exists` decides `not_found` vs `comment_removed`. |
| `tracker-sdlc-metrics.md` S9 (via writeback spec :11-14) | The comment surface is unchanged by S9: it carries **usage** figures only. The forges' lack of history (`unsupported_by_tracker`) does not touch it. |
| `tracker-gitlab.md:131-132` | GitLab `key` = `references.full`, path lowercased (`acme/platform#12`); `external_id` = the issue's global `id` (:130). |
| `tracker-github.md:270-272` | GitHub `key` = `<nameWithOwner>#<number>`, lowercased; `external_id` = `databaseId`. |
| `tracker-gitlab.md:66, :83` | The GitLab PAT is asked for `read_api`. |
| `tracker-github.md:220-225` | The recommended GitHub token is fine-grained, **read-only**: Projects: read, Issues: read, Metadata: read. A classic `repo` token can already write. |
| `tracker-github.md` §H2 | The GitHub GraphQL transport sends validated **queries** only; no mutations. |
| Both forge adapters | `owner()` already resolves `GET /user` → `Person(AccountRef(id))` / `NotAPerson`. |

## 2 · What carries over unchanged

- `writeback::render` and both W5 golden examples: the text content is identical on every kind.
- The pass: owner check (W1), hash guard (W4), lock and deadline, status codes, `comment_removed`
  stickiness, the toggle false → true reset (W8).
- `owner_ref` from `GET /user` on both forges.
- The write transport's resolver and `.no_proxy()` (`tracker-gitlab.md` D13): `write_transport`
  already builds `ReqwestRaw::new` (`C34`).

## 3 · What has to change

### 3.1 Split the capability

`Capabilities { history, writeback }` → `{ history, comment: bool, fields: bool }`
(names for the design). Jira `{true, true, true}`; GitLab and GitHub `{false, true, false}`.

- `PATCH writeback_enabled: true` checks `comment` (forges now allowed).
- `PUT /fields` checks `fields` → still 409 on a forge. Options for the code:
  **(a)** keep `writeback_unsupported` for both; **(b)** add `fields_unsupported`.
  Recommend (b): one code meaning two things makes the Integrations copy lie for one case.
- Surface selection (W6: fields when mappings exist, else comment) needs no change, since a
  forge can never have mappings.
- Integrations: the write-back toggle appears for forge trackers; the field-mapping section
  stays hidden behind `fields`.

### 3.2 The comment methods need the ticket's address, not its id

The forges' comment APIs are addressed by **project + number**, not by the global id stored in
`external_id`:

| Kind | List | Create | Update | Issue exists |
|---|---|---|---|---|
| GitLab | `GET /api/v4/projects/:path/issues/:iid/notes?sort=desc&order_by=created_at&per_page=50` | `POST …/notes {"body"}` | `PUT …/notes/:note_id {"body"}` | `GET /api/v4/projects/:path/issues/:iid` |
| GitHub | `GET /repos/:o/:r/issues/:n/comments?per_page=100` (oldest first; read the `Link: rel="last"` page) | `POST /repos/:o/:r/issues/:n/comments {"body"}` | `PATCH /repos/:o/:r/issues/comments/:id {"body"}` | `GET /repos/:o/:r/issues/:n` |

`:path` is the URL-encoded project path from `key` (`acme%2Fplatform`). Both are fully
derivable from the stored `key`, so **no schema change** is needed.

Options:
- **(A)** Change the four trait methods to take a `&TaskAddr { external_id, key }` (or the task
  row). Jira ignores `key`. One signature, every adapter.
- **(B)** Have forge adapters look the key up by external id. Rejected: the adapter owns no DB
  access (the coordinator owns every DB write/read, forge-trackers §1).

Recommend **(A)**. The W10 shape check moves with it: Jira checks `external_id`
(`^[0-9]{1,18}$`); forges check `key` with `forge::is_key_path` plus the `#<u64>` suffix,
and the comment ref as `^[0-9]{1,18}$` (both forges' note/comment ids are integers).

GraphQL `addComment` (by node id) is **not** an option: §H2 limits the GraphQL transport
to queries, and opening it to mutations would widen the read seam.

### 3.3 `WriteMethod::Patch`

GitHub edits a comment with `PATCH`. `WriteMethod` gains `Patch`, and `ReqwestRaw`'s
`RawWrite::send` maps it. Gate 13 (the only write seam is `writeback.rs`) is unchanged.

### 3.4 Rendering and adoption

- Both forges take **Markdown**. The forge render is: `**<title>**` then one `- <line>` per
  line. Report strings contain only digits, units, `·`, `—`, `$`, brackets and parentheses
  (W5 rules), but the renderer still escapes Markdown metacharacters, not trusting the
  content to stay that way.
- Adoption keys on a hidden marker, not on body text: the body starts with
  `<!-- sensei:writeback -->`. Both forges render HTML comments invisibly. Adopt = newest
  comment whose author id equals `owner_ref` **and** whose body starts with the marker.
  (Jira keeps its existing title-prefix rule; ADF has no hidden node.)
- GitLab: skip `system: true` notes when scanning.

### 3.5 Token scope

| Kind | Read today | Comment write needs |
|---|---|---|
| GitLab | `read_api` | `api` (GitLab has no narrower write scope for notes) |
| GitHub fine-grained | Projects: read, Issues: read, Metadata: read | Issues: **read and write** on each repo |
| GitHub classic | `repo`, `read:project` | already sufficient |

Options:
- **(i)** Require the write scope at connect. Every forge tracker then holds a write-capable
  token, even when write-back stays off (D50: off by default).
- **(ii)** Keep connect read-only; when the toggle goes on, say on the toggle which scope is
  needed; the pass reports `forbidden` per ticket when the scope is missing, and "Replace
  token" fixes it.
- **(iii)** As (ii), plus a probe at toggle time. Neither forge exposes a side-effect-free
  "can I comment here" check for a fine-grained token (GitLab exposes PAT scopes via
  `GET /personal_access_tokens/self`; GitHub fine-grained tokens expose no scope header), so
  the probe would only be exact for GitLab.

Recommend **(ii)**, with (iii)'s GitLab probe only if it is cheap. It keeps least privilege
for workspaces that never opt in, and `forbidden` is already an honest, sticky,
user-actionable status.

### 3.6 Notifications

Jira updates with `notifyUsers=false`. On both forges, **editing** a comment sends no
notification, and **creating** one notifies issue participants. So one notification per
ticket, on the first pass after opt-in. Accepted; the Integrations toggle copy says so.

### 3.7 Rate limits

GitHub's secondary rate limit answers 403 → `forbidden` today (`tracker-github.md:216-218`,
accepted for reads). For writes that would mark tickets `forbidden` (sticky) after a burst.
The pass writes one comment per assigned ticket per change, so a burst is unlikely, but the
design should decide whether a 403 with `retry-after` or `x-ratelimit-remaining: 0` maps to
`rate_limited` on the write transport.

## 4 · What changes (first cut, for the design)

| Area | Change |
|---|---|
| `tracker/mod.rs` | `Capabilities` split (§3.1); trait comment methods take a task address (§3.2); `WriteMethod::Patch` (§3.3) |
| `tracker/transport.rs` | `Patch` in `ReqwestRaw`'s write send |
| `tracker/gitlab.rs`, `tracker/github.rs` | the four methods, Markdown render, marker adoption, id/key shape checks; capabilities `{false, true, false}` |
| `tracker/jira.rs` | new signature, behaviour unchanged |
| `tracker/writeback.rs` | passes the task address; reads `comment` / `fields` capabilities |
| `api/handlers/trackers.rs` | PATCH guard on `comment`, PUT `/fields` guard on `fields` (+ `fields_unsupported` if 3.1 (b)) |
| App Integrations | toggle for forge trackers; scope note per kind; first-post notification note; "Write-back: not available" copy goes; field mapping hidden by `fields` |
| Docs | `tracker-metrics-writeback.md` Out of scope ("adapters other than Jira" → mapped fields only), W5 per-kind rows, W10 shapes; `tracker-gitlab.md` §G8 / Out of scope; `tracker-github.md` capabilities, token text, Out of scope; `decisions.md` new row; screen spec `workspace-integrations.md` |

## 5 · Risks

- **Lowercased keys.** Both `key`s are stored lowercased. GitHub's REST paths are
  case-insensitive. GitLab's project lookup by path is expected to be case-insensitive;
  **unverified** — probe X1.
- **Moved / transferred issues.** A GitLab or GitHub issue moved to another project keeps
  neither its key nor (on GitLab) its id. The next sync drops or re-keys the task; the old
  comment is left behind, as on Jira (deleting sensei comments is out of scope).
- **Fine-grained GitHub token reach.** Issues: write is granted per repo; a project spanning
  repos the token can't write gives `forbidden` on some tickets only. Honest, but the copy
  should say "per repository".
- **Self-managed GitLab versions.** The notes API is stable since long before the minimum
  version §3.1 of forge-trackers accepts; no capability detection needed.

## 6 · Probes for the design (X-list)

| # | Claim | How |
|---|---|---|
| X1 | GitLab resolves a lowercased URL-encoded project path for a mixed-case project | one GET against gitlab.com |
| X2 | An HTML comment as the first line of a note/comment body is preserved verbatim on read-back on both forges | create + list on a test issue |
| X3 | Editing a comment sends no notification on either forge | test issue with a watcher |
| X4 | GitHub fine-grained token with Issues: read only → `POST …/comments` answers 403 (not 404) | test token |

## 7 · Questions (all decided — user, 2026-11-13: recommendations accepted)

| # | Question | Decision |
|---|---|---|
| O1 | Write scope at connect, or only when write-back is turned on? | (ii): only on opt-in; `forbidden` + Replace token |
| O2 | PUT `/fields` on a forge: reuse `writeback_unsupported` or add `fields_unsupported`? | add `fields_unsupported` |
| O3 | GitHub 403 secondary rate limit on the write path: `forbidden` or `rate_limited`? | `rate_limited` when `retry-after` / `x-ratelimit-remaining: 0` is present |
