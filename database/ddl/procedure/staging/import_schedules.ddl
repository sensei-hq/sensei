set search_path to staging, sensei, extensions;

-- Seed the default schedule for each background worker.
--
-- Adding a schedulable worker later is ONE LINE in the datafile plus its
-- registry entry and tick() — no migration and no change to the scheduler. That
-- is the point of making schedules data.
--
-- INCREMENTAL AND NON-DESTRUCTIVE, and here that matters more than usual: these
-- rows are USER-EDITABLE. Deploy order is apply → import, so without a guard the
-- seed would have the last word and silently revert every cadence a user had
-- changed (the same trap that once produced two global-dojo tenants). The
-- `modified_at` guard is the house convention from import_scopes /
-- import_tenants: the datafile only wins when it is at least as new as the live
-- row, so a user's edit survives and a genuinely-updated default still lands.
--
-- Runtime state (last_run_at / last_ok / last_error) is deliberately NOT touched:
-- it belongs to the daemon, not the datafile, and re-importing must not erase the
-- record of what actually happened.
create or replace procedure import_schedules()
language plpgsql
set search_path = staging, sensei, extensions
as $$
begin
  insert into sensei.schedules (
      name, enabled, interval_secs, window_start, window_end, days, modified_at
  )
  select
      stg.name
    , coalesce(stg.enabled, true)
    , stg.interval_secs
    , stg.window_start
    , stg.window_end
    , stg.days
    , coalesce(stg.modified_at, now())
  from staging.schedules stg
  where stg.name is not null
    and stg.interval_secs is not null
  on conflict (name)
  do update set
      enabled       = excluded.enabled
    , interval_secs = excluded.interval_secs
    , window_start  = excluded.window_start
    , window_end    = excluded.window_end
    , days          = excluded.days
    , modified_at   = excluded.modified_at
  where excluded.modified_at >= sensei.schedules.modified_at;
end;
$$;
