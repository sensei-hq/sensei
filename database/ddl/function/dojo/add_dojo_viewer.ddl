set search_path to dojo, extensions;

-- Record someone as a viewer of a dōjō (#307 D8) — what accepting a viewer invite
-- does. service_role only (the accept route, after its email and expiry gates).
-- Idempotent.
create or replace function dojo.add_dojo_viewer(p_tenant uuid, p_principal uuid, p_invited_by uuid)
    returns void
    language sql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
    insert into dojo.dojo_viewers (tenant_id, principal_id, invited_by)
    values (p_tenant, p_principal, p_invited_by)
    on conflict (tenant_id, principal_id) do nothing;
$$;

revoke all on function dojo.add_dojo_viewer(uuid, uuid, uuid) from public;
grant execute on function dojo.add_dojo_viewer(uuid, uuid, uuid) to service_role;

comment on function dojo.add_dojo_viewer(uuid, uuid, uuid) is
'Record a dōjō viewer — the accept of a viewer invite (#307 D8). service_role only.';
