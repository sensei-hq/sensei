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
| B1 | Ticket value is set once from the first session that detects a framework; a later or relinked earlier session never changes it. "First" is B13; the one exception, a still-running provenance session, is B14 | user, 2026-11-10 |
| B2 | Behavioural detection (what the session did) wins; static repo markers are the fallback | user, 2026-11-10 |
| B3 | Only third-party frameworks. sensei's own plan graph / `docs/spec` is **not** a framework value; the sensei repo resolves to `NULL` | user, 2026-11-10 |
| B4 | Scope = the session's repo, among repos sensei observes. **Path evidence counts only inside the repo**: a path is made relative to the repo's `abs_path`; a path outside it is ignored; a session with no repo has no path evidence | user, 2026-11-10; spec audit 2026-11-11 |
| B5 | No match → `NULL`. Never a default (no-fabrication rule). A probe that could not answer is recorded as an explicit state (B10), never as `NULL` | CLAUDE.md hard rule |
| B6 | A repo folder missing on disk (or whose `sensei.folders` row is gone) = no static markers. It is not an error | user, 2026-11-11 |
| B7 | Static markers are probed on disk under the repo's `abs_path`, not read from `sensei.folders`: that table holds no hidden directories (0 rows with a leading dot, 2026-11-10), so `.specify/`, `.kiro/`, `.bmad-core/` are never in it | build-start check, 2026-11-10 |
| B8 | A prompt is a command only when, after stripping leading `<ide_…>…</ide_…>` blocks, it **starts** with the command. Prompts that are relays (`<task-notification`, `<agent-message`) never count: 100 of 580 real prompts are relays, and a quoted `/opsx:apply` inside one must not classify the session | build-start check 2026-11-10; spec audit 2026-11-11 |
| B9 | Backfill: `activity.session_signal_marks.derive_version int not null default 0`; the code constant starts at 1; **every** mark write (success and failure) sets it. The pass re-selects a mark whose version is older. Sessions never derived or stale by events/catalogue are taken **before** sessions stale only by version, so a backfill never delays new sessions. The version only re-selects: a version-stale mark still counts as current for the metrics (spec S6), so a backfill blanks no ticket's figures | build-start check 2026-11-10; spec audit 2026-11-11 |
| B10 | A probe permission error or timeout does **not** fail the session: its signals and mark are stored as usual, and the session gets `sdd_framework = NULL, sdd_framework_source = 'probe_failed'`, an explicit error state. It is retried only when the session's events or `derive_version` change, so the schedule is never held red by one unreadable repo | user, 2026-11-11 |
| B11 | `static_ambiguous` (markers of two or more **distinct** frameworks) stays on the session but is **never pinned** onto a ticket; the ticket waits for unambiguous evidence. Real case: `ai-project-metrics-bmad` has an empty `openspec/` scaffold beside a populated `_bmad/` | user, 2026-11-11 |
| B13 | "First" = **first detected**: a ticket takes its value the first time a pin finds an eligible linked session. Among the sessions eligible at that moment, the earliest `started_at` (then `id`) wins. A session derived later, even one that started earlier (the B9 backfill derives old sessions last), never displaces it | user, 2026-11-11 (pre-build audit) |
| B14 | While the ticket's provenance session is **still running** (`completed_at IS NULL`), the ticket follows that session's current value: a re-derivation to another framework re-pins it, and one to `static_ambiguous` clears the ticket (all three columns `NULL`) so it can be pinned afresh. Once the session completes, the ticket is frozen. No other session ever changes it | user, 2026-11-11 (pre-build audit) |
| B15 | The pass pins **after** the session's write commits, in its own short transaction (`lock_timeout` 5 s, which bounds only the session locks: tickets are taken `SKIP LOCKED`). A ticket held by an apply is skipped; a pin failure is logged and never touches the session's signals; the next tracker sync, which pins tracker-wide, heals both | user, 2026-11-11 (pre-build audit) |
| B16 | Each pin is scoped to what its caller touched: a tracker sync pins **all** its tracker's tickets (so it heals pins missed in a race with the pass); the pass pins the tickets linked to, or provenanced by, its session. The `get_ticket` fetch link does **not** pin: a pin there could wait on a running sync past the MCP client's 30 s, and a failure would roll back the start of work. Sync and pass both read fetch links | pre-build audit 2026-11-11 |
| B17 | Only R1 candidate sessions of the ticket's tracker (`candidate_sessions`, as every ticket read counts them) are eligible: a fetch link or a removed repo's link that R1 no longer counts never pins | pre-build audit 2026-11-11 |
| B18 | The pin locks its chosen sessions `FOR KEY SHARE` before writing `sdd_framework_session_id`, so a session pruned concurrently is skipped (the ticket waits for the next pin) instead of failing the write with an FK violation | pre-build audit 2026-11-11 |
| B19 | `get_ticket` returns top-level `sdd_framework` (string or `null`) read from the ticket's mirror row **before** the start of work is recorded, independent of the session: no mirror row → `null`; a failed read → 500 before any write, like the rules read | pre-build audit 2026-11-11 |
| B12 | `static` means "the repo as found when the session was derived" (a repo that adopted a framework later is attributed to its earlier sessions on re-derivation; accepted). Guard: a re-derivation never replaces a session's non-NULL value with `NULL` or `probe_failed` because static evidence went missing or failed | user, 2026-11-11 |

