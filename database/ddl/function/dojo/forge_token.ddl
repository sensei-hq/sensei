set search_path to dojo, extensions;

-- Read a connection's stored forge token, decrypted from Supabase Vault, for the
-- server's own reads of the forge (#308 R5b, D35). service_role only; the server
-- uses it for one request and never returns, logs or echoes it.
--
-- Fails rather than answering NULL: no connection or no token is no_data_found;
-- a token the forge rejected (needs_new_token) is
-- object_not_in_prerequisite_state — member reads stop until an admin replaces
-- it, and a read that could not be made never ends a membership (#309).
create or replace function dojo.forge_token(
    p_provider    dojo.forge_provider,
    p_external_id text
)
    returns text
    language plpgsql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_conn  dojo.tenant_connections%rowtype;
    v_token text;
begin
    select c.* into v_conn
      from dojo.tenant_connections c
     where c.provider = p_provider and c.external_id = p_external_id;
    if not found or v_conn.token_secret_id is null then
        raise exception 'no token is stored for % unit %', p_provider, p_external_id
            using errcode = 'no_data_found';
    end if;
    if v_conn.token_state = 'needs_new_token' then
        raise exception 'the stored token for % unit % was rejected; an admin must replace it', p_provider, p_external_id
            using errcode = 'object_not_in_prerequisite_state';
    end if;

    select d.decrypted_secret into v_token from vault.decrypted_secrets d where d.id = v_conn.token_secret_id;
    if v_token is null then
        raise exception 'the stored token for % unit % is missing from Vault', p_provider, p_external_id
            using errcode = 'no_data_found';
    end if;
    return v_token;
end;
$$;

revoke all on function dojo.forge_token(dojo.forge_provider, text) from public, anon, authenticated;
grant execute on function dojo.forge_token(dojo.forge_provider, text) to service_role;

comment on function dojo.forge_token(dojo.forge_provider, text) is
'A connection''s forge token, decrypted from Supabase Vault (#308 R5b). Raises
when there is none, or when it was rejected (needs_new_token). service_role only.';
