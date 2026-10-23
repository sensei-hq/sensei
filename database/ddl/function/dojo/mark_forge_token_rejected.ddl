set search_path to dojo, extensions;

-- The forge rejected a connection's stored token (invalid_grant: expired,
-- revoked, consent withdrawn): mark the connection needs_new_token so reads stop
-- until an admin reconnects, and audit forge_token_rejected once per rejection
-- (no actor — the forge said so). Never deletes the secret: the admin decides.
--
-- COMPARE-AND-SET (#308 R5b review 2): only when the stored token is still
-- p_rejected — the one the forge refused. A sync that refreshed token A while an
-- admin reconnected (stored B) must not mark B dead. True when it marked, false
-- otherwise. service_role only.
drop function if exists dojo.mark_forge_token_rejected(dojo.forge_provider, text);
create or replace function dojo.mark_forge_token_rejected(
    p_provider    dojo.forge_provider,
    p_external_id text,
    p_rejected    text
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
    select c.* into v_conn
      from dojo.tenant_connections c
     where c.provider = p_provider and c.external_id = p_external_id
       for update;
    if not found or v_conn.token_secret_id is null or v_conn.token_state is not distinct from 'needs_new_token' then
        return false;
    end if;
    select d.decrypted_secret into v_current from vault.decrypted_secrets d where d.id = v_conn.token_secret_id;
    if v_current is distinct from p_rejected then
        return false;
    end if;

    update dojo.tenant_connections c
       set token_state = 'needs_new_token', updated_at = now()
     where c.id = v_conn.id;
    perform dojo.append_audit_event(v_conn.tenant_id, null, 'forge_token_rejected',
        jsonb_build_object('provider', p_provider, 'external_id', p_external_id,
                           'external_slug', v_conn.external_slug));
    return true;
end;
$$;

revoke all on function dojo.mark_forge_token_rejected(dojo.forge_provider, text, text) from public, anon, authenticated;
grant execute on function dojo.mark_forge_token_rejected(dojo.forge_provider, text, text) to service_role;

comment on function dojo.mark_forge_token_rejected(dojo.forge_provider, text, text) is
'Mark a connection''s stored forge token needs_new_token after the forge rejected it — only while the stored token is still the one rejected (compare-and-set) — and audit it once (#308 R5b). service_role only.';
