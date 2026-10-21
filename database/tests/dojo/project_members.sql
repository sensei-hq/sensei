-- Project-scoped roles (#307 A1). A person's role ON A PROJECT can raise what
-- they may do there — "Project roles only change what someone can do here" — and
-- never lower it: the effective role is the higher of the dōjō role and the
-- project role. Admin is dōjō-wide, never a project role.
--
-- WHAT BREAKS THIS TEST: a project role that also applies to another project;
-- a project role that lowers the dōjō role; a contributor setting roles; adding
-- someone who is not in the project's dōjō; a project-scoped admin; a direct
-- write to dojo.project_members; a non-member reading who is on a project.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000301', 'a1000000-0000-4000-8000-000000000301', 'maya'),
  ('a0000000-0000-4000-8000-000000000302', 'a1000000-0000-4000-8000-000000000302', 'dev'),
  ('a0000000-0000-4000-8000-000000000303', 'a1000000-0000-4000-8000-000000000303', 'cora'),
  ('a0000000-0000-4000-8000-000000000304', 'a1000000-0000-4000-8000-000000000304', 'out');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000030a', 'organization/ztest-pm-t', 'organization', 'ztest-pm-t', 'T', 'x'),
  ('b0000000-0000-4000-8000-00000000030b', 'organization/ztest-pm-u', 'organization', 'ztest-pm-u', 'U', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000030a', 'a0000000-0000-4000-8000-000000000301', 'maintainer', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000030a', 'a0000000-0000-4000-8000-000000000302', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000030a', 'a0000000-0000-4000-8000-000000000303', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000030b', 'a0000000-0000-4000-8000-000000000304', 'admin', 'employer', 'github_oauth');
insert into dojo.projects (id, tenant_id, slug, name, classification) values
  ('c0000000-0000-4000-8000-000000000301', 'b0000000-0000-4000-8000-00000000030a', 'ztest-pm-p', 'p', 'company'),
  ('c0000000-0000-4000-8000-000000000302', 'b0000000-0000-4000-8000-00000000030a', 'ztest-pm-q', 'q', 'company');

set local role authenticated;

do $$
declare
    p constant uuid := 'c0000000-0000-4000-8000-000000000301';
    q constant uuid := 'c0000000-0000-4000-8000-000000000302';
    maya constant uuid := 'a0000000-0000-4000-8000-000000000301';
    dev  constant uuid := 'a0000000-0000-4000-8000-000000000302';
    cora constant uuid := 'a0000000-0000-4000-8000-000000000303';
    outsider constant uuid := 'a0000000-0000-4000-8000-000000000304';
    n int;
begin
    -- dev is a contributor in the dōjō: not a maintainer on p.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000302"}', true);
    if dojo.project_role_at_least(p, 'maintainer') then
        raise exception 'a contributor reads as a maintainer on p before any project role';
    end if;

    -- A contributor cannot set project roles.
    begin
        perform dojo.set_project_member(p, cora, 'maintainer');
        raise exception 'a contributor set a project role';
    exception when insufficient_privilege then null;
    end;

    -- maya (dōjō maintainer) raises dev to maintainer on p.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000301"}', true);
    perform dojo.set_project_member(p, dev, 'maintainer');

    -- Admin is dōjō-wide — never a project role.
    begin
        perform dojo.set_project_member(p, cora, 'admin');
        raise exception 'a project-scoped admin was accepted';
    exception when invalid_parameter_value then null;
    end;

    -- Only people in the project's dōjō can be added.
    begin
        perform dojo.set_project_member(p, outsider, 'contributor');
        raise exception 'someone outside the dōjō was added to a project';
    exception when foreign_key_violation then null;
    end;

    -- A project role never lowers: maya stays a maintainer on p.
    perform dojo.set_project_member(p, maya, 'contributor');
    if not dojo.project_role_at_least(p, 'maintainer') then
        raise exception 'a lower project role lowered a dōjō maintainer';
    end if;

    -- dev is a maintainer on p — and only on p.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000302"}', true);
    if not dojo.project_role_at_least(p, 'maintainer') then
        raise exception 'the raised project role did not apply on p';
    end if;
    if dojo.project_role_at_least(q, 'maintainer') then
        raise exception 'a project role on p applied on q';
    end if;
    if dojo.project_role_at_least(p, 'lead') then
        raise exception 'a maintainer project role read as lead';
    end if;

    -- dev, now a maintainer on p, can manage p's members — but not q's.
    perform dojo.set_project_member(p, cora, 'contributor');
    begin
        perform dojo.set_project_member(q, cora, 'maintainer');
        raise exception 'a project maintainer on p set a role on q';
    exception when insufficient_privilege then null;
    end;

    -- No direct write, for anyone signed in.
    begin
        insert into dojo.project_members (project_id, principal_id, role) values (q, dev, 'lead');
        raise exception 'a direct insert into project_members was accepted';
    exception when insufficient_privilege then null;
    end;

    -- The dōjō's members see who is on its projects; an outsider sees nothing.
    select count(*) into n from dojo.project_members where project_id = p;
    if n <> 3 then raise exception 'a member saw % project members on p, expected 3', n; end if;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000304"}', true);
    select count(*) into n from dojo.project_members where project_id = p;
    if n <> 0 then raise exception 'an outsider saw % project members', n; end if;

    -- Removing the raise returns dev to the dōjō role.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000301"}', true);
    perform dojo.remove_project_member(p, dev);
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000302"}', true);
    if dojo.project_role_at_least(p, 'maintainer') then
        raise exception 'removing the project role left dev a maintainer on p';
    end if;
end $$;

rollback;
