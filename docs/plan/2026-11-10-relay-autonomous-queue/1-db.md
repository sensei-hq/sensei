# Phase 1 — DB (additive)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land every relay schema change the autonomous queue needs that the slice-1 daemon can run on unchanged, with SQL pins, so the repository has a green, committed stopping point before the daemon work.

**Architecture:** Additive only: new enum labels appended, two new enums, three new tables, six new columns, one new view, two new `relay.now` sections, the #405 partial index. The destructive half (drop `reviewer_lane_id`, split the one-active index, ready-coherence check, `state default 'ready'`, nullable `assigned_at`, lane section without `in_review`) is committed as a pending SQL red and lands in phase 2 task 2.1.

**Tech stack:** dbd 0.27.9 DDL files, `database/tests/run.sh` (fresh `sensei_test_<lane>` from the tree), `make test-template` (Rust template), `cargo test -p senseid relay -- --test-threads=1`.

**Conventions enforced by `make test-fast`:** `current_timestamp` not `now()` in relay DDL (`check-no-now-relay`); no two consecutive comment lines inside a column list (`check-ddl-comments`); grants only to roles `apply/before/roles.sql` creates (`check-grant-targets`); every table and non-obvious column carries `comment on`.

---

## File map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `database/ddl/enum/relay/task_state.ddl` | append `ready` |
| Modify | `database/ddl/enum/relay/task_phase.ddl` | append `planning` |
| Modify | `database/ddl/enum/relay/action_kind.ddl` | append `claimed`, `review_claimed`, `released`, `checkpointed` |
| Create | `database/ddl/enum/relay/run_state.ddl` | sprint run lifecycle |
| Create | `database/ddl/enum/relay/task_executor.ddl` | `agent` / `broker` |
| Create | `database/ddl/table/relay/runs.ddl` | sprint runs |
| Create | `database/ddl/table/relay/run_items.ddl` | run ↔ backlog items |
| Create | `database/ddl/table/relay/lane_waiters.ddl` | long-poll waiter registry (F06) |
| Modify | `database/ddl/table/relay/tasks.ddl` | `run_id`, `review_claimed_by/_at`, `executor`, `deterministic`, `checkpoint`; FKs, checks, three indexes |
| Modify | `database/ddl/table/relay/lanes.ddl` | `suggested_name`, role default, #405 index |
| Modify | `database/ddl/view/relay/now.ddl` | lane `suggested_name` + `reviewing`; sections `sprint_summary`, `floating_review` |
| Create | `database/ddl/view/relay/task_queue.ddl` | read-only claim queue (F01) |
| Create | `database/policies/relay/{runs,run_items,lane_waiters,task_queue}.sql` | owner-read RLS + grants |
| Modify | `database/tests/fixtures/relay_apply.ddl` | apply the new files in dependency order |
| Modify | `database/design.yaml` | dojo scope includes the two new enums |
| Modify | `database/tests/relay/relay_enums.sql` | 11 enums, appended labels |
| Modify | `database/tests/relay/relay_schema.sql` | 11 tables, 3 views, grants on 10 |
| Modify | `database/tests/relay/relay_crews_lanes.sql` | role default, `suggested_name`, departed coordinator |
| Modify | `database/tests/relay/relay_tasks.sql` | new columns, review-claim uniques, executor checks, run FK, `ready` label |
| Modify | `database/tests/relay/relay_now.sql` | six sections, `reviewing`, sprint counts, floating rows |
| Create | `database/tests/relay/relay_runs.sql` | runs + run_items |
| Create | `database/tests/relay/relay_lane_waiters.sql` | waiters |
| Create | `database/tests/relay/relay_task_queue.sql` | eligibility classes |
| Create | `database/tests/relay/relay_tasks_cutover.sql.pending` | phase-2 reds |

---

## Task 1.1: Enum labels and the two new enums

**Files:** the five enum files above, `database/tests/relay/relay_enums.sql`.

- [x] **Step 1: Write the failing pins.** In `relay_enums.sql`: header says "eleven vocabularies"; `n <> 11`; `task_phase` pin becomes `array['analysis', 'tests', 'impl', 'docs', 'mockup', 'planning']`; `task_state` pin appends `'ready'`; `action_kind` pin appends `'claimed', 'review_claimed', 'released', 'checkpointed'`; add two blocks:

```sql
  select array_agg(e.enumlabel order by e.enumsortorder) into v_labels
    from pg_catalog.pg_enum e join pg_catalog.pg_type t on t.oid = e.enumtypid
    join pg_catalog.pg_namespace s on s.oid = t.typnamespace
   where s.nspname = 'relay' and t.typname = 'run_state';
  if v_labels is distinct from array['planning', 'active', 'completed', 'cancelled'] then
    raise exception 'run_state labels wrong, saw %.', v_labels;
  end if;

  select array_agg(e.enumlabel order by e.enumsortorder) into v_labels
    from pg_catalog.pg_enum e join pg_catalog.pg_type t on t.oid = e.enumtypid
    join pg_catalog.pg_namespace s on s.oid = t.typnamespace
   where s.nspname = 'relay' and t.typname = 'task_executor';
  if v_labels is distinct from array['agent', 'broker'] then
    raise exception 'task_executor labels wrong, saw %.', v_labels;
  end if;
```

