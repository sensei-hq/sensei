# `activity` — design notes

Why the tables in `database/ddl/table/activity/` are shaped the way they are.

What a column MEANS lives in `comment on column`, where the database catalog can serve it. This file holds the other half: what was measured, what was tried and failed, and what must not be changed back. It is prose about decisions, which is why it is not in the DDL.


## `activity.session_facets`

[`database/ddl/table/activity/session_facets.ddl`](../../database/ddl/table/activity/session_facets.ddl)


A stage is either absent or attributed. Recording one without saying where it came from would let an inference be read as a declaration.


## `activity.sessions`

[`database/ddl/table/activity/sessions.ddl`](../../database/ddl/table/activity/sessions.ddl)


### `sdd_framework`, `sdd_framework_source`


A probe that could not answer (permission denied, a hung mount past 5 s) is stored as `probe_failed`, not as NULL and not as a failed session. NULL would read as "this repo uses no framework", a plausible answer the probe never gave; failing the session would delete its build, test and review signals (the failure path removes them) and keep the schedule red on every pass for one unreadable repo. The check constraint keeps a framework and its source written together.

A value is never replaced by NULL when its repo later disappears or becomes unreadable (blueprint B12): `static` means the repo as found when the session was derived. `static_ambiguous` (markers of two frameworks, e.g. an empty `openspec/` scaffold beside a populated `_bmad/`) stays on the session but is never pinned onto a ticket, whose value is set once (blueprint `docs/blueprints/2026-11-10-ticket-sdd-framework.md`).


### `tokens_fresh`


Session-total usage SPLIT. `tokens_in` above keeps its meaning (all input the model processed) so existing consumers are untouched; these carry the parts. Cache reads are ~96-98% of input across every source measured and bill far cheaper, so anything reading cost off the folded total is ~8x high AND moves the wrong way as caching improves.


## `activity.task_executions`

[`database/ddl/table/activity/task_executions.ddl`](../../database/ddl/table/activity/task_executions.ddl)


### `task_kind`


Enum, not text: a renamed or retired kind used to orphan its history silently (four such orphans accumulated before this constraint existed). See the type's header for the retired values and why they are kept.


## `activity.transcript_turns`

[`database/ddl/table/activity/transcript_turns.ddl`](../../database/ddl/table/activity/transcript_turns.ddl)


### `attrs`


Every per-turn attribute the transcript carried, verbatim. The adapters see a far richer record than we model (parentUuid, requestId, permissionMode, usage.speed, usage.server_tool_use, …) and anything not promoted below used to be dropped on the floor at parse time — unrecoverable without re-reading files the user may have rotated away. Keep the raw shape here so a new signal is a query, not a re-ingest, and promote a column only once something reads it.


── Promoted: token accounting ──────────────────────────────────────────── Split, NOT summed. `tokens_in` on activity.sessions folds fresh input + cache-write + cache-read into one number, and measured against real transcripts ~98% of it is cache reads — which bill about 10x cheaper. Every cost metric built on that sum therefore reads roughly an order of magnitude high, and improving cache use makes it go UP. Kept separate at this grain so cost can be computed honestly.


── Promoted: signals with a known consumer ─────────────────────────────── `max_tokens` is a DETERMINISTIC context-pressure signal; the shipped context_pressure_rate metric currently infers it from a text hint.


## `activity.transcript_raw`

[`database/ddl/table/activity/transcript_raw.ddl`](../../database/ddl/table/activity/transcript_raw.ddl)


The file's bytes as ingested, gzipped — recovery for rotated transcripts. `attrs` keeps the parsed shape; this keeps the unparsed one, so a format detail the parser drops (tool I/O, thinking, record boundaries) survives the file's rotation for the retention window (90 days past last sighting, `activity.transcript_raw_retention_days`).

Three decisions: the sha-compare skip (an upsert with unchanged bytes advances `last_seen_at` without rewriting megabytes of bytea — re-reads are common, changes are not); the sighting clock (fresh skips touch `last_seen_at` without re-reading, so a live file never expires, and a gone file's backup survives exactly the window past its last sighting); best-effort writes (a backup failure warns and the ingest continues — the backup guards the ingest, it must never break it). Units too large to hold whole get no row: the capture re-reads via `load_content`, whose size cap returns None past it, and the Claude adapter streams without keeping bytes at all.
