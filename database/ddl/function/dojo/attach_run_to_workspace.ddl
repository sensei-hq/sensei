set search_path to dojo, extensions;

-- Attach a relay run to the shared workspace it runs in (#299).
--
-- A relay is a person's sessions and tasks IN a workspace. It never creates one:
-- the workspace is a shared item the sync files (dojo.file_repository_workspace).
-- This finds the workspace by (tenant, slug) in a dōjō the caller is a member of
-- and records only what a run knows — when it last ran, and the constitution the
-- daemon resolved for it. Returns the workspace id, or NULL when the dōjō has no
-- such workspace yet (the run is still filed, under no workspace).
create or replace function dojo.attach_run_to_workspace(
    p_tenant_id    uuid,
    p_slug         text,
    p_constitution jsonb
)
    returns uuid
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_workspace uuid;
begin
    if dojo.current_principal_id() is null or not dojo.is_member(p_tenant_id) then
        raise exception 'not a member of this dōjō'
            using errcode = 'insufficient_privilege';
    end if;

    update dojo.workspaces p
       set last_run_at  = now(),
           constitution = coalesce(p_constitution, p.constitution),
           updated_at   = now()
     where p.tenant_id = p_tenant_id
       and p.slug = p_slug
    returning p.id into v_workspace;

    return v_workspace;
end;
$$;

revoke all on function dojo.attach_run_to_workspace(uuid, text, jsonb) from public;
grant execute on function dojo.attach_run_to_workspace(uuid, text, jsonb) to authenticated;

comment on function dojo.attach_run_to_workspace(uuid, text, jsonb) is
'Attach a relay run to its shared workspace (#299): find (tenant, slug) in a dōjō
the caller is a member of and record last_run_at and the constitution. Never
creates a workspace; NULL when there is none yet.';
