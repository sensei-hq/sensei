-- Creating a dōjō (POST /v1/you/dojos, issue #117), run against a real database.
--
-- createDojo used to issue two inserts as the service role — the tenant, then
-- the creator's admin membership — and once sent retired columns (`origin:
-- 'org'`, `org: slug`, `key: 'org/{slug}'`) that a mocked test asserted and
-- passed (spec §VIII.4). It now calls dojo.create_dojo as the signed-in person
-- (#236): one transaction, the key and url built in the database, the admin
-- membership the SESSION's own principal — never a user the caller names.
--
-- WHAT BREAKS THIS TEST: a key whose prefix is not the origin, or a url that
-- does not end in the key; a membership on anyone but the session's principal,
-- or on its login id; a tenant left behind when the membership fails; a slug
-- that is not a slug; creating with no session; reverting the enum or the
-- `slug` rename; a non-unique key.

begin;

insert into dojo.principals (id, auth_user_id, display_name)
values ('aaaaaaaa-1111-1111-1111-111111111111', '11111111-1111-1111-1111-111111111111', 'Alice');

set local role authenticated;

do $$
declare
    r record;
    got record;
    n int;
begin
    -- No session: nobody to make admin.
    perform set_config('request.jwt.claims', '', true);
    begin
        perform dojo.create_dojo('Nobody', 'ztest-nobody', 'employer');
        raise exception 'create_dojo: created a dōjō with no session';
    exception when insufficient_privilege then null;
    end;

    perform set_config('request.jwt.claims', '{"sub":"11111111-1111-1111-1111-111111111111"}', true);
    select * into r from dojo.create_dojo('Acme', 'ztest-acme', 'employer');
    if r.key <> 'organization/ztest-acme' or r.name <> 'Acme' or r.id is null then
        raise exception 'create_dojo returned %', r;
    end if;

    begin
        perform dojo.create_dojo('Acme', 'ZTest Acme!', 'employer');
        raise exception 'create_dojo: accepted a slug that is not a slug';
    exception when invalid_parameter_value then null;
    end;

    -- The collision createDojo maps to 409, and it leaves nothing behind.
    begin
        perform dojo.create_dojo('Acme Again', 'ztest-acme', 'employer');
        raise exception 'dojo.tenants.key is not unique — createDojo''s 409 cannot fire.';
    exception when unique_violation then null;
    end;
end $$;

reset role;

do $$
declare
    got record;
    n int;
begin
    select t.key, t.origin::text as origin, t.slug, t.dojo_url, m.role::text as role, m.user_id
      into got
      from dojo.tenants t join dojo.memberships m on m.tenant_id = t.id
     where t.key = 'organization/ztest-acme';
    if got is null then raise exception 'create_dojo produced no tenant with a membership.'; end if;

    -- The key's first segment IS the origin. dojo-auth.ts resolves a tenant by
    -- joining the two URL segments, so a key whose prefix disagrees with `origin`
    -- yields a tenant unreachable at its own URL — a 404 on a row that exists.
    if got.key is distinct from (got.origin || '/' || got.slug) then
        raise exception 'tenant key % does not equal <origin>/<slug> (%/%).', got.key, got.origin, got.slug;
    end if;
    -- The dōjō url carries the same key; a stale prefix points the daemon at a
    -- path that does not resolve.
    if got.dojo_url not like '%' || got.key then
        raise exception 'tenant dojo_url % does not end in its key %.', got.dojo_url, got.key;
    end if;
    if got.role is distinct from 'admin' then
        raise exception 'create_dojo must make the creator an admin, role is %.', coalesce(got.role, 'NONE');
    end if;
    -- The membership keys on the PRINCIPAL, not the login: dojo.owns_membership
    -- and every ownership read would miss a login id.
    if got.user_id is distinct from 'aaaaaaaa-1111-1111-1111-111111111111' then
        raise exception 'membership.user_id is % — expected the creator''s principal id.', got.user_id;
    end if;

    select count(*) into n from dojo.tenants where slug like 'ztest-%' and key like 'organization/%';
    if n <> 1 then raise exception 'a failed create_dojo left a tenant behind (% organization tenants).', n; end if;
end $$;

-- ── the retired spellings must stay rejected ────────────────────────────────
do $$
begin
    begin
        insert into dojo.tenants (key, origin, slug, name, dojo_url)
        values ('org/ztest-legacy', 'org', 'ztest-legacy', 'Legacy', 'u');
        raise exception '''org'' is still an accepted tenant_origin — the enum was not narrowed.';
    exception when invalid_text_representation then
        null;
    end;
    begin
        execute $q$insert into dojo.tenants (key, origin, org, name, dojo_url)
                   values ('organization/ztest-legacy', 'organization', 'ztest-legacy', 'Legacy', 'u')$q$;
        raise exception 'dojo.tenants still accepts an "org" column — the rename did not happen.';
    exception when undefined_column then
        null;
    end;
end $$;

-- ── personal and organization namespaces stay apart (§IV.7 / F1) ────────────
insert into dojo.tenants (key, origin, slug, name, dojo_url)
values ('personal/ztest-acme', 'personal', 'ztest-acme', 'Acme''s Dōjō',
        'dojo.sensei-hq.org/personal/ztest-acme');

do $$
declare n int;
begin
    select count(*) into n from dojo.tenants where slug = 'ztest-acme';
    if n <> 2 then
        raise exception 'personal/ztest-acme and organization/ztest-acme should coexist as 2 rows, found %.', n;
    end if;
end $$;

rollback;
