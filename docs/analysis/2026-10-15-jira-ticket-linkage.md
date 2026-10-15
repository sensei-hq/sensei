---
name: Jira ticket linkage — local slice
description: Stage-1 analysis — connect Jira read-only, link local sessions to tickets, report active hours + tokens per ticket
date: 2026-10-15
stage: analysis
next: /sensei:design
---

# Jira ticket linkage — local slice (analysis)

**Ask (verbatim):** *"Connect Jira and get sprints and tickets; local sessions need
to detect the ticket id and link it; then get metrics like hours spent on a ticket
and total AI tokens spent for a ticket."*

**Restated from what exists:** this is the **local-daemon cut of T3's T-1a (Jira
read-only adapter) + T-1d (session↔task linkage)**, plus a per-ticket *insight* read.
T3's gates, the target ladder, Slack hooks, ADO, and the Dōjō rollup are **not** in
this slice.

## 1 · What was read (grounding)

| Source | What it says |
|---|---|
| [`design/integrations-and-gating-requirements.md`](../design/integrations-and-gating-requirements.md) §2.1–2.6, §6.2, §7 | Jira is the sprint owner; sync is **read-only** (decision 2026-10-14, no export); a session links when **branch, commit or prompt** carries the key; multi-key session counts toward each; unlinked count toward nothing; roadmap items T-1a / T-1d |
| [`prd/modules/04-team/03-dojo-project-planning`](../prd/modules/04-team/03-dojo-project-planning/README.md) | Same invariants as "Lead"; **"tracker adapters: not present"**; **"no tracker tables yet"**; hours/tokens are *insights* — never gates, and never per-author gates |
| `roadmap/06-gaps.md` | T-1 (adapters mockup-only, P1); **F-3 Dōjō sync never worked (P1)** |
| `database/ddl/table/activity/sessions.ddl` | `duration` = gap-aware **active** time; tokens split `tokens_in` (folded total) / `tokens_fresh` / `cache_read` / `cache_write` / `tokens_out` / `tokens_reasoning`; `metered_cost`; `task` text; **no branch column**; `repo_key`, `project_id` |
| `activity/transcript_turns.ddl` | Per-turn `git_branch`, `user_text`, per-turn tokens; `session_id` is the assistant's text id (joins `sessions.client_session_id`) |
| `activity/task_sessions.ddl` | Existing "task" table = **checkpoint boundaries inside a session** (`task_description`, `task_type`), not tracker tickets |
| `sensei/commits.ddl` | `(repository_id, sha, authored_at, author_email)` — **no commit message, no session link** |
| `crates/senseid/src/tasks/processors/metadata/external_links.rs` | `classify_url` already tags `atlassian.net/browse` URLs as `kind="jira"` — only for repo docs, not sessions |
| `crates/senseid/src/gateway_keys/{mod,keyring_store}.rs` | Cross-platform keyring store (Linux fixed in `50d59f92`) — candidate home for a Jira API token |
| `crates/senseid/src/transcript/mod.rs:112` | Transcript adapters already lift `gitBranch` per turn (Claude, Cursor; Zed has none) |
| `crates/senseid/Cargo.toml` | `reqwest` (rustls, json) already a dependency |

### Live data (this machine, local brew pg17 :5432, 2026-10-15)

| Table | Rows |
|---|---|
| `activity.assistant_events` | 330 (27 `UserPromptSubmit`, 17 `SessionStart`) |
| `activity.sessions` | **0** |
| `activity.transcript_turns` | **0** |
| `activity.task_sessions` | 0 |
| `sensei.commits` | 1,332 |
| `~/.claude/projects/**/*.jsonl` | 107 transcripts on disk, `gitBranch` present (main 9,290 · develop 2,943 · feature/linux_support 2,570 …) |

## 2 · Surprises (the assumptions the ask carried)

1. **It's already designed.** T3 + the integrations requirements doc specify the
   adapter, the linkage sources and the unlinked rule. The work is a *slice of an
   existing spec*, not a new feature — design must cite T3, not re-invent it.
2. **There is nothing to link on this machine.** Hooks capture events (330) but
   **zero sessions and zero transcript turns** are synthesized/ingested on the
   `feature/linux_support` branch. Every metric in this ask reads
   `activity.sessions`; it is currently empty. → **Prerequisite P0** (§5).
3. **Branch isn't a session attribute.** `sessions` has no branch; it lives per
   turn in `transcript_turns.git_branch` (and only for transcript-ingested sources
   — Zed carries none). A session can span branches.
4. **Commit linkage needs new capture.** `sensei.commits` stores no message and no
   session link. "Commit carries the key" is not readable today.
5. **A ticket-key regex is mostly noise.** `[A-Z][A-Z0-9]+-\d+` over the 107 real
   transcripts: top hits are `UTF-8` (91), `AES-256` (51), `AD-6`, `NET-1`,
   `GPT-6`, `BSD-3`, `RFC-3339`, `CVE-2026`, `RUSTSEC-2023`; the plausible keys
   (`AUTH-420`, `PROJ-123`, `ABC-123`) are **examples quoted from docs**, not work.
   ⇒ matching must be against **issue keys that exist in the synced Jira set**, and
   a *prompt mention* is weaker evidence than a branch.
