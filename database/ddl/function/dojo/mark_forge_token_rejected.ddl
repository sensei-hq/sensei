set search_path to dojo, extensions;

-- The forge rejected a connection's stored token (401: expired or revoked):
-- mark the connection needs_new_token so reads stop until an admin replaces it,
-- and audit forge_token_rejected once per rejection (no actor — the forge said
-- so). Never deletes the secret: the admin decides. service_role only.
create or replace function dojo.mark_forge_token_rejected(
    p_provider    dojo.forge_provider,
    p_external_id text
)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_conn dojo.tenant_connections%rowtype;
begin
    update dojo.tenant_connections c
       set token_state = 'needs_new_token', updated_at = now()
     where c.provider = p_provider and c.external_id = p_external_id
       and c.token_secret_id is not null and c.token_state is distinct from 'needs_new_token'
    returning c.* into v_conn;
    if found then
        perform dojo.append_audit_event(v_conn.tenant_id, null, 'forge_token_rejected',
            jsonb_build_object('provider', p_provider, 'external_id', p_external_id,
                               'external_slug', v_conn.external_slug));
    end if;
end;
$$;

revoke all on function dojo.mark_forge_token_rejected(dojo.forge_provider, text) from public, anon, authenticated;
grant execute on function dojo.mark_forge_token_rejected(dojo.forge_provider, text) to service_role;

comment on function dojo.mark_forge_token_rejected(dojo.forge_provider, text) is
'Mark a connection''s stored forge token needs_new_token after the forge rejected
it, and audit it once (#308 R5b). service_role only.';