- [x] **Step 2: Run it red.** `cd database/tests && ./run.sh relay/relay_enums.sql` → FAIL `relay must hold exactly 11 enums, saw 9`.

- [x] **Step 3: Write the DDL.**

`database/ddl/enum/relay/task_state.ddl`:
```sql
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
```

`database/ddl/enum/relay/task_phase.ddl`:
```sql
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
```

`database/ddl/enum/relay/action_kind.ddl`:
```sql
set search_path to relay, extensions;

-- What happened, one row per event. The kind decides which props keys the
-- daemon validates and which fold of the task view reads them. The last four
-- are the self-service queue's: a task or review claimed, a claim released
-- back to the pool, an in-turn checkpoint saved.
create type relay.action_kind as enum (
  'assigned',
  'brief_updated',
  'started',
  'reported',
  'reviewed',
  'asked',
  'answered',
  'message',
  'delivered',
  'read',
  'merged',
  'interrupted',
  'reoffered',
  'compacted',
  'handed_over',
  'claimed',
  'review_claimed',
  'released',
  'checkpointed'
);

comment on type relay.action_kind is
'delivered/read are receipts: they fold into their message, never entries of their own. claimed/review_claimed/released/checkpointed are the queue''s hand-offs.';
```

`database/ddl/enum/relay/run_state.ddl`:
```sql
set search_path to relay, extensions;

-- Where a sprint run stands: Task 0 under way, the plan unpacked and tasks
-- flowing, every task done, or stopped by the person.
create type relay.run_state as enum (
  'planning',
  'active',
  'completed',
  'cancelled'
);

comment on type relay.run_state is
'planning until the peer-approved plan unpacks; active until every task is done or cancelled; completed/cancelled are terminal.';
```

`database/ddl/enum/relay/task_executor.ddl`:
```sql
set search_path to relay, extensions;

-- Who runs a task: an assistant lane (a cognitive turn) or the broker itself
-- (a deterministic machine operation: a merge gate, a build, a cleanup).
create type relay.task_executor as enum (
  'agent',
  'broker'
);

comment on type relay.task_executor is
'agent = claimed by a lane; broker = the daemon runs it from tasks.deterministic, never a lane.';
```

- [x] **Step 4: Register the new enums** in `database/tests/fixtures/relay_apply.ddl` after `limit_scope.ddl`:
```
\ir ../../ddl/enum/relay/run_state.ddl
\ir ../../ddl/enum/relay/task_executor.ddl
```
and in `database/design.yaml` under the dojo scope `includes:` after `- relay.action_kind`:
```yaml
      - relay.run_state
      - relay.task_executor
```

- [x] **Step 5: Run it green.** `./run.sh relay/relay_enums.sql` → `ok`.

---

## Task 1.2: `relay.runs`, `relay.run_items`, `relay.lane_waiters`

**Files:** the three table files, three policy files, `relay_apply.ddl`, `relay_runs.sql`, `relay_lane_waiters.sql`, `relay_schema.sql`.

- [x] **Step 1: Write the failing pins.**

`database/tests/relay/relay_runs.sql`:
```sql
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
```

`database/tests/relay/relay_lane_waiters.sql`:
```sql
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
```

`relay_schema.sql` edits: "Eight tables" → eleven (`n <> 11`); `user_id` NOT NULL on nine (`table_name in ('crews','lanes','leases','items','tasks','actions','runs','run_items','lane_waiters')`, `n <> 9`); PK(user_id, id) on seven (add `'runs'`, `n <> 7`); add two PK pins:

```sql
  -- The two junction-shaped tables key on user_id first too (the sync key's
  -- leading column), then their natural key.
  select count(*) into n from (
    select c.relname, (select array_agg(a.attname::text order by array_position(con.conkey, a.attnum))
              from pg_catalog.pg_attribute a
             where a.attrelid = c.oid and a.attnum = any (con.conkey)) as cols
      from pg_catalog.pg_constraint con
      join pg_catalog.pg_class c on c.oid = con.conrelid
      join pg_catalog.pg_namespace s on s.oid = c.relnamespace
     where s.nspname = 'relay' and con.contype = 'p'
  ) pk where (relname = 'run_items' and cols = array['user_id', 'run_id', 'item_id'])
          or (relname = 'lane_waiters' and cols = array['user_id', 'lane_id']);
  if n <> 2 then
    raise exception 'run_items must be PK(user_id, run_id, item_id) and lane_waiters PK(user_id, lane_id).';
  end if;
```

RLS on 11 (`n <> 11`, message "all 11"); policies `n < 11`; the four grant blocks list ten tables (add `('runs'), ('run_items'), ('lane_waiters')`; messages "all 10"); the views pin: `relname in ('now', 'task_history', 'task_queue')`, `n <> 3`.

- [x] **Step 2: Run them red.** `./run.sh relay/relay_runs.sql` → FAIL `relation "relay.runs" does not exist`; same for waiters; schema → `relay must hold 11 tables, saw 8`.

- [x] **Step 3: Write the DDL.**

