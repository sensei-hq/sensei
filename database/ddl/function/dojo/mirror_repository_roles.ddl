set search_path to dojo, extensions;

-- Mirror the forge's per-repo permissions onto a repository (#307 D5). Called by the
-- server-side mirror job (service_role) with everyone the forge lists:
-- `[{"principal_id": uuid, "permission": "write"}, …]`. Sets the forge columns for
-- those listed and clears them for anyone it no longer lists; admin overrides are
-- never touched.
create or replace function dojo.mirror_repository_roles(p_repository uuid, p_entries jsonb)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    -- Who may call this is the GRANT below (service_role only): inside a SECURITY
    -- DEFINER body current_user is the owner, so it cannot tell callers apart.
    insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role, mirrored_at)
    select p_repository,
           (e ->> 'principal_id')::uuid,
           e ->> 'permission',
           dojo.forge_permission_to_role(e ->> 'permission'),
           now()
      from jsonb_array_elements(p_entries) e
    on conflict (repository_id, principal_id) do update
       set forge_permission = excluded.forge_permission,
           forge_role       = excluded.forge_role,
           mirrored_at      = excluded.mirrored_at;

    update dojo.repository_roles rr
       set forge_permission = null, forge_role = null, mirrored_at = now()
     where rr.repository_id = p_repository
       and not exists (
             select 1 from jsonb_array_elements(p_entries) e
              where (e ->> 'principal_id')::uuid = rr.principal_id
           );
end;
$$;

revoke all on function dojo.mirror_repository_roles(uuid, jsonb) from public;
grant execute on function dojo.mirror_repository_roles(uuid, jsonb) to service_role;

comment on function dojo.mirror_repository_roles(uuid, jsonb) is
'The forge mirror (#307 D5): set forge roles for everyone the forge lists on the
repository, clear them for anyone it no longer lists; overrides untouched.
service_role only.';
