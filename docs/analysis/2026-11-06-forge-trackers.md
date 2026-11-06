---
name: Forge trackers — GitLab (incl. self-managed) and GitHub
description: Analysis — add GitLab (gitlab.com + self-managed, Free/CE up) and github.com as tracker adapters on the daemon path, milestones as sprints
date: 2026-11-06
stage: analysis
depends_on:
  - spec/daemon/tracker-jira-linkage.md
  - spec/tracker-levels.md
  - spec/daemon/tracker-metrics-writeback.md
next: /sensei:design
---

# Forge trackers: GitLab and GitHub (analysis)

**Ask (verbatim):** *"lets implment self hosted gitlab and github.com as trackers"*

**Restated from what exists:** the daemon has one tracker adapter, Jira
(`crates/senseid/src/tracker/jira.rs`), behind the `TrackerAdapter` trait
(`tracker/mod.rs:443`). The Jira spec already describes the GitLab seam ("Future
adapters", `tracker-jira-linkage.md:670`). This slice adds two adapters, **GitLab**
(gitlab.com and self-managed) and **GitHub** (github.com), on the same daemon path.

## 1 · What was read (grounding)

| Source | What it says |
|---|---|
| `tracker/mod.rs:443` `TrackerAdapter` | One trait carries all provider knowledge: `connect_params`, `auth`, `owner`, `validate`, `fetch_snapshot`, `match_links`, `unknown_keys`, `fetch_tasks`, `board_terms`, `changelog`, and the write-back methods (`find/create/update_comment`, `check_fields`, `write_fields`, …). The coordinator owns every DB write. |
| `tracker/mod.rs` `TrackerKind` | One variant, `Jira`. The DDL enum `database/ddl/enum/sensei/tracker_kind.ddl` matches it. A test pins Rust enum = DDL enum. |
| `tracker/jira.rs:13-42` | The host rule is a predicate passed to the shared `normalize_base_url(raw, is_jira_cloud_host)`. Jira accepts `*.atlassian.net` / `*.jira.com` only. |
| `tracker-jira-linkage.md:670` "Future adapters" | GitLab concerns are listed: iterations vs milestones, `#12` scoped by the session's `repo_key` (`tracker_tasks.scope` = `normalize_repo_key(project web URL)`), a self-managed host allow-rule (SSRF review), PAT/OAuth, `opened/closed` states. |
| `tracker/sprint.rs:81` | `no_sprints_means_backlog`: a ticket in no sprint has `current_sprint_id = NULL`. A board with zero sprints is already a valid state. |
| `spec/tracker-levels.md` (D40) | A tracker serves a level (`epic / sprint / task`). Several trackers per workspace are allowed. |
| `decisions.md` D49 | Org workspaces get dōjō-owned tracker connections (Wave C, needs #310). **The daemon path stays for solo/personal use.** New org adapters go on the dōjō path. |
| `decisions.md` D71 | For GitHub Issues the daemon uses **the person's own forge sign-in**, read-only, until dōjō-owned connections exist. |
| `decisions.md` D50 | Write-back is opt-in per workspace and off by default. |
| `api/handlers/auth.rs:730` | The stored GitHub credential (`provider_token.<slot>`) carries the `repo` and `read:org` scopes. |
| Issue #310 (open) | The dōjō has no secret store yet: encryption at rest, never returned to a client, admin-only, audited. Not designed. |
| `decisions.md` D83 + CLAUDE.md hard rule | No fabricated values. A missing value is an error or honest-empty, never a plausible default. |

## 2 · Decisions taken in the brainstorm (user, 2026-11-06)

| # | Decision |
|---|---|
| FT1 | **Daemon path** for both adapters: tokens in the local Keychain, like Jira. Neither waits for #310. The dōjō-owned path (D49) is a later slice. |
| FT2 | **GitLab must work on Free/CE.** At connect, the adapter **detects the version** and **probes for iterations**. It uses iterations where they exist and **milestones** otherwise. |
| FT3 | **GitHub: milestones are sprints.** |
| FT4 | **No milestones → the board connects as `level = sprint` and shows "No sprints yet".** Tickets sit in backlog. The daemon never changes the level the user picked, and never invents sprints (D83). |
| FT5 | GitLab auth in v1 is a **PAT** (`read_api`). GitHub uses the person's forge sign-in (D71). |

## 3 · GitLab

### 3.1 Version and capability detection

| Probe | Endpoint | Notes |
|---|---|---|
| Version | `GET /api/v4/version` (all versions) or `GET /api/v4/metadata` (15.2+) | Needs an authenticated token. The result goes into `params.version`. |
| Iterations | `GET /groups/:id/iterations?per_page=1` | **200** → iterations are available. **403/404** → Free/CE (or the feature is off): use milestones. Any other failure (5xx, timeout) is a **connect error**, never "assume milestones", because a transient failure must not silently pick the wrong sprint source. |
| Tier | ~~`GET /api/v4/license`~~ | Rejected: it needs an admin token. The capability probe answers the question we actually have. |

The outcome is stored in `trackers.params` (no secrets), for example
`{"version":"16.11.2","sprint_source":"milestone"}`. **Each sync probes again**, so
an upgrade to Premium switches to iterations without a reconnect.

**Open (O1):** what happens when the sprint source changes from milestones to
iterations mid-life? The old milestone sprint rows no longer come back, so they get
`removed_at` the way any vanished sprint does. Is that acceptable, or should a
switch need the user to confirm it?

### 3.2 Sprint mapping

| GitLab | → `SprintRow` |
|---|---|
| Iteration `upcoming / current / closed` | `future / active / closed`. `seq` = iteration id. |
| Milestone `active` with `start_date ≤ today ≤ due_date` | `active` |
| Milestone `active`, start in the future | `future` |
| Milestone `active`, past `due_date` | `active` (an overrunning sprint stays active, as in Jira R7) |
| Milestone `closed` | `closed` |
| Milestone with **no dates** | **Open (O2).** Recommendation: keep it as a sprint with NULL `start_at`/`end_at` and its `active/closed` state, shown as "undated". Skipping it would drop its tickets from every sprint view without saying so. |

`board_ref` names the scope: a group path (group milestones/iterations) or a project
path (project milestones). **Open (O3):** must v1 support both group and project
milestones, or only group milestones?

### 3.3 Keys and links

- A key is `#12` per project, or `group/project#12` globally. A branch defaults to `12-title`.
- `tracker_tasks.key` = `group/project#12` and `scope` = `normalize_repo_key(project web URL)`, as the Jira spec already fixes.
- `match_links` resolves a bare `#12` or `12-title` only against the session's own `repo_key`. A session with NULL `repo_key` never resolves a bare key.

### 3.4 Self-managed host policy (SSRF)

Jira's host rule is a fixed predicate. GitLab needs **gitlab.com plus a host the
user confirms at connect**:

- The connect body carries the base URL. The UI shows the resolved host and the user confirms it.
- `https` only. A private, loopback or link-local address (after DNS resolution, so a DNS name pointing at `127.0.0.1` is caught) is **refused unless the user opts in**, because self-managed GitLab often lives on a LAN.
- The confirmed host is pinned for this tracker: redirects to another host are not followed.
- **Needs a security review** (`sensei-security-reviewer`) before build.

**Open (O4):** should a self-signed TLS certificate be supported (a custom CA path),
or is it out of v1?

## 4 · GitHub

### 4.1 Auth

The daemon already holds the person's GitHub credential with `repo` and `read:org`
(`auth.rs:730`). Reads use it, as D71 says. `repo` also grants issue writes, so the
**D50 write-back works on GitHub** once a workspace opts in. There is no second
token to collect.

### 4.2 Milestone scope across a project's repos

GitHub milestones belong to **one repository**. A sensei project spans several repos.

| Option | How it works | For | Against |
|---|---|---|---|
| **A · Merge by title** (recommended) | One tracker per project. Milestones with the same title across the project's repos are one sprint. | Matches the common GitHub convention ("v1.4" in every repo); one sprint lane per project | Dates can disagree across repos: take the earliest start and the latest due date, and flag the disagreement |
| B · One repo is the board | The user picks one repo whose milestones are the sprints. Issues from other repos join only if they carry a same-titled milestone. | Simple and unambiguous | Arbitrary for a multi-repo project |
| C · One tracker per repo | Each repo is its own sprint tracker (D40 allows several). | No merging logic | N sprint lanes per project; sprint rollups never combine |

**Open (O5):** confirm option A.

### 4.3 Keys, states, sprint mapping

- Keys: `owner/repo#12`, bare `#12` and `12-title` resolved by the session's `repo_key`, the same as GitLab (§3.3).
- Pull requests are issues in GitHub's REST API. **Exclude them** (drop items that carry `pull_request`) so a PR is never counted as a ticket.
- Milestone `open` + `due_on` → `active` or `future` (GitHub milestones have **no start date**). **Open (O6):** what does `start_at` mean? Recommendation: NULL. A sprint window then ends at `due_on` and has no start, so per-sprint rollups count by membership, not by date window. Inventing a start date would be D83 fabrication.
- `closed` → `closed`.

## 5 · Shared between the two forges

Both forges share the key grammar (`#12`, `path#12`, `12-title`), repo-scoped
resolution through `repo_key`, and a milestone-to-sprint mapping. Per the DRY rule
these go in **one shared module** (for example `tracker/forge.rs`). The two adapters
(`gitlab.rs`, `github.rs`) keep only endpoints, auth headers and response shapes.
This keeps the Jira-spec isolation gate (provider logic lives only in its adapter
file), with the shared part being provider-neutral.

## 6 · What changes (first cut, for the design)

| Layer | Change |
|---|---|
| DDL | `tracker_kind` enum gains `gitlab`, `github` (a new migration, then `make test-template`) |
| Rust | `TrackerKind::{Gitlab, Github}`; `tracker/forge.rs` (shared); `tracker/gitlab.rs`; `tracker/github.rs`; registered in `adapters()` |
| Transport | GitLab `PRIVATE-TOKEN` header; GitHub `Bearer`. Link-header pagination for both. |
| Host policy | A host-confirmation step for self-managed GitLab (§3.4) |
| App | Integrations: connect form per kind (GitLab: URL + confirm host + PAT; GitHub: pick the signed-in account + the project); "No sprints yet" empty state |
| CLI | `sensei tracker connect --kind gitlab|github` |
| Fixtures | Recorded, scrubbed responses for each endpoint: GitLab Free/CE (milestones), GitLab Premium (iterations), GitHub |

## 7 · Risks

| Risk | Mitigation |
|---|---|
| SSRF through a user-supplied GitLab host | §3.4 policy + security review before build |
| A transient probe failure picks the wrong sprint source | Only 200 / 403 / 404 decide. Anything else is a connect or sync error. |
| A PR counted as an issue (GitHub) | Filter `pull_request` items; a test with a mixed fixture |
| Bare `#12` linked to the wrong repo | Resolved only through the session's `repo_key`; a NULL `repo_key` never links |
| No live GitLab Premium instance to record against | **Human prerequisite:** a Premium/Ultimate group (trial) for the iteration fixture, and a Free/CE instance (a local docker `gitlab-ce` will do) |

## 8 · Order

1. **GitLab first.** Its seam is already specified, and it covers both the iteration and the milestone paths.
2. **GitHub second.** It reuses `forge.rs` and adds only its own endpoints and the cross-repo milestone merge.

Each ships as its own slice.

## 9 · Open questions

| # | Question | Recommendation |
|---|---|---|
| O1 | Sprint source switches from milestones to iterations mid-life | Accept: the old rows get `removed_at`, with a one-line notice on Integrations |
| O2 | GitLab milestone with no dates | Keep it as an undated sprint |
| O3 | GitLab group vs project milestones | Both; `board_ref` names which |
| O4 | Self-signed TLS on self-managed GitLab | Out of v1 |
| O5 | GitHub milestone scope across repos | Option A, merge by title |
| O6 | GitHub milestone has no start date | `start_at` NULL; rollups by membership |
