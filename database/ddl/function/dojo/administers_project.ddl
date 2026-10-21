set search_path to dojo, extensions;

-- Whether the signed-in principal is an admin of the project's dōjō. SECURITY
-- DEFINER so it does not depend on dojo.projects' own RLS, which shows a project
-- only to those who can read one of its repositories.
create or replace function dojo.administers_project(p_project uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select exists (
        select 1 from dojo.projects p
         where p.id = p_project
           and dojo.tenant_role_at_least(p.tenant_id, 'admin')
    );
$$;

revoke all on function dojo.administers_project(uuid) from public;
grant execute on function dojo.administers_project(uuid) to authenticated;

comment on function dojo.administers_project(uuid) is
'Whether the signed-in principal is an admin of the project''s dōjō.';