## Architecture

```
hook events ──► tasks/session_signals.rs ──► sdlc::framework::behavioural(events, repo_root) ──┐
                         │                                                                   │ none
              repo_folder_id → abs_path ──► repo_markers (detached thread, 5 s, once per repo/pass) ─► from_markers
                                                                                             ▼
                                                    activity.sessions.sdd_framework (+ source), B12 guard
                                                                                             │
tracker sync ──► write_links (trackers.rs) ──► pin_ticket_frameworks(Tracker) (same tx)        │
session pass (after its commit, own tx, B15) ──► pin_ticket_frameworks(Session) ◄────────────────┘
                                                        │   (behavioural + static only, B11; R1 candidates, B17)
                                                        ▼
                         sensei.tracker_tasks.sdd_framework  (first detected B13, + sdd_framework_session_id)
```

## Components

### `sensei.sdd_framework` enum (new)

`'openspec' | 'spec_kit' | 'kiro' | 'bmad'`. Adding a framework = an enum value + a row in the detector's
signature table.

### Detector: `crates/senseid/src/sdlc/framework.rs` (new, pure)

- `SddFramework` (`as_str` / `parse`, as `SignalKind` does) and `DetectionSource`
  (`Behavioural | Static | StaticAmbiguous | ProbeFailed`).
- `behavioural(events: &[SignalEvent], repo_root: Option<&str>) -> Option<SddFramework>`: pure.
- `from_markers(present: &[SddFramework]) -> Option<(SddFramework, DetectionSource)>`: pure; dedups; one distinct
  framework → `Static`, two or more → first in order `openspec, spec_kit, kiro, bmad` with `StaticAmbiguous`.
- `repo_markers(repo: &Path, probe: impl Fn(&Path) -> io::Result<bool>) -> Result<Vec<SddFramework>, String>`: thin;
  checks the fixed marker paths only (never lists or walks a directory). Repo dir `NotFound` → `Ok(vec![])` (B6).

`SignalEvent` gains two projected fields (in `session_events_for_signals`): `prompt` (a `UserPromptSubmit`'s
`payload.prompt`, whole; `typed_command` skips leading `<ide_…>` blocks in Rust, so no cap may cut it first) and `skill` (a `Skill`
call's `tool_input.skill`).

Signature table (one place, reused by `is_code` and `doc_indexer`; see Integration). Paths are **repo-relative** (B4):

| Framework | Prompt starts with (B8) | Skill (after an optional `plugin:` prefix) | Repo-relative path prefix | Bash command word | Static marker |
|---|---|---|---|---|---|
| OpenSpec | `/opsx:`, `/openspec:`, `/openspec-` | `openspec-*` | `openspec/` | `openspec` | `openspec/` |
| Spec Kit | `/speckit.`, `/speckit-` | `speckit-*`, `speckit.*` | `.specify/`; `specs/<seg>/` where `<seg>` matches `^[0-9]+[a-z]?-` | `specify` | `.specify/` |
| Kiro | none (IDE-driven) | none | `.kiro/specs/` | none | `.kiro/specs/` |
| BMAD | `/bmad-`, `/bmad:` (case-insensitive) | `bmad-*` | `_bmad/`, `_bmad-output/`, `.bmad-core/` | none | `_bmad/`, `.bmad-core/` |

Event rules:
- Path evidence: `PostToolUse` of `Edit`, `MultiEdit`, `Write` or `NotebookEdit` only (the `code_edit` tool set); a
  `PreToolUse` or a failed write is not evidence.
- Bash evidence: the CLI name as a **command word** — the first word of the command or of a segment after `;`, `&&`,
  `||`, `|` or a newline. `grep -i openspec f` is not evidence.
- The **earliest** matching event by `(ts, id)` decides.

### Session columns: `activity.sessions` (+2 columns)

- `sdd_framework sdd_framework`
- `sdd_framework_source text check in ('behavioural','static','static_ambiguous','probe_failed')`
- `check ((sdd_framework is null) = (sdd_framework_source is null or sdd_framework_source = 'probe_failed'))`

Written by the session-signals pass in the same per-session transaction as `store_session_signals`. A re-derivation may
change a **session's** value (more events arrived), subject to the B12 guard. Only the ticket is frozen.

### Ticket columns: `sensei.tracker_tasks` (+3 columns)

- `sdd_framework sdd_framework`
- `sdd_framework_session_id uuid references activity.sessions(id) on delete set null`: provenance
- `sdd_framework_at timestamptz`: when it was pinned

A check keeps `sdd_framework` and `sdd_framework_at` set together; `sdd_framework_session_id` may be `NULL` beside a
value (its session was deleted).

Pinning (`pin_ticket_frameworks(conn, scope)`). The scope's tickets are fixed **first** (B16: `Tracker(id)` → the
tracker's tickets; `Session(id)` → tickets linked to it or provenanced by it, taken `FOR NO KEY UPDATE SKIP LOCKED`, so
the pass never waits on a ticket a tracker apply holds and the two cannot deadlock; the apply pins what was skipped).
Fixing them first keeps a ticket the follow clears in scope for the pin. Then two statements over that list, each
`RETURNING t.id`; the tickets changed are counted once:

