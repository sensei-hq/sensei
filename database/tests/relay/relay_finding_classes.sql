-- `relay.finding_classes` — the shared vocabulary of review findings.
--
-- WHAT BREAKS THIS TEST: the seed drifting (a code renamed, dropped or added
-- outside the learning loop); the label going nullable; the table gaining
-- `user_id` (it is one vocabulary, seeded identically both sides); anyone but
-- the daemon owner writing it; anyone unable to read it.
--
-- Run from database/tests/ (`run.sh relay`, `run.sh --apply sensei`
-- fixtures/relay_apply.ddl relay/relay_finding_classes.sql`, or psql $SENSEI_DATABASE_URL -f relay/relay_finding_classes.sql):
-- the file applies its own DDL in-txn (see below) and rolls back.
-- with --apply the runner applies the DDL and the file skips its own apply.
--
-- NOTE (for L0): the ten classes are proposed from the README test-design
-- checklist; the gate's two (merge_conflict, build_failure) came with the
-- queue cutover. The daemon rejects unknown classes (item 2); the learning
-- loop adds rows with `added_by` set. Rule on the set if otherwise.

begin;

-- The real DDL, in this transaction, rolled back with it.
\ir ../fixtures/relay_apply.ddl

-- The seed through the real path: the JSONL into staging, then the proc.
-- (\copy resolves from the psql cwd: run from database/tests/.)
create temp table fc_seed (line text);
\copy fc_seed from '../import/staging/finding_classes.jsonl'
insert into staging.finding_classes (code, label)
  select line::jsonb->>'code', line::jsonb->>'label' from fc_seed;
call staging.import_finding_classes();

do $$
declare
  n int;
  v_codes text[];
  v_labels text[];
begin
  -- The seed: the ten proposed classes plus the gate's two (queue cutover,
  -- task 2.8: a merge conflict, a failed build or verify), code and label
  -- pinned.
  select array_agg(code order by code), array_agg(label order by code)
    into v_codes, v_labels from relay.finding_classes where added_by is null;
  if v_codes is distinct from
      array['build_failure', 'fabricated_default', 'flaky_time', 'merge_conflict', 'missing_failure_test',
            'role_gap', 'secret_leak', 'silent_drop', 'skipped_live', 'surviving_mutant', 'tenant_leak',
            'unverified_claim'] then
    raise exception 'finding_classes seed codes wrong, saw %.', v_codes;
  end if;
  if v_labels is distinct from
      array['Build or verify failure', 'Fabricated default', 'Wall-clock or string-time compare', 'Merge conflict',
            'Missing failure test', 'Role-matrix gap', 'Secret in response or log', 'Silent drop (D83)',
            'Live test skipped', 'Surviving mutant', 'Tenant leak', 'Claim without evidence'] then
    raise exception 'finding_classes seed labels wrong, saw %.', v_labels;
  end if;

  -- Seeded rows name no adder; the loop's rows will.
  select count(*) into n from relay.finding_classes
   where added_by is null and code = any (v_codes);
  if n <> 12 then
    raise exception 'the twelve seeded classes must leave added_by null, saw % null.', n;
  end if;

  -- The code is the key; the label is required.
  begin
    insert into relay.finding_classes (code, label) values ('silent_drop', 'Dupe');
    raise exception 'a duplicate finding class was accepted.';
  exception when unique_violation then null;
  end;
  begin
    insert into relay.finding_classes (code) values ('no_label');
    raise exception 'a labelless finding class was accepted.';
  exception when not_null_violation then null;
  end;

  -- A loop-added class roundtrips (then removed: the seed stays twelve).
  insert into relay.finding_classes (code, label, added_by) values
    ('stale_mock', 'Stale mock', 'a1111111-1111-4111-8111-111111111111');
  select count(*) into n from relay.finding_classes where code = 'stale_mock';
  if n <> 1 then
    raise exception 'an added class should read back.';
  end if;
  delete from relay.finding_classes where code = 'stale_mock';

  -- The daemon owner sees the twelve.
  select count(*) into n from relay.finding_classes where added_by is null;
  if n <> 12 then
    raise exception 'the owner should see 12 classes, saw %.', n;
  end if;

  -- A re-run over doubled staging rows is a no-op, not an error.
  insert into staging.finding_classes (code, label)
    select code, label from staging.finding_classes;
  call staging.import_finding_classes();
  select count(*) into n from relay.finding_classes where added_by is null;
  if n <> 12 then
    raise exception 'a seed re-run should keep 12 classes, saw %.', n;
  end if;
end $$;

-- The vocabulary is shared: both users read all twelve, neither writes.
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"a1111111-1111-4111-8111-111111111111","role":"authenticated"}', true);
do $$
declare
  n int;
begin
  select count(*) into n from relay.finding_classes where added_by is null;
  if n <> 12 then
    raise exception 'U1 should read 12 classes, saw %.', n;
  end if;
  begin
    insert into relay.finding_classes (code, label) values ('user_class', 'User class');
    raise exception 'an authenticated insert into finding_classes was allowed.';
  exception when insufficient_privilege then null;
  end;
end $$;
reset role;

set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"a2222222-2222-4222-8222-222222222222","role":"authenticated"}', true);
do $$
declare
  n int;
begin
  select count(*) into n from relay.finding_classes where added_by is null;
  if n <> 12 then
    raise exception 'U2 should read 12 classes, saw %.', n;
  end if;
end $$;
reset role;

rollback;