`database/ddl/table/relay/runs.ddl`:
```sql
set search_path to relay, extensions;

-- A sprint run: backlog items grouped under a goal by the person. Minting a
-- run dispatches Task 0 (phase planning) into the queue; a peer-approved plan
-- unpacks into ready tasks and the run turns active; it completes itself when
-- every task is done. The crew is where the lanes are; the run is what they
-- are working through.
create table if not exists relay.runs (
  user_id      uuid            not null,
  id           uuid            not null,
  crew_id      uuid            not null,
  title        text            not null,
  goal         text            not null,
  state        relay.run_state not null default 'planning',
  config       jsonb           not null default '{}',
  created_at   timestamptz     not null default current_timestamp,
  started_at   timestamptz,
  completed_at timestamptz,
  constraint runs_pkey primary key (user_id, id),
  constraint runs_id_v7 check (substring(id::text from 15 for 1) = '7'),
  constraint runs_crew_fk foreign key (user_id, crew_id) references relay.crews (user_id, id)
);

create index if not exists runs_crew_open_idx
  on relay.runs (user_id, crew_id) where state in ('planning', 'active');

alter table relay.runs enable row level security;

comment on table relay.runs is
'Sprint runs. Minted by the person from backlog items; Task 0 plans, the approved plan unpacks, the run completes when every task is done.';
comment on column relay.runs.goal is 'The sprint goal in the person''s words: what Task 0 decomposes.';
comment on column relay.runs.config is 'Gate inputs and knobs: base_checkout, base_branch, verify_cmd, auto_merge. Empty until the person sets them.';
comment on column relay.runs.started_at is 'When the plan unpacked (planning → active). Null while planning.';
comment on column relay.runs.completed_at is 'When the last task reached done. Null until then.';
```

`database/ddl/table/relay/run_items.ddl`:
```sql
set search_path to relay, extensions;

-- The backlog items a sprint run groups, in the order the person picked them.
-- An item may be on one run at a time only by convention (the daemon checks);
-- the key is (run, item) so the same item never doubles up on one run.
create table if not exists relay.run_items (
  user_id  uuid        not null,
  run_id   uuid        not null,
  item_id  uuid        not null,
  position int         not null default 0,
  added_at timestamptz not null default current_timestamp,
  constraint run_items_pkey primary key (user_id, run_id, item_id),
  constraint run_items_run_fk foreign key (user_id, run_id) references relay.runs (user_id, id) on delete cascade,
  constraint run_items_item_fk foreign key (user_id, item_id) references relay.items (user_id, id) on delete cascade
);

alter table relay.run_items enable row level security;

comment on table relay.run_items is
'Backlog items on a sprint run, in pick order. Cascades with the run and with the item.';
comment on column relay.run_items.position is 'Pick order within the run, 0-based.';
```

`database/ddl/table/relay/lane_waiters.ddl`:
```sql
set search_path to relay, extensions;

-- Who is holding tasks/wait open right now (F06). One row per waiting lane,
-- inserted when the hold begins and deleted when it answers; the sweeper
-- reads waiting_since to nudge idle lanes and evicts rows whose lane lease
-- lapsed. Never synced: a wait is a local fact.
create table if not exists relay.lane_waiters (
  user_id       uuid        not null,
  lane_id       uuid        not null,
  crew_id       uuid        not null,
  waiting_since timestamptz not null default current_timestamp,
  last_poll_at  timestamptz not null default current_timestamp,
  constraint lane_waiters_pkey primary key (user_id, lane_id),
  constraint lane_waiters_lane_fk foreign key (user_id, lane_id) references relay.lanes (user_id, id) on delete cascade,
  constraint lane_waiters_crew_fk foreign key (user_id, crew_id) references relay.crews (user_id, id)
);

alter table relay.lane_waiters enable row level security;

comment on table relay.lane_waiters is
'Active long-poll waiters, one per lane. Inserted on hold, deleted on answer; the sweeper nudges by waiting_since and evicts dead lanes.';
comment on column relay.lane_waiters.waiting_since is 'When the lane first found nothing to do (not reset by a re-poll that still finds nothing).';
comment on column relay.lane_waiters.last_poll_at is 'The most recent hold the lane opened.';
```

Policies (`database/policies/relay/runs.sql`; `run_items.sql` and `lane_waiters.sql` are the same text with the table name swapped):
```sql
-- RLS for relay.runs: a row is readable by its owner, and by others only by an
-- explicit share (none yet). Writes are the daemon owner's.
alter table relay.runs enable row level security;

drop policy if exists runs_owner_read on relay.runs;
create policy runs_owner_read on relay.runs
    for select to authenticated
    using (user_id::text = (nullif(current_setting('request.jwt.claims', true), '')::json ->> 'sub'));

grant select on relay.runs to authenticated;
```

`relay_apply.ddl` ordering: `runs.ddl` after `crews.ddl` and before `tasks.ddl`; `run_items.ddl` after `items.ddl`; `lane_waiters.ddl` after `lanes.ddl`; the three policy files after `policies/relay/tasks.sql`.

- [x] **Step 4: Run them green.** `./run.sh relay/relay_runs.sql relay/relay_lane_waiters.sql` → `ok`; `relay_schema.sql` still red on the views pin until Task 1.4.

---

## Task 1.3: `relay.tasks` and `relay.lanes` columns, checks, indexes

**Files:** `tasks.ddl`, `lanes.ddl`, `relay_tasks.sql`, `relay_crews_lanes.sql`.

