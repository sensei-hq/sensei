set search_path to dojo, extensions;

-- Whether the signed-in principal is an admin of the workspace's dōjō. SECURITY
-- DEFINER so it does not depend on dojo.workspaces' own RLS, which shows a workspace
-- only to those who can read one of its repositories.
create or replace function dojo.administers_workspace(p_workspace uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select exists (
        select 1 from dojo.workspaces p
         where p.id = p_workspace
           and dojo.tenant_role_at_least(p.tenant_id, 'admin')
    );
$$;

revoke all on function dojo.administers_workspace(uuid) from public;
grant execute on function dojo.administers_workspace(uuid) to authenticated;

comment on function dojo.administers_workspace(uuid) is
'Whether the signed-in principal is an admin of the workspace''s dōjō.';
