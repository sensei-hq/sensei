set search_path to dojo, extensions;

-- Claim an organization's dōjō (#308 R5a, D20; §II.4).
--
-- An org dōjō is created by whoever signs in first, who may be a plain member, so
-- its existence proves nothing about ownership. The caller — the server, as
-- service_role — has just read the forge with the claimer's OWN token and found
-- them an admin of the unit; this records what that proved:
--   * the connection is re-proved (verified_at = now()),
--   * the dōjō is claimed (claimed_at, claimed_by) if nobody has yet — FIRST
--     CLAIM WINS, so a later admin re-proves without rewriting who established
--     the claim,
--   * a dojo_claimed audit row, in the same transaction, only when this call
--     made the claim.
--
-- The claim is the TENANT's, not the connection's: it gates billing and private
-- sync for the whole dōjō (all_my_repositories reads tenants.claimed_at), and a
-- dōjō may be known to several forges.
create or replace function dojo.claim_forge_unit(
    p_principal   uuid,
    p_provider    dojo.forge_provider,
    p_external_id text
)
    returns table (tenant_id uuid, claimed_at timestamptz, claimed_by uuid, claimed_now boolean)
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_conn   dojo.tenant_connections%rowtype;
    v_origin dojo.tenant_origin;
    v_now    boolean;
begin
    select c.* into v_conn
      from dojo.tenant_connections c
     where c.provider = p_provider and c.external_id = p_external_id;
    if not found then
        raise exception 'no dōjō is connected to % unit %', p_provider, p_external_id
            using errcode = 'no_data_found';
    end if;

    select t.origin into v_origin from dojo.tenants t where t.id = v_conn.tenant_id;
    if v_origin is distinct from 'organization' then
        raise exception 'only an organization''s dōjō is claimed'
            using errcode = 'invalid_parameter_value';
    end if;

    update dojo.tenant_connections c
       set verified_at = now(), updated_at = now()
     where c.id = v_conn.id;

    update dojo.tenants t
       set claimed_at = now(), claimed_by = p_principal, updated_at = now()
     where t.id = v_conn.tenant_id and t.claimed_at is null;
    v_now := found;

    if v_now then
        perform dojo.append_audit_event(v_conn.tenant_id, p_principal, 'dojo_claimed',
            jsonb_build_object('provider', p_provider, 'external_id', p_external_id,
                               'external_slug', v_conn.external_slug));
    end if;

    return query
        select t.id, t.claimed_at, t.claimed_by, v_now from dojo.tenants t where t.id = v_conn.tenant_id;
end;
$$;

revoke all on function dojo.claim_forge_unit(uuid, dojo.forge_provider, text) from public;
grant execute on function dojo.claim_forge_unit(uuid, dojo.forge_provider, text) to service_role;

comment on function dojo.claim_forge_unit(uuid, dojo.forge_provider, text) is
'Record a proven org admin''s claim on the unit''s dōjō (#308 R5a): re-prove the
connection, claim the tenant if unclaimed (first claim wins) and audit the claim.
service_role only — the server proves the admin against the forge first.';
