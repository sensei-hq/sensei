-- RLS for dojo.projects.
--
-- Moved out of ddl/table/dojo/projects.ddl because this policy now CALLS a
-- function — `dbd apply` creates every table before any function, so inline it
-- would fail the deploy. See policies/README.md.
--
-- WHAT CHANGED AND WHY. The policy read `using (user_id = (select auth.uid()))`.
-- `projects.user_id` holds a PRINCIPAL id and `auth.uid()` returns a LOGIN id
-- (spec dojo-auth-provisioning §VIII.2), so it matched nothing: a signed-in user
-- reading their own projects directly got an empty list. Nothing surfaced it,
-- because the Worker connects as `service_role` and bypasses RLS entirely — the
-- console kept working while the client-direct path was blind. Covered by
-- database/tests/dojo/rls_principal_grain.sql.
--
-- The client-direct read path connects as `authenticated`; RLS narrows to the
-- user's own projects. SELECT only — the daemon upserts via the service_role
-- path. Own-rows model, like relay_sessions, but keyed on the owner column
-- directly (projects have no membership indirection).
--
-- Idempotent: the pre-release deploy re-applies every DDL file declaratively on
-- each deploy, and `create policy` is not idempotent — so drop-if-exists first.
-- `enable row level security` is idempotent (no error if already enabled).
alter table dojo.projects enable row level security;

-- RLS filters ROWS, but a table-level GRANT is still required for the role to
-- touch the table at all — without it the read is "permission denied" and RLS
-- never even evaluates. That is also why moving the grant here alongside the
-- policy is safe: a database brought up with a bare `dbd apply` (no
-- `--with-policies`) has neither, so `authenticated` cannot read the table at
-- all. A loud failure, never a silent leak.
grant select on dojo.projects to authenticated;

drop policy if exists projects_select_own on dojo.projects;
create policy projects_select_own
    on dojo.projects
    for select
    to authenticated
    using (user_id = dojo.current_principal_id());
