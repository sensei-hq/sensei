-- `dojo.file_repository_workspace` — the sync files a repository under its sensei
-- workspace (#299, decided: one dōjō workspace per sensei workspace).
--
-- Before this, `dojo.workspaces` was written only by a relay run and
-- `dojo.repositories_in_workspaces` by nothing at all, so a synced repository
-- never appeared under any workspace.
--
-- A workspace is ONE SHARED item, keyed (tenant_id, slug) in the dōjō it lives in
-- — an org's, or the personal dōjō of the owner's GitHub account (#299).
--
-- WHAT BREAKS THIS TEST: filing for a repository in a tenant the caller is not a
-- member of; a second filing — by the same member or ANOTHER — creating a second
-- workspace or a second link instead of converging; a moved repository keeping its
-- old link (two workspaces in one tenant); a personal-dōjō workspace landing outside
-- that dōjō; or the link table opening to direct writes.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000171', 'a1000000-0000-4000-8000-000000000171', 'ola'),
  ('a0000000-0000-4000-8000-000000000172', 'a1000000-0000-4000-8000-000000000172', 'eve'),
  ('a0000000-0000-4000-8000-000000000173', 'a1000000-0000-4000-8000-000000000173', 'ada');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000017a', 'organization/ztest-rpf-t', 'organization', 'ztest-rpf-t', 'T', 'x'),
  ('b0000000-0000-4000-8000-00000000017b', 'personal/ztest-rpf-ola',   'personal',     'ztest-rpf-ola', 'P', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000017a', 'a0000000-0000-4000-8000-000000000171', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000017b', 'a0000000-0000-4000-8000-000000000171', 'admin',       'personal', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000017a', 'a0000000-0000-4000-8000-000000000173', 'contributor', 'employer', 'github_oauth');
insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('97000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000017a', 'github.com/ztest-rpf/api', 'api', 'github'),
  ('97000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-00000000017a', 'github.com/ztest-rpf/web', 'web', 'github'),
  ('97000000-0000-4000-8000-000000000003', 'b0000000-0000-4000-8000-00000000017b', 'github.com/ztest-ola/mine', 'mine', 'github');

set local role authenticated;

do $$
declare
    p1 uuid;
    p2 uuid;
    n  int;
    pr record;
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000171"}', true);

    -- Filing creates ola's workspace in T and links the repository under it.
    p1 := dojo.file_repository_workspace('97000000-0000-4000-8000-000000000001', 'ztest-rpf-sensei', 'sensei');
    if p1 is null then raise exception 'filing returned no workspace'; end if;
    reset role;
    select tenant_id, slug, name, classification::text as c into pr from dojo.workspaces where id = p1;
    if pr.tenant_id <> 'b0000000-0000-4000-8000-00000000017a'
       or pr.slug <> 'ztest-rpf-sensei' or pr.c <> 'company' then
        raise exception 'the workspace row is wrong: %', pr;
    end if;
    select count(*) into n from dojo.repositories_in_workspaces
     where workspace_id = p1 and repository_id = '97000000-0000-4000-8000-000000000001'
       and tenant_id = 'b0000000-0000-4000-8000-00000000017a';
    if n <> 1 then raise exception 'the repository was not linked under its workspace (% links)', n; end if;
    set local role authenticated;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000171"}', true);

    -- A second pass converges: same workspace, one link. The second repository of
    -- the same sensei workspace joins the same row.
    if dojo.file_repository_workspace('97000000-0000-4000-8000-000000000001', 'ztest-rpf-sensei', 'sensei') <> p1 then
        raise exception 'a second filing made a second workspace';
    end if;
    if dojo.file_repository_workspace('97000000-0000-4000-8000-000000000002', 'ztest-rpf-sensei', 'sensei') <> p1 then
        raise exception 'two repositories of one sensei workspace landed in two workspaces';
    end if;
    reset role;
    select count(*) into n from dojo.repositories_in_workspaces where workspace_id = p1;
    if n <> 2 then raise exception 'expected 2 links under the workspace, got %', n; end if;
    select count(*) into n from dojo.workspaces where tenant_id = 'b0000000-0000-4000-8000-00000000017a' and slug = 'ztest-rpf-sensei';
    if n <> 1 then raise exception 'expected one (tenant, slug) workspace, got %', n; end if;
    set local role authenticated;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000171"}', true);

    -- A repository moved to another sensei workspace moves: one workspace per
    -- repository per tenant, never two.
    p2 := dojo.file_repository_workspace('97000000-0000-4000-8000-000000000002', 'ztest-rpf-other', 'other');
    reset role;
    select count(*) into n from dojo.repositories_in_workspaces where repository_id = '97000000-0000-4000-8000-000000000002';
    if n <> 1 then raise exception 'a moved repository has % links', n; end if;
    if not exists (select 1 from dojo.repositories_in_workspaces where repository_id = '97000000-0000-4000-8000-000000000002' and workspace_id = p2) then
        raise exception 'the moved repository is not under its new workspace';
    end if;
    set local role authenticated;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000171"}', true);

    -- A repository in the caller's personal dōjō files a `personal` workspace IN
    -- that dōjō — personal workspaces live there, never under an org (#299).
    p2 := dojo.file_repository_workspace('97000000-0000-4000-8000-000000000003', 'ztest-rpf-mine', 'mine');
    reset role;
    select tenant_id, classification::text as c into pr from dojo.workspaces where id = p2;
    if pr.tenant_id is distinct from 'b0000000-0000-4000-8000-00000000017b' or pr.c <> 'personal' then
        raise exception 'a personal-dōjō repository should file a personal workspace in that dōjō, got %', pr;
    end if;
    set local role authenticated;

    -- ANOTHER member filing the same workspace converges on the one shared row.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000173"}', true);
    if dojo.file_repository_workspace('97000000-0000-4000-8000-000000000001', 'ztest-rpf-sensei', 'sensei') <> p1 then
        raise exception 'a second member filing the same workspace made a second workspace';
    end if;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000171"}', true);

    -- Someone who is not a member of the repository's tenant files nothing.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000172"}', true);
    begin
        perform dojo.file_repository_workspace('97000000-0000-4000-8000-000000000001', 'ztest-rpf-eve', 'eve');
        raise exception 'a non-member filed a workspace for another tenant''s repository';
    exception when insufficient_privilege then null;
    end;

    -- The link table stays closed to direct writes: filing is the only way in.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000171"}', true);
    begin
        insert into dojo.repositories_in_workspaces (workspace_id, repository_id, tenant_id)
        values (p1, '97000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000017a');
        raise exception 'repositories_in_workspaces accepted a direct write';
    exception when insufficient_privilege or others then
        if sqlerrm like 'repositories_in_workspaces accepted%' then raise; end if;
    end;
end $$;

rollback;
