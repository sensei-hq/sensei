set search_path to export, activity, sensei, extensions;

-- Exportable projection of activity.hook_events for `dbd export -n export.hook_events`.
-- Drops the surrogate `id` (regenerated on import) and casts the assistant_family
-- enum to text. Column set matches staging.hook_events so the data round-trips via
-- `dbd import` → staging.hook_events → CALL staging.import_hook_events().
create or replace view hook_events as
select he.session_id
     , he.assistant_family::text as assistant_family
     , he.event_type
     , he.tool_name
     , he.cwd
     , he.ts
     , he.success
     , he.payload
     , he.created_at
from activity.hook_events he;

comment on view export.hook_events is
'Exportable projection of activity.hook_events (drops id, casts assistant_family to text). Columns match staging.hook_events for round-trip via import_hook_events().';
