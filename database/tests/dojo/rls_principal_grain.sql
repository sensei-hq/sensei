-- The dōjō's row-level security must resolve a login to its PRINCIPAL.
--
-- Spec: docs/spec/dojo/dojo-auth-provisioning.md §VIII.2. `dojo.principals` is
-- the stable identity every dōjō foreign key points at; `principals.auth_user_id`
-- is a re-pointable POINTER at the Supabase login. So `memberships.user_id` and
-- `projects.user_id` hold a PRINCIPAL id, and a policy comparing one of them to
-- `auth.uid()` — which is the *login* id — matches nothing.
--
-- That failure is invisible from the application: the Worker connects as
-- `service_role`, which bypasses RLS entirely. Only a client-direct read as
-- `authenticated` can see it, which is what this file does.
--
-- WHAT BREAKS THIS TEST: reverting any of the three surfaces to compare against
-- the login id — `dojo.projects`'s policy to `user_id = auth.uid()`,
-- `dojo.owns_membership` to `m.user_id = auth.uid()`, or deleting
-- `dojo.current_principal_id()`. Each turns a visible row invisible.

begin;

-- ── fixture: two people, each with a login and a principal ───────────────────
-- Alice and Bob are separate principals in one tenant. Bob exists solely so the
-- assertions can distinguish "the policy resolves the principal" from "the
-- policy lets everyone see everything" — a test with one user passes under both.

insert into dojo.principals (id, auth_user_id, display_name) values
  ('aaaaaaaa-1111-1111-1111-111111111111', '11111111-1111-1111-1111-111111111111', 'Alice'),
  ('bbbbbbbb-2222-2222-2222-222222222222', '22222222-2222-2222-2222-222222222222', 'Bob');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('cccccccc-3333-3333-3333-333333333333', 'organization/ztest-rls-fixture', 'organization',
   'ztest-rls-fixture', 'RLS fixture', 'dojo.sensei-hq.org/organization/ztest-rls-fixture');

-- memberships.user_id holds the PRINCIPAL id (§VIII.2), not the login id.
insert into dojo.memberships (id, tenant_id, user_id, kind, authenticated_via) values
  ('dddddddd-4444-4444-4444-444444444444', 'cccccccc-3333-3333-3333-333333333333',
   'aaaaaaaa-1111-1111-1111-111111111111', 'employer', 'github_oauth'),
  ('eeeeeeee-5555-5555-5555-555555555555', 'cccccccc-3333-3333-3333-333333333333',
   'bbbbbbbb-2222-2222-2222-222222222222', 'employer', 'github_oauth');

-- projects.user_id likewise.
insert into dojo.projects (user_id, tenant_id, slug, name) values
  ('aaaaaaaa-1111-1111-1111-111111111111', 'cccccccc-3333-3333-3333-333333333333',
   'rls-fixture-alice', 'Alice''s project'),
  ('bbbbbbbb-2222-2222-2222-222222222222', 'cccccccc-3333-3333-3333-333333333333',
   'rls-fixture-bob', 'Bob''s project');

-- ── read the way a signed-in browser does ────────────────────────────────────
-- auth.uid() reads request.jwt.claims ->> 'sub', so this pair is what a real
-- Supabase JWT amounts to as far as a policy is concerned. Note the sub is
-- Alice's LOGIN id — the thing the policy must translate into her principal id.
do $$ begin
    perform set_config('request.jwt.claims',
                       '{"sub":"11111111-1111-1111-1111-111111111111"}', true);
end $$;
set local role authenticated;

do $$
declare
    mine   int;
    theirs int;
    owns_own   boolean;
    owns_other boolean;
begin
    -- 1. Alice sees her own project. Fails when the policy compares a principal
    --    id to a login id: the row is hers, and she cannot see it.
    select count(*) into mine
      from dojo.projects where slug = 'rls-fixture-alice';
    if mine <> 1 then
        raise exception
            'dojo.projects RLS: Alice should see her own project, saw % row(s). %',
            mine, 'The policy is not resolving auth.uid() to a principal id.';
    end if;

    -- 2. …and only her own. Guards the opposite failure — a policy relaxed into
    --    `using (true)` would satisfy assertion 1 and leak every user's projects.
    select count(*) into theirs
      from dojo.projects where slug = 'rls-fixture-bob';
    if theirs <> 0 then
        raise exception
            'dojo.projects RLS: Alice must not see Bob''s project, saw % row(s).',
            theirs;
    end if;

    -- 3. dojo.owns_membership resolves the same way. It backs the relay_sessions
    --    / relay_inbox / relay_segments policies, so the same login-vs-principal
    --    confusion silently empties all three.
    select dojo.owns_membership('dddddddd-4444-4444-4444-444444444444') into owns_own;
    if not owns_own then
        raise exception
            'dojo.owns_membership: Alice should own her own membership. %',
            'It is comparing memberships.user_id (a principal id) to a login id.';
    end if;

    -- 4. …and not Bob's. Catches a predicate loosened to `true`.
    select dojo.owns_membership('eeeeeeee-5555-5555-5555-555555555555') into owns_other;
    if owns_other then
        raise exception 'dojo.owns_membership: Alice must not own Bob''s membership.';
    end if;
end $$;

reset role;

-- ── the shape principal-resolve.ts depends on ────────────────────────────────
-- Its unit tests mock supabase-js, so they assert the payload the module SENDS
-- and would stay green if these columns were renamed underneath it — the exact
-- failure mode of §VIII.4. This is the round-trip those tests cannot make.
do $$
declare found uuid;
begin
    -- The lookup: select id where auth_user_id = <login>.
    select id into found
      from dojo.principals
     where auth_user_id = '11111111-1111-1111-1111-111111111111';
    if found is distinct from 'aaaaaaaa-1111-1111-1111-111111111111' then
        raise exception
            'dojo.principals lookup by auth_user_id returned %, expected Alice''s principal.',
            coalesce(found::text, 'NULL');
    end if;

    -- The insert: (auth_user_id, display_name) returning id. A rename of either
    -- column breaks resolvePrincipalId, and only this notices.
    insert into dojo.principals (auth_user_id, display_name)
    values ('33333333-3333-3333-3333-333333333333', 'Carol')
    returning id into found;
    if found is null then
        raise exception 'dojo.principals insert did not return an id.';
    end if;

    -- auth_user_id is UNIQUE, which is what makes the concurrent-sign-in retry
    -- in resolvePrincipalId correct rather than a guess. Prove the constraint
    -- is really there: without it the retry path is dead code and two principals
    -- could exist for one human.
    begin
        insert into dojo.principals (auth_user_id) values ('33333333-3333-3333-3333-333333333333');
        raise exception 'dojo.principals.auth_user_id is NOT unique — the 23505 retry path cannot fire.';
    exception when unique_violation then
        null;  -- expected
    end;
end $$;

rollback;
