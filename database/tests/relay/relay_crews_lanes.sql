-- `relay.crews` + `relay.lanes` — who crews are and who is registered.
--
-- WHAT BREAKS THIS TEST: the workspace gate returning (it must stay optional);
-- `family` / `token_hash` / `status` turning nullable; a second coordinator
-- per crew; a duplicate code per crew; a lane on a missing crew; a non-v7 id;
-- a non-owner reading or writing a row.
--
-- Run from database/tests/ (`run.sh relay`, `run.sh --apply sensei`
-- fixtures/relay_apply.ddl relay/relay_crews_lanes.sql`, or psql $SENSEI_DATABASE_URL -f relay/relay_crews_lanes.sql):
-- the file applies its own DDL in-txn (see below) and rolls back.
-- with --apply the runner applies the DDL and the file skips its own apply.
--
-- NOTE (Q6 ruling): a coordinator minting a crew may name no repository, so
-- both `crews.repo_key` and the coordinator lane's `repo` are nullable — a
-- lane with no checkout yet, honestly null. Contributors register with one.

begin;

-- The real DDL, in this transaction, rolled back with it.
\ir ../fixtures/relay_apply.ddl

-- U1 owns C1 (workspace W1, founded on a repo) and C2 (neither); U2 owns C3.
insert into relay.crews (user_id, id, workspace_id, repo_key, goal, status) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f301',
   'b1111111-1111-4111-8111-111111111111', 'github.com/sensei-hq/sensei', 'Dogfood the next item', 'active'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f302',
   null, null, 'Spare crew', 'active'),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000f303',
   null, 'github.com/sensei-hq/other', 'Their crew', 'active');

-- C1: coordinator L1 + contributor L2. C2: contributor L3. L4/L5 pin the
-- per-crew uniques (same code another crew; a coordinator per crew).
insert into relay.lanes
  (user_id, id, crew_id, code, title, role, family, version, model, capabilities, repo, worktree, token_hash) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f311',
   '11111111-1111-7000-8000-00000000f301', 'L1', 'Diagrams', 'coordinator',
   'claude-code', '2.1', 'claude-opus-5-5', '{ts,rust}', 'github.com/sensei-hq/sensei', 'lanes/L1',
   '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f312',
   '11111111-1111-7000-8000-00000000f301', 'L2', null, 'contributor',
   'claude-code', null, null, '{sql}', 'github.com/sensei-hq/sensei', null,
   '60303ae22b998861bce3b28f33eec1be758a213c86c93c076dbe9f558c11c7c4'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f313',
   '11111111-1111-7000-8000-00000000f302', 'L2', null, 'contributor',
   'codex', null, null, '{mockups}', 'github.com/sensei-hq/sensei', null,
   'df9b6720d8b57c4a2ab0d0c4b6e8b5c0b6e8b5c0d8b57c4a2ab0d0c4b6e8b00'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f314',
   '11111111-1111-7000-8000-00000000f302', 'L1', null, 'contributor',
   'codex', null, null, '{sql}', 'github.com/sensei-hq/sensei', null,
   'e99a18c428cb38d5f260853678922e03abd8f64b1ef0a3aea5b0a1b2c3d4e5f6'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f315',
   '11111111-1111-7000-8000-00000000f302', 'L3', null, 'coordinator',
   'codex', null, null, '{}', 'github.com/sensei-hq/sensei', null,
   'a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e'),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000f316',
   '22222222-2222-7000-8000-00000000f303', 'L1', null, 'coordinator',
   'claude-code', null, null, '{ts}', 'github.com/sensei-hq/other', null,
   'b5bb9d8014a0f9b1d61e21e796d78dccdf1352f23cd32812f4850b878ae4944');

do $$
declare
  n int;
  v_text text;
  v_arr text[];
  v_ts timestamptz;
begin
  -- A crew roundtrips; the server stamps the start, nothing else is invented.
  select goal into v_text from relay.crews where id = '11111111-1111-7000-8000-00000000f301';
  if v_text is distinct from 'Dogfood the next item' then
    raise exception 'crew goal should read back, saw %.', v_text;
  end if;
  select started_at into v_ts from relay.crews where id = '11111111-1111-7000-8000-00000000f301';
  if v_ts is null then
    raise exception 'crews.started_at must default when omitted.';
  end if;
  select count(*) into n from relay.crews
   where id = '11111111-1111-7000-8000-00000000f301' and completed_at is null and heartbeat_at is null;
  if n <> 1 then
    raise exception 'completed_at/heartbeat_at must stay null until set.';
  end if;

  -- The workspace stays optional: C2 has none and reads fine.
  select count(*) into n from relay.crews
   where id = '11111111-1111-7000-8000-00000000f302' and workspace_id is null;
  if n <> 1 then
    raise exception 'a crew without a workspace must read back.';
  end if;

  -- ... but the status is required.
  begin
    insert into relay.crews (user_id, id, goal) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f321', 'No status');
    raise exception 'crews.status accepted null.';
  exception when not_null_violation then null;
  end;

  -- A lane roundtrips with its server stamps and nulls.
  select count(*) into n from relay.lanes
   where id = '11111111-1111-7000-8000-00000000f311'
     and registered_at is not null and unregistered_at is null and checkpoint is null;
  if n <> 1 then
    raise exception 'lane stamps/nulls wrong on read-back.';
  end if;

  -- Capabilities default to the empty list when the register names none.
  insert into relay.lanes (user_id, id, crew_id, code, role, family, repo, token_hash) values
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f322',
     '11111111-1111-7000-8000-00000000f302', 'L4', 'contributor', 'codex',
     'github.com/sensei-hq/sensei', 'cafebeefcafebeefcafebeefcafebeefcafebeefcafebeefcafebeefcafebeef');
  select capabilities into v_arr from relay.lanes where id = '11111111-1111-7000-8000-00000000f322';
  if v_arr is distinct from '{}' then
    raise exception 'capabilities should default to {}, saw %.', v_arr;
  end if;

  -- The crew's founding repo reads back; a crew minted without one reads null.
  select repo_key into v_text from relay.crews where id = '11111111-1111-7000-8000-00000000f301';
  if v_text is distinct from 'github.com/sensei-hq/sensei' then
    raise exception 'crews.repo_key should read back, saw %.', v_text;
  end if;
  select count(*) into n from relay.crews
   where id = '11111111-1111-7000-8000-00000000f302' and repo_key is null;
  if n <> 1 then
    raise exception 'a crew minted without a repo must read null.';
  end if;

  -- A lane with no checkout yet reads null (the role is incidental to the pin;
  -- the case that forces it is a coordinator minted without a repo).
  insert into relay.lanes (user_id, id, crew_id, code, role, family, token_hash) values
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f323',
     '11111111-1111-7000-8000-00000000f302', 'L5', 'contributor', 'codex', 'x');
  select count(*) into n from relay.lanes
   where id = '11111111-1111-7000-8000-00000000f323' and repo is null;
  if n <> 1 then
    raise exception 'a lane with no repo must read null.';
  end if;
  delete from relay.lanes where id = '11111111-1111-7000-8000-00000000f323';

  -- family and the token hash are required; the rest may wait.
  begin
    insert into relay.lanes (user_id, id, crew_id, code, role, repo, token_hash) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f324',
       '11111111-1111-7000-8000-00000000f302', 'L6', 'contributor',
       'github.com/sensei-hq/sensei', 'x');
    raise exception 'lanes.family accepted null.';
  exception when not_null_violation then null;
  end;
  begin
    insert into relay.lanes (user_id, id, crew_id, code, role, family, repo) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f325',
       '11111111-1111-7000-8000-00000000f302', 'L7', 'contributor', 'codex',
       'github.com/sensei-hq/sensei');
    raise exception 'lanes.token_hash accepted null.';
  exception when not_null_violation then null;
  end;

  -- The code is unique per crew, not globally (L4 above proves the other crew).
  begin
    insert into relay.lanes (user_id, id, crew_id, code, role, family, repo, token_hash) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f326',
       '11111111-1111-7000-8000-00000000f301', 'L1', 'contributor', 'codex',
       'github.com/sensei-hq/sensei', 'x');
    raise exception 'a duplicate lane code on one crew was accepted.';
  exception when unique_violation then null;
  end;

  -- One coordinator per crew: a second is refused, contributors are not.
  begin
    insert into relay.lanes (user_id, id, crew_id, code, role, family, repo, token_hash) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f327',
       '11111111-1111-7000-8000-00000000f301', 'L8', 'coordinator', 'codex',
       'github.com/sensei-hq/sensei', 'x');
    raise exception 'a second coordinator on one crew was accepted.';
  exception when unique_violation then null;
  end;

  -- A lane on a missing crew fails the foreign key.
  begin
    insert into relay.lanes (user_id, id, crew_id, code, role, family, repo, token_hash) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f328',
       '11111111-1111-7000-8000-00000000f399', 'L9', 'contributor', 'codex',
       'github.com/sensei-hq/sensei', 'x');
    raise exception 'a lane on a missing crew was accepted.';
  exception when foreign_key_violation then null;
  end;

  -- Ids are v7: a v4 id is refused on both tables.
  begin
    insert into relay.crews (user_id, id, status) values
      ('a1111111-1111-4111-8111-111111111111', '99999999-9999-4999-8999-999999999999', 'active');
    raise exception 'a v4 crew id was accepted.';
  exception when check_violation then null;
  end;
  begin
    insert into relay.lanes (user_id, id, crew_id, code, role, family, repo, token_hash) values
      ('a1111111-1111-4111-8111-111111111111', '99999999-9999-4999-8999-999999999998',
       '11111111-1111-7000-8000-00000000f301', 'Lx', 'contributor', 'codex',
       'github.com/sensei-hq/sensei', 'x');
    raise exception 'a v4 lane id was accepted.';
  exception when check_violation then null;
  end;

  -- The daemon owner sees every row (3 crews, 7 lanes: six fixtures + L4).
  select count(*) into n from relay.crews
   where user_id in ('a1111111-1111-4111-8111-111111111111', 'a2222222-2222-4222-8222-222222222222');
  if n <> 3 then
    raise exception 'the owner should see 3 crews, saw %.', n;
  end if;
  select count(*) into n from relay.lanes
   where user_id in ('a1111111-1111-4111-8111-111111111111', 'a2222222-2222-4222-8222-222222222222');
  if n <> 7 then
    raise exception 'the owner should see 7 lanes, saw %.', n;
  end if;
end $$;

-- U1 reads exactly their own rows (2 crews, 6 lanes).
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"a1111111-1111-4111-8111-111111111111","role":"authenticated"}', true);
do $$
declare
  n int;
begin
  select count(*) into n from relay.crews;
  if n <> 2 then
    raise exception 'U1 should read 2 crews, saw %.', n;
  end if;
  select count(*) into n from relay.lanes;
  if n <> 6 then
    raise exception 'U1 should read 6 lanes, saw %.', n;
  end if;

  -- ... and writes none: no write grant, so every write is refused.
  begin
    insert into relay.crews (user_id, id, status) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f331', 'active');
    raise exception 'an authenticated insert into crews was allowed.';
  exception when insufficient_privilege then null;
  end;
  begin
    update relay.crews set status = 'done' where id = '11111111-1111-7000-8000-00000000f301';
    raise exception 'an authenticated update of crews was allowed.';
  exception when insufficient_privilege then null;
  end;
  begin
    delete from relay.crews where id = '11111111-1111-7000-8000-00000000f301';
    raise exception 'an authenticated delete from crews was allowed.';
  exception when insufficient_privilege then null;
  end;
end $$;
reset role;

-- U2 reads exactly their own (1 crew, 1 lane).
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"a2222222-2222-4222-8222-222222222222","role":"authenticated"}', true);
do $$
declare
  n int;
begin
  select count(*) into n from relay.crews;
  if n <> 1 then
    raise exception 'U2 should read 1 crew, saw %.', n;
  end if;
  select count(*) into n from relay.lanes;
  if n <> 1 then
    raise exception 'U2 should read 1 lane, saw %.', n;
  end if;
end $$;
reset role;

rollback;
