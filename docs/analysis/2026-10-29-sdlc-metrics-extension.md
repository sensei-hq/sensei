---
name: SDLC metrics extension
description: Analysis — extend metrics collection to the ticket/sprint/project reference set in ai_sdlc_metrics/docs/metrics.json
date: 2026-10-29
stage: analysis
depends_on:
  - spec/daemon/tracker-jira-linkage.md
  - spec/daemon/tracker-metrics-writeback.md
  - analysis/2026-10-26-tracker-metrics-writeback.md
  - spec/2026-08-18-repo-grain-metrics-watermark-engine.md
next: /sensei:design
---

# SDLC metrics extension (analysis)

**Ask (verbatim):** *"can we extend the metrics collection based on this list
../ai_sdlc_metrics/docs/metrics.json"*

**Restated from what exists:** the reference set has 27 metrics at ticket (18), sprint
(7) and project (2) level, built from 40 inputs (`metric-inputs.json`). sensei's metric
catalogue (`database/import/staging/metrics.jsonl`, 34 metrics) is repo/user-scoped at
session or daily grain. The only ticket-grain numbers today are the tracker linkage's
R7 usage, computed at read, which the write-back slice pushes to Jira. This slice
brings the reference set into sensei, shown in sensei's screens and written back to
the tracker.

## Decisions (user, 2026-10-29)

| # | Question | Answer |
|---|---|---|
| D1 | Where are the numbers read? | **Both**: sensei's screens (Tickets, Sprints) **and** tracker write-back |
| D2 | Sprint history: backfill from the tracker, or snapshot from now? | **Backfill** from the tracker (and keep it current) |
| D3 | `hourly_rate` | **Project-level** blended rate for now; no per-person rate |
| D4 | Branch | `feature/sdlc_metrics`, cut from `feature/tracker_metrics_writeback` (D1 needs its write path; that PR is not merged yet) |
| D5 | Colliding names (§6 Q1) | Keep R7's `cycle_time` (first session → last session). The list's edit → PR span is `execution_time`; the list's rework share is `post_review_rework_ratio` |
| D6 | Estimates (§6 Q2) | `estimated_points` = the tracker's story points (pre-work). No `tasks.md` heuristic; `points_variance` needs only an `actual_points` definition |
| D7 | Review defects (§6 Q3) | Detect two sources (§4 "Review defects"): **spec-driven tasks that turn out broken at build time** (e.g. OpenSpec tasks ticked done, then the build/test fails), and **cross-model review** (generated code reviewed by another model, and its findings fixed) |
| D8 | Build/test classifier (§6 Q4) | **Every framework**, not a fixed list (§4 "Classifier") |
| D9 | Write-back field set (§6 Q5) | Deferred; the user decides later. Design keeps every new figure write-back-capable but maps none by default |

## 1 · What was read (grounding)

| Source | What it says |
|---|---|
| `../ai_sdlc_metrics/docs/metrics.json`, `metric-inputs.json` | 27 metrics, 40 inputs; inputs carry hook-proxy `source` rules (e.g. `work_started_at` = first Edit/Write outside spec/docs) |
| `../ai_sdlc_metrics/docs/implementation_ideas.md` | Option 1 (write-back) vs Option 2 (hooks + local store); sensei is already Option 2 plus write-back |
| `database/ddl/enum/sensei/metric_grain.ddl`, `metric_scope.ddl` | grain ∈ `session, daily`; scope ∈ `repo, user`. No ticket or sprint grain |
| `database/import/staging/metrics.jsonl` | 34 metrics; `rework_ratio` already exists ("tool-calls from corrected sessions") |
| `crates/senseid/src/tracker/usage.rs` `Usage`; linkage spec R7 | Per ticket / sprint / backlog: sessions, active / Claude / user / wall time, token split, `metered_cost`, `api_cost` (list prices), `first_at`/`last_at` → cycle time = `last_at − first_at` |
| `database/ddl/table/sensei/tracker_tasks.ddl` | `status`, `issue_type`, `current_sprint_id`; **no story points, no status history** |
| `database/ddl/table/sensei/tracker_task_sprints.ddl` | Sprint membership **replaced per sync**, so no record of committed vs added vs removed |
| `database/ddl/table/activity/assistant_events.ddl` | Every hook event with `tool_name`, `success`, `payload` (command, `tool_use_id`, `duration_ms`) |
| `marketplace/plugins/sensei/.claude-plugin/plugin.json` | Registers PreToolUse, PostToolUse, PostToolUseFailure, Notification, UserPromptSubmit, Stop, SubagentStop, SessionStart/End, PreCompact. **No `PermissionRequest`** |
| `crates/senseid/src/tracker/*.rs` | No story-point, changelog or sprint-report read anywhere |

## 2 · Surprises (the assumptions the list carries)

1. **Cost needs no token rates.** The list prices tokens with project
   `input_token_rate`/`output_token_rate`. sensei already prices each session at list
   prices per model (`api_cost`, session-api-cost C5), with cache tokens priced correctly.
   `ai_token_cost` = R7 `api_cost`; adding rates would be a second, less accurate cost.