- [x] **Step 1: Write the failing pins.**

Append to the main `do $$` block of `relay_tasks.sql`, before the "owner sees twelve" count:
```sql
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
```
Also update the header comment ("WHAT BREAKS THIS TEST") to name: a lane claiming its own review; two review claims on one lane; a broker task with no spec; a task on a missing run.

Append to `relay_crews_lanes.sql` after the "second coordinator refused" block (line ~174):
```sql
  -- A departed coordinator holds the slot no longer (#405): once L1 leaves,
  -- a replacement registers; the history row stays.
  update relay.lanes set unregistered_at = now() where id = '11111111-1111-7000-8000-00000000f311';
  insert into relay.lanes (user_id, id, crew_id, code, role, family, repo, token_hash) values
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f32a',
     '11111111-1111-7000-8000-00000000f301', 'L8', 'coordinator', 'codex',
     'github.com/sensei-hq/sensei', 'x');
  delete from relay.lanes where id = '11111111-1111-7000-8000-00000000f32a';
  update relay.lanes set unregistered_at = null where id = '11111111-1111-7000-8000-00000000f311';

  -- A register that names no role joins as a contributor; the assistant's
  -- own name is kept beside the code, null when it offered none.
  insert into relay.lanes (user_id, id, crew_id, code, family, token_hash, suggested_name) values
    ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f32b',
     '11111111-1111-7000-8000-00000000f302', 'L9', 'codex', 'x', 'worker-alpha');
  select count(*) into n from relay.lanes
   where id = '11111111-1111-7000-8000-00000000f32b' and role = 'contributor' and suggested_name = 'worker-alpha';
  if n <> 1 then
    raise exception 'lanes.role must default to contributor and suggested_name must read back.';
  end if;
  select count(*) into n from relay.lanes
   where id = '11111111-1111-7000-8000-00000000f322' and suggested_name is null;
  if n <> 1 then
    raise exception 'suggested_name must default null.';
  end if;
  delete from relay.lanes where id = '11111111-1111-7000-8000-00000000f32b';
```
Header: add "a departed coordinator still holding the slot; `role` losing its contributor default".

- [x] **Step 2: Run them red.** `./run.sh relay/relay_tasks.sql relay/relay_crews_lanes.sql` → FAIL `column "run_id" of relation "tasks" does not exist` / `a second coordinator ... was accepted` (unique_violation surfaces as the uncaught error).

- [x] **Step 3: Write the DDL.**

`database/ddl/table/relay/tasks.ddl` (whole file):
```sql
set search_path to relay, extensions;

-- What a lane works on. Repository required (the headline rule: work on a
-- workspace-less repository must still appear); workspace optional. One lane
-- holds one active task; a task never reviews itself; a blocked task remembers
-- where it was (Q5) so the resume lands back there. The queue's columns: the
-- run a task belongs to, who holds its review claim, who executes it, the
-- in-turn checkpoint.
create table if not exists relay.tasks (
  user_id            uuid                not null,
  id                 uuid                not null,
  crew_id            uuid                not null,
  run_id             uuid,
  item_id            uuid,
  repo_key           text                not null,
  workspace_id       uuid,
  lane_id            uuid,
  reviewer_lane_id   uuid,
  review_claimed_by  uuid,
  review_claimed_at  timestamptz,
  title              text                not null,
  phase              relay.task_phase    not null,
  state              relay.task_state    not null default 'assigned',
  round              int                 not null default 1,
  executor           relay.task_executor not null default 'agent',
  deterministic      jsonb               not null default '{}',
  owned_paths        text[]              not null default '{}',
  acceptance         text[]              not null default '{}',
  resources          text[]              not null default '{}',
  deps               uuid[]              not null default '{}',
  blocked_from_state relay.task_state,
  assigned_at        timestamptz         not null default current_timestamp,
  state_changed_at   timestamptz         not null default current_timestamp,
  completed_at       timestamptz,
  checkpoint         jsonb               not null default '{}',
  steps              jsonb               not null default '[]',
  constraint tasks_pkey primary key (user_id, id),
  constraint tasks_id_v7 check (substring(id::text from 15 for 1) = '7'),
  constraint tasks_crew_fk foreign key (user_id, crew_id) references relay.crews (user_id, id),
  constraint tasks_run_fk foreign key (user_id, run_id) references relay.runs (user_id, id),
  constraint tasks_item_fk foreign key (user_id, item_id) references relay.items (user_id, id),
  constraint tasks_lane_fk foreign key (user_id, lane_id) references relay.lanes (user_id, id),
  constraint tasks_reviewer_fk foreign key (user_id, reviewer_lane_id) references relay.lanes (user_id, id),
  constraint tasks_review_claim_fk foreign key (user_id, review_claimed_by) references relay.lanes (user_id, id),
  constraint tasks_no_self_review check
    (reviewer_lane_id is null or lane_id is null or reviewer_lane_id <> lane_id),
  constraint tasks_no_self_review_claim check
    (review_claimed_by is null or lane_id is null or review_claimed_by <> lane_id),
  constraint tasks_round_from_1 check (round >= 1),
  constraint tasks_blocked_memory check ((state = 'blocked') = (blocked_from_state is not null)),
  constraint tasks_broker_deterministic check (executor <> 'broker' or deterministic <> '{}'::jsonb)
);

create unique index if not exists tasks_one_active_per_lane
  on relay.tasks (user_id, lane_id) where lane_id is not null
  and state in ('assigned', 'working', 'in_review', 'rework', 'blocked');

create unique index if not exists tasks_one_active_review_per_lane
  on relay.tasks (user_id, review_claimed_by) where review_claimed_by is not null
  and state = 'in_review';

create index if not exists tasks_claim_queue_idx
  on relay.tasks (user_id, crew_id, state, state_changed_at)
  where executor = 'agent' and state in ('ready', 'in_review');

create index if not exists tasks_broker_queue_idx
  on relay.tasks (user_id, crew_id, state, state_changed_at)
  where executor = 'broker' and state = 'ready';

alter table relay.tasks enable row level security;

comment on table relay.tasks is
'Tasks. The active-task unique is (user_id, lane_id) over the five lane-holding states; the review-claim unique is (user_id, review_claimed_by) over in_review.';
comment on column relay.tasks.run_id is 'The sprint run this task belongs to. Null for tasks planned outside a run.';
comment on column relay.tasks.repo_key is 'Required: the repository this work belongs to. No row without one.';
comment on column relay.tasks.reviewer_lane_id is 'Slice 1: the picked reviewer. Retired by the floating review claim (review_claimed_by) at the queue cutover.';
comment on column relay.tasks.review_claimed_by is 'The lane reviewing this task. Null while the review floats on the queue.';
comment on column relay.tasks.review_claimed_at is 'When the reviewer claimed. The sweeper clears a claim whose lane lease lapsed.';
comment on column relay.tasks.executor is 'agent: a lane claims and works it. broker: the daemon runs tasks.deterministic.';
comment on column relay.tasks.deterministic is 'When executor = broker: {action, manifest, target, timeout_seconds, ...}. Empty for agent tasks.';
comment on column relay.tasks.deps is 'Task ids this one waits on. Plain uuids, no FK: graphs, not trees. A report is gated until every dep is done.';
comment on column relay.tasks.blocked_from_state is 'Where the task was when it blocked (Q5). Set on block, cleared on resume.';
comment on column relay.tasks.state_changed_at is 'Every transition stamps it. completed_today falls back to it when completed_at is null.';
comment on column relay.tasks.checkpoint is 'In-turn checkpoint ({current_step, notes, files, ...}) saved by the holder; kept across interrupt and re-offer.';
comment on column relay.tasks.steps is 'Flat checklist (F8): [{key, title, state, reason, updated_at}], insertion order. The holder posts it turn by turn; the task view serves it as-is.';
```

