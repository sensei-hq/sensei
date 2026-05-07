set search_path to staging, activity, sensei, extensions;

-- ── Import procedure ────────────────────────────────────────────────────────
--
-- Transforms rows in staging.hook_events into activity.hook_events.
-- staging.hook_events mirrors the final table structure (all columns text/basic types).
-- assistant_family is text in both tables — no enum cast needed.
--
-- New format (sensei-hook.ts): payload already has event_type, session_id, assistant_family.
-- Load via dbd import or COPY, then call this procedure.
--
-- Usage:
--   -- 1. Load JSONL into staging:
--   COPY staging.hook_events(session_id, assistant_family, event_type, tool_name, cwd, ts, payload)
--     FROM '/path/to/events.jsonl' (FORMAT text);
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
    , coalesce(stg.assistant_family, 'claude')
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
Both tables use text for assistant_family (no enum cast needed).

Hook script (sensei-hook.ts) enriches payloads with:
  assistant_family: "claude"  (or other family)
  event_type: <hook_event_name>  (column-name alias for staging compatibility)

So import_jsonb_to_table can be used to load the JSONL into staging directly,
and this procedure handles the final coalescing and insertion.

Full workflow:
  CALL import_jsonb_to_table(''staging._temp'', ''staging.hook_events'');  -- via dbd
  CALL staging.import_hook_events();
  TRUNCATE staging.hook_events;';
