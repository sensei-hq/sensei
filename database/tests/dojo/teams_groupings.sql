-- Teams as groupings (#307 A3, D7): a named group of a dōjō's members, for "by
-- team" reports and for granting the workspace viewer to many people at once. A
-- team grants NO access — repository roles do (D5).
--
-- WHAT BREAKS THIS TEST: a non-admin creating, renaming, deleting or changing a
-- team; adding someone who is not in the dōjō; an outsider reading a dōjō's teams;
-- a team-wide viewer grant across dōjōs or by a non-admin; team membership
-- reading a repository; a direct write to the tables.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000341', 'a1000000-0000-4000-8000-000000000341', 'ada'),
  ('a0000000-0000-4000-8000-000000000342', 'a1000000-0000-4000-8000-000000000342', 'dev'),
  ('a0000000-0000-4000-8000-000000000343', 'a1000000-0000-4000-8000-000000000343', 'mae'),
  ('a0000000-0000-4000-8000-000000000344', 'a1000000-0000-4000-8000-000000000344', 'out');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000034a', 'organization/ztest-tg-t', 'organization', 'ztest-tg-t', 'T', 'x'),
  ('b0000000-0000-4000-8000-00000000034b', 'organization/ztest-tg-u', 'organization', 'ztest-tg-u', 'U', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000034a', 'a0000000-0000-4000-8000-000000000341', 'admin', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000034a', 'a0000000-0000-4000-8000-000000000342', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000034a', 'a0000000-0000-4000-8000-000000000343', 'maintainer', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000034b', 'a0000000-0000-4000-8000-000000000344', 'admin', 'employer', 'github_oauth'),
  -- ada also administers U, so only the same-dōjō rule can stop a T team being granted on U's workspace.
  ('b0000000-0000-4000-8000-00000000034b', 'a0000000-0000-4000-8000-000000000341', 'admin', 'employer', 'github_oauth');
insert into dojo.workspaces (id, tenant_id, slug, name, classification) values
  ('c0000000-0000-4000-8000-000000000341', 'b0000000-0000-4000-8000-00000000034a', 'ztest-tg-p', 'p', 'company'),
  ('c0000000-0000-4000-8000-000000000342', 'b0000000-0000-4000-8000-00000000034b', 'ztest-tg-q', 'q', 'company');
insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-000000000341', 'b0000000-0000-4000-8000-00000000034a', 'github.com/ztest-tg/r', 'r', 'github');
insert into dojo.repositories_in_workspaces (workspace_id, repository_id, tenant_id) values
  ('c0000000-0000-4000-8000-000000000341', '90000000-0000-4000-8000-000000000341', 'b0000000-0000-4000-8000-00000000034a');

set local role authenticated;

do $$
declare
    t constant uuid := 'b0000000-0000-4000-8000-00000000034a';
    p constant uuid := 'c0000000-0000-4000-8000-000000000341';
    q constant uuid := 'c0000000-0000-4000-8000-000000000342';
    dev constant uuid := 'a0000000-0000-4000-8000-000000000342';
    mae constant uuid := 'a0000000-0000-4000-8000-000000000343';
    outsider constant uuid := 'a0000000-0000-4000-8000-000000000344';
    team uuid;
    n int;
begin
    -- A maintainer is not an admin: cannot create a team.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000343"}', true);
    begin
        perform dojo.create_team(t, 'Platform');
        raise exception 'a non-admin created a team';
    exception when insufficient_privilege then null;
    end;

    -- The admin creates a team and puts dev and mae on it.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000341"}', true);
    team := dojo.create_team(t, 'Platform');
    perform dojo.add_team_member(team, dev);
    perform dojo.add_team_member(team, mae);
    perform dojo.add_team_member(team, mae); -- idempotent
    perform dojo.rename_team(team, 'Platform & Infra');

    -- Only members of the dōjō can be on its teams.
    begin
        perform dojo.add_team_member(team, outsider);
        raise exception 'someone outside the dōjō was put on a team';
    exception when foreign_key_violation then null;
    end;

    -- A team carries no access: dev is on it and still cannot read the repository.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000342"}', true);
    if dojo.can_read_repository('90000000-0000-4000-8000-000000000341', t) then
        raise exception 'team membership read a repository';
    end if;

    -- A member reads the dōjō's teams and who is on them.
    select count(*) into n from dojo.teams where tenant_id = t and name = 'Platform & Infra';
    if n <> 1 then raise exception 'a member saw % of the dōjō''s team, expected 1', n; end if;
    select count(*) into n from dojo.team_members where team_id = team;
    if n <> 2 then raise exception 'a member saw % team members, expected 2', n; end if;

    -- A contributor cannot change it; nobody writes the tables directly.
    begin
        perform dojo.remove_team_member(team, mae);
        raise exception 'a non-admin changed a team';
    exception when insufficient_privilege then null;
    end;
    begin
        insert into dojo.team_members (team_id, principal_id) values (team, outsider);
        raise exception 'a direct insert into team_members was accepted';
    exception when insufficient_privilege then null;
    end;

    -- An outsider (admin of another dōjō) sees none of it and grants nothing here.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000344"}', true);
    select count(*) into n from dojo.teams where tenant_id = t;
    if n <> 0 then raise exception 'an outsider saw % of the dōjō''s teams', n; end if;
    begin
        perform dojo.grant_workspace_viewer_to_team(p, team);
        raise exception 'an admin of another dōjō granted viewers here';
    exception when insufficient_privilege then null;
    end;

    -- The admin grants the viewer on p to the whole team at once.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000341"}', true);
    perform dojo.grant_workspace_viewer_to_team(p, team);
    select count(*) into n from dojo.workspace_viewers where workspace_id = p;
    if n <> 2 then raise exception 'a team grant made % viewers, expected 2', n; end if;

    -- A team cannot grant on another dōjō's workspace.
    begin
        perform dojo.grant_workspace_viewer_to_team(q, team);
        raise exception 'a team was granted a viewer on another dōjō''s workspace';
    exception when insufficient_privilege then null;
    end;

    -- Removing a member and deleting the team.
    perform dojo.remove_team_member(team, mae);
    select count(*) into n from dojo.team_members where team_id = team;
    if n <> 1 then raise exception 'removing a member left % members, expected 1', n; end if;
    perform dojo.delete_team(team);
    select count(*) into n from dojo.teams where id = team;
    if n <> 0 then raise exception 'a deleted team is still there'; end if;
    -- A viewer grant made through a team outlives the team: it was granted to people.
    select count(*) into n from dojo.workspace_viewers where workspace_id = p;
    if n <> 2 then raise exception 'deleting the team revoked % viewer grants', 2 - n; end if;
end $$;

rollback;
