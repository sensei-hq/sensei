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

-- Scope guard. `dbd policies` applies every file under policies/ regardless of
-- --scope, so this file also runs against the daemon plane, where the `dojo`
-- schema does not exist. Ungated it reported "FAILED … schema dojo does not
-- exist" on every deploy there — an expected condition dressed as an error,
-- which is how real failures stop being read. Logged as a dbd gap in
-- docs/backlog.md; guarded here so both planes deploy clean.
do $$
begin
  if to_regclass('dojo.repository_metrics') is null then
    return;
  end if;

  -- Reads go through can_read_repository_metric (SECURITY DEFINER) so the
  -- caller needs no grant on the org graph the traversal walks. Writes are
  -- service_role only — a push is governed, and that cannot be a row policy.
  execute $stmt$alter table dojo.repository_metrics enable row level security$stmt$;
  execute $stmt$grant select on dojo.repository_metrics to authenticated$stmt$;
  execute $stmt$drop policy if exists repository_metrics_team_read on dojo.repository_metrics$stmt$;
  execute $stmt$create policy repository_metrics_team_read on dojo.repository_metrics
     for select to authenticated
     using (dojo.can_read_repository_metric(repository_id, scope, principal_id, tenant_id))$stmt$;
  execute $stmt$drop policy if exists repository_metrics_no_client_write on dojo.repository_metrics$stmt$;
  execute $stmt$create policy repository_metrics_no_client_write on dojo.repository_metrics
     for all to anon using (false) with check (false)$stmt$;
end $$;
