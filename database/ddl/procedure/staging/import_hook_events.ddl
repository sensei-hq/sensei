set search_path to staging, activity, sensei, extensions;

-- ── Import procedure ────────────────────────────────────────────────────────
--
-- Transforms rows in staging.hook_events into activity.hook_events.
-- staging.hook_events uses text for assistant_family; final table uses the
-- sensei.assistant_family enum. The cast happens here.
--
-- dbd's updated import_jsonb_to_table handles enum types, so staging can be
-- loaded directly from JSONL via dbd import. This procedure then does the
-- final coalescing and enum cast for the activity table insert.
--
-- Usage:
--   -- 1. Load JSONL into staging (via dbd import or COPY):
--   dbd import hook_events ~/.sensei/events.jsonl
--
--   -- 2. Import into final table:
--   CALL staging.import_hook_events();
--
--   -- 3. Clear staging:
--   TRUNCATE staging.hook_events;

create or replace procedure staging.import_hook_events()
language plpgsql
as $$
declare
  v_count int := 0;
begin
  insert into activity.hook_events
    (session_id, assistant_family, event_type, tool_name, cwd, ts, success, payload, created_at)
  select
      coalesce(stg.session_id, '')
    , coalesce(stg.assistant_family, 'claude')::sensei.assistant_family
    , coalesce(stg.event_type, 'unknown')
    , nullif(stg.tool_name, '')
    , nullif(stg.cwd, '')
    , coalesce(stg.ts, extract(epoch from now())::bigint * 1000)
    , stg.success
    , coalesce(stg.payload, '{}'::jsonb)
    , coalesce(stg.created_at, now())
  from staging.hook_events stg
  where stg.event_type is not null
  ;

  get diagnostics v_count = row_count;

  raise notice 'import_hook_events: inserted % rows into activity.hook_events', v_count;
end;
$$;

comment on procedure staging.import_hook_events() is
'Import staging.hook_events into activity.hook_events.
staging.hook_events.assistant_family is text; cast to sensei.assistant_family enum here.
dbd import_jsonb_to_table (updated to handle custom types) loads JSONL into staging.

Hook script (sensei-hook.ts) enriches payloads with:
  assistant_family: "claude"  (or other family)
  event_type: <hook_event_name>  (column-name alias for staging compatibility)

Full workflow:
  dbd import hook_events ~/.sensei/events.jsonl   -- loads into staging.hook_events
  CALL staging.import_hook_events();               -- staging → activity (with enum cast)
  TRUNCATE staging.hook_events;';
