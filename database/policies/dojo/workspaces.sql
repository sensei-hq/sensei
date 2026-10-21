-- RLS for dojo.workspaces.
--
-- Moved out of ddl/table/dojo/workspaces.ddl because this policy now CALLS a
-- function — `dbd apply` creates every table before any function, so inline it
-- would fail the deploy. See policies/README.md.
--
-- WHAT CHANGED AND WHY. The policy read `using (user_id = (select auth.uid()))`.
-- the owner column (gone since #299) held a PRINCIPAL id and `auth.uid()` returns a LOGIN id
-- (spec dojo-auth-provisioning §VIII.2), so it matched nothing: a signed-in user
-- reading their own workspaces directly got an empty list. Nothing surfaced it,
-- because the Worker connects as `service_role` and bypasses RLS entirely — the
-- console kept working while the client-direct path was blind. Covered by
-- database/tests/dojo/rls_principal_grain.sql.
--
-- Reads connect as `authenticated`; writes go through security-definer functions.
--
-- Idempotent: the pre-release deploy re-applies every DDL file declaratively on
-- each deploy, and `create policy` is not idempotent — so drop-if-exists first.
-- `enable row level security` is idempotent (no error if already enabled).
alter table dojo.workspaces enable row level security;

-- RLS filters ROWS, but a table-level GRANT is still required for the role to
-- touch the table at all — without it the read is "permission denied" and RLS
-- never even evaluates. That is also why moving the grant here alongside the
-- policy is safe: a database brought up with a bare `dbd apply` (no
-- `--with-policies`) has neither, so `authenticated` cannot read the table at
-- all. A loud failure, never a silent leak.
grant select on dojo.workspaces to authenticated;

-- Who reads a workspace (#299; access matrix "Workspaces and repositories"): a
-- workspace is ONE SHARED item with no owner, so it is readable by whoever may
-- read one of its repositories — the repository's access is the workspace's.
-- dojo.can_read_workspace (SECURITY DEFINER) holds that one definition; it reads no
-- workspace RLS, so this is not circular. Covered by database/tests/dojo/workspaces_read.sql.
drop policy if exists projects_select_own on dojo.workspaces;
drop policy if exists workspaces_select on dojo.workspaces;
create policy workspaces_select
    on dojo.workspaces
    for select
    to authenticated
    using (dojo.can_read_workspace(id));

-- Nobody writes a workspace directly (#299). The sync files it
-- (dojo.file_repository_workspace, the only creator); a relay run attaches to it
-- (dojo.attach_run_to_workspace). Both are SECURITY DEFINER. The grants and
-- policies the per-person relay upsert used are removed — revoking is what makes
-- a direct insert "permission denied" rather than an RLS miss.
-- Covered by database/tests/dojo/workspaces_write.sql.
revoke insert, update on dojo.workspaces from authenticated;
drop policy if exists projects_file_own on dojo.workspaces;
drop policy if exists projects_refile_own on dojo.workspaces;
