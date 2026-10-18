set search_path to dojo, extensions;

-- Which runs in a tenant the signed-in person may know are going, and their
-- status — never their content (#252; access matrix "Relay", run-status row).
-- The owner sees their runs; a member or lead of a team owning the run's project
-- sees it; a tenant admin sees every run in the tenant. Relay content (title,
-- goal, phase, gates, replies) stays the owner's (#249), so none of it is
-- returned. Pinned to the session's principal: there is no user argument.
create or replace function dojo.peer_run_status(p_tenant uuid)
    returns table (
        run_id       uuid,
        project_id   uuid,
        status       dojo.relay_run_status,
        started_at   timestamptz,
        heartbeat_at timestamptz,
        completed_at timestamptz
    )
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select s.run_id, s.project_id, s.status, s.started_at, s.heartbeat_at, s.completed_at
      from dojo.relay_sessions s
      join dojo.memberships m on m.id = s.membership_id
     where m.tenant_id = p_tenant
       and m.disabled_at is null
       and dojo.is_member(p_tenant)
       and (   m.user_id = dojo.current_principal_id()
            or (s.project_id is not null and dojo.on_team_of(s.project_id))
            or dojo.tenant_role_at_least(p_tenant, 'admin'))
     order by s.started_at desc;
$$;

revoke all on function dojo.peer_run_status(uuid) from public;
grant execute on function dojo.peer_run_status(uuid) to authenticated;

comment on function dojo.peer_run_status(uuid) is
'Status (never content) of the tenant''s runs the signed-in person may know are
going: their own, those on a project their team owns, and — for a tenant admin —
all of them (#252). Pinned to the session''s principal.';
