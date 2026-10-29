---
name: Tracker SDLC metrics
status: reviewed
date: 2026-10-29
analysis: docs/analysis/2026-10-29-sdlc-metrics-extension.md
parent:
  - docs/spec/daemon/tracker-jira-linkage.md (R1, R5, R7, R8)
  - docs/spec/daemon/tracker-metrics-writeback.md (W3, gate 13)
reference: ../ai_sdlc_metrics/docs/metrics.json, metric-inputs.json
---

# Tracker SDLC metrics

## Purpose

Compute the reference set of ticket, sprint and project metrics
(`ai_sdlc_metrics/docs/metrics.json`) for every tracker ticket and sprint, show them on the
Tickets screen, and make them mappable for tracker write-back. They come from two sources:

1. **Session signals**: what happened in the AI sessions linked to a ticket (builds,
   tests, PR creation, holds, rework, review fixes, task ticks). These are derived once per
   session from `activity.assistant_events` and stored.
2. **Ticket history**: what happened to the ticket in the tracker (sprint membership,
   status, story points over time). This is backfilled from the Jira issue changelog by a
   history task that runs after sync.

Metrics are **computed at read** from those two stores plus R7 usage, as R7 itself is.

## Decisions carried in (analysis D1–D11)

- **D1:** shown in screens and write-back.
- **D2:** history backfilled from the tracker.
- **D3:** project-level `hourly_rate`; no per-person rate.
- **D5:** R7's cycle time (`cycle_hours` in write-back, `last_at − first_at`) keeps its
  meaning. The list's edit→PR span is `execution_hours`, and its rework share is
  `post_review_rework_ratio`.
- **D6:** `estimated_points` is the tracker's story points.
- **D7:** review defects are escaped task defects plus review fixes.
- **D8:** every framework, with the catalogue held as data.
- **D9:** the write-back field set is the user's to choose. Nothing is mapped by default.
- **D10:** a finding is one subsequent fix.
- **D11:** escaped defects count across sessions.

## Overrides of parents

