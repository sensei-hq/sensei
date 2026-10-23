set search_path to dojo, extensions;

-- Bring an organization dōjō's members in line with a COMPLETE read of the
-- organization's members on one forge (#308 R5b; #309's rule, read from the
-- organization's side — an Azure DevOps organization is read with an admin's
-- stored token, not each person's own).
--
-- p_subjects are the forge's ids for everyone the read listed; p_auth is the
-- identity kind those ids are recorded under (dojo.identities.provider, e.g.
-- azure_devops_pat). Each ACTIVE membership the forge's provisioning made
-- (source provisioned) in p_tenant — an organization dōjō connected to
-- p_provider — whose person holds an identity of that kind, none of them listed,
-- ends (left_org), and with it any admin or lead power. A membership that ended
-- by leaving and whose person is listed again is restored. Invited memberships,
-- people with no identity of that kind (the read can neither name nor clear
-- them), personal dōjōs and dōjōs not connected to p_provider are never touched.
--
-- A NULL or empty list is refused: only a complete read may end anything (#309),
-- and an organization read with a member's token always lists that member.
-- service_role only: the server calls it after a complete forge read, never on a
-- failed or partial one.
create or replace function dojo.reconcile_unit_members(
    p_tenant   uuid,
    p_provider dojo.forge_provider,
    p_auth     dojo.auth_method,
    p_subjects text[]
)
    returns integer
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_ended integer;
begin
    if p_subjects is null or cardinality(p_subjects) = 0 then
        raise exception 'a complete member list is needed to end a membership'
            using errcode = 'invalid_parameter_value';
    end if;

    if not exists (
        select 1 from dojo.tenants t
         where t.id = p_tenant and t.origin = 'organization'
           and exists (select 1 from dojo.tenant_connections c where c.tenant_id = t.id and c.provider = p_provider)
    ) then
        return 0;
    end if;

    update dojo.memberships m
       set disabled_at = now(), disabled_reason = 'left_org', updated_at = now()
     where m.tenant_id = p_tenant
       and m.source = 'provisioned'
       and m.disabled_at is null
       and exists (select 1 from dojo.identities i where i.principal_id = m.user_id and i.provider = p_auth)
       and not exists (select 1 from dojo.identities i
                        where i.principal_id = m.user_id and i.provider = p_auth and i.subject = any (p_subjects));
    get diagnostics v_ended = row_count;

    update dojo.memberships m
       set disabled_at = null, disabled_reason = null, updated_at = now()
     where m.tenant_id = p_tenant
       and m.disabled_reason = 'left_org'
       and exists (select 1 from dojo.identities i
                    where i.principal_id = m.user_id and i.provider = p_auth and i.subject = any (p_subjects));

    return v_ended;
end;
$$;

revoke all on function dojo.reconcile_unit_members(uuid, dojo.forge_provider, dojo.auth_method, text[]) from public, anon, authenticated;
grant execute on function dojo.reconcile_unit_members(uuid, dojo.forge_provider, dojo.auth_method, text[]) to service_role;

comment on function dojo.reconcile_unit_members(uuid, dojo.forge_provider, dojo.auth_method, text[]) is
'End the provisioned memberships of an organization dōjō whose person a complete
forge member read no longer lists, and restore those that ended by leaving and are
listed again (#308 R5b, #309). Refuses an empty list. service_role only.';
