set search_path to dojo, extensions;

-- Store the token a refresh handed back in place of the one stored (#308 R5b,
-- D42). Microsoft Entra rotates refresh tokens: each refresh answers with a new
-- one, which must replace the old in Vault or the grant is lost once the old one
-- lapses. The same secret is updated in place (one secret per connection) and
-- NO audit row is written — the grant did not change hands; set / replace /
-- remove (a person's acts) are audited.
--
-- COMPARE-AND-SET (#308 R5b review 2): only when the stored token is still
-- p_expected — the one the caller refreshed. A sync that read token A while an
-- admin reconnected (stored B) must not overwrite B with A's rotation. True when
-- it rotated, false when the stored token had moved on. The state is never
-- touched: a rotate cannot un-reject a grant marked needs_new_token.
-- service_role only; the server calls it right after a successful refresh.
drop function if exists dojo.rotate_forge_token(dojo.forge_provider, text, text);
create or replace function dojo.rotate_forge_token(
    p_provider    dojo.forge_provider,
    p_external_id text,
    p_expected    text,
    p_token       text
)
    returns boolean
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_conn    dojo.tenant_connections%rowtype;
    v_current text;
begin
    if btrim(coalesce(p_token, '')) = '' then
        raise exception 'a token is needed'
            using errcode = 'invalid_parameter_value';
    end if;

    select c.* into v_conn
      from dojo.tenant_connections c
     where c.provider = p_provider and c.external_id = p_external_id
       for update;
    if not found or v_conn.token_secret_id is null then
        raise exception 'no token is stored for % unit %', p_provider, p_external_id
            using errcode = 'no_data_found';
    end if;
    select d.decrypted_secret into v_current from vault.decrypted_secrets d where d.id = v_conn.token_secret_id;
    if v_current is null then
        raise exception 'no token is stored for % unit %', p_provider, p_external_id
            using errcode = 'no_data_found';
    end if;

    if v_current is distinct from p_expected then
        return false;
    end if;
    perform vault.update_secret(v_conn.token_secret_id, btrim(p_token));
    update dojo.tenant_connections c set updated_at = now() where c.id = v_conn.id;
    return true;
end;
$$;

revoke all on function dojo.rotate_forge_token(dojo.forge_provider, text, text, text) from public, anon, authenticated;
grant execute on function dojo.rotate_forge_token(dojo.forge_provider, text, text, text) to service_role;

comment on function dojo.rotate_forge_token(dojo.forge_provider, text, text, text) is
'Replace a connection''s stored forge token with the one a refresh returned — only while the stored one is still the token refreshed (compare-and-set) — in place, unaudited, the state untouched (#308 R5b, D42). service_role only.';
