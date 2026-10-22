-- The dōjō's ladder is dōjō → workspace → repository too (#318 R6, D25/D29/D32/D33).
--
-- `sensei.scopes` and `sensei.namespaces` are deployed to the dōjō as well as
-- to the daemon, and the two exchange the rung keys (a rule's `scope_key`, a
-- folder's `scope_key:slug` pairs) — so `team`, `client` and `technology` leave
-- both in the same release, through the same hooks.
--
-- On this plane there is nothing to convert a stack INTO: no repository is
-- bound to a namespace here (`repository_namespaces` is the daemon's). So a
-- namespace on any retired rung — `technology` included — is REFUSED, never
-- deleted (a shared rule or a billing seat may point at its id); an empty rung
-- is dropped.
--
-- WHAT BREAKS THIS TEST: the seed still carrying a retired rung; a hook that
-- skips the dōjō's database, deletes an occupied rung's namespace, or is not a
-- no-op once it has run.

begin;
set local client_min_messages = warning;

do $$
begin
    if (select array_agg(key order by level) from sensei.scopes)
       is distinct from array['general', 'user', 'organization', 'workspace', 'repository'] then
        raise exception 'the dōjō''s ladder is not general · user · organization · workspace · repository: %',
            (select array_agg(key order by level) from sensei.scopes);
    end if;
end $$;

-- ── the rungs as an older dōjō holds them ────────────────────────────────────
insert into sensei.scopes (key, name, level, shareable, description, modified_at) values
  ('client',     'Client',     30, true,  'Standards for one client.',       '2026-06-05'),
  ('technology', 'Technology', 40, false, 'Language/framework conventions.', '2026-06-05'),
  ('team',       'Team',       50, true,  'Decisions owned by a team.',      '2026-06-05');

-- ── an occupied team or client rung is refused ───────────────────────────────
insert into sensei.namespaces (id, scope_key, name, slug)
values ('e0000000-0000-4000-8000-000000031851', 'client', 'ztest-r6-acme', 'ztest-r6-acme');

savepoint occupied;
-- The ERROR psql prints next is the expected refusal.
\set ON_ERROR_STOP 0
\ir ../../apply/before/sensei_ladder_rungs_retired.sql
\set ON_ERROR_STOP 1
rollback to savepoint occupied;
create temp table occupied as select :'LAST_ERROR_MESSAGE'::text as message;

do $$
begin
    if (select message from occupied) not like '%client:ztest-r6-acme%' then
        raise exception 'the hook did not refuse a namespace on the client rung: %', (select message from occupied);
    end if;
end $$;
delete from sensei.namespaces where id = 'e0000000-0000-4000-8000-000000031851';

-- ── a stack namespace is refused: there is no repository to adopt it at ──────
insert into sensei.namespaces (id, scope_key, name, slug)
values ('e0000000-0000-4000-8000-000000031852', 'technology', 'ztest-r6-react', 'ztest-r6-react');

savepoint stacked;
-- The ERROR psql prints next is the expected refusal.
\set ON_ERROR_STOP 0
\ir ../../apply/after/sensei_stack_packs_converted.sql
\set ON_ERROR_STOP 1
rollback to savepoint stacked;
create temp table stacked as select :'LAST_ERROR_MESSAGE'::text as message;

do $$
begin
    if (select message from stacked) not like '%technology:ztest-r6-react%' then
        raise exception 'the conversion did not refuse a technology namespace on the dōjō: %',
            (select message from stacked);
    end if;
    if not exists (select 1 from sensei.namespaces where id = 'e0000000-0000-4000-8000-000000031852') then
        raise exception 'the refused namespace was deleted';
    end if;
end $$;
delete from sensei.namespaces where id = 'e0000000-0000-4000-8000-000000031852';

-- ── empty rungs are dropped, and a second run changes nothing ────────────────
\ir ../../apply/before/sensei_ladder_rungs_retired.sql
\ir ../../apply/after/sensei_stack_packs_converted.sql
\ir ../../apply/before/sensei_ladder_rungs_retired.sql
\ir ../../apply/after/sensei_stack_packs_converted.sql

do $$
begin
    if (select array_agg(key order by level) from sensei.scopes)
       is distinct from array['general', 'user', 'organization', 'workspace', 'repository'] then
        raise exception 'the hooks did not drop the empty retired rungs: %',
            (select array_agg(key order by level) from sensei.scopes);
    end if;
end $$;

rollback;
