set search_path to relay, extensions;

-- Where a task stands. Assigned/working/in_review/rework/blocked hold the
-- lane; approved, done, cancelled and interrupted release it. `ready` is the
-- public pool: unclaimed, no lane. Labels append, never reorder, so a
-- converged database and a fresh one agree.
create type relay.task_state as enum (
  'assigned',
  'working',
  'in_review',
  'rework',
  'approved',
  'done',
  'blocked',
  'interrupted',
  'cancelled',
  'ready'
);

comment on type relay.task_state is
'Active states occupy the lane (one active task per lane); the rest release it. ready = unclaimed, in the pool.';
