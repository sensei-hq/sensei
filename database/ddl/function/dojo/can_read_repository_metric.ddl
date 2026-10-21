set search_path to dojo, extensions;

-- Authorization for a repository-metric row, as a SECURITY DEFINER function.
--
-- WHY NOT INLINE IN THE POLICY. A row policy executes as the QUERYING role, so
-- every table it touches needs its own SELECT grant. Written inline, this check
-- reads principals, repository_roles, repositories and memberships — which
-- would mean granting every signed-in user read access to the ENTIRE
-- organisation graph just to filter their own metrics. The check
-- would leak far more than it protects.
--
-- SECURITY DEFINER runs the traversal as the owner instead, so the caller needs
-- no grant on any of it and learns only the boolean.
--
-- `search_path` is pinned: a SECURITY DEFINER function that resolves names
-- through the caller's search_path can be hijacked by a same-named object in a
-- schema they control.
create or replace function dojo.can_read_repository_metric(
  p_repository_id uuid,
  p_scope         text,
  p_principal_id  uuid,
  p_tenant_id     uuid
) returns boolean
language sql
stable
security definer
set search_path = dojo, pg_temp
as $$
  select
       -- a person's own user-scoped numbers, always
       (p_scope = 'user' and p_principal_id = dojo.current_principal_id())
       -- repo-scoped: the rule every repository-scoped row shares
    or (p_scope = 'repo' and dojo.can_read_repository(p_repository_id, p_tenant_id))
       -- management: the whole tenant, user-scoped rows included
    or dojo.tenant_role_at_least(p_tenant_id, 'admin')
       -- another person's numbers: a maintainer or above on the repository (#307 D6)
    or (p_scope = 'user' and dojo.repo_role_at_least(p_repository_id, 'maintainer'));
$$;

revoke all on function dojo.can_read_repository_metric(uuid, text, uuid, uuid) from public;
grant execute on function dojo.can_read_repository_metric(uuid, text, uuid, uuid) to authenticated;

comment on function dojo.can_read_repository_metric(uuid, text, uuid, uuid) is
'Row authorization for dojo.repository_metrics, per the access matrix ("Metrics",
docs/spec/dojo/access-matrix.md): own user-scoped rows always; repo-scoped rows for
anyone with a role on the repository (#307 D5); another person''s user-scoped rows
for a maintainer or above on the repository (D6); the whole tenant for its admins.
Composed from current_principal_id, can_read_repository, repo_role_at_least and
tenant_role_at_least, so the relationships have one definition.';
