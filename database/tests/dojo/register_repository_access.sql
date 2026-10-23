-- Registering a repository as the signed-in person (#236 step 3; access matrix
-- "Workspaces and repositories": a member registers, the daemon reports).
--
-- dojo.register_repository maps a repository to the ONE tenant of the caller's
-- that its forge org is connected to, and files it there. It reads every
-- tenant's connections to tell "nobody connected this org" from "a dōjō did, but
-- you are not in it" — and returns only a reason code for the second, never
-- another tenant's id. A session neither reads nor writes dojo.repositories
-- directly.
--
-- WHAT BREAKS THIS TEST: a mapping that ignores membership, a disabled
-- membership, or ambiguity; registering a key whose path does not carry the org
-- it claims; an insert that duplicates on a second registration; any session
-- grant on dojo.repositories.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000111', 'a1000000-0000-4000-8000-000000000111', 'ola'),
  ('a0000000-0000-4000-8000-000000000112', 'a1000000-0000-4000-8000-000000000112', 'dis');

-- T ↔ ztest-acme · U ↔ ztest-other · V and W both ↔ ztest-dup
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000011a', 'organization/ztest-reg-t', 'organization', 'ztest-reg-t', 'T', 'x'),
  ('b0000000-0000-4000-8000-00000000011b', 'organization/ztest-reg-u', 'organization', 'ztest-reg-u', 'U', 'x'),
  ('b0000000-0000-4000-8000-00000000011c', 'organization/ztest-reg-v', 'organization', 'ztest-reg-v', 'V', 'x'),
  ('b0000000-0000-4000-8000-00000000011d', 'organization/ztest-reg-w', 'organization', 'ztest-reg-w', 'W', 'x');

-- V and W are two PROVEN orgs (different forge ids) that have shared a slug —
-- an org renamed upstream and its old name re-registered: the real ambiguity.
insert into dojo.tenant_connections (tenant_id, provider, external_id, external_slug, connected_by) values
  ('b0000000-0000-4000-8000-00000000011a', 'github', null,          'ztest-acme',  'a0000000-0000-4000-8000-000000000111'),
  ('b0000000-0000-4000-8000-00000000011b', 'github', null,          'ztest-other', 'a0000000-0000-4000-8000-000000000111'),
  ('b0000000-0000-4000-8000-00000000011c', 'github', 'ztest-gh-1',  'ztest-dup',   'a0000000-0000-4000-8000-000000000111'),
  ('b0000000-0000-4000-8000-00000000011d', 'github', 'ztest-gh-2',  'ztest-dup',   'a0000000-0000-4000-8000-000000000111');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, disabled_at) values
  ('b0000000-0000-4000-8000-00000000011a', 'a0000000-0000-4000-8000-000000000111', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000011c', 'a0000000-0000-4000-8000-000000000111', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000011d', 'a0000000-0000-4000-8000-000000000111', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000011a', 'a0000000-0000-4000-8000-000000000112', 'contributor', 'employer', 'github_oauth', now()),
  ('b0000000-0000-4000-8000-00000000011b', 'a0000000-0000-4000-8000-000000000112', 'contributor', 'employer', 'github_oauth', null);

set local role authenticated;

do $$
declare
    r record;
    first_id uuid;
    n int;
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000111"}', true);

    select * into r from dojo.register_repository('github.com/ztest-acme/api', 'github', 'ztest-acme', 'api', null);
    if r.reason is not null or r.tenant_id <> 'b0000000-0000-4000-8000-00000000011a' or r.repository_id is null then
        raise exception 'register: ola''s repo should map to T, got %', r;
    end if;
    if r.tenant_key <> 'organization/ztest-reg-t' then raise exception 'register: tenant key %', r.tenant_key; end if;
    first_id := r.repository_id;
    select * into r from dojo.register_repository('github.com/ztest-acme/api', 'github', 'ztest-acme', 'api', null);
    if r.repository_id is distinct from first_id then
        raise exception 'register: a second registration made a second row (% vs %)', r.repository_id, first_id;
    end if;

    select * into r from dojo.register_repository('github.com/ztest-other/x', 'github', 'ztest-other', 'x', null);
    if r.reason is distinct from 'not_a_member' or r.tenant_id is not null then
        raise exception 'register: an org connected to a tenant ola is not in must be not_a_member, got %', r;
    end if;
    select * into r from dojo.register_repository('github.com/ztest-none/x', 'github', 'ztest-none', 'x', null);
    if r.reason is distinct from 'no_connection' then raise exception 'register: expected no_connection, got %', r; end if;
    select * into r from dojo.register_repository('github.com/ztest-dup/x', 'github', 'ztest-dup', 'x', null);
    if r.reason is distinct from 'ambiguous' or r.tenant_id is not null then
        raise exception 'register: a slug in two of ola''s tenants must be ambiguous, got %', r;
    end if;

    begin
        perform dojo.register_repository('github.com/someone-else/x', 'github', 'ztest-acme', 'x', null);
        raise exception 'register: a key whose path does not carry its org was filed';
    exception when invalid_parameter_value then null;
    end;

    -- The org is read at its place in the key: a repository NAMED like the org
    -- (someone-else/ztest-acme) is not the org's.
    begin
        perform dojo.register_repository('github.com/someone-else/ztest-acme', 'github', 'ztest-acme', 'ztest-acme', null);
        raise exception 'register: a repository named like an org was filed under that org';
    exception when invalid_parameter_value then null;
    end;

    begin
        select count(*) into n from dojo.repositories;
        raise exception 'repositories: a session read the table directly';
    exception when insufficient_privilege then null;
    end;

    -- dis: disabled in T — no tenant of theirs.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000112"}', true);
    select * into r from dojo.register_repository('github.com/ztest-acme/web', 'github', 'ztest-acme', 'web', null);
    if r.reason is distinct from 'not_a_member' then
        raise exception 'register: a disabled membership must not register, got %', r;
    end if;
    -- …and the disabled one does not spoil the active one.
    select * into r from dojo.register_repository('github.com/ztest-other/y', 'github', 'ztest-other', 'y', null);
    if r.reason is not null or r.tenant_id <> 'b0000000-0000-4000-8000-00000000011b' then
        raise exception 'register: dis''s ACTIVE membership in U should still map, got %', r;
    end if;
end $$;

reset role;
do $$
begin
    if (select count(*) from dojo.repositories where repo_key like 'github.com/ztest-%') <> 2 then
        raise exception 'register: exactly two repositories should exist, found %',
            (select count(*) from dojo.repositories where repo_key like 'github.com/ztest-%');
    end if;
end $$;

rollback;
