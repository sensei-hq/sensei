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
-- Reads and the daemon's run upsert both connect as `authenticated`.
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

-- Who reads a project (access matrix "Projects and repositories", #236): its
-- owner wherever it is homed; a member of a team that owns it while it is homed
-- in the team's tenant (dojo.on_team_of); an admin of the tenant it is homed in.
-- A personal project (tenant_id null) is its owner's alone.
-- Covered by database/tests/dojo/projects_read.sql.
drop policy if exists projects_select_own on dojo.projects;
drop policy if exists projects_select on dojo.projects;
create policy projects_select
    on dojo.projects
    for select
    to authenticated
    using (
        user_id = dojo.current_principal_id()
        or dojo.on_team_of(id)
        or (tenant_id is not null and dojo.tenant_role_at_least(tenant_id, 'admin'))
    );

-- Who files a project (#236 step 4): the relay session POST upserts the run's
-- project on (user_id, slug) as the person whose daemon is running. The owner is
-- the caller, and the project is personal (no tenant) or homed in a tenant they
-- are an active member of. `phase` is not granted: a run never advances it.
-- Covered by database/tests/dojo/projects_write.sql.
grant insert (user_id, tenant_id, slug, name, classification, last_run_at, updated_at, constitution),
      update (user_id, tenant_id, slug, name, classification, last_run_at, updated_at, constitution)
   on dojo.projects to authenticated;

drop policy if exists projects_file_own on dojo.projects;
create policy projects_file_own
    on dojo.projects
    for insert
    to authenticated
    with check (
        user_id = dojo.current_principal_id()
        and (tenant_id is null or dojo.is_member(tenant_id))
    );

drop policy if exists projects_refile_own on dojo.projects;
create policy projects_refile_own
    on dojo.projects
    for update
    to authenticated
    using (user_id = dojo.current_principal_id())
    with check (
        user_id = dojo.current_principal_id()
        and (tenant_id is null or dojo.is_member(tenant_id))
    );
