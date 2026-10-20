-- `dojo.file_repository_project` — the sync files a repository under its sensei
-- project (#299, decided: one dōjō project per sensei project).
--
-- Before this, `dojo.projects` was written only by a relay run and
-- `dojo.repositories_in_projects` by nothing at all, so a synced repository
-- never appeared under any project.
--
-- WHAT BREAKS THIS TEST: filing for a repository in a tenant the caller is not a
-- member of; a second filing creating a second project or a second link instead
-- of converging; a project the caller owns being keyed on anything but
-- (user_id, slug) — the row a relay run writes; a moved repository keeping its
-- old link (two projects in one tenant); or the link table opening to direct
-- writes.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000171', 'a1000000-0000-4000-8000-000000000171', 'ola'),
  ('a0000000-0000-4000-8000-000000000172', 'a1000000-0000-4000-8000-000000000172', 'eve');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000017a', 'organization/ztest-rpf-t', 'organization', 'ztest-rpf-t', 'T', 'x'),
  ('b0000000-0000-4000-8000-00000000017b', 'personal/ztest-rpf-ola',   'personal',     'ztest-rpf-ola', 'P', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000017a', 'a0000000-0000-4000-8000-000000000171', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000017b', 'a0000000-0000-4000-8000-000000000171', 'admin',       'personal', 'github_oauth');
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

    -- Filing creates ola's project in T and links the repository under it.
    p1 := dojo.file_repository_project('97000000-0000-4000-8000-000000000001', 'ztest-rpf-sensei', 'sensei');
    if p1 is null then raise exception 'filing returned no project'; end if;
    reset role;
    select user_id, tenant_id, slug, name, classification::text as c into pr from dojo.projects where id = p1;
    if pr.user_id <> 'a0000000-0000-4000-8000-000000000171' or pr.tenant_id <> 'b0000000-0000-4000-8000-00000000017a'
       or pr.slug <> 'ztest-rpf-sensei' or pr.c <> 'company' then
        raise exception 'the project row is wrong: %', pr;
    end if;
    select count(*) into n from dojo.repositories_in_projects
     where project_id = p1 and repository_id = '97000000-0000-4000-8000-000000000001'
       and tenant_id = 'b0000000-0000-4000-8000-00000000017a';
    if n <> 1 then raise exception 'the repository was not linked under its project (% links)', n; end if;
    set local role authenticated;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000171"}', true);

    -- A second pass converges: same project, one link. The second repository of
    -- the same sensei project joins the same row.
    if dojo.file_repository_project('97000000-0000-4000-8000-000000000001', 'ztest-rpf-sensei', 'sensei') <> p1 then
        raise exception 'a second filing made a second project';
    end if;
    if dojo.file_repository_project('97000000-0000-4000-8000-000000000002', 'ztest-rpf-sensei', 'sensei') <> p1 then
        raise exception 'two repositories of one sensei project landed in two projects';
    end if;
    reset role;
    select count(*) into n from dojo.repositories_in_projects where project_id = p1;
    if n <> 2 then raise exception 'expected 2 links under the project, got %', n; end if;
    select count(*) into n from dojo.projects where user_id = 'a0000000-0000-4000-8000-000000000171' and slug = 'ztest-rpf-sensei';
    if n <> 1 then raise exception 'expected one (user, slug) project, got %', n; end if;
    set local role authenticated;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000171"}', true);

    -- A repository moved to another sensei project moves: one project per
    -- repository per tenant, never two.
    p2 := dojo.file_repository_project('97000000-0000-4000-8000-000000000002', 'ztest-rpf-other', 'other');
    reset role;
    select count(*) into n from dojo.repositories_in_projects where repository_id = '97000000-0000-4000-8000-000000000002';
    if n <> 1 then raise exception 'a moved repository has % links', n; end if;
    if not exists (select 1 from dojo.repositories_in_projects where repository_id = '97000000-0000-4000-8000-000000000002' and project_id = p2) then
        raise exception 'the moved repository is not under its new project';
    end if;
    set local role authenticated;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000171"}', true);

    -- The classification follows WHERE THE REPOSITORY LIVES (#299): an employer
    -- membership files `company` in that tenant (asserted above); a repository in
    -- the caller's personal dōjō files a tenant-less `personal` project, as a
    -- relay run files one.
    p2 := dojo.file_repository_project('97000000-0000-4000-8000-000000000003', 'ztest-rpf-mine', 'mine');
    reset role;
    select tenant_id, classification::text as c into pr from dojo.projects where id = p2;
    if pr.tenant_id is not null or pr.c <> 'personal' then
        raise exception 'a personal-dōjō repository should file a tenant-less personal project, got %', pr;
    end if;
    if not exists (select 1 from dojo.repositories_in_projects where project_id = p2
                    and repository_id = '97000000-0000-4000-8000-000000000003'
                    and tenant_id = 'b0000000-0000-4000-8000-00000000017b') then
        raise exception 'the personal repository was not linked under its tenant';
    end if;
    set local role authenticated;

    -- Someone who is not a member of the repository's tenant files nothing.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000172"}', true);
    begin
        perform dojo.file_repository_project('97000000-0000-4000-8000-000000000001', 'ztest-rpf-eve', 'eve');
        raise exception 'a non-member filed a project for another tenant''s repository';
    exception when insufficient_privilege then null;
    end;

    -- The link table stays closed to direct writes: filing is the only way in.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000171"}', true);
    begin
        insert into dojo.repositories_in_projects (project_id, repository_id, tenant_id)
        values (p1, '97000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000017a');
        raise exception 'repositories_in_projects accepted a direct write';
    exception when insufficient_privilege or others then
        if sqlerrm like 'repositories_in_projects accepted%' then raise; end if;
    end;
end $$;

rollback;
