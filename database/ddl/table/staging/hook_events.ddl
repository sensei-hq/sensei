set search_path to staging, extensions;

drop table if exists hook_events cascade;
create table hook_events (
  session_id        text
, assistant_family  text        default 'claude'
, event_type        text
, tool_name         text
, cwd               text
, ts                bigint
, success           boolean
, payload           jsonb
, created_at        timestamptz
);

comment on table hook_events is
'Staging buffer for activity.hook_events.
assistant_family is text here (cast to enum during import).
Load via dbd import or CALL staging.import_jsonb_to_table(''_temp'', ''staging.hook_events'').
The hook script enriches payload with event_type (mapped from hook_event_name)
so that import_jsonb_to_table can map field names directly.
Then call: CALL staging.import_hook_events() to move into activity.hook_events.';
