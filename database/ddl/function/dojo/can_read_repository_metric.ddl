set search_path to dojo, extensions;

-- Authorization for a repository-metric row, as a SECURITY DEFINER function.
--
-- WHY NOT INLINE IN THE POLICY. A row policy executes as the QUERYING role, so
-- every table it touches needs its own SELECT grant. Written inline, this check
-- reads principals, team_members, teams, team_projects, repositories_in_projects
-- and memberships — which would mean granting every signed-in user read access
-- to the ENTIRE organisation graph just to filter their own metrics. The check
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
  select exists (
    select 1
      from dojo.principals p
      join dojo.team_members tm            on tm.principal_id = p.id
      join dojo.teams t                    on t.id = tm.team_id
      join dojo.team_projects tp           on tp.team_id = t.id
      join dojo.repositories_in_projects r on r.project_id = tp.project_id
     where p.auth_user_id = auth.uid()
       and r.repository_id = p_repository_id
       and (
         -- Whole-repository numbers: any team member who reaches the repo.
         p_scope = 'repo'
         -- One person's own numbers: themselves…
         or p_principal_id = p.id
         -- …or a tenant admin. Deliberately NOT the whole team: "metrics by
         -- user" visible to every peer is surveillance, not transparency.
         or exists (
              select 1 from dojo.memberships m
               where m.tenant_id = p_tenant_id
                 and m.user_id  = p.auth_user_id
                 and m.role     = 'admin'
                 and m.disabled_at is null)
       )
  );
$$;

revoke all on function dojo.can_read_repository_metric(uuid, text, uuid, uuid) from public;
grant execute on function dojo.can_read_repository_metric(uuid, text, uuid, uuid) to authenticated;

-- No argument list on the COMMENT: dbd's SQL parser handles
-- `comment on function <name> is …` but not `…(uuid, text, …) is …`, and a file
-- it cannot parse is dropped from the entity set SILENTLY — the function is then
-- never created and the policy that calls it fails at deploy. Unambiguous here
-- because the function is not overloaded.
comment on function dojo.can_read_repository_metric is
'Row authorization for dojo.repository_metrics: principal → team_members → team →
team_projects → project → repositories_in_projects → repository.

SECURITY DEFINER so the caller needs no grant on the org graph the traversal
reads — inline in a policy it would have required exposing all of it.';
