---
name: Tracker delivery metrics
status: draft (round 2, after claims-verifier and spec review, 2026-11-04)
date: 2026-11-04
analysis: none (brainstorm 2026-11-04, decisions B1–B15 below)
parent:
  - docs/spec/daemon/tracker-sdlc-metrics.md (S1, S5, S6, S8)
  - docs/spec/daemon/tracker-jira-linkage.md (R7)
references:
  - crates/senseid/src/tasks/dojo_sync.rs (the push, consent and #240 rules this reuses)
  - database/ddl/function/dojo/can_read_repository_metric.ddl (who reads a person's numbers)
  - database/ddl/function/dojo/workspace_metric_series.ddl (#307 D8: pooled rows for everyone else)
---

# Tracker delivery metrics

## Purpose

A developer opens **Delivery Metrics** in the workspace window and sees how each sprint went:
commitment kept, scope that moved, carry-over, velocity, and the AI time and cost behind it.
A team lead opens the same view in the Dōjō and sees the **whole team's** figures for each
sprint and ticket, merged from every developer's daemon, with ticket titles. Each daemon knows
only its own sessions; the Dōjō is the one place work on a shared board comes together.

## Decisions carried in (user, 2026-11-04)

| # | Decision |
|---|---|
| B1 | A new workspace section, **Delivery Metrics**. The existing Metrics section (計) is not touched. |
| B2 | Delivery Metrics shows **sprint-level** figures. Ticket figures stay on Tickets. |
| B3 | The Tickets lane header's "This sprint / This board" SDLC row (S8) **moves** to Delivery Metrics. Tickets keeps the sprint filter, the usage strip and each ticket's SDLC group. |
| B4 | Ticket and sprint figures **sync to the Dōjō**, which merges them across developers. |
| B5 | **Merged figures show in the Dōjō only.** The app never pulls team figures back. |
| B6 | The Dōjō receives **ticket titles** and every non-removed ticket of the board, not only totals. |
| B7 | Branch `feature/delivery_metrics`, first cut from `feature/sdlc_metrics` (it builds on S5, S6 and S8), rebased onto `develop` once that branch merged (#350, 2026-11-04). |
| B8 | A local workspace whose **allowed** repositories (`plan.allowed`) resolve to more than one Dōjō workspace sends its boards nowhere and logs `tracker_workspace_ambiguous` (fail closed). |
| B9 | Titles travel for **every** classification, `client` included. A title is the only ticket free text that travels: never a description, comment, assignee, attachment or issue URL. |
| B10 | `effort_cost` and `rework_cost` travel. The hourly rate itself never does. Jira write-back is unchanged (S9). |
| B11 | **The integration stays local.** The Dōjō keeps no tracker connection: no base URL, account, secret, field mapping, write-back setting or sync state, and no board can be connected from it. |
| B12 | B9 **overrides** the rule on `sensei.tracker_tasks.summary` ("Client-confidential: never leaves the machine"). The column comment is updated in this slice. |
| B13 | **Replace and withdraw.** Each push replaces everything that person reported for the board. Disconnecting the board, or the board losing consent (no allowed repository, or ambiguous), withdraws that person's data. Items removed in Jira are marked removed. Signing out keeps the data, shown as stale. |
| B14 | **Each person's cost at their own rate, summed**, labelled so. A contributor with no rate leaves the sum partial. `expected_minutes` and `time_variance_minutes` (local calibration, S1 `minutes_per_point`) never travel. |
| B15 | Board and sprint **names** travel, under the same decision as titles (B9). |

## Overrides of parents

- **S8 (tracker-sdlc-metrics):** the lane header's sprint/project row leaves Tickets (B3) and
  renders in Delivery Metrics (D2). `docs/spec/screen/workspace-tickets.md:58` drops the row and
  its gates in this slice.
- **Parent "Out of scope: Dōjō rollups":** in scope here (B4).
- **`tracker_tasks.summary` comment:** B12.

## Data invariants

- A board's figures exist locally only after a successful tracker sync; sprint facts only once
  the history task has read every ticket the sprint window needs (`history_pending` otherwise,
  S5). Nothing is pushed for a board whose tracker has `last_success_at` null.
- A session counts toward pushed effort only if its repository is in the persona's
  `plan.allowed` **and** exclusive to that persona (`exclusive_repos`, #240). Sessions with no
  repository, or from any other repository, are held back and counted. **There is no
  session→person column** (`activity.sessions` has none); repository exclusivity is the proof.
- In the Dōjō, one board copy per `(workspace, board_key)`, and at most one effort row per
  `(board, ticket | sprint | project, principal)`.

## Resolved design

### D1 · What merges, and how

| Family | Source | Same on every daemon? | Merge in the Dōjō |
|---|---|---|---|
| **Board facts**: board name; sprint details and removal; `SprintFacts`, `SprintSdlc`; `predictability`; each ticket's key, title, status category, points, sprint membership, `issue_created_at`, removal | Jira, via sync and the history task | Yes, once each daemon has read the same history | **One copy per board, replaced whole** (D4 order). Never summed. |
| **Sprint effort**: the sprint's `linked_sprint` `Usage` partition | This persona's provable sessions in the sprint window linked to the sprint's tickets | No | **Sum per field** across people, each value with its `_n`. |
| **Project effort**: tokens and prompts over the distinct union of the board's tickets' sessions, `sessions_without_tokens` | This persona's provable sessions | No | **Sum**, then recompute the per-ticket ratios (below). |
| **Ticket effort**: the `TicketSdlc` numerators below | This persona's provable sessions linked to the ticket | No | **Per field**, below. |

`linked_other` and `unlinked` are linkage diagnostics for the local Tickets strip; they do not
travel. The sprint's effort is its own row, never a sum of its tickets (R7 counts a session
linked to two tickets once).

**Usage fields (sprint effort).** `sessions`, `shared`, and each of `active_secs`,
`tokens_fresh`, `cache_read`, `cache_write`, `tokens_out`, `tokens_reasoning`,
`metered_cost`, `claude_secs`, `user_secs`, `wall_secs`, `api_cost` with its `_n`, plus
`api_cost_partial`, sum. `by_source` sums per source. `first_at` is the min, `last_at` the max.

**Ticket effort on the wire.** Each person's row carries **numerators and counts, never a
ratio**. New numerators the daemon already has but `TicketSdlc` does not expose are marked ✚.

| Wire field | Merge | Local source |
|---|---|---|
| `sessions`, `api_cost_n`, `api_cost_partial` | sum | the ticket's `Usage` |
| `worked_ms` | sum | union of slices (`worked_minutes`) |
| `prompts` | sum | |
| `ai_token_cost` | sum (known values; see partial) | |
| `compile_defects`, `test_defects`, `review_fixes`, `escaped_task_defects` | sum | |
| `build_ok` ✚, `build_fail` ✚ | sum | `count(BuildOk)`, `count(BuildFail)` |
| `runs_unknown`, `unclassified_failures`, `build_unrecovered` | sum | |
| `recovery_ms_sum` ✚, `recovery_n` ✚ | sum | the `gaps` vector |
| `on_hold_ms`, `on_hold_count`, `on_hold_by_reason` (per reason) | sum | |
| `execution_ms` ✚ | sum | this person's `submitted − started` window |
| `rework_ms`, `rework_cycles` | sum | |
| `effort_cost`, `rework_cost` | sum (B14) | |
| `work_started_at` | min | |
| `submitted_at`, `submitted_via` | max; `via` rides with the max | |

**Recomputed in the Dōjō from merged values** (the same formulas as `sdlc/ticket.rs`):

| Figure | Formula |
|---|---|
| `worked_minutes`, `on_hold_minutes`, `rework_minutes` | ms ÷ 60 000 |
| `review_defects` | `review_fixes + escaped_task_defects` |
| `total_defects` | `compile + test + review_defects` |
| `testing_efficiency` | `(compile + test) ÷ total × 100`; null when total = 0 |
| `review_efficiency` | `review_defects ÷ total × 100`; null when total = 0 |
| `build_runs` | `build_ok + build_fail` |
| `build_failure_rate` | `build_fail ÷ build_runs × 100`; null when runs = 0 |
| `mean_build_recovery_minutes` | `recovery_ms_sum ÷ recovery_n ÷ 60 000`; null when n = 0 |
| `on_hold_ratio` | `Σ on_hold_ms ÷ Σ execution_ms × 100` (person-time over person-time) |
| `post_review_rework_ratio` | `Σ rework_ms ÷ Σ worked_ms × 100` |
| `cost_per_prompt_cycle` | `Σ ai_token_cost ÷ Σ prompts`; null if prompts = 0 or cost partial |
| `execution_hours` | `max submitted_at − min work_started_at` |
| `lead_hours` | `max submitted_at − issue_created_at` (board fact); null if negative |

**Unknown is not zero.** A wire value may be `null` with a reason code. A sum is `null` (reason
`no_contributor_value`) when every contributor's value is `null`; otherwise it is the sum of the
known values, with `partial: k` naming how many contributors were unknown. A recomputed ratio is
`partial` when any input is, and `null` when its denominator is `null`. Min and max ignore
`null`. A person with **no** provable session on a ticket sends **no row** for it.

**Known limit.** An escaped defect whose tick is in one person's sessions and whose failure is
in another's is invisible to both, so `escaped_task_defects` can undercount across people. The
Dōjō view says so in the figure's explainer.

### D2 · App: the Delivery Metrics section

- **Nav:** `SECTIONS` in `workspace-sidebar-view.ts` gains
  `{ id: 'delivery', kanji: '納', label: 'Delivery Metrics' }`, directly after `tickets`. `納`
  joins `KANJI_ICONS` in `packages/glyphs/index.js` (`i-solar-delivery-bold-duotone` or the
  nearest Solar icon), so `glyph-vocabulary.spec.ts` passes.
- **Route:** `(workspace)/workspace/[id]/delivery/+page.svelte`, state in
  `delivery-state.svelte.ts`.
- **Data:** exactly four daemon reads, and the screen's API type has only these:
  `GET /api/workspaces/{id}/trackers` (`sdlc_project`), `GET …/trackers/{t}/sprints` (`sdlc`
  and the usage partitions), `GET …/trackers/{t}/tasks?sprint=` (the selected sprint's
  tickets) and `GET /api/reason-codes/sdlc_metrics`. Never `/api/dojo/*` (B5): the app
  reaches the Dōjō only through those daemon proxy routes. **No new daemon endpoint, no new
  storage.** It shows the values those endpoints return and never re-applies D1's #240 filter
  (that is the push's, D3).
- **Layout, per board:**
  - Project row, in two parts: "This board" with predictability ("12.5 pts", a board fact);
    "Your sessions" with tokens per ticket and prompts per ticket (project effort, D1).
  - Sprint table in `/sprints`' own order, never re-sorted (start date newest first, undated
    last, then `seq` desc; `tracker_usage.rs:194`): name, state, dates; the seven `SprintSdlc`
    figures (six percents, and velocity in points); then "Your sessions": `linked_sprint`'s
    active AI time, fresh tokens and API cost (`usageCells`: `api_cost`, "+" when partial,
    the app's established cost cell, not `metered_cost`).
  - A sprint with no window (`linked_sprint` null, `reason` `not_started` or `no_dates`) shows
    Tickets' words for that reason in its effort cells, never zeros. Those words move to the
    shared module with the formatters.
  - A selected sprint in full: its `SprintFacts` counts (committed, added, removed, completed
    committed, completed added, carried over; plus "n tickets with an unknown status" when
    `unknown_status > 0`, widening the app's `SprintFacts` type), and its tickets by key and
    title. Each links to `/workspace/{id}/tickets?board={tracker}&ticket={key}`.
  - No ticket SDLC figure appears here (B2).
- **Labelling:** every figure from your sessions is headed "Your sessions": the sprint
  effort columns and the project row's tokens and prompts per ticket. Board facts are not.
- **States:** no board: "This workspace has no sprint board connected." with the Integrations
  link. A failed read is the daemon-error state with Retry, never an empty table. A `null`
  shows its reason's `summary`. A real `0` shows as `0%`, `0 pts` or `0`, never a reason or
  "—".
- **Tickets:** `[data-sdlc-lane]` and `sdlcRow` are removed (B3). Tickets gains the deep link
  above: after load it opens that ticket, and Back names the board's lane. An unknown key
  shows the ticket error; an unknown board opens it from the first lane.
- **Surfaces that list sections** change with the nav: the sidebar spec
  (tickets → delivery → integrations), `app/e2e/tests/multi-window.spec.ts`'s nav list,
  `app/e2e/tests-inspect/console-errors-inspect.spec.ts`'s screen list,
  `workspace-tickets.md`'s ordering lines, and the route list in
  `docs/prd/modules/03-delivery/16-workspace-window/README.md`.

### D3 · The push

**Where it runs.** A new step in the `dojo_sync` tick, inside the per-persona push loop, after
the graph push (`push_planned`). It skips a tracker while that tracker's sync or history guard
is held, and runs nothing when the Dōjō is unreachable (the tick's existing transport
handling).

**Eligibility, per persona and tracker.** Take the local workspace's repositories that are in
the persona's `plan.allowed`. None → the board is ineligible (`tracker_not_shared`). Their
Dōjō tenants (`sensei.repositories.tenant_id`) differ → ineligible
(`tracker_workspace_ambiguous`, B8). Otherwise the request names the tenant and those
repositories' `repo_key`s; **the Dōjō resolves the workspace** from them
(`dojo.repositories_in_workspaces`), so a forge-unit mapping (`map_forge_unit`) is honoured.
Becoming ineligible after having pushed queues a withdrawal (below).

**The person** is stamped by the Dōjō from the caller's token, never from the body (as
`/v1/you/metrics`). Effort is computed over **provable sessions only** (Data invariants).
Held-back sessions are counted and logged per pass, as held-back metric rows are.

**Board identity.** `board_key = hex(sha256(kind + "\n" + base_url + "\n" + board_ref))`, with
`base_url` already canonical (`normalize_base_url`: `https://` + lowercase host). It is a
stable pseudonym, not anonymisation: names travel anyway (B15). Its purpose is that no URL or
account reaches the Dōjō.

**One request per board.** `POST /v1/you/delivery`, see D4 for the contract. Payload:

```text
{ v: 1, tenant_id, repo_keys: [text], board_key, kind, board_name,
  facts: {                                   // one-copy part
    history_at, pending_tickets: int,        // tickets whose history is unread
    predictability, predictability_reason,
    sprints: [{ external_id, seq, name, state, start_at, end_at, complete_at,
                removed: bool, facts: SprintFacts|null, sdlc: SprintSdlc }],
    tickets: [{ key, title, status_category, points, sprints: [external_id],
                issue_created_at, removed: bool }] },
  effort: {                                  // this person's part
    held_back_sessions: int,
    project: { tokens, tokens_n, prompts, sessions_without_tokens },
    sprints: [{ external_id, usage }],
    tickets: [{ key, ...D1 ticket wire fields }] } }
```

A ticket object has exactly those keys (B9). Limits: at most 5 000 tickets and 4 MB per
request; a bigger board is skipped and logged `delivery_board_too_large`.

**Re-push watermark.** `sensei.tracker_push_marks (tracker_id, persona_id, tenant_id,
part ∈ {facts, effort}, hash, pushed_at)`, primary key `(tracker_id, persona_id, tenant_id,
part)`, cascading on tracker and persona delete. `hash` is sha256 of the part's canonical
JSON (keys sorted), **excluding** any timestamp of the pass itself. A part is sent when its
hash differs; it is marked when the Dōjō answers `accepted` **or** `stale` for it. A rejected
part is not marked and is retried next pass, without blocking the other part or other boards
(the per-item marking `push_allowed` does today). A changed tenant changes the key, so nothing
is suppressed after re-filing.

**Withdrawal (B13).** `DELETE /v1/you/delivery/{board_key}?tenant_id=` removes everything the
caller reported for that board. Queued in `sensei.delivery_withdrawals (board_key, tenant_id,
persona_id, queued_at)`, written when a tracker is deleted (in the same transaction, before
the cascade), or when a pushed board becomes ineligible; sent next tick; deleted on 2xx or 404.
Signing out queues nothing.

### D4 · Dōjō: contract, storage and reads

**Contract** (`dojo/src/routes/v1/you/delivery/+server.ts`, house error shape `apiError`):

| Case | Status |
|---|---|
| Both parts applied or stale | 200 `{ facts: "accepted"|"stale", effort: "accepted" }` |
| Malformed body, unknown `v`, over a limit | 400 / 413 |
| No or bad token | 401 |
| A `repo_key` is not the caller's shared and elected repository (`my_repositories`) | 403 — the Dōjō re-checks consent, never trusts the daemon's |
| The repositories are in no workspace | 404 `workspace_not_filed` (nothing is created) |
| The repositories span workspaces | 409 `workspace_ambiguous` |

One request is one transaction: either both parts' outcomes hold or nothing changed.
`DELETE` answers 204, or 404 when there is nothing of the caller's.

**Tables** (schema `dojo`, written only by security-definer functions):

| Table | Key | Holds |
|---|---|---|
| `delivery_boards` | `id` uuid; unique `(workspace_id, board_key)` | `tenant_id`, kind, name, and the one copy's `reported_by`, `reported_at`, `history_at`, `pending_tickets`, `predictability` — **nothing else (B11)** |
| `delivery_sprints` | `(board_id, external_id)` | details, `removed`, `facts`, `sdlc` |
| `delivery_tickets` | `(board_id, key)` | title, status category, points, sprint ids, `issue_created_at`, `removed` |
| `delivery_effort` | `(board_id, item_kind, item_ref, principal_id)`; `item_kind ∈ {project, sprint, ticket}`, `item_ref` = `''` / sprint `external_id` / ticket key | `repository_ids uuid[]` (whose sessions), `values` jsonb (D1 wire fields), `reported_at` |

All cascade from `delivery_boards`, which cascades from `dojo.workspaces`.

**Which copy wins (facts).** The stored copy is replaced **whole** (board row, all its sprints
and tickets) by an incoming one that is greater in this order:
`(pending_tickets = 0, history_at, reported_at, principal_id)`. `reported_at` is the Dōjō's
clock. So a daemon mid-backfill never replaces a complete copy, and a copy's sprints and
tickets always come from the same report. A lesser copy answers `stale`.

**Effort.** The caller's rows for the board are replaced by the request's: rows absent from it
are deleted (B13).

**Reads** (security definer, each first checking `can_read_workspace`; the route answers 403
when it is false, as `getWorkspaceMetrics` does):

- `workspace_delivery_boards(p_workspace)` — boards with predictability and the merged project
  row.
- `workspace_delivery_sprints(p_workspace, p_board, p_limit, p_offset)` — in the daemon's
  order (start date desc, undated last, then `seq` desc), as D2; one-copy facts plus merged
  sprint effort.
- `workspace_delivery_tickets(p_workspace, p_board, p_sprint null|external_id, p_backlog bool,
  p_limit, p_offset)` — all, one sprint, or backlog; title, merged effort.

**Who sees a person's numbers.** An effort row is shown per person only to a caller for whom
`can_read_repository_metric(r, 'user', principal, tenant)` holds for **every** `r` in its
`repository_ids` (called, not copied): the person themself, a tenant admin, a maintainer on
those repositories, or the person's team lead. Every other reader of the workspace, viewers
included, gets the merged value only, with `contributors` (a count) and the latest
`reported_at` across them, never a per-contributor time. Removed members' rows still count in
merged values until withdrawn.

**Staleness.** A contributor whose last report is older than 7 days is shown as stale to those
who see per-person rows; others see "includes stale data" when any is.

### D5 · Dōjō: the Delivery view

`(app)/org/[slug]/workspaces/[id]/delivery`, linked from the workspace page. The same layout
as D2 with merged figures, effort headed "Team", "from N contributors · last report …", and the
explainers for D1's known limit and B14's rates. A selected sprint lists its tickets with key,
title and merged effort; the Dōjō has no Tickets screen, so a ticket row expands in place to
its merged figures (and per-person rows for those allowed). A `null` shows its reason's
summary. A failed read is an error state, never an empty table.

## Signals shown

| Element | Value | Meaning | Example |
|---|---|---|---|
| Commitment reliability | `SprintSdlc.commitment_reliability` | committed tickets done ÷ committed, % | 8 of 10 → 80% |
| Scope change | `scope_change` | (added + removed) ÷ committed, % | 3 added, 1 removed of 10 → 40% |
| Carry-over rate | `carry_over_rate` | committed not done and moved to the next sprint ÷ committed | 2 of 10 → 20% |
| Velocity | `velocity` | points completed | 21 pts |
| AI time (app) | `linked_sprint.active_secs` | **your** active time in the sprint on its tickets | 6h 10m |
| AI time (Dōjō) | Σ sprint effort `active_secs` | the team's, from N contributors | 31h · from 4 |
| API cost (app and Dōjō) | `api_cost` (Σ in the Dōjō), "+" when partial | list-price cost of the AI work | $41.20+ |
| Effort cost (Dōjō) | Σ `effort_cost` | each at the contributor's own rate (B14) | $412 · partial, 1 without a rate |
| Contributors | count of effort rows | who reported, never who didn't | from 3 · last report 2 h ago |

## Done gate

Layers: **app** = vitest spec in `app/` (root test script); **daemon** = `cargo test -p senseid`
with a stub Dōjō; **sql** = pgTAP or the dojo SQL test harness against `sensei_test`;
**dojo** = vitest in `dojo/`.

1. **Section (app).** `buildWorkspaceNav` lists `delivery` right after `tickets` and before
   `integrations`, with `納`; `glyph-vocabulary.spec.ts` passes; every existing `metrics` spec
   passes unchanged; the e2e nav list includes Delivery Metrics.
2. **App figures (app).** Given a mocked `/sprints` with two sprints, the page renders each
   sprint's seven figures (velocity as "N pts") and `linked_sprint` active time and API cost
   equal to the response (`api_cost` 1.24 with `metered_cost` 9 shows "$1.24"); a `null` with
   `history_pending` renders that reason's summary, not "0" or blank; a real `0` renders `0%` /
   `0 pts`; a `not_started` sprint's effort cells show "Not started…" and no "0"; no
   `TicketSdlc` field renders.
3. **Tickets (app).** No `[data-sdlc-lane]`; the sprint filter, usage strip and ticket SDLC
   group render. `?board=t-1&ticket=AUTH-2` opens AUTH-2 with "Back to …"; a Delivery ticket
   row's href is exactly that URL.
4. **Failure (app).** A 500 from `/sprints` renders the daemon-error state; Retry recovers.
5. **Only four reads (app).** Loading Delivery Metrics and selecting a sprint calls only the
   four D2 paths (recorded through the injected API); none starts with `/api/dojo/`.
6. **Not shared (daemon).** A workspace with no repository in `plan.allowed` posts nothing and
   logs `tracker_not_shared`.
7. **Ambiguous (daemon).** Allowed repositories with two tenant ids post nothing and log
   `tracker_workspace_ambiguous`.
8. **Shape (daemon).** For a fixture tracker (`jira`, `https://acme.atlassian.net`, board
   `42`), `board_key` equals the sha256 of `"jira\nhttps://acme.atlassian.net\n42"`; the body
   contains neither `acme.atlassian.net` nor any hourly rate nor `minutes_per_point`; each
   ticket object's keys are exactly D3's.
9. **#240 (daemon).** Sessions from a repository another persona also elected, and sessions
   with no repository, are absent from effort and counted in `held_back_sessions`.
10. **Watermark (daemon).** A second pass with nothing changed posts nothing. A 400 on one
    board leaves that board unmarked and the other boards marked; the next pass re-sends only
    it.
11. **Withdrawal (daemon).** Deleting a pushed tracker queues and then sends one `DELETE`; so
    does the workspace losing its allowed repository.
12. **Contract (dojo).** Each row of D4's contract table has a test returning that status;
    a 403 for a `repo_key` not in the caller's shared and elected set.
13. **One copy (sql).** A copy with `pending_tickets > 0` never replaces one with 0, whatever
    `history_at`; between complete copies the later `history_at` wins, in either arrival order;
    a replaced copy's tickets absent from the winner are gone.
14. **Merge (sql).** Fixture: A and B on `T-1`, A alone on `T-2`.
    `workspace_delivery_tickets` returns for `T-1` `worked_minutes` = A+B, `work_started_at` =
    min, `build_failure_rate` = (fA+fB)÷(runsA+runsB)×100, `mean_build_recovery_minutes` from
    summed sums and counts, `effort_cost` = A+B; `contributors` 2 and 1.
15. **Unknown (sql).** A sends `ai_token_cost` null, B sends 3.0 → 3.0 with `partial: 1`;
    `cost_per_prompt_cycle` null.
16. **Replace (sql).** A pushes `T-1`, `T-2`, then pushes `T-1` only → A's `T-2` row is gone.
17. **Visibility (sql).** Person A reads their own rows; a tenant admin reads A's; a plain
    workspace member and a viewer read merged values with no `principal_id` and no
    per-contributor time; `can_read_workspace` false → the route answers 403.
18. **View (dojo).** With fixture 14 loaded, the Delivery view shows `T-1`'s title,
    "from 2 contributors", and an error state on a failed read.

## Wrong gate

- The Metrics section changes, or Delivery Metrics reads `workspace_metrics` or any
  `/api/dojo/*` route.
- The app and the Dōjō list one board's sprints in different orders.
- App effort columns are not labelled as yours.
- A sprint's AI usage in the Dōjō is the sum of its tickets' usage.
- A board fact is summed, or a ratio is averaged instead of recomputed.
- An unknown value merges as 0, or a merged figure hides that it is partial.
- A mid-backfill copy replaces a complete one, or a board's sprints and tickets come from
  different reports.
- `base_url`, an account, an issue URL, a description, comment or assignee, the hourly rate or
  `minutes_per_point` reaches the Dōjō, or `expected_minutes` / `time_variance_minutes` shows
  there.
- The Dōjō stores a tracker connection setting, or a board can be connected from it.
- A session not provably the persona's (#240) reaches effort.
- The Dōjō trusts the daemon's consent without re-checking `repo_keys`.
- Board data leaves for an ineligible workspace, or stays after disconnect or consent loss.
- A rejected part is marked, or one rejected board blocks the others.
- A reader without `can_read_repository_metric` on every source repository sees a person's row
  or a per-contributor time.
- A failed read renders as an empty table, in either app; the view claims a total is complete.

## Claims (round 2: independent claims-verifier 2026-11-04, then corrected)

| # | claim | evidence | verdict |
|---|---|---|---|
| C1 | Metrics section exists, id `metrics`, 計 | `workspace-sidebar-view.ts:20` | CONFIRMED |
| C2 | Sprint and project SDLC figures are computed at read, never stored | `sprint_sdlc` / `project_sdlc` callers only `trackers.rs:80,714` | CONFIRMED |
| C3 | `sensei.sprints` holds provider details only | `sprints.ddl` | CONFIRMED |
| C4 | `SprintFacts` / `SprintSdlc` are board-derived only | `sprint_replay.rs:46-63, 225-236` | CONFIRMED |
| C5 | Sprint usage is grouped distinct sessions, not a ticket sum | `usage.rs:146-197`; `tickets-state.svelte.ts:309` | CONFIRMED |
| C6 | `/sprints` returns `sdlc` and partitions `linked_sprint`, `linked_other`, `unlinked` (no single `usage`) | `trackers.rs:700-745` | CONFIRMED (corrected) |
| C7 | `/trackers` carries `sdlc_project`; `tokens_per_ticket`, `prompts_per_ticket` are session-derived | `trackers.rs:80,108`; `sprint_replay.rs:340-348` | CONFIRMED (corrected) |
| C8 | Lane row is `data-sdlc-lane` from `sdlcRow` | `TicketsScreen.svelte:167-169` | CONFIRMED |
| C9 | No model or network call in the figures | rg over `sdlc/`, `tracker_history.rs`, `usage.rs` | CONFIRMED |
| C10 | `sensei.trackers` unique `(kind, base_url, board_ref)`; `base_url` canonical | `trackers.ddl:33`; `tracker/http.rs:22` | CONFIRMED |
| C11 | The sync marks every item the Dōjō did not reject (not all-or-nothing); graph push follows metrics | `dojo_sync.rs:1006-1028, 651-653`; test `a_partial_acceptance_marks_only_what_was_not_refused` | CONFIRMED (corrected) |
| C12 | The Dōjō stamps the person from the session | `dojo_sync.rs:44-51`; `metrics-ingest.ts:68` | CONFIRMED |
| C13 | Workspaces are `(tenant_id, slug)`, created by `file_repository_workspace` **and** `map_forge_unit` | `workspaces.ddl:13`; `map_forge_unit.ddl:105` | CONFIRMED (corrected) |
| C14 | `can_read_workspace` admits viewers | `can_read_workspace.ddl:19` | CONFIRMED (new) |
| C15 | `can_read_repository_metric` admits self, tenant admin, maintainer+, team lead for user rows | `can_read_repository_metric.ddl` | CONFIRMED (new) |
| C16 | No session→person column; #240 proves by repository exclusivity | `activity/sessions.ddl:2-39`; `sync.rs:112-115`; `dojo_sync.rs:851-875` | CONFIRMED (new) |
| C17 | `expected_minutes` uses the local `minutes_per_point` | `ticket.rs:181`; `sdlc/settings.rs:1-10` | CONFIRMED (new) |
| C18 | Build failure count, recovery sum/count and execution window are not `TicketSdlc` fields but exist in `from_signals` | `ticket.rs:297-326` | CONFIRMED (new) |
| C19 | `history_through` is per ticket only; trackers have `history_at` | `tracker_tasks.ddl:26`; `trackers.ddl:30` | CONFIRMED (new) |
| C20 | `tracker_tasks.summary` is documented as never leaving the machine | `tracker_tasks.ddl:44-45` | CONFIRMED (B12 overrides) |
| C21 | `納` is not in `KANJI_ICONS` | `packages/glyphs/index.js` | CONFIRMED |
| C22 | The Dōjō workspace route has one page, no sub-routes | `+page.svelte`, `+page.ts`, `metrics-load.spec.ts` | CONFIRMED (corrected) |
| C23 | `GET /api/reason-codes/{domain}` serves `sdlc_metrics` | `routes.rs:491` | CONFIRMED |
| — | Two installs produce the same `board_ref` for one board | not checked | **verify at build start** |
