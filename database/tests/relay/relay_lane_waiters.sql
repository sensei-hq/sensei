-- `relay.lane_waiters` — the long-poll registry (F06): one row per lane that
-- is holding `tasks/wait` open, so the sweeper can see who is idle, for how
-- long, and nudge or evict.
--
-- WHAT BREAKS THIS TEST: two rows for one lane; a waiter on a missing lane; a
-- waiter outliving its lane; `waiting_since`/`last_poll_at` not defaulting; a
-- non-owner reading a row; `authenticated` gaining a write grant.
--
-- Run from database/tests/ (`run.sh relay`, `run.sh --apply sensei`
-- fixtures/relay_apply.ddl relay/relay_lane_waiters.sql`, or psql $SENSEI_DATABASE_URL -f relay/relay_lane_waiters.sql):
-- the file applies its own DDL in-txn (see below) and rolls back.
-- with --apply the runner applies the DDL and the file skips its own apply.

begin;

-- The real DDL, in this transaction, rolled back with it.
\ir ../fixtures/relay_apply.ddl

insert into relay.crews (user_id, id, status) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fb01', 'active'),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000fb02', 'active');

insert into relay.lanes (user_id, id, crew_id, code, family, token_hash) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fb11',
   '11111111-1111-7000-8000-00000000fb01', 'L1', 'claude-code', 'x'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fb12',
   '11111111-1111-7000-8000-00000000fb01', 'L2', 'codex', 'x'),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000fb19',
   '22222222-2222-7000-8000-00000000fb02', 'L1', 'claude-code', 'x');

insert into relay.lane_waiters (user_id, lane_id, crew_id) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fb11',
   '11111111-1111-7000-8000-00000000fb01'),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000fb19',
   '22222222-2222-7000-8000-00000000fb02');

do $$
declare
  n int;
begin
  -- Both stamps default.
  select count(*) into n from relay.lane_waiters
   where lane_id = '11111111-1111-7000-8000-00000000fb11'
     and waiting_since is not null and last_poll_at is not null;
  if n <> 1 then
    raise exception 'lane_waiters stamps must default.';
  end if;

  -- One row per lane.
  begin
    insert into relay.lane_waiters (user_id, lane_id, crew_id) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fb11',
       '11111111-1111-7000-8000-00000000fb01');
    raise exception 'a second waiter row for one lane was accepted.';
  exception when unique_violation then null;
  end;

  -- A waiter on a missing lane, or a missing crew, fails the foreign key.
  begin
    insert into relay.lane_waiters (user_id, lane_id, crew_id) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fb98',
       '11111111-1111-7000-8000-00000000fb01');
    raise exception 'a waiter on a missing lane was accepted.';
  exception when foreign_key_violation then null;
  end;
  begin
    insert into relay.lane_waiters (user_id, lane_id, crew_id) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000fb12',
       '11111111-1111-7000-8000-00000000fb99');
    raise exception 'a waiter on a missing crew was accepted.';
  exception when foreign_key_violation then null;
  end;

  -- A waiter goes with its lane.
  delete from relay.lanes where id = '11111111-1111-7000-8000-00000000fb11';
  select count(*) into n from relay.lane_waiters where lane_id = '11111111-1111-7000-8000-00000000fb11';
  if n <> 0 then
    raise exception 'a waiter must cascade with its lane, saw % left.', n;
  end if;

  if has_table_privilege('authenticated', 'relay.lane_waiters', 'INSERT') then
    raise exception 'authenticated must hold no INSERT grant on lane_waiters.';
  end if;
end $$;

-- U2 reads their one; U1 (whose waiter went with L1) reads none.
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"a2222222-2222-4222-8222-222222222222","role":"authenticated"}', true);
do $$
declare
  n int;
begin
  select count(*) into n from relay.lane_waiters;
  if n <> 1 then
    raise exception 'U2 should read 1 waiter, saw %.', n;
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
  select count(*) into n from relay.lane_waiters;
  if n <> 0 then
    raise exception 'U1 should read 0 waiters, saw %.', n;
  end if;
end $$;
reset role;

rollback;
