-- `relay_archive` shape (#393): seven mirrors of the live crew/run tables,
-- each with its live columns plus `archived_at`; no FK leaves the archive;
-- `lanes.token_hash` is nullable (nulled on archival); RLS on; the history
-- union views resolve. Run from database/tests/ like relay_schema.sql: the
-- file applies its own DDL in-txn and rolls back.
begin;

\ir ../fixtures/relay_apply.ddl
create schema if not exists relay_archive;
\ir ../../ddl/table/relay_archive/runs.ddl
\ir ../../ddl/table/relay_archive/tasks.ddl
\ir ../../ddl/table/relay_archive/run_items.ddl
\ir ../../ddl/table/relay_archive/actions.ddl
\ir ../../ddl/table/relay_archive/crews.ddl
\ir ../../ddl/table/relay_archive/lanes.ddl
\ir ../../ddl/table/relay_archive/leases.ddl
\ir ../../ddl/view/relay/runs_all.ddl
\ir ../../ddl/view/relay/tasks_all.ddl
\ir ../../ddl/view/relay/actions_all.ddl
\ir ../../ddl/view/relay/lanes_all.ddl
\ir ../../ddl/view/relay/run_list.ddl
\ir ../../ddl/view/relay/task_history.ddl
\ir ../../ddl/view/relay/task_queue.ddl

do $$
declare
  n int;
begin
  -- Seven mirrors, no more (waiters are dropped, never archived).
  select count(*) into n from pg_catalog.pg_class c
    join pg_catalog.pg_namespace s on s.oid = c.relnamespace
   where s.nspname = 'relay_archive' and c.relkind = 'r';
  if n <> 7 then
    raise exception 'relay_archive must hold 7 tables, saw %.', n;
  end if;

  -- Every mirror carries archived_at timestamptz NOT NULL ...
  select count(*) into n from information_schema.columns
   where table_schema = 'relay_archive' and column_name = 'archived_at'
     and data_type = 'timestamp with time zone' and is_nullable = 'NO';
  if n <> 7 then
    raise exception 'archived_at timestamptz NOT NULL must be on all seven tables, saw % of 7.', n;
  end if;

  -- ... and every column of its live twin.
  select count(*) into n from (
    select table_name, column_name from information_schema.columns
     where table_schema = 'relay'
       and table_name in ('crews', 'lanes', 'leases', 'runs', 'run_items', 'tasks', 'actions')
    except
    select table_name, column_name from information_schema.columns
     where table_schema = 'relay_archive'
  ) missing;
  if n <> 0 then
    raise exception '% live columns missing from the archive.', n;
  end if;

  -- Token hashes are copied, not nulled: a re-close authenticates its
  -- caller lane against the archive for its 200 no-op.
  perform 1 from information_schema.columns
   where table_schema = 'relay_archive' and table_name = 'lanes'
     and column_name = 'token_hash' and is_nullable = 'NO';
  if not found then
    raise exception 'relay_archive.lanes.token_hash must be NOT NULL.';
  end if;

  -- No foreign key leaves the archive (archived rows never pin live ones).
  select count(*) into n from pg_catalog.pg_constraint con
    join pg_catalog.pg_class c on c.oid = con.conrelid
    join pg_catalog.pg_namespace s on s.oid = c.relnamespace
   where s.nspname = 'relay_archive' and con.contype = 'f';
  if n <> 0 then
    raise exception 'relay_archive must hold no foreign keys, saw %.', n;
  end if;

  -- RLS on, everywhere.
  select count(*) into n from pg_catalog.pg_class c
    join pg_catalog.pg_namespace s on s.oid = c.relnamespace
   where s.nspname = 'relay_archive' and c.relkind = 'r' and c.relrowsecurity;
  if n <> 7 then
    raise exception 'RLS must be on for all seven archive tables, saw % of 7.', n;
  end if;

  -- The history views resolve over the union (a select proves the shape).
  perform 1 from relay.run_list limit 0;
  perform 1 from relay.tasks_all limit 0;
  perform 1 from relay.actions_all limit 0;
  perform 1 from relay.lanes_all limit 0;
  perform 1 from relay.runs_all limit 0;
  perform 1 from relay.task_history limit 0;
  perform 1 from relay.task_queue limit 0;
end $$;

rollback;
