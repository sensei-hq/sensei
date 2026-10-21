set search_path to dojo, extensions;

-- Record someone as a viewer of a dōjō (#307 D8) — what accepting a viewer invite
-- does. service_role only (the accept route, after its email and expiry gates).
-- Idempotent. Keeps the invited email (proven at sign-in) to name the viewer.
create or replace function dojo.add_dojo_viewer(p_tenant uuid, p_principal uuid, p_invited_by uuid, p_email text)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    insert into dojo.dojo_viewers (tenant_id, principal_id, invited_by, email)
    values (p_tenant, p_principal, p_invited_by, lower(p_email))
    on conflict (tenant_id, principal_id) do nothing;
    if found then
        perform dojo.append_audit_event(p_tenant, p_invited_by, 'viewer_invited',
                                        jsonb_build_object('principal_id', p_principal));
    end if;
end;
$$;

revoke all on function dojo.add_dojo_viewer(uuid, uuid, uuid, text) from public;
grant execute on function dojo.add_dojo_viewer(uuid, uuid, uuid, text) to service_role;

comment on function dojo.add_dojo_viewer(uuid, uuid, uuid, text) is
'Record a dōjō viewer — the accept of a viewer invite (#307 D8). service_role only.';
