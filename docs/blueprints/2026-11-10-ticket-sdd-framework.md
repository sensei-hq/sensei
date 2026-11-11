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
| B6 | A repo folder that no longer exists on disk = no static markers (`NULL`). A permission or other I/O error is still an `Err` (B5). A deleted repo must not turn the pass red on every run | user, 2026-11-11 |
| B7 | Static markers are probed on disk under the repo's `abs_path`, not read from `sensei.folders`: that table holds no hidden directories (0 rows with a leading dot, 2026-11-10), so `.specify/`, `.kiro/`, `.bmad-core/` are never in it | build-start check, 2026-11-10 |
| B8 | Slash commands are matched at the start of **any line** of the prompt: a prompt can begin with IDE context (`<ide_opened_file>…`) before the command | build-start check, 2026-11-10 |
| B9 | Existing sessions are backfilled by a derivation version on `activity.session_signal_marks`: the pass re-selects a session whose mark has an older version. Today it re-selects only on an event or catalogue change | build-start check, 2026-11-10 |

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

`detect(events: &[SignalEvent], markers: &[SddFramework]) -> Option<(SddFramework, DetectionSource)>`.
Pure, with no I/O. The marker list is passed in, so it can be unit-tested. The disk probe is a separate thin function,
`repo_markers(repo: &Path, probe) -> Result<Vec<SddFramework>, String>`, whose `probe` is injected (B6, B7).

`SignalEvent` gains two projected fields for this (read in `session_events_for_signals`): `prompt` (a
`UserPromptSubmit`'s `payload.prompt`, its first 4 KiB) and `skill` (a `Skill` call's `tool_input.skill`).

Signature table (one place, reused by `is_code` and `doc_indexer` and not duplicated there; see Integration):

| Framework | Behavioural (B2, first) | Static marker (fallback) |
|---|---|---|
| OpenSpec | prompt line starting `/opsx:` or `/openspec:` (B8); skill `openspec-*`; Edit/Write under `openspec/` | `openspec/` dir |
| Spec Kit | prompt line starting `/speckit.` (B8); skill `speckit*`; Edit/Write under `specs/NNN-*/` or `.specify/` | `.specify/` dir |
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
2. `detect(events, markers)`. Markers are probed on disk under the `abs_path` of the session's `repo_folder_id`, only
   when there is no behavioural hit (B7). A missing repo folder gives no markers (B6). Any other probe failure is an
   `Err`: the session counts as failed and its columns are left untouched, not set to `NULL` (B5).
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
