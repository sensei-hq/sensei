-- Convert the ladder's `technology` rung into stack packs (#318 R6, D29/D33,
-- user 2026-10-21/22): a stack is a tech stack's rules, packed and applied at a
-- level — not a level. Each technology namespace becomes the pack
-- `stack:<slug>` (sensei.stack_pack), adopted at each repository the namespace
-- was bound to (sensei.adopt_stack_pack) — exactly the reach it had.
--
-- WHY AFTER, NOT BEFORE. The conversion calls the functions the scan calls when
-- it detects a stack (stack_pack, repository_namespace, adopt_stack_pack), so
-- the upgrade and every later detection make the same pack and the same
-- repository namespace — one definition, not two. Those functions exist only
-- once `apply` has written them. Nothing the DDL changes depends on the rung
-- being gone, and the seed import that follows only upserts rungs.
--
-- EVERYTHING ON A TECHNOLOGY NAMESPACE'S ID is moved, never dropped with it:
--   - its governing memories (active, reinforced, battle-tested) become the
--     pack's rules, each citing its memory in `evidence`; then every memory on
--     it is archived and tagged `stack:<slug>`. Archived, because a memory whose
--     namespace is deleted is set to NULL — and a memory with neither namespace
--     nor workspace is a GLOBAL rule: left active, it would govern everything.
--   - a pack adopted at the stack is adopted at each of its repositories; two
--     such adoptions reaching one repository merge at the stronger override (an
--     override only raises a rule's tier, so the stronger one is what reached).
--   - a stance on the stack moves to each of its repositories. Two stances of
--     one user on two stacks of one repository would collide: refused.
--   - anything else that references the namespace (a pack it owns, a federation
--     source scoped to it, a dōjō shared rule or seat) has no repository to move
--     to: refused, by name. Found generically, from the catalogue's foreign keys,
--     so a reference added later cannot be dropped silently either.
-- A REFUSAL raises: the deploy stops, nothing is deleted, the user decides.
--
-- On the dōjō's database nothing binds a namespace to a repository
-- (`repository_namespaces` is the daemon's), so a technology namespace there is
-- refused outright; none exists in production (measured 2026-10-22).
--
-- A no-op once the rung is gone, and on a fresh database (the seed has no
-- technology rung). Retires `team` and `client` alongside:
-- apply/before/sensei_ladder_rungs_retired.sql.
do $$
declare
    t         record;
    fk        record;
    v_pack    uuid;
    v_refused text;
    v_found   bigint;
begin
    -- Nested, not `and`-ed: an IF condition is planned whole, so a read of
    -- sensei.scopes in it fails on a fresh database before the guard is tested.
    if to_regclass('sensei.scopes') is null then
        return;
    end if;
    if not exists (select 1 from sensei.scopes where key = 'technology') then
        return;
    end if;

    -- ── the dōjō's plane: nothing to adopt a stack at ────────────────────────
    if to_regclass('sensei.repository_namespaces') is null then
        select string_agg('technology:' || slug, ', ' order by slug) into v_refused
          from sensei.namespaces where scope_key = 'technology';
        if v_refused is not null then
            raise exception 'the technology rung is retired (#318 R6, D29), and this database binds no repository to adopt a stack pack at; namespaces still on it: %', v_refused
                using hint = 'Move each one''s rules to a pack adopted at the dōjō or a workspace, delete the namespace, and deploy again.';
        end if;
        delete from sensei.scopes where key = 'technology';
        return;
    end if;

    -- ── what cannot move: refused ────────────────────────────────────────────
    for fk in select c.conrelid::regclass as rel, a.attname as col
                from pg_constraint c
                join pg_attribute a on a.attrelid = c.conrelid and a.attnum = c.conkey[1]
               where c.contype = 'f'
                 and c.confrelid = 'sensei.namespaces'::regclass
                 and c.conrelid not in ('sensei.repository_namespaces'::regclass,
                                        'sensei.rule_pack_adoptions'::regclass,
                                        'sensei.stances'::regclass,
                                        'sensei.memories'::regclass) loop
        execute format('select string_agg(distinct n.slug, '', '' order by n.slug), count(*)
                          from %s x join sensei.namespaces n on n.id = x.%I
                         where n.scope_key = ''technology''', fk.rel, fk.col)
           into v_refused, v_found;
        if v_found > 0 then
            raise exception 'the technology rung is retired (#318 R6, D29), but % row(s) of % reference a technology namespace and have no repository to move to: %',
                    v_found, fk.rel, v_refused
                using hint = 'Re-scope or delete those rows, then deploy again.';
        end if;
    end loop;

    select string_agg(format('%s on repository %s', user_key, repository_id), ', ') into v_refused
      from (select st.user_key, rn.repository_id
              from sensei.stances st
              join sensei.namespaces n on n.id = st.namespace_id and n.scope_key = 'technology'
              join sensei.repository_namespaces rn on rn.namespace_id = n.id
             group by st.user_key, rn.repository_id
            having count(*) > 1) collide;
    if v_refused is not null then
        raise exception 'the technology rung is retired (#318 R6, D29), and stances on two stacks of one repository would collide there: %', v_refused
            using hint = 'Keep one stance per user on those stacks, then deploy again.';
    end if;

    -- ── each stack becomes a pack, adopted where it was bound ────────────────
    for t in select id, slug, name from sensei.namespaces where scope_key = 'technology' order by slug loop
        v_pack := sensei.stack_pack(t.slug, t.name);

        insert into sensei.rule_pack_rules (pack_id, ordinal, statement, body, rationale, enforcement, evidence)
        select v_pack,
               coalesce((select max(r.ordinal) from sensei.rule_pack_rules r where r.pack_id = v_pack), -1)
                 + row_number() over (order by m.created_at, m.id),
               m.title, m.content, m.impact, m.enforcement,
               jsonb_build_object('memory_id', m.id, 'converted_from', 'technology:' || t.slug)
          from sensei.memories m
         where m.namespace_id = t.id
           and m.status in ('active', 'reinforced', 'battle_tested');

        update sensei.memories
           set status = case when status in ('archived', 'rejected') then status else 'archived' end,
               tags = case when ('stack:' || t.slug) = any (tags) then tags else tags || ('stack:' || t.slug) end,
               modified_at = now()
         where namespace_id = t.id;

        perform sensei.adopt_stack_pack(rn.repository_id, t.slug, t.name)
           from sensei.repository_namespaces rn
          where rn.namespace_id = t.id;

        insert into sensei.rule_pack_adoptions (pack_id, namespace_id, pinned_version, enforcement,
                                                adopted_by, adopted_at, updated_at)
        select a.pack_id, sensei.repository_namespace(rn.repository_id), a.pinned_version, a.enforcement,
               a.adopted_by, a.adopted_at, now()
          from sensei.rule_pack_adoptions a
          join sensei.repository_namespaces rn on rn.namespace_id = a.namespace_id
         where a.namespace_id = t.id
        on conflict (pack_id, namespace_id) do update
           set enforcement = greatest(sensei.rule_pack_adoptions.enforcement, excluded.enforcement),
               updated_at = now();

        insert into sensei.stances (user_key, namespace_id, autonomy, sharing, review, updated_at)
        select st.user_key, sensei.repository_namespace(rn.repository_id),
               st.autonomy, st.sharing, st.review, st.updated_at
          from sensei.stances st
          join sensei.repository_namespaces rn on rn.namespace_id = st.namespace_id
         where st.namespace_id = t.id;

        -- Its bindings, adoptions and stances go with it (cascade); every one
        -- was carried to the repositories above.
        delete from sensei.namespaces where id = t.id;
    end loop;

    delete from sensei.scopes where key = 'technology';
end $$;