`database/ddl/table/relay/lanes.ddl` changes: the column list gains `suggested_name text,` after `title`; `role relay.lane_role not null default 'contributor',`; the coordinator index becomes:
```sql
create unique index if not exists lanes_one_coordinator
  on relay.lanes (user_id, crew_id) where role = 'coordinator' and unregistered_at is null;
```
plus comments:
```sql
comment on column relay.lanes.suggested_name is 'The name the assistant proposed at register (worker-alpha). Null when it offered none; the code stays the handle.';
comment on column relay.lanes.role is 'contributor by default: the broker coordinates. A coordinator lane is optional and one per crew while registered.';
```

- [x] **Step 4: Run them green.** `./run.sh relay/relay_tasks.sql relay/relay_crews_lanes.sql` → `ok`.

---

## Task 1.4: `relay.task_queue` view and the `relay.now` sections

**Files:** `view/relay/task_queue.ddl`, `view/relay/now.ddl`, `policies/relay/task_queue.sql`, `relay_apply.ddl`, `relay_task_queue.sql`, `relay_now.sql`, `relay_schema.sql`.

- [x] **Step 1: Write the failing pins.**

`database/tests/relay/relay_task_queue.sql`:
```sql
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
  ('a2222222-2222-4222-8222-222222222222', '22222222-2222-7000-8000-00000000fc19',
   '22222222-2222-7000-8000-00000000fc02', 'L1', 'claude-code', 'x');

-- Q5: done (out, and a satisfied dependency). Q1: ready, no deps. Q2: ready,
-- waits on Q1 (unmet). Q3: ready, waits on Q5 (met). Q4: in review, floating
-- (author L1). Q6: in review, claimed by L1 (author L2). Q7: working (out).
-- Q9: U2's ready task.
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
   '11111111-1111-7000-8000-00000000fc01', '11111111-1111-7000-8000-00000000fc11', null, null,
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
```

