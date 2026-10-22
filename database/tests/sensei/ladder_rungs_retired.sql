-- The ladder is dōjō → workspace → repository (#318 R6, D25/D29/D32/D33).
--
-- Three rungs leave `sensei.scopes`:
--   - `team` and `client` (D25, D32) held no namespace anywhere. They are
--     dropped by apply/before/sensei_ladder_rungs_retired.sql, which REFUSES —
--     raises, never deletes — while a namespace still sits on either.
--   - `technology` (D29, D33): a stack is a rule pack applied at a level, not a
--     level. apply/after/sensei_stack_packs_converted.sql turns each technology
--     namespace into a pack (`stack:<slug>`), its rules into the pack's rules,
--     and adopts it at each repository the stack was bound to — exactly the
--     reach it had. What else hung off the namespace's id is re-homed onto those
--     repositories (a pack adopted at the stack, a stance) or refused (a pack it
--     owns, a federation source on it); its memories become the pack's rules and
--     are archived with a `stack:<slug>` tag, so none outlives its namespace as
--     an unscoped — that is, global — rule.
--
-- WHAT BREAKS THIS TEST: a retired rung left in the seed or the database; a hook
-- that deletes a namespace instead of refusing; a stack pack adopted anywhere
-- but the repositories its stack was bound to; a rule that reached a repository
-- before and does not after (or the reverse), or reaches it weaker; a stance or
-- re-adopted pack lost with its namespace; a hook that is not a no-op once run.

begin;
set local client_min_messages = warning;

-- ── the converged ladder ─────────────────────────────────────────────────────
do $$
begin
    if (select array_agg(key order by level) from sensei.scopes)
       is distinct from array['general', 'user', 'organization', 'workspace', 'repository'] then
        raise exception 'the ladder is not general · user · organization · workspace · repository: %',
            (select array_agg(key order by level) from sensei.scopes);
    end if;
    if exists (select 1 from sensei.namespaces where scope_key in ('team', 'client', 'technology')) then
        raise exception 'a namespace still sits on a retired rung';
    end if;
end $$;

-- ── the rungs as an older database holds them ────────────────────────────────
insert into sensei.scopes (key, name, level, shareable, description, modified_at) values
  ('client',     'Client',     30, true,  'Standards for one client.',       '2026-06-05'),
  ('technology', 'Technology', 40, false, 'Language/framework conventions.', '2026-06-05'),
  ('team',       'Team',       50, true,  'Decisions owned by a team.',      '2026-06-05');

insert into sensei.repositories (id, repo_key, name) values
  ('e0000000-0000-4000-8000-000000031801', 'ztest.host/r6/one', 'ztest-r6-one'),
  ('e0000000-0000-4000-8000-000000031802', null,                'ztest-r6-two');

-- rust on both repositories, react on one, json on none.
insert into sensei.namespaces (id, scope_key, name, slug) values
  ('e0000000-0000-4000-8000-000000031811', 'technology', 'ztest-r6-rust',  'ztest-r6-rust'),
  ('e0000000-0000-4000-8000-000000031812', 'technology', 'ztest-r6-react', 'ztest-r6-react'),
  ('e0000000-0000-4000-8000-000000031813', 'technology', 'ztest-r6-json',  'ztest-r6-json');
insert into sensei.repository_namespaces (repository_id, namespace_id) values
  ('e0000000-0000-4000-8000-000000031801', 'e0000000-0000-4000-8000-000000031811'),
  ('e0000000-0000-4000-8000-000000031802', 'e0000000-0000-4000-8000-000000031811'),
  ('e0000000-0000-4000-8000-000000031801', 'e0000000-0000-4000-8000-000000031812');

-- A rule on rust (governs), a proposal on rust (does not, yet).
insert into sensei.memories (id, namespace_id, enforcement, status, type, title, content, impact) values
  ('e0000000-0000-4000-8000-000000031821', 'e0000000-0000-4000-8000-000000031811', 'required', 'active',
   'convention', 'ztest-r6 no unwrap', 'ztest-r6: propagate errors; never unwrap in library code', 'panics'),
  ('e0000000-0000-4000-8000-000000031822', 'e0000000-0000-4000-8000-000000031811', 'recommended', 'proposed',
   'convention', 'ztest-r6 proposal', 'ztest-r6: a proposal on the stack', null);

-- A library pack adopted at both stacks: at rust raised to required, at react as is.
insert into sensei.rule_packs (id, slug, name, area, attribution, published_by) values
  ('e0000000-0000-4000-8000-000000031831', 'ztest-r6-lib', 'ztest-r6 lib', 'security', 'ztest', 'ztest');
insert into sensei.rule_pack_rules (pack_id, ordinal, statement, body, enforcement) values
  ('e0000000-0000-4000-8000-000000031831', 0, 'ztest-r6 lib rule', 'ztest-r6: the lib rule', 'recommended');
insert into sensei.rule_pack_adoptions (pack_id, namespace_id, pinned_version, enforcement, adopted_by) values
  ('e0000000-0000-4000-8000-000000031831', 'e0000000-0000-4000-8000-000000031811', 1, 'required', 'ztest'),
  ('e0000000-0000-4000-8000-000000031831', 'e0000000-0000-4000-8000-000000031812', 1, null,       'ztest');

-- A stance at react.
insert into sensei.stances (user_key, namespace_id, autonomy) values
  ('ztest-r6', 'e0000000-0000-4000-8000-000000031812', 'run_freely');

-- ── rule reach: the conservation property ────────────────────────────────────
-- Every rule that reaches a repository through the namespaces bound to it, at
-- the strongest tier it reaches with. A memory governs while active,
-- reinforced or battle-tested; a pack rule at its tier raised by the adoption.
create temp view repository_rule_reach as
select repository_id, rule, max(enforcement) as enforcement
  from (select rn.repository_id, m.title || ' | ' || m.content as rule, m.enforcement
          from sensei.repository_namespaces rn
          join sensei.memories m on m.namespace_id = rn.namespace_id
         where m.status in ('active', 'reinforced', 'battle_tested')
        union all
        select rn.repository_id, r.statement || ' | ' || r.body,
               greatest(r.enforcement, coalesce(a.enforcement, r.enforcement))
          from sensei.repository_namespaces rn
          join sensei.rule_pack_adoptions a on a.namespace_id = rn.namespace_id
          join sensei.rule_pack_rules r on r.pack_id = a.pack_id) reach
 where repository_id in ('e0000000-0000-4000-8000-000000031801', 'e0000000-0000-4000-8000-000000031802')
 group by repository_id, rule;

create temp table reach_before as select * from repository_rule_reach;

\ir ../../apply/after/sensei_stack_packs_converted.sql

do $$
declare
    one constant uuid := 'e0000000-0000-4000-8000-000000031801';
    two constant uuid := 'e0000000-0000-4000-8000-000000031802';
    one_ns uuid;
    two_ns uuid;
    diff text;
begin
    if exists (select 1 from sensei.scopes where key = 'technology')
       or exists (select 1 from sensei.namespaces where scope_key = 'technology') then
        raise exception 'the technology rung or a namespace on it is still there';
    end if;

    -- Each repository has its own namespace on the repository rung, bound to it:
    -- slugged by its remote, or by its id when it has none.
    select n.id into one_ns from sensei.namespaces n
      join sensei.repository_namespaces rn on rn.namespace_id = n.id and rn.repository_id = one
     where n.scope_key = 'repository' and n.slug = 'ztest.host/r6/one' and n.name = 'ztest-r6-one';
    select n.id into two_ns from sensei.namespaces n
      join sensei.repository_namespaces rn on rn.namespace_id = n.id and rn.repository_id = two
     where n.scope_key = 'repository' and n.slug = two::text;
    if one_ns is null or two_ns is null then
        raise exception 'a repository has no repository-rung namespace bound to it';
    end if;

    -- One pack per stack, the json one included; adopted exactly where the stack was bound.
    if (select count(*) from sensei.rule_packs
         where slug in ('stack:ztest-r6-rust', 'stack:ztest-r6-react', 'stack:ztest-r6-json')
           and area = 'tech_stack' and owner_namespace_id is null) <> 3 then
        raise exception 'a technology namespace did not become a tech_stack pack';
    end if;
    if (select array_agg(n.id order by n.slug) from sensei.rule_pack_adoptions a
          join sensei.rule_packs p on p.id = a.pack_id
          join sensei.namespaces n on n.id = a.namespace_id
         where p.slug = 'stack:ztest-r6-rust') is distinct from array[two_ns, one_ns]
       or (select array_agg(a.namespace_id) from sensei.rule_pack_adoptions a
             join sensei.rule_packs p on p.id = a.pack_id
            where p.slug = 'stack:ztest-r6-react') is distinct from array[one_ns]
       or exists (select 1 from sensei.rule_pack_adoptions a
                    join sensei.rule_packs p on p.id = a.pack_id
                   where p.slug = 'stack:ztest-r6-json') then
        raise exception 'a stack pack is not adopted at exactly the repositories its stack was bound to';
    end if;

    -- The stack's rule became the pack's rule, citing its memory.
    if not exists (select 1 from sensei.rule_pack_rules r join sensei.rule_packs p on p.id = r.pack_id
                    where p.slug = 'stack:ztest-r6-rust'
                      and r.statement = 'ztest-r6 no unwrap'
                      and r.body = 'ztest-r6: propagate errors; never unwrap in library code'
                      and r.rationale = 'panics'
                      and r.enforcement = 'required'
                      and r.evidence ->> 'memory_id' = 'e0000000-0000-4000-8000-000000031821')
       or (select count(*) from sensei.rule_pack_rules r join sensei.rule_packs p on p.id = r.pack_id
            where p.slug = 'stack:ztest-r6-rust') <> 1 then
        raise exception 'the stack''s governing rule did not become exactly one rule of its pack';
    end if;
    -- Its memories are archived and tagged, never left unscoped (= global).
    if exists (select 1 from sensei.memories
                where id in ('e0000000-0000-4000-8000-000000031821', 'e0000000-0000-4000-8000-000000031822')
                  and not (status = 'archived' and 'stack:ztest-r6-rust' = any (tags))) then
        raise exception 'a memory on a technology namespace was not archived with its stack tag';
    end if;

    -- The pack adopted at a stack is adopted at the stack's repositories; the
    -- two adoptions reaching repository one merged at the stronger tier.
    if (select count(*) from sensei.rule_pack_adoptions
         where pack_id = 'e0000000-0000-4000-8000-000000031831'
           and namespace_id in (one_ns, two_ns) and enforcement = 'required') <> 2 then
        raise exception 'a pack adopted at a stack was not re-adopted at its repositories at its tier';
    end if;

    -- The stance at react moved to the repository react was bound to.
    if not exists (select 1 from sensei.stances
                    where user_key = 'ztest-r6' and namespace_id = one_ns and autonomy = 'run_freely') then
        raise exception 'the stance on a technology namespace was not re-homed to its repository';
    end if;

    -- Conservation: the same rules reach each repository, at the same tier.
    select string_agg(format('%s %s %s', side, repository_id, rule), E'\n  ') into diff
      from (select 'lost' as side, * from (select * from reach_before
                                           except select * from repository_rule_reach) l
            union all
            select 'gained', * from (select * from repository_rule_reach
                                     except select * from reach_before) g) d;
    if diff is not null then
        raise exception E'rule reach changed:\n  %', diff;
    end if;
    if (select count(*) from reach_before) <> 4 then
        raise exception 'the fixture reach is not the four (repository, rule) pairs it was built with';
    end if;
end $$;

-- ── a second run is a no-op ──────────────────────────────────────────────────
create temp table after_first as
select (select count(*) from sensei.rule_packs) as packs
     , (select count(*) from sensei.rule_pack_rules) as pack_rules
     , (select count(*) from sensei.rule_pack_adoptions) as adoptions
     , (select count(*) from sensei.namespaces) as namespaces;

\ir ../../apply/after/sensei_stack_packs_converted.sql

do $$
begin
    if (select row(packs, pack_rules, adoptions, namespaces) from after_first)
       is distinct from row((select count(*) from sensei.rule_packs), (select count(*) from sensei.rule_pack_rules),
              (select count(*) from sensei.rule_pack_adoptions), (select count(*) from sensei.namespaces)) then
        raise exception 'a second run of the conversion changed something';
    end if;
end $$;

-- ── detection adopts, idempotently ───────────────────────────────────────────
-- A stack newly detected on a repository adopts its pack there (what the scan
-- calls), the pack and the repository's namespace made on first use.
do $$
begin
    perform sensei.adopt_stack_pack('e0000000-0000-4000-8000-000000031802', 'ztest-r6-go', 'Go');
    perform sensei.adopt_stack_pack('e0000000-0000-4000-8000-000000031802', 'ztest-r6-go', 'Go');
end $$;

do $$
begin
    if (select count(*) from sensei.rule_pack_adoptions a
          join sensei.rule_packs p on p.id = a.pack_id
          join sensei.repository_namespaces rn on rn.namespace_id = a.namespace_id
         where p.slug = 'stack:ztest-r6-go' and p.name = 'Go'
           and rn.repository_id = 'e0000000-0000-4000-8000-000000031802') <> 1
       or (select count(*) from sensei.namespaces
            where scope_key = 'repository' and slug = 'e0000000-0000-4000-8000-000000031802') <> 1 then
        raise exception 'adopting a detected stack twice did not leave exactly one adoption and one namespace';
    end if;
end $$;

-- ── what the conversion refuses ──────────────────────────────────────────────
-- A federation source scoped to a stack has no repository to move to; nulling
-- it would widen it to every shareable namespace. The hook raises instead.
insert into sensei.scopes (key, name, level, shareable, description, modified_at)
values ('technology', 'Technology', 40, false, 'Language/framework conventions.', '2026-06-05');
insert into sensei.namespaces (id, scope_key, name, slug)
values ('e0000000-0000-4000-8000-000000031814', 'technology', 'ztest-r6-go', 'ztest-r6-go');
insert into sensei.knowledge_sources (kind, name, url, namespace_id)
values ('hive_mind', 'ztest-r6', 'https://ztest', 'e0000000-0000-4000-8000-000000031814');

savepoint refused;
-- The ERROR psql prints next is the expected refusal.
\set ON_ERROR_STOP 0
\ir ../../apply/after/sensei_stack_packs_converted.sql
\set ON_ERROR_STOP 1
rollback to savepoint refused;
create temp table refusal as select :'LAST_ERROR_MESSAGE'::text as message;

do $$
begin
    if (select message from refusal) not like '%sensei.knowledge_sources%ztest-r6-go%' then
        raise exception 'the conversion did not refuse a federation source on a stack: %',
            (select message from refusal);
    end if;
    if not exists (select 1 from sensei.namespaces where id = 'e0000000-0000-4000-8000-000000031814') then
        raise exception 'the refused namespace was deleted';
    end if;
end $$;

-- ── team and client: refused while occupied, dropped when empty ──────────────
insert into sensei.namespaces (id, scope_key, name, slug)
values ('e0000000-0000-4000-8000-000000031815', 'team', 'ztest-r6-mobile', 'ztest-r6-mobile');

savepoint occupied;
-- The ERROR psql prints next is the expected refusal.
\set ON_ERROR_STOP 0
\ir ../../apply/before/sensei_ladder_rungs_retired.sql
\set ON_ERROR_STOP 1
rollback to savepoint occupied;
create temp table occupied as select :'LAST_ERROR_MESSAGE'::text as message;

do $$
begin
    if (select message from occupied) not like '%team:ztest-r6-mobile%' then
        raise exception 'the hook did not refuse a namespace on the team rung: %', (select message from occupied);
    end if;
    if not exists (select 1 from sensei.namespaces where id = 'e0000000-0000-4000-8000-000000031815')
       or not exists (select 1 from sensei.scopes where key = 'team') then
        raise exception 'the refusal deleted the team namespace or its rung';
    end if;
end $$;

delete from sensei.namespaces where id = 'e0000000-0000-4000-8000-000000031815';
\ir ../../apply/before/sensei_ladder_rungs_retired.sql
\ir ../../apply/before/sensei_ladder_rungs_retired.sql

do $$
begin
    if exists (select 1 from sensei.scopes where key in ('team', 'client')) then
        raise exception 'the empty team and client rungs were not dropped';
    end if;
end $$;

rollback;
