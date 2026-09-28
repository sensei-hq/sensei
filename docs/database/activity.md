# `activity` — design notes

Why the tables in `database/ddl/table/activity/` are shaped the way they are.

What a column MEANS lives in `comment on column`, where the database catalog can serve it. This file holds the other half: what was measured, what was tried and failed, and what must not be changed back. It is prose about decisions, which is why it is not in the DDL.


## `activity.session_facets`

[`database/ddl/table/activity/session_facets.ddl`](../../database/ddl/table/activity/session_facets.ddl)


A stage is either absent or attributed. Recording one without saying where it came from would let an inference be read as a declaration.


## `activity.sessions`

[`database/ddl/table/activity/sessions.ddl`](../../database/ddl/table/activity/sessions.ddl)


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
