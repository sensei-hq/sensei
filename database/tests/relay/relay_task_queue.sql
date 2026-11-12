-- `relay.task_queue` — the claimable work, read-only (F01): what a lane may
-- take next and in which class. A read never starts work.
--
-- WHAT BREAKS THIS TEST: a working/done task in the queue; a floating review
-- read as claimed or the reverse; a ready task with an unmet dependency read
-- as unblocked (or a done dependency still listed as unmet); a non-owner
-- reading a row; the view not security-invoker.
--
-- Run from database/tests/ (`run.sh relay`, `run.sh --apply sensei`
-- fixtures/relay_apply.ddl relay/relay_task_queue.sql`, or psql $SENSEI_DATABASE_URL -f relay/relay_task_queue.sql):
-- the file applies its own DDL in-txn (see below) and rolls back.
-- with --apply the runner applies the DDL and the file skips its own apply.
--
-- NOTE: Q7 (working) sits on its own lane L3, not beside Q4 on L1 — until the
-- phase-2 cutover `in_review` still holds the lane, so one lane cannot carry
-- both. The queue reads the same either way.

begin;

-- The real DDL, in this transaction, rolled back with it.
\ir ../fixtures/relay_apply.ddl

insert into relay.crews (user_id, id, status) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fc01', 'active'),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000fc02', 'active');

insert into relay.lanes (user_id, id, crew_id, code, family, token_hash) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fc11',
   '11111111-1111-7000-8000-00000000fc01', 'L1', 'claude-code', 'x'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fc12',
   '11111111-1111-7000-8000-00000000fc01', 'L2', 'codex', 'x'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fc13',
   '11111111-1111-7000-8000-00000000fc01', 'L3', 'codex', 'x'),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000fc19',
   '22222222-2222-7000-8000-00000000fc02', 'L1', 'claude-code', 'x');

-- Q5: done (out, and a satisfied dependency). Q1: ready, no deps. Q2: ready,
-- waits on Q1 (unmet). Q3: ready, waits on Q5 (met). Q4: in review, floating
-- (author L1). Q6: in review, claimed by L1 (author L2). Q7: working on L3
-- (out). Q9: U2's ready task.
insert into relay.tasks (user_id, id, crew_id, lane_id, review_claimed_by, review_claimed_at, repo_key, title, phase, state, deps, state_changed_at) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fc25',
   '11111111-1111-7000-8000-00000000fc01', '11111111-1111-7000-8000-00000000fc12', null, null,
   'github.com/sensei-hq/sensei', 'Done dep', 'impl', 'done', '{}', '2026-11-10T10:00:00Z'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fc21',
   '11111111-1111-7000-8000-00000000fc01', null, null, null,
   'github.com/sensei-hq/sensei', 'Free', 'impl', 'ready', '{}', '2026-11-10T10:01:00Z'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fc22',
   '11111111-1111-7000-8000-00000000fc01', null, null, null,
   'github.com/sensei-hq/sensei', 'Waits on Free', 'impl', 'ready',
   '{11111111-1111-7000-8000-00000000fc21}', '2026-11-10T10:02:00Z'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fc23',
   '11111111-1111-7000-8000-00000000fc01', null, null, null,
   'github.com/sensei-hq/sensei', 'Waits on done', 'tests', 'ready',
   '{11111111-1111-7000-8000-00000000fc25}', '2026-11-10T10:03:00Z'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fc24',
   '11111111-1111-7000-8000-00000000fc01', '11111111-1111-7000-8000-00000000fc11', null, null,
   'github.com/sensei-hq/sensei', 'Floating', 'impl', 'in_review', '{}', '2026-11-10T10:04:00Z'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fc26',
   '11111111-1111-7000-8000-00000000fc01', '11111111-1111-7000-8000-00000000fc12',
   '11111111-1111-7000-8000-00000000fc11', '2026-11-10T10:06:00Z',
   'github.com/sensei-hq/sensei', 'Claimed', 'impl', 'in_review', '{}', '2026-11-10T10:05:00Z'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fc27',
   '11111111-1111-7000-8000-00000000fc01', '11111111-1111-7000-8000-00000000fc13', null, null,
   'github.com/sensei-hq/sensei', 'Working', 'impl', 'working', '{}', '2026-11-10T10:07:00Z'),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000fc29',
   '22222222-2222-7000-8000-00000000fc02', null, null, null,
   'github.com/sensei-hq/other', 'Theirs', 'impl', 'ready', '{}', '2026-11-10T10:08:00Z');

do $$
declare
  n int;
  v_class text;
  v_deps uuid[];
begin
  -- Exactly the pool and the reviews: Q1 Q2 Q3 Q4 Q6 for U1, Q9 for U2.
  select count(*) into n from relay.task_queue
   where user_id in ('a1111111-1111-4111-8111-111111111111', 'a2222222-2222-4222-8222-222222222222');
  if n <> 6 then
    raise exception 'task_queue should hold 6 rows, saw %.', n;
  end if;
  select count(*) into n from relay.task_queue
   where task_id in ('11111111-1111-7000-8000-00000000fc25', '11111111-1111-7000-8000-00000000fc27');
  if n <> 0 then
    raise exception 'done/working tasks must leave the queue, saw %.', n;
  end if;

  -- Q1: free to claim.
  select claim_eligibility, unmet_deps into v_class, v_deps from relay.task_queue
   where task_id = '11111111-1111-7000-8000-00000000fc21';
  if v_class is distinct from 'ready_unblocked' or v_deps is distinct from '{}'::uuid[] then
    raise exception 'Q1 should read ready_unblocked with no unmet deps, saw % %.', v_class, v_deps;
  end if;

  -- Q2: claim-early, naming Q1 as unmet.
  select claim_eligibility, unmet_deps into v_class, v_deps from relay.task_queue
   where task_id = '11111111-1111-7000-8000-00000000fc22';
  if v_class is distinct from 'ready_claim_early'
     or v_deps is distinct from array['11111111-1111-7000-8000-00000000fc21']::uuid[] then
    raise exception 'Q2 should read ready_claim_early with Q1 unmet, saw % %.', v_class, v_deps;
  end if;

  -- Q3: its only dependency is done, so unblocked.
  select claim_eligibility, unmet_deps into v_class, v_deps from relay.task_queue
   where task_id = '11111111-1111-7000-8000-00000000fc23';
  if v_class is distinct from 'ready_unblocked' or v_deps is distinct from '{}'::uuid[] then
    raise exception 'Q3 should read ready_unblocked (dep done), saw % %.', v_class, v_deps;
  end if;

  -- Q4 floats; Q6 is claimed, and says by whom.
  select claim_eligibility into v_class from relay.task_queue
   where task_id = '11111111-1111-7000-8000-00000000fc24';
  if v_class is distinct from 'review_floating' then
    raise exception 'Q4 should read review_floating, saw %.', v_class;
  end if;
  select count(*) into n from relay.task_queue
   where task_id = '11111111-1111-7000-8000-00000000fc26' and claim_eligibility = 'review_claimed'
     and review_claimed_by = '11111111-1111-7000-8000-00000000fc11'
     and author_lane_id = '11111111-1111-7000-8000-00000000fc12';
  if n <> 1 then
    raise exception 'Q6 should read review_claimed by L1, authored by L2.';
  end if;

  -- The view is security-invoker.
  select count(*) into n from pg_catalog.pg_class c
    join pg_catalog.pg_namespace s on s.oid = c.relnamespace
   cross join pg_catalog.pg_options_to_table(c.reloptions) o
   where s.nspname = 'relay' and c.relname = 'task_queue'
     and o.option_name = 'security_invoker' and o.option_value = 'true';
  if n <> 1 then
    raise exception 'relay.task_queue must be security-invoker.';
  end if;
end $$;

-- U2 reads their one row; U1 their five.
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"a2222222-2222-4222-8222-222222222222","role":"authenticated"}', true);
do $$
declare
  n int;
begin
  select count(*) into n from relay.task_queue;
  if n <> 1 then
    raise exception 'U2 should read 1 queue row, saw %.', n;
  end if;
end $$;
reset role;

set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"a1111111-1111-4111-8111-111111111111","role":"authenticated"}', true);
do $$
declare
  n int;
begin
  select count(*) into n from relay.task_queue;
  if n <> 5 then
    raise exception 'U1 should read 5 queue rows, saw %.', n;
  end if;
end $$;
reset role;

rollback;
