set search_path to staging, activity, sensei, extensions;

-- ── Import procedure ────────────────────────────────────────────────────────
--
-- Transforms rows in staging.assistant_events into activity.assistant_events.
-- staging uses text for `family`; the final table uses the
-- sensei.assistant_family enum. The cast happens here.
--
-- Usage:
--   dbd import assistant_events ~/.sensei/events.jsonl   -- loads into staging
--   CALL staging.import_assistant_events();               -- staging → activity (enum cast)
--   TRUNCATE staging.assistant_events;

create or replace procedure staging.import_assistant_events()
language plpgsql
as $$
declare
  v_count int := 0;
begin
  insert into activity.assistant_events
    (session_id, family, event_type, tool_name, cwd, ts, success, payload, created_at)
  select
      coalesce(stg.session_id, '')
    , coalesce(stg.family, 'claude')::sensei.assistant_family
    , coalesce(stg.event_type, 'unknown')
    , nullif(stg.tool_name, '')
    , nullif(stg.cwd, '')
    , coalesce(stg.ts, extract(epoch from now())::bigint * 1000)
    , stg.success
    , coalesce(stg.payload, '{}'::jsonb)
    , coalesce(stg.created_at, now())
  from staging.assistant_events stg
  where stg.event_type is not null
  ;

  get diagnostics v_count = row_count;

  raise notice 'import_assistant_events: inserted % rows into activity.assistant_events', v_count;
end;
$$;

comment on procedure staging.import_assistant_events() is
'Import staging.assistant_events into activity.assistant_events (casts family text →
sensei.assistant_family enum). Workflow:
  dbd import assistant_events ~/.sensei/events.jsonl   -- loads staging
  CALL staging.import_assistant_events();               -- staging → activity
  TRUNCATE staging.assistant_events;';
