-- `relay.tasks` — what a lane works on. Repository required, workspace optional.
--
-- WHAT BREAKS THIS TEST: `repo_key` turning nullable (the headline rule —
-- work on a workspace-less repository must still appear); a lane holding two
-- active tasks (assigned/working/in_review/rework/blocked); a task reviewing
-- itself; a lane claiming the review of its own task; a lane holding two
-- review claims at once; a broker task with no deterministic spec; a task on
-- a missing run; `ready` not a usable label; a blocked task with no memory of
-- where it was (or any other state carrying one); a non-v7 id; a non-owner
-- reading a row.
--
-- Run from database/tests/ (`run.sh relay`, `run.sh --apply sensei`
-- fixtures/relay_apply.ddl relay/relay_tasks.sql`, or psql $SENSEI_DATABASE_URL -f relay/relay_tasks.sql):
-- the file applies its own DDL in-txn (see below) and rolls back.
-- with --apply the runner applies the DDL and the file skips its own apply.
--
-- NOTE (for L0): "active" is assigned/working/in_review/rework/blocked —
-- approved, done, cancelled and interrupted release the lane. `state`
-- defaults to assigned, `round` to 1, the four lists to {}.
-- `blocked_from_state` is ruled (Q5): set on block, cleared on resume.

begin;

-- The real DDL, in this transaction, rolled back with it.
\ir ../fixtures/relay_apply.ddl

insert into relay.crews (user_id, id, status) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f601', 'active'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f602', 'active'),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000f603', 'active');

insert into relay.lanes (user_id, id, crew_id, code, role, family, repo, token_hash) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f611',
   '11111111-1111-7000-8000-00000000f601', 'L1', 'coordinator', 'claude-code',
   'github.com/sensei-hq/sensei', 'x'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f612',
   '11111111-1111-7000-8000-00000000f601', 'L2', 'contributor', 'claude-code',
   'github.com/sensei-hq/sensei', 'x'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f613',
   '11111111-1111-7000-8000-00000000f602', 'L1', 'coordinator', 'codex',
   'github.com/sensei-hq/sensei', 'x'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f615',
   '11111111-1111-7000-8000-00000000f601', 'L5', 'contributor', 'codex',
   'github.com/sensei-hq/sensei', 'x'),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000f614',
   '22222222-2222-7000-8000-00000000f603', 'L1', 'coordinator', 'claude-code',
   'github.com/sensei-hq/other', 'x');

insert into relay.items (user_id, id, title, kind, state) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f621',
   'Diagrams header on ScreenShell', 'feature', 'backlog');

-- T1: the full row (working on L2, reviewed by L1, no workspace — the repo
-- carries it). T2/T3: approved + done on L1 (terminal states share a lane).
-- T4: cancelled on L2 beside working T1. T5/T6: unassigned working pair (a null
-- lane holds any number). T9: blocked on L3 with its memory. T7: U2's.
insert into relay.tasks
  (user_id, id, crew_id, item_id, repo_key, workspace_id, lane_id, reviewer_lane_id,
   title, phase, state, round, owned_paths, acceptance, resources, deps, blocked_from_state) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f631',
   '11111111-1111-7000-8000-00000000f601', '11111111-1111-7000-8000-00000000f621',
   'github.com/sensei-hq/sensei', null, '11111111-1111-7000-8000-00000000f612',
   '11111111-1111-7000-8000-00000000f611', 'Diagrams header on ScreenShell', 'tests', 'working', 1,
   '{app/src/lib/diagrams/**}', '{"header matches the shell"}', '{rust-build}',
   '{11111111-1111-7000-8000-00000000f634}', null),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f632',
   '11111111-1111-7000-8000-00000000f601', null, 'github.com/sensei-hq/sensei', null,
   '11111111-1111-7000-8000-00000000f611', null, 'Approved one', 'impl', 'approved', 2,
   '{}', '{}', '{}', '{}', null),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f633',
   '11111111-1111-7000-8000-00000000f601', null, 'github.com/sensei-hq/sensei', null,
   '11111111-1111-7000-8000-00000000f611', null, 'Done one', 'impl', 'done', 1,
   '{}', '{}', '{}', '{}', null),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f634',
   '11111111-1111-7000-8000-00000000f601', null, 'github.com/sensei-hq/sensei', null,
   '11111111-1111-7000-8000-00000000f612', null, 'Cancelled one', 'tests', 'cancelled', 1,
   '{}', '{}', '{}', '{}', null),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f635',
   '11111111-1111-7000-8000-00000000f602', null, 'github.com/sensei-hq/sensei', null,
   null, null, 'Unassigned one', 'tests', 'working', 1, '{}', '{}', '{}', '{}', null),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f636',
   '11111111-1111-7000-8000-00000000f602', null, 'github.com/sensei-hq/sensei', null,
   null, null, 'Unassigned two', 'tests', 'working', 1, '{}', '{}', '{}', '{}', null),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f639',
   '11111111-1111-7000-8000-00000000f602', null, 'github.com/sensei-hq/sensei', null,
   '11111111-1111-7000-8000-00000000f613', null, 'Blocked one', 'tests', 'blocked', 1,
   '{}', '{}', '{}', '{}', 'working'),
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000f637',
   '22222222-2222-7000-8000-00000000f603', null, 'github.com/sensei-hq/other', null,
   '22222222-2222-7000-8000-00000000f614', null, 'Their task', 'tests', 'working', 1,
   '{}', '{}', '{}', '{}', null);

do $$
declare
  n int;
  v_text text;
  v_state text;
  v_int int;
begin
  -- The full row roundtrips.
  select title into v_text from relay.tasks where id = '11111111-1111-7000-8000-00000000f631';
  if v_text is distinct from 'Diagrams header on ScreenShell' then
    raise exception 'task title should read back, saw %.', v_text;
  end if;

  -- THE headline rule: no repository, no task.
  begin
    insert into relay.tasks (user_id, id, crew_id, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f651',
       '11111111-1111-7000-8000-00000000f601', 'No repo', 'tests', 'assigned');
    raise exception 'a task without repo_key was accepted.';
  exception when not_null_violation then null;
  end;

  -- Title and phase are required; the workspace, item, lane and reviewer may wait.
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f652',
       '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei', 'tests', 'assigned');
    raise exception 'a task without a title was accepted.';
  exception when not_null_violation then null;
  end;
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, title, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f653',
       '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei', 'No phase', 'assigned');
    raise exception 'a task without a phase was accepted.';
  exception when not_null_violation then null;
  end;

  -- One active task per lane: a second working, assigned or blocked is refused.
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, lane_id, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f654',
       '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei',
       '11111111-1111-7000-8000-00000000f612', 'Second working', 'tests', 'working');
    raise exception 'a second working task on one lane was accepted.';
  exception when unique_violation then null;
  end;
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, lane_id, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f655',
       '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei',
       '11111111-1111-7000-8000-00000000f612', 'Assigned beside working', 'tests', 'assigned');
    raise exception 'an assigned task beside a working one on one lane was accepted.';
  exception when unique_violation then null;
  end;
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, lane_id, title, phase, state, blocked_from_state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f656',
       '11111111-1111-7000-8000-00000000f602', 'github.com/sensei-hq/sensei',
       '11111111-1111-7000-8000-00000000f613', 'Working beside blocked', 'tests', 'working', null);
    raise exception 'a working task beside a blocked one on one lane was accepted.';
  exception when unique_violation then null;
  end;
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, lane_id, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f665',
       '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei',
       '11111111-1111-7000-8000-00000000f612', 'In review beside working', 'tests', 'in_review');
    raise exception 'an in_review task beside a working one on one lane was accepted.';
  exception when unique_violation then null;
  end;
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, lane_id, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f666',
       '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei',
       '11111111-1111-7000-8000-00000000f612', 'Rework beside working', 'tests', 'rework');
    raise exception 'a rework task beside a working one on one lane was accepted.';
  exception when unique_violation then null;
  end;

  -- ... while approved, done and interrupted share the lane (cancelled T4 does).
  insert into relay.tasks (user_id, id, crew_id, repo_key, lane_id, title, phase, state) values
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f641',
     '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei',
     '11111111-1111-7000-8000-00000000f612', 'Approved beside working', 'impl', 'approved'),
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f642',
     '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei',
     '11111111-1111-7000-8000-00000000f612', 'Done beside working', 'impl', 'done'),
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f643',
     '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei',
     '11111111-1111-7000-8000-00000000f612', 'Interrupted beside working', 'impl', 'interrupted');

  -- A task never reviews itself (probed on free lane L5: only the CHECK can fire).
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, lane_id, reviewer_lane_id, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f657',
       '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei',
       '11111111-1111-7000-8000-00000000f615', '11111111-1111-7000-8000-00000000f615',
       'Self review', 'tests', 'working');
    raise exception 'a task reviewing itself was accepted.';
  exception when check_violation then null;
  end;

  -- Bad crew / item / lane / reviewer fail the foreign keys.
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f658',
       '11111111-1111-7000-8000-00000000f699', 'github.com/sensei-hq/sensei', 'No crew', 'tests', 'assigned');
    raise exception 'a task on a missing crew was accepted.';
  exception when foreign_key_violation then null;
  end;
  begin
    insert into relay.tasks (user_id, id, crew_id, item_id, repo_key, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f659',
       '11111111-1111-7000-8000-00000000f601', '11111111-1111-7000-8000-00000000f699',
       'github.com/sensei-hq/sensei', 'No item', 'tests', 'assigned');
    raise exception 'a task on a missing item was accepted.';
  exception when foreign_key_violation then null;
  end;
  begin
    insert into relay.tasks (user_id, id, crew_id, lane_id, repo_key, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f660',
       '11111111-1111-7000-8000-00000000f601', '11111111-1111-7000-8000-00000000f699',
       'github.com/sensei-hq/sensei', 'No lane', 'tests', 'assigned');
    raise exception 'a task on a missing lane was accepted.';
  exception when foreign_key_violation then null;
  end;
  begin
    insert into relay.tasks (user_id, id, crew_id, reviewer_lane_id, repo_key, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f661',
       '11111111-1111-7000-8000-00000000f601', '11111111-1111-7000-8000-00000000f699',
       'github.com/sensei-hq/sensei', 'No reviewer', 'tests', 'assigned');
    raise exception 'a task with a missing reviewer was accepted.';
  exception when foreign_key_violation then null;
  end;

  -- References are user-scoped: a U1 task naming U2's lane fails (composite FK).
  begin
    insert into relay.tasks (user_id, id, crew_id, lane_id, repo_key, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f667',
       '11111111-1111-7000-8000-00000000f601', '22222222-2222-7000-8000-00000000f614',
       'github.com/sensei-hq/sensei', 'Foreign lane', 'tests', 'assigned');
    raise exception 'a task naming another user''s lane was accepted.';
  exception when foreign_key_violation then null;
  end;

  -- Defaults: state assigned, round 1, the four lists empty (on L1, whose
  -- approved + done prove the terminal states release the lane).
  insert into relay.tasks (user_id, id, crew_id, lane_id, repo_key, title, phase) values
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f644',
     '11111111-1111-7000-8000-00000000f601', '11111111-1111-7000-8000-00000000f611',
     'github.com/sensei-hq/sensei', 'Defaults', 'tests');
  select state::text, round into v_state, v_int from relay.tasks
   where id = '11111111-1111-7000-8000-00000000f644';
  if v_state is distinct from 'assigned' or v_int is distinct from 1 then
    raise exception 'state/round should default to assigned/1, saw %/%.', v_state, v_int;
  end if;
  select count(*) into n from relay.tasks
   where id = '11111111-1111-7000-8000-00000000f644'
     and owned_paths = '{}' and acceptance = '{}' and resources = '{}' and deps = '{}';
  if n <> 1 then
    raise exception 'owned_paths/acceptance/resources/deps should default to {}.';
  end if;

  -- The round counts from 1.
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, title, phase, round) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f662',
       '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei', 'Round zero', 'tests', 0);
    raise exception 'round 0 was accepted.';
  exception when check_violation then null;
  end;

  -- Blocked memory (ruled, Q5): blocked carries where it was, and only
  -- blocked carries it. T9 proves the steady state; the probes run on free L5.
  select blocked_from_state::text into v_state from relay.tasks
   where id = '11111111-1111-7000-8000-00000000f639';
  if v_state is distinct from 'working' then
    raise exception 'T9 should remember working, saw %.', v_state;
  end if;
  begin
    insert into relay.tasks (user_id, id, crew_id, lane_id, repo_key, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f663',
       '11111111-1111-7000-8000-00000000f601', '11111111-1111-7000-8000-00000000f615',
       'github.com/sensei-hq/sensei', 'Blocked amnesiac', 'tests', 'blocked');
    raise exception 'a blocked task with no memory was accepted.';
  exception when check_violation then null;
  end;
  begin
    insert into relay.tasks
      (user_id, id, crew_id, lane_id, repo_key, title, phase, state, blocked_from_state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f664',
       '11111111-1111-7000-8000-00000000f601', '11111111-1111-7000-8000-00000000f615',
       'github.com/sensei-hq/sensei', 'Working with memory', 'tests', 'working', 'assigned');
    raise exception 'a working task carrying blocked memory was accepted.';
  exception when check_violation then null;
  end;

  -- Ids are v7.
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '99999999-9999-4999-8999-999999999995',
       '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei', 'v4 task', 'tests', 'assigned');
    raise exception 'a v4 task id was accepted.';
  exception when check_violation then null;
  end;

  -- The queue's columns default: no run, no review claim, agent-run, no
  -- deterministic spec, no checkpoint.
  select count(*) into n from relay.tasks
   where id = '11111111-1111-7000-8000-00000000f631'
     and run_id is null and review_claimed_by is null and review_claimed_at is null
     and executor = 'agent' and deterministic = '{}'::jsonb and checkpoint = '{}'::jsonb;
  if n <> 1 then
    raise exception 'tasks queue-column defaults wrong on T1.';
  end if;

  -- The author never claims its own review (the claim-side twin of the
  -- reviewer rule).
  begin
    update relay.tasks set review_claimed_by = '11111111-1111-7000-8000-00000000f612'
     where id = '11111111-1111-7000-8000-00000000f631';
    raise exception 'a lane claiming the review of its own task was accepted.';
  exception when check_violation then null;
  end;

  -- One active review claim per lane: a second in_review task claimed by L1
  -- is refused while the first is in review.
  insert into relay.tasks (user_id, id, crew_id, repo_key, title, phase, state, review_claimed_by, review_claimed_at) values
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f671',
     '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei', 'Review one', 'impl', 'in_review',
     '11111111-1111-7000-8000-00000000f611', now());
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, title, phase, state, review_claimed_by, review_claimed_at) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f672',
       '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei', 'Review two', 'impl', 'in_review',
       '11111111-1111-7000-8000-00000000f611', now());
    raise exception 'a lane holding two review claims was accepted.';
  exception when unique_violation then null;
  end;
  delete from relay.tasks where id = '11111111-1111-7000-8000-00000000f671';

  -- The executor is the enum; a broker task names its deterministic spec.
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, title, phase, state, executor) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f673',
       '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei', 'Robot', 'impl', 'assigned', 'robot');
    raise exception 'an executor outside task_executor was accepted.';
  exception when invalid_text_representation then null;
  end;
  begin
    insert into relay.tasks (user_id, id, crew_id, repo_key, title, phase, state, executor) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f674',
       '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei', 'Bare broker', 'impl', 'assigned', 'broker');
    raise exception 'a broker task with no deterministic spec was accepted.';
  exception when check_violation then null;
  end;
  insert into relay.tasks (user_id, id, crew_id, repo_key, title, phase, state, executor, deterministic) values
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f675',
     '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei', 'Gate', 'impl', 'assigned', 'broker',
     '{"action": "gate"}');
  delete from relay.tasks where id = '11111111-1111-7000-8000-00000000f675';

  -- A task on a missing run fails the foreign key.
  begin
    insert into relay.tasks (user_id, id, crew_id, run_id, repo_key, title, phase, state) values
      ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f676',
       '11111111-1111-7000-8000-00000000f601', '11111111-1111-7000-8000-00000000f6ff',
       'github.com/sensei-hq/sensei', 'Orphan of a run', 'impl', 'assigned');
    raise exception 'a task on a missing run was accepted.';
  exception when foreign_key_violation then null;
  end;

  -- `ready` is a usable label: an unclaimed pool task reads back.
  insert into relay.tasks (user_id, id, crew_id, repo_key, title, phase, state) values
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f677',
     '11111111-1111-7000-8000-00000000f601', 'github.com/sensei-hq/sensei', 'In the pool', 'impl', 'ready');
  select count(*) into n from relay.tasks
   where id = '11111111-1111-7000-8000-00000000f677' and state = 'ready' and lane_id is null;
  if n <> 1 then
    raise exception 'a ready task must read back with no lane.';
  end if;
  delete from relay.tasks where id = '11111111-1111-7000-8000-00000000f677';

  -- The daemon owner sees all twelve tasks (8 fixtures + 3 coexists + defaults).
  select count(*) into n from relay.tasks
   where user_id in ('a1111111-1111-4111-8111-111111111111', 'a2222222-2222-4222-8222-222222222222');
  if n <> 12 then
    raise exception 'the owner should see 12 tasks, saw %.', n;
  end if;

  if has_table_privilege('authenticated', 'relay.tasks', 'INSERT') then
    raise exception 'authenticated must hold no INSERT grant on tasks.';
  end if;
end $$;

-- U1 reads their eleven; U2 reads their one.
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"a1111111-1111-4111-8111-111111111111","role":"authenticated"}', true);
do $$
declare
  n int;
begin
  select count(*) into n from relay.tasks;
  if n <> 11 then
    raise exception 'U1 should read 11 tasks, saw %.', n;
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
  select count(*) into n from relay.tasks;
  if n <> 1 then
    raise exception 'U2 should read 1 task, saw %.', n;
  end if;
end $$;
reset role;

rollback;
