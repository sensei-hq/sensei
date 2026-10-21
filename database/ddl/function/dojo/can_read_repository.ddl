set search_path to dojo, extensions;

-- Who may read a repository's shared rows — its repo-scoped metrics and its code
-- graph: the tenant's admins and anyone with a role on the repository (#307 D5 —
-- access lives on repositories, mirrored from the forge; access matrix
-- "Metrics"; graph-sync.md). Workspaces have no owner (#299) — and a workspace's own
-- read follows this function, so an owner clause here would also make that
-- circular.
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
      or dojo.repo_role_at_least(p_repository_id, 'contributor');
$$;

revoke all on function dojo.can_read_repository(uuid, uuid) from public;
grant execute on function dojo.can_read_repository(uuid, uuid) to authenticated;

comment on function dojo.can_read_repository(uuid, uuid) is
'Read authorization for a repository''s shared, repo-scoped rows: the tenant''s admins, or anyone with a role on the repository (#307 D5). Workspaces have no owner (#299). SECURITY DEFINER so the caller learns only the boolean.';
