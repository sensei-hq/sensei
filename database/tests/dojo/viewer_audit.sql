-- A viewer sees outcomes, never people; and every change to who may view is
-- on the record (#313, T6, D6/D8).
--
-- 1. Every grant, revoke, team grant, viewer invite and viewer removal writes
--    one row to dojo.audit_events — the trail the admin console reads — stamped
--    with the acting admin and the dōjō. A call that changes nothing (a repeat
--    grant, a revoke of no grant) writes none.
-- 2. A workspace viewer, signed in, reads no row keyed to another person in any
--    dōjō table: every person-keyed column is checked, so a table added later
--    is covered the day it lands. A provenance column (granted_by, invited_by,
--    ...) on the viewer's OWN record — who granted or invited them — is theirs
--    to see; on any other row it counts.
--
-- WHAT BREAKS THIS TEST: a grant path that skips the audit; an audit row with
-- the wrong actor, tenant or action; a no-op that writes a row; a policy that
-- lets a viewer read another person's membership, identity, metric, role,
-- grant or audit row.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000391', 'a1000000-0000-4000-8000-000000000391', 'ada'),
  ('a0000000-0000-4000-8000-000000000392', 'a1000000-0000-4000-8000-000000000392', 'dev'),
  ('a0000000-0000-4000-8000-000000000393', 'a1000000-0000-4000-8000-000000000393', 'client');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000039a', 'organization/ztest-va', 'organization', 'ztest-va', 'VA', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000039a', 'a0000000-0000-4000-8000-000000000391', 'admin', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000039a', 'a0000000-0000-4000-8000-000000000392', 'maintainer', 'employer', 'github_oauth');
insert into dojo.workspaces (id, tenant_id, slug, name, classification) values
  ('c0000000-0000-4000-8000-000000000391', 'b0000000-0000-4000-8000-00000000039a', 'ztest-va-p', 'p', 'company');
insert into dojo.teams (id, tenant_id, name) values
  ('d0000000-0000-4000-8000-000000000391', 'b0000000-0000-4000-8000-00000000039a', 'ztest-va-team');
insert into dojo.team_members (team_id, principal_id) values
  ('d0000000-0000-4000-8000-000000000391', 'a0000000-0000-4000-8000-000000000392');

-- The accept path (service_role) records the client as a dōjō viewer.
select dojo.add_dojo_viewer('b0000000-0000-4000-8000-00000000039a', 'a0000000-0000-4000-8000-000000000393',
                            'a0000000-0000-4000-8000-000000000391', 'client@example.com');
select dojo.add_dojo_viewer('b0000000-0000-4000-8000-00000000039a', 'a0000000-0000-4000-8000-000000000393',
                            'a0000000-0000-4000-8000-000000000391', 'client@example.com'); -- no-op

set local role authenticated;
select set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000391"}', true);
select dojo.grant_workspace_viewer('c0000000-0000-4000-8000-000000000391', 'a0000000-0000-4000-8000-000000000393');
select dojo.grant_workspace_viewer('c0000000-0000-4000-8000-000000000391', 'a0000000-0000-4000-8000-000000000393'); -- no-op
select dojo.grant_workspace_viewer_to_team('c0000000-0000-4000-8000-000000000391', 'd0000000-0000-4000-8000-000000000391');
select dojo.revoke_workspace_viewer('c0000000-0000-4000-8000-000000000391', 'a0000000-0000-4000-8000-000000000392');
select dojo.revoke_workspace_viewer('c0000000-0000-4000-8000-000000000391', 'a0000000-0000-4000-8000-000000000392'); -- no-op

-- ── 2. the viewer reads no one else ─────────────────────────────────────────
select set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000393"}', true);
do $$
declare
    c record;
    n bigint;
    leaks text := '';
begin
    if not dojo.is_workspace_viewer('c0000000-0000-4000-8000-000000000391') then
        raise exception 'the client is not a viewer of the workspace — the test proves nothing';
    end if;
    for c in select col.table_name, col.column_name,
                    col.column_name ~ '(granted_by|invited_by|connected_by|created_by)' as provenance,
                    exists (select 1 from information_schema.columns s
                             where s.table_schema = 'dojo' and s.table_name = col.table_name
                               and s.column_name = 'principal_id') as has_subject
               from information_schema.columns col
               join information_schema.tables t using (table_schema, table_name)
              where col.table_schema = 'dojo' and t.table_type = 'BASE TABLE' and col.data_type = 'uuid'
                and col.column_name ~ '(user|principal|author|actor|member|person|granted_by|invited_by|connected_by|created_by)'
    loop
        begin
            execute format('select count(*) from dojo.%I where %I is not null and %I not in (%L, %L)%s',
                           c.table_name, c.column_name, c.column_name,
                           'a0000000-0000-4000-8000-000000000393', 'a1000000-0000-4000-8000-000000000393',
                           case when c.provenance and c.has_subject
                                then ' and principal_id is distinct from ''a0000000-0000-4000-8000-000000000393'''
                                else '' end)
               into n;
        exception when insufficient_privilege then
            n := 0; -- no grant at all: nothing readable
        end;
        if n > 0 then
            leaks := leaks || format(' %s.%s=%s', c.table_name, c.column_name, n);
        end if;
    end loop;
    if leaks <> '' then
        raise exception 'a workspace viewer read rows keyed to other people:%', leaks;
    end if;
end $$;
reset role;

-- ── 1. the trail ────────────────────────────────────────────────────────────
do $$
declare
    got text;
    want constant text :=
        'viewer_invited:a0000000-0000-4000-8000-000000000391,'
     || 'workspace_view_granted:a0000000-0000-4000-8000-000000000391,'
     || 'workspace_view_granted_to_team:a0000000-0000-4000-8000-000000000391,'
     || 'workspace_view_revoked:a0000000-0000-4000-8000-000000000391';
begin
    select string_agg(action || ':' || actor_id, ',' order by id) into got
      from dojo.audit_events where tenant_id = 'b0000000-0000-4000-8000-00000000039a';
    if got is distinct from want then
        raise exception 'the audit trail is [%], expected [%]', got, want;
    end if;
    if (select detail from dojo.audit_events
         where tenant_id = 'b0000000-0000-4000-8000-00000000039a' and action = 'workspace_view_granted_to_team')
       is distinct from jsonb_build_object('workspace_id', 'c0000000-0000-4000-8000-000000000391',
                                           'team_id', 'd0000000-0000-4000-8000-000000000391', 'granted', 1) then
        raise exception 'the team grant does not record its workspace, team and count';
    end if;
end $$;

-- Removing the viewer from the dōjō is on the record too.
set local role authenticated;
select set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000391"}', true);
select dojo.remove_dojo_viewer('b0000000-0000-4000-8000-00000000039a', 'a0000000-0000-4000-8000-000000000393');
select dojo.remove_dojo_viewer('b0000000-0000-4000-8000-00000000039a', 'a0000000-0000-4000-8000-000000000393'); -- no-op
reset role;
do $$
begin
    if (select count(*) from dojo.audit_events
         where tenant_id = 'b0000000-0000-4000-8000-00000000039a' and action = 'viewer_removed'
           and actor_id = 'a0000000-0000-4000-8000-000000000391'
           and detail = jsonb_build_object('principal_id', 'a0000000-0000-4000-8000-000000000393')) <> 1 then
        raise exception 'removing a dōjō viewer was not audited exactly once';
    end if;
end $$;

rollback;
