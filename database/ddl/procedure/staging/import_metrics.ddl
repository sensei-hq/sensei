set search_path to staging, sensei, extensions;

-- Seed sensei.metrics — the metric registry — from staging.metrics (dbd auto-runs
-- this on import). Only the v1 computable-now metrics are seeded (each one's
-- task_name maps to a shipped handler); blocked metrics stay out until their
-- handler exists. The enum facets arrive as text and are cast to their
-- sensei.<enum> types here (mirrors import_rule_packs).
--
-- Incremental, not a full reload: on conflict (key) it updates only when the
-- datafile's modified_at is >= the live row's — so re-importing unchanged data
-- re-applies identical values (row count stays put) and a row edited more recently
-- in prod is never clobbered. Same guard style as import_scopes.
--
-- effective_until/retire_reason ride the SAME upsert so RETIREMENT is declarative:
-- bumping a metric's modified_at + setting effective_until in the datafile retires
-- it in place on the next import (the metric drops out of the active window and
-- stops being computed/scheduled). effective_from is intentionally NOT in the
-- column list — a new row takes the table default (current_date) and an existing
-- row keeps its original start; only the end boundary + reason are seed-owned.
create or replace procedure import_metrics()
language plpgsql as $$
begin
  insert into sensei.metrics (
      key, name, description, family, type, unit, direction,
      purpose, how_to_read, formula, task_name, weight, target,
      effective_until, retire_reason, modified_at
  )
  select
      stg.key
    , stg.name
    , stg.description
    , stg.family::sensei.metric_family
    , stg.type::sensei.metric_type
    , nullif(stg.unit, '')
    , stg.direction::sensei.metric_direction
    , stg.purpose
    , stg.how_to_read
    , stg.formula
    , stg.task_name
    , coalesce(stg.weight, 1)
    , stg.target
    , stg.effective_until
    , nullif(stg.retire_reason, '')
    , coalesce(stg.modified_at, now())
  from staging.metrics stg
  where stg.key is not null
  on conflict (key)
  do update set
      name            = excluded.name
    , description     = excluded.description
    , family          = excluded.family
    , type            = excluded.type
    , unit            = excluded.unit
    , direction       = excluded.direction
    , purpose         = excluded.purpose
    , how_to_read     = excluded.how_to_read
    , formula         = excluded.formula
    , task_name       = excluded.task_name
    , weight          = excluded.weight
    , target          = excluded.target
    , effective_until = excluded.effective_until
    , retire_reason   = excluded.retire_reason
    , modified_at     = excluded.modified_at
  where excluded.modified_at >= sensei.metrics.modified_at;
end;
$$;
