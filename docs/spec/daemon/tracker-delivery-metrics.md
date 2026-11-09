---
name: Tracker delivery metrics
status: built (F1–F6, #351–#356, 2026-11-05; amendments A1–A4); A5 designed 2026-11-09 (gate passed)
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
| B16 | **Cost and solo values are gated** (user, 2026-11-05, after the F3 pre-build audit: `effort_cost ÷ worked time` reveals a person's rate, and one contributor's "merged" value is that person's). Merged `effort_cost` and `rework_cost`, and every merged figure of an item with fewer than **2** contributors, show only to readers allowed per-person rows (D4 "Who sees"); every other reader sees "one contributor" and no cost. F3 serves the restricted view to every reader; F4 opens it to the allowed. A contributor counts only with a row of ≥ 1 session, and the dōjō validates every row's fields and types; but the daemon's #240 filter is the **only proof** a row's figures are that person's own — a member who forges a daemon push can still pose as a second contributor and difference the merge. B16 stops a reader, not a forger. |

## Overrides of parents

- **S8 (tracker-sdlc-metrics):** the lane header's sprint/project row leaves Tickets (B3) and
  renders in Delivery Metrics (D2). `docs/spec/screen/workspace-tickets.md:58` drops the row and
  its gates in this slice.
- **Dōjō rollups:** the parent left them unaddressed (it names no Dōjō side at all); they are
  this spec's (B4). *(Corrected at F6: an earlier draft cited a parent out-of-scope line that
  does not exist.)*
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
| **Board facts**: board name; sprint details and removal; `SprintFacts`, `SprintSdlc`; `predictability`; each ticket's key, title, status category, points, sprint membership, `issue_created_at`, removal | The tracker (Jira, GitLab or GitHub), via sync and, where the adapter reads it, the history task. A board without history reports 0 pending and history-built figures read `unsupported_by_tracker` (2026-11-09, `docs/analysis/2026-11-09-delivery-metrics-forge-trackers.md`) | Yes, once each daemon has read the same history | **One copy per board, replaced whole** (D4 order). Never summed. |
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

**Amendment A3 (F3 pre-build audit, 2026-11-05)** corrects the rules above where merging
per-person rows would not reproduce what one daemon holding every session computes:

- **Submission is the first.** `submitted_at` merges as the **min** over rows with `via =
  pr`, else the min over `push`; `via` rides with it. `execution_hours` = that − min
  `work_started_at`; `lead_hours` = that − `issue_created_at`.
- **Not applicable is not unknown.** Each null travels with its reason (keyed by the wire
  field). `no_activity` is sent as `0`. `not_started` / `not_submitted` take the person out of
  that field's population without making it partial. Only `not_derived`, `derivation_failed`,
  `not_priced` and `no_rate` are unknowns that count toward `partial`. When every
  contributor is unknown, the merged reason is theirs if they agree, else
  `no_contributor_value` (a new `sdlc_metrics` code).
- **Ratios pair their populations.** Each recomputed ratio sums numerator and denominator
  over the **same** contributors, those with both known: `on_hold_ratio` (on_hold_ms,
  execution_ms), `post_review_rework_ratio` (rework_ms, worked_ms), `testing_` and
  `review_efficiency`, `build_failure_rate`, `mean_build_recovery_minutes`. It is `partial`
  when a contributor was left out as unknown.
- **`cost_per_prompt_cycle`** is null `partial_cost` unless every contributor's
  `ai_token_cost` and `prompts` are known, Σ`api_cost_n` = Σ`sessions` and
  Σ`api_cost_partial` = 0 (the local rule, summed); `no_prompts` when Σ`prompts` = 0.
- **Person-time.** Merged `worked_minutes`, `rework_minutes` and `on_hold_minutes` are summed
  person-time, not a union of wall-clock slices; the view labels them so.
- **Known limits** (the view's explainer) widen: each person's work is read on its own, so
  build recovery pairs a failure with the next success in *that person's* sessions
  (`mean_build_recovery_minutes`, `build_unrecovered` can differ from one daemon's), an
  escape lost across people counts as a compile or test defect (total unchanged; the
  efficiencies shift), a submission needs that person's own first edit (a PR opened in
  another person's session without an edit is not seen), and rework counts each person's
  edits after their own submission.
- **One person, two installs.** Effort rows are keyed by person **and install** (`source`, the
  local persona's id, a random pseudonym): a push replaces only its own install's rows, the
  merge sums every install's rows, and `contributors` counts distinct people.
- **Project row.** Its sessions are those linked to **non-removed** tickets (the tickets that
  get rows). `tokens` is null (`no_usage`) when no session has tokens. The Dōjō's
  `tokens_per_ticket` = Σ`tokens` ÷ distinct ticket keys with any ticket row, null
  `partial_tokens` when Σ`sessions_without_tokens` > 0; `prompts_per_ticket` = Σ`prompts` ÷
  the same, null with the contributor's reason when any `prompts` is null.
- **Sprint usage** fields carry their coverage in their `_n` (`active_n`, `claude_n`,
  `user_n`, `wall_n`, `tokens_fresh_n`, …): summed, never marked partial.
- **Units.** Every `_ms` field is milliseconds; `on_hold_by_reason` stays minutes per reason,
  as locally.
- **Reason words.** The Dōjō reads them from `dojo.reason_codes` as F2's reads do.

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

*Amendment A1 (F2 pre-build audit, 2026-11-04): eligibility reads the plan's tenant; facts are
built once per tick; removed sprints carry no figures and removed tickets do not travel; the
hash is canonical and leaves `history_at` out (and `synced_at`, A5); marks are written before sending, keyed by
persona id, expire, and record failures; the push skips a tracker whose guard is held.*

**Where it runs.** A new step in the `dojo_sync` tick, after the graph push (`push_planned`).
The tick receives the tracker runtime (as `tracker_sync` does) and skips a tracker for this
tick when `locks.try_lock(tracker)` is taken (a sync or history run is writing it). An
unreachable Dōjō is the tick's existing transport handling.

**Facts are built once per tick, per tracker**, before the persona loop: they do not depend on
the persona. Predictability comes from the same replay as the sprint figures (one
`replay_inputs` read per tracker), never from a second `project_sdlc` load.

**Eligibility, per persona and tracker.** Take the local workspace's repositories
(`sensei.repositories_in_workspaces`) whose `repo_key` is in the persona's `plan.allowed`.
None → ineligible (`tracker_not_shared`). Their plan tenants (`MappedRepo.tenant_id`, the value
the Dōjō's own consent check compares) differ → ineligible (`tracker_workspace_ambiguous`,
B8). Otherwise the request names that tenant and those `repo_key`s; **the Dōjō resolves the
workspace** from the consent-checked repositories (`dojo.repositories_in_workspaces`), so a
forge-unit mapping (`map_forge_unit`) is honoured. A Dōjō 409 is logged as
`tracker_workspace_ambiguous` too.

**The person** is stamped by the Dōjō from the caller's token, never from the body (as
`/v1/you/metrics`). Effort is computed over **provable sessions only** (Data invariants).
Held-back sessions are counted and logged per pass, as held-back metric rows are.

**Board identity.** `board_key = hex(sha256(kind + "\n" + base_url + "\n" + board_ref))`.
`base_url` is canonical (`normalize_base_url`: `https://` + lowercase host) and `board_ref`
is the canonical board id (`canonical_board_id`: digits, no leading zeros), so two installs
on one board produce one key. It is a stable pseudonym, not anonymisation: names travel anyway
(B15). Its purpose is that no URL or account reaches the Dōjō.

**One request per board.** `POST /v1/you/delivery`, see D4 for the contract. Payload:

```text
{ v: 1, tenant_id, repo_keys: [text], board_key, kind, board_name,
  facts: {                                   // one-copy part
    history_at,                              // trackers.history_at; sent, not hashed
    synced_at,                               // last_success_at, only for a no-history tracker; else null. Not hashed (A5)
    pending_tickets: int,                    // tickets the history task would still read
    predictability, predictability_reason,
    sprints: [{ external_id, seq, name, state, start_at, end_at, complete_at,
                removed: bool, sdlc: SprintSdlc|null }],   // null for a removed sprint
    tickets: [{ key, title, status_category, points, sprints: [external_id],
                issue_created_at }] },        // non-removed tickets only
  effort: { … } | null }                     // this person's part (F3); null until then
```

- `pending_tickets` counts with the history task's own candidate predicate (non-removed, with
  an `issue_updated_at`, whose `history_through` is null or older), one shared SQL fragment.
  A tracker whose adapter reads no history (`capabilities().history == false`: GitHub, GitLab)
  sends 0: the history task never reads it (2026-11-09).
- `synced_at` (A5) is the `last_success_at` of the consistency token read before the facts,
  sent **only when the tracker's adapter reads no history**; a Jira board sends null, so its
  order is untouched. A token whose `last_success_at` reads null (the tracker was reconnected
  after the board list was read) skips the board this tick, as the Data invariants require. Every successful sync moves it, so like
  `history_at` it is left out of the facts hash: a sync that changed nothing re-sends nothing.
- A sprint's `sdlc` is its `SprintSdlc` (whose `facts` carries the counts); a removed sprint
  has no replay and sends `null`, never a made-up figure.
- A ticket's `sprints` is its membership (`sensei.tracker_task_sprints`, non-removed sprints),
  not only its current sprint. Removed tickets do not travel.
- A ticket object has exactly those keys (B9).

**The effort part** (*Amendment A2, F3 pre-build, 2026-11-05*). Built per persona and board,
after the facts, from **provable sessions only**: a candidate session (the tracker's
`candidate_sessions`, each with exactly one repository) is provable when its repository's
`repo_key` is in the persona's `exclusive_repos` (#240). Every other candidate session linked
to a ticket or in a sprint's `linked_sprint` is held back: counted in `held_back_sessions`,
logged per pass, never sent.

```text
effort: {
  source: uuid,                             // this install: the local persona's id (A3, F3 review)
  project: Row | null,                      // null when no provable session links any ticket
  sprints: [{ external_id, …Row }],         // a non-removed sprint with ≥ 1 provable linked_sprint session
  tickets: [{ key, …Row }] }                // a facts ticket with ≥ 1 provable session
Row = { repo_keys: [text],                  // the repositories of the row's sessions, ⊆ request repo_keys
        values: { field: number|text|null|object },
        reasons: { field: code } }          // one code for every null value
```

- **project** `values`: `sessions` (distinct provable sessions over the board's tickets),
  `tokens` (their summed tokens, `null` with `no_usage` only when there are none),
  `sessions_without_tokens`, `prompts` (`null` with `not_derived` / `derivation_failed`
  while any session's signals are not current). The Dōjō divides by the number of distinct
  tickets that have a ticket row from anyone.
- **sprint** `values`: D1's usage fields of `linked_sprint` over provable sessions, each with
  its `_n` (`sessions`, `shared`, `by_source`, `active_secs`… `api_cost_partial`,
  `first_at`, `last_at`). `linked_other` and `unlinked` never travel.
- **ticket** `values`: exactly D1's wire fields (`worked_ms` … `submitted_via`), from
  `TicketSdlc` computed over the ticket's provable sessions and their usage, plus the ✚
  numerators. A `null` keeps `TicketSdlc`'s own reason. `expected_minutes`,
  `time_variance_minutes`, `estimated_points` and the rate never travel.
- `effort` is always an object when sent (lists may be empty): it **replaces** the caller's
  rows for the board. Its hash is the `effort` part's mark, separate from `facts`; one request
  carries both parts whenever either is due.

Limits: at most 5 000 tickets, and a serialized body of at most 3.5 MB on the daemon (the
Dōjō refuses over 4 MB). A bigger board is skipped and recorded `delivery_board_too_large`.

**Consistency.** The facts are read with a token taken before and after
(`last_success_at`, the pending count, `max(history_through)`, `max(issue_updated_at)`): if it
moved, the board is skipped this tick rather than sent half-updated.

**Re-push watermark.** `sensei.tracker_push_marks (tracker_id → trackers, persona_id →
personas, tenant_id, part ∈ {facts, effort}, attempted_at, hash, pushed_at, failed_hash,
failed_at, error)`, primary key `(tracker_id, persona_id, tenant_id, part)`, cascading on
tracker and persona delete.

- `hash` is sha256 of the part's **canonical** JSON: object keys sorted recursively, sprints
  ordered by `(seq, external_id)`, tickets by key, each ticket's `sprints` sorted;
  `history_at` and `synced_at` are left out (each moves on every history run or sync, even one
  that read nothing; A5).
- The row is written (`attempted_at`) **before** sending, so a push that commits in the Dōjō
  but times out on the way back still leaves a trace a withdrawal can find (F5).
- Sent when the hash differs from `hash`, or `pushed_at` is older than **6 hours** (so a copy
  withdrawn or replaced in the Dōjō is restored by a contributor whose data did not change).
- `accepted` or `stale` → `hash`, `pushed_at` set, failure cleared. A refusal (4xx) →
  `failed_hash`, `failed_at`, `error` set; the same hash is not re-sent for 1 hour. A
  transport error → nothing but `attempted_at`; re-sent next tick.
- Each board's outcome is also recorded in the sync state (`mark_synced` / `mark_sync_error` /
  `mark_sync_skipped`, entity `tracker_board`, key `board_key`), so a failing board is visible
  outside the log: Settings → Dōjō shows it among the identity's own sync — its activity line,
  and a fault notice when failing — named by its key's start (`board abababab`), never as a
  repository. A board that fails to build, or a board read that fails, is
  recorded too and reaches the tick's failures, after every board has been tried. One board's failure, including a database error writing its mark, never
  stops the others.
- A changed tenant changes the key, so nothing is suppressed after re-filing.

**Withdrawal (B13).** `DELETE /v1/you/delivery/{board_key}?tenant_id=&source=` removes what
the caller's install (`source`, A3) reported for that board, and the board's kept copy when the
caller reported it. Queued in `sensei.delivery_withdrawals (board_key, tenant_id, persona_id,
queued_at, attempted_at, refused_at, error)` for **every** mark row of a tracker as it is deleted
by any path — a `BEFORE DELETE` trigger on `sensei.trackers` (`queue_delivery_withdrawals`), so a
workspace pruned or removed withdraws too; marks carry their `board_key` for it — and when a
board with a mark becomes ineligible (its marks then go). Sent first in the next tick. Done on 204,
or on the route's own 404 `{"withdrawn": false}`; any other 404 proves nothing and is retried. A
refusal waits an hour, outside the tick's faults meanwhile. A board pushed and acknowledged again
clears its queued withdrawal, so a late DELETE never removes a fresh push. A landed withdrawal
marks the board synced. Signing out queues nothing. When the withdrawn copy was the board's one
copy, the next contributor whose mark expires (6 h) restores it. *(As built in F5, after its
failure-mode review.)*

### D4 · Dōjō: contract, storage and reads

**Contract** (`dojo/src/routes/v1/you/delivery/+server.ts`, house error shape `apiError`):

| Case | Status |
|---|---|
| Applied or stale | 200 `{ facts: "accepted"|"stale" }`, plus `effort: "accepted"` when sent |
| Malformed body, unknown `v`, a ticket or sprint with other keys | 400 |
| Over 4 MB or 5 000 tickets | 413 |
| No or bad token | 401 |
| A `repo_key` is not the caller's shared and elected repository in `tenant_id` (`dojo.all_my_repositories.sync_enabled`) | 403 — the Dōjō re-checks consent, never trusts the daemon's |
| The repositories are in no workspace of that tenant | 404 `workspace_not_filed` (nothing is created) |
| The repositories span workspaces | 409 `workspace_ambiguous` |

One request is one transaction: either both parts' outcomes hold or nothing changed.
`DELETE` answers 204, or 404 when there is nothing of the caller's.

**Tables** (schema `dojo`, row-level security on, no grants: written and read only through
security-definer functions):

| Table | Key | Holds |
|---|---|---|
| `delivery_boards` | `id` uuid; unique `(workspace_id, board_key)` | `tenant_id`, kind, name, and the one copy's `reported_by` (a principal), `reported_at`, `history_at`, `synced_at` (A5; within B11, DF3), `pending_tickets`, `predictability`, `predictability_reason` — **nothing else (B11)** |
| `delivery_sprints` | `(board_id, external_id)` | details, `removed`, `sdlc` (null for a removed sprint) |
| `delivery_tickets` | `(board_id, key)` | title, status category, points, sprint ids, `issue_created_at` |
| `delivery_effort` | `(board_id, item_kind, item_ref, principal_id)`; `item_kind ∈ {project, sprint, ticket}`, `item_ref` = `''` / sprint `external_id` / ticket key | `repository_ids uuid[]` (whose sessions), `values` jsonb (D1 wire fields), `reported_at` |

All cascade from `delivery_boards`, which cascades from `dojo.workspaces`.

**Which copy wins (facts).** The function inserts the board row if missing (`on conflict do
nothing`), then locks it (`select … for update`), then compares, so two concurrent pushes are
ordered. The stored copy is replaced **whole** (board row, all its sprints and tickets) unless
it is greater: complete (`pending_tickets = 0`) beats incomplete, then the later `history_at`
wins (null sorts lowest), then, for two copies without history that both carry one, the later
`synced_at` (A5); on a tie the later arrival wins. So a daemon mid-backfill never
replaces a complete copy, and a copy's sprints and tickets always come from the same report.
A lesser copy answers `stale` and changes nothing.

*Amendment A5 (2026-11-09, DF1 of `docs/analysis/2026-11-09-delivery-metrics-forge-trackers.md`;
revised after the design review the same day).* A board whose tracker reads no history (GitHub,
GitLab) never has a `history_at`, so its copies tie on every comparison and the last arrival
wins: a daemon that last synced hours ago can replace a copy synced minutes ago.

**What travels.** The daemon sends `facts.synced_at` = its token's `last_success_at` **only for
a tracker whose adapter has `capabilities().history == false`**; for Jira it sends null. So the
dōjō needs no knowledge of tracker kinds, and Jira's order is untouched, before and after its
first history read (DF1).

**Order.** The stored copy is kept (`stale`) or replaced (`accepted`) by the first rule that
decides:

1. Complete beats incomplete (unchanged).
2. At least one copy has a `history_at`: the later wins, null lowest (unchanged).
3. Both `history_at` null **and both `synced_at` non-null**: the incoming copy is `stale` when its
   `synced_at` is earlier than the stored one's.
4. Otherwise the later arrival wins (unchanged). This covers equal `synced_at`, and any pair where
   either side has none: a Jira board, an old daemon, a row filed before A5.

Rule 3 compares only two values that are both present, so an old daemon (no `synced_at`) is
never locked out by a new one: it keeps today's later-arrival behaviour (design review F-DC1).

**Clamp, not refuse.** The function stores `least(synced_at, now())`. `last_success_at` is
never legitimately in the future, so a future value only shifts to the dōjō's clock: it can
not pin a copy past real time, and a daemon whose clock runs fast is not refused (design
review F-SEC1; `history_at` keeps its existing one-day refusal).

**Malformed timestamps.** `checkDeliveryBody` refuses with 400 a `history_at` or `synced_at`
that is a string `Date.parse` cannot read, through one shared check for both fields; today such
a string reaches the function's cast and fails as 500 (design review F-SEC2).

**B11.** `synced_at` is held per copy only to order copies, as `history_at` already is. It is
not a connection setting: it names no URL, account or field, and nothing can be connected or
re-synced from it. B11's "no sync state" does not cover it (user, 2026-11-09, DF3).

**Rollout**, in either order: the ingest passes facts keys it does not name through to the
function, which reads only named keys, so an old dōjō ignores `synced_at`; an old daemon sends
none, which reads as null and falls to rule 4. The column is nullable and added by the schema
reconcile (schema unreleased), with no backfill.

**Changes, by surface.**

| Surface | Change |
|---|---|
| `crates/senseid/src/dojo/delivery.rs` | `BoardFacts.synced_at: Option<DateTime<Utc>>`; `facts_json` emits it; `facts_hash` also removes `synced_at` and its doc says so; tests building `BoardFacts` (e.g. `:485`) set it |
| `crates/senseid/src/tasks/dojo_delivery.rs` | `build_one` skips the board (`Ok(None)`) when `before.last_success_at` is null; sets `synced_at` from it when `capabilities_of_kind(&t.kind)?.history` is false, else `None`; module doc |
| `crates/senseid/src/tasks/dojo_sync.rs` | tests for gate 19 |
| `database/ddl/table/dojo/delivery_boards.ddl` | nullable `synced_at timestamptz` and its column comment; table comment's order and its B11 sentence; `history_at` comment no longer "the" orderer |
| `database/ddl/function/dojo/file_delivery_facts.ddl` | `v_synced := least((v_facts ->> 'synced_at')::timestamptz, now())`; rule 3 in the stale predicate; insert and update set `synced_at`; header comment and `comment on function` state the four rules |
| `database/tests/dojo/delivery_facts.sql` | header restates the order; gates 20–23 (sql) |
| `dojo/src/lib/server/delivery-ingest.ts` | `DeliveryFacts.synced_at?: string \| null`; the shared timestamp check for `history_at` and `synced_at`; its spec file for gate 23 (dojo) |
| `docs/backlog.md` | close the DF1 entry when built |
**Workspace.** Resolved from the `repository_id`s of the caller's verified
`all_my_repositories` rows for `tenant_id`, through `dojo.repositories_in_workspaces` — never
by matching `repo_key` across tenants. No workspace → 404; more than one → 409.

**Effort.** The caller's rows for the board **from the same install** (`effort.source`) are
replaced by the request's: rows absent from it are deleted (B13). `dojo.delivery_effort_row_ok`
admits only the kind's D1 fields, each of its type, with ≥ 1 session; the effort part is
bounded like the facts (5 000 tickets, 1 000 sprints, 4 MB) in the database itself. *A2:* one function, `dojo.file_delivery(p_tenant, p_repo_keys, p_board,
p_effort)`, files the facts (as `file_delivery_facts`) and then the effort in one transaction
and answers `{facts, effort}`; effort is filed even when the facts answer `stale` (it is the
caller's own). A row's `repo_keys` outside the request's → 422 (`invalid_parameter_value`, mapped by `db-refusal.ts`; corrected 2026-11-09, was 400);
its `repository_ids` are the verified ids of those keys.

**Merge** (*A2, as built*): `dojo.delivery_effort_merged(p_kind, p_rows jsonb,
p_issue_created_at, p_tickets)` — `p_rows` the contributors' `{values, reasons}`, `p_tickets`
the project's per-ticket denominator — is pure (immutable), built on `delivery_merge_sum`
(one field) and `delivery_merge_pair` (a ratio's two sides over the same contributors), and
answers `{ values, reasons, partial }` for one item, by D1 and A3: each sum
over known values with `partial[field]` = contributors whose value was `null`, `null` with
`no_contributor_value` when none knew it; min / max ignore `null`; the recomputed figures
from those sums, `partial` when any input is, `null` with the local reason when a denominator
is 0 or `null`. Reads add `team` (that object), `contributors` (a count) and `last_report`
(the latest `reported_at`) to each board (project), sprint and ticket, through
`dojo.delivery_team`, which applies B16 with `dojo.delivery_team_view` and gives
`last_report` only from two contributors on (one person's report time is theirs).

**Reads** (security definer, each answering nothing unless `can_read_workspace`; the route
answers 403 when it is false, as `getWorkspaceMetrics` does):

- `workspace_delivery_boards(p_workspace)` — boards with predictability (and, from F3, the
  merged project row).
- `workspace_delivery_sprints(p_workspace, p_board, p_limit, p_offset)` — non-removed sprints
  in the daemon's order (start date desc, undated last, then `seq` desc, then
  `external_id`), as D2; the route reads every page (`readAllPages`).
- `workspace_delivery_tickets(p_workspace, p_board, p_sprint null|external_id, p_backlog bool,
  p_limit, p_offset)` — all, one sprint, or the backlog (in no non-removed sprint), in the
  app's order (key prefix, then its number, then key).

**Who sees a person's numbers.** An effort row is shown per person only to a caller for whom
`can_read_repository_metric(r, 'user', principal, tenant)` holds for **every** `r` in its
`repository_ids` (called, not copied): the person themself, a tenant admin, a maintainer on
those repositories, or the person's team lead. Every other reader of the workspace, viewers
included (`can_read_workspace` admits them), gets the merged value only, with `contributors`
(a count) and the latest `reported_at` across them, never a per-contributor time. Removed
members' rows still count in merged values until withdrawn.

**Staleness.** A contributor whose last report is older than 7 days is shown as stale to those
who see per-person rows; others see "includes stale data" when any is.

*Amendment A4 (F4 build, 2026-11-05).* `dojo.delivery_team` decides per reader, per item:

- A reader **sees the people** of an item when `can_read_repository_metric(r, 'user',
  principal, tenant)` holds for every `r` of every row on it. They get the **unrestricted**
  merge (cost and one-contributor items included, B16 opened), `last_report`, and `people`:
  one entry per contributor — their rows across installs merged by the same function, their
  `display_name`, latest `reported_at` and `stale` (older than 7 days).
- **Anyone else** keeps B16's restricted view and `people` = null; `stale` is a single
  boolean, true when any contributor's last report is older than 7 days, never which — and
  only from two contributors on, since one person's report age is theirs (F4 review).
- The reads add `people jsonb` and `stale boolean` beside `team`, `contributors` and
  `last_report`.

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
   ticket object's keys are exactly D3's; a removed ticket is absent; a removed sprint has
   `sdlc: null`; a ticket in two sprints lists both.
9. **#240 (daemon).** Candidate sessions from a repository another persona also elected, or
   from a repository with no `repo_key`, are absent from effort and counted in
   `held_back_sessions` (a session with no repository is never a candidate: A2).
10. **Watermark (daemon).** A second pass with nothing changed posts nothing, including after
    a history run that read nothing new (`history_at` moved). Two payloads equal but for key
    insertion and sprint order hash equal. A mark older than 6 h re-sends. A 400 on one board
    records its failed hash and sync-state error and leaves the other boards marked; the same
    hash is not re-sent within the hour, a changed one is. A mark-write failure on one board
    does not stop the next. The mark row exists before the stub plane is called.
10a. **Guard (daemon).** With the tracker's lock held, the pass posts nothing for it; facts
    are built once per tracker per tick whatever the number of personas.
11. **Withdrawal (daemon).** Deleting a pushed tracker queues and then sends one `DELETE`; so
    does the workspace losing its allowed repository.
12. **Contract (dojo).** Each row of D4's contract table has a test returning that status;
    a 403 for a `repo_key` not in the caller's shared and elected set.
13. **One copy (sql).** A copy with `pending_tickets > 0` never replaces one with 0, whatever
    `history_at`; between complete copies the later `history_at` wins, in either arrival order;
    equal `history_at` → the later arrival wins; a null `history_at` loses to any; a replaced
    copy's tickets absent from the winner are gone; repositories filed in another tenant's
    workspace under the same `repo_key` are not used; the four functions are in
    `definer_functions_gate_the_caller.sql`, called by a non-member with alice's ids.
14. **Merge (sql).** Fixture: A and B on `T-1`, A alone on `T-2`.
    `workspace_delivery_tickets` returns for `T-1` `worked_minutes` = A+B, `work_started_at` =
    min, `submitted_at` = the earliest PR, `execution_hours` from those two,
    `build_failure_rate` = (fA+fB)÷(runsA+runsB)×100, `mean_build_recovery_minutes` from
    summed sums and counts; `contributors` 2 and 1. `T-2` (one contributor) and every
    `effort_cost` show no figure to any reader in F3 (B16).
15. **Unknown (sql).** A sends `ai_token_cost` null, B sends 3.0 → 3.0 with `partial: 1`;
    `cost_per_prompt_cycle` null. A not submitted (`rework_ms` null `not_submitted`) leaves
    `post_review_rework_ratio` = B's alone and **not** partial; Σ`api_cost_n` < Σ`sessions`
    → `cost_per_prompt_cycle` null `partial_cost`.
16. **Replace (sql).** A pushes `T-1`, `T-2`, then pushes `T-1` only → A's `T-2` row is gone.
17. **Visibility (sql).** Person A reads their own rows; a tenant admin reads A's; a plain
    workspace member and a viewer read merged values with no `principal_id` and no
    per-contributor time; `can_read_workspace` false → the route answers 403.
18. **View (dojo).** With fixture 14 loaded, the Delivery view shows `T-1`'s title,
    "from 2 contributors", and an error state on a failed read.

**A5 (copy order without history).**

19. **Push (daemon).** `facts_json` carries `synced_at`; two facts differing only in `synced_at`
    give one `facts_hash`. In `tasks/dojo_sync.rs`, beside
    `an_unchanged_board_is_not_sent_again_until_its_mark_expires` and on its fixture: a GitLab
    tracker's pushed body carries `synced_at` = its `last_success_at`; the same board as Jira
    carries `synced_at: null`; moving only `last_success_at` (`UPDATE … SET last_success_at =
    now()`) sends nothing on the next tick; a GitLab tracker whose `last_success_at` is set to
    NULL after the board list is read is not posted.
20. **Order without history (sql).** Both `history_at` null, both `pending_tickets` 0, stored
    `synced_at` t5: incoming t3 → `stale`, the stored row, sprints and tickets unchanged;
    incoming t7 → `accepted`, the row holds t7; incoming t5 → `accepted` (arrival). Incoming
    with no `synced_at` against stored t5 → `accepted` (old daemon); incoming t3 against a
    stored null → `accepted`.
21. **Jira order unchanged (sql).** Stored `history_at` h2, `synced_at` null; incoming h3 →
    `accepted`; incoming h1 → `stale`; incoming null history → `stale`. Two copies with both
    `history_at` and both `synced_at` null, pending 3 each → the later arrives `accepted`.
22. **Completeness first (sql).** Stored `pending_tickets` 0 at t1, incoming 3 at t9, both
    history null → `stale`.
23. **Clamp and shape (sql + dojo).** Incoming `synced_at` now + 2 days → `accepted`, and the
    stored `synced_at` ≤ `now()`; a following copy with `synced_at` = now() → `accepted`.
    `checkDeliveryBody`: `synced_at` absent, null or an ISO string → ok; `5` → 400;
    `"garbage"` as `synced_at` or as `history_at` → 400 `facts.<field> must be a timestamp or
    null`.

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
- (A5) A no-history board's older-synced copy replaces a newer one; a Jira board sends a
  `synced_at` or its order changes; `synced_at` is hashed, so every sync re-sends every board;
  an old daemon's push (no `synced_at`) is refused or answered `stale` for lack of one; a
  future `synced_at` is stored as given or refused.
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
| C22 | The Dōjō workspace route had one page, no sub-routes | `+page.svelte`, `+page.ts`, `metrics-load.spec.ts` | CONFIRMED (corrected); superseded by F2, which adds `workspaces/[id]/delivery` |
| C23 | `GET /api/reason-codes/{domain}` serves `sdlc_metrics` | `routes.rs:491` | CONFIRMED |
| C24 | Two installs produce the same `board_ref` for one board | `canonical_board_id` (`tracker/jira.rs:27`) for typed and pasted board ids; `normalize_base_url` | CONFIRMED (F2 build start) |

### Claims for A5 (independent claims-verifier 2026-11-09, then corrected)

| # | claim | evidence | verdict |
|---|---|---|---|
| C25 | GitHub and GitLab adapters read no history, so their trackers' `history_at` stays null | `github.rs:468`, `gitlab.rs:318` `history: false`; `history.rs:46` skips before writing; only writer `tracker_history.rs:149` | CONFIRMED |
| C26 | every successful sync sets `last_success_at = now()`, unconditionally; nothing else sets it but a reconnect (NULL) | `trackers.rs:755`, `:1141`; failure path `:545` sets only `last_error` | CONFIRMED |
| C27 | the consistency token carries `last_success_at`, read before and after the facts | `delivery.rs:50-56, :213`; `dojo_delivery.rs:102-106` | CONFIRMED |
| C28 | `facts_hash` leaves out `history_at` and nothing else | `dojo/delivery.rs:165-171` | CONFIRMED |
| C29 | the ingest checks no key set on `facts` and hands it whole to the function, which reads only named keys | `delivery-ingest.ts` `checkDeliveryBody` (`facts: f`); `file_delivery_facts.ddl` `->>` / `jsonb_to_recordset` | CONFIRMED |
| C30 | an absent `facts.synced_at` reads as null | `('{"a":1}'::jsonb->>'synced_at')::timestamptz is null` → t | CONFIRMED |
| C31 | today, equally complete copies with both `history_at` null are ordered by arrival only | `file_delivery_facts.ddl:90-93` | CONFIRMED |
| C32 | the schema is unreleased and reconciled; a nullable column on an existing dōjō table needs no `apply/` script | no `released` in `design.yaml`; `deploy-dojo-db.sh:38-43`; precedent `f2b39c355` (v0.20.0) | CONFIRMED |
| C33 | a `stale` answer is acknowledged, and that hash is re-sent only after the 6 h mark expiry | `dojo_delivery.rs:440-454`; `is_due` (`dojo/delivery.rs:424-435`) | MISLEADING → corrected (was "not retried") |
| C34 | ~~a future value is refused with `invalid_parameter_value`, 400~~ | `db-refusal.ts:15` maps `22023` → 422 via `callFn` (`tenant-rows.ts:9-12`) | **FALSE**. Found by the verifier and the spec reviewer independently. Cost: gate 23 would have asserted a status the route never gives. A5 now clamps instead, so no refusal remains; the inherited D4 line (effort `repo_keys` → "400") was the same error and is corrected to 422 |
| C35 | a board with `last_success_at` null is never listed, but the token is re-read separately and can read null | `delivery_trackers` `WHERE … last_success_at IS NOT NULL` (`delivery.rs:124`); `delivery_token`; `build_one` compares tokens for equality only | CONFIRMED. The first rewrite sent null here, which broke the Data invariant (spec:60) and let a reconnected tracker's copy win by arrival; corrected to skip (re-audit) |
| C36 | `history_at` is the daemon's own clock and has a one-day future refusal | `tracker_history.rs:149` `now()`; `file_delivery_facts.ddl:69-71` | CONFIRMED |
| C37 | a malformed timestamp string passes `checkDeliveryBody` and fails at the function's cast as 500 | `textOrNull` (`delivery-ingest.ts:225`) tests only "is a string"; 22007 is not in `db-refusal.ts` → 500 | CONFIRMED (security review; re-checked against `db-refusal.ts`) |
