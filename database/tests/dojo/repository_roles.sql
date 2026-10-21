-- Repository roles (#307 A2, D5): access lives on repositories. A person's role on
-- a repository mirrors the forge's per-repo permission — read / triage / write →
-- contributor, maintain → maintainer, admin → admin — refreshed by the mirror job,
-- and a dōjō admin may override it (including to no access). The override wins and
-- survives the next mirror. A role counts only with an active membership in the
-- repository's dōjō.
--
-- WHAT BREAKS THIS TEST: a wrong mapping; an override lost on re-mirror; someone
-- the forge no longer lists keeping their forge role; a non-admin overriding; a
-- signed-in caller running the mirror or writing the table; a role counting
-- without a membership; one repository's role reading on another; a member seeing
-- other people's roles.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000321', 'a1000000-0000-4000-8000-000000000321', 'ada'),
  ('a0000000-0000-4000-8000-000000000322', 'a1000000-0000-4000-8000-000000000322', 'dev'),
  ('a0000000-0000-4000-8000-000000000323', 'a1000000-0000-4000-8000-000000000323', 'mae'),
  ('a0000000-0000-4000-8000-000000000324', 'a1000000-0000-4000-8000-000000000324', 'gone');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000032a', 'organization/ztest-rr-t', 'organization', 'ztest-rr-t', 'T', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000032a', 'a0000000-0000-4000-8000-000000000321', 'admin', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000032a', 'a0000000-0000-4000-8000-000000000322', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000032a', 'a0000000-0000-4000-8000-000000000323', 'contributor', 'employer', 'github_oauth');
insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-000000000321', 'b0000000-0000-4000-8000-00000000032a', 'github.com/ztest-rr/r', 'r', 'github'),
  ('90000000-0000-4000-8000-000000000322', 'b0000000-0000-4000-8000-00000000032a', 'github.com/ztest-rr/s', 's', 'github');

-- The mapping, as a pure function.
do $$
begin
    if dojo.forge_permission_to_role('read') <> 'contributor'
       or dojo.forge_permission_to_role('triage') <> 'contributor'
       or dojo.forge_permission_to_role('write') <> 'contributor'
       or dojo.forge_permission_to_role('maintain') <> 'maintainer'
       or dojo.forge_permission_to_role('admin') <> 'admin' then
        raise exception 'forge permission mapping is wrong';
    end if;
    if dojo.forge_permission_to_role('none') is not null then
        raise exception 'an unknown forge permission mapped to a role';
    end if;
end $$;

-- The mirror runs server-side (service_role): dev writes, mae maintains, gone reads.
set local role service_role;
select dojo.mirror_repository_roles('90000000-0000-4000-8000-000000000321',
  '[{"principal_id":"a0000000-0000-4000-8000-000000000322","permission":"write"},
    {"principal_id":"a0000000-0000-4000-8000-000000000323","permission":"maintain"},
    {"principal_id":"a0000000-0000-4000-8000-000000000324","permission":"read"}]'::jsonb);
reset role;

set local role authenticated;

do $$
declare
    r constant uuid := '90000000-0000-4000-8000-000000000321';
    s constant uuid := '90000000-0000-4000-8000-000000000322';
    dev  constant uuid := 'a0000000-0000-4000-8000-000000000322';
    mae  constant uuid := 'a0000000-0000-4000-8000-000000000323';
    n int;
begin
    -- A signed-in caller cannot run the mirror.
    begin
        perform dojo.mirror_repository_roles(r, '[]'::jsonb);
        raise exception 'a signed-in caller ran the forge mirror';
    exception when insufficient_privilege then null;
    end;

    -- gone has a forge read on r but no membership in the dōjō: no role counts.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000324"}', true);
    if dojo.repo_role_at_least(r, 'contributor') then raise exception 'a role counted without a membership'; end if;

    -- mae: maintainer on r, nothing on s.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000323"}', true);
    if not dojo.repo_role_at_least(r, 'maintainer') then raise exception 'the forge maintain did not make mae a maintainer on r'; end if;
    if dojo.repo_role_at_least(s, 'contributor') then raise exception 'a role on r read as a role on s'; end if;

    -- A member sees only their own roles.
    select count(*) into n from dojo.repository_roles;
    if n <> 1 then raise exception 'a member saw % repository roles, expected only their own', n; end if;

    -- A contributor cannot override; nobody writes the table directly.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000322"}', true);
    if not dojo.repo_role_at_least(r, 'contributor') or dojo.repo_role_at_least(r, 'maintainer') then
        raise exception 'forge write did not map to exactly contributor';
    end if;
    begin
        perform dojo.set_repository_role_override(r, dev, 'admin');
        raise exception 'a non-admin set an override';
    exception when insufficient_privilege then null;
    end;
    begin
        insert into dojo.repository_roles (repository_id, principal_id, forge_role) values (s, dev, 'admin');
        raise exception 'a direct insert into repository_roles was accepted';
    exception when insufficient_privilege then null;
    end;

    -- The admin raises dev to maintainer and takes mae's access away.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000321"}', true);
    perform dojo.set_repository_role_override(r, dev, 'maintainer');
    perform dojo.set_repository_role_override(r, mae, null);
    select count(*) into n from dojo.repository_roles where repository_id = r;
    if n <> 3 then raise exception 'the admin saw % roles on r, expected 3', n; end if;
end $$;

-- The forge reports again — gone has left; mae's and dev's permissions are unchanged.
reset role;
set local role service_role;
select dojo.mirror_repository_roles('90000000-0000-4000-8000-000000000321',
  '[{"principal_id":"a0000000-0000-4000-8000-000000000322","permission":"write"},
    {"principal_id":"a0000000-0000-4000-8000-000000000323","permission":"maintain"}]'::jsonb);
reset role;

set local role authenticated;

do $$
declare
    r constant uuid := '90000000-0000-4000-8000-000000000321';
    dev constant uuid := 'a0000000-0000-4000-8000-000000000322';
    fr dojo.member_role;
begin
    -- Overrides survive the re-mirror.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000322"}', true);
    if not dojo.repo_role_at_least(r, 'maintainer') then raise exception 'the override was lost on re-mirror'; end if;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000323"}', true);
    if dojo.repo_role_at_least(r, 'contributor') then raise exception 'a no-access override was lost on re-mirror'; end if;

    -- Clearing the override returns dev to what the forge says.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000321"}', true);
    perform dojo.clear_repository_role_override(r, dev);
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000322"}', true);
    if dojo.repo_role_at_least(r, 'maintainer') then raise exception 'clearing the override left dev a maintainer'; end if;

    -- gone left the repository on the forge: no role, even before anything else.
    reset role;
    select forge_role into fr from dojo.repository_roles
     where repository_id = r and principal_id = 'a0000000-0000-4000-8000-000000000324';
    if fr is not null then raise exception 'someone the forge no longer lists kept forge role %', fr; end if;
end $$;


rollback;
