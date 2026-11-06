set search_path to relay, extensions;

-- Which phase of the work a task is in. There is no merge phase: the merge is
-- an action the coordinator takes on an approved task.
create type relay.task_phase as enum (
  'analysis',
  'tests',
  'impl',
  'docs',
  'mockup'
);

comment on type relay.task_phase is
'The round counts per phase; approval of one phase moves the task to the next.';
