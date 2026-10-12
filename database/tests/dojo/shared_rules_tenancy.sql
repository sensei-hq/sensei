-- A shared rule is OWNED BY ONE TENANT (decision 2026-10-12, #236;
-- docs/spec/dojo/access-matrix.md "Knowledge").
--
-- `dojo.shared_rules` had no tenant: any member of any tenant read every
-- tenant's rules, and one tenant's publish could overwrite another's. This file
-- holds the three parts of the fix to the database:
--
--   1. the BACKFILL — a rule's owner is the tenant of its EARLIEST `publish`
--      audit event; a rule with no publish event has no provable owner and stays
--      NULL (hidden until a maintainer re-publishes it) — never a guessed tenant;
--   2. RLS — members read their own tenant's rules, and only a maintainer or
--      above writes them;
--   3. uniqueness per tenant — two tenants may publish the same content.
--
-- WHAT BREAKS THIS TEST: owner = LATEST publish instead of earliest; any
-- fallback tenant for an unprovable rule; a read policy without is_member; a
-- write policy at contributor; the old global (namespace_id, content_hash) key.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000011', 'a1000000-0000-4000-8000-000000000011', 'alice'),
  ('a0000000-0000-4000-8000-000000000012', 'a1000000-0000-4000-8000-000000000012', 'dave'),
  ('a0000000-0000-4000-8000-000000000013', 'a1000000-0000-4000-8000-000000000013', 'carol');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000001a', 'organization/ztest-rules-t', 'organization',
   'ztest-rules-t', 'rules fixture T', 'dojo.sensei-hq.org/organization/ztest-rules-t'),
  ('b0000000-0000-4000-8000-00000000001b', 'organization/ztest-rules-u', 'organization',
   'ztest-rules-u', 'rules fixture U', 'dojo.sensei-hq.org/organization/ztest-rules-u');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000001a', 'a0000000-0000-4000-8000-000000000011', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000001a', 'a0000000-0000-4000-8000-000000000012', 'maintainer',  'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000001b', 'a0000000-0000-4000-8000-000000000013', 'contributor', 'employer', 'github_oauth');

insert into sensei.namespaces (id, scope_key, name, slug) values
  ('e0000000-0000-4000-8000-0000000000a1', 'organization', 'ztest rules ns', 'ztest-rules-ns');

-- The backfill reports totals over the whole table (what a deploy wants), so
-- measure against what is already there — a database with other rules in it
-- (the relay round trip publishes some) must not fail this.
create temp table backfill_baseline on commit drop as
    select b.owned, b.unprovable from dojo.backfill_shared_rules_tenant() b;

-- Three rules as they stand BEFORE the backfill: no tenant yet.
insert into dojo.shared_rules (id, namespace_id, content_hash, rule_type, title, content,
                               enforcement, published_by, published_at) values
  ('f0000000-0000-4000-8000-0000000000a1', 'e0000000-0000-4000-8000-0000000000a1', 'h1', 'rule',
   'first published by T, later re-published by U', 'x', 'advisory', 'p', now()),
  ('f0000000-0000-4000-8000-0000000000a2', 'e0000000-0000-4000-8000-0000000000a1', 'h2', 'rule',
   'published by U only', 'x', 'advisory', 'p', now()),
  ('f0000000-0000-4000-8000-0000000000a3', 'e0000000-0000-4000-8000-0000000000a1', 'h3', 'rule',
   'no publish audit at all', 'x', 'advisory', 'p', now());

insert into dojo.audit_events (tenant_id, ts, action, target) values
  ('b0000000-0000-4000-8000-00000000001b', now() - interval '1 day',  'publish', 'f0000000-0000-4000-8000-0000000000a1'),
  ('b0000000-0000-4000-8000-00000000001a', now() - interval '3 days', 'publish', 'f0000000-0000-4000-8000-0000000000a1'),
  ('b0000000-0000-4000-8000-00000000001b', now() - interval '2 days', 'publish', 'f0000000-0000-4000-8000-0000000000a2'),
  -- A retract is not a publish: it must not make U the owner of h3.
  ('b0000000-0000-4000-8000-00000000001b', now() - interval '2 days', 'retract', 'f0000000-0000-4000-8000-0000000000a3');

-- ── 1. the backfill ──────────────────────────────────────────────────────────
do $$
declare
    owned int;
    unprovable int;
