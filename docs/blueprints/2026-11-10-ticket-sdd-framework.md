---
title: Ticket SDD framework
description: Detect which third-party spec-driven-development framework a session's repo works in, and pin the first one seen onto every tracker ticket it touches
type: blueprint
status: blueprint
created: 2026-11-10
depends_on:
  - docs/analysis/2026-10-29-sdlc-metrics-extension.md
  - docs/spec/daemon/tracker-sdlc-metrics.md
related_issues: [390, 391, 392]
references:
  - crates/senseid/src/sdlc/signals.rs
  - crates/senseid/src/tasks/session_signals.rs
  - crates/senseid/src/db/pg_store/session_signals.rs
  - crates/senseid/src/db/pg_store/trackers.rs
  - crates/senseid/src/tracker/sync.rs
  - database/ddl/table/activity/sessions.ddl
  - database/ddl/table/sensei/tracker_tasks.ddl
---

# Ticket SDD framework

## Objective

Record, for each tracker ticket, which third-party spec-driven-development (SDD) framework its work was done in:
OpenSpec, Spec Kit, Kiro or BMAD. Detection is per **session**, scoped to the repo the session ran in
(`activity.sessions.repo_folder_id`), observed by sensei. The ticket takes the **first framework encountered**, and keeps
it: set once, never overwritten (decision B1).

Today the framework is assumed, not recorded: `task_tick` matches any `tasks.md`, `is_code` excludes only `openspec/`,
and `doc_indexer` knows only OpenSpec paths. This blueprint makes it a recorded fact.

## Decisions

| # | Decision | Source |
|---|---|---|
| B1 | Ticket value is set once from the first session that detects a framework; a later or relinked earlier session never changes it | user, 2026-11-10 |
| B2 | Behavioural detection (what the session did) wins; static repo markers are the fallback | user, 2026-11-10 |
| B3 | Only third-party frameworks. sensei's own plan graph / `docs/spec` is **not** a framework value; the sensei repo resolves to `NULL` | user, 2026-11-10 |
| B4 | Scope = the session's repo, among repos sensei observes | user, 2026-11-10 |
| B5 | No match → `NULL`. Never a default (no-fabrication rule). A detection failure (unreadable repo) is an error, not `NULL` | CLAUDE.md hard rule |

## Architecture

```
hook events ──► tasks/session_signals.rs ──► sdlc::framework::detect(events, markers) ─┐
                         │                                                             │
              repo_folder_id ──► repo marker scan (fallback) ─────────────────────────┤
                                                                                       ▼
                                                        activity.sessions.sdd_framework (+ source)
                                                                                       │
tracker sync ──► write_links (trackers.rs) ──► pin_ticket_frameworks (same tx) ◄───────┘
                                                        │
                                                        ▼
                         sensei.tracker_tasks.sdd_framework  (set once, + sdd_framework_session_id)
```

## Components

### `sensei.sdd_framework` enum (new)

`'openspec' | 'spec_kit' | 'kiro' | 'bmad'`. Adding a framework = an enum value + a row in the detector's
signature table.

### Detector: `crates/senseid/src/sdlc/framework.rs` (new, pure)

`detect(events: &[SignalEvent], repo_markers: &[RepoMarker]) -> Option<(SddFramework, DetectionSource)>`.
Pure, with no I/O. The marker list is passed in, so it can be unit-tested.

Signature table (one place, reused by `is_code` and `doc_indexer` and not duplicated there; see Integration):

| Framework | Behavioural (B2, first) | Static marker (fallback) |
|---|---|---|
| OpenSpec | prompt/skill `/opsx:*`, `/openspec:*`, `openspec-*` skills; Edit/Write under `openspec/` | `openspec/` dir |
| Spec Kit | `/speckit.*`; Edit/Write under `specs/NNN-*/` or `.specify/` | `.specify/` dir |
| Kiro | Edit/Write under `.kiro/specs/` | `.kiro/specs/` dir |
| BMAD | Edit/Write under `_bmad/`, `.bmad-core/` | `_bmad/` or `.bmad-core/` dir |

Rules:
- Behavioural: the **earliest event** (by `ts`) that matches a signature decides. A single session that uses two
  frameworks takes the first.
- Static fallback only when no behavioural match. If more than one marker is present, use the order
  `openspec, spec_kit, kiro, bmad` and mark the result `static_ambiguous`, so it can be reported and not silently trusted.
