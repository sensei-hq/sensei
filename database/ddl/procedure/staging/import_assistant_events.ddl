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
  -- Idempotent: one row per event, and only events not already imported. This
  -- was a plain INSERT, so every `dbd import` appended the staging file again —
  -- 167,932 duplicate events (26%) on one install, tool calls stored 12 times.
  -- An event is (session, ts, event_type, payload): the hook writes each once.
  insert into activity.assistant_events
    (session_id, family, event_type, tool_name, cwd, ts, success, payload, created_at)
  select distinct on (e.session_id, e.ts, e.event_type, e.payload)
         e.session_id, e.family, e.event_type, e.tool_name, e.cwd, e.ts, e.success, e.payload, e.created_at
    from (
      select
          coalesce(stg.session_id, '')                                   as session_id
        , coalesce(stg.family, 'claude')::sensei.assistant_family        as family
        , coalesce(stg.event_type, 'unknown')                            as event_type
        , nullif(stg.tool_name, '')                                      as tool_name
        , nullif(stg.cwd, '')                                            as cwd
        , coalesce(stg.ts, extract(epoch from now())::bigint * 1000)     as ts
        , stg.success                                                    as success
        , coalesce(stg.payload, '{}'::jsonb)                             as payload
        , coalesce(stg.created_at, now())                                as created_at
        from staging.assistant_events stg
       where stg.event_type is not null
    ) e
   where not exists (
           select 1
             from activity.assistant_events a
            where a.session_id = e.session_id
              and a.ts         = e.ts
              and a.event_type = e.event_type
              and a.payload    = e.payload
         )
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
