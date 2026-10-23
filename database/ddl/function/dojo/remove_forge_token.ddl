set search_path to dojo, extensions;

-- Remove a connection's stored forge token (#308 R5b, D35): delete its Vault
-- secret, clear the connection's token columns and audit forge_token_removed (by
-- whom; never the token). An active admin of the dōjō only. True when a token was
-- removed, false when there was none.
create or replace function dojo.remove_forge_token(
    p_principal   uuid,
    p_provider    dojo.forge_provider,
    p_external_id text
)
    returns boolean
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_conn dojo.tenant_connections%rowtype;
begin
    select c.* into v_conn
      from dojo.tenant_connections c
     where c.provider = p_provider and c.external_id = p_external_id
       for update;
    if not found then
        raise exception 'no dōjō is connected to % unit %', p_provider, p_external_id
            using errcode = 'no_data_found';
    end if;

    if not exists (
        select 1 from dojo.memberships m
         where m.tenant_id = v_conn.tenant_id and m.user_id = p_principal
           and m.disabled_at is null and m.role = 'admin'
    ) then
        raise exception 'only an admin of the dōjō may remove its token'
            using errcode = 'insufficient_privilege';
    end if;

    if v_conn.token_secret_id is null then
        return false;
    end if;

    delete from vault.secrets s where s.id = v_conn.token_secret_id;
    update dojo.tenant_connections c
       set token_secret_id = null, token_state = null, token_expires_at = null, updated_at = now()
     where c.id = v_conn.id;

    perform dojo.append_audit_event(v_conn.tenant_id, p_principal, 'forge_token_removed',
        jsonb_build_object('provider', p_provider, 'external_id', p_external_id,
                           'external_slug', v_conn.external_slug));
    return true;
end;
$$;

revoke all on function dojo.remove_forge_token(uuid, dojo.forge_provider, text) from public, anon, authenticated;
grant execute on function dojo.remove_forge_token(uuid, dojo.forge_provider, text) to service_role;

comment on function dojo.remove_forge_token(uuid, dojo.forge_provider, text) is
'Delete a connection''s forge token from Supabase Vault and audit it (#308 R5b).
An active admin of the dōjō only. service_role only.';
