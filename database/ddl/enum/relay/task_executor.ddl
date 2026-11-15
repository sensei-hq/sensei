set search_path to relay, extensions;

-- Who runs a task: an assistant lane (a cognitive turn) or the broker itself
-- (a deterministic machine operation: a merge gate, a build, a cleanup).
create type relay.task_executor as enum (
  'agent',
  'broker'
);

comment on type relay.task_executor is
'agent = claimed by a lane; broker = the daemon runs it from tasks.deterministic, never a lane.';
