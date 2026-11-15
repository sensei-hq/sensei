set search_path to relay, extensions;

-- Which phase of the work a task is in. There is no merge phase: the merge is
-- an action on an approved task. `planning` is Task 0 of a sprint run: the
-- decomposition a peer reviews before anything unpacks into the queue.
create type relay.task_phase as enum (
  'analysis',
  'tests',
  'impl',
  'docs',
  'mockup',
  'planning'
);

comment on type relay.task_phase is
'The round counts per phase; approval of one phase moves the task to the next. planning = a run''s Task 0.';
