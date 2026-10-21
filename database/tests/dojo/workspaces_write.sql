-- Who writes a workspace (#299). A workspace is ONE SHARED item in a dōjō, not a
-- person's row, so nobody writes it directly:
--
--   * the SYNC files it — dojo.file_repository_workspace (see
--     repository_workspace_filing.sql), the only thing that CREATES a workspace;
--   * a RELAY run is a person's sessions IN a workspace — it attaches to the
--     existing workspace through dojo.attach_run_to_workspace, which may move only
--     last_run_at and the constitution, and never creates one.
--
-- WHAT BREAKS THIS TEST: a grant or policy that lets `authenticated` insert or
-- update dojo.workspaces directly; attach creating a workspace that does not exist;
-- attach for a tenant the caller is not a member of; attach touching anything
-- but last_run_at / constitution.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000161', 'a1000000-0000-4000-8000-000000000161', 'ola'),
  ('a0000000-0000-4000-8000-000000000162', 'a1000000-0000-4000-8000-000000000162', 'bob');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000016a', 'organization/ztest-pw-t', 'organization', 'ztest-pw-t', 'T', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000016a', 'a0000000-0000-4000-8000-000000000161', 'contributor', 'employer', 'github_oauth');
insert into dojo.workspaces (id, tenant_id, slug, name, classification) values
  ('c0000000-0000-4000-8000-000000000161', 'b0000000-0000-4000-8000-00000000016a', 'ztest-pw-sensei', 'sensei', 'company');

set local role authenticated;

do $$
declare
    got uuid;
    n   int;
    pr  record;
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000161"}', true);

    -- No direct write, for anyone signed in.
    begin
        insert into dojo.workspaces (tenant_id, slug, name)
        values ('b0000000-0000-4000-8000-00000000016a', 'ztest-pw-direct', 'direct');
        raise exception 'workspaces: a direct insert by a signed-in caller was accepted';
    exception when insufficient_privilege then null;
    end;
    begin
        update dojo.workspaces set name = 'renamed' where id = 'c0000000-0000-4000-8000-000000000161';
        get diagnostics n = row_count;
        if n > 0 then raise exception 'workspaces: a direct update by a signed-in caller changed % rows', n; end if;
    exception when insufficient_privilege then null;
    end;

    -- Attach finds the shared workspace and moves only last_run_at / constitution.
    got := dojo.attach_run_to_workspace('b0000000-0000-4000-8000-00000000016a', 'ztest-pw-sensei', '{"rules":[]}'::jsonb);
    if got is distinct from 'c0000000-0000-4000-8000-000000000161' then
        raise exception 'attach: expected the shared workspace, got %', got;
    end if;

    -- An unknown workspace is not created: a run is never where a workspace begins.
    got := dojo.attach_run_to_workspace('b0000000-0000-4000-8000-00000000016a', 'ztest-pw-nope', null);
    if got is not null then raise exception 'attach: an unknown workspace returned %', got; end if;

    -- A tenant the caller is not a member of is refused.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000162"}', true);
    begin
        perform dojo.attach_run_to_workspace('b0000000-0000-4000-8000-00000000016a', 'ztest-pw-sensei', null);
        raise exception 'attach: a non-member attached a run';
    exception when insufficient_privilege then null;
    end;

    reset role;
    select name, last_run_at, constitution into pr from dojo.workspaces where id = 'c0000000-0000-4000-8000-000000000161';
    if pr.name <> 'sensei' then raise exception 'attach or update renamed the workspace to %', pr.name; end if;
    if pr.last_run_at is null then raise exception 'attach did not record the run time'; end if;
    if pr.constitution is distinct from '{"rules":[]}'::jsonb then
        raise exception 'attach did not record the constitution: %', pr.constitution;
    end if;
    select count(*) into n from dojo.workspaces where slug = 'ztest-pw-nope';
    if n <> 0 then raise exception 'attach created a workspace for an unknown slug'; end if;
end $$;

rollback;
