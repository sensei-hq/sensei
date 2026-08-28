# Schedules — background work as configuration

> **Status:** plan. Supersedes **D4** in `docs/spec/dojo/daemon-sync.md`: `dojo_sync`
> becomes a schedule row plus a `tick()`, not a bespoke worker.

## Why

`api/handlers/scheduled_tasks.rs` already registers the daemon's nine background
workers — name, description, last-run watermark — and serves them at
`GET /api/tasks/scheduled`. It is **static, read-only, and documents its own
drift risk**: *"Registry, not reflection — keep in step when a worker is added."*

Meanwhile nine ad-hoc config keys (`metrics.interval_secs`,
`analyzer.full_refresh_secs`, `activity.prune_interval_secs`, …) already express
"schedule as data", unstructured and undiscoverable.

This finishes both: one table users can edit, one registry the code and the table
agree on, and the drift the handler warns about becomes a failing test.

It also changes what a toggle means. "Sync on/off" stops being a boolean and
becomes `enabled + timing` — and the same mechanism then answers "analyse every
15 minutes", "prune at 3am", "never sync during my working hours" without new
machinery each time.

## What stays code

The **schedule** is data. The **decision** — what each task enqueues when it
wakes — stays code, and the set of **legal task names** stays code too: a
schedule naming a task with no implementation is a bug, not configuration.

That mirrors an existing house pattern exactly. `sensei.task_execution_kind` is
validated against `TaskKind::ALL` by a test that fails in both directions — a
live kind missing from the DB, or a DB value no kind produces. Schedules get the
same test.

## Schema

```
sensei.schedules
  name           text primary key       -- must match a code-side registry entry
  enabled        boolean     not null default true
  interval_secs  integer     not null check (interval_secs > 0)
  window_start   time                   -- NULL = any time of day
  window_end     time                   -- NULL = any time of day
  days           smallint[]             -- ISO 1=Mon..7=Sun; NULL = every day
  last_run_at    timestamptz
  last_ok        boolean
  last_error     text
  updated_at     timestamptz not null default now()
```

`interval_secs > 0` is a CHECK, not a runtime fallback: a zero interval
busy-loops, and the database is the right place to make that unrepresentable.

**Windows wrap midnight.** `22:00–05:00` is the obvious way to say "overnight",
so `window_start > window_end` means the window spans midnight rather than being
empty. Getting this wrong silently means a nightly task never runs.

**Local time, deliberately.** A user saying "not during my working hours" means
their hours. The pure predicate takes an already-converted local `NaiveDateTime`
so it carries no timezone reasoning of its own.

## The predicate

```rust
pub fn should_run(s: &Schedule, now_local: NaiveDateTime, last_run: Option<DateTime<Utc>>) -> bool
```

Three independent reasons not to run, each testable alone:

1. `!enabled` — never. On-demand paths (the API enqueueing the same task
   directly) are unaffected: disabling a *schedule* must not disable a
   *capability*.
2. not due — `last_run + interval_secs` has not elapsed.
3. outside the window — wrong time of day, or wrong day of week.

## Defaults ship as SEED DATA, not as code

`database/import/staging/schedules.jsonl` carries the default schedule for every
schedulable task, loaded through `staging.import_schedules` like every other
seeded row in this project. No defaults in Rust, no `apply` hook holding literal
VALUES — `import_tenants` records why: a procedure with literal values *"can
silently drift from the table it writes to, which is exactly how
seed_ponytail_pack came to reference a column that had been renamed."*

**A user's edit must survive a re-deploy**, and the house convention already
guarantees it. Ordering is apply → import, so the seed otherwise has the last
word — the trap that produced two `global-dojo` tenants. `import_tenants` and
`import_scopes` solve it with a timestamp guard, and schedules use the same one:

```sql
on conflict (name) do update set …
 where excluded.updated_at >= sensei.schedules.updated_at;
```

The datafile only wins when it is at least as new as the live row. A user who
sets `analyzer` to 15 minutes keeps it; a genuinely-updated default still lands.

**Adding a schedulable task later is one line in the datafile** plus its
registry entry and `tick()` — no migration, no code change to the scheduler.
That is the point of the whole design.

Defaults are today's constants, so upgrading changes no behaviour: `metrics`
3600, `analyzer` 900, `activity_prune` 86400 (still skipping its boot tick via
`FirstTick`), `log_prune` 86400, and so on. Every existing `*.interval_secs`
config key is migrated into its row and then retired, so a cadence lives in
exactly one place.

## Surface

- `GET /api/tasks/scheduled` — gains `enabled`, `interval_secs`, `window`, `days`
  alongside the `last_run_at` it already returns.
- `PATCH /api/tasks/scheduled/{name}` — edit. Validates against the code
  registry, so an unknown name is a 404 rather than a row nobody runs.

## Order

1. schema + `staging.schedules` + `import_schedules` + the seed datafile + the
   code↔table agreement test
2. the pure `should_run` predicate (windows, wrap, day mask, due-ness)
3. `ticker` consults it; the twelve keep their `tick()` unchanged
4. API read + patch
5. `dojo_sync` lands as a row + a `tick()` — the first task to never have a
   bespoke worker

## Done gate

- [ ] a disabled schedule never runs, and its on-demand path still works
- [ ] a window of `22:00–05:00` runs at 23:00 and at 04:00, and not at noon
- [ ] a day mask of Mon–Fri does not run on Sunday
- [ ] `interval_secs = 0` is rejected by the database, not by a fallback
- [ ] adding a worker without a registry entry fails a test (the drift the
      current handler warns about)
- [ ] every existing cadence is unchanged after migration
- [ ] a user edit survives `dbd deploy` — the seed does not revert it
- [ ] a new task added to the datafile appears after a deploy, with no code change
      to the scheduler itself
