set search_path to dojo, extensions;

-- The per-person forge mirror (#307 A2c, D5). Provisioning reads a member's own
-- token for their permission on every repository they can see and passes those
-- for one dōjō: `[{"repo_key": …, "permission": …}, …]`. Sets this person's forge
-- roles on the dōjō's repositories listed (matched on repo_key); clears their forge
-- roles on the dōjō's repositories not listed. Never touches another person,
-- another dōjō, or an admin override; a key the dōjō does not hold is ignored.
-- Call it only after a SUCCESSFUL read — an empty list clears every forge role.
create or replace function dojo.mirror_principal_repository_roles(
    p_principal uuid,
    p_tenant    uuid,
    p_entries   jsonb
)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    -- Who may call this is the GRANT below (service_role only).
    insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role, mirrored_at)
    select r.id, p_principal, e ->> 'permission', dojo.forge_permission_to_role(e ->> 'permission'), now()
      from jsonb_array_elements(p_entries) e
      join dojo.repositories r
        on r.tenant_id = p_tenant
       and r.repo_key = lower(e ->> 'repo_key')
    on conflict (repository_id, principal_id) do update
       set forge_permission = excluded.forge_permission,
           forge_role       = excluded.forge_role,
           mirrored_at      = excluded.mirrored_at;

    update dojo.repository_roles rr
       set forge_permission = null, forge_role = null, mirrored_at = now()
      from dojo.repositories r
     where rr.repository_id = r.id
       and r.tenant_id = p_tenant
       and rr.principal_id = p_principal
       and not exists (
             select 1 from jsonb_array_elements(p_entries) e
              where lower(e ->> 'repo_key') = r.repo_key
           );
end;
$$;

revoke all on function dojo.mirror_principal_repository_roles(uuid, uuid, jsonb) from public;
grant execute on function dojo.mirror_principal_repository_roles(uuid, uuid, jsonb) to service_role;

comment on function dojo.mirror_principal_repository_roles(uuid, uuid, jsonb) is
'Per-person forge mirror (#307 A2c): one member''s permissions on one dōjō''s
repositories, from their own token. Overrides, other people and other dōjōs are
untouched. service_role only; call only after a successful read.';