`relay_now.sql` edits (fixtures first, after the existing `relay.lanes` insert and before the leases insert):
```sql
-- L5/L6: contributors on C1 with no lease (authors of the two reviews below);
-- L5 registered under its own name.
insert into relay.lanes (user_id, id, crew_id, code, family, token_hash, suggested_name) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f915',
   '11111111-1111-7000-8000-00000000f901', 'L5', 'codex', 'x', 'worker-alpha'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f916',
   '11111111-1111-7000-8000-00000000f901', 'L6', 'codex', 'x', null);

-- R1: active on C1 (T1 + the three tasks below belong to it). R2: planning on
-- C2, no tasks yet. R3: completed (out of sprint_summary).
insert into relay.runs (user_id, id, crew_id, title, goal, state, started_at, completed_at) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f961',
   '11111111-1111-7000-8000-00000000f901', 'Sprint 1', 'Ship it', 'active', now(), null),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f962',
   '11111111-1111-7000-8000-00000000f902', 'Sprint 2', 'Plan it', 'planning', null, null),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f963',
   '11111111-1111-7000-8000-00000000f901', 'Sprint 0', 'Done with', 'completed', now(), now());
```
After the existing tasks inserts:
```sql
-- T13: ready in the pool. T14: in review, floating (author L5). T15: in
-- review, claimed by L1 (author L6) — L1 works T1 and reviews T15 at once.
update relay.tasks set run_id = '11111111-1111-7000-8000-00000000f961'
 where id = '11111111-1111-7000-8000-00000000f931';
insert into relay.tasks (user_id, id, crew_id, run_id, lane_id, review_claimed_by, review_claimed_at, repo_key, title, phase, state, round, state_changed_at) values
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f93c',
   '11111111-1111-7000-8000-00000000f901', '11111111-1111-7000-8000-00000000f961', null, null, null,
   'github.com/sensei-hq/sensei', 'Ready one', 'impl', 'ready', 1, now()),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f93d',
   '11111111-1111-7000-8000-00000000f901', '11111111-1111-7000-8000-00000000f961',
   '11111111-1111-7000-8000-00000000f915', null, null,
   'github.com/sensei-hq/sensei', 'Floating review', 'tests', 'in_review', 2, '2026-11-05T17:30:00Z'),
  ('a1111111-1111-4111-8111-111111111111', '11111111-1111-7000-8000-00000000f93e',
   '11111111-1111-7000-8000-00000000f901', '11111111-1111-7000-8000-00000000f961',
   '11111111-1111-7000-8000-00000000f916', '11111111-1111-7000-8000-00000000f911', '2026-11-05T17:40:00Z',
   'github.com/sensei-hq/sensei', 'Claimed review', 'impl', 'in_review', 1, '2026-11-05T17:35:00Z');
```
Assertions: the sections pin becomes `array['completed_today', 'floating_review', 'lane', 'open_asked', 'sprint_summary', 'triage_count']`; the L1 payload block adds `or v_json->>'suggested_name' is not null` to the "wrong" condition and a new block:
```sql
  -- L1 reviews T15 while working T1; L5 carries its suggested name and reads
  -- no review; L6's own task is in review (slice 1: still its active task).
  if v_json->'reviewing'->>'id' is distinct from '11111111-1111-7000-8000-00000000f93e'
     or v_json->'reviewing'->>'title' is distinct from 'Claimed review'
     or v_json->'reviewing'->>'phase' is distinct from 'impl'
     or (v_json->'reviewing'->>'claimed_at')::timestamptz is distinct from '2026-11-05T17:40:00Z'::timestamptz then
    raise exception 'L1 reviewing payload wrong: %.', v_json->'reviewing';
  end if;
  select payload into v_json from relay.now where section = 'lane' and id = '11111111-1111-7000-8000-00000000f915';
  if v_json->>'suggested_name' is distinct from 'worker-alpha' or v_json->'reviewing' is distinct from 'null'::jsonb then
    raise exception 'L5 payload wrong: %.', v_json;
  end if;

  -- Sprint summary: R1 active with its four tasks counted by state; R2
  -- planning with none; R3 completed is out.
  select count(*) into n from relay.now where section = 'sprint_summary'
   and user_id = 'a1111111-1111-4111-8111-111111111111';
  if n <> 2 then
    raise exception 'sprint_summary should hold R1 + R2, saw %.', n;
  end if;
  select payload into v_json from relay.now where section = 'sprint_summary' and id = '11111111-1111-7000-8000-00000000f961';
  if v_json->>'title' is distinct from 'Sprint 1' or v_json->>'state' is distinct from 'active'
     or (v_json->>'total_tasks')::int is distinct from 4
     or (v_json->>'ready_tasks')::int is distinct from 1
     or (v_json->>'working_tasks')::int is distinct from 1
     or (v_json->>'in_review_tasks')::int is distinct from 2
     or (v_json->>'floating_reviews')::int is distinct from 1
     or (v_json->>'done_tasks')::int is distinct from 0 then
    raise exception 'R1 sprint_summary wrong: %.', v_json;
  end if;
  select payload into v_json from relay.now where section = 'sprint_summary' and id = '11111111-1111-7000-8000-00000000f962';
  if v_json->>'state' is distinct from 'planning' or (v_json->>'total_tasks')::int is distinct from 0 then
    raise exception 'R2 sprint_summary wrong: %.', v_json;
  end if;
  select crew_id::text into v_text from relay.now where section = 'sprint_summary' and id = '11111111-1111-7000-8000-00000000f962';
  if v_text is distinct from '11111111-1111-7000-8000-00000000f902' then
    raise exception 'R2 should carry crew C2, saw %.', v_text;
  end if;

  -- Floating reviews: T14 only (T15 is claimed), with its author.
  select count(*) into n from relay.now where section = 'floating_review'
   and user_id = 'a1111111-1111-4111-8111-111111111111';
  if n <> 1 then
    raise exception 'floating_review should hold T14 only, saw %.', n;
  end if;
  select payload into v_json from relay.now where section = 'floating_review' and id = '11111111-1111-7000-8000-00000000f93d';
  if v_json->>'title' is distinct from 'Floating review' or v_json->>'author_lane_code' is distinct from 'L5'
     or v_json->>'run_id' is distinct from '11111111-1111-7000-8000-00000000f961'
     or (v_json->>'round')::int is distinct from 2
     or (v_json->>'reported_at')::timestamptz is distinct from '2026-11-05T17:30:00Z'::timestamptz then
    raise exception 'T14 floating_review payload wrong: %.', v_json;
  end if;
```
Check the rest of the file for lane-count or per-user row counts that the two new lanes and three tasks shift (the `completed_today` count is unaffected: none of the new tasks is approved/done); update any such pin and the header's "WHAT BREAKS" line ("a section outside the six").

