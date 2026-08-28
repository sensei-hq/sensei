set search_path to sensei, extensions;

-- When each background worker runs — configuration, not code.
--
-- The daemon's periodic workers used to carry their cadence in nine ad-hoc
-- config keys (`metrics.interval_secs`, `analyzer.full_refresh_secs`, …), and
-- `api/handlers/scheduled_tasks.rs` listed them in a static Rust const that
-- documented its own drift risk: "Registry, not reflection — keep in step when a
-- worker is added." This table is the editable half of that; the set of legal
-- NAMES stays in code (`tasks::schedule::SCHEDULABLE`), because a schedule
-- naming a worker with no implementation is a bug, not data. A test asserts the
-- two agree in both directions.
--
-- It also changes what a toggle means: "sync on/off" stops being a boolean and
-- becomes enabled + timing, which is the same mechanism that answers "analyse
-- every 15 minutes", "prune at 3am" and "never during my working hours" without
-- new machinery for each.
create table if not exists schedules (
  name           text        primary key
, enabled        boolean     not null default true
, interval_secs  integer     not null
      -- A CHECK, not a runtime fallback: a zero interval busy-loops a core, and
      -- the database is the right place to make that unrepresentable.
      check (interval_secs > 0)
, window_start   time
, window_end     time
, days           smallint[]
      -- ISO weekdays: 1 = Monday … 7 = Sunday. Rejects anything else rather than
      -- silently never matching.
      check (days is null or (
             array_length(days, 1) between 1 and 7
         and days <@ array[1,2,3,4,5,6,7]::smallint[]))
, last_run_at    timestamptz
, last_ok        boolean
, last_error     text
, created_at     timestamptz not null default now()
, modified_at    timestamptz not null default now()
);

comment on table schedules is
'When each background worker runs. One row per schedulable task; the legal names
live in code (tasks::schedule::SCHEDULABLE) and a test asserts code and table
agree, so a worker added without a row — or a row naming no worker — fails the
build rather than silently never running.

Seeded from database/import/staging/schedules.jsonl. The import guards on
modified_at, so a user edit is never clobbered by a re-deploy.';

comment on column schedules.name
     is 'The worker this schedules. Must match a tasks::schedule::SCHEDULABLE entry.';
comment on column schedules.enabled
     is 'False = never run on a schedule. On-demand paths (an API enqueue) are unaffected: disabling a SCHEDULE must not disable a CAPABILITY.';
comment on column schedules.interval_secs
     is 'How often, in seconds. CHECK > 0 — a zero interval would busy-loop.';
comment on column schedules.window_start
     is 'Start of the allowed time-of-day window (local time). NULL with window_end = any time. A window whose start is AFTER its end wraps midnight: 22:00-05:00 means overnight, not an empty range.';
comment on column schedules.window_end
     is 'End of the allowed window (local time), inclusive. NULL with window_start = any time.';
comment on column schedules.days
     is 'ISO weekdays the worker may run on (1=Mon..7=Sun). NULL = every day — an unset mask must never mean "never".';
comment on column schedules.last_run_at
     is 'When the worker last completed a scheduled pass. NULL = never run, which is always due (a freshly seeded schedule runs on its first tick).';
comment on column schedules.last_ok
     is 'Outcome of that last pass. NULL until one has run — never a fabricated success.';
comment on column schedules.last_error
     is 'Why the last pass failed, when it did. Cleared on success, so a stale error never sits beside a healthy run.';
