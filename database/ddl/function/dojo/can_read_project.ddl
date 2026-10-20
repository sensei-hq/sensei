set search_path to dojo, extensions;

-- Who may read a project (#299): a project is ONE SHARED item with no owner, so
-- whoever may read one of its repositories may read it — the repository's
-- access (dojo.can_read_repository) is the project's.
--
-- SECURITY DEFINER because the repository links (dojo.repositories_in_projects)
-- admit no direct reads; the caller learns only the boolean. One definition, for
-- the projects read policy and anything else that asks.
create or replace function dojo.can_read_project(p_project uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select exists (select 1
                     from dojo.repositories_in_projects rip
                    where rip.project_id = p_project
                      and dojo.can_read_repository(rip.repository_id, rip.tenant_id));
$$;

revoke all on function dojo.can_read_project(uuid) from public;
grant execute on function dojo.can_read_project(uuid) to authenticated;

comment on function dojo.can_read_project(uuid) is
'Read authorization for a shared project (#299): true when the caller may read at
least one of its repositories (dojo.can_read_repository). SECURITY DEFINER so the
caller learns only the boolean.';