6. **`tokens_in` overstates cost ~8×.** It is the folded total incl. cache reads.
   "Tokens per ticket" must report fresh / cache-read / cache-write / out
   separately (and `metered_cost` where known), never `tokens_in` alone.
7. **"Task" is taken.** `activity.task_sessions` means checkpoint boundaries. The
   new concept must be named *ticket / tracker issue* to avoid colliding.
8. **The user's own branches carry no keys** (`develop`, `feature/linux_support`).
   For a team that doesn't name branches by ticket, prompt is the only source —
   and the noisiest one.

9. **T3 already names the tables and the API** (design §5.1/§5.2):
   `sensei.trackers`, `sensei.tracker_links`, `sensei.tracker_tasks`,
   `sensei.sprints`, `sensei.session_task_links`; `/api/projects/{id}/trackers…`,
   `/api/projects/{id}/sessions/unlinked`. The slice adopts them rather than
   inventing a parallel schema (caught by the depth review, not the first read).
10. **T3 §2.5 says linkage is "a read of existing session metadata — no new
    capture".** Branch (`transcript_turns.git_branch`) and prompt
    (`transcript_turns.user_text`) satisfy that; **commit does not** — it would
    need `commits.message` + a session↔commit rule. That is why commit is deferred
    (D4), not merely for convenience.

## 3 · Requirement

**Who** (personas per [`prd/03-personas.md`](../prd/03-personas.md)): the **solo
and team developer** (personas 1, 2) who want to know *what a ticket cost in AI
time and tokens* on their own machine. The **manager** (6) and **client lead** (5)
are served later via the Dōjō rollup (F-3) — this slice must not pre-empt their
constraints (§3.3). It is an **insight**: never a gate, never keyed by person
(T3 invariant).

### 3.1 Decisions

