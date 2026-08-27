-- The exact INSERT `createDojo` issues, run against a real database.
--
-- This is the test that was missing. `createDojo` (POST /v1/you/dojos — the
-- acceptance criterion of issue #117) sent `origin: 'org'`, `org: slug` and
-- `key: 'org/{slug}'` after all three were retired by the phase-1 schema change.
-- The insert failed with `column "org" of relation "tenants" does not exist`,
-- while `admin-data.spec.ts` asserted that very payload against a mocked
-- supabase-js client and passed. Spec §VIII.4.
--
-- WHAT BREAKS THIS TEST: reverting any of the three, or renaming tenants.slug.

begin;

-- ── what createDojo inserts today, verbatim ─────────────────────────────────
insert into dojo.tenants (key, origin, slug, name, dojo_url, scope)
values ('organization/ztest-acme', 'organization', 'ztest-acme', 'Acme',
        'dojo.sensei-hq.org/organization/ztest-acme', 'private');

do $$
declare
    got record;
begin
    select key, origin::text as origin, slug, dojo_url into got
      from dojo.tenants where key = 'organization/ztest-acme';
    if got is null then
        raise exception 'createDojo''s insert produced no tenant row.';
    end if;

    -- The key's first segment IS the origin. dojo-auth.ts resolves a tenant by
    -- joining the two URL segments (`.eq('key', origin || ''/'' || slug)`), so a
    -- key whose prefix disagrees with `origin` yields a tenant unreachable at
    -- its own URL — a 404 on a row that plainly exists.
    if got.key is distinct from (got.origin || '/' || got.slug) then
        raise exception
            'tenant key % does not equal <origin>/<slug> (%/%) — /t/%/% would 404.',
            got.key, got.origin, got.slug, got.origin, got.slug;
    end if;

    -- The dōjō url carries the same key; a stale prefix here points the daemon
    -- at a path that does not resolve.
    if got.dojo_url not like '%' || got.key then
        raise exception 'tenant dojo_url % does not end in its key %.', got.dojo_url, got.key;
    end if;
end $$;

-- ── the second half of createDojo: the creator becomes admin ────────────────
-- Issue #117's acceptance criterion is a tenant row AND an admin membership.
-- `addMember`'s insert carries the caller's PRINCIPAL id (§VIII.2), which is
-- what `resolveCaller` now returns.
insert into dojo.principals (id, auth_user_id, display_name)
values ('aaaaaaaa-1111-1111-1111-111111111111',
        '11111111-1111-1111-1111-111111111111', 'Alice');

insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via, role)
select id, 'aaaaaaaa-1111-1111-1111-111111111111', 'employer', 'sso', 'admin'
  from dojo.tenants where key = 'organization/ztest-acme';

do $$
declare
    role_got text;
    principal_got uuid;
begin
    select m.role::text, m.user_id into role_got, principal_got
      from dojo.memberships m
      join dojo.tenants t on t.id = m.tenant_id
     where t.key = 'organization/ztest-acme';

    if role_got is distinct from 'admin' then
        raise exception
            'createDojo must make the creator an admin, membership role is %.',
            coalesce(role_got, 'NONE');
    end if;

    -- The membership must key on the principal, not the login. If it held the
    -- login id, dojo.owns_membership and every ownership read would miss it —
    -- silently, since the Worker bypasses RLS.
    if principal_got is distinct from 'aaaaaaaa-1111-1111-1111-111111111111' then
        raise exception 'membership.user_id is % — expected the creator''s principal id.',
            principal_got;
    end if;
    if principal_got = '11111111-1111-1111-1111-111111111111' then
        raise exception 'membership.user_id holds the LOGIN id, not the principal id.';
    end if;
end $$;

-- ── the retired spellings must stay rejected ────────────────────────────────
do $$
begin
    -- `origin` is an enum now: personal | organization. 'org' was a label once.
    begin
        insert into dojo.tenants (key, origin, slug, name, dojo_url)
        values ('org/ztest-legacy', 'org', 'ztest-legacy', 'Legacy', 'u');
        raise exception '''org'' is still an accepted tenant_origin — the enum was not narrowed.';
    exception when invalid_text_representation then
        null;  -- expected: not a valid enum label
    end;
end $$;

do $$
begin
    -- The column is `slug`. `org` is what createDojo used to send.
    begin
        execute $q$insert into dojo.tenants (key, origin, org, name, dojo_url)
                   values ('organization/ztest-legacy', 'organization', 'ztest-legacy', 'Legacy', 'u')$q$;
        raise exception 'dojo.tenants still accepts an "org" column — the rename did not happen.';
    exception when undefined_column then
        null;  -- expected
    end;
end $$;

-- ── the collision createDojo maps to 409 ────────────────────────────────────
do $$
begin
    begin
        insert into dojo.tenants (key, origin, slug, name, dojo_url)
        values ('organization/ztest-acme', 'organization', 'ztest-acme', 'Acme Again', 'u');
        raise exception
            'dojo.tenants.key is not unique — createDojo''s 409 on a name collision cannot fire.';
    exception when unique_violation then
        null;  -- expected
    end;
end $$;

-- ── personal and organization namespaces stay apart (§IV.7 / F1) ────────────
-- The whole reason the origin is the key prefix: one unique on `key` has to keep
-- a user named `acme` from colliding with the org `acme`.
insert into dojo.tenants (key, origin, slug, name, dojo_url)
values ('personal/ztest-acme', 'personal', 'ztest-acme', 'Acme''s Dōjō',
        'dojo.sensei-hq.org/personal/ztest-acme');

do $$
declare n int;
begin
    select count(*) into n from dojo.tenants where slug = 'ztest-acme';
    if n <> 2 then
        raise exception
            'personal/ztest-acme and organization/ztest-acme should coexist as 2 rows, found %.', n;
    end if;
end $$;

rollback;