- [x] **Step 2: Run them red.** `./run.sh relay/relay_task_queue.sql relay/relay_now.sql` → FAIL `relation "relay.task_queue" does not exist` / `relay.now sections wrong`.

- [x] **Step 3: Write the DDL.**

`database/ddl/view/relay/task_queue.ddl`:
```sql
set search_path to relay, extensions;

-- The claimable work, read-only (F01: a read never starts work). One row per
-- task in the pool or in review, classed for the claim order: a floating
-- review first, then a ready task whose dependencies are all done, then a
-- ready task that may be claimed early (its unmet dependencies listed; the
-- report is gated until they are done). A claimed review reads as such.
-- Security-invoker: the caller's RLS applies.
create or replace view relay.task_queue with (security_invoker = true) as
select
  t.user_id,
  t.crew_id,
  t.run_id,
  t.id as task_id,
  t.title,
  t.phase,
  t.state,
  t.round,
  t.repo_key,
  t.owned_paths,
  t.acceptance,
  t.lane_id as author_lane_id,
  t.review_claimed_by,
  case
    when t.state = 'in_review' and t.review_claimed_by is null then 'review_floating'
    when t.state = 'in_review' then 'review_claimed'
    when not exists (
      select 1 from relay.tasks d
       where d.user_id = t.user_id and d.id = any (t.deps) and d.state <> 'done')
      then 'ready_unblocked'
    else 'ready_claim_early'
  end as claim_eligibility,
  coalesce((
    select array_agg(d.id order by d.id) from relay.tasks d
     where d.user_id = t.user_id and d.id = any (t.deps) and d.state <> 'done'),
    '{}'::uuid[]) as unmet_deps,
  t.state_changed_at
from relay.tasks t
where t.state in ('ready', 'in_review');

comment on view relay.task_queue is
'The claim queue: review_floating, review_claimed, ready_unblocked, ready_claim_early. Read-only; claims go through the daemon.';
```

`database/policies/relay/task_queue.sql`:
```sql
-- Read grant for relay.task_queue. No policy: it is security-invoker, so the
-- caller's RLS on relay.tasks applies.
grant select on relay.task_queue to authenticated;
```

`database/ddl/view/relay/now.ddl` changes — the header comment lists six sections; the lane payload gains, after `'title', l.title,`: `'suggested_name', l.suggested_name,` and, after the `'task'` entry:
```sql
    'reviewing', case when tr.id is null then null
      else jsonb_build_object(
        'id', tr.id, 'title', tr.title, 'phase', tr.phase, 'round', tr.round,
        'claimed_at', tr.review_claimed_at) end
```
with the join after the `t` join:
```sql
left join relay.tasks tr on tr.user_id = l.user_id and tr.review_claimed_by = l.id
  and tr.state = 'in_review'
```
Two new sections, inserted after the `person_message` section (before `triage_count`):
```sql
union all
-- One row per open sprint run with its tasks counted by state: the band
-- above the lanes. Planning runs count their Task 0; completed runs leave.
select r.user_id, 'sprint_summary'::text as section, r.crew_id, r.id,
  jsonb_build_object(
    'run_id', r.id, 'title', r.title, 'goal', r.goal, 'state', r.state,
    'created_at', r.created_at, 'started_at', r.started_at,
    'total_tasks', count(t.id),
    'ready_tasks', count(t.id) filter (where t.state = 'ready'),
    'working_tasks', count(t.id) filter (where t.state in ('assigned', 'working', 'rework')),
    'in_review_tasks', count(t.id) filter (where t.state = 'in_review'),
    'floating_reviews', count(t.id) filter (where t.state = 'in_review' and t.review_claimed_by is null),
    'approved_tasks', count(t.id) filter (where t.state = 'approved'),
    'done_tasks', count(t.id) filter (where t.state = 'done'),
    'blocked_tasks', count(t.id) filter (where t.state = 'blocked'),
    'interrupted_tasks', count(t.id) filter (where t.state = 'interrupted')) as payload
from relay.runs r
left join relay.tasks t on t.user_id = r.user_id and t.run_id = r.id
where r.state in ('planning', 'active')
group by r.user_id, r.crew_id, r.id
union all
-- One row per report waiting for a reviewer: the review queue's head, which
-- any non-author lane claims next.
select t.user_id, 'floating_review'::text as section, t.crew_id, t.id,
  jsonb_build_object(
    'task_id', t.id, 'run_id', t.run_id, 'title', t.title, 'phase', t.phase, 'round', t.round,
    'repo_key', t.repo_key, 'author_lane_code', l.code, 'author_lane_title', l.title,
    'reported_at', t.state_changed_at) as payload
from relay.tasks t
left join relay.lanes l on l.user_id = t.user_id and l.id = t.lane_id
where t.state = 'in_review' and t.review_claimed_by is null
```
and the view comment names all six sections.

