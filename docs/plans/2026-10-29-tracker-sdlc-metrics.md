---
title: Tracker SDLC metrics — plan
description: Decompose the tracker-sdlc-metrics spec into ordered, testable features
type: plan
status: plan
created: 2026-10-29
depends_on:
  - docs/spec/daemon/tracker-sdlc-metrics.md
  - docs/analysis/2026-10-29-sdlc-metrics-extension.md
related_issues: []   # TBD: gh CLI not installed on this machine; create manually
references:
  - docs/spec/daemon/tracker-jira-linkage.md
  - docs/spec/daemon/tracker-metrics-writeback.md
  - docs/spec/screen/workspace-tickets.md
milestone:
---

# Tracker SDLC metrics — plan

## Objective

Implement `docs/spec/daemon/tracker-sdlc-metrics.md` (S1–S9). S, gate and C numbers refer
to that spec. Layers: **DDL + staging → daemon (`sdlc/`, `tracker/`, `db::pg_store`) → API
→ app**. Each feature adds its own DDL and runs `make test-template` (memory: local
Postgres is brew; build with `-j 2`). Pure functions (`classify`, `derive_signals`,
`replay`, the S6 computers) are built test-first against the spec's fixtures. Work goes on
`feature/sdlc_metrics`, which is based on `feature/tracker_metrics_writeback` (unmerged).
Rebase or merge when that PR lands.

**Before starting any feature,** re-run the claims ledger (C1–C39). Any `FALSE` stops the
build.

## Prerequisites (block build, not planning)

- **Plugin update (human).** The installed sensei plugin is 0.11.0 and lacks
  `PostToolUseFailure` (C1). Unit work doesn't need it. Gate 5's live meaning, and C34
  (`is_interrupt`), need it. `claude plugin update` or reinstall from the marketplace.
- **Daemon running** for gate 9 (live history backfill against board 333) and the
  MCP-backed workflow tools.

## Features

### F1 · Command catalogue + classifier (S3)
- DDL: `sensei.command_patterns`; `staging.import_command_patterns`;
  `database/import/staging/command_patterns.jsonl` seed (S3's toolchain list, with
  category-scoped markers).
- daemon: `sdlc/mod.rs`, `sdlc/classify.rs` (split, strip, folder-command match, catalogue
  match, category-scoped markers, exit attribution); `catalogue_hash`. Reuse
  `command_category::categorise`.
- **Done:** gate 1 (every listed case, plus the add-a-row case).

### F2 · Session signals (S2)
- DDL: enums `session_signal_kind`, `hold_reason`; `activity.session_signals`,
  `activity.session_signal_marks`.
- daemon: `sdlc/signals.rs` `derive_signals` (event time from `ts`; code paths; task ticks
  a/b/c; reviews; `review_fix` path tokens; slices and reasons; `unmatched_call`);
  `pg_store` read and replace-in-one-transaction; the `derive_session_signals` task added
  to `SCHEDULABLE` (10 min, 500 per pass, staleness rules).
- **Done:** gate 2 (fixture A, including the `created_at` + 3 h variant), gate 3
  task-tick parts, gate 4 (fixture C + the trailing-punctuation token), gate 5 (signal part).

### F3 · Settings (S1)
- DDL: `sensei.workspace_metric_settings`.
- daemon: `pg_store` get/upsert; `api/handlers/metric_settings.rs` under
  `local_caller_guard`; marking derivations stale on a threshold change.
- **Done:** gate 10.

### F4 · Ticket figures (S6 ticket + project) and reason codes
- staging: `reason_codes.jsonl` domain `sdlc_metrics` (every reason code the spec names).
- daemon: `sdlc/ticket.rs` (merged timeline, union slices, `worked_minutes`, defects with
  trigger fails, timing, holds, rework, costs, `not_derived`); project `tokens_per_ticket`
  and `prompts_per_ticket` over the distinct union.
- API: `sdlc` on `…/trackers/{t}/tasks` rows and `…/tracker-tasks/{key}`.
- **Done:** gate 2 (figures), gate 3, fixture E (in gate 4), gate 5 (figures), gate 13
  (ticket parts).

### F5 · Ticket history (S4)
- DDL: `tracker_tasks.{issue_created_at, issue_updated_at, points, status_category,
  history_through}`; `trackers.{estimation_field, history_error, history_at}`;
  `sensei.tracker_task_changes`; `sensei.tracker_statuses`; enum `tracked_change_field`;
  `design.dbml`.
- daemon: Jira board configuration + `/status` reads and the new issue fields, inside the
  existing R5 apply; `tracker/history.rs` with `HISTORY_DEADLINE`, per-issue commits,
  `issue_path`; chained inside `writeback::spawn_after_sync`'s guard (W2 override).
- **Done:** gate 8; then gate 9 live (needs the daemon).

### F6 · Sprint replay + sprint/project figures (S5, S6)
- daemon: `sdlc/sprint_replay.rs` (reuse `sprint_window`); sprint figures; `predictability`.
- API: `sdlc` on each `…/sprints` item; `sdlc_project` on each `GET …/trackers` item and
  the PATCH response.
- **Done:** gates 6, 7, 13 (sprint and project parts).

### F7 · Write-back keys (S9)
- DDL: append the 21 keys to `sensei.tracker_metric`.
- daemon: `TrackerMetric` additions; `TicketFigures`; `metric_value(metric, &TicketFigures)`
  with S9's kinds and rounding; the never-mappable four plus `estimated_points` not parseable.
- **Done:** gate 11 (including the comment golden).

### F8 · Screens (S8) — P-SCREEN
- docs: `docs/spec/screen/workspace-tickets.md` gains the SDLC group, the lane-header
  sprint/project row and "≥ n" defects, each with done and wrong gates.
- app: `api.ts` types and `try*` wrappers; Tickets detail SDLC group; lane header row;
  workspace metrics settings panel (PUT + named-key 400).
- **Done:** the screen spec's new gates, through the app's spec tests (`bun run` the root
  test script).

### F9 · Docs (P-DOCS)
- Every surface in the spec's Overrides list, plus the new `decisions.md` row and
  `backlog.md` (this slice in progress).
- **Done:** gate 12.

## Order and parallelism

F1 → F2 → F4 (signals feed figures). F3 runs in parallel with F1–F2. F5 → F6. F7 after
F4 and F6. F8 after F4, F6 and F3. F9 any time, and must be done before merge.

## Out of scope

`actual_points`, `points_variance`; ticket and sprint grains in the watermark engine;
Dōjō rollups; other tracker adapters; the SDLC phase tagging (P0–P7, T0–T4).
