set search_path to dojo, extensions;

-- An admin's override of a person's role on a repository (#307 D5): a role, or null
-- for no access. It wins over the forge and survives the next mirror. The person
-- must be a member of the repository's dōjō.
create or replace function dojo.set_repository_role_override(
    p_repository uuid,
    p_principal  uuid,
    p_role       dojo.member_role
)
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
        raise exception 'only an admin of this repository''s dōjō overrides its roles'
            using errcode = 'insufficient_privilege';
    end if;
    if not exists (select 1 from dojo.memberships m
                    where m.tenant_id = v_tenant and m.user_id = p_principal and m.disabled_at is null) then
        raise exception 'that person is not a member of this repository''s dōjō'
            using errcode = 'foreign_key_violation';
    end if;

    insert into dojo.repository_roles (repository_id, principal_id, overridden, override_role, overridden_by, overridden_at)
    values (p_repository, p_principal, true, p_role, dojo.current_principal_id(), now())
    on conflict (repository_id, principal_id) do update
       set overridden = true, override_role = excluded.override_role,
           overridden_by = excluded.overridden_by, overridden_at = excluded.overridden_at;
end;
$$;

revoke all on function dojo.set_repository_role_override(uuid, uuid, dojo.member_role) from public;
grant execute on function dojo.set_repository_role_override(uuid, uuid, dojo.member_role) to authenticated;

comment on function dojo.set_repository_role_override(uuid, uuid, dojo.member_role) is
'Admin override of a person''s repository role (#307 D5); null = no access. Wins over
the forge and survives re-mirroring.';