1. **Follow** (B14): a ticket whose provenance session is still running takes that session's current value, or is
   cleared when the session's source is no longer `behavioural`/`static` (even with the same framework).
2. **Pin** (B1, B13): each ticket still `NULL` takes its earliest-`started_at` eligible session: source `behavioural` or
   `static` (B11), an R1 candidate of the ticket's tracker (B17), locked `FOR KEY SHARE` first (B18).

```sql
WITH pick AS (
  SELECT DISTINCT ON (l.task_id) l.task_id, s.id AS session_id, s.sdd_framework
    FROM sensei.session_task_links l
    JOIN sensei.tracker_tasks t ON t.id = l.task_id
    JOIN activity.sessions s ON s.id = l.session_id
   WHERE t.sdd_framework IS NULL AND t.id = ANY(<scope's tickets>)
     AND s.sdd_framework_source IN ('behavioural', 'static')                     -- B11
     AND l.session_id IN (SELECT session_id FROM (<candidate_sessions(t.tracker_id)>) c)  -- B17
   ORDER BY l.task_id, s.started_at, s.id),
held AS (SELECT id FROM activity.sessions WHERE id IN (SELECT session_id FROM pick) FOR KEY SHARE)  -- B18
UPDATE sensei.tracker_tasks t
   SET sdd_framework = p.sdd_framework, sdd_framework_session_id = p.session_id, sdd_framework_at = now()
  FROM pick p JOIN held h ON h.id = p.session_id
 WHERE t.id = p.task_id AND t.sdd_framework IS NULL;                           -- B1
```

## Data flow

1. Hook events land, and the session-signals pass selects the session (B9 ordering).
2. The pass loads the repo's `abs_path` (a gone folder row = no repo, B6) and runs `behavioural(events, repo_root)`.
3. No behavioural hit and a repo: `repo_markers` runs on a detached thread (not the runtime's blocking pool, so a probe stuck on a dead mount cannot hold daemon shutdown open) under a **5 s timeout**, at most once per repo
   per pass (memoised; a repo whose probe timed out is not probed again that pass). Then `from_markers`.
4. A probe permission error or timeout → `probe_failed` (B10). The session is derived, not failed.
5. The B12 guard: a static-side `None` / `probe_failed` never replaces an existing non-NULL value.
6. Signals, mark (with `derive_version`) and session columns are written in one transaction. A session deleted
   mid-pass (its row is gone) is skipped, not failed.
7. Then the pass pins the session's tickets in its own transaction (B15); a failure is logged, not a session failure.
8. A tracker apply (sync relink or discovery) writes links in `write_links`, then pins the whole tracker in the same
   tx (B16).

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

1. DDL: `sdd_framework` enum, session columns + check, marks `derive_version`, ticket columns (`make test-template`
   after); regenerate `database/design.dbml`; column comments; `docs/database/activity.md`.
2. `sdlc/framework.rs`: signature table, `behavioural`, `from_markers`, `repo_markers`, with table-driven unit tests.
3. Session-signals pass: projection (`prompt`, `skill`), B9 selection/ordering, probe under timeout + memo, B10/B12,
   one-transaction store. Update `docs/spec/daemon/tracker-sdlc-metrics.md` S2 (marks table, projection list,
   selection rule).
4. `pin_ticket_frameworks` (#391): inside `write_links` (sync relink and discovery), and the pass after each session
   commits (B15); `get_ticket` returns `sdd_framework` (B19). Tests: first detected wins, no overwrite by another
   session, provenance follows until it completes, ambiguous never pinned, non-candidates never pin, NULL stays NULL.
5. Re-point `is_code` and `doc_indexer` at the signature table (#392), with existing tests staying green.

## Personas

No `.sensei/personas/` defined; skipped.

## Out of scope

- sensei's own plan graph as a framework value (B3).
- Tracker write-back of the field (D9, deferred).
- Multiple frameworks per ticket (B1 picks one).
- UI surfacing in the app or Dōjō.
