-- `dojo.identities` is GLOBAL and keys on principal_id.
--
-- It was tenant-scoped once, and `admin-data.ts` / `identity-resolve.ts` went on
-- filtering `.eq('tenant_id', …)` and selecting `user_id` long after the columns
-- were gone (commit 75565304). Every identity route and the whole members screen
-- 500'd on `column "user_id" does not exist`, while 1328 mocked tests stayed
-- green — spec dojo-auth-provisioning §VIII.4.
--
-- These assertions are the ones a mocked supabase-js client cannot make.
--
-- WHAT BREAKS THIS TEST: reintroducing a tenant_id or user_id filter on
-- identities, renaming principal_id, dropping the (provider, subject) unique, or
-- changing the column list `IDENTITY_COLS` selects.

begin;

-- ── the columns the code selects must exist, and the retired ones must not ───
do $$
declare
    missing text;
    resurrected text;
begin
    -- IDENTITY_COLS in admin-data.ts, verbatim.
    select string_agg(c, ', ') into missing
      from unnest(array['id', 'principal_id', 'provider', 'subject',
                        'email', 'display_name', 'created_at', 'last_login_at']) as c
     where not exists (
       select 1 from information_schema.columns
        where table_schema = 'dojo' and table_name = 'identities' and column_name = c);
    if missing is not null then
        raise exception 'dojo.identities is missing column(s) the code selects: %', missing;
    end if;

    -- The two the code used to filter on. If either comes back, the scoping
    -- decision in admin-data.ts (an explicit membership check) needs revisiting
    -- rather than silently coexisting with a column that would also work.
    select string_agg(c, ', ') into resurrected
      from unnest(array['tenant_id', 'user_id']) as c
     where exists (
       select 1 from information_schema.columns
        where table_schema = 'dojo' and table_name = 'identities' and column_name = c);
    if resurrected is not null then
        raise exception
            'dojo.identities grew back column(s) %; identity scoping assumes they do not exist.',
            resurrected;
    end if;
end $$;

-- ── the scoping read: tenant → memberships → principals → identities ─────────
insert into dojo.principals (id, auth_user_id, display_name) values
  ('aaaaaaaa-1111-1111-1111-111111111111', '11111111-1111-1111-1111-111111111111', 'Alice'),
  ('bbbbbbbb-2222-2222-2222-222222222222', '22222222-2222-2222-2222-222222222222', 'Bob');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('cccccccc-3333-3333-3333-333333333333', 'organization/ztest-ident-a', 'organization',
   'ztest-ident-a', 'Tenant A', 'dojo.sensei-hq.org/organization/ztest-ident-a'),
  ('dddddddd-4444-4444-4444-444444444444', 'organization/ztest-ident-b', 'organization',
   'ztest-ident-b', 'Tenant B', 'dojo.sensei-hq.org/organization/ztest-ident-b');

-- Alice is in tenant A, Bob in tenant B. memberships.user_id holds a PRINCIPAL id.
insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via) values
  ('cccccccc-3333-3333-3333-333333333333', 'aaaaaaaa-1111-1111-1111-111111111111',
   'employer', 'github_oauth'),
  ('dddddddd-4444-4444-4444-444444444444', 'bbbbbbbb-2222-2222-2222-222222222222',
   'employer', 'github_oauth');

insert into dojo.identities (principal_id, provider, subject, display_name) values
  ('aaaaaaaa-1111-1111-1111-111111111111', 'github_oauth', 'gh|alice', 'Alice'),
  ('bbbbbbbb-2222-2222-2222-222222222222', 'github_oauth', 'gh|bob', 'Bob');

do $$
declare
    n int;
    subj text;
begin
    -- listIdentities: the tenant's members' principals, and only those. Bob's
    -- identity must not appear under tenant A — this is the check that replaced
    -- the dropped tenant_id filter.
    select count(*), min(i.subject) into n, subj
      from dojo.identities i
     where i.principal_id in (select m.user_id
                                from dojo.memberships m
                               where m.tenant_id = 'cccccccc-3333-3333-3333-333333333333');
    if n <> 1 or subj <> 'gh|alice' then
        raise exception
            'identity scoping: tenant A should see exactly Alice''s identity, saw % row(s) (%).',
            n, coalesce(subj, 'none');
    end if;

    -- (provider, subject) is UNIQUE and GLOBAL — one GitHub account is one
    -- person everywhere, which is what makes createIdentity's 409 meaningful and
    -- what lets one sign-in fan out to many dōjōs without duplicating the row.
    begin
        insert into dojo.identities (principal_id, provider, subject)
        values ('bbbbbbbb-2222-2222-2222-222222222222', 'github_oauth', 'gh|alice');
        raise exception
            'dojo.identities (provider, subject) is NOT unique — one forge account could map to two people.';
    exception when unique_violation then
        null;  -- expected
    end;
end $$;

rollback;
