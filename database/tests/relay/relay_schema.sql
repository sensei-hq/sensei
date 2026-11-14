-- `relay` schema shape — tables, keys, RLS, grants, and the three views' nature.
--
-- WHAT BREAKS THIS TEST: a missing table; `user_id` missing or nullable on the
-- nine; a PK that is not `(user_id, id)` (or, on the two junction-shaped
-- tables, not `(user_id, <natural key>)`); a foreign key that leaves `relay`;
-- RLS off; `authenticated` gaining a write grant or `anon` any grant; a policy
-- that dereferences `auth.uid` (it exists on the dōjō, not on the desktop —
-- the same DDL deploys to both); a view that is not security-invoker.
--
-- Run from database/tests/ (`run.sh relay`, `run.sh --apply sensei`
-- fixtures/relay_apply.ddl relay/relay_schema.sql`, or psql $SENSEI_DATABASE_URL -f relay/relay_schema.sql):
-- the file applies its own DDL in-txn (see below) and rolls back.
-- with --apply the runner applies the DDL and the file skips its own apply.

begin;

-- The real DDL, in this transaction, rolled back with it.
\ir ../fixtures/relay_apply.ddl

do $$
declare
  n int;
  v_ok boolean;
begin
  -- Eleven tables: the six plus the queue's three (runs, run_items,
  -- lane_waiters) plus the finding-classes lookup plus the machine-local
  -- daemon user (local identity; never synced, hence outside the
  -- PK(user_id, id) sync-key pin below).
  select count(*) into n from pg_catalog.pg_class c
    join pg_catalog.pg_namespace s on s.oid = c.relnamespace
   where s.nspname = 'relay' and c.relkind = 'r';
  if n <> 11 then
    raise exception 'relay must hold 11 tables, saw %.', n;
  end if;

  -- `user_id` uuid NOT NULL on the nine (finding_classes is the shared lookup).
  select count(*) into n
    from information_schema.columns
   where table_schema = 'relay' and column_name = 'user_id'
     and data_type = 'uuid' and is_nullable = 'NO'
     and table_name in ('crews', 'lanes', 'leases', 'items', 'tasks', 'actions',
                        'runs', 'run_items', 'lane_waiters');
  if n <> 9 then
    raise exception 'user_id uuid NOT NULL must be on all nine tables, saw % of 9.', n;
  end if;

  -- ... and NOT on the lookup: one shared vocabulary, seeded identically both sides.
  select count(*) into n
    from information_schema.columns
   where table_schema = 'relay' and table_name = 'finding_classes' and column_name = 'user_id';
  if n <> 0 then
    raise exception 'finding_classes must not carry user_id.';
  end if;

  -- PK(user_id, id), in that order: the sync's idempotent upsert key.
  select count(*) into n from (
    select c.relname from pg_catalog.pg_constraint con
      join pg_catalog.pg_class c on c.oid = con.conrelid
      join pg_catalog.pg_namespace s on s.oid = c.relnamespace
     where s.nspname = 'relay' and con.contype = 'p'
       and (select array_agg(a.attname::text order by array_position(con.conkey, a.attnum))
              from pg_catalog.pg_attribute a
             where a.attrelid = c.oid and a.attnum = any (con.conkey))
           = array['user_id', 'id']
  ) pk where relname in ('crews', 'lanes', 'leases', 'items', 'tasks', 'actions', 'runs');
  if n <> 7 then
    raise exception 'PK(user_id, id) must be on all seven tables, saw % of 7.', n;
  end if;

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

  select count(*) into n from pg_catalog.pg_constraint con
    join pg_catalog.pg_class c on c.oid = con.conrelid
    join pg_catalog.pg_namespace s on s.oid = c.relnamespace
   where s.nspname = 'relay' and c.relname = 'finding_classes' and con.contype = 'p'
     and (select array_agg(a.attname::text order by array_position(con.conkey, a.attnum))
            from pg_catalog.pg_attribute a
           where a.attrelid = c.oid and a.attnum = any (con.conkey))
         = array['code'];
  if n <> 1 then
    raise exception 'finding_classes must be PK(code).';
  end if;

  -- No foreign key leaves `relay` (tickets, workspaces, principals resolve per side).
  select count(*) into n from pg_catalog.pg_constraint con
    join pg_catalog.pg_class c on c.oid = con.conrelid
    join pg_catalog.pg_namespace s on s.oid = c.relnamespace
    join pg_catalog.pg_class fc on fc.oid = con.confrelid
    join pg_catalog.pg_namespace fs on fs.oid = fc.relnamespace
   where s.nspname = 'relay' and con.contype = 'f' and fs.nspname <> 'relay';
  if n <> 0 then
    raise exception '% relay FK(s) point outside the schema.', n;
  end if;

  -- RLS on all eleven (the views inherit it through security-invoker).
  select count(*) into n from pg_catalog.pg_class c
    join pg_catalog.pg_namespace s on s.oid = c.relnamespace
   where s.nspname = 'relay' and c.relkind = 'r' and c.relrowsecurity;
  if n <> 11 then
    raise exception 'RLS must be on all 11 tables, saw % of 11.', n;
  end if;

  -- At least the owner-read policy per table (slice 2 adds dōjō write policies).
  select count(*) into n from pg_catalog.pg_policy p
    join pg_catalog.pg_class c on c.oid = p.polrelid
    join pg_catalog.pg_namespace s on s.oid = c.relnamespace
   where s.nspname = 'relay';
  if n < 11 then
    raise exception 'relay tables must carry at least 11 policies, saw %.', n;
  end if;

  -- Portability: no policy dereferences the dōjō-only `auth` schema.
  select count(*) into n from pg_catalog.pg_policy p
    join pg_catalog.pg_class c on c.oid = p.polrelid
    join pg_catalog.pg_namespace s on s.oid = c.relnamespace
   where s.nspname = 'relay' and pg_get_expr(p.polqual, p.polrelid) ilike '%auth.uid%';
  if n <> 0 then
    raise exception '% relay polic(ies) dereference auth.uid, which the desktop lacks.', n;
  end if;

  -- Grants: `authenticated` reads all ten, writes none; `anon` has nothing.
  select bool_and(has_table_privilege('authenticated', 'relay.' || t, 'SELECT'))
    into v_ok from (values ('crews'), ('lanes'), ('leases'), ('items'), ('tasks'), ('actions'),
                          ('finding_classes'), ('runs'), ('run_items'), ('lane_waiters')) v(t);
  if v_ok is distinct from true then
    raise exception 'authenticated must SELECT all 10 relay tables.';
  end if;

  select bool_and(not has_table_privilege('authenticated', 'relay.' || t, priv))
    into v_ok
    from (values ('crews'), ('lanes'), ('leases'), ('items'), ('tasks'), ('actions'),
                 ('finding_classes'), ('runs'), ('run_items'), ('lane_waiters')) v(t)
   cross join (values ('INSERT'), ('UPDATE'), ('DELETE')) p(priv);
  if v_ok is distinct from true then
    raise exception 'authenticated must hold no write grant on any relay table.';
  end if;

  select bool_and(not has_table_privilege('anon', 'relay.' || t, priv))
    into v_ok
    from (values ('crews'), ('lanes'), ('leases'), ('items'), ('tasks'), ('actions'),
                 ('finding_classes'), ('runs'), ('run_items'), ('lane_waiters')) v(t)
   cross join (values ('SELECT'), ('INSERT'), ('UPDATE'), ('DELETE')) p(priv);
  if v_ok is distinct from true then
    raise exception 'anon must hold no grant on any relay table.';
  end if;

  -- The daemon owner holds the full grant on all ten.
  select bool_and(has_table_privilege(current_user, 'relay.' || t, 'SELECT,INSERT,UPDATE,DELETE'))
    into v_ok from (values ('crews'), ('lanes'), ('leases'), ('items'), ('tasks'), ('actions'),
                          ('finding_classes'), ('runs'), ('run_items'), ('lane_waiters')) v(t);
  if v_ok is distinct from true then
    raise exception 'the daemon owner must hold full grants on all 10 relay tables.';
  end if;

  -- All three views exist and are security-invoker (the caller's RLS applies).
  select count(*) into n
    from pg_catalog.pg_class c
    join pg_catalog.pg_namespace s on s.oid = c.relnamespace
   cross join pg_catalog.pg_options_to_table(c.reloptions) o
   where s.nspname = 'relay' and c.relname in ('now', 'task_history', 'task_queue')
     and o.option_name = 'security_invoker' and o.option_value = 'true';
  if n <> 3 then
    raise exception 'relay.now, relay.task_history and relay.task_queue must all be security-invoker.';
  end if;
end $$;

rollback;
