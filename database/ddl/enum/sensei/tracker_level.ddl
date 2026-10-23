set search_path to sensei, extensions;

-- What a tracker's board is for in its workspace (D40). A workspace may hold one
-- tracker per board, each at a level. Spec: docs/spec/tracker-levels.md.
create type tracker_level as enum (
  'epic'      -- a portfolio board: epics and milestones (T3, V11)
, 'sprint'    -- a team board: sprints and their tasks, one lane each on the Sprints screen
, 'task'      -- a board whose tickets are linked from changes only
);

comment on type tracker_level is
'The level a sensei.trackers board serves in its workspace (D40). Mirrors crate::tracker::TrackerLevel.';
