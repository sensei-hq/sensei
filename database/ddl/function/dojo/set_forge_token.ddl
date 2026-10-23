set search_path to dojo, extensions;

-- Store the forge token an admin pasted for one connection (#308 R5b, D35: an
-- Azure DevOps PAT) in Supabase Vault, or replace the one stored.
--
-- The caller is the server (service_role). It has already validated the token
-- against the forge — it lists the organization and proves the admin — so a
-- failing token never reaches this. What this enforces:
--   * the connection exists, and the principal is an ACTIVE admin of its dōjō;
--   * the token is not blank;
--   * ONE secret per connection: the first write creates it, a replace updates it
--     in place (vault.update_secret), so no old value is left behind;
--   * state ok, expiry as given (NULL = unknown);
--   * an audit row — forge_token_set or forge_token_replaced, by whom — that
--     never carries the token.
-- Returns 'set' or 'replaced'.
create or replace function dojo.set_forge_token(
    p_principal   uuid,
    p_provider    dojo.forge_provider,
    p_external_id text,
    p_token       text,
    p_expires_at  timestamptz
)
    returns text
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_conn   dojo.tenant_connections%rowtype;
    v_secret uuid;
    v_action text;
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
        raise exception 'only an admin of the dōjō may set its token'
            using errcode = 'insufficient_privilege';
    end if;

    if btrim(coalesce(p_token, '')) = '' then
        raise exception 'a token is needed'
            using errcode = 'invalid_parameter_value';
    end if;

    if v_conn.token_secret_id is not null
       and exists (select 1 from vault.secrets s where s.id = v_conn.token_secret_id) then
        perform vault.update_secret(v_conn.token_secret_id, btrim(p_token));
        v_secret := v_conn.token_secret_id;
        v_action := 'replaced';
    else
        v_secret := vault.create_secret(
            btrim(p_token),
            'dojo/forge_token/' || v_conn.id,
            format('The %s token of dōjō connection %s', p_provider, v_conn.id));
        v_action := 'set';
    end if;

    update dojo.tenant_connections c
       set token_secret_id = v_secret, token_state = 'ok', token_expires_at = p_expires_at, updated_at = now()
     where c.id = v_conn.id;

    perform dojo.append_audit_event(v_conn.tenant_id, p_principal, 'forge_token_' || v_action,
        jsonb_build_object('provider', p_provider, 'external_id', p_external_id,
                           'external_slug', v_conn.external_slug, 'expires_at', p_expires_at));
    return v_action;
end;
$$;

revoke all on function dojo.set_forge_token(uuid, dojo.forge_provider, text, text, timestamptz) from public, anon, authenticated;
grant execute on function dojo.set_forge_token(uuid, dojo.forge_provider, text, text, timestamptz) to service_role;

comment on function dojo.set_forge_token(uuid, dojo.forge_provider, text, text, timestamptz) is
'Store or replace a connection''s forge token in Supabase Vault (#308 R5b, D35):
one secret per connection, an active admin of the dōjō only, audited without the
token. service_role only — the server validates the token against the forge first.';