2. **Two names collide with different meanings.**
   - `rework_ratio`: catalogue = share of tool calls in *corrected sessions*; list =
     share of *active time after PR submission*.
   - `cycle_time`: R7 = first session start → last session end; list = first code edit →
     PR created. Both are legitimate; they must not share a key.
3. **The engine is repo-grain.** `metric_grain`/`metric_scope` cannot express ticket or
   sprint. R7 is computed at read with nothing stored, so the ticket metrics have a
   home in R7, not in the watermark engine.
4. **Sprint metrics need history we overwrite.** Every sprint metric compares the sprint
   at its start against its end. `tracker_task_sprints` is replaced per sync, so the
   state at sprint start is already lost. D2 says backfill.
5. **Story points aren't synced.** Jira keeps them in a per-site custom field (the board's
   estimation field). Every `*_points` input and `velocity` waits on this.
6. **A daemon sees only its own machine** (write-back analysis §2.1). Ticket metrics
   built from hooks are one developer's view, the same caveat as R7. The write-back slice
   already narrows writes to tickets assigned to the token owner, and that holds here.
7. **`estimated_points` / `actual_points`** come from the list's own heuristics (task
   count in `tasks.md`, `prompts − 1 + files…`). sensei has plans (`docs/plans/`,
   registered plan graphs) but no `tasks.md` convention. These are inventions to design,
   not inputs to wire.

## 3 · Readiness by metric

**R = ready** (inputs captured today). **H = hook-derived** (captured, needs a
classifier/derivation). **T = tracker** (needs a new tracker read). **C = config**
(project-level setting, D3). **X = undefined input** (needs design).

| Metric | Level | Status | Note |
|---|---|---|---|
| `ai_token_cost` | ticket | R | = R7 `api_cost` (surprise 1) |
| `tokens_per_ticket` | project | R | Σ R7 tokens / tickets with ≥1 linked session |
| `prompts_per_ticket` | project | H | count `UserPromptSubmit` over linked sessions |
| `cost_per_prompt_cycle` | ticket | H | `api_cost` / prompts |
| `build_failure_rate` | ticket | H | Bash build-command classifier over PostToolUse(+Failure) |
| `mean_build_recovery_time` | ticket | H | failed build → next successful build |
| `total_defects` | ticket | H + X | compile + test are H; `review_defects` is X (seed: `review_finding` events) |
| `testing_efficiency` | ticket | H + X | as above |
| `review_efficiency` | ticket | H + X | as above |
| `cycle_time` (list) | ticket | H | first non-docs Edit/Write → `gh pr create` / `glab mr create` (fallback first push). Rename (surprise 2) |
| `lead_time` | ticket | H / T | list proxy = first hook event; **D2 backfill gives the real one** from the ticket's created date |
| `on_hold_ratio` | ticket | H + C | gaps > `idle_threshold` |
| `on_hold_by_reason` | ticket | H | `permission_wait` must come from `Notification` (no `PermissionRequest` hook) |
| `time_variance` | ticket | X + C | needs `estimated_points` × `minutes_per_point` |
| `rework_time` | ticket | H | active time after PR creation with Edit/Write |
| `rework_ratio` (list) | ticket | H | rename (surprise 2) |
| `rework_cost` | ticket | H + C | × project `hourly_rate` |
| `estimated_effort_cost` | ticket | R + C | R7 `active_secs` × project `hourly_rate` |
| `points_variance` | ticket | X | `actual_points − estimated_points` |
| `commitment_reliability` | sprint | T | sprint start/end membership |
| `total_completion` | sprint | T | as above |
| `scope_change` | sprint | T | added / removed after start |
| `unplanned_work_ratio` | sprint | T | as above |
| `carry_over_rate` | sprint | T | not done at end and in the next sprint |
| `velocity` | sprint | T | + story points |
| `points_commitment_reliability` | sprint | T | + story points |
| `predictability` | project | T | stddev of commitment reliability over last N sprints |

Totals (27): 2 ready, 1 ready + config, 14 hook-derived (3 of them also wait on
`review_defects`), 8 tracker, 2 blocked on undefined estimate inputs.

## 4 · Options

### Where the computation lives

| Option | What | For | Against |
|---|---|---|---|
| **A · Extend R7 (computed at read)** | New ticket fields join `Usage`; hook-derived inputs computed from `assistant_events` over the ticket's linked sessions at read | One definition already feeds screens **and** write-back (D1) with no new seam; matches R7's "nothing materialised" | Event scans at read; cost grows with history (needs a perf check) |
| **B · New grain in the watermark engine** | Add `ticket`/`sprint` to `metric_grain`, compute into `repository_metrics`-style rows | Trends and Dōjō rollups for free | A ticket isn't repo-scoped (spans repos); duplicates R7's definitions in a second place (DRY) |
| **C · A + materialised derivations** | R7 stays the read; the per-session derivations (build runs, prompts, PR-created-at, idle gaps) are stored per session once, R7 sums them | Read cost bounded; the classifier runs once per session; still one definition | One new per-session table to keep current |

