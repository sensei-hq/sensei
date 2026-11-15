-- `relay.runs` + `relay.run_items` — a sprint run and the backlog items it groups.
--
-- WHAT BREAKS THIS TEST: `state` losing its planning default or taking a
-- label outside run_state; `title`/`goal` turning nullable; a run on a missing
-- crew; a non-v7 id; the same item twice on one run; an item that is not in
-- relay.items; run_items surviving their run; a non-owner reading a row;
-- `authenticated` gaining a write grant.
--
-- Run from database/tests/ (`run.sh relay`, `run.sh --apply sensei`
-- fixtures/relay_apply.ddl relay/relay_runs.sql`, or psql $SENSEI_DATABASE_URL -f relay/relay_runs.sql):
-- the file applies its own DDL in-txn (see below) and rolls back.
-- with --apply the runner applies the DDL and the file skips its own apply.

begin;

-- The real DDL, in this transaction, rolled back with it.
\ir ../fixtures/relay_apply.ddl

insert into relay.crews (user_id, id, status) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa01', 'active'),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000fa02', 'active');

insert into relay.items (user_id, id, title, kind, state) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa11', 'Claim queue', 'feature', 'backlog'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa12', 'Floating reviews', 'feature', 'backlog'),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000fa19', 'Their item', 'bug', 'backlog');

-- R1: minted with defaults. R2: explicitly active. R9: U2's.
insert into relay.runs (user_id, id, crew_id, title, goal) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa21',
   '11111111-1111-7000-8000-00000000fa01', 'Sprint 1', 'Ship the queue');
insert into relay.runs (user_id, id, crew_id, title, goal, state, started_at) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa22',
   '11111111-1111-7000-8000-00000000fa01', 'Sprint 2', 'Ship reviews', 'active', now()),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000fa29',
   '22222222-2222-7000-8000-00000000fa02', 'Theirs', 'Their goal', 'planning', null);

insert into relay.run_items (user_id, run_id, item_id, position) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa21',
   '11111111-1111-7000-8000-00000000fa11', 0),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa21',
   '11111111-1111-7000-8000-00000000fa12', 1),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000fa29',
   '22222222-2222-7000-8000-00000000fa19', 0);

do $$
declare
  n int;
  v_state text;
  v_config jsonb;
begin
  -- A run minted with defaults is planning, configless, stamped, not started.
  select state::text, config into v_state, v_config from relay.runs
   where id = '11111111-1111-7000-8000-00000000fa21';
  if v_state is distinct from 'planning' or v_config is distinct from '{}'::jsonb then
    raise exception 'run defaults wrong: state %, config %.', v_state, v_config;
  end if;
  select count(*) into n from relay.runs
   where id = '11111111-1111-7000-8000-00000000fa21'
     and created_at is not null and started_at is null and completed_at is null;
  if n <> 1 then
    raise exception 'run stamps wrong on read-back.';
  end if;

  -- title and goal are required.
  begin
    insert into relay.runs (user_id, id, crew_id, goal) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa31',
       '11111111-1111-7000-8000-00000000fa01', 'No title');
    raise exception 'runs.title accepted null.';
  exception when not_null_violation then null;
  end;
  begin
    insert into relay.runs (user_id, id, crew_id, title) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa32',
       '11111111-1111-7000-8000-00000000fa01', 'No goal');
    raise exception 'runs.goal accepted null.';
  exception when not_null_violation then null;
  end;

  -- The state is the enum: a word outside it is refused.
  begin
    insert into relay.runs (user_id, id, crew_id, title, goal, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa33',
       '11111111-1111-7000-8000-00000000fa01', 'Bad state', 'x', 'paused');
    raise exception 'runs.state accepted a label outside run_state.';
  exception when invalid_text_representation then null;
  end;

  -- A run on a missing crew fails the foreign key.
  begin
    insert into relay.runs (user_id, id, crew_id, title, goal) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa34',
       '11111111-1111-7000-8000-00000000fa99', 'Orphan', 'x');
    raise exception 'a run on a missing crew was accepted.';
  exception when foreign_key_violation then null;
  end;

  -- Ids are v7.
  begin
    insert into relay.runs (user_id, id, crew_id, title, goal) values
      ('a1111111-1111-4111-8111-111111111111', '99999999-9999-4999-8999-999999999991',
       '11111111-1111-7000-8000-00000000fa01', 'v4', 'x');
    raise exception 'a v4 run id was accepted.';
  exception when check_violation then null;
  end;

  -- An item is on a run once.
  begin
    insert into relay.run_items (user_id, run_id, item_id, position) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa21',
       '11111111-1111-7000-8000-00000000fa11', 5);
    raise exception 'the same item twice on one run was accepted.';
  exception when unique_violation then null;
  end;

  -- ... and must exist.
  begin
    insert into relay.run_items (user_id, run_id, item_id) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa21',
       '11111111-1111-7000-8000-00000000fa98');
    raise exception 'a run item naming no relay.items row was accepted.';
  exception when foreign_key_violation then null;
  end;

  -- position defaults to 0 and added_at stamps.
  insert into relay.run_items (user_id, run_id, item_id) values
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fa22',
     '11111111-1111-7000-8000-00000000fa11');
  select count(*) into n from relay.run_items
   where run_id = '11111111-1111-7000-8000-00000000fa22' and position = 0 and added_at is not null;
  if n <> 1 then
    raise exception 'run_items defaults wrong.';
  end if;

  -- Deleting a run takes its items with it (the items themselves stay).
  delete from relay.runs where id = '11111111-1111-7000-8000-00000000fa22';
  select count(*) into n from relay.run_items where run_id = '11111111-1111-7000-8000-00000000fa22';
  if n <> 0 then
    raise exception 'run_items must cascade with their run, saw % left.', n;
  end if;
  select count(*) into n from relay.items where id = '11111111-1111-7000-8000-00000000fa11';
  if n <> 1 then
    raise exception 'deleting a run must not delete its items.';
  end if;

  if has_table_privilege('authenticated', 'relay.runs', 'INSERT')
     or has_table_privilege('authenticated', 'relay.run_items', 'INSERT') then
    raise exception 'authenticated must hold no INSERT grant on runs/run_items.';
  end if;
end $$;

-- U1 reads their one run and two items; U2 their one and one.
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"a1111111-1111-4111-8111-111111111111","role":"authenticated"}', true);
do $$
declare
  n int;
begin
  select count(*) into n from relay.runs;
  if n <> 1 then
    raise exception 'U1 should read 1 run, saw %.', n;
  end if;
  select count(*) into n from relay.run_items;
  if n <> 2 then
    raise exception 'U1 should read 2 run items, saw %.', n;
  end if;
end $$;
reset role;

set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"a2222222-2222-4222-8222-222222222222","role":"authenticated"}', true);
do $$
declare
  n int;
begin
  select count(*) into n from relay.runs;
  if n <> 1 then
    raise exception 'U2 should read 1 run, saw %.', n;
  end if;
  select count(*) into n from relay.run_items;
  if n <> 1 then
    raise exception 'U2 should read 1 run item, saw %.', n;
  end if;
end $$;
reset role;

rollback;
