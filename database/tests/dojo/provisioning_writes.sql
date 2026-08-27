-- The row sequence `ensureProvisioned` writes, against the real schema.
--
-- `provisioning.spec.ts` runs against `fakeDojoDb`, which enforces the unique
-- constraints it was told about — enough to prove idempotence, but it does not
-- know about NOT NULLs, enum labels or foreign keys. So the unit tests would
-- stay green if `kind`, `authenticated_via` or `origin` carried a value the
-- database rejects. Spec §VIII.4 is the standing reason to distrust that.
--
-- WHAT BREAKS THIS TEST: any value ensureProvisioned writes that the real schema
-- will not accept, or a constraint it relies on disappearing.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('aaaaaaaa-1111-1111-1111-111111111111', '11111111-1111-1111-1111-111111111111', 'Jerry'),
  ('bbbbbbbb-2222-2222-2222-222222222222', '22222222-2222-2222-2222-222222222222', 'Bob');

-- ── step 1: the identity ────────────────────────────────────────────────────
-- provider is `dojo.auth_method`, and phase 1 writes 'github_oauth' because the
-- enum has no generic 'oauth' label yet (§VIII.6). If that stops being true the
-- provider mapping in provisioning.ts needs revisiting, so pin it.
insert into dojo.identities (principal_id, provider, subject, email, display_name, last_login_at)
values ('aaaaaaaa-1111-1111-1111-111111111111', 'github_oauth', '4242',
        'j@example.com', 'Jerry Thomas', now());

do $$
begin
    if exists (select 1 from pg_enum e join pg_type t on t.oid = e.enumtypid
                join pg_namespace n on n.oid = t.typnamespace
               where n.nspname = 'dojo' and t.typname = 'auth_method' and e.enumlabel = 'oauth')
    then
        raise exception
            'dojo.auth_method gained ''oauth'' — provisioning.ts should stop hardcoding github_oauth.';
    end if;
end $$;

-- ── step 2: the personal tenant + its admin membership ──────────────────────
insert into dojo.tenants (key, origin, slug, name, dojo_url, scope)
values ('personal/ztest-jerrythomas', 'personal', 'ztest-jerrythomas', 'Jerry Thomas''s Dōjō',
        'dojo.sensei-hq.org/personal/ztest-jerrythomas', 'private');

insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via, role)
select id, 'aaaaaaaa-1111-1111-1111-111111111111', 'personal', 'github_oauth', 'admin'
  from dojo.tenants where key = 'personal/ztest-jerrythomas';

-- ── step 3: an org tenant, its connection, and the derived membership ───────
insert into dojo.tenants (key, origin, slug, name, dojo_url, scope)
values ('organization/ztest-sensei-hq', 'organization', 'ztest-sensei-hq', 'ztest-sensei-hq',
        'dojo.sensei-hq.org/organization/ztest-sensei-hq', 'private');

insert into dojo.tenant_connections
  (tenant_id, provider, external_id, external_slug, connected_by, verified_at)
select id, 'github', '11', 'sensei-hq', 'aaaaaaaa-1111-1111-1111-111111111111', now()
  from dojo.tenants where key = 'organization/ztest-sensei-hq';

insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via, role)
select id, 'aaaaaaaa-1111-1111-1111-111111111111', 'employer', 'github_oauth', 'admin'
  from dojo.tenants where key = 'organization/ztest-sensei-hq';

-- ── the constraints idempotence actually rests on ───────────────────────────
do $$
declare tid uuid;
begin
    select id into tid from dojo.tenants where key = 'organization/ztest-sensei-hq';

    -- (provider, external_id) WHERE external_id IS NOT NULL — one PROVEN forge
    -- org maps to at most one tenant, forever. Without it, a second sign-in
    -- forks the org into a second tenant.
    begin
        insert into dojo.tenant_connections
          (tenant_id, provider, external_id, external_slug, connected_by)
        values (tid, 'github', '11', 'sensei-hq-elsewhere',
                'bbbbbbbb-2222-2222-2222-222222222222');
        raise exception
            'tenant_connections (provider, external_id) is not unique — one forge org could map to two tenants.';
    exception when unique_violation then
        null;  -- expected
    end;

    -- (tenant_id, user_id) — what makes ensureMembership converge under the
    -- concurrent sign-in of Part I Scenario 22 instead of duplicating.
    begin
        insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via, role)
        values (tid, 'aaaaaaaa-1111-1111-1111-111111111111', 'employer', 'github_oauth', 'contributor');
        raise exception 'dojo.memberships (tenant_id, user_id) is not unique.';
    exception when unique_violation then
        null;  -- expected
    end;

    -- (provider, subject) — one forge account is one person. This is what makes
    -- the 409 in ensureIdentity a real guarantee rather than a hope.
    begin
        insert into dojo.identities (principal_id, provider, subject)
        values ('bbbbbbbb-2222-2222-2222-222222222222', 'github_oauth', '4242');
        raise exception 'dojo.identities (provider, subject) is not unique.';
    exception when unique_violation then
        null;  -- expected
    end;

    -- A SECOND member of the same org joins the SAME tenant. The row that
    -- `syncGithubMemberships` could never create, because nothing made the
    -- tenant in the first place.
    insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via, role)
    values (tid, 'bbbbbbbb-2222-2222-2222-222222222222', 'employer', 'github_oauth', 'contributor');
end $$;

-- ── what the pass produced ──────────────────────────────────────────────────
do $$
declare
    personal_n int;
    org_members int;
begin
    select count(*) into personal_n
      from dojo.tenants t
      join dojo.memberships m on m.tenant_id = t.id
     where t.origin = 'personal'
       and m.user_id = 'aaaaaaaa-1111-1111-1111-111111111111'
       and m.role = 'admin';
    if personal_n <> 1 then
        raise exception 'expected exactly one personal dōjō with the user as admin, found %.', personal_n;
    end if;

    select count(*) into org_members
      from dojo.memberships m
      join dojo.tenants t on t.id = m.tenant_id
     where t.key = 'organization/ztest-sensei-hq';
    if org_members <> 2 then
        raise exception 'expected both members on the one org tenant, found %.', org_members;
    end if;
end $$;

rollback;
