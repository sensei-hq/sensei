set search_path to dojo, extensions;

-- Store the token a refresh handed back in place of the one stored (#308 R5b,
-- D42). Microsoft Entra rotates refresh tokens: each refresh answers with a new
-- one, which must replace the old in Vault or the grant is lost once the old one
-- lapses. The same secret is updated in place (one secret per connection), the
-- state stays ok, and NO audit row is written — the grant did not change hands;
-- set / replace / remove (a person's acts) are audited. service_role only; the
-- server calls it right after a successful refresh.
create or replace function dojo.rotate_forge_token(
    p_provider    dojo.forge_provider,
    p_external_id text,
    p_token       text
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
    if btrim(coalesce(p_token, '')) = '' then
        raise exception 'a token is needed'
            using errcode = 'invalid_parameter_value';
    end if;

    select c.* into v_conn
      from dojo.tenant_connections c
     where c.provider = p_provider and c.external_id = p_external_id
       for update;
    if not found or v_conn.token_secret_id is null
       or not exists (select 1 from vault.secrets s where s.id = v_conn.token_secret_id) then
        raise exception 'no token is stored for % unit %', p_provider, p_external_id
            using errcode = 'no_data_found';
    end if;

    perform vault.update_secret(v_conn.token_secret_id, btrim(p_token));
    update dojo.tenant_connections c
       set token_state = 'ok', updated_at = now()
     where c.id = v_conn.id;
end;
$$;

revoke all on function dojo.rotate_forge_token(dojo.forge_provider, text, text) from public, anon, authenticated;
grant execute on function dojo.rotate_forge_token(dojo.forge_provider, text, text) to service_role;

comment on function dojo.rotate_forge_token(dojo.forge_provider, text, text) is
'Replace a connection''s stored forge token with the one a refresh returned, in place and unaudited (#308 R5b, D42: Entra rotates refresh tokens). service_role only.';