**Recommendation: C.** Ticket metrics belong with R7, which already serves both D1
surfaces. The per-session derivations are the expensive, reusable part and are
session-scoped, the grain the engine already handles. B should wait until someone
asks for ticket metrics as a trend.

### Sprint history (D2)

What must be stored: per sprint, the membership (and points) at start, the
additions and removals with their timestamps, and the state at completion. This is a new
persisted table (e.g. `sprint_scope_events`), because history can't be recomputed at
read. Sources, **to be verified against a live Jira in design**:

| Source | Gives | Risk |
|---|---|---|
| Issue changelog (`expand=changelog`, Sprint + status + estimate field history) | Added/removed timestamps, status transitions (real lead/cycle time), point changes | Public API; one paged call per issue, so backfill cost scales with the board |
| Greenhopper sprint report (`/rest/greenhopper/1.0/rapid/charts/sprintreport`) | Committed / completed / not-completed / punted / added-during-sprint, per sprint | **Private API**, undocumented, can change without notice |
| Our own snapshots per sync | Going forward only | Misses the past (D2 rejects as sole source) |

**Recommendation:** changelog for backfill and as the source of truth. Per-sync diffs keep
it current, so the changelog is re-read only for changed issues. Don't depend on the
greenhopper API.

### Project-level configuration (D3)

`hourly_rate`, `idle_threshold`, `minutes_per_point` are per workspace (sensei's
"project"). Token rates are **not** added (surprise 1). The rate is a cost
assumption and carries no person data. Write-back writes it nowhere, only the cost
figure derived from it.

### Review defects (D7)

The list's `review_defects` ("real defects found and fixed from code review") becomes
two detected kinds, both from data already captured (`assistant_events.payload` keeps
`tool_input`, and `pg_store/sessions.rs` already reads `tool_input.subagent_type`):

| Kind | Signal | Counted when |
|---|---|---|
| **Escaped task defect** | An Edit/Write that ticks a task done in a spec-driven task list (OpenSpec `openspec/changes/*/tasks.md` `- [ ]` → `- [x]`; sensei plan graph `update_task_status` → done) | A build or test failure (D8 classifier) follows the tick, before the next tick or the PR, and a later Edit/Write fixes it. The defect belongs to the ticked task |
| **Cross-model review finding** | A review by **another model**: a reviewer subagent (`Agent` with a reviewer `subagent_type`), another CLI over Bash (`codex`, `gemini`, …), sensei `consensus` / `/sensei:review` (`review_finding` events) | The finding is followed by an Edit/Write on the file it names. Refuted findings (`refuted_finding_rate`'s population) aren't defects |

Escaped task defects are a **new quality signal** ("claimed done, broke at build"),
distinct from ordinary `compile_defects`. They should be reported on their own, not just
folded into `total_defects`.

### Build/test classifier (D8)

Every framework means the classifier must be data, not code branches:

- **Command catalogue:** a table mapping toolchain → build / test command patterns
  (cargo, go, dotnet, mvn/gradle, make/cmake/ninja, npm/pnpm/bun/yarn, pytest/tox/uv,
  swift, mix, bundle/rake, …), extended without a code change. It's owned in one place
  (DRY), and checked first against what the indexer already records per repo
  (stack detection, `get_project_summary`).
- **Indirection:** `bun run test`, `make check`, `just ci` only say what they do in the
  repo's own `package.json` scripts / Makefile / justfile. The classifier resolves the
  script name through the repo's manifest, falling back to its name (`test*`, `build`,
  `check`, `ci`).
- **Unclassified commands are reported, not guessed.** Each session keeps the count of
  non-zero-exit Bash commands that matched no pattern, so the catalogue's gaps can be
  seen rather than turning into silent zeros.

## 5 · Out of scope (proposed)

- Per-person rates, or any per-person figure beyond the existing assigned-to-me write.
- An `actual_points` heuristic until it is designed (D6); `points_variance` waits on it.
  `time_variance` is in, via story points × `minutes_per_point`.
- Watermark-engine grains (option B), and Dōjō rollups of ticket/sprint metrics.
- Project phases (P0–P7) / ticket phases (T0–T4) tagging from `sdlc-phases.json`.
- GitLab / other adapters (Jira first, the same seam as linkage).

## 6 · Open questions for design

Q1–Q4 resolved as D5–D8; Q5 deferred (D9). Still open:

1. **A finding's identity in another model's output.** A reviewer subagent or CLI
   returns prose. Is a finding one item in its output (needs parsing per reviewer), or
   one subsequent fix (an Edit on a file the review named)? The second is detectable
   everywhere but undercounts findings fixed together.
2. **Escaped defects across sessions.** A task ticked in one session and broken in the
   next (or by another developer) — count it against the ticking task, or only within
   the same session?