| Parent says | Now | Why |
|---|---|---|
| linkage R5: sprints, tasks, memberships and links commit "together or not at all"; a partial sync changes no data rows (wrong gate :749) | **Unchanged for sync.** Ticket history is **not** part of sync: S5's history task runs *after* a successful sync, commits per issue, and never sets `last_error` or touches `last_success_at` | D2 backfills 733+ issues at one GET each, which cannot fit one all-or-nothing transaction under `SYNC_DEADLINE` (`tracker/sync.rs:20`, where a timeout aborts the sync, `sync.rs:118-130`) |
| "usage metrics only" write-back. Each surface is listed below | The local daemon writes **usage and SDLC metrics** (S9's list), and only into fields the user maps. **Rate- and calibration-derived figures are never written.** The comment is unchanged | D1 (user, 2026-10-29: shown in screens **and** write-back) |
| writeback W2: after a sync, `spawn_after_sync` takes the tracker guard (`try_lock`; busy → no pass this round) and runs the write-back pass | Under the **same guard and spawned task**, the history task (S4) runs **after** the pass, or alone when write-back is off. One guard and one spawn, so neither starves the other, and both are serialised with sync | S4 |
| writeback W3: each mapped metric "comes from the R7 `Usage`"; `metric_value(metric, &Usage)`; a kind per metric | `metric_value(metric, &TicketFigures)`, where `TicketFigures { usage: Usage, sdlc: TicketSdlc }`. The 13 existing keys read `usage` exactly as before. The new keys take S9's kinds with W3's rounding and canonical-value rules | S9 |
| analysis §3: `estimated_effort_cost` = R7 `active_secs` × rate | Effort cost, time variance and the rework ratio use `worked_minutes` (S6, slice-based), so a gap counted as a hold is never also billed as work. R7 `active_secs` stays the screen's "Active AI time" | data review 2026-10-29 |

**Write-back scope: the new canonical sentence.** Every copy changes to:

> Tracker reads are read-only: sync never writes sprints, statuses or memberships, and the
> dōjō never writes to a tracker. The one write path is metrics write-back (usage and SDLC
> metrics) by the local daemon, to tickets assigned to the token owner
> (`spec/daemon/tracker-metrics-writeback.md`, `spec/daemon/tracker-sdlc-metrics.md`).

**The rule P-DOCS applies.** The feature is renamed **metrics write-back** wherever it is
called "usage write-back", and every statement that write-back is usage-only is widened to
"usage and SDLC metrics". The two exceptions:
- **Dated history** (an old `decisions.md` row, a `backlog.md` entry, roadmap D3) keeps its
  wording and gains "superseded 2026-10-29 (tracker-sdlc-metrics)".
- **`docs/decisions.md`** gains a new row dated 2026-10-29.

The surfaces are exactly what gate 12's command finds. On 2026-10-29 that is 53 lines in 24
files:
- **App**
  - `app/src/lib/api.ts`:786
  - `app/src/lib/types.ts`:2166
- **Daemon**
  - `crates/senseid/src/api/handlers/scheduled_tasks.rs`:63
  - `crates/senseid/src/db/pg_store/trackers.rs`:36
  - `crates/senseid/src/tracker/writeback.rs`:1, :131
- **DDL and schema design**
  - `database/ddl/enum/sensei/tracker_metric.ddl`:3, :23
  - `database/ddl/enum/sensei/tracker_writeback_status.ddl`:3
  - `database/ddl/table/sensei/sprints.ddl`:21
  - `database/ddl/table/sensei/tracker_fields.ddl`:3, :14
  - `database/ddl/table/sensei/trackers.ddl`:31, :66
  - `database/ddl/table/sensei/tracker_tasks.ddl`:29
  - `database/design.dbml`:2809, :2817, :3644, :4177, :4347
- **Decisions, backlog, design and roadmap**
  - `docs/backlog.md`:119
  - `docs/decisions.md`:15, :20, :25, :96
  - `docs/design/integrations-and-gating-requirements.md`:25, :50, :77, :146, :147, :332,
    :409, :444, :461
  - `docs/roadmap/09-dojo-expansion.md`:14
- **PRD**
  - `docs/prd/modules/04-team/03-dojo-workspace-planning/README.md`:10, :39, :62, :109,
    :162, :201, :227. The wrong gate at :226-227 becomes "anything other than usage or
    SDLC metrics, or a rate- or calibration-derived figure"
  - `docs/prd/modules/04-team/03-dojo-workspace-planning/scenarios/dojo-workspace-planning.feature`:5, :15
  - `docs/prd/modules/04-team/03-dojo-workspace-planning/scenarios/dojo-workspace-planning-gaps.feature`:29
  - `docs/prd/modules/04-team/05-releases-and-dora/README.md`:25
  - `docs/prd/modules/04-team/README.md`:22, :30
- **Specs**
  - `docs/spec/daemon/tracker-jira-linkage.md`:22, :513, :514
  - `docs/spec/dojo/daemon-sync.md`:20
  - `docs/spec/screen/workspace-integrations.md`:14

The excluded files are also updated: `tracker-metrics-writeback.md:30` (the canonical
sentence) changes in place.

## Resolved design

### S1 · Settings (D3)

New table `sensei.workspace_metric_settings`, one row per workspace, created on first `PUT`:

| column | type / check | null means |
|---|---|---|
| `workspace_id` | uuid PK → `sensei.workspaces` on delete cascade | — |
| `hourly_rate` | numeric `check (hourly_rate > 0 and hourly_rate <= 10000)` | `effort_cost`, `rework_cost` null, reason `no_rate` |
| `minutes_per_point` | numeric `check (minutes_per_point > 0 and minutes_per_point <= 10080)` | `expected_minutes`, `time_variance_minutes` null, reason `no_calibration` |
| `idle_threshold_minutes` | integer not null default **15**, `check (between 1 and 240)` | — |
| `modified_at` | timestamptz not null default now() | — |

The upper-bound checks also reject `NaN`, because Postgres orders `NaN` above every number.
A workspace with no row reads as `{hourly_rate: null, minutes_per_point: null,
idle_threshold_minutes: 15}`.

**Routes** (`api/handlers/metric_settings.rs`), both under **`local_caller_guard`**
(`routes.rs:71`, the tracker routes' layer, which refuses a foreign Origin on reads too):

- `GET /api/workspaces/{id}/metric-settings` → 200 with the three values. 404
  `workspace_not_found`.
- `PUT` the same path, with a JSON object holding any subset of the three keys.
  - A number sets the key. `null` clears `hourly_rate` or `minutes_per_point`. `null` for
    `idle_threshold_minutes` → 400 `invalid_setting`.
  - A non-finite value, an out-of-range value, a wrong type or an unknown key → 400
    `invalid_setting`, naming the key.
  - Returns 200 with the full settings.
  - `idle_threshold_minutes` is applied **at read** (S6), so changing it re-derives nothing.

### S2 · Session signals (derived once, stored)

*Amended 2026-10-29 before F2 (amendment A3, pre-implementation swarm):*
- *slices are cut at a fixed 1-minute base and merged by the workspace threshold at read, because a session can belong to several workspaces, or none;*
- *a failing session is recorded and skipped, never blocking the queue;*
- *staleness uses `max(id)` and `count(*)` over a new `(session_id, id)` index;*
- *events are read projected and ordered by `(ts, id)`;*
- *an Agent review's text and time come from its `SubagentHandback`.*

**Event time.** Every derived time comes from `assistant_events.ts`
(`to_timestamp(ts / 1000.0)`, "when the event fired"), and events are ordered by `(ts, id)`.
`created_at` is the insert time and is never used. A drained event is inserted hours later,
but its `ts` is the event's own. Known limit, as documented at `capture_drain.rs:24-27`:
legacy dead-letter lines without a client `ts` import with `ts` = drain time. A `ts` outside
`[2000-01-01, now + 1 day]` makes that session's derivation fail (recorded, see the
incremental pass). It never fails the pass.

**Tables.**

`activity.session_signal_marks`: one row per derived (or failed) session.

| column | meaning |
|---|---|
| `session_id` uuid PK → `activity.sessions` on delete cascade | |
| `through_event_id` bigint not null | highest `assistant_events.id` read; 0 for a session with no events |
| `event_count` integer not null | events read. A lower id committing after a higher one raises the count, not the max |
| `catalogue_hash` text not null | S3 catalogue hash at derivation |
| `error` text | NULL when derived. Otherwise why derivation failed, and the session's signals are absent |
| `derived_at` timestamptz not null default now() | |

`activity.session_signals`: PK `(session_id, seq)`.

| column | meaning |
|---|---|
| `session_id` uuid → `activity.sessions` on delete cascade | |
| `seq` integer | order within the session |
| `kind` `sensei.session_signal_kind` | below |
| `at` timestamptz not null | event time (`ts`) |
| `until` timestamptz | `slice` only: last event time |
| `category` text | run signals: `build`, `typecheck`, `test`, `e2e` |
| `reason` `sensei.hold_reason` | `slice` only |
| `ref` text | `review`: reviewer name. `review_fix`: the matched token from the review text (repo-relative, e.g. `crates/cli/src/lib.rs`). `task_tick`: `path:line` or `plan:<task id>` |

`sensei.session_signal_kind` takes these values:
- `slice`, `prompt`, `code_edit`
- `build_ok`, `build_fail`, `test_ok`, `test_fail`
- `run_unknown`, `unclassified_fail`, `unmatched_call`
- `pr_created`, `pushed`, `task_tick`, `review`, `review_fix`

`sensei.hold_reason` takes these values: `build_failure`, `test_failure`, `permission_wait`,
`interrupted`, `session_ended`, `unknown`.

New index: `activity.assistant_events (session_id, id)`, used for staleness and for the
read.

**Reading a session's events.** `assistant_events.session_id = sessions.client_session_id`
(the idiom across `pg_store`). Sessions with a NULL `client_session_id` are never selected:
they have no events.

The new reader `session_events_for_signals(client_session_id)` orders by `(ts, id)` and
projects only what derivation reads:
- `id`, `event_type`, `tool_name`, `ts`, `success`
- `tool_input`: `command`, `file_path`, `notebook_path`, `subagent_type`, `state`, `message`
- `tool_response`: `type`, `agentId`
- `old_string` / `new_string` / `content` / `originalFile`, only for paths ending `tasks.md`
- the output text `stdout + stderr` (or `error` for a `PostToolUseFailure`), capped at its
  **last 64 KiB**, where result lines are
- `agent_id`, `notification_type`, `is_interrupt`

The existing `get_hook_events_for_session_with_id` (whole payloads, `ts` order only) is not
used.

**Derivation**: the pure function `derive_signals(events, catalogue, folder_commands) ->
Result<Vec<Signal>, String>` in `crates/senseid/src/sdlc/signals.rs`. No threshold: slices
are cut at the fixed base.

| signal | rule |
|---|---|
| `prompt` | each `UserPromptSubmit` |
| `code_edit` | each `PostToolUse` of `Edit`, `MultiEdit` or `Write` (path `tool_input.file_path`) or `NotebookEdit` (path `tool_input.notebook_path`) whose path is **code**: no path segment in {`docs`, `doc`, `spec`, `specs`, `openspec`, `.sensei`, `.claude`}, and extension not `.md`/`.mdx` |
| run signals | each Bash `PostToolUse`/`PostToolUseFailure`, classified by S3 (`classify(command, output, exit, catalogue, folder_commands)`) into `{category}_ok`, `{category}_fail`, `run_unknown` or `unclassified_fail`. `build`/`typecheck` → `build_*`; `test`/`e2e` → `test_*` |
| `unmatched_call` | a Bash `PreToolUse` whose `tool_use_id` has no `PostToolUse`/`PostToolUseFailure` in the session. `category` is set from S3's `run_categories(command, …)` (the same tokenizer and recognition, no outcome) when it names a run. A plugin without `PostToolUseFailure` (C1) loses failures exactly this way, so a lost failure is **counted**, never read as 0 |
| `pr_created` | a successful Bash whose command contains `gh pr create` or `glab mr create` |
| `pushed` | a successful Bash whose command contains `git push` |
| `task_tick` | (a) an `Edit` on a path ending `tasks.md`: one per line that is `- [ ] …` in `old_string` and `- [x] …` in `new_string`. (b) a `Write` with `tool_response.type = "update"` on such a path: one per line that is unchecked in `tool_response.originalFile` and checked in `tool_input.content` (matched on the text after the box). (c) `mcp__plugin_sensei_sensei__update_task_status` with `tool_input.state = "done"` (`crates/mcp/src/lib.rs:878`) |
| `review` | (a) an `Agent` review: the `PostToolUse` of `Agent` whose `tool_input.subagent_type` (after any `plugin:` prefix) contains `review`, `auditor`, `verifier` or `hunter` is a launch receipt (`tool_response.status = async_launched`, no text). The review is the matching `SubagentHandback` `PostToolUse` (`payload.agent_id` = the launch's `tool_response.agentId`), at **its** time, with text `tool_input.message`; failing that, the matching `SubagentStop`'s `last_assistant_message`. A launch with neither yields no review. (b) a Bash whose first word after S3 prefix-stripping is in {`codex`, `gemini`, `aider`, `cursor-agent`, `opencode`, `claude`}, with its output text. (c) any `mcp__*__consensus` tool, with its output text |
| `review_fix` (D10) | the first `code_edit` after a `review` (its text's arrival), before the session's next `review`, of a file **named** in that review's text. Path tokens are taken from the text: runs of non-space characters containing `/`. Strip surrounding quotes and backticks and trailing `.,;:)]`, then strip a `:N`, `:N-M` or `:N:M` suffix. Expand a brace list (`api/{server.rs,routes.rs}` gives two tokens). Keep tokens ending in `.<ext>`. Over 66 real reviewer responses, this keeps the 73 tokens the plain rule missed (`main.rs.`, `server.rs,`). The edited path is named when a token with ≥2 segments equals a **segment-aligned suffix** of it. `crates/cli/src/lib.rs` names `/repo/crates/cli/src/lib.rs` but not `/repo/crates/senseid/src/lib.rs`. A bare basename never matches (`mod.rs` ×42, `+page.svelte` ×82 in this repo). One per distinct file per review |
| `slice` | **base slices**: the session's events cut where consecutive `ts` differ by more than **1 minute** (`BASE_SLICE_GAP`). `at` is the first event, `until` the last. The workspace threshold T (1–240 min) is applied at read (S6) by merging consecutive base slices whose gap ≤ T. Because T ≥ 1 minute, that gives exactly the slices a cut at T would |

**Slice reason** (per base slice; a merged slice takes its last base slice's reason):
- The slice's last event is `SessionEnd` → `session_ended`.
- Otherwise drop trailing `Stop`, `SubagentStop`, `PreCompact` and `UserPromptSubmit` events
  (the turn's tail), then take the last remaining event:
  - a `PostToolUseFailure` with `is_interrupt = true` → `interrupted`
  - a `build_fail` run → `build_failure`
  - a `test_fail` run → `test_failure`
  - `Notification` with `notification_type = permission_prompt` → `permission_wait`
  - anything else → `unknown`

**Incremental pass.** The task `derive_session_signals` runs every 10 minutes. It is registered
in `SCHEDULABLE` (`tasks/schedule.rs:23`), `DESCRIPTIONS` (`api/handlers/scheduled_tasks.rs`)
and `database/import/staging/schedules.jsonl`, and spawned in `api/server.rs` like
`capture_drain`.

1. Read the catalogue once (`command_catalogue()`). If it fails (a read error or a row that
   doesn't compile, named), the **pass** fails, loudly, and no session is touched. A
   catalogue is configuration, and deriving with part of it would be a quiet wrong answer.
2. Select up to 500 sessions with a non-NULL `client_session_id`, oldest `started_at` first,
   where:
   - there is no mark; or
   - the events' `max(id)` ≠ `through_event_id` or `count(*)` ≠ `event_count`; or
   - `catalogue_hash` ≠ the current hash.

   A mark with an `error` is selected again only when one of these inputs changed. A
   failed session is not retried until then.
3. Per session, in one transaction: read its events, derive, delete its signals, insert the
   new ones, and upsert the mark with the `max(id)` and `count` **actually read**. A
   derivation error, an out-of-range `ts`, or a write error rolls that session back. Its
   mark is then upserted alone, in a new statement, with `error` set and the inputs it
   failed on, and the pass **continues** with the next session. The pass reports the count
   derived and the count failed. A failed session's figures read as `not_derived` (S6).

### S3 · Build/test classifier (D8)

*Amended 2026-10-29 at build start, after the F1 pre-implementation swarm (amendment A1).
The previous version attributed markers by category alone. It gave two `test_fail`s for one
passing and one failing `cargo test` (row 1735), and missed compile errors inside `cargo
test` (9 rows), loop bodies (9 rows), `bun run check` (38 rows), `make test-template` as a
non-test (12 rows) and `pnpm --filter` (33 rows). Amendment A2 (F1 post-implementation
swarm): the catalogue is consulted before folder commands, script names are categorised
whole, prefix commands skip their option arguments, and `&`, `<<<` and `case` arms are
handled.*

**The catalogue is data**, in two tables seeded from staging jsonl by
`staging.import_command_patterns` and `staging.import_command_markers` (the same pattern as
`reason_codes`). Adding a toolchain or a marker is a jsonl row, not a code change.

`sensei.command_patterns`, PK `(toolchain, pattern)`:

| column | meaning |
|---|---|
| `toolchain` | e.g. `cargo`, `vitest`, `make`. `wrapper` = true for script and target runners |
| `pattern` | regex anchored at the start of a stage (after S3.1 stripping). Workspace options are allowed between the tool and its verb (`pnpm --filter a run x`, `-F`, `-C`, `--dir`, `-r`, `-w`) |
| `category` | `build`, `typecheck`, `test`, `e2e`, or `none` (a known command that isn't a run, e.g. `cargo clippy`, `make test-template`). NULL = derive from the pattern's first capture group (S3.2) |
| `wrapper` | boolean. The stage runs a named script or target whose toolchain is unknown (`bun run <name>`, `make <target>`, `just <target>`) |
| `compiles` | boolean. The stage compiles before it runs (`cargo test`, `cargo clippy`, `go test`, `dotnet test`, `mvn test`) |
| `priority` | integer, higher first. A specific row (`make test-template` → `none`) beats a general one (`make <target>`) |
| `modified_at` | import guard |

`sensei.command_markers`, PK `(toolchain, category, kind, marker)`. `kind` is `fail` or
`ok`, and `marker` is a literal substring. Seed examples:
- cargo/build: fail `error: could not compile`, `error[E`; ok `Finished `
- cargo/test: fail `test result: FAILED`; ok `test result: ok`
- vitest/test: fail ` failed |`, `Tests  ` with ` failed`; ok ` passed (`
- svelte-check/typecheck: fail `svelte-check found ` (with a nonzero count); ok `svelte-check found 0 errors`
- tsc/typecheck: fail `error TS`
- pytest/test: fail ` failed`, `= FAILURES =`; ok ` passed in `
- go/test: fail `FAIL\t`, `--- FAIL`; ok `ok  \t`
- go/build: fail `# ` package-error header

A marker may need a numeric guard ("found 0 errors" vs "found 2 errors"). That is
expressed as a regex marker (`kind` = `fail_re` / `ok_re`). Plain substrings stay the norm.

`catalogue_hash` = SHA-256 (hex) computed in Rust over the rows `command_catalogue()` actually
read, in key order, so the hash always describes the compiled rules (A3). Any catalogue
change re-derives every session (S2).

**The seed covers at least:**
- cargo `build|check|b|c` (build), `test|t|nextest` (test, compiles), `clippy` (none, compiles), `fmt` (none)
- go, dotnet, mvn/`./mvnw`, gradle/`./gradlew`
- npm/pnpm/yarn/bun `run <name>` and `test` (wrapper), `bun test`
- npx/bunx/`pnpm exec` with `tsc`, `vitest`, `jest`, `playwright`, `svelte-check`
- make/just `<target>` (wrapper) and bare `make`, with `none` rows for `make test-template`
  and other known non-runs in this repo's Makefile
- pytest/`python -m pytest`/`uv run pytest`/tox/nox
- tsc, svelte-check, mypy, pyright
- cmake `--build`, ninja, msbuild, xcodebuild
- swift, mix, rspec/`bundle exec rspec`/`rake test`
- vitest, jest, `playwright test`

**`classify(command, output_text, exit, catalogue, folder_commands)` → `Vec<RunSignal>`**
(`exit` is `PostToolUse` or `PostToolUseFailure`):

1. **Tokenize** with shell quoting rules:
   - Single and double quotes and backslash escapes make one word.
   - A heredoc body (`<<EOF … EOF`, `<<'EOF'`, `<<-EOF`) is skipped.
   - `$(…)` and backticks are opaque words.

   Split on the operators `&&`, `||`, `;`, `&`, newline (list elements, each keeping the
   operator that **enters** it) and `|` (pipeline stages). At each stage's start, strip:
   - env assignments (`X=1`)
   - prefix commands with their options, including those that take an argument:
     `timeout [-k n] [-s sig] <duration>`, `nice [-n n]`, `sudo [-u user]`, `time`, `exec`,
     `command`
   - grouping and keyword tokens: `!`, `{`, `(`, `do`, `then`, `else`, `elif`, `if`, `while`,
     `until`

   `<<<` is a here-string (a redirection), not a heredoc. An unmatched `)` ends a `case`
   arm's pattern. An element ended by `&` runs in the background, so the shell's exit
   is never its (step 4).

   A stage that is then `cd <dir>`, or that starts with `for`, `while`, `until`, `if`,
   `case`, `done`, `fi`, `esac`, `}` or `)`, is a **no-op stage** (not a command). The words
   of `bash -c`, `sh -c` and `eval` are not analysed (known limit).
2. **Classify each stage.**
   - First, `command_patterns`, by `priority` descending, then pattern length descending.
     The first match decides the toolchain, `wrapper` and `compiles`.
   - An explicit row category decides the category. For a **wrapper** row or a NULL
     category, a repository folder command (`sensei.folder_commands` for the session's
     repository folder and its descendants, the longest `command_line` that prefixes the
     stage on a word boundary, non-NULL `category` only) decides it. Otherwise the
     category is `categorise` of the captured script or target name, **whole**
     (`type-check` → typecheck, `test:e2e` → e2e, `test-fast` → test). A higher-priority
     `none` row catches non-runs such as `make test-template`.
   - A stage no pattern matches takes a matching folder command's category, as a wrapper
     of unknown toolchain.
   - `adapters::manifest::command_category::categorise` gains exact `check` and `check:*`
     → `typecheck`. This is a shared change, and the indexer's `folder_commands` writer
     benefits.
   - Categories other than `build`, `typecheck`, `test` and `e2e` (lint, format, docs, run,
     dev, start, bench, none) are **not runs**: no signal. But a `compiles` stage still
     **owns** build-fail markers (step 3).
3. **Outcome from markers, by ownership.** Find every catalogue marker in the output. A
   marker `(T, C, kind)` is **owned** by:
   - the stages with toolchain `T` and category `C`; or
   - for `C = build` and kind `fail`, also every `compiles` stage of toolchain `T`, whatever
     its category, `none` included; or
   - when no stage has toolchain `T`, the **wrapper** stages of category `C`.

   A marker with **exactly one owner** is attributed to it. A marker with several owners
   is **ambiguous** for each of them. Per classified stage, the first rule that applies:
   1. an attributed build-fail marker on a `compiles` stage → `build_fail` (a compile error,
      whichever command compiled)
   2. an attributed fail marker of its category → `{category}_fail`
   3. it co-owns an ambiguous **fail** marker → `run_unknown`. An ambiguous failure
      outranks this stage's own ok marker
   4. an attributed ok marker, on `PostToolUse` → `{category}_ok`. On `PostToolUseFailure`,
      an ok marker never yields `_ok`
   5. it co-owns an ambiguous ok marker → `run_unknown`
   6. otherwise → step 4

   A non-run stage (`none`, lint, …) emits nothing, but its ownership still counts.
4. **Outcome from the exit**, only when it is attributable:
   - **Failure** (`PostToolUseFailure`): the command, without no-op stages, is exactly one
     element of one stage, and that stage is classified → `_fail`.
   - **Success** (`PostToolUse`): for each classified stage, `_ok` only when it is the last
     stage of its element, its element was not entered by `||`, and every later element is
     entered by `&&`, and its element is not backgrounded (`&`).
   - Anything else → `run_unknown`.
5. A `PostToolUseFailure` with no classified stage → `unclassified_fail`.

### S4 · Ticket history (D2)

**Columns on `sensei.tracker_tasks`:**
- `issue_created_at`, `issue_updated_at` timestamptz
- `points` numeric (the estimation field's current value)
- `status_category` text (`new`, `indeterminate`, `done`)
- `history_through` timestamptz (the `issue_updated_at` the changelog was last read through)

**Columns on `sensei.trackers`:**
- `estimation_field` text: the board's field id, or NULL when the board estimates by count
  or not at all
- `history_error` text
- `history_at` timestamptz

**New table `sensei.tracker_task_changes`:** PK `(task_id, change_id, field)`, insert-only.

| column | meaning |
|---|---|
| `task_id` | uuid → `tracker_tasks` on delete cascade |
| `change_id` | text, the changelog history id |
| `field` | `sensei.tracked_change_field`: `sprint`, `status` or `estimate` |
| `at` | timestamptz not null |
| `from_value`, `to_value` | text |

**New table `sensei.tracker_statuses`:** `(tracker_id, status_id)` PK, `name`, `category`.

**Sync additions** (inside the existing R5 fetch and apply, all-or-nothing as before):
1. `GET /rest/agile/1.0/board/{board}/configuration` → `estimation.field.fieldId` when
   `estimation.type = "field"`. A value failing `field_number` (`jira.rs:678`) is stored as
   NULL.
2. `GET /rest/api/3/status` → `tracker_statuses` (`statusCategory.key`).
3. `ISSUE_FIELDS` and `SEARCH_FIELDS` gain `created`, `updated` and the estimation field.
   `status_category` is read from `fields.status.statusCategory.key`.

**History task** `tracker_history` (`tracker/history.rs`). It is not a separate spawn:
`writeback::spawn_after_sync` keeps its one `try_lock(tracker_id)`, runs the write-back pass
when `writeback_enabled`, then `history::run(…, HISTORY_DEADLINE, &guard)`, then drops the
guard. Because sync holds the same per-tracker lock, history never runs concurrently with a
sync's apply.
- Selects non-removed tasks with `history_through IS NULL OR issue_updated_at > history_through`,
  oldest `issue_updated_at` first.
- Pages `GET /rest/api/3/issue/{id}/changelog?startAt=&maxResults=100` to `isLast`. The
  path is built by `issue_path` (`jira.rs:686`, guarded by `is_jira_id`).
- Keeps items with `field = "Sprint"`, `fieldId = "status"`, or `fieldId = estimation_field`.
- Per issue, inserts the rows (skipping existing ones) and sets `history_through =
  issue_updated_at` in one transaction.
- Its own deadline is `HISTORY_DEADLINE = 5 min`. On the deadline it stops cleanly, sets
  `history_at` and leaves `history_error` NULL. The rest continues next run.
- A transport or decode error sets `history_error = <TrackerError code>` and stops. It
  never touches `last_error` or `last_success_at`.
- Uses the GET-only transport (writeback gate 13). The board issue `expand` is documented as
  unused, the JQL `expand=changelog` returns only "recent updates", and bulkfetch is a POST.

### S5 · Sprint replay (pure, at read)

`sdlc/sprint_replay.rs`: `replay(sprint, sprints_of_tracker, tasks, changes, statuses) -> SprintFacts`.

- **Window.** Reuse `tracker::usage::sprint_window` (R7): `S = start_at`, and
  `E = coalesce(complete_at, end_at)` for closed or `now()` for active. Its reasons apply
  as they are (`not_started`, `no_dates`).
- **Membership at t.**
  - The initial state is the `from_value` of the issue's earliest `sprint` change, or its
    current `tracker_task_sprints` membership when it has none.
  - Apply `sprint` changes with `at ≤ t`.
  - An issue with `issue_created_at > t` is not a member.
- **Status at t** is replayed the same way over `status`, starting from the current status
  when there is no change. **Done** means category `done`. **Points at t** are replayed the
  same way over `estimate`.
- **The sets:**
  - `committed`: members at `S`.
  - `added`: not in `committed`, and a member at `E` or at any of its `sprint` change times
    or its `issue_created_at` in `(S, E]`. An issue created directly in the sprint has no
    change row.
  - `removed`: members at some time in `[S, E)` and not members at `E`.
  - `completed_committed` and `completed_added`: members at `E` and done at `E`.
  - `carried_over`: members at `E`, not done at `E`, and members at some time after `E` of a
    sprint of the same tracker with a **higher `seq`**. Older closed sprints stay in Jira's
    field forever and must not count.
- **Points sums.** Each set has `{set}_points` (the sum of parseable points at the relevant
  time: `S` for committed, `E` for the others) and `{set}_points_n` (the count summed).
  Unparseable values are excluded and counted in `points_unparsed`. A sum over
  `_n = 0` is **null**, as in R7, never 0.

### S6 · Metrics (at read)

All figures live in an `sdlc` object. Each key is a number or `null`, and
`reasons: {key: code}` explains every null. Reason codes are rows in
`database/import/staging/reason_codes.jsonl`, domain `sdlc_metrics`, rendered through the
existing reason lookup (`pg_store/reasons.rs`). A `0` means the inputs exist and the count
is zero.

**Ticket.** The ticket's sessions are R7's ticket `S` (linked R1 candidates). Their signals
are merged by `at` (D11). **Slices at read:** each session's base slices are merged where
consecutive base slices are ≤ T apart, with T = the tracker's workspace
`idle_threshold_minutes` (S1; a tracker belongs to exactly one workspace). Below, "slices"
means these merged slices.

**Derivation must be current.** Every session in `S` needs a mark with no `error`, whose
`through_event_id` and `event_count` equal its events' `max(id)` and `count(*)`, and whose
catalogue hash is current. Otherwise every
signal-based figure (everything below except `ai_token_cost`, `estimated_points` and
`expected_minutes`) is null with reason `not_derived`. A not-yet-derived session never reads
as zeros.

| key | definition | null when (reason) |
|---|---|---|
| `worked_minutes` | Σ over the union of the sessions' slices of `until − at`. This is the SDLC base, consistent with holds | no slices (`no_sessions`) |
| `prompts` | count `prompt` | no sessions |
| `ai_token_cost` | R7 `api_cost` | as R7 |
| `cost_per_prompt_cycle` | `api_cost / prompts` | `api_cost_n < sessions` or `api_cost_partial > 0` (`partial_cost`); prompts 0 (`no_prompts`) |
| `escaped_task_defects` | number of `task_tick`s followed, before the next `task_tick` or `pr_created`, by ≥1 `build_fail`/`test_fail` and later by an `_ok` of that failure's category. The escape's category is the category of the **first** fail after the tick; one escape per tick at most. Its **trigger fails** are the fails **of that category only**, from the tick up to the first `_ok` of that category. A fail of another category in between stays an ordinary defect | — |
| `compile_defects` | count `build_fail` that are not trigger fails | — |
| `test_defects` | count `test_fail` that are not trigger fails | — |
| `review_fixes` | count `review_fix` | — |
| `review_defects` | `review_fixes + escaped_task_defects` (D7) | — |
| `total_defects` | `compile_defects + test_defects + review_defects` (each failure is counted once) | — |
| `testing_efficiency` | `(compile + test) / total × 100` | total 0 (`no_defects`) |
| `review_efficiency` | `review_defects / total × 100` | total 0 |
| `build_runs` | `build_ok + build_fail` | — |
| `runs_unknown` | `run_unknown` + classified `unmatched_call` | — |
| `unclassified_failures` | `unclassified_fail` + unclassified `unmatched_call` | — |
| `build_failure_rate` | `(build_fail) / build_runs × 100` | build_runs 0 (`no_runs`) |
| `mean_build_recovery_minutes` | for each `build_fail` followed by a `build_ok` **of the same category**: the gap. Σ / number of such fails (the reference sums one gap per failed build). `build_unrecovered` = the rest | none recovered (`no_recovery`) |
| `work_started_at` | min `code_edit.at` | no code edit (`not_started`) |
| `submitted_at` | min `pr_created.at` with `at ≥ work_started_at`; else min such `pushed.at`. `submitted_via` = `pr` or `push` | none after work start (`not_submitted`) |
| `execution_hours` (D5) | `submitted_at − work_started_at` (≥ 0 by construction) | either null |
| `lead_hours` | `submitted_at − issue_created_at` | either null; `submitted_at < issue_created_at` (`created_after_submit`; tickets are often filed after the work) |
| `on_hold_minutes`, `on_hold_count`, `on_hold_by_reason` | take the union of the sessions' slices, clipped to `[work_started_at, submitted_at]`. Each gap **longer than `idle_threshold_minutes`** between consecutive union intervals is a hold, with reason = the slice ending before the gap. Shorter gaps between back-to-back sessions are not holds | window null |
| `on_hold_ratio` | `on_hold_minutes / (submitted − work_started, minutes) × 100` | window null or 0 (`no_window`) |
| `rework_minutes` | for each slice with a `code_edit` at ≥ `submitted_at`: `until − max(at, first such code_edit.at)` | `submitted_at` null |
| `rework_cycles` | `pushed` signals after `submitted_at` with ≥1 `code_edit` since the previous push (or since `submitted_at`) | `submitted_at` null |
| `post_review_rework_ratio` (D5) | `rework_minutes / worked_minutes × 100` | either null or active 0 |
| `estimated_points` | `tracker_tasks.points` | `estimation_field` NULL (`no_estimation_field`) or no value (`not_estimated`) |
| `expected_minutes` | `estimated_points × minutes_per_point` | `no_calibration` / points null |
| `time_variance_minutes` | `worked_minutes − expected_minutes` | either null |
| `effort_cost` | `hourly_rate × worked_minutes / 60` | `no_rate` |
| `rework_cost` | `hourly_rate × rework_minutes / 60` | `no_rate` / rework null |

Where `runs_unknown > 0`, the defect counts are lower bounds. The screen shows them as
"≥ n" (S8).

**Sprint**, from S5. Counts are tickets, and points are as defined there.

| key | formula | null when |
|---|---|---|
| `commitment_reliability` | `completed_committed / committed × 100` | committed 0 (`no_commitment`), or a window reason |
| `total_completion` | `(completed_committed + completed_added) / committed × 100` | same |
| `scope_change` | `(added + removed) / committed × 100` | same |
| `carry_over_rate` | `carried_over / committed × 100` | same |
| `unplanned_work_ratio` | `completed_added / (completed_committed + completed_added) × 100` | denominator 0 (`nothing_completed`) |
| `velocity` | `completed_committed_points + completed_added_points` (a null part counts as absent; null when both are null) | `no_estimation_field`; both sums null (`not_estimated`) |
| `points_commitment_reliability` | `completed_committed_points / committed_points × 100` | `no_estimation_field`; either sum null (`not_estimated`); committed_points 0 |

**Project (tracker).**

| key | formula | null when |
|---|---|---|
| `predictability` | population stddev of `commitment_reliability` over the tracker's last **6** closed sprints (by `seq`) with a non-null value | fewer than 2 (`too_few_sprints`) |
| `tokens_per_ticket` | over the **distinct union** `U` of the tracker's tickets' candidate sessions (a shared session counts once): Σ (`tokens_fresh + cache_read + cache_write + tokens_out`) / the number of tickets with a non-empty `S`. Null when any session in `U` lacks one of the four fields (`partial_tokens`, with `sessions_without_tokens` beside it) | no such ticket (`no_usage`) |
| `prompts_per_ticket` | Σ `prompt` signals over `U` / tickets with a non-empty `S` | none (`no_usage`); any session in `U` not derived (`not_derived`) |

**Out of v1** (analysis §5): `actual_points`, `points_variance`.

### S7 · API

All on the existing R8 tracker routes (`local_caller_guard`). The only new routes are S1's.

| route | gains |
|---|---|
| `GET /api/workspaces/{id}/trackers/{tracker}/tasks` (`routes.rs:64`) | each row: `sdlc` (S6 ticket) |
| `GET /api/workspaces/{id}/tracker-tasks/{key}` (`routes.rs:70`) | `sdlc` (S6 ticket) |
| `GET /api/workspaces/{id}/trackers/{tracker}/sprints` (`routes.rs:63`) | each sprint: `sdlc` (S6 sprint) |
| `GET /api/workspaces/{id}/trackers` (`routes.rs:50-52`, the list the Tickets screen reads boards from) and the `PATCH …/trackers/{tracker}` response, which has the list item's shape | each tracker: `sdlc_project` (S6 project) |

### S8 · Screens

The screen spec `docs/spec/screen/workspace-tickets.md` gains these elements, with done and
wrong gates, in this slice (P-SCREEN):
- **Ticket detail → "SDLC" group**, in four rows: cost (`ai_token_cost`,
  `cost_per_prompt_cycle`, `effort_cost`), defects (`compile`/`test`/`review`/`escaped`,
  shown "≥ n" when `runs_unknown > 0`, followed by "n runs with unknown outcome"), timing
  (`execution_hours`, `lead_hours`, `on_hold_*`), and rework. Each null renders its reason
  code's `summary`.
- **Lane header, when the lane's sprint filter selects one sprint** (a lane is a board,
  `workspace-tickets.md:23-27`): that sprint's S6 sprint row. With the filter on "All" or
  "Backlog", the board's `sdlc_project` row.
- **Workspace metrics route:** a settings panel for S1's three values, which calls `PUT`
  and shows a 400's named key.

### S9 · Write-back (D1, D9)

`sensei.tracker_metric` and `TrackerMetric` gain exactly these keys, appended after the 13
existing ones in this order. Each value is the S6 figure. **A ticket whose figures carry
`not_derived` is not ready: write-back skips its field writes this pass** (no request, and its
`writeback_at` stays as it was), so a derivation lag never erases a value in Jira (A3).
Otherwise a null S6 figure is
`FieldValue::Null`, as W3 does for a `None`.

| metric | kind | rounding (W3 rules: half away from zero, division last, `-0.0` → `0.0`) |
|---|---|---|
| `compile_defects`, `test_defects`, `review_fixes`, `escaped_task_defects`, `review_defects`, `total_defects`, `build_runs`, `rework_cycles`, `prompts` | integer | — |
| `testing_efficiency`, `review_efficiency`, `build_failure_rate`, `on_hold_ratio`, `post_review_rework_ratio` | decimal | percent: the ratio is multiplied by 100 first, then `(x * 100.0).round() / 100.0` (2 dp) |
| `mean_build_recovery_minutes`, `on_hold_minutes`, `rework_minutes`, `worked_minutes` | decimal | minutes, `(secs * 100.0 / 60.0).round() / 100.0` |
| `execution_hours`, `lead_hours` | decimal | hours, as W3's `cycle_hours`; a negative span is null |
| `cost_per_prompt_cycle` | decimal | as W3's costs |

**Never mappable:** `effort_cost`, `rework_cost`, `expected_minutes` and
`time_variance_minutes` (rate- or
calibration-derived), and `estimated_points` (Jira's own field). A null writes the field's
null, as the existing keys do (`FieldValue::Null`). Nothing is mapped by default (D9). The
comment `render` is unchanged.

## Done gate

1. **Classifier.**
   - **Pure:** `cargo test -p senseid sdlc::classify` passes, with one case per seed
     toolchain and these outcomes:
     - `cargo test 2>&1 | tail -3` with output `test result: FAILED` → `test_fail`;
       with no marker → `run_unknown`
     - `cargo test 2>&1 | tail -3` with `error: could not compile` → `build_fail`
     - `cargo test > l 2>&1; echo $?` (PostToolUse, no marker) → `run_unknown`
     - `cargo build || true` → `run_unknown`
     - `cargo fmt --check && cargo test`, PostToolUseFailure, no marker → `run_unknown`,
       never `test_fail`
     - failing `cargo build && cargo test`, no marker → `run_unknown` ×2
     - passing `cargo build && cargo test` → `build_ok` + `test_ok`
     - `cargo build …; cargo test …` with `Finished` and `test result: FAILED` →
       `build_ok` + `test_fail`; with `test result: FAILED` and no build marker →
       build `run_unknown` + `test_fail`, never `build_fail`
     - row 1735's shape (two `cargo test` stages, outputs `ok` and `FAILED`) →
       `run_unknown` ×2, never two `test_fail`
     - row 3508's shape (`cargo test …; bun run test:unit …`, `test result: ok` and vitest
       ` 1 failed |`) → `test_ok` (cargo) + `test_fail` (bun)
     - `for i in 1 2; do cargo test 2>&1 | grep "test result"; done` with
       `test result: FAILED` → `test_fail`
     - `make test-template 2>&1 | grep stamped; cargo test … | grep …` with
       `test result: FAILED` → exactly one `test_fail`
     - `cargo build 2>&1 | tail -1; cargo clippy -- -D warnings` with `Finished` and
       `could not compile` → build `run_unknown` (clippy co-owns the compile marker), never
       `build_fail`, and no signal for clippy
     - `bun run check` (no folder command) with `svelte-check found 2 errors` →
       `build_fail` (typecheck)
     - `pnpm --filter a run typecheck 2>&1 | tail -3`, no marker → `run_unknown`
       (typecheck)
     - `rg 'cargo test|make test' f.yml | head` and a PR-body heredoc mentioning
       `cargo test` → no signals
     - `PostToolUseFailure` of `cargo test` whose output has only `Finished` → not `test_ok`
     - a folder command with category `typecheck` beats the table; one with NULL category
       falls through
     - `cargo bench` / `npm run bench` → no run signal
   - `categorise("check")` and `categorise("check:all")` → `typecheck`, and the existing
     `categorise` tests still pass.
   - **DB:** the `pg_store` test `command_catalogue_import` inserts a staging row and calls
     the import. The new toolchain classifies through `command_catalogue()`, and
     `catalogue_hash` changes.
2. **Session fixture A.** Events by `ts` (min): 0 Edit `src/a.rs`; 1 failing `cargo build`
   (PostToolUseFailure); 2 Stop; 30 Edit `src/a.rs`; 31 passing `cargo build`; 32
   `gh pr create`; 50 Edit `src/a.rs`; 52 `git push`; 53 SessionEnd. Threshold 15.
   - Signals: `code_edit`@0, `build_fail`@1, `code_edit`@30, `build_ok`@31, `pr_created`@32,
     `code_edit`@50, `pushed`@52.
   - Base slices (1 min): [0–2, `build_failure`], [30–32, `unknown`], [50, `unknown`],
     [52–53, `session_ended`]. Merged at T = 15: [0–2, `build_failure`], [30–32, `unknown`],
     [50–53, `session_ended`].
   - Ticket figures:
     - defects and builds: `compile_defects=1`, `build_runs=2`, `build_failure_rate=50`,
       `mean_build_recovery_minutes=30`
     - timing: `work_started_at=0`, `submitted_at=32`, `execution_hours=32/60`,
       `on_hold_count=1`, `on_hold_minutes=28`, `on_hold_by_reason={build_failure:28}`
     - rework: `rework_minutes=3`, `rework_cycles=1`
     - `worked_minutes=7`
   - The same events inserted with `created_at` = now + 3 h give identical signals.
   - At T = 1 the holds are every gap over 1 minute inside [0, 32]: `on_hold_count=1`
     ([2, 30]). At T = 30 the first two slices merge, so `on_hold_count=0`. The marks are
     unchanged between the two reads.
2b. **Pass resilience (A3, DB).**
   - The oldest selected session has an event with `ts` = 9e18, and a newer one is valid.
     After one pass the newer one has a mark with no `error`. The bad one has a mark with
     `error` naming the `ts`, and a second pass doesn't select it again.
   - A session derived at `count` N gains an event with a **lower** id than its
     `through_event_id`. The next pass selects it (count ≠ N), and its signals include that
     event.
   - Two events with the same `ts`, inserted in reverse id order, derive in id order.
   - An `Agent` review launch at t, a code edit at t+1, then its `SubagentHandback` naming
     that file at t+2, and another edit at t+3 → one `review` at t+2 and one `review_fix` at
     t+3, never at t+1.
   - A catalogue row that doesn't compile fails the pass, naming it, and no mark changes.
3. **Fixture B, across sessions (D11).**
   - Session 1: tick `openspec/changes/x/tasks.md` line 3 (Edit `- [ ]`→`- [x]`). Session 2,
     linked to the same ticket: failing `cargo test` with `test result: FAILED`, then a
     passing `cargo test`.
     → `escaped_task_defects=1`, `test_defects=0`, `review_defects=1`, `total_defects=1`.
   - A `Write` update whose `originalFile` has `- [ ] a` and content `- [x] a` → 1
     `task_tick`.
   - `update_task_status {state:"done"}` → 1 `task_tick` (`plan:<id>`).
4. **Fixture C, review fixes.** An Agent `sensei:sensei-security-reviewer` response naming
   `crates/cli/src/lib.rs`, then Edits of `crates/senseid/src/lib.rs` and `crates/cli/src/lib.rs`
   → exactly 1 `review_fix` (`ref = crates/cli/src/lib.rs`). The text "see
   crates/cli/src/main.rs." then an Edit of `/r/crates/cli/src/main.rs` → 1 `review_fix`.
   - **Fixture E (D7, D11, holds).**
     - tick@0, `test_fail`@1, `build_fail`@2, `build_ok`@3, `test_ok`@4 →
       `escaped_task_defects=1`, `compile_defects=1`, `total_defects=2`.
     - Session A slice [0,10], session B slice [12,20], window [0,20], threshold 15 →
       `on_hold_count=0`.
     - A session shared by tickets A and B with 1000 tokens → `tokens_per_ticket=500`.
     - A ticket with one linked, un-derived session → `prompts` null, `not_derived`.
     - `issue_created_at` 10:00 and `submitted_at` 09:00 → `lead_hours` null,
       `created_after_submit`.
5. **Fixture D, lost failures.** A Bash `PreToolUse` `cargo build` with no Post event →
   `unmatched_call(build)`. The ticket's `runs_unknown=1`, and the screen shows "≥ 0" plus
   "1 run with unknown outcome".
6. **Sprint replay (unit).** Sprint 5 (seq 5, S=day 0, E=day 14), with these issues:
   - i1 in it from day −1, done day 10
   - i2 added day 3, done day 12
   - i3 in it from day −1, removed day 5
   - i4 in from day −1, not done, field gains sprint 6 on day 14
   - i5 in sprints 4 and 5, not done, sent to the backlog on day 14. As Jira does, this
     writes **no** sprint change: the field stays `4,5`
   - i6 created day 2 directly in sprint 5

   Expect:
   - sets: `committed={i1,i3,i4,i5}`, `added={i2,i6}`, `removed={i3}`,
     `completed_committed={i1}`, `completed_added={i2}`, `carried_over={i4}`
   - rates: `commitment_reliability=25`, `total_completion=50`, `scope_change=75`,
     `carry_over_rate=25`, `unplanned_work_ratio=50`
   - A closed sprint with `complete_at` NULL and `end_at` X evaluates at X.
   - Points {3, "abc"} → sum 3, `_n` 1, `points_unparsed` 1.
7. **Predictability.** Commitment reliabilities [50, 100, null, 50, 100] over the last 6
   closed sprints → 25. One value → null `too_few_sprints`.
8. **History task.**
   - A fake transport timing out after 10 of 30 issues leaves exactly 10 with
     `history_through` set, and the next run does the remaining 20.
   - A run after that with no `issue_updated_at` change makes **0** changelog requests.
   - In none of these is `last_error` or `last_success_at` touched, and the linkage
     "page 2 errors → zero data rows changed" sync test still passes.
   - After one successful sync with write-back on and history pending, both have run once
     (the assigned ticket's `tracker_tasks.writeback_at` and the tracker's `history_at` are
     set). With write-back off, history still runs.
9. **Live.** On any connected Jira board, after `tracker_history` has run until it reports
   no pending task, `select count(*) from sensei.tracker_tasks where tracker_id = $1 and
   removed_at is null and (history_through is null or issue_updated_at > history_through)`
   = 0, and `tracker_task_changes` has ≥1 `sprint` row for that tracker.
10. **Settings.**
    - **F3:** `GET` with no row → `{hourly_rate: null, minutes_per_point: null, idle_threshold_minutes: 15}`.
      `PUT {"hourly_rate": 50}` → 200 with the full settings, and a later `GET` returns them.
      `PUT {"hourly_rate": null}` clears it.
    - **F4:** with no rate, cost figures are null with `no_rate`; after the PUT they are numbers.
    - `PUT {"hourly_rate": "NaN"}`, `{"hourly_rate": -1}`, `{"idle_threshold_minutes": null}`
      and `{"x": 1}` → 400 `invalid_setting`.
    - A request with a foreign `Origin` → 403.
    - **F4:** changing the threshold changes holds and `worked_minutes` at the next read,
      with no re-derivation (marks unchanged).
11. **Write-back.**
    - A ticket with a mapped `compile_defects` field and one linked session whose mark is
      behind its events: the pass makes no field request for that ticket, and its
      `writeback_at` is unchanged (A3).
    - A field mapped to `compile_defects` is written with the S6 value.
    - Under a fixed `TicketFigures`, `metric_value` of each new key yields the canonical
      string its S9 kind implies (e.g. `on_hold_ratio` 33.333… → `"33.33"`, `worked_minutes`
      for 90 s → `"1.50"`).
    - `TrackerMetric::parse` of `effort_cost`, `rework_cost`, `expected_minutes`,
      `time_variance_minutes` and `estimated_points` → `None`.
    - The comment for a given usage is byte-identical to before this slice (golden).
12. **Docs sweep.** Every line this command prints contains `SDLC` or `superseded`:
    ```
    rg -n -i --no-ignore 'usage write-back|usage-metrics write-back|usage metrics only|only usage|\bthe (one|only) write( is| path|:|,|$)|write, usage metrics|a usage metric (a|of)|usage is written back|usage to tickets|fields usage|writes usage|writes back usage|usage metrics (are )?(written|back)|but usage metrics|than usage metrics|except usage metrics|for usage metrics' docs database crates app/src -g '!target' -g '!node_modules' -g '!docs/spec/daemon/tracker-metrics-writeback.md' -g '!docs/spec/daemon/tracker-sdlc-metrics.md' -g '!docs/analysis/**' -g '!docs/plans/**'
    ```
    Run before P-DOCS, it prints exactly the 53 lines listed under Overrides, and none
    contains `SDLC`. A rewritten statement keeps `SDLC` on the line the pattern matches. In
    `tracker-metrics-writeback.md`, `rg -c 'usage and SDLC'` ≥ 1.
13. **API.**
    - `/trackers/{t}/tasks` rows, `/tracker-tasks/{key}` and each `/sprints` item carry `sdlc`.
    - Each `GET /trackers` item carries `sdlc_project`.
    - GET `…/trackers/{t}` is still 405.
    - Every null has a `reasons` entry that exists in `reason_codes` domain `sdlc_metrics`.

## Wrong gate

- A drained event's signal is dated at its insert time.
- `cargo test 2>&1 | tail`, `…; echo $?` or `… || true` yields an `_ok` or `_fail` without
  a marker.
- A machine that never captured failures shows plain `0` defects instead of "≥ 0" with
  unknown runs.
- `review_fixes` counts an edit to a file with the same basename the review did not name.
- `execution_hours`, `lead_hours` or `on_hold_ratio` is negative.
- A test failure's output marks a build stage as a compile defect.
- A session not yet derived reads as zero prompts or zero defects.
- `carried_over` counts an issue sent to the backlog.
- Sprint `committed` is read from today's `tracker_task_sprints`.
- The history task sets `last_error`, or a sync commits changelog rows.
- `effort_cost`, `rework_cost` or `hourly_rate` reaches Jira, or the rate is keyed by person.
- The write-back comment gains lines.
- Toolchain support needs a code change.

## Claims (verified 2026-10-29; independent verifier run, then re-checked after rewrite)

| # | claim | check | expect | actual | verdict |
|---|---|---|---|---|---|
| C1 | No Bash failure has been captured on this machine: the installed plugin 0.11.0 lacks `PostToolUseFailure` | count rows; grep the installed plugin.json | 0; 0 | 0; 0 (repo 0.20.2 declares it) | CONFIRMED |
| C2 | `PostToolUseFailure` → `success=false` | `capture_drain.rs:57-69` | arm present | present | CONFIRMED |
| C3 | The payload keeps `tool_input` and `tool_response` | `jsonb_object_keys` | both | 3579 / 3392 | CONFIRMED |
| C4 | `Notification.notification_type = permission_prompt` | group by | present | 17 | CONFIRMED |
| C5 | `folder_commands` has `command_line` + `category` | DDL, `manifest.rs:207` | present | present | CONFIRMED |
| C6 | `folder_commands` is sparse | count | small | 15 | CONFIRMED |
| C7 | No make/just manifest adapter | `ls adapters/manifest/` | none | none | CONFIRMED |
| C8 | Sync reads no points, created/updated or changelog | `jira.rs:553,555` | absent | absent | CONFIRMED |
| C9 | `tracker_task_sprints` is replaced per sync | DDL comment | as stated | as stated | CONFIRMED |
| C10 | The board issue `expand` is unused | Software OpenAPI | "not used" | "currently not used" | CONFIRMED |
| C11 | The changelog is a paged GET with items `field/fieldId/from/to`; bulkfetch is POST | Platform OpenAPI | as stated | as stated | CONFIRMED |
| C12 | Board config `estimation.field.fieldId` | `EstimationConfigBean` | present | present | CONFIRMED |
| C13 | `/rest/api/3/status` has `statusCategory` | `StatusDetails` | present | present | CONFIRMED |
| C14 | The metric engine has no ticket/sprint grain | enum DDL | absent | `session,daily` / `repo,user` | CONFIRMED |
| C15 | Write-back writes a field only when mapped; `render` is separate; 13 keys | `writeback.rs:133-256`; DB enum | as stated | as stated | CONFIRMED |
| C16 | R7 `Usage` has `api_cost(_n,_partial)`, `active_secs`, `first_at/last_at` | `usage.rs:20-56` | present | present | CONFIRMED |
| C17 | `update_task_status` takes `state`, with `done` as a value | `crates/mcp/src/lib.rs:878-886,1815` | `state` | `state: "done"` | **was FALSE** (draft said `status`), fixed |
| C18 | One Jira board is connected (733 tasks, 13 sprints) | psql | ≥1 | as stated | CONFIRMED |
| C19 | A lane is a board, and the sprint is a filter within it | `workspace-tickets.md:23-27`, `tickets-state.svelte.ts:321` | — | as stated | **was MISLEADING** (draft: "sprints are lanes"), fixed (S8) |
| C20 | `ts` is the event time and `created_at` the insert time | DDL comments; 273 rows differ > 15 min | as stated | as stated | **was FALSE** (draft used `created_at`), fixed (S2) |
| C21 | The routes are `/trackers/{t}/tasks` and `/tracker-tasks/{key}`; there is no `/tickets` | `routes.rs:55-70` | no `/tickets` | none | **was FALSE**, fixed (S7) |
| C22 | A `Write` update carries `originalFile` | `tool_response` keys on Write | absent | 211/211 | **was FALSE** (draft excluded Write), fixed |
| C23 | NotebookEdit's path is `notebook_path` | `dojo/gate.rs:155` | — | `notebook_path` | **was FALSE**, fixed |
| C24 | A sync timeout aborts the sync and sets `last_error` | `sync.rs:20,118-130` | hard abort | hard abort | CONFIRMED, which drove the R5 override (separate history task) |
| C25 | `metric_value` takes `&Usage` only | `writeback.rs:198` | — | `(TrackerMetric, &Usage)` | CONFIRMED, which drove the W3 override |
| C26 | Most build/test runs are piped, and output markers are common | psql over 370 runs | — | 318 piped; 190 fail-marker, 209 ok-marker | CONFIRMED, which drove S3 step 3 |
| C27 | Bash PreToolUse outnumbers Post on most days (lost failures) | per-day counts | — | e.g. 10-27: 557 vs 552 | CONFIRMED, which drove `unmatched_call` |
| C28 | `local_caller_guard` is the tracker routes' layer and refuses a foreign Origin on reads | `routes.rs:71`, `api/origin_guard.rs` | — | as stated | CONFIRMED |
| C29 | `issue_path`/`is_jira_id` and `field_number` exist | `jira.rs:678,686` | — | present | CONFIRMED |
| C30 | `sprint_window` implements R7's `coalesce(complete_at, end_at)` | `usage.rs:58-72` | — | as stated | CONFIRMED |
| C31 | `command_category::categorise` is `pub` and reachable from a new module | `manifest.rs:286,295` | — | `pub mod`, `pub fn` | CONFIRMED |
| C32 | Staging jsonl + `import_*` is the catalogue pattern; `reason_codes.jsonl` is the reason catalogue | `ls database/import/staging`, `procedure/staging` | — | present; `pg_store/reasons.rs` reads it | CONFIRMED |
| C33 | `SCHEDULABLE` lists the schedulable tasks | `tasks/schedule.rs:23` | — | present | CONFIRMED |
| C35 | Sync takes the same per-tracker lock as the write-back pass | `sync.rs:97` `locks.try_lock(tracker_id)`; `writeback.rs:343` | same `TrackerLocks` | same | CONFIRMED |
| C36 | `writeback_at` is per ticket (`tracker_tasks`), not per tracker | `rg writeback_at database/ddl` | — | `tracker_tasks.ddl:20` | CONFIRMED (gate 8 names it so) |
| C37 | A command's output can carry one stage's marker but not another's | psql rows 10477, 10729, 10737, 10794: cargo build + cargo test, `test result: FAILED`, no compile marker | — | 4 rows | CONFIRMED, which drove category-scoped markers (S3 step 3) |
| C38 | `GET …/trackers/{tracker}` doesn't exist (DELETE, PATCH only); the list is `GET …/trackers` | `routes.rs:50-56` | — | as stated | **was FALSE** (rewrite put `sdlc_project` there), fixed (S7) |
| C39 | Usage-only write-back statements live exactly on the surfaces listed under Overrides | independent broad `rg` (verifier), then gate 12's widened pattern, tested to catch each verifier-found line and to exclude "the only writer" / "usage is written down" | all listed | 53 lines, 24 files, all listed | **was FALSE** (a narrow pattern saw 19 of ~34), fixed: list = gate output |
| C34 | `PostToolUseFailure` carries `is_interrupt` | no captured failure locally | — | — | UNCHECKABLE here: not load-bearing (absent → the rule falls through to the next reason); re-check in build once the plugin is updated |
