set search_path to dojo, extensions;

-- How many of a tenant's relay sessions have heartbeated since a moment — for
-- the admin's health rollup. Relay sessions are their owner's content (access
-- matrix "Relay"), so an admin gets this COUNT and never the rows.
--
-- Anyone but an admin of the tenant is refused with insufficient_privilege, not
-- answered 0: a 0 would read as "no live sessions". A disabled membership's
-- sessions do not count.
create or replace function dojo.tenant_live_session_count(p_tenant uuid, p_since timestamptz)
    returns integer
    language plpgsql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    if not dojo.tenant_role_at_least(p_tenant, 'admin') then
        raise exception 'only an admin of the tenant counts its sessions'
            using errcode = 'insufficient_privilege';
    end if;
    return (
        select count(*)::integer
          from dojo.relay_sessions s
          join dojo.memberships m on m.id = s.membership_id
         where m.tenant_id = p_tenant
           and m.disabled_at is null
           and s.heartbeat_at >= p_since
    );
end;
$$;

revoke all on function dojo.tenant_live_session_count(uuid, timestamptz) from public;
grant execute on function dojo.tenant_live_session_count(uuid, timestamptz) to authenticated;

comment on function dojo.tenant_live_session_count(uuid, timestamptz) is
'Count of the tenant''s relay sessions with a heartbeat since p_since, for its
admin''s health rollup — the matrix''s "count only". Refuses (insufficient_privilege)
anyone but an admin of the tenant rather than answering 0 (#236).';
