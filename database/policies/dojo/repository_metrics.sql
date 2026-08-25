-- RLS for dojo.repository_metrics.
--
-- Lives here, not in ddl/table/dojo/repository_metrics.ddl, because this policy CALLS a
-- function and `dbd apply` creates every table before any function. Inline, it
-- failed the deploy outright:
--   function dojo.can_read_repository_metric(...) does not exist
--
-- `dbd policies` runs after all entities exist, which is the only ordering in
-- which a function-dependent policy can be created. Policies that call no
-- function (auth.uid() alone) stay with their table — see policies/README.md.

alter table dojo.repository_metrics enable row level security;

-- The authorization path, as a policy rather than as endpoint code:
--   principal → team_members → team → team_projects → project
--             → repositories_in_projects → repository
--
-- Being a member of the tenant is deliberately NOT enough. Access is granted at
-- the TEAM, so an org member outside the relevant team sees nothing — which is
-- the thing teams exist to enforce.
drop policy if exists repository_metrics_team_read on dojo.repository_metrics;
create policy repository_metrics_team_read on dojo.repository_metrics
  for select to authenticated
  using (
    dojo.can_read_repository_metric(
      repository_id, scope, principal_id, tenant_id)
  );

-- Writes are service_role only: a push is governed (per-repo authorization,
-- attribution rules), and that governance cannot be expressed as a row policy.
drop policy if exists repository_metrics_no_client_write on dojo.repository_metrics;
create policy repository_metrics_no_client_write on dojo.repository_metrics
    for all to anon using (false) with check (false);

-- Table-level SELECT grant for the `authenticated` read path. RLS filters WHICH
-- rows; the grant is what lets the role touch the table at all — without it the
-- policy above is dead code and every read fails with "permission denied"
-- rather than returning an authorised subset. Same pairing as dojo.projects and
-- dojo.relay_inbox.
grant select on dojo.repository_metrics to authenticated;
