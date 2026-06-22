set search_path to export, activity, sensei, extensions;

-- Exportable projection of activity.assistant_events for `dbd export -n export.assistant_events`.
-- Drops the surrogate `id` (regenerated on import) and casts the `family` enum
-- to text. Column set matches staging.assistant_events so the data round-trips
-- via `dbd import` → staging.assistant_events → CALL staging.import_assistant_events().
create or replace view assistant_events as
select ae.session_id
     , ae.family::text as family
     , ae.event_type
     , ae.tool_name
     , ae.cwd
     , ae.ts
     , ae.success
     , ae.payload
     , ae.created_at
from activity.assistant_events ae;

comment on view export.assistant_events is
'Exportable projection of activity.assistant_events (drops id, casts family to text). Columns match staging.assistant_events for round-trip via import_assistant_events().';