| # | Question | Decision | By |
|---|---|---|---|
| D1 | Where does it live? | **Local daemon first.** Per-developer Jira connection (own API token in the existing `gateway_keys` keyring), linkage + metrics in `senseid`. Dōjō rollup deferred behind F-3. | user 2026-10-15 |
| D2 | Session carrying ≥2 keys | **Full attribution to each ticket**, the link marked `shared` (= the session links to ≥2 tickets). Any total above ticket grain (sprint, project) **de-duplicates by session id**. | user 2026-10-15 |
| D3 | "Hours" | **Active AI-session time** = Σ `sessions.duration` (gap-aware). Every surface labels it "active AI-session time" — never "hours worked". | user 2026-10-15 |
| D4 | Link sources (A1) | **v1 = `branch` + `prompt`.** `commit` is deferred to v2 (needs new capture — breaks T3 §2.5). The `source` enum **reserves** `commit` now so v2 is additive. Deviation from T3's "all three on by default" recorded in `backlog.md`. | analysis default |
| D5 | Prompt precision (A2) | A **synced** key appearing in **any user prompt** of the session links it, `source=prompt`, with `mentions` (count) stored on the link. No ≥N threshold in v1: provenance + count let the read separate strong from weak. **Accepted false-positive class:** a real key of the bound project quoted in passing. Prompt links count toward hours/tokens like branch links. | analysis default |
| D6 | Branch per session (A3) | Derived **at link time** from `transcript_turns.git_branch` (distinct values over the session's turns), joined via `sessions.client_session_id`. No `SessionStart` capture in v1. A session with **no transcript turns has no branch link** — honest absence, never guessed. | analysis default |
| D7 | Which sessions can link (A4) | **Explicit repo binding required.** A connection is bound to ≥1 repo (`sensei.tracker_links`, by `repo_key`); only sessions whose `repo_key` is bound are link candidates. Sessions in unbound repos never link — this is the client-confidentiality guard (a client key can't match in an unrelated repo). | analysis default |
| D8 | Storage (A5) | **T3 §5.1 names**, `sensei` schema: `trackers` (connection, secret *ref* only), `tracker_links` (repo binding + sync cursor + status), `tracker_tasks` (read-only mirror, PK = Jira issue **id**, `key` updatable on rename, `removed_at` soft-delete), `sprints` (Jira sprint id, name, state, start/end), `session_task_links` (session_id, task_id, source, matched_key, mentions). No `assignee` column (D9). | analysis default |
| D9 | Person data | **No assignee/author in v1.** Not synced, not stored, no read groups by person. Removes the de-facto per-person hours figure (persona 2). Re-adding it is a Dōjō/F-3 decision. | analysis default (persona review) |
| D10 | Sprint membership | A session belongs to a sprint when `started_at ∈ [sprint.start, sprint.end)`. A straddling session counts once, in the sprint it started in. | analysis default |
| D11 | Sync | Manual (`POST …/sync`) + periodic default **every 30 min**. Status: `connected` (last sync ok), `stale` (last success older than 2× interval = 60 min), `error` (last attempt failed — message kept). Read-only: `GET` only. Uses Jira Cloud REST v3 (issues via JQL search, paginated) + Agile API (`/rest/agile/1.0/board/{id}/sprint`) — the board id is part of the connection. | analysis default |

### 3.2 Done looks like (observable)

1. **Connect:** `POST /api/projects/{id}/trackers/jira/connect` (and a CLI
   wrapper) takes site URL, email, API token, Jira project key, board id, and
   ≥1 bound repo. Token goes to the keyring; the `trackers` row holds only the
   ref. A bad token → 4xx with an auth error the CLI prints, **never** an empty
   board. Token never appears in DB, logs or API responses.
2. **Sync is read-only:** the Jira client exposes **only GET** (no other verb is
   constructible — enforced by the client type, unit-tested). Sync pulls sprints
   + issues (key, summary, type, status, sprint id) for the bound project,
   paginating to the end. Re-sync is idempotent by Jira issue id; a renamed key
   updates `key` and existing links survive; an issue gone from Jira gets
   `removed_at`, links kept. 429 → honours `Retry-After`, then `error` status if
   exhausted; a **partial** sync is recorded as `error`, never `connected`.
3. **Link:** each candidate session (D7) gets 0..n `session_task_links` with
   `source ∈ {branch, prompt}` and `matched_key`. A key links **only if it exists
   in `tracker_tasks`** — `UTF-8`, `AES-256`, `CVE-2026` never link (test uses the
   real noise list from §2.5). Linking re-runs after each sync, so newly-synced
   keys resolve on old sessions.
4. **Metrics** (`GET /api/projects/{id}/tracker-tasks/{key}/usage`,
   `GET /api/projects/{id}/sprints/{sid}/usage`): per ticket → linked sessions
   (split by source), `shared` count, active AI-session time, tokens fresh /
   cache-read / cache-write / out, `metered_cost` when known. Per sprint → the
   same de-duplicated by session (D2), plus **unlinked** candidate-session count
   (`GET /api/projects/{id}/sessions/unlinked` lists them).
5. **Honest unknowns:** sessions whose `duration` or token fields are NULL are
   counted as `sessions_missing_duration` / `sessions_missing_tokens` — not
   summed as 0. A DB failure is a 5xx, never `[]`/`0`.
6. **Verified on real data:** on this machine, after P0, a labelled sample of
   **≥10 real sessions** with known tickets: **0 links to non-existent keys**,
   precision reported per source, and one ticket's numbers equal a hand-written
   SQL sum.

### 3.3 Constraints (from `CLAUDE.md`)

- **No fabricated data** on failure paths — sync/read failures surface as error
  state or 5xx; NULL ≠ 0 (criterion 5).
- **DRY / shared code:** reuse `gateway_keys` for the secret, `reqwest` client
  conventions, `sessions.repo_key` for repo identity; DDL under
  `database/ddl/table/sensei/`.
- **Deviations** from T3 (D4 commit deferred, D9 no assignee, local-first D1)
  recorded in `docs/backlog.md`, not code comments.
- **Nothing leaves the machine** in this slice. Before any F-3 upload, issue
  summaries are treated as client-confidential (persona 5) — that is F-3's
  design problem, flagged here so the tables don't assume upload.

### 3.4 Out of scope (explicit)

- Writing anything back to Jira (decision 2026-10-14).
- Dōjō: tracker config UI, team rollup, session upload (F-3).
- ADO / GitHub Issues; one-sprint-owner switching; task-type mapping (T-1b/T-1c)
  — Jira is the only tracker, so it owns sprints by construction; `type` is stored
  raw, unmapped.
- Commit-message linkage (D4 → v2).
- Targets, ladder, gates, close-with-note, Slack (T-1e/T-1f).
- Jira OAuth, Jira Server/Data Center (Cloud + API token only).
- Human time-tracking (Jira worklogs); any per-person figure.
- A desktop-app screen — the read API + CLI is the contract for v1.

## 4 · Remaining ambiguities (safe to defer)

- Sprint custom-field quirks when an issue sat in several sprints — v1 records the
  issue's **current** sprint from the Agile API; history is v2.
- Whether to auto-suggest the Jira site from `classify_url` hits in repo docs —
  nicety, not required.

## 5 · Prerequisite — P0 (hard gate)

`activity.sessions` = 0 and `activity.transcript_turns` = 0 on this machine,
with 330 `assistant_events` captured and 107 transcripts unread. **Build of this
slice is blocked** until a separate slice on `feature/linux_support` makes
session synthesis + transcript ingest work on Linux.
**P0 done check:**

```sql
select (select count(*) from activity.sessions) > 0
   and (select count(*) from activity.transcript_turns where git_branch is not null) > 0;
```

Design may proceed in parallel; build may not.

## 6 · Risks

- **Precision over recall.** A wrong link silently inflates a ticket; `source` +
  `mentions` on every link and D7's binding are the guards.
- **Double counting** — any surface above ticket grain must de-duplicate by
  session (D2); a test asserts sprint total ≤ Σ distinct sessions.
- **Branch coverage** depends on transcript ingest; Zed sessions carry no branch
  (D6) and rely on prompt only.
