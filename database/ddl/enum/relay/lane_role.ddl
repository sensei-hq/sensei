set search_path to relay, extensions;

-- A lane's part in its crew: one coordinator (the broker's planner), any
-- number of contributors (the sessions doing the work).
create type relay.lane_role as enum (
  'coordinator',
  'contributor'
);

comment on type relay.lane_role is
'coordinator plans and merges; contributor works tasks. One coordinator per crew.';