`relay_apply.ddl`: `\ir ../../ddl/view/relay/task_queue.ddl` after `now.ddl`; `\ir ../../policies/relay/task_queue.sql` after `task_history.sql`.

- [x] **Step 4: Run them green.** `./run.sh relay/relay_task_queue.sql relay/relay_now.sql relay/relay_schema.sql` → `ok` ×3.

---

## Task 1.5: The cutover reds, pending

**Files:** Create `database/tests/relay/relay_tasks_cutover.sql.pending` (run.sh only picks `*.sql`, so it does not run until phase 2 renames it).

- [x] **Step 1: Write it.**
```sql
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
```

- [x] **Step 2:** `./run.sh relay` → the file is not listed (pending). `psql "$(./lib/sensei-target.sh; sensei_lane_db_url)" -f relay/relay_tasks_cutover.sql.pending` → FAIL `tasks.reviewer_lane_id must be dropped at the cutover` (proves it is a real red).

---

## Task 1.6: Converge the test databases, prove the daemon unaffected, commit

- [x] **Step 1: Fresh lane DB + the whole relay suite.** `dropdb --if-exists -h localhost sensei_test_develop` (run.sh derives the name from the branch); `cd database/tests && ./run.sh relay` → every file `ok`, including the unchanged `relay_actions`, `relay_leases`, `relay_items`, `relay_finding_classes`, `relay_task_history`.
- [x] **Step 2: Lint gates.** `make check-ddl-grants check-ddl-comments check-no-now-relay check-sql-in-layer` → PASS. `cd database && dbd inspect` → no errors.
- [x] **Step 3: Preview the live delta.** `cd database && dbd reconcile --dry-run -d postgresql://localhost:5432/sensei_test` → the plan lists only CREATEs and ADD COLUMN/ADD VALUE/ADD CONSTRAINT; no DROP. If it lists an index it cannot alter (`lanes_one_coordinator`), note it: that index is recreated by `make test-template` (fresh) and by the cutover's manual step on the live DB.
- [x] **Step 4: Rebuild the Rust template.** `make test-template` → `sensei_test template: … MB`.
- [x] **Step 5: The daemon relay suite on the new schema.** `cargo test -p senseid relay -- --test-threads=1` (memory: serial; a killed run leaves stale `activity.runs` — `DELETE FROM activity.runs` on `sensei_test` if a scheduler test hangs) → all green. Also `cargo test -p senseid --lib relay_lease_sweep`.
- [x] **Step 6: Pre-commit gate.** `make test-fast` → PASS.
- [x] **Step 7: Commit, one concern.** `git add database/ddl database/policies database/tests database/design.yaml && git diff --cached --stat` (only those paths) → `git commit -m "feat(relay): additive queue schema — runs, run_items, lane_waiters, review claims, broker tasks, task_queue view, now sections (autonomous queue phase 1)"`.
- [x] **Step 8: Checkpoint.** `/sensei:checkpoint "relay-autonomous-queue: phase 1 DB additive committed; next: phase 2 task 2.1 cutover"` and mirror into `docs/CHECKPOINT.md` (slice; done/remaining; next command `see docs/plan/2026-11-10-relay-autonomous-queue/2-daemon.md task 2.1`; open questions; known-broken none).
- [x] **Step 9: Do NOT converge the live daemon DB yet.** The running daemon does not use the new columns; the live reconcile is phase 2 task 2.10, run together with the daemon install.

---

## Executed 2026-11-12 (`b9eb57ac2` on develop) — deviations from the steps above

- `run.sh` takes a scope directory, not a file: single files ran as `psql <lane-db-url> -X -q -v ON_ERROR_STOP=1 -f relay/<file>.sql`. The red/green loop used a scratch database with no `relay` schema (`createdb sensei_test_rq1`, then `apply/before/roles.sql` and `create schema extensions`) so the fixture self-applies the current tree on every run; the fresh `sensei_test_develop` mint (`dbd deploy`) was the final proof.
- `relay_task_queue.sql`: Q7 (working) sits on its own lane L3. The slice-1 `tasks_one_active_per_lane` still counts `in_review`, so Q4 and Q7 could not share L1 before the cutover.
- `task_queue.ddl` `unmet_deps` is `array(select … order by d.id)`, not `coalesce(array_agg …)`: `check-no-fabrication` rule S1 flags `coalesce` in SQL, and the array constructor is honest-empty without it.
- `relay_lane_waiters.sql` stayed red through Task 1.2 on `lanes.role` NOT NULL (its fixture omits `role`) and went green with Task 1.3's contributor default.
- `dbd diff -v` plans `DROP INDEX … lanes_one_coordinator` + `CREATE` for the predicate change: dbd 0.27.9 does converge it, under `--allow-destructive`.
- The `now` lane payload also carries `round` inside `reviewing`, and the `now` pins check `author_lane_title`, `started_at`, the four zero counts and the crew id of the floating row, beyond what the step text listed.