begin
    select b.owned - base.owned, b.unprovable - base.unprovable into owned, unprovable
      from dojo.backfill_shared_rules_tenant() b, backfill_baseline base;
    if (owned, unprovable) is distinct from (2, 1) then
        raise exception 'backfill: expected 2 more owned / 1 more unprovable, got % / %', owned, unprovable;
    end if;
    if (select tenant_id from dojo.shared_rules where id = 'f0000000-0000-4000-8000-0000000000a1')
       is distinct from 'b0000000-0000-4000-8000-00000000001a' then
        raise exception 'backfill: the EARLIEST publish (T) owns h1, not the latest (U)';
    end if;
    if (select tenant_id from dojo.shared_rules where id = 'f0000000-0000-4000-8000-0000000000a3') is not null then
        raise exception 'backfill: a rule with no publish event has no provable owner — it must stay NULL';
    end if;
end $$;

-- ── 3. uniqueness is per tenant ──────────────────────────────────────────────
insert into dojo.shared_rules (tenant_id, namespace_id, content_hash, rule_type, title, content,
                               enforcement, published_by, published_at)
values ('b0000000-0000-4000-8000-00000000001b', 'e0000000-0000-4000-8000-0000000000a1', 'h1', 'rule',
        'U publishes the same content T owns', 'x', 'advisory', 'p', now());

-- ── 2. RLS, as each person ───────────────────────────────────────────────────
set local role authenticated;

do $$
declare
    titles text[];
    changed int;
begin
    -- alice (contributor, T): T's rules only — not U's, not the unowned one.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000011"}', true);
    select array_agg(content_hash order by content_hash) into titles from dojo.shared_rules
     where namespace_id = 'e0000000-0000-4000-8000-0000000000a1';
    if titles is distinct from array['h1'] then
        raise exception 'RLS read: alice should see only T''s h1, saw %', titles;
    end if;

    -- carol (contributor, U): U's rules only.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000013"}', true);
    select array_agg(content_hash order by content_hash) into titles from dojo.shared_rules
     where namespace_id = 'e0000000-0000-4000-8000-0000000000a1';
    if titles is distinct from array['h1', 'h2'] then
        raise exception 'RLS read: carol should see U''s h1 and h2, saw %', titles;
    end if;

    -- alice cannot write: a contributor is below the maintainer floor. (As
    -- ALICE — the claims still name carol from the read above, and a write by a
    -- member of another tenant would be refused by any policy at all.)
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000011"}', true);
    begin
        insert into dojo.shared_rules (tenant_id, namespace_id, content_hash, rule_type, title, content,
                                       enforcement, published_by, published_at)
        values ('b0000000-0000-4000-8000-00000000001a', 'e0000000-0000-4000-8000-0000000000a1', 'h9',
                'rule', 'contributor write', 'x', 'advisory', 'p', now());
        raise exception 'RLS write: a contributor must not publish';
    exception when insufficient_privilege then
        -- ONLY the policy may refuse it. A missing grant raises the same code,
        -- and would let this pass with no policy at all.
        if sqlerrm not like '%row-level security%' then
            raise exception 'RLS write: refused for the wrong reason: %', sqlerrm;
        end if;
    end;

    -- dave (maintainer, T) writes T's rules, and cannot touch U's.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000012"}', true);
    insert into dojo.shared_rules (tenant_id, namespace_id, content_hash, rule_type, title, content,
                                   enforcement, published_by, published_at)
    values ('b0000000-0000-4000-8000-00000000001a', 'e0000000-0000-4000-8000-0000000000a1', 'h8',
            'rule', 'maintainer publish', 'x', 'advisory', 'p', now());
    update dojo.shared_rules set status = 'tombstoned'
     where id = 'f0000000-0000-4000-8000-0000000000a1';
    get diagnostics changed = row_count;
    if changed <> 1 then
        raise exception 'RLS write: a maintainer of T should retract T''s rule, changed %', changed;
    end if;
    update dojo.shared_rules set status = 'tombstoned'
     where id = 'f0000000-0000-4000-8000-0000000000a2';
    get diagnostics changed = row_count;
    if changed <> 0 then
        raise exception 'RLS write: a maintainer of T must not retract U''s rule, changed %', changed;
    end if;
end $$;

rollback;
