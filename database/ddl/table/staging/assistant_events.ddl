set search_path to staging, extensions;

drop table if exists assistant_events cascade;
create table assistant_events (
  session_id        text
, family            text        default 'claude'
, event_type        text
, tool_name         text
, cwd               text
, ts                bigint
, success           boolean
, payload           jsonb
, created_at        timestamptz
);

comment on table assistant_events is
'Staging buffer for activity.assistant_events.
family is text here (cast to the sensei.assistant_family enum during import).
Load via dbd import or CALL staging.import_jsonb_to_table(''_temp'', ''staging.assistant_events'').
Then call: CALL staging.import_assistant_events() to move into activity.assistant_events.';
