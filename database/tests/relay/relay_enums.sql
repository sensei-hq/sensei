-- `relay` enums — the model's eight vocabularies, labels and order pinned.
--
-- WHAT BREAKS THIS TEST: a renamed/reordered/dropped label; a ninth enum in
-- `relay`; `item_state` losing `triage` (lanes-data's "one extra state").
--
-- Run from database/tests/ (`run.sh relay`, `run.sh --apply sensei`
-- fixtures/relay_apply.ddl relay/relay_enums.sql`, or psql $SENSEI_DATABASE_URL -f relay/relay_enums.sql):
-- the file applies its own DDL in-txn (see below) and rolls back.
-- with --apply the runner applies the DDL and the file skips its own apply.
--
-- NOTE (for L0): `item_state` leads with `triage` (lifecycle order:
-- triage → backlog → planned → closed). The rest follow lanes-data's listed
-- order. Rule if otherwise.

begin;

-- The real DDL, in this transaction, rolled back with it.
\ir ../fixtures/relay_apply.ddl

do $$
declare
  v_labels text[];
  n int;
begin
  -- The schema holds exactly the eight vocabularies: a ninth enum is a model
  -- change, not a quiet addition.
  select count(*) into n from pg_catalog.pg_type t
    join pg_catalog.pg_namespace s on s.oid = t.typnamespace
   where s.nspname = 'relay' and t.typtype = 'e';
  if n <> 8 then
    raise exception 'relay must hold exactly 8 enums, saw %.', n;
  end if;

  select array_agg(e.enumlabel order by e.enumsortorder) into v_labels
    from pg_catalog.pg_enum e join pg_catalog.pg_type t on t.oid = e.enumtypid
    join pg_catalog.pg_namespace s on s.oid = t.typnamespace
   where s.nspname = 'relay' and t.typname = 'lane_role';
  if v_labels is distinct from array['coordinator', 'contributor'] then
    raise exception 'lane_role labels wrong, saw %.', v_labels;
  end if;

  select array_agg(e.enumlabel order by e.enumsortorder) into v_labels
    from pg_catalog.pg_enum e join pg_catalog.pg_type t on t.oid = e.enumtypid
    join pg_catalog.pg_namespace s on s.oid = t.typnamespace
   where s.nspname = 'relay' and t.typname = 'lease_kind';
  if v_labels is distinct from array['lane', 'resource'] then
    raise exception 'lease_kind labels wrong, saw %.', v_labels;
  end if;

  select array_agg(e.enumlabel order by e.enumsortorder) into v_labels
    from pg_catalog.pg_enum e join pg_catalog.pg_type t on t.oid = e.enumtypid
    join pg_catalog.pg_namespace s on s.oid = t.typnamespace
   where s.nspname = 'relay' and t.typname = 'lease_state';
  if v_labels is distinct from array['queued', 'granted', 'released', 'expired', 'refused'] then
    raise exception 'lease_state labels wrong, saw %.', v_labels;
  end if;

  select array_agg(e.enumlabel order by e.enumsortorder) into v_labels
    from pg_catalog.pg_enum e join pg_catalog.pg_type t on t.oid = e.enumtypid
    join pg_catalog.pg_namespace s on s.oid = t.typnamespace
   where s.nspname = 'relay' and t.typname = 'item_kind';
  if v_labels is distinct from array['bug', 'feature', 'gap', 'issue', 'finding'] then
    raise exception 'item_kind labels wrong, saw %.', v_labels;
  end if;

  select array_agg(e.enumlabel order by e.enumsortorder) into v_labels
    from pg_catalog.pg_enum e join pg_catalog.pg_type t on t.oid = e.enumtypid
    join pg_catalog.pg_namespace s on s.oid = t.typnamespace
   where s.nspname = 'relay' and t.typname = 'item_state';
  if v_labels is distinct from array['triage', 'backlog', 'planned', 'closed'] then
    raise exception 'item_state labels wrong, saw %.', v_labels;
  end if;

  select array_agg(e.enumlabel order by e.enumsortorder) into v_labels
    from pg_catalog.pg_enum e join pg_catalog.pg_type t on t.oid = e.enumtypid
    join pg_catalog.pg_namespace s on s.oid = t.typnamespace
   where s.nspname = 'relay' and t.typname = 'task_phase';
  if v_labels is distinct from array['analysis', 'tests', 'impl', 'docs', 'mockup'] then
    raise exception 'task_phase labels wrong, saw %.', v_labels;
  end if;

  select array_agg(e.enumlabel order by e.enumsortorder) into v_labels
    from pg_catalog.pg_enum e join pg_catalog.pg_type t on t.oid = e.enumtypid
    join pg_catalog.pg_namespace s on s.oid = t.typnamespace
   where s.nspname = 'relay' and t.typname = 'task_state';
  if v_labels is distinct from
      array['assigned', 'working', 'in_review', 'rework', 'approved', 'done', 'blocked', 'interrupted', 'cancelled'] then
    raise exception 'task_state labels wrong, saw %.', v_labels;
  end if;

  select array_agg(e.enumlabel order by e.enumsortorder) into v_labels
    from pg_catalog.pg_enum e join pg_catalog.pg_type t on t.oid = e.enumtypid
    join pg_catalog.pg_namespace s on s.oid = t.typnamespace
   where s.nspname = 'relay' and t.typname = 'action_kind';
  if v_labels is distinct from
      array['assigned', 'brief_updated', 'started', 'reported', 'reviewed', 'asked', 'answered',
            'message', 'delivered', 'read', 'merged', 'interrupted', 'reoffered', 'compacted'] then
    raise exception 'action_kind labels wrong, saw %.', v_labels;
  end if;
end $$;

rollback;
