set search_path to dojo, extensions;

-- Remove an admin override, returning the person to what the forge says (#307 D5).
create or replace function dojo.clear_repository_role_override(p_repository uuid, p_principal uuid)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_tenant uuid;
begin
    select tenant_id into v_tenant from dojo.repositories where id = p_repository;
    if v_tenant is null or not dojo.tenant_role_at_least(v_tenant, 'admin') then
        raise exception 'only an admin of this repository''s dōjō clears its overrides'
            using errcode = 'insufficient_privilege';
    end if;
    update dojo.repository_roles
       set overridden = false, override_role = null, overridden_by = null, overridden_at = null
     where repository_id = p_repository and principal_id = p_principal;
end;
$$;

revoke all on function dojo.clear_repository_role_override(uuid, uuid) from public;
grant execute on function dojo.clear_repository_role_override(uuid, uuid) to authenticated;

comment on function dojo.clear_repository_role_override(uuid, uuid) is
'Remove an admin''s repository role override; the forge''s role applies again.';
