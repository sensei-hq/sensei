-- Electing a repository for sharing, as the signed-in person (#236 step 3;
-- access matrix "Projects and repositories": repository elections R/W own (user
-- authority), R/W tenant for an admin (org authority)).
--
-- dojo.set_repository_election decides from the CALLER's own verdict row (the
-- one dojo.my_repositories returns): who holds the authority, and whether this
-- person may change it. It refuses with a distinct SQLSTATE per reason — no such
-- repository for the caller (no_data_found), no authority yet
-- (object_not_in_prerequisite_state), not theirs to change
-- (insufficient_privilege) — and re-reads the verdict it reports.
--
-- The election, share-policy and connection tables are reached only through
-- SECURITY DEFINER functions: RLS on, no session grant.
--
-- WHAT BREAKS THIS TEST: deciding from anyone's row but the caller's; skipping
-- the authority or configurable_by_me check; a user election filed without the
-- caller as its principal; an org election filed WITH one; an upsert that
-- duplicates; any session grant on repository_elections; RLS left off.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000121', 'a1000000-0000-4000-8000-000000000121', 'ada'),
  ('a0000000-0000-4000-8000-000000000122', 'a1000000-0000-4000-8000-000000000122', 'ola');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-00000000012a', 'organization/ztest-el-t', 'organization', 'ztest-el-t', 'T', 'x', now());
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000012a', 'a0000000-0000-4000-8000-000000000121', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000012a', 'a0000000-0000-4000-8000-000000000122', 'contributor', 'employer', 'github_oauth');
insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-00000000012a', 'team', 'active', current_date - 1, current_date + 30);

-- priv: org-private (organization authority) · pub: public (user authority) · unk: never captured
insert into dojo.repositories (tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('b0000000-0000-4000-8000-00000000012a', 'github.com/ztest-el/priv', 'priv', 'private', now(), 'github'),
  ('b0000000-0000-4000-8000-00000000012a', 'github.com/ztest-el/pub',  'pub',  'public',  now(), 'github'),
  ('b0000000-0000-4000-8000-00000000012a', 'github.com/ztest-el/unk',  'unk',  null,      null,  'github');

set local role authenticated;

do $$
declare
    r record;
    n int;
begin
    -- ── ola (contributor) ────────────────────────────────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000122"}', true);

    select * into r from dojo.set_repository_election('github.com/ztest-el/pub', true);
    if r.elected is distinct from true or r.authority <> 'user' or r.sync_enabled is distinct from true then
        raise exception 'election: ola''s own public repo should elect and sync, got %', r;
    end if;
    select * into r from dojo.set_repository_election('github.com/ztest-el/pub', false);
    if r.elected is distinct from false then raise exception 'election: ola''s second choice should stand, got %', r; end if;

    begin
        perform dojo.set_repository_election('github.com/ztest-el/priv', true);
        raise exception 'election: a contributor changed the organisation''s election';
    exception when insufficient_privilege then null;
    end;
    begin
        perform dojo.set_repository_election('github.com/ztest-el/unk', true);
        raise exception 'election: a repository with no authority was elected';
    exception when object_not_in_prerequisite_state then null;
    end;
    begin
        perform dojo.set_repository_election('github.com/ztest-el/none', true);
        raise exception 'election: an unknown repository was elected';
    exception when no_data_found then null;
    end;

    begin
        select count(*) into n from dojo.repository_elections;
        raise exception 'repository_elections: a session read the table directly';
    exception when insufficient_privilege then null;
    end;

    -- ── ada (admin) holds the organisation's election ────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000121"}', true);
    select * into r from dojo.set_repository_election('github.com/ztest-el/priv', true);
    if r.elected is distinct from true or r.authority <> 'organization' then
        raise exception 'election: the admin should elect the org repo, got %', r;
    end if;
    select * into r from dojo.set_repository_election('github.com/ztest-el/priv', true);

    -- ola now sees the organisation's choice.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000122"}', true);
    select elected into r from dojo.my_repositories() where repo_key = 'github.com/ztest-el/priv';
    if r.elected is distinct from true then raise exception 'election: ola should see the org''s election'; end if;
end $$;

reset role;
do $$
declare
    rows text;
begin
    select string_agg(authority || ':' || coalesce(e.principal_id::text, 'none') || ':' || elected, ', '
                      order by authority)
      into rows
      from dojo.repository_elections e join dojo.repositories r on r.id = e.repository_id
     where r.repo_key like 'github.com/ztest-el/%';
    if rows is distinct from 'user:a0000000-0000-4000-8000-000000000122:false, organization:none:true' then
        raise exception 'elections stored: %', rows;
    end if;
    if exists (select 1 from pg_class c join pg_namespace s on s.oid = c.relnamespace
                where s.nspname = 'dojo'
                  and c.relname in ('repository_elections', 'tenant_share_policy', 'tenant_connections')
                  and not c.relrowsecurity) then
        raise exception 'RLS must be on for repository_elections, tenant_share_policy and tenant_connections';
    end if;
end $$;

rollback;
