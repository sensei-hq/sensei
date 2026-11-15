set search_path to sensei, extensions;

-- Which part of a ticket's history a sensei.tracker_task_changes row records
-- (spec tracker-sdlc-metrics S4).
create type tracked_change_field as enum (
  'sprint'    -- the issue's sprints (comma-separated sprint ids)
, 'status'    -- its status id
, 'estimate'  -- its story points (the board's estimation field)
);

comment on type tracked_change_field is
'The parts of a ticket''s tracker history sensei keeps (spec tracker-sdlc-metrics S4).';
