-- `relay.tasks` after the queue cutover (phase 2 task 2.1) — the destructive
-- half of the autonomous-queue schema. Pending until the daemon switches:
-- rename to `.sql` in the cutover commit.
--
-- WHAT BREAKS THIS TEST: `reviewer_lane_id` still present; `in_review` still
-- holding the lane (one working + one in_review on a lane must coexist; two
-- working must not); a ready task with a lane, or a laned task without a
-- state that holds it; `state` not defaulting to ready; `assigned_at` not
-- nullable; the lane section of relay.now still showing an in_review task.

begin;

\ir ../fixtures/relay_apply.ddl

insert into relay.crews (user_id, id, status) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fd01', 'active');
insert into relay.lanes (user_id, id, crew_id, code, family, token_hash) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fd11',
   '11111111-1111-7000-8000-00000000fd01', 'L1', 'claude-code', 'x');

do $$
declare
  n int;
  v_state text;
begin
  -- The slice-1 reviewer column is gone; the claim column is the reviewer.
  select count(*) into n from information_schema.columns
   where table_schema = 'relay' and table_name = 'tasks' and column_name = 'reviewer_lane_id';
  if n <> 0 then
    raise exception 'tasks.reviewer_lane_id must be dropped at the cutover.';
  end if;

  -- A new task is ready, in the pool, with no lane and no assigned_at.
  insert into relay.tasks (user_id, id, crew_id, repo_key, title, phase) values
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fd21',
     '11111111-1111-7000-8000-00000000fd01', 'github.com/sensei-hq/sensei', 'Defaults', 'impl');
  select state::text into v_state from relay.tasks where id = '11111111-1111-7000-8000-00000000fd21';
  if v_state is distinct from 'ready' then
    raise exception 'tasks.state must default to ready, saw %.', v_state;
  end if;
  select count(*) into n from relay.tasks
   where id = '11111111-1111-7000-8000-00000000fd21' and assigned_at is null;
  if n <> 1 then
    raise exception 'assigned_at must be null until a claim.';
  end if;

  -- Ready ⟺ no lane.
  begin
    update relay.tasks set lane_id = '11111111-1111-7000-8000-00000000fd11'
     where id = '11111111-1111-7000-8000-00000000fd21';
    raise exception 'a ready task on a lane was accepted.';
  exception when check_violation then null;
  end;
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fd22',
       '11111111-1111-7000-8000-00000000fd01', 'github.com/sensei-hq/sensei', 'Laneless working', 'impl', 'working');
    raise exception 'a working task with no lane was accepted.';
  exception when check_violation then null;
  end;

  -- in_review frees the lane: L1 holds one in review and one working.
  insert into relay.tasks (user_id, id, crew_id, lane_id, repo_key, title, phase, state) values
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fd23',
     '11111111-1111-7000-8000-00000000fd01', '11111111-1111-7000-8000-00000000fd11',
     'github.com/sensei-hq/sensei', 'In review', 'impl', 'in_review'),
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fd24',
     '11111111-1111-7000-8000-00000000fd01', '11111111-1111-7000-8000-00000000fd11',
     'github.com/sensei-hq/sensei', 'Working', 'impl', 'working');
  -- ... but not two working.
  begin
    insert into relay.tasks (user_id, id, crew_id, lane_id, repo_key, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fd25',
       '11111111-1111-7000-8000-00000000fd01', '11111111-1111-7000-8000-00000000fd11',
       'github.com/sensei-hq/sensei', 'Second working', 'impl', 'working');
    raise exception 'a lane holding two working tasks was accepted.';
  exception when unique_violation then null;
  end;

  -- The lane section shows the working task, not the one in review.
  select count(*) into n from relay.now
   where section = 'lane' and id = '11111111-1111-7000-8000-00000000fd11'
     and payload->'task'->>'id' = '11111111-1111-7000-8000-00000000fd24';
  if n <> 1 then
    raise exception 'the lane section must show the working task once in_review frees the lane.';
  end if;

  -- The slice-1 index is gone; the worker index is in.
  select count(*) into n from pg_indexes
   where schemaname = 'relay' and tablename = 'tasks' and indexname = 'tasks_one_active_per_lane';
  if n <> 0 then
    raise exception 'tasks_one_active_per_lane must be dropped at the cutover.';
  end if;
  select count(*) into n from pg_indexes
   where schemaname = 'relay' and tablename = 'tasks' and indexname = 'tasks_one_active_worker_per_lane';
  if n <> 1 then
    raise exception 'tasks_one_active_worker_per_lane must exist.';
  end if;
end $$;

rollback;
