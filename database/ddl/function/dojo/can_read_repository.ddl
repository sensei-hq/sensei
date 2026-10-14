set search_path to dojo, extensions;

-- Who may read a repository's shared rows — its repo-scoped metrics and its code
-- graph: the tenant's admins, anyone on a team owning a project with the
-- repository, and that project's owner (access matrix "Metrics"; graph-sync.md).
--
-- One definition for every repository-scoped table; can_read_repository_metric
-- adds the person-scoped rules on top of it. SECURITY DEFINER for the reason
-- given there: the caller needs no grant on the org graph the traversal reads.
create or replace function dojo.can_read_repository(
  p_repository_id uuid,
  p_tenant_id     uuid
) returns boolean
language sql
stable
security definer
set search_path = dojo, pg_temp
as $$
  select dojo.tenant_role_at_least(p_tenant_id, 'admin')
      or exists (
           select 1
             from dojo.repositories_in_projects r
             join dojo.projects pr on pr.id = r.project_id
            where r.repository_id = p_repository_id
              and (dojo.on_team_of(r.project_id) or pr.user_id = dojo.current_principal_id())
         );
$$;

revoke all on function dojo.can_read_repository(uuid, uuid) from public;
grant execute on function dojo.can_read_repository(uuid, uuid) to authenticated;

comment on function dojo.can_read_repository(uuid, uuid) is
'Read authorization for a repository''s shared, repo-scoped rows: the tenant''s admins, anyone on a team owning a project with the repository, or that project''s owner. SECURITY DEFINER so the caller learns only the boolean.';
