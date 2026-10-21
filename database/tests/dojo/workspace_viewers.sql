-- The workspace viewer (#307 D8, revised 2026-10-21): the only permission a workspace
-- has. A read-only managerial view — metrics, dashboards, reports for the workspace.
-- A person must be IN the dōjō first — a member, or invited as a dōjō viewer (a
-- manager or client with no repository access) — and then the dōjō's admins grant
-- the view on selected workspaces. A dōjō viewer is not a member: no member surface,
-- no repository. Access to code and per-person data lives on repositories (D5).
--
-- WHAT BREAKS THIS TEST: a grant to someone in no dōjō; a non-admin granting or
-- revoking; a grant on one workspace reading as a grant on another; a direct write
-- to dojo.workspace_viewers or dojo.dojo_viewers; a viewer seeing other people's
-- grants; an admin of another dōjō granting here; a dōjō viewer counting as a
-- member or reading a repository; the dōjō's admin unable to read a viewer's name
-- (the Viewers list), or anyone else able to.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000311', 'a1000000-0000-4000-8000-000000000311', 'ada'),
  ('a0000000-0000-4000-8000-000000000312', 'a1000000-0000-4000-8000-000000000312', 'dev'),
  ('a0000000-0000-4000-8000-000000000313', 'a1000000-0000-4000-8000-000000000313', 'client'),
  ('a0000000-0000-4000-8000-000000000314', 'a1000000-0000-4000-8000-000000000314', 'otheradmin');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000031a', 'organization/ztest-pv-t', 'organization', 'ztest-pv-t', 'T', 'x'),
  ('b0000000-0000-4000-8000-00000000031b', 'organization/ztest-pv-u', 'organization', 'ztest-pv-u', 'U', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000031a', 'a0000000-0000-4000-8000-000000000311', 'admin', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000031a', 'a0000000-0000-4000-8000-000000000312', 'maintainer', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000031b', 'a0000000-0000-4000-8000-000000000314', 'admin', 'employer', 'github_oauth');
insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-000000000311', 'b0000000-0000-4000-8000-00000000031a', 'github.com/ztest-pv/r', 'r', 'github');
insert into dojo.workspaces (id, tenant_id, slug, name, classification) values
  ('c0000000-0000-4000-8000-000000000311', 'b0000000-0000-4000-8000-00000000031a', 'ztest-pv-p', 'p', 'company'),
  ('c0000000-0000-4000-8000-000000000312', 'b0000000-0000-4000-8000-00000000031a', 'ztest-pv-q', 'q', 'company');
insert into dojo.identities (principal_id, provider, subject, email, display_name) values
  ('a0000000-0000-4000-8000-000000000313', 'github_oauth', 'ztest-pv-client', 'client@example.com', 'Client Person');

set local role authenticated;

do $$
declare
    p constant uuid := 'c0000000-0000-4000-8000-000000000311';
    q constant uuid := 'c0000000-0000-4000-8000-000000000312';
    dev    constant uuid := 'a0000000-0000-4000-8000-000000000312';
    client constant uuid := 'a0000000-0000-4000-8000-000000000313';
    n int;
begin
    -- A maintainer is not an admin: cannot grant.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000312"}', true);
    begin
        perform dojo.grant_workspace_viewer(p, client);
        raise exception 'a non-admin granted workspace viewer';
    exception when insufficient_privilege then null;
    end;

    -- An admin of ANOTHER dōjō cannot grant on this one.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000314"}', true);
    begin
        perform dojo.grant_workspace_viewer(p, client);
        raise exception 'an admin of another dōjō granted workspace viewer';
    exception when insufficient_privilege then null;
    end;

    -- Someone in no dōjō cannot be granted: they are invited as a dōjō viewer first.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000311"}', true);
    begin
        perform dojo.grant_workspace_viewer(p, client);
        raise exception 'someone in no dōjō was granted a workspace view';
    exception when foreign_key_violation then null;
    end;

    -- The client accepts a viewer invite: the accept path (service_role) records
    -- them as a dōjō viewer — modelled here by the same function it calls.
    reset role;
    perform dojo.add_dojo_viewer('b0000000-0000-4000-8000-00000000031a', client, 'a0000000-0000-4000-8000-000000000311', 'Client@Example.com');
    set local role authenticated;

    -- The viewer record keeps the email they were invited at (and proved by
    -- signing in) — a viewer who signs in by email link has no identity to name them.
    select count(*) into n from dojo.dojo_viewers where principal_id = client and email = 'client@example.com';
    if n <> 1 then raise exception 'the dōjō viewer did not keep the invited email'; end if;

    -- The dōjō's admin reads the viewer's name (the Viewers list); a maintainer
    -- and another dōjō's admin do not.
    select count(*) into n from dojo.identities where principal_id = client;
    if n <> 1 then raise exception 'the admin read % of the viewer''s identities, expected 1', n; end if;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000312"}', true);
    select count(*) into n from dojo.identities where principal_id = client;
    if n <> 0 then raise exception 'a maintainer read a viewer''s identity'; end if;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000314"}', true);
    select count(*) into n from dojo.identities where principal_id = client;
    if n <> 0 then raise exception 'another dōjō''s admin read a viewer''s identity'; end if;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000311"}', true);

    -- The admin grants the client (now a dōjō viewer) and a member.
    perform dojo.grant_workspace_viewer(p, client);
    perform dojo.grant_workspace_viewer(p, dev);
    perform dojo.grant_workspace_viewer(p, client); -- idempotent

    -- The admin sees every grant on the dōjō's workspaces.
    select count(*) into n from dojo.workspace_viewers;
    if n <> 2 then raise exception 'the admin saw % grants, expected 2', n; end if;

    -- The client views p — and only p — and is not a member of the dōjō.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000313"}', true);
    if not dojo.is_workspace_viewer(p) then raise exception 'the granted client is not a viewer of p'; end if;
    if dojo.is_workspace_viewer(q) then raise exception 'a grant on p read as a grant on q'; end if;
    if dojo.is_member('b0000000-0000-4000-8000-00000000031a') then raise exception 'a dōjō viewer counted as a member'; end if;
    if not dojo.is_dojo_viewer('b0000000-0000-4000-8000-00000000031a') then raise exception 'the dōjō viewer is not one'; end if;
    if dojo.can_read_repository('90000000-0000-4000-8000-000000000311', 'b0000000-0000-4000-8000-00000000031a') then
        raise exception 'a dōjō viewer read a repository';
    end if;
    begin
        insert into dojo.dojo_viewers (tenant_id, principal_id) values ('b0000000-0000-4000-8000-00000000031b', client);
        raise exception 'a direct insert into dojo_viewers was accepted';
    exception when insufficient_privilege then null;
    end;

    -- A viewer sees their own grant, not anyone else's.
    select count(*) into n from dojo.workspace_viewers;
    if n <> 1 then raise exception 'a viewer saw % grants, expected only their own', n; end if;

    -- No direct write, for anyone signed in.
    begin
        insert into dojo.workspace_viewers (workspace_id, principal_id) values (q, client);
        raise exception 'a direct insert into workspace_viewers was accepted';
    exception when insufficient_privilege then null;
    end;

    -- A non-admin cannot revoke; the admin can.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000312"}', true);
    begin
        perform dojo.revoke_workspace_viewer(p, client);
        raise exception 'a non-admin revoked workspace viewer';
    exception when insufficient_privilege then null;
    end;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000311"}', true);
    perform dojo.revoke_workspace_viewer(p, client);
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000313"}', true);
    if dojo.is_workspace_viewer(p) then raise exception 'a revoked viewer is still a viewer'; end if;

    -- Removing a dōjō viewer takes their workspace views with them.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000311"}', true);
    perform dojo.grant_workspace_viewer(p, client);
    perform dojo.remove_dojo_viewer('b0000000-0000-4000-8000-00000000031a', client);
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000313"}', true);
    if dojo.is_workspace_viewer(p) then raise exception 'a removed dōjō viewer kept a workspace view'; end if;
    if dojo.is_dojo_viewer('b0000000-0000-4000-8000-00000000031a') then raise exception 'a removed dōjō viewer is still one'; end if;
end $$;

rollback;