- Nothing matched → `None`.

### Session column: `activity.sessions` (+2 columns)

- `sdd_framework sdd_framework` (NULL = none detected)
- `sdd_framework_source text check in ('behavioural','static','static_ambiguous')`

Written by the existing session-signals task (`tasks/session_signals.rs`, beside `store_session_signals`), so it
reuses the same read mark and re-derivation cadence. A re-derivation may change a **session's** value (more events
arrived). Only the ticket is frozen.

### Ticket columns: `sensei.tracker_tasks` (+3 columns)

- `sdd_framework sdd_framework`
- `sdd_framework_session_id uuid references activity.sessions(id) on delete set null`: provenance
- `sdd_framework_at timestamptz`: when it was pinned

Pinning (`pin_ticket_frameworks`): one statement, run in the same transaction as `write_links` and also after a
session's framework is first stored:

```sql
UPDATE sensei.tracker_tasks t
   SET sdd_framework = s.sdd_framework, sdd_framework_session_id = s.id, sdd_framework_at = now()
  FROM (SELECT DISTINCT ON (l.task_id) l.task_id, s.id, s.sdd_framework
          FROM sensei.session_task_links l JOIN activity.sessions s ON s.id = l.session_id
         WHERE s.sdd_framework IS NOT NULL
         ORDER BY l.task_id, s.started_at) s
 WHERE t.id = s.task_id AND t.sdd_framework IS NULL;      -- B1: never overwrite
```

`WHERE t.sdd_framework IS NULL` enforces B1. Among sessions linked at pin time, the earliest by `started_at` wins.
After that, nothing changes it.

## Data flow

1. Hook events land, and the session-signals task re-derives the session.
2. `detect(events, markers)`. Markers are read from the session's `repo_folder_id` path only when there is no
   behavioural hit. A read failure is an `Err` and is logged; the column is left untouched and not set to `NULL` (B5).
3. Session columns are stored, then `pin_ticket_frameworks` runs for that session's linked tickets.
4. Tracker sync relinks in `write_links`, then `pin_ticket_frameworks` runs in the same tx, for tickets that gained
   their first framework-bearing session.

## Integration points

| Integration | Method | Notes |
|---|---|---|
| `sdlc::signals::is_code` | read `NOT_CODE_DIRS` from the framework signature table | today it hard-codes `openspec`; Spec Kit/Kiro/BMAD dirs join it |
| `sdlc::signals::ticked` / `task_tick` | unchanged matching; signal may carry the session's framework | the escaped-task-defect metric can now be broken down by framework |
| `indexer::doc_indexer` classification | consume the same table for spec/design paths | removes the OpenSpec-only path literals |
| `tracker/writeback.rs` | none in v1 | D9 still defers the write-back field set; the column is write-back-capable |
| Dōjō / app ticket view | read `tracker_tasks.sdd_framework` | display only; out of scope here |

## Dependencies

| Dependency | Status | Impact if missing |
|---|---|---|
| `activity.sessions.repo_folder_id` | exists | no static fallback; behavioural still works |
| `session_task_links` + tracker sync | exists | no ticket to pin onto; session value still recorded |
| session-signals task (`tasks/session_signals.rs`) | exists | no detection cadence |
| Hook event capture (prompts, Edit/Write paths, Skill calls) | exists | behavioural detection blind; static only |

## Implementation order

1. DDL: `sdd_framework` enum, session columns, ticket columns (`make test-template` after).
2. `sdlc/framework.rs`: signature table and `detect`, with unit tests per framework, both precedence rules, and `None`.
3. Re-point `is_code` and `doc_indexer` at the signature table (DRY), with existing tests staying green.
4. Session-signals task: detect and store; marker read errors propagate.
5. `pin_ticket_frameworks` in `pg_store/trackers.rs`, called from both `write_links` call sites and after session store.
   Tests: first-wins, no overwrite on later session, no overwrite on relinked earlier session, NULL stays NULL.
6. Backfill: run the session-signals re-derivation over existing sessions; pinning follows.

## Personas

No `.sensei/personas/` defined; skipped.

## Out of scope

- sensei's own plan graph as a framework value (B3).
- Tracker write-back of the field (D9, deferred).
- Multiple frameworks per ticket (B1 picks one).
- UI surfacing in the app or Dōjō.
