set search_path to dojo, extensions;

-- Bring a person's org memberships in line with a COMPLETE read of their orgs on
-- one forge (#309, D5). p_kept are the organization dōjōs that read proved them a
-- member of. Each ACTIVE membership org provisioning made in an organization dōjō
-- connected to p_provider, and not in p_kept, ends (left_org) — and with it any
-- admin or lead power there. A membership that ended by leaving and is in p_kept
-- again is restored. Invited memberships, personal dōjōs and dōjōs on another
-- forge are never touched. service_role only: provisioning calls it after a
-- complete forge read, never on a failed or partial one.
create or replace function dojo.reconcile_org_memberships(p_principal uuid, p_provider dojo.forge_provider, p_kept uuid[])
    returns integer
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_ended integer;
begin
    update dojo.memberships m
       set disabled_at = now(), disabled_reason = 'left_org', updated_at = now()
      from dojo.tenants t
     where t.id = m.tenant_id
       and t.origin = 'organization'
       and m.user_id = p_principal
       and m.source = 'provisioned'
       and m.disabled_at is null
       and not (m.tenant_id = any (coalesce(p_kept, '{}')))
       and exists (select 1 from dojo.tenant_connections c
                    where c.tenant_id = m.tenant_id and c.provider = p_provider);
    get diagnostics v_ended = row_count;

    update dojo.memberships m
       set disabled_at = null, disabled_reason = null, updated_at = now()
     where m.user_id = p_principal
       and m.disabled_reason = 'left_org'
       and m.tenant_id = any (coalesce(p_kept, '{}'));

    return v_ended;
end;
$$;

revoke all on function dojo.reconcile_org_memberships(uuid, dojo.forge_provider, uuid[]) from public;
grant execute on function dojo.reconcile_org_memberships(uuid, dojo.forge_provider, uuid[]) to service_role;

comment on function dojo.reconcile_org_memberships(uuid, dojo.forge_provider, uuid[]) is
'End the provisioned org memberships a complete forge read no longer proves, and restore those that ended by leaving (#309). service_role only.';
